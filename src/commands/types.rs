//! Enum argument types for BG9x AT commands.
//!
//! Adapted from SC Robotics' `quectel-bg9x-eh-driver` (MIT) — see `/NOTICE.md`.

use atat::atat_derive::AtatEnum;
use atat::heapless_bytes::Bytes;

/// Echo on/off (`ATE`).
#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum EchoOn {
    /// Unit does not echo the characters in command mode
    Off = 0,
    /// Unit echoes the characters in command mode (default)
    On = 1,
}

/// Functionality level of the UE (`AT+CFUN`).
#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum FunctionalityLevelOfUE {
    /// Minimum functionality
    Minimum = 0,
    /// Full functionality (default)
    Full = 1,
    /// Disable modem both transmit and receive RF circuits
    DisableRF = 4,
}

/// Power-down mode (`AT+QPOWD`).
#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum PowerDownMode {
    /// Immediately power down
    Immediate = 0,
    /// Normal power down (default)
    Normal = 1,
}

/// MQTT protocol version (`AT+QMTCFG="version"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum MqttVersion {
    /// MQTT 3.1 (default)
    V3_1 = 3,
    /// MQTT 3.1.1
    V3_1_1 = 4,
}

/// Whether an MQTT socket uses SSL/TLS (`AT+QMTCFG="ssl"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum MqttSslEnable {
    /// Plain TCP (default)
    False = 0,
    /// SSL/TLS
    True = 1,
}

/// SSL/TLS protocol version (`AT+QSSLCFG="sslversion"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SslVersion {
    Ssl3_0 = 0,
    Tls1_0 = 1,
    Tls1_1 = 2,
    Tls1_2 = 3,
    All = 4,
}

/// SSL authentication/security level (`AT+QSSLCFG="seclevel"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SslAuthenticationMode {
    /// No authentication
    None = 0,
    /// Verify the server's certificate only
    ServerOnly = 1,
    /// Mutual authentication (server verifies client too, if requested)
    Mutual = 2,
}

/// Whether to skip certificate validity-period checks
/// (`AT+QSSLCFG="ignorelocaltime"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SslIgnoreLocalTime {
    /// Enforce certificate not-before/not-after checks
    Care = 0,
    /// Skip them (useful when the module has no RTC/NTP time yet)
    Ignore = 1,
}

/// Server Name Indication toggle (`AT+QSSLCFG="sni"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SslSniEnable {
    Disable = 0,
    Enable = 1,
}

/// Hostname validation toggle (`AT+QSSLCFG="checkhost"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SslCheckHostEnable {
    Disable = 0,
    Enable = 1,
}

/// Raw hex-string cipher suite argument for `AT+QSSLCFG="ciphersuite"`, e.g.
/// `"0xFFFF"`. Build one with [`SslCipherSuiteEnum::to_bytes`].
pub type SslCipherSuites = Bytes<6>;

/// Named TLS cipher suites the BG9x supports selecting by ID.
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u16)]
pub enum SslCipherSuiteEnum {
    TlsRsaWithAes256CbcSha = 0x0035,
    TlsRsaWithAes128CbcSha = 0x002F,
    TlsRsaWithRc4_128Sha = 0x0005,
    TlsRsaWithRc4_128Md5 = 0x0004,
    TlsRsaWith3desEdeCbcSha = 0x000A,
    TlsRsaWithAes256CbcSha256 = 0x003D,
    TlsEcdheRsaWithRc4_128Sha = 0xC011,
    TlsEcdheRsaWith3desEdeCbcSha = 0xC012,
    TlsEcdheRsaWithAes128CbcSha = 0xC013,
    TlsEcdheRsaWithAes256CbcSha = 0xC014,
    TlsEcdheRsaWithAes128CbcSha256 = 0xC027,
    TlsEcdheRsaWithAes256CbcSha384 = 0xC028,
    TlsEcdheRsaWithAes128GcmSha256 = 0xC02F,
    /// Let the module pick from everything it supports.
    SupportAll = 0xFFFF,
}

impl SslCipherSuiteEnum {
    /// Encode as the `"0xNNNN"` hex-string argument `AT+QSSLCFG="ciphersuite"`
    /// expects. `no_std`-safe (no `format!`/allocator needed).
    pub fn to_bytes(self) -> SslCipherSuites {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let value = self as u16;
        let mut bytes = SslCipherSuites::new();
        let _ = bytes.push(b'0');
        let _ = bytes.push(b'x');
        for shift in [12u16, 8, 4, 0] {
            let nibble = ((value >> shift) & 0xF) as usize;
            let _ = bytes.push(HEX[nibble]);
        }
        bytes
    }
}
