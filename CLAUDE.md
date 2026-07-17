# quectel-bg9x-atat

Async, `no_std` `atat` driver for Quectel BG9x (BG95/BG96) modems. See
`README.md` for usage and design rationale, `NOTICE.md` for attribution.
This file covers only what isn't obvious from the code or README.

## Version pinning gotcha

`atat` 0.24.1 pins `embedded-io-async = "0.6"` and `embassy-time = "0.4"`.
This crate's own `embassy-time = "0.5"` dependency (used for
`network_attach` polling and its public `Duration` timeout parameters)
deliberately does NOT try to match atat's internal 0.4 pin — verified safe
to coexist (the underlying `embassy-time-driver` unifies to one shared
instance regardless of which `embassy-time` major version sits on top of
it). Don't "fix" this by trying to align the two versions.

This crate intentionally does **not** depend on `embedded-io-async` at
all — that's the point of being generic over `AtatClient`. If a future
change adds such a dependency, make sure it doesn't accidentally
reintroduce a 3-way version split with whatever the consuming firmware
uses.
