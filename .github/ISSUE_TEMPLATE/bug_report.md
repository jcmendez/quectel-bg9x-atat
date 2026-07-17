---
name: Bug report
about: Report incorrect modem behavior or a driver bug
title: ""
labels: bug
assignees: ""
---

## Description

A clear description of what's wrong.

## Environment

- Crate version:
- Rust toolchain (`rustc --version`):
- Target/MCU:
- `atat` version:
- Modem variant (BG95 / BG96) and firmware version, if known:

## Steps to reproduce

Minimal code to reproduce, e.g.:

```rust,ignore
let mut modem = Bg9xModem::new(client);
modem.wait_sim_ready(Duration::from_secs(10)).await?;
let iccid = modem.get_iccid().await?;
```

## Expected behavior

What you expected the driver/modem to do.

## Actual behavior

What actually happened — wrong parse, timeout, error returned, panic,
etc. If you have a capture of the raw AT command/response traffic (e.g.
from a UART sniffer or `atat`'s own logging), please attach it — it's the
most useful thing you can give us for parsing or protocol bugs.
