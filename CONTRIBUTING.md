# Contributing

Thanks for considering a contribution to `quectel-bg9x-atat`. By
participating, you're expected to uphold our
[Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

For anything beyond a small fix (typo, doc tweak, obvious bug), please
open an issue first to discuss the change — especially for new AT
commands or behavior changes, since those touch the modem bring-up
sequence directly.

## Development setup

You'll need a stable Rust toolchain plus the target this crate is
validated against (Cortex-M0+, e.g. RP2040):

```sh
rustup component add rustfmt clippy
rustup target add thumbv6m-none-eabi
```

## Before opening a PR

Run the same checks CI runs:

```sh
cargo fmt --check
cargo clippy --target thumbv6m-none-eabi --no-default-features -- -D warnings
cargo check --target thumbv6m-none-eabi --no-default-features
cargo clippy --target thumbv6m-none-eabi --no-default-features --features defmt -- -D warnings
cargo check --target thumbv6m-none-eabi --no-default-features --features defmt
cargo test --no-default-features
```

All of the above must pass. `cargo fmt` (without `--check`) will fix
formatting for you.

## Scope

This crate is a bring-up-focused `atat` driver for the Quectel BG9x AT
command set: identity, SIM/network status, signal strength, and PDP
context. It's generic over any `atat::asynch::AtatClient` and doesn't own
a transport, UART, or modem power control — please keep contributions
that way; don't add a HAL/board dependency or hardcode a transport.

MQTT, SSL, GNSS, and internal-flash file management exist in the
reference implementation this crate was ported from (see `NOTICE.md`) but
aren't ported here yet. Adding one follows the same mechanical process as
the existing commands: a command struct + response struct in
`src/commands/`, wired into `src/driver.rs`.

## Adding or changing AT commands

Cite the relevant section of Quectel's BG95&BG96 AT Commands Manual in
your PR description, and note whether the change has been exercised
against real hardware or only checked against the manual.

## Commit / PR style

- Keep PRs focused on one change.
- Write commit messages that explain *why*, not just *what*.
- Add or update tests for any behavior change.

## Reporting bugs

See the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) —
opening an issue will offer it automatically.
