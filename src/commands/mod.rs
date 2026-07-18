//! AT command definitions for the Quectel BG9x command set: identity,
//! SIM/network status, PDP context control, MQTT, and SSL/TLS context
//! configuration.
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

/// `AT+QLTS` — queries the latest time synchronized through the network.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QLTS", NitzTimeResponse, timeout_ms = 300)]
pub struct GetNetworkNitzTime {
    #[at_arg(position = 1)]
    pub mode: NitzTimeQueryMode,
}

/// `AT+QNTP` — synchronizes local time with an NTP server. Requires an
/// active PDP context. Success here only means the request was accepted;
/// the result arrives as a [`crate::commands::urc::Urc::NtpTime`] URC.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QNTP", NoResponse, timeout_ms = 300)]
pub struct GetNetworkNtpTime {
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub server: String<100>,
}

/// `AT+QPOWD` — powers down the module.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QPOWD", NoResponse, timeout_ms = 300)]
pub struct PowerDown {
    #[at_arg(position = 1)]
    pub mode: PowerDownMode,
}

// --- MQTT ---

/// `AT+QMTCFG="version"` — sets the MQTT protocol version for a socket.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QMTCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureMqttVersion {
    /// Literal `"version"`.
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub tcp_connect_id: u8,
    #[at_arg(position = 2)]
    pub version: MqttVersion,
}

/// `AT+QMTCFG="ssl"` — enables/disables SSL for an MQTT socket and binds it
/// to an SSL context configured via the `ConfigureSsl*` commands.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QMTCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureMqttSsl {
    /// Literal `"ssl"`.
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub tcp_connect_id: u8,
    #[at_arg(position = 2)]
    pub ssl_enable: MqttSslEnable,
    /// SSL context ID, 0-5 — same ID used with the `ConfigureSsl*` commands.
    #[at_arg(position = 3)]
    pub ssl_ctx_id: u8,
}

/// `AT+QMTOPEN` — opens the network connection for an MQTT client. Success
/// here only means the command was accepted; the actual result (including
/// failures) arrives as a [`crate::commands::urc::Urc::MqttOpen`] URC, which
/// can take up to ~75s.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QMTOPEN", NoResponse, timeout_ms = 300)]
pub struct MqttOpen {
    #[at_arg(position = 1)]
    pub tcp_connect_id: u8,
    #[at_arg(position = 2)]
    pub server: String<100>,
    #[at_arg(position = 3)]
    pub port: u16,
}

/// `AT+QMTCONN` — sends the MQTT CONNECT packet. Result arrives as a
/// [`crate::commands::urc::Urc::MqttConnect`] URC.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QMTCONN", NoResponse, timeout_ms = 5000)]
pub struct MqttConnect {
    #[at_arg(position = 1)]
    pub tcp_connect_id: u8,
    /// Max 23 bytes.
    #[at_arg(position = 2)]
    pub client_id: String<23>,
    #[at_arg(position = 3)]
    pub username: Option<String<64>>,
    #[at_arg(position = 4)]
    pub password: Option<String<64>>,
}

/// `AT+QMTPUBEX` — publishes with extended parameters (message ID, retain).
/// Result arrives as a [`crate::commands::urc::Urc::MqttPublish`] URC.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QMTPUBEX", NoResponse, timeout_ms = 300)]
pub struct MqttPublishExtended {
    #[at_arg(position = 1)]
    pub tcp_connect_id: u8,
    /// 0-65535. Required (can be 0) even for QoS 0.
    #[at_arg(position = 2)]
    pub msg_id: u16,
    /// 0: at most once, 1: at least once, 2: exactly once.
    #[at_arg(position = 3)]
    pub qos: u8,
    /// 0: don't retain, 1: retain.
    #[at_arg(position = 4)]
    pub retain: u8,
    /// Max 128 bytes, UTF-8, no `+`/`#` wildcards.
    #[at_arg(position = 5)]
    pub topic: String<128>,
    /// Max 1024 bytes, UTF-8, no NUL bytes.
    #[at_arg(position = 6)]
    pub payload: String<1024>,
}

/// `AT+QMTDISC` — sends the MQTT DISCONNECT packet. Result arrives as a
/// [`crate::commands::urc::Urc::MqttDisconnect`] URC.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QMTDISC", NoResponse, timeout_ms = 300)]
pub struct MqttDisconnect {
    #[at_arg(position = 1)]
    pub tcp_connect_id: u8,
}

/// `AT+QMTCLOSE` — closes the MQTT network connection. Result arrives as a
/// [`crate::commands::urc::Urc::MqttClose`] URC.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QMTCLOSE", NoResponse, timeout_ms = 300)]
pub struct MqttClose {
    #[at_arg(position = 1)]
    pub tcp_connect_id: u8,
}

// --- SSL/TLS context configuration ---
//
// All `AT+QSSLCFG` commands share the same shape: a literal subcommand
// name, an SSL context ID (0-5), and one value. `context_id` is independent
// of the PDP context ID used for `ConfigureContext`/`ActivatePDPContext`.

/// `AT+QSSLCFG="sslversion"`.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslVersion {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub ssl_version: SslVersion,
}

/// `AT+QSSLCFG="ciphersuite"`. `cipher_suites` is a `"0xNNNN"` hex string —
/// see [`SslCipherSuiteEnum::to_bytes`].
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslCipherSuites {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub cipher_suites: SslCipherSuites,
}

/// `AT+QSSLCFG="cacert"` — path to a CA certificate already present in the
/// module's UFS file system (this crate doesn't yet handle uploading files —
/// see `NOTICE.md`/README for scope).
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslCaCertificate {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub ca_cert_path: String<128>,
}

/// `AT+QSSLCFG="clientcert"` — path to a client certificate in UFS.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslClientCertificate {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub client_cert_path: String<128>,
}

/// `AT+QSSLCFG="clientkey"` — path to a client private key in UFS.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslClientPrivateKey {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub client_key_path: String<128>,
}

/// `AT+QSSLCFG="seclevel"`.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslSecurityLevel {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub security_level: SslAuthenticationMode,
}

/// `AT+QSSLCFG="ignorelocaltime"`.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslIgnoreLocalTime {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub ignore_local_time: SslIgnoreLocalTime,
}

/// `AT+QSSLCFG="sni"`.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslSni {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub sni_enable: SslSniEnable,
}

/// `AT+QSSLCFG="checkhost"`.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QSSLCFG", NoResponse, timeout_ms = 300)]
pub struct ConfigureSslCheckHost {
    #[at_arg(position = 0)]
    pub subcommand: String<16>,
    #[at_arg(position = 1)]
    pub context_id: u8,
    #[at_arg(position = 2)]
    pub checkhost_enable: SslCheckHostEnable,
}
