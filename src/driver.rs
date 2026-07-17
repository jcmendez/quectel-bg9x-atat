//! High-level async driver built on top of the raw AT command set in
//! [`crate::commands`].
//!
//! This does not own the UART or power control (PWRKEY/STATUS) — those are
//! board-specific and stay in the caller. This driver only knows how to talk
//! AT commands over whatever [`atat::asynch::AtatClient`] it's given.

use atat::asynch::AtatClient;
use atat::heapless::String;
use atat::UrcSubscription;
use embassy_time::{with_timeout, Duration, Instant, Timer};

use crate::commands::responses::{GetSignalStrengthResponse, NetworkInfo, PDPContextInfo};
use crate::commands::types::{
    EchoOn, FunctionalityLevelOfUE, MqttSslEnable, PowerDownMode, SslAuthenticationMode,
    SslCheckHostEnable, SslCipherSuiteEnum, SslCipherSuites, SslIgnoreLocalTime, SslSniEnable,
    SslVersion,
};
use crate::commands::urc::Urc;
use crate::commands::*;

const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ModemError {
    /// The command channel didn't get a (valid) response in time.
    NotResponding,
    /// SIM is missing, locked, or otherwise unusable.
    SimError,
    /// No network of the requested kind, or registration was denied.
    NoNetwork,
    /// A PDP context operation failed.
    NoContext,
    /// Waited past the caller-supplied deadline.
    OperationTimeout,
    /// A string argument didn't fit the command's fixed-capacity buffer.
    ArgumentTooLong,
    /// An MQTT open/publish/disconnect/close operation was rejected by the
    /// modem; payload is the raw Quectel result code from the corresponding
    /// URC (see that command's doc comment for what the code means).
    MqttRequestFailed(i8),
    /// The MQTT broker refused the CONNECT (bad credentials, protocol
    /// version, etc.); payload is the raw CONNACK return code.
    MqttConnectRefused(u8),
}

impl From<atat::Error> for ModemError {
    fn from(_: atat::Error) -> Self {
        ModemError::NotResponding
    }
}

/// Radio access technology the modem ended up camped on, as reported by
/// `AT+QNWINFO`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RadioAccessTechnology {
    Gsm,
    Gprs,
    Edge,
    Ltem,
    NbIot,
}

impl RadioAccessTechnology {
    fn from_act(act: &str) -> Option<Self> {
        if act.contains("eMTC") || act.contains("CAT-M1") {
            Some(Self::Ltem)
        } else if act.contains("NBIoT") || act.contains("CAT-NB1") {
            Some(Self::NbIot)
        } else if act.contains("EDGE") {
            Some(Self::Edge)
        } else if act.contains("GPRS") {
            Some(Self::Gprs)
        } else if act.contains("GSM") {
            Some(Self::Gsm)
        } else {
            None
        }
    }

    fn uses_eps_registration(self) -> bool {
        matches!(self, Self::Ltem | Self::NbIot)
    }
}

/// SSL/TLS context configuration, applied with [`Bg9xModem::configure_ssl_context`].
///
/// `context_id` (0-5) identifies the SSL context and is independent of the
/// PDP context ID used elsewhere — an MQTT socket is bound to one via
/// [`MqttModem::mqtt_connect`]'s `ssl_ctx_id` parameter.
///
/// Certificate paths refer to files already present in the module's UFS file
/// system — this crate doesn't yet handle uploading them (see `NOTICE.md`).
/// Leave `ca_cert_filename`/`client_cert_filename`/`client_key_filename` as
/// `None` for unauthenticated TLS (still encrypted, just no cert pinning).
#[derive(Clone, Debug)]
pub struct SslConfig {
    pub context_id: u8,
    pub ca_cert_filename: Option<String<128>>,
    pub client_cert_filename: Option<String<128>>,
    pub client_key_filename: Option<String<128>>,
    pub ssl_version: SslVersion,
    pub cipher_suite: SslCipherSuiteEnum,
    pub auth_mode: SslAuthenticationMode,
    pub sni_enable: SslSniEnable,
    pub checkhost_enable: SslCheckHostEnable,
    pub ignore_localtime: SslIgnoreLocalTime,
}

impl SslConfig {
    /// Defaults: TLS 1.2, all cipher suites, server-only auth, SNI enabled,
    /// hostname check disabled, certificate validity dates ignored (no
    /// RTC/NTP time on the module until you set one up).
    pub fn new(context_id: u8) -> Self {
        Self {
            context_id,
            ca_cert_filename: None,
            client_cert_filename: None,
            client_key_filename: None,
            ssl_version: SslVersion::Tls1_2,
            cipher_suite: SslCipherSuiteEnum::SupportAll,
            auth_mode: SslAuthenticationMode::ServerOnly,
            sni_enable: SslSniEnable::Enable,
            checkhost_enable: SslCheckHostEnable::Disable,
            ignore_localtime: SslIgnoreLocalTime::Ignore,
        }
    }
}

/// Async driver for the Quectel BG9x AT command set.
///
/// Generic over any [`AtatClient`], so it doesn't care which UART/transport
/// or embedded-io-async version backs it — wire that up in the caller and
/// hand over the client.
pub struct Bg9xModem<C: AtatClient> {
    client: C,
}

impl<C: AtatClient> Bg9xModem<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Give back the underlying client, e.g. to send commands this driver
    /// doesn't wrap yet.
    pub fn into_inner(self) -> C {
        self.client
    }

    /// Attach a URC subscription to unlock the MQTT methods, which need to
    /// observe URCs (`+QMTOPEN`/`+QMTCONN`/`+QMTPUB`/...) rather than direct
    /// command responses. Get a subscription from the same `atat::UrcChannel`
    /// your `Ingress` task was built with: `urc_channel.subscribe()`.
    pub fn with_urc_subscription<'sub, const URC_CAPACITY: usize, const URC_SUBSCRIBERS: usize>(
        self,
        urc_sub: UrcSubscription<'sub, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    ) -> MqttModem<'sub, C, URC_CAPACITY, URC_SUBSCRIBERS> {
        MqttModem {
            base: self,
            urc_sub,
        }
    }

    /// Applies an [`SslConfig`] to the modem: certificate paths (if any),
    /// security level, SSL version, cipher suite, SNI, hostname check, and
    /// local-time validity checking, in that order. Call before opening an
    /// MQTT (or other) socket that references this `context_id`.
    pub async fn configure_ssl_context(&mut self, config: &SslConfig) -> Result<(), ModemError> {
        let context_id = config.context_id;

        if let Some(path) = &config.ca_cert_filename {
            self.client
                .send(&ConfigureSslCaCertificate {
                    subcommand: String::try_from("cacert").unwrap(),
                    context_id,
                    ca_cert_path: path.clone(),
                })
                .await?;
        }
        if let Some(path) = &config.client_cert_filename {
            self.client
                .send(&ConfigureSslClientCertificate {
                    subcommand: String::try_from("clientcert").unwrap(),
                    context_id,
                    client_cert_path: path.clone(),
                })
                .await?;
        }
        if let Some(path) = &config.client_key_filename {
            self.client
                .send(&ConfigureSslClientPrivateKey {
                    subcommand: String::try_from("clientkey").unwrap(),
                    context_id,
                    client_key_path: path.clone(),
                })
                .await?;
        }

        self.client
            .send(&ConfigureSslSecurityLevel {
                subcommand: String::try_from("seclevel").unwrap(),
                context_id,
                security_level: config.auth_mode,
            })
            .await?;

        self.client
            .send(&ConfigureSslVersion {
                subcommand: String::try_from("sslversion").unwrap(),
                context_id,
                ssl_version: config.ssl_version,
            })
            .await?;

        let cipher_suites: SslCipherSuites = config.cipher_suite.to_bytes();
        self.client
            .send(&ConfigureSslCipherSuites {
                subcommand: String::try_from("ciphersuite").unwrap(),
                context_id,
                cipher_suites,
            })
            .await?;

        self.client
            .send(&ConfigureSslSni {
                subcommand: String::try_from("sni").unwrap(),
                context_id,
                sni_enable: config.sni_enable,
            })
            .await?;

        self.client
            .send(&ConfigureSslCheckHost {
                subcommand: String::try_from("checkhost").unwrap(),
                context_id,
                checkhost_enable: config.checkhost_enable,
            })
            .await?;

        self.client
            .send(&ConfigureSslIgnoreLocalTime {
                subcommand: String::try_from("ignorelocaltime").unwrap(),
                context_id,
                ignore_local_time: config.ignore_localtime,
            })
            .await?;

        Ok(())
    }

    /// Bare `AT` liveness check.
    pub async fn is_alive(&mut self) -> Result<(), ModemError> {
        self.client.send(&At).await?;
        Ok(())
    }

    pub async fn set_echo(&mut self, on: bool) -> Result<(), ModemError> {
        self.client
            .send(&SetEcho {
                on: if on { EchoOn::On } else { EchoOn::Off },
            })
            .await?;
        Ok(())
    }

    pub async fn set_full_functionality(&mut self) -> Result<(), ModemError> {
        self.client
            .send(&SetUeFunctionality {
                fun: FunctionalityLevelOfUE::Full,
            })
            .await?;
        Ok(())
    }

    /// Queries `AT+CPIN?` once. `Ok(true)` means the SIM is ready to use;
    /// `Ok(false)` means it responded but isn't ready yet (e.g. still
    /// initializing); `Err` means a PIN/PUK is required or the SIM is
    /// missing/faulty.
    pub async fn sim_ready(&mut self) -> Result<bool, ModemError> {
        let status = self.client.send(&GetSimStatus).await?;
        if status.code.as_str().contains("READY") {
            Ok(true)
        } else if status.code.as_str().contains("SIM PIN")
            || status.code.as_str().contains("SIM PUK")
        {
            Err(ModemError::SimError)
        } else {
            Ok(false)
        }
    }

    /// Polls [`Self::sim_ready`] until it reports ready or `timeout` elapses.
    pub async fn wait_sim_ready(&mut self, timeout: Duration) -> Result<(), ModemError> {
        let deadline = Instant::now() + timeout;
        loop {
            if self.sim_ready().await? {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(ModemError::OperationTimeout);
            }
            Timer::after(POLL_INTERVAL).await;
        }
    }

    pub async fn get_imei(&mut self) -> Result<atat::heapless_bytes::Bytes<15>, ModemError> {
        Ok(self.client.send(&GetImei).await?.imei)
    }

    pub async fn get_iccid(&mut self) -> Result<atat::heapless_bytes::Bytes<20>, ModemError> {
        Ok(self.client.send(&GetIccid).await?.iccid)
    }

    pub async fn get_signal_strength(&mut self) -> Result<GetSignalStrengthResponse, ModemError> {
        Ok(self.client.send(&GetSignalStrength).await?)
    }

    pub async fn get_network_info(&mut self) -> Result<NetworkInfo, ModemError> {
        Ok(self.client.send(&GetNetworkInfo).await?)
    }

    /// Configures PDP context `context_id` with the given APN/credentials.
    /// Call before [`Self::activate_context`].
    pub async fn configure_context(
        &mut self,
        context_id: u8,
        apn: &str,
        username: &str,
        password: &str,
        authentication: u8,
    ) -> Result<(), ModemError> {
        self.client
            .send(&ConfigureContext {
                context_id,
                context_type: 1, // IPV4
                apn: String::try_from(apn).map_err(|_| ModemError::ArgumentTooLong)?,
                username: String::try_from(username).map_err(|_| ModemError::ArgumentTooLong)?,
                password: String::try_from(password).map_err(|_| ModemError::ArgumentTooLong)?,
                authentication,
            })
            .await?;
        Ok(())
    }

    /// Activates `context_id` and returns its state (including the assigned
    /// IP address, once up). Can take up to 150s on the network side.
    pub async fn activate_context(&mut self, context_id: u8) -> Result<PDPContextInfo, ModemError> {
        self.client.send(&ActivatePDPContext { context_id }).await?;
        let info = self.client.send(&GetPDPContextInfo).await?;
        if info.context_state != 1 {
            return Err(ModemError::NoContext);
        }
        Ok(info)
    }

    pub async fn deactivate_context(&mut self, context_id: u8) -> Result<(), ModemError> {
        self.client
            .send(&DeactivatePDPContext { context_id })
            .await?;
        Ok(())
    }

    pub async fn power_down(&mut self) -> Result<(), ModemError> {
        self.client
            .send(&PowerDown {
                mode: PowerDownMode::Normal,
            })
            .await?;
        Ok(())
    }

    /// Waits for the modem to camp on a network and register, polling
    /// `AT+QNWINFO` and then `AT+CEREG?`/`AT+CGREG?` (whichever applies to
    /// the RAT it camped on) until home/roaming registration, denial, or
    /// `timeout`.
    pub async fn network_attach(
        &mut self,
        timeout: Duration,
    ) -> Result<RadioAccessTechnology, ModemError> {
        let deadline = Instant::now() + timeout;

        let rat = loop {
            let info = self.client.send(&GetNetworkInfo).await?;
            if let Some(rat) = RadioAccessTechnology::from_act(info.act.as_str()) {
                break rat;
            }
            if Instant::now() >= deadline {
                return Err(ModemError::OperationTimeout);
            }
            Timer::after(POLL_INTERVAL).await;
        };

        loop {
            let stat = if rat.uses_eps_registration() {
                self.client
                    .send(&GetEPSNetworkRegistrationStatus)
                    .await?
                    .stat
            } else {
                self.client
                    .send(&GetEGPRSNetworkRegistrationStatus)
                    .await?
                    .stat
            };
            match stat {
                1 | 5 => return Ok(rat),
                3 | 4 => return Err(ModemError::NoNetwork),
                _ => {}
            }
            if Instant::now() >= deadline {
                return Err(ModemError::OperationTimeout);
            }
            Timer::after(POLL_INTERVAL).await;
        }
    }
}

/// A [`Bg9xModem`] plus a URC subscription, unlocking the MQTT methods.
/// Build one with [`Bg9xModem::with_urc_subscription`].
///
/// Derefs to [`Bg9xModem`], so all the base methods (`is_alive`,
/// `network_attach`, `configure_ssl_context`, etc.) are still available.
pub struct MqttModem<'sub, C: AtatClient, const URC_CAPACITY: usize, const URC_SUBSCRIBERS: usize> {
    base: Bg9xModem<C>,
    urc_sub: UrcSubscription<'sub, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<'sub, C: AtatClient, const URC_CAPACITY: usize, const URC_SUBSCRIBERS: usize> core::ops::Deref
    for MqttModem<'sub, C, URC_CAPACITY, URC_SUBSCRIBERS>
{
    type Target = Bg9xModem<C>;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'sub, C: AtatClient, const URC_CAPACITY: usize, const URC_SUBSCRIBERS: usize>
    core::ops::DerefMut for MqttModem<'sub, C, URC_CAPACITY, URC_SUBSCRIBERS>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<'sub, C: AtatClient, const URC_CAPACITY: usize, const URC_SUBSCRIBERS: usize>
    MqttModem<'sub, C, URC_CAPACITY, URC_SUBSCRIBERS>
{
    /// Detach the URC subscription and hand back a plain [`Bg9xModem`].
    pub fn into_base(self) -> Bg9xModem<C> {
        self.base
    }

    /// Waits until `matcher` returns `Some` for an incoming URC, or
    /// `deadline` passes. `matcher` returning `None` means "not the URC I'm
    /// waiting for, keep going" — other URCs (e.g. an unrelated `+CME
    /// ERROR`) are silently skipped rather than treated as failures, since
    /// several command flows run concurrently against the same URC stream.
    async fn wait_urc<R>(
        &mut self,
        deadline: Instant,
        mut matcher: impl FnMut(&Urc) -> Option<Result<R, ModemError>>,
    ) -> Result<R, ModemError> {
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(ModemError::OperationTimeout);
            }
            let urc = with_timeout(deadline - now, self.urc_sub.next_message_pure())
                .await
                .map_err(|_| ModemError::OperationTimeout)?;
            if let Some(result) = matcher(&urc) {
                return result;
            }
        }
    }

    /// Opens an MQTT network socket, optionally over SSL/TLS, and connects
    /// with the given client ID/credentials. `ssl_ctx_id`, if given, must
    /// already be configured via [`Bg9xModem::configure_ssl_context`].
    /// `timeout` bounds the whole sequence (open can alone take ~75s on a
    /// slow network).
    #[allow(clippy::too_many_arguments)]
    pub async fn mqtt_connect(
        &mut self,
        tcp_connect_id: u8,
        host: &str,
        port: u16,
        client_id: &str,
        username: Option<&str>,
        password: Option<&str>,
        ssl_ctx_id: Option<u8>,
        timeout: Duration,
    ) -> Result<(), ModemError> {
        let deadline = Instant::now() + timeout;

        if let Some(ctx_id) = ssl_ctx_id {
            self.base
                .client
                .send(&ConfigureMqttSsl {
                    subcommand: String::try_from("ssl").unwrap(),
                    tcp_connect_id,
                    ssl_enable: MqttSslEnable::True,
                    ssl_ctx_id: ctx_id,
                })
                .await?;
        }

        self.base
            .client
            .send(&MqttOpen {
                tcp_connect_id,
                server: String::try_from(host).map_err(|_| ModemError::ArgumentTooLong)?,
                port,
            })
            .await?;

        self.wait_urc(deadline, |urc| match urc {
            Urc::MqttOpen(r) if r.tcpconnect_id == tcp_connect_id => Some(match r.result {
                0 => Ok(()),
                code => Err(ModemError::MqttRequestFailed(code)),
            }),
            _ => None,
        })
        .await?;

        self.base
            .client
            .send(&MqttConnect {
                tcp_connect_id,
                client_id: String::try_from(client_id).map_err(|_| ModemError::ArgumentTooLong)?,
                username: username
                    .map(String::try_from)
                    .transpose()
                    .map_err(|_| ModemError::ArgumentTooLong)?,
                password: password
                    .map(String::try_from)
                    .transpose()
                    .map_err(|_| ModemError::ArgumentTooLong)?,
            })
            .await?;

        self.wait_urc(deadline, |urc| match urc {
            Urc::MqttConnect(r) if r.tcpconnect_id == tcp_connect_id => {
                Some(match (r.result, r.ret_code) {
                    (0, 0) => Ok(()),
                    (0, ret_code) => Err(ModemError::MqttConnectRefused(ret_code)),
                    (result, _) => Err(ModemError::MqttRequestFailed(result as i8)),
                })
            }
            _ => None,
        })
        .await
    }

    /// Publishes `payload` to `topic`. `qos` 0-2. Waits for the broker's
    /// PUBACK (relayed as the `+QMTPUB` URC) or `timeout`.
    pub async fn mqtt_publish(
        &mut self,
        tcp_connect_id: u8,
        topic: &str,
        payload: &str,
        qos: u8,
        timeout: Duration,
    ) -> Result<(), ModemError> {
        let deadline = Instant::now() + timeout;
        // Quectel wants a nonzero message ID even for QoS 0.
        let msg_id: u16 = if qos == 0 { 0 } else { 1 };

        self.base
            .client
            .send(&MqttPublishExtended {
                tcp_connect_id,
                msg_id,
                qos,
                retain: 0,
                topic: String::try_from(topic).map_err(|_| ModemError::ArgumentTooLong)?,
                payload: String::try_from(payload).map_err(|_| ModemError::ArgumentTooLong)?,
            })
            .await?;

        self.wait_urc(deadline, |urc| match urc {
            Urc::MqttPublish(r) if r.tcpconnect_id == tcp_connect_id => Some(match r.result {
                0 => Ok(()),
                code => Err(ModemError::MqttRequestFailed(code as i8)),
            }),
            _ => None,
        })
        .await
    }

    /// Disconnects the MQTT client (`+QMTDISC`) and closes the underlying
    /// network socket (`+QMTCLOSE`), waiting for both URCs.
    pub async fn mqtt_disconnect(
        &mut self,
        tcp_connect_id: u8,
        timeout: Duration,
    ) -> Result<(), ModemError> {
        let deadline = Instant::now() + timeout;

        self.base
            .client
            .send(&MqttDisconnect { tcp_connect_id })
            .await?;
        self.wait_urc(deadline, |urc| match urc {
            Urc::MqttDisconnect(r) if r.tcpconnect_id == tcp_connect_id => Some(match r.result {
                0 => Ok(()),
                code => Err(ModemError::MqttRequestFailed(code)),
            }),
            _ => None,
        })
        .await?;

        self.base.client.send(&MqttClose { tcp_connect_id }).await?;
        self.wait_urc(deadline, |urc| match urc {
            Urc::MqttClose(r) if r.tcpconnect_id == tcp_connect_id => Some(match r.result {
                0 => Ok(()),
                code => Err(ModemError::MqttRequestFailed(code)),
            }),
            _ => None,
        })
        .await
    }
}
