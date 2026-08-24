# Codex probe helper protocol foundation

This standalone library defines AI Switchboard's bounded, preparation-only
protocol for a future separately signed Codex probe helper.

It is intentionally not a runnable helper. The crate has no binary, transport,
filesystem, process, network, provider, Tauri, app dependency, build script, or
bundle registration. Version 1 can validate only a no-process preparation frame
and produce a shape-consistent no-process response frame. It independently
recomputes the complete opaque host binding, request, and preparation-receipt
transcript and requires one canonical JSON wire encoding. That proves internal
consistency only: even a fully self-consistent transcript remains
unauthenticated. The response therefore keeps authentication, freshness, and
launch authority explicitly false. It cannot reserve execution, start Codex,
prove freshness or trust, or establish macOS sandbox enforcement.

The application does not depend on this crate yet. A future phase must add a
one-shot attempt claim, authenticated native transport, separately signed nested
helper, launch-time identity revalidation, fixed descriptor-owned payload lease,
real containment, and terminal execution receipts under a new protocol version.

Verification:

```bash
rtk cargo fmt --check --manifest-path src-tauri/codex-probe-helper/Cargo.toml
rtk cargo test --locked --manifest-path src-tauri/codex-probe-helper/Cargo.toml
rtk cargo clippy --locked --manifest-path src-tauri/codex-probe-helper/Cargo.toml --all-targets -- -D warnings
rtk cargo metadata --locked --no-deps --format-version 1 --manifest-path src-tauri/codex-probe-helper/Cargo.toml
rtk cargo deny --manifest-path src-tauri/codex-probe-helper/Cargo.toml --locked check
```
