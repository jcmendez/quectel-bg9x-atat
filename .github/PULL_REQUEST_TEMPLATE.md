## Summary

<!-- What does this change do, and why? -->

## Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --target thumbv6m-none-eabi --no-default-features -- -D warnings` passes (with and without `--features defmt`)
- [ ] `cargo check --target thumbv6m-none-eabi --no-default-features` passes (with and without `--features defmt`)
- [ ] `cargo test --no-default-features` passes
- [ ] Tests added/updated for any behavior change

## Evidence (only for new/changed AT commands)

<!--
If this PR adds or changes an AT command, include:
  - The relevant section of Quectel's BG95&BG96 AT Commands Manual.
  - Whether this was exercised against real hardware or only checked
    against the manual.

Delete this section if not applicable (e.g. pure refactor, doc fix).
-->

## Related issue

<!-- Closes #... -->
