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
use atat::heapless_bytes::Bytes;

use responses::*;
use types::*;

/// Bare `AT` — the simplest liveness check.
#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse, timeout_ms = 1000)]
pub struct At;

/// `AT&F` — resets all parameters to their factory default values.
#[derive(Clone, AtatCmd)]
#[at_cmd("&F", NoResponse, timeout_ms = 300)]
pub struct ResetToFactoryDefault;

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

// --- Radio/network configuration ---
//
// All `AT+QCFG` commands here share the same shape: the subcommand name is
// baked into the `at_cmd` string (it's always a fixed literal, never chosen
// at runtime), followed by one or more values and a trailing
// `ConfigurationEffect` (0: after reboot, 1: immediately).

/// `AT+QCFG="band"` — narrows the search to the given GSM/eMTC/NB-IoT band
/// bitmasks. Masks are hex strings from the Quectel AT command manual's
/// per-variant band tables (BG95 vs BG96 differ) — compute them yourself and
/// pass the raw string; see [`crate::driver::Bg9xModem::configure_bands`].
#[derive(Clone, AtatCmd)]
#[at_cmd("+QCFG=\"band\"", NoResponse, timeout_ms = 300)]
pub struct ConfigureBands {
    #[at_arg(position = 0)]
    pub gsm_band_mask: Bytes<24>,
    #[at_arg(position = 1)]
    pub emtc_band_mask: Bytes<24>,
    #[at_arg(position = 2)]
    pub nbiot_band_mask: Bytes<24>,
    #[at_arg(position = 3)]
    pub effect: ConfigurationEffect,
}

/// `AT+QCFG="nwscanseq"` — configures the RAT searching sequence, e.g.
/// `"020301"` for eMTC -> NB-IoT -> GSM, or `"00"` for automatic. Build the
/// value with [`types::build_rat_search_order`].
#[derive(Clone, AtatCmd)]
#[at_cmd("+QCFG=\"nwscanseq\"", NoResponse, timeout_ms = 300)]
pub struct ConfigureRatSearchingSequence {
    #[at_arg(position = 0)]
    pub rat_searching_sequence: Bytes<8>,
    #[at_arg(position = 1)]
    pub effect: ConfigurationEffect,
}

/// `AT+QCFG="nwscanmode"` — configures the RAT searching mode.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QCFG=\"nwscanmode\"", NoResponse, timeout_ms = 300)]
pub struct ConfigureRatSearchingMode {
    #[at_arg(position = 0)]
    pub rat_searching_mode: RatSearchingMode,
    #[at_arg(position = 1)]
    pub effect: ConfigurationEffect,
}

/// `AT+QCFG="servicedomain"` — configures the service domain to register on.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QCFG=\"servicedomain\"", NoResponse, timeout_ms = 300)]
pub struct ConfigureServiceDomain {
    #[at_arg(position = 0)]
    pub service_domain: ServiceDomain,
    #[at_arg(position = 1)]
    pub effect: ConfigurationEffect,
}

/// `AT+QCFG="iotopmode"` — configures the network category to search for
/// under LTE RAT.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QCFG=\"iotopmode\"", NoResponse, timeout_ms = 300)]
pub struct ConfigureIotOpMode {
    #[at_arg(position = 0)]
    pub mode: IotOperationMode,
    #[at_arg(position = 1)]
    pub effect: ConfigurationEffect,
}

/// `AT+QCFG="nvrestore",0` — restores the factory NV configuration. Fixed
/// command, no arguments.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QCFG=\"nvrestore\",0", NoResponse, timeout_ms = 300)]
pub struct RestoreFactoryConfiguration;

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
/// module's UFS file system. Get it there first with
/// [`crate::driver::MqttModem::upload_file`] or
/// [`crate::driver::MqttModem::write_file`].
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

// --- UFS file management ---
//
// Ported from the reference driver's file-management support — see
// `NOTICE.md`. `AT+QFDWL` is intentionally not ported: the reference's own
// implementation of it can't capture the binary payload with the atat
// framework and just drops it on the floor. `OpenFile`+`ReadFile`+`CloseFile`
// below is the reference's own working read path and covers the same need
// (e.g. reading back an uploaded cert to verify it).

/// `AT+QFUPL` — uploads a file to UFS in a single shot. The modem replies to
/// this command with a bare `CONNECT` line instead of `OK`, delivered as the
/// [`crate::commands::urc::Urc::FileDataModeStarted`] URC — so this
/// command's own response never resolves normally, and its `Err`/timeout is
/// discarded rather than propagated. See
/// [`crate::driver::MqttModem::upload_file`].
#[derive(Clone, AtatCmd)]
#[at_cmd("+QFUPL", NoResponse, timeout_ms = 1000)]
pub struct FileUploadToInternalFlash {
    /// The file path in the file system.
    #[at_arg(position = 1)]
    pub file_path: String<80>,
    /// Size of the file to be uploaded, in bytes.
    #[at_arg(position = 2)]
    pub file_size: u32,
    /// Seconds to wait for data to be sent in. 1-65535.
    #[at_arg(position = 3)]
    pub timeout: Option<u16>,
    /// Whether to use acknowledgment mode.
    #[at_arg(position = 4)]
    pub ack_mode: Option<bool>,
}

/// Raw file bytes sent in response to a `CONNECT` prompt from
/// [`FileUploadToInternalFlash`] or [`WriteFile`]. Not a real AT command —
/// no formatting, no `OK`/`ERROR` expected (`EXPECTS_RESPONSE_CODE = false`).
///
/// Requires the `atat::asynch::Client`'s shared command buffer to be at
/// least `MAX_LEN` (256) bytes — that buffer is sized once by whoever wires
/// up the client, independent of any single command's `MAX_LEN`, so this
/// can't be enforced at compile time. `AtatCmd::write`'s impl below asserts
/// on it instead of panicking on an out-of-bounds slice copy.
pub struct SendRawContents {
    pub bytes: Bytes<256>,
}

impl atat::AtatCmd for SendRawContents {
    type Response = NoResponse;
    const MAX_LEN: usize = 256;
    const EXPECTS_RESPONSE_CODE: bool = false;

    fn write(&self, buf: &mut [u8]) -> usize {
        let len = self.bytes.len();
        assert!(
            buf.len() >= len,
            "atat client buffer must be at least SendRawContents::MAX_LEN (256) bytes"
        );
        buf[..len].copy_from_slice(&self.bytes);
        len
    }

    fn parse(
        &self,
        _resp: Result<&[u8], atat::InternalError>,
    ) -> Result<Self::Response, atat::Error> {
        Ok(NoResponse)
    }
}

/// `AT+QFDEL` — deletes a file from UFS.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QFDEL", NoResponse, timeout_ms = 300)]
pub struct DeleteFileFromInternalFlash {
    /// The file path in the file system.
    #[at_arg(position = 1)]
    pub file_path: String<80>,
}

/// `AT+QFLST` — lists files in UFS matching `name_pattern` (e.g. `"*"` for
/// all files), up to 5 entries per query.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QFLST", FileListResponse, timeout_ms = 1000)]
pub struct ListFilesFromInternalFlash {
    /// The file pattern to be listed, e.g. `"*"` for all files.
    #[at_arg(position = 1)]
    pub name_pattern: String<80>,
}

/// `AT+QFOPEN` — opens (or creates) a file, returning a filehandle for use
/// with [`ReadFile`]/[`WriteFile`]/[`CloseFile`]. `mode` defaults to
/// [`FileOpenMode::CreateOrOpen`] if omitted.
#[derive(Clone, AtatCmd)]
#[at_cmd("+QFOPEN", FileOpenResponse, timeout_ms = 1000)]
pub struct OpenFile {
    /// The file path in the file system.
    #[at_arg(position = 1)]
    pub filename: String<80>,
    /// The open mode of the file.
    #[at_arg(position = 2)]
    pub mode: Option<FileOpenMode>,
}

/// `AT+QFCLOSE` — closes a filehandle opened with [`OpenFile`].
#[derive(Clone, AtatCmd)]
#[at_cmd("+QFCLOSE", NoResponse, timeout_ms = 1000)]
pub struct CloseFile {
    /// The handle of the file to be closed.
    #[at_arg(position = 1)]
    pub filehandle: u32,
}

/// Extracts the binary payload from an `AT+QFREAD` response
/// (`CONNECT <read_length>\r\n<binary_data>`), which doesn't fit atat's
/// normal comma-separated response parsing since the payload can contain
/// arbitrary bytes. Used as [`ReadFile`]'s `#[at_cmd(parse = ...)]` override
/// — everything else about the command (its request formatting) still goes
/// through the regular derive.
fn parse_read_file(resp: &[u8]) -> Result<FileReadStarted, ()> {
    const CONNECT: &[u8] = b"CONNECT ";
    let start = resp
        .windows(CONNECT.len())
        .position(|w| w == CONNECT)
        .ok_or(())?
        + CONNECT.len();
    let after = &resp[start..];
    let crlf = after.windows(2).position(|w| w == b"\r\n").ok_or(())?;
    let claimed_len: usize = core::str::from_utf8(&after[..crlf])
        .map_err(|_| ())?
        .trim()
        .parse()
        .map_err(|_| ())?;
    let binary = &after[crlf + 2..];
    let take = claimed_len.min(binary.len()).min(256);
    let mut data = Bytes::<256>::new();
    data.extend_from_slice(&binary[..take]).map_err(|_| ())?;
    Ok(FileReadStarted {
        read_length: take as u32,
        data,
    })
}

/// `AT+QFREAD` — reads up to `length` bytes (256 max; omit for "as many as
/// fit in one response") from an already-open filehandle, starting at its
/// current position. The modem auto-advances the read position on each
/// call, so repeated calls without an explicit seek read through the file
/// sequentially; a short read (less than requested) means end of file. See
/// [`parse_read_file`] for how the non-standard `CONNECT`-prefixed response
/// is handled, and [`crate::driver::Bg9xModem::read_file`].
#[derive(Clone, AtatCmd)]
#[at_cmd("+QFREAD", FileReadStarted, timeout_ms = 2000, parse = parse_read_file)]
pub struct ReadFile {
    /// The handle of the file to be operated.
    #[at_arg(position = 1)]
    pub filehandle: u32,
    /// The length of the file to be read out. If omitted, reads as many
    /// bytes as fit in one response.
    #[at_arg(position = 2)]
    pub length: Option<u32>,
}

/// `AT+QFWRITE` — writes to an already-open filehandle. Like
/// [`FileUploadToInternalFlash`], the modem replies with `CONNECT` instead
/// of `OK`, so this command's own response is discarded rather than
/// propagated. See [`crate::driver::MqttModem::write_file`].
#[derive(Clone, AtatCmd)]
#[at_cmd("+QFWRITE", NoResponse, timeout_ms = 5000)]
pub struct WriteFile {
    /// The handle of the file to be operated.
    #[at_arg(position = 1)]
    pub filehandle: u32,
    /// The length of the file to be written.
    #[at_arg(position = 2)]
    pub length: u32,
    /// Seconds to wait for data. 1-65535, default 5.
    #[at_arg(position = 3)]
    pub timeout: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::parse_read_file;

    #[test]
    fn parses_connect_response_with_binary_payload() {
        let mut resp = b"CONNECT 5\r\n".to_vec();
        resp.extend_from_slice(b"\x00\x01\xff\r\n");
        resp.extend_from_slice(b"\r\nOK\r\n");

        let parsed = parse_read_file(&resp).unwrap();
        assert_eq!(parsed.read_length, 5);
        assert_eq!(&parsed.data[..], b"\x00\x01\xff\r\n");
    }

    #[test]
    fn caps_at_256_bytes_even_if_modem_claims_more() {
        let mut resp = b"CONNECT 300\r\n".to_vec();
        resp.extend(core::iter::repeat_n(b'a', 300));

        let parsed = parse_read_file(&resp).unwrap();
        assert_eq!(parsed.read_length, 256);
        assert_eq!(parsed.data.len(), 256);
    }

    #[test]
    fn short_read_reports_actual_bytes_available() {
        let mut resp = b"CONNECT 128\r\n".to_vec();
        resp.extend_from_slice(b"only 10 b\r\n"); // 11 bytes < claimed 128

        let parsed = parse_read_file(&resp).unwrap();
        assert_eq!(parsed.read_length, 11);
        assert_eq!(&parsed.data[..], b"only 10 b\r\n");
    }

    #[test]
    fn rejects_response_without_connect() {
        assert!(parse_read_file(b"\r\nERROR\r\n").is_err());
    }

    #[test]
    fn rejects_response_with_bare_connect_and_no_length() {
        // The `CONNECT\r\n` URC line for QFUPL/QFWRITE, not QFREAD's shape.
        assert!(parse_read_file(b"CONNECT\r\n").is_err());
    }
}
