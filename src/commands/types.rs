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

/// Which time value to return (`AT+QLTS`).
#[derive(Debug, Clone, Copy, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum NitzTimeQueryMode {
    /// Latest time synced through the network
    LatestSyncedTime = 0,
    /// Current GMT time calculated from that synced time
    CurrentGmtTime = 1,
    /// Current local time calculated from that synced time
    CurrentLocalTime = 2,
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

/// When an `AT+QCFG` radio-configuration change takes effect.
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum ConfigurationEffect {
    /// Takes effect only after the next reboot.
    AfterReboot = 0,
    /// Takes effect immediately (and is saved).
    Immediately = 1,
}

/// RAT search mode (`AT+QCFG="nwscanmode"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum RatSearchingMode {
    /// GSM and LTE (default)
    Automatic = 0,
    GsmOnly = 1,
    LteOnly = 3,
}

/// Service domain to register on (`AT+QCFG="servicedomain"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum ServiceDomain {
    PsOnly = 1,
    CsAndPs = 2,
}

/// Network category searched for under LTE RAT (`AT+QCFG="iotopmode"`).
#[derive(Copy, Clone, Debug, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum IotOperationMode {
    Emtc = 0,
    NbIot = 1,
    EmtcAndNbIot = 2,
}

/// A radio access technology, for building the `AT+QCFG="nwscanseq"` search
/// order with [`build_rat_search_order`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SearchRat {
    Gsm,
    Emtc,
    NbIot,
}

impl SearchRat {
    fn code(self) -> &'static str {
        match self {
            SearchRat::Gsm => "01",
            SearchRat::Emtc => "02",
            SearchRat::NbIot => "03",
        }
    }
}

/// Raw `<scanseq>` argument for `ConfigureRatSearchingSequence`.
pub type RatSearchOrder = Bytes<8>;

/// [`build_rat_search_order`]'s RAT list was empty, had more than 3 entries,
/// or contained a duplicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidRatOrder;

/// Builds the `<scanseq>` string for `AT+QCFG="nwscanseq"` from an ordered,
/// duplicate-free list of 1-3 RATs, e.g. `[Emtc, NbIot, Gsm]` -> `"020301"`
/// (eMTC, then NB-IoT, then GSM).
pub fn build_rat_search_order(order: &[SearchRat]) -> Result<RatSearchOrder, InvalidRatOrder> {
    if order.is_empty() || order.len() > 3 {
        return Err(InvalidRatOrder);
    }
    let mut bytes = RatSearchOrder::new();
    for (i, &rat) in order.iter().enumerate() {
        if order[..i].contains(&rat) {
            return Err(InvalidRatOrder);
        }
        for b in rat.code().bytes() {
            bytes.push(b).map_err(|_| InvalidRatOrder)?;
        }
    }
    Ok(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_rat_search_order() {
        assert_eq!(
            build_rat_search_order(&[SearchRat::Emtc, SearchRat::NbIot, SearchRat::Gsm])
                .unwrap()
                .as_slice(),
            b"020301"
        );
        assert_eq!(
            build_rat_search_order(&[SearchRat::Gsm])
                .unwrap()
                .as_slice(),
            b"01"
        );
    }

    #[test]
    fn rejects_empty_order() {
        assert_eq!(build_rat_search_order(&[]), Err(InvalidRatOrder));
    }

    #[test]
    fn rejects_too_many_entries() {
        assert_eq!(
            build_rat_search_order(&[
                SearchRat::Gsm,
                SearchRat::Emtc,
                SearchRat::NbIot,
                SearchRat::Gsm
            ]),
            Err(InvalidRatOrder)
        );
    }

    #[test]
    fn rejects_duplicate_entries() {
        assert_eq!(
            build_rat_search_order(&[SearchRat::Gsm, SearchRat::Gsm]),
            Err(InvalidRatOrder)
        );
    }
}
