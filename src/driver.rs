//! High-level async driver built on top of the raw AT command set in
//! [`crate::commands`].
//!
//! This does not own the UART or power control (PWRKEY/STATUS) — those are
//! board-specific and stay in the caller. This driver only knows how to talk
//! AT commands over whatever [`atat::asynch::AtatClient`] it's given.

use atat::asynch::AtatClient;
use atat::heapless::String;
use embassy_time::{Duration, Instant, Timer};

use crate::commands::responses::{GetSignalStrengthResponse, NetworkInfo, PDPContextInfo};
use crate::commands::types::{EchoOn, FunctionalityLevelOfUE, PowerDownMode};
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
        } else if status.code.as_str().contains("SIM PIN") || status.code.as_str().contains("SIM PUK") {
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
        self.client.send(&DeactivatePDPContext { context_id }).await?;
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
    pub async fn network_attach(&mut self, timeout: Duration) -> Result<RadioAccessTechnology, ModemError> {
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
                self.client.send(&GetEPSNetworkRegistrationStatus).await?.stat
            } else {
                self.client.send(&GetEGPRSNetworkRegistrationStatus).await?.stat
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
