//! Unsolicited result codes (URCs) the BG9x can emit outside of a
//! command/response exchange.
//!
//! Adapted from SC Robotics' `quectel-bg9x-eh-driver` (MIT) — see `/NOTICE.md`.

use atat::atat_derive::AtatUrc;

use super::responses::CmeError;

#[derive(Clone, AtatUrc, Debug)]
pub enum Urc {
    /// Module application processor ready.
    #[at_urc("APP RDY")]
    AppReady,
    /// Module ready (also seen at boot on some firmware).
    #[at_urc("RDY")]
    Ready,
    /// `AT+QPOWD` completed.
    #[at_urc("POWERED DOWN")]
    PowerDown,
    /// Mobile-equipment/network error, e.g. surfaced instead of a normal
    /// response to `AT+CPIN?` when no SIM is inserted.
    #[at_urc("+CME ERROR")]
    CmeError(CmeError),
}
