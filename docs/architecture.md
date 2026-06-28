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
- Planned connectors are visible and guided, but automatic setup stays disabled until backup, restore, and off-mode cleanup are implemented per tool. Each planned connector carries a readiness contract: config surfaces the app may inspect, safe Switchboard modes available today, automation gates that must pass before writes are allowed, the current manual workflow, and the first safe automation step.
- Repo Intelligence is read-only in the current app: it scans local repo metadata, estimates tokens, builds bounded context packs, persists the latest summary in managed app storage, exposes Doctor warnings for stale or missing indexes, and lets users clear the saved index. It now surfaces dependency hubs, path-based import/dependency edges, content-derived import references, lightweight call references, reverse dependency hubs, and a bounded symbol graph alongside directories, languages, entrypoints, tests, and config hubs. The app exposes read-only local query commands for manifest, pack, handoff, freshness, symbol, and dependent lookups; it does not yet provide a full AST call graph or external MCP-style transport.

## Planned Connector Architecture

Planned connectors are intentionally split into three layers:

- Backend detection in `src-tauri/src/client_adapters.rs` looks for local binaries and known config surfaces without writing files.
- Frontend contract data in `src/lib/plannedConnectors.ts` explains setup phase, safe modes, readiness stages, automation gates, safety badges, and rollback strategy.
- Doctor and Settings render those contracts as manual evidence until a connector has dry-run diff, backup, apply, verify, rollback, and Off cleanup coverage.

Gemini CLI is the first detection-only connector slice. The backend reports the Gemini binary path, `gemini --version` output when available, detected `~/.gemini` or `~/.config/gemini` surfaces, and an explicit routing blocker. Settings turns that evidence into a compatibility report, and Doctor keeps the issue manual. Gemini remains `planned` and `guide`; Mac AI Switchboard must not write Gemini provider config or route Gemini traffic until model/account compatibility can be verified locally and every readiness stage is implemented.

## Repo Intelligence

Repo Intelligence exists to reduce repeated agent discovery work before large coding sessions. The current implementation has three local surfaces:

- CLI: `npm run repo:intelligence -- <repo-path>` prints file roles, rough token estimates, implementation, verification, and handoff packs. `--manifest` emits an agent-readable JSON contract with pack ids, commands, graph availability, symbol counts, symbol edges, estimated token savings, read-only API query names, and safety flags.
  Agents can discover supported handoff ids with `npm run repo:intelligence -- <repo-path> --list-agents`, discover local API query names with `npm run repo:intelligence -- <repo-path> --list-api`, request one task-specific Markdown pack with `npm run repo:intelligence -- <repo-path> --pack implementation --format markdown`, or request a ready-to-paste Markdown handoff with `npm run repo:intelligence -- <repo-path> --agent codex --format markdown`. For automation and future MCP-style adapters, `npm run repo:intelligence -- <repo-path> --agent gemini --format json` emits a read-only `mac_ai_switchboard.repo_agent_handoff` payload with selected files, graph hints, token estimates, and safety flags. Supported handoff targets are `claude`, `codex`, `gemini`, `opencode`, `aider`, `goose`, `cursor`, `continue`, `grok`, `qwen`, `amazonq`, `windsurf`, and `zed`.
  The app-side read-only query contract is discoverable as `apiQueries` in the manifest: `get_repo_manifest`, `get_repo_pack`, `get_agent_handoff`, `get_index_freshness`, `clear_repo_index`, `search_repo_intelligence_symbols`, and `get_repo_intelligence_dependents`. `clear_repo_index` only removes Switchboard's managed saved index metadata; it does not write into the indexed repository.
  Default packs exclude secret-like paths such as `.env*`, private-key folders, `.pem`, `.p8`, `.p12`, and certificate files.
- App UI: the dedicated Repo Intelligence sidebar view accepts a local repo path, runs the read-only backend indexer, shows compact context packs and symbol graph signals, supports combined and individual pack copy, reloads the latest summary on launch, and can clear the saved summary. The Addons card is a status and entry point, not the main workflow.
- Doctor: stale or missing saved repo indexes appear as manual warnings and are excluded from **Repair all**.

The saved summary lives under managed app storage:

```text
~/Library/Application Support/Headroom/config/repo-intelligence-latest.json
```

User repositories are not modified.

## Remaining Architecture Work

- Persist file hashes, parser versions, symbols, imports, routes, package scripts, likely tests, and freshness metadata.
- Add a proper repo picker and richer index status UI.
- Add an MCP-style transport around the existing read-only manifest/query contract for Claude Code, Codex, Gemini CLI, OpenCode, Aider, Goose, Qwen Code, Amazon Q Developer CLI, Cursor, Continue, Windsurf, Zed AI, and similar tools.
- Add explicit cleanup/off-mode behavior for any future Repo Intelligence hooks.
- Keep Rust formatting and desktop tests in release/CI gates; local shells without `cargo` must rely on CI for backend validation.
