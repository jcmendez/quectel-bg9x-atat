//! Response types for BG9x AT commands.
//!
//! Adapted from SC Robotics' `quectel-bg9x-eh-driver` (MIT) — see `/NOTICE.md`.
//! Doc comments quoting AT command semantics are from Quectel's BG95&BG96 AT
//! Commands Manual.

use atat::atat_derive::AtatResp;
use atat::heapless::String;
use atat::heapless_bytes::Bytes;

#[derive(Clone, AtatResp)]
pub struct NoResponse;

/// International Mobile Equipment Identity (IMEI) number of the module.
#[derive(Clone, Debug, AtatResp)]
pub struct Imei {
    pub imei: Bytes<15>,
}

/// Integrated Circuit Card Identifier number of the (U)SIM card.
#[derive(Clone, Debug, AtatResp)]
pub struct Iccid {
    pub iccid: Bytes<20>,
}

/// Response to `AT+CPIN?`.
///
/// `+CPIN: <code>`. May also surface as `+CME ERROR: <err>` (see [`CmeError`])
/// if e.g. no SIM is inserted.
#[derive(Clone, Debug, AtatResp)]
pub struct SimStatus {
    pub code: String<32>,
}

/// Firmware version identification (`AT+QGMR` / `AT+CGMR`).
#[derive(Clone, Debug, AtatResp)]
pub struct VersionInfo {
    #[at_arg(position = 0)]
    pub code: Bytes<64>,
}

/// Network information (`AT+QNWINFO`).
#[derive(Clone, Debug, AtatResp)]
pub struct NetworkInfo {
    /// Access technology selected: "No Service", "GSM", "GPRS", "EDGE",
    /// "eMTC", "NBIoT".
    #[at_arg(position = 1)]
    pub act: String<32>,
    /// Operator name in numeric format.
    #[at_arg(position = 2)]
    pub oper: Option<String<32>>,
    /// Band selected, e.g. "LTE BAND 1"..."LTE BAND 85".
    #[at_arg(position = 3)]
    pub band: Option<String<32>>,
    /// Channel selected.
    #[at_arg(position = 4)]
    pub channel: Option<u32>,
}

/// EPS (LTE-M/NB-IoT) network registration status (`AT+CEREG?`).
///
/// `+CEREG: <n>,<stat>[,[<tac>],[<ci>],[<AcT>][,...]]`
#[derive(Clone, Debug, AtatResp)]
pub struct EPSNetworkRegistrationStatusResponse {
    #[at_arg(position = 1)]
    pub n: u8,
    /// 0: not registered, not searching. 1: registered, home. 2: not
    /// registered, searching. 3: registration denied. 4: unknown. 5:
    /// registered, roaming.
    #[at_arg(position = 2)]
    pub stat: u8,
    /// Two-byte tracking area code in hex.
    #[at_arg(position = 3)]
    pub tac: Option<String<4>>,
    /// Four-byte E-UTRAN cell ID in hex.
    #[at_arg(position = 4)]
    pub ci: Option<String<8>>,
    /// Access technology: 0 GSM (n/a), 8 eMTC, 9 NB-IoT.
    #[at_arg(position = 5)]
    pub act: Option<u8>,
    #[at_arg(position = 6)]
    pub cause_type: Option<u8>,
    #[at_arg(position = 7)]
    pub reject_cause: Option<u8>,
    #[at_arg(position = 8)]
    pub active_time: Option<String<8>>,
    #[at_arg(position = 9)]
    pub periodic_tau: Option<String<8>>,
}

/// EGPRS (GSM/GPRS) network registration status (`AT+CGREG?`).
///
/// `+CGREG: <n>,<stat>[,[<lac>],[<ci>],[<AcT>],[<rac>][,...]]`
#[derive(Clone, Debug, AtatResp)]
pub struct EGPRSNetworkRegistrationStatusResponse {
    #[at_arg(position = 1)]
    pub n: u8,
    /// Same status codes as [`EPSNetworkRegistrationStatusResponse::stat`].
    #[at_arg(position = 2)]
    pub stat: u8,
    /// Two-byte location area code in hex.
    #[at_arg(position = 3)]
    pub lac: Option<String<4>>,
    /// Four-byte cell ID in hex.
    #[at_arg(position = 4)]
    pub ci: Option<String<8>>,
    /// Access technology: 0 GSM, 8/9 n/a here.
    #[at_arg(position = 5)]
    pub act: Option<u8>,
    #[at_arg(position = 6)]
    pub rac: Option<u8>,
    #[at_arg(position = 7)]
    pub reject_cause: Option<u8>,
    #[at_arg(position = 8)]
    pub active_time: Option<String<8>>,
    #[at_arg(position = 9)]
    pub periodic_rau: Option<String<8>>,
    #[at_arg(position = 10)]
    pub gprs_ready_timer: Option<String<8>>,
}

/// Signal quality (`AT+QCSQ`). All fields absent / `mode == "NOSERVICE"` when
/// not camped on any network.
#[derive(Clone, Debug, AtatResp)]
pub struct GetSignalStrengthResponse {
    pub mode: String<32>,
    /// Received signal strength (GSM and LTE).
    pub rssi: Option<i16>,
    /// Reference signal received power (LTE).
    pub lte_rsrp: Option<i16>,
    /// Signal to interference plus noise ratio, in 1/5th dB (LTE).
    pub lte_sinr: Option<i16>,
    /// Reference signal received quality, dB (LTE).
    pub lte_rsrq: Option<i16>,
}

/// PDP context info (`AT+QIACT?`). Only the first activated context is
/// parsed; the module may report up to 16.
#[derive(Clone, Debug, AtatResp)]
pub struct PDPContextInfo {
    #[at_arg(position = 1)]
    pub context_id: u8,
    /// 0: deactivated, 1: activated.
    #[at_arg(position = 2)]
    pub context_state: u8,
    /// 1: IPV4, 2: IPV6.
    #[at_arg(position = 3)]
    pub context_type: u8,
    #[at_arg(position = 4)]
    pub ip_address: Option<String<64>>,
}

/// Final result code indicating a mobile-equipment/network error
/// (`+CME ERROR: <err>`), delivered as a URC e.g. in response to
/// `AT+CPIN?` when no SIM is inserted.
#[derive(Clone, Debug, AtatResp)]
pub struct CmeError {
    #[at_arg(position = 1)]
    pub err: u8,
}
