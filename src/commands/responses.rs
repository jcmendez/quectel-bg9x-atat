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

/// Latest time synchronized through the network (`AT+QLTS`).
///
/// `+QLTS: "yy/MM/dd,hh:mm:ss±zz,dst"` — a single quoted string combining the
/// timestamp and daylight-saving flag.
#[derive(Clone, Debug, AtatResp)]
pub struct NitzTimeResponse {
    #[at_arg(position = 1)]
    pub time_and_dst: String<32>,
}

/// URC `+QNTP: <err>,<time>` — result of an `AT+QNTP` NTP sync.
///
/// `err`: 0 on success, nonzero on failure (see Quectel's TCP/IP AT command
/// manual for the code table). `time`: `"yy/MM/dd,hh:mm:ss±zz"`, only
/// meaningful when `err == 0`.
///
/// `time` is modeled as `Option` because it's unconfirmed against real
/// hardware (or the manual) whether the module omits the `<time>` field on a
/// failed sync — same rationale as `MqttPublishResponse::value` for its
/// documented `[,<value>]` field elsewhere in this file. Modeling it as
/// `Option` means the URC parses correctly either way, so
/// [`crate::driver::MqttModem::ntp_sync`] can distinguish a real
/// [`crate::driver::ModemError::NtpRequestFailed`] from a genuine
/// [`crate::driver::ModemError::OperationTimeout`] regardless of which
/// behavior the modem actually exhibits.
#[derive(Clone, Debug, AtatResp)]
pub struct NtpTimeResponse {
    #[at_arg(position = 1)]
    pub err: u8,
    #[at_arg(position = 2)]
    pub time: Option<String<32>>,
}

/// Final result code indicating a mobile-equipment/network error
/// (`+CME ERROR: <err>`), delivered as a URC e.g. in response to
/// `AT+CPIN?` when no SIM is inserted.
#[derive(Clone, Debug, AtatResp)]
pub struct CmeError {
    #[at_arg(position = 1)]
    pub err: u8,
}

/// URC `+QMTOPEN: <tcpconnectID>,<result>` — result of opening the MQTT
/// network socket.
///
/// result: -1 failed to open network, 0 opened successfully, 1 wrong
/// parameter, 2 MQTT identifier occupied, 3 PDP activation failed, 4 domain
/// name parse failed, 5 network disconnection error.
#[derive(Clone, Debug, AtatResp)]
pub struct MqttOpenResponse {
    #[at_arg(position = 1)]
    pub tcpconnect_id: u8,
    #[at_arg(position = 2)]
    pub result: i8,
}

/// URC `+QMTSTAT: <tcpconnectID>,<err>` — asynchronous MQTT connection
/// status change (the socket was closed for some reason).
#[derive(Clone, Debug, AtatResp)]
pub struct MqttStatusResponse {
    #[at_arg(position = 1)]
    pub tcpconnect_id: u8,
    #[at_arg(position = 2)]
    pub err: u8,
}

/// URC `+QMTCONN: <tcpconnectID>,<result>[,<ret_code>]` — result of the
/// MQTT CONNECT handshake.
///
/// result: 0 CONNECT packet sent, 1 retransmission, 2 failed to send.
/// ret_code (only meaningful when result == 0): 0 accepted, 1 unacceptable
/// protocol version, 2 identifier rejected, 3 server unavailable, 4 bad
/// username/password, 5 not authorized.
#[derive(Clone, Debug, AtatResp)]
pub struct MqttConnectResponse {
    #[at_arg(position = 1)]
    pub tcpconnect_id: u8,
    #[at_arg(position = 2)]
    pub result: u8,
    #[at_arg(position = 3)]
    pub ret_code: u8,
}

/// URC `+QMTPUB: <tcpconnectID>,<messageID>,<result>[,<value>]` — result of
/// an MQTT publish.
///
/// result: 0 sent, 1 retransmission (`value` = retransmit count), 2 failed
/// to send.
#[derive(Clone, Debug, AtatResp)]
pub struct MqttPublishResponse {
    #[at_arg(position = 1)]
    pub tcpconnect_id: u8,
    #[at_arg(position = 2)]
    pub message_id: u16,
    #[at_arg(position = 3)]
    pub result: u8,
    #[at_arg(position = 4)]
    pub value: Option<u8>,
}

/// URC `+QMTDISC: <tcpconnectID>,<result>` — result of an MQTT DISCONNECT.
/// result: -1 failed, 0 closed successfully.
#[derive(Clone, Debug, AtatResp)]
pub struct MqttDisconnectResponse {
    #[at_arg(position = 1)]
    pub tcpconnect_id: u8,
    #[at_arg(position = 2)]
    pub result: i8,
}

/// URC `+QMTCLOSE: <tcpconnectID>,<result>` — result of closing the MQTT
/// network socket. result: -1 failed, 0 closed successfully.
#[derive(Clone, Debug, AtatResp)]
pub struct MqttCloseResponse {
    #[at_arg(position = 1)]
    pub tcpconnect_id: u8,
    #[at_arg(position = 2)]
    pub result: i8,
}
