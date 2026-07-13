# Mac AI Switchboard Architecture

Mac AI Switchboard is a small Tauri desktop shell with a local daemon-oriented backend built around the Headroom engine.

- `src/`: Tauri frontend tray UI, onboarding surfaces, Switchboard controls, Addons, Doctor, live activity, and local-first account gating.
- `src-tauri/src/lib.rs`: Tauri command entrypoints, tray wiring, Switchboard mode orchestration, Doctor report construction, and release/update commands.
- `src-tauri/src/state.rs`: top-level application state and dashboard shaping.
- `src-tauri/src/tool_manager.rs`: bootstrap, runtime, and tool installation boundary.
- `src-tauri/src/client_adapters.rs`: client detection, managed Claude Code/Codex setup, managed connector registry, and reversible config edits.
- `src-tauri/src/repo_intelligence.rs`: read-only local repo indexing, context-pack summaries, persisted latest summary, and clear-index cleanup.
- `src-tauri/src/insights.rs`: daily local recommendation generation.
- `research/tool-compatibility-matrix.md`: v1 inclusion gate for external tools.

## Bootstrap Strategy

The downloadable app stays small: it ships the Tauri shell, Rust backend, and installer logic. Third-party Python components are fetched on first launch into Mac AI Switchboard's managed application support directory.

The packaged app identity is `Mac AI Switchboard`, and new runtime storage lives under `~/Library/Application Support/Mac AI Switchboard`. On first launch after an older build, the app copies legacy `~/Library/Application Support/Headroom` storage into the current app storage path, records `config/migrations.json`, and preserves the legacy directory for one-release compatibility with existing runtimes, logs, receipts, backups, cleanup paths, and reversible client setup state.

## V1 Boundaries

- macOS is the polished v1 target.
- `headroom` is required for proxy compression.
- `rtk` is required for local shell-output compression.
- Managed tools may be Python-based or standalone binaries, but Headroom owns their install path.
- Client configuration changes require explicit user consent and rollback support.
- Managed connectors are visible and repairable. Claude Code, Codex, Gemini CLI, OpenCode, Windsurf, Zed AI, Goose, and Grok / xAI CLI have managed setup/verify/repair coverage for their proven routing surfaces; Goose's native writes are limited to documented OpenAI/Anthropic endpoint fields plus the read-only Repo Memory MCP bridge, and Grok's to `endpoints.models_base_url`. Cursor, Aider, Continue, Qwen Code, and Amazon Q Developer CLI remain guided or detected until their native surfaces satisfy the same backup, restore, Doctor verification, rollback, and Off-mode cleanup contract. Each connector carries a readiness contract: config surfaces the app may inspect, safe Switchboard modes available today, automation gates, the current manual workflow, and the first safe automation step.
- Repo Intelligence is read-only in the current app: it scans local repo metadata, estimates tokens, builds bounded context packs, persists the latest summary in managed app storage, exposes Doctor warnings for stale or missing indexes, and lets users clear the saved index. It now surfaces dependency hubs, path-based import/dependency edges, content-derived import references, lightweight call references, reverse dependency hubs, and a bounded symbol graph alongside directories, languages, entrypoints, tests, and config hubs. The app exposes read-only local query commands for manifest, pack, handoff, freshness, symbol, and dependent lookups, plus a repo-memory MCP stdio transport guarded by read-only smoke tests and app-session install/start/stop controls. It does not yet provide a full AST call graph, persistent parser index, or long-running MCP daemon supervision.

## Managed Connector Architecture

Managed connectors are intentionally split into three layers:

- Backend detection in `src-tauri/src/client_adapters.rs` looks for local binaries and known config surfaces without writing files.
- Frontend contract data in `src/lib/plannedConnectors.ts` explains setup phase, safe modes, readiness stages, automation gates, safety badges, native config gate state, and rollback strategy.
- Doctor and Settings render those contracts as manual evidence until a connector has dry-run diff, backup, apply, verify, rollback, and Off cleanup coverage.

Gemini CLI, OpenCode, Windsurf, and Zed AI are the promoted native-routing surfaces beyond Claude Code and Codex. Gemini has managed shell base-url env routing, OpenCode has provider config routing, and Windsurf/Zed have editor settings routing, all behind the shared lifecycle. Goose is promoted only for the app-managed read-only Repo Memory MCP bridge. Remaining gated connectors stay manual while provider/editor native config mutation waits for model/account compatibility, fixture-home restore tests, Doctor verification, rollback, and Off cleanup.

## Repo Intelligence

Repo Intelligence exists to reduce repeated agent discovery work before large coding sessions. The current implementation has three local surfaces:

- CLI: `npm run switchboard -- repo-intelligence <repo-path>` prints file roles, rough token estimates, implementation, verification, and handoff packs. `npm run repo:intelligence -- <repo-path>` remains the compatibility path. `--manifest` emits an agent-readable JSON contract with pack ids, commands, graph availability, symbol counts, symbol edges, estimated token savings, read-only API query names, and safety flags.
  Agents can discover supported handoff ids with `npm run repo:intelligence -- <repo-path> --list-agents`, discover local API query names with `npm run repo:intelligence -- <repo-path> --list-api`, request one task-specific Markdown pack with `npm run repo:intelligence -- <repo-path> --pack implementation --format markdown`, or request a ready-to-paste Markdown handoff with `npm run repo:intelligence -- <repo-path> --agent codex --format markdown`. For automation and MCP-style adapters, `npm run repo:intelligence -- <repo-path> --agent gemini --format json` emits a read-only `mac_ai_switchboard.repo_agent_handoff` payload with selected files, graph hints, token estimates, safety flags, and managed connector config readiness when the target has a managed connector dossier. Supported handoff targets are `claude`, `codex`, `gemini`, `opencode`, `aider`, `goose`, `cursor`, `continue`, `grok`, `qwen`, `amazonq`, `windsurf`, and `zed`.
  The app-side read-only query contract is discoverable as `apiQueries` in the manifest: `get_repo_manifest`, `get_repo_pack`, `get_agent_handoff`, `get_index_freshness`, `clear_repo_index`, `search_repo_intelligence_symbols`, and `get_repo_intelligence_dependents`. `get_agent_handoff` returns connector readiness fields for managed connector targets, including next gate, evidence requirements, config path strategy, account caveat, and rollback strategy. `clear_repo_index` only removes Switchboard's managed saved index metadata; it does not write into the indexed repository. The repo-memory MCP stdio transport exposes read-only `repo_context_pack`, `repo_symbol_lookup`, and `repo_dependents_of` tools; direct app start now verifies the read-only smoke contract before marking MCP active.
  Default packs exclude secret-like paths such as `.env*`, private-key folders, `.pem`, `.p8`, `.p12`, and certificate files.
- App UI: the dedicated Repo Intelligence sidebar view accepts a local repo path, runs the read-only backend indexer, shows compact context packs and symbol graph signals, supports combined and individual pack copy, reloads the latest summary on launch, and can clear the saved summary. The Addons card is a status and entry point, not the main workflow.
- Doctor: stale, missing, moved, corrupt, or parser/index-health-mismatched saved repo indexes appear as **Clear index** automatic cleanup items. **Repair all** may clear only Switchboard-managed saved index metadata; choosing a replacement repo path and re-indexing remain deliberate Repo Intelligence actions.

The saved summary lives under managed app storage:

```text
~/Library/Application Support/Mac AI Switchboard/config/repo-intelligence-latest.json
```

User repositories are not modified.

## Remaining Architecture Work

- Persist file hashes, parser versions, symbols, imports, routes, package scripts, likely tests, and freshness metadata.
- Add a proper repo picker and richer index status UI.
- Promote the existing repo-memory MCP transport into deeper connector bridge docs and, if needed, long-running service supervision for MCP-aware agents.
- Add explicit cleanup/off-mode behavior for any future Repo Intelligence hooks.
- Keep Rust formatting and desktop tests in release/CI gates; local shells without `cargo` must rely on CI for backend validation.
