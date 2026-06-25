# Mac AI Switchboard Architecture

Mac AI Switchboard is a small Tauri desktop shell with a local daemon-oriented backend built around the Headroom engine.

- `src/`: Tauri frontend tray UI, onboarding surfaces, Switchboard controls, Addons, Doctor, live activity, and local-first account gating.
- `src-tauri/src/lib.rs`: Tauri command entrypoints, tray wiring, Switchboard mode orchestration, Doctor report construction, and release/update commands.
- `src-tauri/src/state.rs`: top-level application state and dashboard shaping.
- `src-tauri/src/tool_manager.rs`: bootstrap, runtime, and tool installation boundary.
- `src-tauri/src/client_adapters.rs`: client detection, managed Claude Code/Codex setup, planned connector registry, and reversible config edits.
- `src-tauri/src/repo_intelligence.rs`: read-only local repo indexing, context-pack summaries, persisted latest summary, and clear-index cleanup.
- `src-tauri/src/insights.rs`: daily local recommendation generation.
- `research/tool-compatibility-matrix.md`: v1 inclusion gate for external tools.

## Bootstrap Strategy

The downloadable app stays small: it ships the Tauri shell, Rust backend, and installer logic. Third-party Python components are fetched on first launch into the Headroom-managed application support directory.

The packaged app identity is `Mac AI Switchboard`, but runtime storage intentionally remains under `~/Library/Application Support/Headroom` for now. That preserves existing managed Python runtimes, logs, receipts, backups, cleanup paths, and reversible client setup state until a dedicated migration can copy and verify state safely.

## V1 Boundaries

- macOS is the polished v1 target.
- `headroom` is required for proxy compression.
- `rtk` is required for local shell-output compression.
- Managed tools may be Python-based or standalone binaries, but Headroom owns their install path.
- Client configuration changes require explicit user consent and rollback support.
- Planned connectors are visible and guided, but automatic setup stays disabled until backup, restore, and off-mode cleanup are implemented per tool.
- Repo Intelligence is read-only in the current app: it scans local repo metadata, estimates tokens, builds bounded context packs, persists the latest summary in managed app storage, exposes Doctor warnings for stale or missing indexes, and lets users clear the saved index. It does not yet provide a full Graphy-style symbol graph, call graph, dependency graph, or agent-facing context-pack API.

## Repo Intelligence

Repo Intelligence exists to reduce repeated agent discovery work before large coding sessions. The current implementation has three local surfaces:

- CLI: `npm run repo:intelligence -- <repo-path>` prints file roles, rough token estimates, and implementation, verification, and handoff packs.
- App UI: the Repo Intelligence add-on card accepts a local repo path, runs the read-only backend indexer, shows compact context packs, reloads the latest summary on launch, and can clear the saved summary.
- Doctor: stale or missing saved repo indexes appear as manual warnings and are excluded from **Repair all**.

The saved summary lives under managed app storage:

```text
~/Library/Application Support/Headroom/config/repo-intelligence-latest.json
```

User repositories are not modified.

## Remaining Architecture Work

- Persist file hashes, parser versions, symbols, imports, routes, package scripts, likely tests, and freshness metadata.
- Add a proper repo picker and richer index status UI.
- Expose read-only context packs through a local CLI/MCP-style surface for Claude Code, Codex, Gemini CLI, OpenCode, Aider, Goose, and similar tools.
- Add explicit cleanup/off-mode behavior for any future Repo Intelligence hooks.
- Keep Rust formatting and desktop tests in release/CI gates; local shells without `cargo` must rely on CI for backend validation.
