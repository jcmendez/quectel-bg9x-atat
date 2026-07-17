//! Enum argument types for BG9x AT commands.
//!
//! Adapted from SC Robotics' `quectel-bg9x-eh-driver` (MIT) — see `/NOTICE.md`.

use atat::atat_derive::AtatEnum;

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
