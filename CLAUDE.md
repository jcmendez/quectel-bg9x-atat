# quectel-bg9x-atat

Async, `no_std` `atat` driver for Quectel BG9x (BG95/BG96) modems. See
`README.md` for usage and `NOTICE.md` for attribution. This file is
context for future work that isn't obvious from the code.

## Design decisions

- **Generic over `atat::asynch::AtatClient`, not tied to any transport.**
  `Bg9xModem<C>` only knows how to send/parse AT commands; it doesn't own a
  UART or know about `embedded-io-async` at all. The caller builds the
  `atat::asynch::Client` (over whatever transport, with whatever
  `embedded-io-async` version) and hands it over. This keeps the crate
  decoupled from any specific HAL/board's dependency versions.
- **Doesn't own modem power control.** PWRKEY/STATUS pins are board-specific
  GPIO wiring — that stays in the firmware repo that uses this crate.
- **Bring-up subset only, for now.** Ported from SC Robotics'
  `quectel-bg9x-eh-driver` (see `NOTICE.md`): identity (IMEI/ICCID/firmware
  version), SIM status, network registration (CEREG/CGREG/QNWINFO), signal
  strength, PDP context. MQTT+SSL, GNSS, and internal-flash file management
  exist in the reference implementation but aren't ported yet — same
  mechanical process (command struct + response struct + wire into
  `driver.rs`) would apply if/when needed.

## Version pinning

`atat` 0.24.1 pins `embedded-io-async = "0.6"` and `embassy-time = "0.4"`.
This crate's own `embassy-time = "0.5"` dependency (used for `network_attach`
polling and its public `Duration` timeout parameters) deliberately does NOT
try to match atat's internal 0.4 pin — verified safe to coexist (the
underlying `embassy-time-driver` unifies to one shared instance regardless of
which `embassy-time` major version sits on top of it). Consumers of this
crate should generally be on `embassy-time 0.5.x` to pass `Duration` values
into this crate's public API without conversion.

This crate intentionally does **not** depend on `embedded-io-async` at all —
that's the point of being generic over `AtatClient`. If a future change adds
such a dependency, make sure it doesn't accidentally reintroduce a 3-way
version split with whatever the consuming firmware uses.

## Validation

Exercised end-to-end on a Sixfab Pico LTE board (RP2040 + Quectel BG95-M3)
via the sibling `sixfabpico-embassy` firmware repo: SIM ready, ICCID/IMEI
read, LTE-M (eMTC) network registration, signal strength query, and PDP
context activation with a real assigned IP, all against Sixfab's live
connectivity service. See that repo's `CLAUDE.md`/README for the
board-specific details (APN, pin mapping, etc.) — this crate itself has no
board dependency.
