//! AT command definitions for the bring-up subset of the Quectel BG9x
//! command set: identity, SIM/network status, and PDP context control.
//!
//! Adapted from SC Robotics' `quectel-bg9x-eh-driver` (MIT) — see `/NOTICE.md`.
//! Doc comments quoting AT command semantics are from Quectel's BG95&BG96 AT
//! Commands Manual.

pub mod responses;
pub mod types;
pub mod urc;

use atat::atat_derive::AtatCmd;
use atat::heapless::String;

use responses::*;
use types::*;

/// Bare `AT` — the simplest liveness check.
#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse, timeout_ms = 1000)]
pub struct At;

/// `ATE` — configures whether the unit echoes characters received from the
/// DTE in command mode.
#[derive(Debug, PartialEq, Clone, AtatCmd)]
#[at_cmd("E", NoResponse, timeout_ms = 1000, value_sep = false)]
pub struct SetEcho {
    #[at_arg(position = 0)]
    pub on: EchoOn,
}

/// `AT+CFUN` — sets UE functionality. Can take up to 15s per Quectel's docs.
#[derive(Debug, PartialEq, Clone, AtatCmd)]
#[at_cmd("+CFUN", NoResponse, timeout_ms = 15000)]
pub struct SetUeFunctionality {
    #[at_arg(position = 0)]
    pub fun: FunctionalityLevelOfUE,
}

/// `AT+CPIN?` — queries SIM card status.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CPIN?", SimStatus, timeout_ms = 1000)]
pub struct GetSimStatus;

/// `AT+QGMR` — queries the module firmware version.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QGMR", VersionInfo, timeout_ms = 1000)]
pub struct GetVersionInfo;

/// `AT+CGMR` — newer alias of `AT+QGMR`, returns only the first part of the
/// firmware version (e.g. "BG95M3LAR02A03" from
/// "BG95M3LAR02A03_01.012.01.012").
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGMR", VersionInfo, timeout_ms = 1000)]
pub struct GetVersionInfoCGMR;

/// `AT+GSN` — International Mobile Equipment Identity (IMEI).
#[derive(Clone, AtatCmd)]
#[at_cmd("+GSN", Imei, timeout_ms = 300)]
pub struct GetImei;

/// `AT+QCCID` — Integrated Circuit Card Identifier (ICCID) of the (U)SIM.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QCCID", Iccid, timeout_ms = 300)]
pub struct GetIccid;

/// `AT+QNWINFO` — access technology, operator, and band currently in use.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QNWINFO", NetworkInfo, timeout_ms = 300)]
pub struct GetNetworkInfo;

/// `AT+CEREG?` — LTE (EPS) network registration status.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CEREG?", EPSNetworkRegistrationStatusResponse, timeout_ms = 300)]
pub struct GetEPSNetworkRegistrationStatus;

/// `AT+CGREG?` — GSM/GPRS (EGPRS) network registration status.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGREG?", EGPRSNetworkRegistrationStatusResponse, timeout_ms = 300)]
pub struct GetEGPRSNetworkRegistrationStatus;

/// `AT+QCSQ` — signal strength of the current service network. Returns
/// `"NOSERVICE"` mode if not camped on any network.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QCSQ", GetSignalStrengthResponse, timeout_ms = 300)]
pub struct GetSignalStrength;

/// `AT+QICSGP` — configures the APN, username, password, and auth method of
/// a TCP/IP context. Must be set before [`ActivatePDPContext`].
#[derive(Clone, AtatCmd)]
#[at_cmd("+QICSGP", NoResponse, timeout_ms = 300)]
pub struct ConfigureContext {
    /// Context ID, 1-16.
    #[at_arg(position = 0)]
    pub context_id: u8,
    /// 1: IPV4, 2: IPV6, 3: IPV4V6.
    #[at_arg(position = 1)]
    pub context_type: u8,
    #[at_arg(position = 2)]
    pub apn: String<64>,
    #[at_arg(position = 3)]
    pub username: String<64>,
    #[at_arg(position = 4)]
    pub password: String<64>,
    /// 0: none, 1: PAP, 2: CHAP, 3: PAP or CHAP.
    #[at_arg(position = 5)]
    pub authentication: u8,
}

/// `AT+QIACT` — activates a PDP context previously configured with
/// [`ConfigureContext`]. Can take up to 150s depending on the network.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QIACT", NoResponse, timeout_ms = 150000)]
pub struct ActivatePDPContext {
    #[at_arg(position = 1)]
    pub context_id: u8,
}

/// `AT+QIACT?` — queries the state (and IP address, once activated) of PDP
/// contexts.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QIACT?", PDPContextInfo, timeout_ms = 300)]
pub struct GetPDPContextInfo;

/// `AT+QIDEACT` — deactivates a PDP context and closes all TCP/IP
/// connections in it. Can take up to 40s.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QIDEACT", NoResponse, timeout_ms = 40000)]
pub struct DeactivatePDPContext {
    #[at_arg(position = 1)]
    pub context_id: u8,
}

/// `AT+QPOWD` — powers down the module.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QPOWD", NoResponse, timeout_ms = 300)]
pub struct PowerDown {
    #[at_arg(position = 1)]
    pub mode: PowerDownMode,
}
