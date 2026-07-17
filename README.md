# quectel-bg9x-atat

Async, `no_std` [`atat`](https://crates.io/crates/atat) driver for Quectel
BG9x (BG95/BG96) cellular modems — LTE-M/NB-IoT/EGPRS, AT command based.

Built for [embassy](https://embassy.dev): the AT command layer generates
command/response encoding via `atat`'s derive macros, and the driver on top
is generic over any `atat::asynch::AtatClient`, so it doesn't care which
transport or `embedded-io-async` version backs it — wire that up in your
firmware and hand over the client.

Currently covers the bring-up subset of the BG9x AT command set: identity
(IMEI/ICCID/firmware version), SIM status, network registration
(`AT+CEREG`/`AT+CGREG`/`AT+QNWINFO`), signal strength, and PDP context
setup/activation. MQTT, SSL, GNSS, and internal-flash file management from
the reference implementation are not yet ported — see `NOTICE.md`.

## Usage

This crate doesn't own your UART or the modem's power control (PWRKEY/STATUS
pins) — both are board-specific. Set those up yourself, build an
`atat::asynch::Client` over your UART, then hand it to `Bg9xModem`:

```rust,ignore
use quectel_bg9x_atat::{Bg9xModem, Urc};

// ... construct an atat::asynch::Client<YourUart, INGRESS_BUF_SIZE> as `client`,
// with an `atat::Ingress` running in its own task consuming `Urc` ...

let mut modem = Bg9xModem::new(client);
modem.wait_sim_ready(Duration::from_secs(10)).await?;
let iccid = modem.get_iccid().await?;
let rat = modem.network_attach(Duration::from_secs(60)).await?;
modem.configure_context(1, "iot.sixfab.com", "", "", 0).await?;
let ctx = modem.activate_context(1).await?;
// ctx.ip_address is now Some(...)
```

## Attribution

The AT command struct definitions in `src/commands/` were ported from
SC Robotics' `quectel-bg9x-eh-driver` (MIT) and rewritten for `no_std` async
use. See `NOTICE.md` for the full attribution and reproduced license notice.
