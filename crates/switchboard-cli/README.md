# Switchboard CLI crate

This crate is the standalone, cross-platform, read-only Rust command surface
for shared AI Switchboard contracts. It does not replace the existing Node
package command or package the desktop app in this phase. The native binary is
source-built for local use and CI validation; no native CLI installer or
release artifact is currently shipped.

## Commands

```bash
cargo run --manifest-path crates/switchboard-cli/Cargo.toml -- harness status
cargo run --manifest-path crates/switchboard-cli/Cargo.toml -- router endpoint plan < endpoint-plan.json
cargo run --manifest-path crates/switchboard-cli/Cargo.toml -- workbench session serialize < session.json
```

`harness status` is produced by `switchboard-runtime`'s fail-closed
`PortableRuntime`. Provider transport and process start must both remain false.

`router endpoint plan` reads one bounded, content-free endpoint request from
standard input and writes a deterministic observe-only route plan. It does not
start a provider process or send provider traffic.

`workbench session serialize` reads one JSON document from standard input, with
a hard 1 MiB limit. It deserializes the shared `switchboard-core`
`WorkbenchSession`, rejects unknown fields, validates the content-free lifecycle
ledger and bounded RFC3339 session timestamps, and writes compact deterministic
JSON followed by one newline.

The serializer does not create or mutate sessions. Production code has no file,
child-process, network, provider, Tauri, keychain, or database dependency.

## Opt-in Node bridge

The package-level `switchboard` command remains the default Node Repo
Intelligence preview. Its `router <repo-path>` and `optimize <repo-path>`
commands remain Node planning aliases. To delegate one of the native-capable
commands to an explicitly selected Rust binary, set `SWITCHBOARD_NATIVE_CLI`
to that executable path and add `--native`:

```bash
SWITCHBOARD_NATIVE_CLI=/absolute/path/to/switchboard \
  switchboard harness status --native
SWITCHBOARD_NATIVE_CLI=/absolute/path/to/switchboard \
  switchboard workbench session serialize --native < session.json
SWITCHBOARD_NATIVE_CLI=/absolute/path/to/switchboard \
  switchboard router endpoint plan --native < endpoint-plan.json
```

The bridge never guesses an installation path or invokes a shell. Only
`harness status --native`, `workbench session serialize --native`, and the
exact `router endpoint plan --native` shape are delegated. Legacy
`router <repo-path> --native` shapes and all `optimize --native` shapes are
rejected. If the variable is unset or unusable, the bridge fails closed with an
actionable error.

Linux and Windows native CLI support is source/CI-supported only at present.
There is no native installer or prebuilt platform artifact; build the Rust
crate locally with the commands above.

## Verification

```bash
cargo fmt --manifest-path crates/switchboard-cli/Cargo.toml --check
cargo test --manifest-path crates/switchboard-cli/Cargo.toml --locked
cargo clippy --manifest-path crates/switchboard-cli/Cargo.toml --locked -- -D warnings
```
