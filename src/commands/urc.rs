//! Unsolicited result codes (URCs) the BG9x can emit outside of a
//! command/response exchange.
//!
//! Adapted from SC Robotics' `quectel-bg9x-eh-driver` (MIT) — see `/NOTICE.md`.

use atat::atat_derive::AtatUrc;

use super::responses::{
    CmeError, MqttCloseResponse, MqttConnectResponse, MqttDisconnectResponse, MqttOpenResponse,
    MqttPublishResponse, MqttStatusResponse, NtpTimeResponse,
};

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
    /// Result of an `AT+QNTP` NTP time sync (`GetNetworkNtpTime` command).
    #[at_urc("+QNTP")]
    NtpTime(NtpTimeResponse),
    /// Result of opening the MQTT network socket (`MqttOpen` command).
    #[at_urc("+QMTOPEN")]
    MqttOpen(MqttOpenResponse),
    /// Asynchronous MQTT connection status change (socket closed).
    #[at_urc("+QMTSTAT")]
    MqttStatus(MqttStatusResponse),
    /// Result of the MQTT CONNECT handshake (`MqttConnect` command).
    #[at_urc("+QMTCONN")]
    MqttConnect(MqttConnectResponse),
    /// Result of an MQTT publish (`MqttPublishExtended` command).
    #[at_urc("+QMTPUB")]
    MqttPublish(MqttPublishResponse),
    /// Result of an MQTT DISCONNECT (`MqttDisconnect` command).
    #[at_urc("+QMTDISC")]
    MqttDisconnect(MqttDisconnectResponse),
    /// Result of closing the MQTT network socket (`MqttClose` command).
    #[at_urc("+QMTCLOSE")]
    MqttClose(MqttCloseResponse),
}
