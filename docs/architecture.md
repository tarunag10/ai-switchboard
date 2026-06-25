# Mac AI Switchboard Architecture

Mac AI Switchboard is split into a small desktop shell and a local daemon-oriented backend built on the Headroom engine:

- `src/`: Tauri frontend for the tray UI, onboarding surfaces, live activity, and research visibility.
- `src-tauri/src/lib.rs`: Tauri command entrypoints and tray wiring.
- `src-tauri/src/state.rs`: top-level application state and dashboard shaping.
- `src-tauri/src/tool_manager.rs`: bootstrap/runtime/tool installation boundary.
- `src-tauri/src/client_adapters.rs`: client detection and guided setup contract.
- `src-tauri/src/pipeline.rs`: request-stage summary model for prompt optimization flows.
- `src-tauri/src/insights.rs`: daily local recommendation generation.
- `research/tool-compatibility-matrix.md`: v1 inclusion gate for external tools.

## Bootstrap strategy

The downloadable app stays small because it ships only the Tauri shell, Rust daemon, and installer logic. Third-party Python components are fetched after first launch into a Headroom-managed application support directory.

The packaged app identity is `Mac AI Switchboard`, but runtime storage intentionally remains under `~/Library/Application Support/Headroom` for now. That preserves existing managed Python runtimes, logs, receipts, backups, and cleanup paths until a dedicated migration can copy and verify state safely.

## v1 boundaries

- macOS is the only polished target for v1.
- `headroom` is required.
- `rtk` is required.
- `vitals` is included as the primary scanner.
- Managed tools may be Python-based or standalone binaries when Headroom owns the install path.
- Client configuration changes require explicit user consent and rollback support.
- Repo Intelligence is planned as a local-first Graphy-style code graph, symbol index, and repo memory layer. First implementation should be read-only context planning; any write or auto-repair action must remain explicit.
