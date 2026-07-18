# Changelog

## Unreleased

### Added

- `Bg9xModem::get_version_info` (`AT+QGMR`) — the `GetVersionInfo` command
  struct existed but was never wired up, despite the README/CHANGELOG
  claiming firmware version was already covered under "identity".

## 0.1.1

Everything below was found and fixed against real hardware (Sixfab Pico LTE,
RP2040 + BG95-M3) while bringing up a mutual-TLS MQTT deployment against AWS
IoT Core in `sixfabpico-embassy`.

### Added

- UFS file management: `upload_file`/`write_file`/`read_file`/`open_file`/
  `close_file`/`delete_file`/`list_files` (`AT+QFUPL`/`QFWRITE`/`QFREAD`/
  `QFOPEN`/`QFCLOSE`/`QFDEL`/`QFLST`) — enough to get a CA or client
  certificate onto the module for cert-pinned/mutual-auth TLS.
- Radio/network config: RAT search order/mode, service domain, IoT
  operation mode, band masks, factory reset (`AT+QCFG`, `AT&F`).
- `AT+QLTS`/`AT+QNTP` network time sync (`Bg9xModem::get_nitz_time`,
  `MqttModem::ntp_sync`).
- `MqttModem::mqtt_close` (`AT+QMTCLOSE`) — force-closes the underlying TCP
  socket regardless of MQTT-level state, unlike `mqtt_disconnect`. Needed to
  recover a `tcp_connect_id` a prior process left "occupied" (see Fixed,
  below) — call it best-effort before `mqtt_connect` if your device can
  restart ungracefully while the modem stays powered.
- `mqtt_publish` gained a `retain` parameter (was hardcoded to non-retained).

### Fixed

- **`AT+QNTP` reports a 4-digit year, not the 2-digit year the AT command
  manual documents for both `AT+QLTS` and `AT+QNTP`.** Confirmed on
  hardware: a live `+QNTP` URC's timestamp read
  `"2026/07/18,16:32:18-20"` — every real NTP sync was failing to parse.
  `parse_timestamp` now detects which width was used from where the first
  `/` lands and accepts either.
- **`upload_file` sometimes reports `OperationTimeout` after a transfer
  that actually succeeded.** The file lands correctly on UFS but the
  `+QFUPL` completion URC isn't always observed in time. Not fixed at the
  driver level yet — tracked as
  [#5](https://github.com/jcmendez/quectel-bg9x-atat/issues/5); callers
  should treat an `upload_file` error as non-fatal if a follow-up
  `list_files` shows the file present with the expected size (see
  `sixfabpico-embassy`'s `ensure_file_on_modem` for the pattern).
- `activate_context`/`mqtt_disconnect` hardening against real hardware
  responses (see 0.1.0 below, carried a fix forward early in this cycle).
- `ModemError::TimeParseFailed` now carries the raw string that failed to
  parse, instead of being a bare unit variant — there was no way to see
  what the modem actually sent. This drops `ModemError`'s `Copy` derive
  (a `String<32>` payload isn't `Copy`); nothing in this crate relied on it.

### Why `mqtt_close` matters beyond this one bug

The underlying scenario — modem power survives a restart (RP2040 reflash,
crash, brownout) that never called `mqtt_disconnect`/`mqtt_close` first,
so the modem keeps `tcp_connect_id` marked occupied and the next
`mqtt_connect`'s `AT+QMTOPEN` fails with `MqttRequestFailed(2)` ("MQTT
identifier is occupied") — isn't a dev-cycle artifact. Any device whose
modem is on a separate, longer-lived power rail than its MCU (true of this
board, and probably most cellular designs) can hit it in the field from an
unexpected MCU reset alone.

## 0.1.0

Initial release: identity (IMEI/ICCID/firmware version), SIM status,
network registration (`AT+CEREG`/`AT+CGREG`/`AT+QNWINFO`), signal strength,
PDP context setup/activation, MQTT (with optional SSL/TLS), and SSL/TLS
context configuration. `activate_context` and `mqtt_disconnect` were fixed
against real hardware responses before this tag (see git history) — the
first case where this crate's initial best-guess parsing/protocol-flow
assumptions turned out to differ from what the module actually does, a
pattern that continued into 0.1.1 above.
