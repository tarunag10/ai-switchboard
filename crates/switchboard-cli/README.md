# Switchboard CLI crate

This crate is the standalone, cross-platform, read-only command surface for
shared AI Switchboard contracts. It does not replace the existing Node command
or package the desktop app in this phase.

## Commands

```bash
cargo run --manifest-path crates/switchboard-cli/Cargo.toml -- harness status
cargo run --manifest-path crates/switchboard-cli/Cargo.toml -- workbench session serialize < session.json
```

`harness status` is produced by `switchboard-runtime`'s fail-closed
`PortableRuntime`. Provider transport and process start must both remain false.

`workbench session serialize` reads one JSON document from standard input, with
a hard 1 MiB limit. It deserializes the shared `switchboard-core`
`WorkbenchSession`, rejects unknown fields, validates the content-free lifecycle
ledger and bounded RFC3339 session timestamps, and writes compact deterministic
JSON followed by one newline.

The serializer does not create or mutate sessions. Production code has no file,
child-process, network, provider, Tauri, keychain, or database dependency.

## Opt-in Node bridge

The package-level `switchboard` command remains the default Node Repo
Intelligence preview. To delegate only the native-capable commands to an
explicitly selected Rust binary, set `SWITCHBOARD_NATIVE_CLI` to that
executable path and add `--native`:

```bash
SWITCHBOARD_NATIVE_CLI=/absolute/path/to/switchboard \
  switchboard harness status --native
SWITCHBOARD_NATIVE_CLI=/absolute/path/to/switchboard \
  switchboard workbench session serialize --native < session.json
```

The bridge never guesses an installation path, invokes a shell, or forwards
`router` or `optimize` to Rust. If the variable is unset or unusable, it fails
closed with an actionable error.

## Verification

```bash
cargo fmt --manifest-path crates/switchboard-cli/Cargo.toml --check
cargo test --manifest-path crates/switchboard-cli/Cargo.toml --locked
cargo clippy --manifest-path crates/switchboard-cli/Cargo.toml --locked -- -D warnings
```
