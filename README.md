# quectel-bg9x-atat

Async, `no_std` [`atat`](https://crates.io/crates/atat) driver for Quectel
BG9x (BG95/BG96) cellular modems — LTE-M/NB-IoT/EGPRS, AT command based.

Built for [embassy](https://embassy.dev): the AT command layer generates
command/response encoding via `atat`'s derive macros, and the driver on top
is generic over any `atat::asynch::AtatClient`, so it doesn't care which
transport or `embedded-io-async` version backs it — wire that up in your
firmware and hand over the client.

Currently covers: identity (IMEI/ICCID/firmware version), SIM status,
network registration (`AT+CEREG`/`AT+CGREG`/`AT+QNWINFO`), signal strength,
PDP context setup/activation, MQTT (with optional SSL/TLS), and SSL/TLS
context configuration. GNSS and internal-flash file management from the
reference implementation are not yet ported — see `NOTICE.md`.

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
modem.configure_context(1, "your-apn", "", "", 0).await?;
let ctx = modem.activate_context(1).await?;
// ctx.ip_address is now Some(...)
```

### MQTT

MQTT responses arrive as URCs, not direct command replies, so those methods
live on a separate `MqttModem`, built by attaching a URC subscription from
the same `atat::UrcChannel` your `Ingress` task consumes:

```rust,ignore
use quectel_bg9x_atat::{Bg9xModem, SslConfig};

let mut modem = Bg9xModem::new(client).with_urc_subscription(urc_channel.subscribe()?);
// `modem` still derefs to the base Bg9xModem, so is_alive()/network_attach()/etc. all still work.

// Plain TCP:
modem.mqtt_connect(0, "broker.example.com", 1883, "my-client-id", None, None, None, Duration::from_secs(30)).await?;

// Or over TLS — configure the SSL context first (context IDs 0-5, independent
// of the PDP context ID), then pass its id as ssl_ctx_id:
modem.configure_ssl_context(&SslConfig::new(2)).await?;
modem.mqtt_connect(0, "broker.example.com", 8883, "my-client-id", None, None, Some(2), Duration::from_secs(30)).await?;

modem.mqtt_publish(0, "my/topic", "hello", 1, Duration::from_secs(10)).await?;
modem.mqtt_disconnect(0, Duration::from_secs(10)).await?;
```

`SslConfig` defaults to TLS 1.2, all cipher suites, server-only auth, no
hostname check, and ignores certificate validity dates (no RTC/NTP time on
the module by default). Client/CA certificates referenced by filename must
already be present in the module's UFS file system — this crate doesn't
handle uploading them yet.

## Validated

Exercised end-to-end on a Sixfab Pico LTE board (RP2040 + Quectel BG95-M3):
SIM ready, ICCID/IMEI read, LTE-M (eMTC) registration, signal strength
query, and PDP context activation with a real assigned IP against a live
carrier SIM.

## Attribution

The AT command struct definitions in `src/commands/` were ported from
SC Robotics' `quectel-bg9x-eh-driver` (MIT) and rewritten for `no_std` async
use. See `NOTICE.md` for the full attribution and reproduced license notice.

## Contributing

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) for
development setup and the checks CI runs.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
