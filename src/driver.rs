//! High-level async driver built on top of the raw AT command set in
//! [`crate::commands`].
//!
//! This does not own the UART or power control (PWRKEY/STATUS) — those are
//! board-specific and stay in the caller. This driver only knows how to talk
//! AT commands over whatever [`atat::asynch::AtatClient`] it's given.

use atat::asynch::AtatClient;
use atat::heapless::String;
use atat::heapless_bytes::Bytes;
use atat::UrcSubscription;
use embassy_time::{with_timeout, Duration, Instant, Timer};

use crate::commands::responses::{
    FileListResponse, FileReadStarted, GetSignalStrengthResponse, NetworkInfo, PDPContextInfo,
};
use crate::commands::types::{
    build_rat_search_order, ConfigurationEffect, EchoOn, FileOpenMode, FunctionalityLevelOfUE,
    IotOperationMode, MqttSslEnable, NitzTimeQueryMode, PowerDownMode, RatSearchingMode, SearchRat,
    ServiceDomain, SslAuthenticationMode, SslCheckHostEnable, SslCipherSuiteEnum, SslCipherSuites,
    SslIgnoreLocalTime, SslSniEnable, SslVersion,
};
use crate::commands::urc::Urc;
use crate::commands::*;
use crate::time::parse_timestamp;

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
    /// An `AT+QNTP` sync failed; payload is the raw Quectel error code from
    /// the `+QNTP` URC.
    NtpRequestFailed(u8),
    /// A `+QLTS`/`+QNTP` timestamp string didn't match the expected
    /// `"yy/MM/dd,hh:mm:ss±zz"` layout.
    TimeParseFailed,
    /// [`Bg9xModem::configure_rat_search_order`]'s RAT list was empty, had
    /// more than 3 entries, or contained a duplicate.
    InvalidRatOrder,
    /// An `AT+QFUPL`/`AT+QFWRITE` file transfer completed, but the modem
    /// reported a different size than what was actually sent.
    FileTransferFailed,
    /// `AT+QFLST` matched more files than [`FileListResponse`] has capacity
    /// for (5) — the modem's response parsed but couldn't be captured in
    /// full. Narrow `name_pattern` (e.g. list a specific filename instead of
    /// `"*"`) or delete unused files and retry.
    TooManyFiles,
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
/// system — get them there first with [`MqttModem::upload_file`] or
/// [`MqttModem::write_file`]. Leave `ca_cert_filename`/`client_cert_filename`/
/// `client_key_filename` as `None` for unauthenticated TLS (still encrypted,
/// just no cert pinning).
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
    ///
    /// Idempotent: `AT+QIACT` on an already-active context is rejected by
    /// the modem — e.g. when retrying after some later step (MQTT connect,
    /// etc.) failed and left the context up. On a rejected activate, this
    /// checks whether it's actually already active for `context_id` before
    /// treating it as a real failure. (`AT+QIACT?` isn't probed up front,
    /// since it may not report anything useful — not even a parseable
    /// response — before any context has ever been activated.)
    pub async fn activate_context(&mut self, context_id: u8) -> Result<PDPContextInfo, ModemError> {
        if self
            .client
            .send(&ActivatePDPContext { context_id })
            .await
            .is_err()
        {
            if let Ok(info) = self.client.send(&GetPDPContextInfo).await {
                if info.context_id == context_id && info.context_state == 1 {
                    return Ok(info);
                }
            }
            return Err(ModemError::NoContext);
        }

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

    /// `AT+QCFG="band"` — narrows the search to the given GSM/eMTC/NB-IoT
    /// band bitmasks, each a hex string from the Quectel AT command manual's
    /// per-variant band table (BG95 vs BG96 differ) — compute them yourself.
    pub async fn configure_bands(
        &mut self,
        gsm_mask: &str,
        emtc_mask: &str,
        nbiot_mask: &str,
        effect: ConfigurationEffect,
    ) -> Result<(), ModemError> {
        self.client
            .send(&ConfigureBands {
                gsm_band_mask: Bytes::try_from(gsm_mask.as_bytes())
                    .map_err(|_| ModemError::ArgumentTooLong)?,
                emtc_band_mask: Bytes::try_from(emtc_mask.as_bytes())
                    .map_err(|_| ModemError::ArgumentTooLong)?,
                nbiot_band_mask: Bytes::try_from(nbiot_mask.as_bytes())
                    .map_err(|_| ModemError::ArgumentTooLong)?,
                effect,
            })
            .await?;
        Ok(())
    }

    /// `AT+QCFG="nwscanseq"` — configures the RAT searching order from 1-3
    /// distinct RATs, e.g. `&[SearchRat::Emtc, SearchRat::NbIot]`.
    pub async fn configure_rat_search_order(
        &mut self,
        order: &[SearchRat],
        effect: ConfigurationEffect,
    ) -> Result<(), ModemError> {
        let rat_searching_sequence =
            build_rat_search_order(order).map_err(|_| ModemError::InvalidRatOrder)?;
        self.client
            .send(&ConfigureRatSearchingSequence {
                rat_searching_sequence,
                effect,
            })
            .await?;
        Ok(())
    }

    /// `AT+QCFG="nwscanmode"` — configures the RAT searching mode.
    pub async fn configure_rat_search_mode(
        &mut self,
        mode: RatSearchingMode,
        effect: ConfigurationEffect,
    ) -> Result<(), ModemError> {
        self.client
            .send(&ConfigureRatSearchingMode {
                rat_searching_mode: mode,
                effect,
            })
            .await?;
        Ok(())
    }

    /// `AT+QCFG="servicedomain"` — configures the service domain to register
    /// on.
    pub async fn configure_service_domain(
        &mut self,
        domain: ServiceDomain,
        effect: ConfigurationEffect,
    ) -> Result<(), ModemError> {
        self.client
            .send(&ConfigureServiceDomain {
                service_domain: domain,
                effect,
            })
            .await?;
        Ok(())
    }

    /// `AT+QCFG="iotopmode"` — configures the network category to search for
    /// under LTE RAT.
    pub async fn configure_iot_op_mode(
        &mut self,
        mode: IotOperationMode,
        effect: ConfigurationEffect,
    ) -> Result<(), ModemError> {
        self.client
            .send(&ConfigureIotOpMode { mode, effect })
            .await?;
        Ok(())
    }

    /// Resets the modem to factory defaults (`AT&F`), then restores the
    /// factory NV configuration (`AT+QCFG="nvrestore",0`).
    ///
    /// **This erases the module's internal flash**, including any
    /// certificates uploaded for SSL/TLS via [`MqttModem::upload_file`]/
    /// [`MqttModem::write_file`]. Not for routine use — reserve it for
    /// provisioning/recovery flows.
    pub async fn factory_reset(&mut self) -> Result<(), ModemError> {
        self.client.send(&ResetToFactoryDefault).await?;
        self.client.send(&RestoreFactoryConfiguration).await?;
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

    /// Queries the latest time synchronized through the network
    /// (`AT+QLTS`), returning a Unix timestamp.
    ///
    /// [`NitzTimeQueryMode::CurrentLocalTime`] adds the network's timezone
    /// offset to the wall-clock fields, so parsing it the same way as
    /// [`NitzTimeQueryMode::CurrentGmtTime`] doesn't put it back into a true
    /// GMT timestamp — prefer `CurrentGmtTime` unless the local time is what's
    /// actually wanted.
    pub async fn get_nitz_time(&mut self, mode: NitzTimeQueryMode) -> Result<i64, ModemError> {
        let response = self.client.send(&GetNetworkNitzTime { mode }).await?;
        parse_timestamp_or_err(response.time_and_dst.as_str())
    }

    /// Opens (or creates) a file on UFS (`AT+QFOPEN`), returning a
    /// filehandle for use with [`Self::read_file`]/[`Self::close_file`], or
    /// [`MqttModem::write_file`]. `mode` defaults to
    /// [`FileOpenMode::CreateOrOpen`] if `None`.
    ///
    /// There's no RAII guard tying the handle's lifetime to
    /// [`Self::close_file`] — the modem caps how many files can be open at
    /// once, so on an `Err` from a subsequent read/write, close the handle
    /// yourself before propagating the error rather than leaking it.
    pub async fn open_file(
        &mut self,
        filename: &str,
        mode: Option<FileOpenMode>,
    ) -> Result<u32, ModemError> {
        let filename = String::try_from(filename).map_err(|_| ModemError::ArgumentTooLong)?;
        Ok(self
            .client
            .send(&OpenFile { filename, mode })
            .await?
            .filehandle)
    }

    /// Closes a filehandle opened with [`Self::open_file`] (`AT+QFCLOSE`).
    pub async fn close_file(&mut self, filehandle: u32) -> Result<(), ModemError> {
        self.client.send(&CloseFile { filehandle }).await?;
        Ok(())
    }

    /// Deletes a file from UFS (`AT+QFDEL`).
    pub async fn delete_file(&mut self, filename: &str) -> Result<(), ModemError> {
        let file_path = String::try_from(filename).map_err(|_| ModemError::ArgumentTooLong)?;
        self.client
            .send(&DeleteFileFromInternalFlash { file_path })
            .await?;
        Ok(())
    }

    /// Lists files on UFS matching `name_pattern` (`AT+QFLST`), e.g. `"*"`
    /// for everything. Up to 5 entries per query — a 6th+ match doesn't
    /// truncate the list, it fails the whole call with
    /// [`ModemError::TooManyFiles`].
    pub async fn list_files(&mut self, name_pattern: &str) -> Result<FileListResponse, ModemError> {
        let name_pattern =
            String::try_from(name_pattern).map_err(|_| ModemError::ArgumentTooLong)?;
        self.client
            .send(&ListFilesFromInternalFlash { name_pattern })
            .await
            .map_err(|e| match e {
                atat::Error::Parse => ModemError::TooManyFiles,
                e => e.into(),
            })
    }

    /// Reads up to `length` bytes (256 max — this driver's read buffer size;
    /// larger requests are silently capped rather than rejected, and `None`
    /// defaults to the cap) from an already-open filehandle (`AT+QFREAD`),
    /// starting at its current position. The modem auto-advances the read
    /// position on each call — loop this, checking `read_length` against
    /// what was requested to detect end of file, to read more than 256
    /// bytes. `filehandle` is only valid until [`Self::close_file`]; on an
    /// `Err` from this call, close it anyway to avoid leaking the modem-side
    /// handle.
    pub async fn read_file(
        &mut self,
        filehandle: u32,
        length: Option<u32>,
    ) -> Result<FileReadStarted, ModemError> {
        let length = Some(length.unwrap_or(256).min(256));
        Ok(self.client.send(&ReadFile { filehandle, length }).await?)
    }
}

/// Parses a `+QLTS`/`+QNTP` timestamp string, mapping a parse failure to
/// [`ModemError::TimeParseFailed`]. Shared by [`Bg9xModem::get_nitz_time`]
/// and [`MqttModem::ntp_sync`].
fn parse_timestamp_or_err(s: &str) -> Result<i64, ModemError> {
    parse_timestamp(s).ok_or(ModemError::TimeParseFailed)
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

    /// Discards any URCs already sitting in the subscription queue.
    ///
    /// `wait_urc` never drains on timeout, so a URC that arrives just after
    /// one call's deadline is left queued for whichever call reads next.
    /// That's harmless for the MQTT flows below, which key their matcher on
    /// `tcpconnect_id` and simply ignore anything that doesn't match — but
    /// [`MqttModem::ntp_sync`]'s `+QNTP` URC, and [`MqttModem::upload_file`]/
    /// [`MqttModem::write_file`]'s `CONNECT`/`+QFUPL`/`+QFWRITE` URCs, carry
    /// no such correlation id, so a stale one from a previous timed-out call
    /// would otherwise be silently accepted as the result of a later,
    /// unrelated call. Call this right before issuing a fresh request that
    /// has no way to tell its own URC apart from a stale one.
    fn drain_stale_urcs(&mut self) {
        while self.urc_sub.try_next_message_pure().is_some() {}
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

    /// Publishes `payload` to `topic`. `qos` 0-2. `retain` sets the MQTT
    /// retain flag, so the broker holds this as the topic's last-known value
    /// for new subscribers. Waits for the broker's PUBACK (relayed as the
    /// `+QMTPUB` URC) or `timeout`.
    #[allow(clippy::too_many_arguments)]
    pub async fn mqtt_publish(
        &mut self,
        tcp_connect_id: u8,
        topic: &str,
        payload: &str,
        qos: u8,
        retain: bool,
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
                retain: if retain { 1 } else { 0 },
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

    /// Disconnects the MQTT client (`+QMTDISC`), waiting for its URC.
    ///
    /// The reference implementation this was ported from also sends
    /// `+QMTCLOSE` afterward, but only for a specific Quectel firmware
    /// revision ("R200") — gated off by default. Left out here since it
    /// isn't needed on a BG95-M3 and its URC never arrived when tried
    /// unconditionally (see `NOTICE.md`'s reference project for that path
    /// if a future modem/firmware combination needs it).
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
        .await
    }

    /// Synchronizes local time with an NTP server (`AT+QNTP`) and returns
    /// the result as a Unix timestamp. `context_id` must already be an
    /// active PDP context (see [`Bg9xModem::activate_context`]).
    ///
    /// The `+QNTP` URC carries no id to correlate it with this specific
    /// call, so [`Self::drain_stale_urcs`] is used to clear out anything
    /// left over from an earlier timed-out `ntp_sync` before issuing a new
    /// request — see that method's doc comment for why.
    pub async fn ntp_sync(
        &mut self,
        context_id: u8,
        server: &str,
        timeout: Duration,
    ) -> Result<i64, ModemError> {
        let deadline = Instant::now() + timeout;

        self.drain_stale_urcs();

        self.base
            .client
            .send(&GetNetworkNtpTime {
                context_id,
                server: String::try_from(server).map_err(|_| ModemError::ArgumentTooLong)?,
            })
            .await?;

        self.wait_urc(deadline, |urc| match urc {
            Urc::NtpTime(r) => Some(match (r.err, &r.time) {
                (0, Some(time)) => parse_timestamp_or_err(time.as_str()),
                (0, None) => Err(ModemError::TimeParseFailed),
                (code, _) => Err(ModemError::NtpRequestFailed(code)),
            }),
            _ => None,
        })
        .await
    }

    /// Clamps a caller-supplied timeout down to the 1-65535s range accepted
    /// by the modem's file-transfer `timeout` argument.
    fn clamp_timeout_secs(timeout: Duration) -> u16 {
        timeout.as_secs().clamp(1, u16::MAX as u64) as u16
    }

    /// Waits for the `CONNECT` data-mode prompt ([`Urc::FileDataModeStarted`]),
    /// sends `data` in 256-byte chunks, then waits for the transfer's
    /// completion URC via `on_complete` — the shared back half of
    /// [`Self::upload_file`] and [`Self::write_file`], which differ only in
    /// which command starts the transfer and which completion URC/field
    /// they check.
    async fn send_file_data<R>(
        &mut self,
        deadline: Instant,
        data: &[u8],
        on_complete: impl FnMut(&Urc) -> Option<Result<R, ModemError>>,
    ) -> Result<R, ModemError> {
        self.wait_urc(deadline, |urc| match urc {
            Urc::FileDataModeStarted => Some(Ok(())),
            _ => None,
        })
        .await?;

        for chunk in data.chunks(256) {
            self.base
                .client
                .send(&SendRawContents {
                    bytes: Bytes::try_from(chunk).unwrap(),
                })
                .await?;
        }

        self.wait_urc(deadline, on_complete).await
    }

    /// Uploads `data` as `filename` to UFS in one shot (`AT+QFUPL`) —
    /// creates or overwrites the file itself, unlike
    /// [`Bg9xModem::open_file`] + [`Self::write_file`], which write into an
    /// already-open filehandle. On success, returns the modem's reported
    /// checksum (`+QFUPL`'s 16-bit XOR-based checksum, as 4 hex digits) —
    /// this driver only verifies the reported *size* against what was sent;
    /// verify the checksum yourself if you need corruption detection beyond
    /// that (its exact algorithm isn't reproduced here).
    ///
    /// The modem answers `AT+QFUPL` itself with a bare `CONNECT` line
    /// instead of `OK`, delivered as [`Urc::FileDataModeStarted`] rather
    /// than this command's own response — so that command's own `send` can
    /// only return `Ok` on a bug-for-bug-unlikely modem, or `Err`, and an
    /// `Err` here is ambiguous between "the expected case: no OK/ERROR ever
    /// arrives, because the modem replied CONNECT instead" (an
    /// `atat::Error::Timeout`, silently expected) and a genuine modem
    /// rejection (`ERROR`/`+CME ERROR`, an `atat::Error` of any other kind),
    /// which resolves promptly and is propagated as a real error instead of
    /// being swallowed.
    ///
    /// `timeout` bounds waiting for `CONNECT` and the completion URC — not
    /// the initial `AT+QFUPL` round-trip itself, which always runs out to
    /// that command's own fixed internal timeout (currently 1s) first,
    /// since on the expected/successful path nothing ever signals that
    /// command's own response.
    ///
    /// Neither the `CONNECT` nor the completion URC carries an id to
    /// correlate it with this specific call, so [`Self::drain_stale_urcs`]
    /// clears out anything left over from an earlier timed-out
    /// `upload_file` before issuing a new request — see that method's doc
    /// comment for why.
    pub async fn upload_file(
        &mut self,
        filename: &str,
        data: &[u8],
        timeout: Duration,
    ) -> Result<Bytes<4>, ModemError> {
        let len = data.len() as u32;
        let upload_timeout_secs = Self::clamp_timeout_secs(timeout);

        self.drain_stale_urcs();

        let file_path = String::try_from(filename).map_err(|_| ModemError::ArgumentTooLong)?;
        match self
            .base
            .client
            .send(&FileUploadToInternalFlash {
                file_path,
                file_size: len,
                timeout: Some(upload_timeout_secs),
                ack_mode: None,
            })
            .await
        {
            Ok(_) | Err(atat::Error::Timeout) => {}
            Err(e) => return Err(e.into()),
        }

        let deadline = Instant::now() + timeout;
        self.send_file_data(deadline, data, |urc| match urc {
            Urc::FileUploadDone(r) => Some(if r.upload_size == len {
                Ok(r.checksum.clone())
            } else {
                Err(ModemError::FileTransferFailed)
            }),
            _ => None,
        })
        .await
    }

    /// Writes `data` to an already-open filehandle (`AT+QFWRITE`) — see
    /// [`Bg9xModem::open_file`]. Same `CONNECT`/completion-URC handshake,
    /// same stale-URC draining, and the same caveat about `timeout` not
    /// bounding the initial round-trip (this command's own internal timeout
    /// floor is 5s) or about a genuine modem error being propagated instead
    /// of swallowed — see [`Self::upload_file`]'s doc comment for why.
    pub async fn write_file(
        &mut self,
        filehandle: u32,
        data: &[u8],
        timeout: Duration,
    ) -> Result<(), ModemError> {
        let len = data.len() as u32;
        let write_timeout_secs = Self::clamp_timeout_secs(timeout);

        self.drain_stale_urcs();

        match self
            .base
            .client
            .send(&WriteFile {
                filehandle,
                length: len,
                timeout: Some(write_timeout_secs),
            })
            .await
        {
            Ok(_) | Err(atat::Error::Timeout) => {}
            Err(e) => return Err(e.into()),
        }

        let deadline = Instant::now() + timeout;
        self.send_file_data(deadline, data, |urc| match urc {
            Urc::FileWriteDone(r) => Some(if r.written_length == len {
                Ok(())
            } else {
                Err(ModemError::FileTransferFailed)
            }),
            _ => None,
        })
        .await
    }
}
