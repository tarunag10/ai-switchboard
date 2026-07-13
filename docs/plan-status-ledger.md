# Plan Status Ledger

Updated: 2026-07-12

## AI Switchboard Platform Rebrand

Status: complete for the platform rebrand docs/evidence slice
Plan: `docs/ai-switchboard-platform-rebrand-implementation-plan.md`

Goal: move the product identity from Mac AI Switchboard to AI Switchboard / Switchboard while preserving macOS install compatibility, CLI visibility, cross-platform roadmap clarity, and accurate attribution for Headroom, RTK, Caveman, Ponytail, MarkItDown, and other integrated tools.

This is the current done/left ledger for the AI Switchboard roadmap, including the Repo Map/token-compression work, Fable security hardening, local evidence gates, and release-readiness work.

## Done

- Live Token X-Ray and Daily AI Usage Briefing are implemented: versioned local read models, deterministic normalization, source-confidence labels, X-Ray freshness/pressure/timeline/anomaly surfaces, daily briefing recommendations, secret-free Markdown/JSON export, 365-day local history, and scoped preview/clear controls. The analytics store is content-free and does not touch the existing savings ledger when cleared. See `docs/live-token-xray-daily-briefing-implementation-plan.md`.
- Token X-Ray depth/live updates are complete: bounded revisioned updates, event coalescing, model/context/cache metadata, projected pressure, timestamped cache evidence, recommendation controls, and explicit unavailable states are covered by fixtures and local checks.

- Agent Memory is complete for the planned local slice: source discovery, secret screening, structural/compaction previews, exact-confirmation apply, SHA-verified backup receipts, drift-safe rollback, session handoffs, and content-free attribution.

- Repo Map one-click generation is built for the app repo and local repo paths: Graphify, Madge, dependency-cruiser, Cargo metadata, Tauri invoke/handler scan, tool preflight, partial-success handling, generated artifacts, and estimated token-savings output.
- Repo Map UI is reachable from the sidebar. The existing `RepoMapView` is mounted, and `check:repo-map-mounted` is wired into `evidence:local` so the sidebar route cannot silently disappear again.
- Repo Map artifact controls are built: open `GRAPH_TREE.html`, `README.md`, `COMPACT_CONTEXT.md`, and the generated map folder.
- Repo Map history and staleness/freshness warnings are built for generated local maps.
- Repo Intelligence consumes Repo Map context: freshness, graph-input paths, selected packs, handoffs, CLI exports, stale-map warnings, and MCP smoke evidence are wired through local checks.
- Repo Memory MCP local proof exists: manifest, context pack retrieval, symbol lookup, clear-index path, stale-health surface, app-managed descriptor recheck, and read-only/no-mutation evidence.
- Repo Intelligence incremental index reuse and graph-aware ranking are complete for the current parser schema: unchanged repositories reuse saved metadata, schema mismatches force a safe rebuild, and task affinity/reverse-dependency hubs contribute deterministic pack ranking.
- Repo Memory MCP now has an app-owned, read-only supervised stdio lifecycle with child/restart/exit evidence, stale-health detection, and safe restart behavior.
- Token-savings evidence is no longer just static dashboard constants. Runtime/session attribution, measured benchmark fixtures, anomaly warnings, and source caveats exist for RTK, Repo Intelligence, Caveman, Ponytail, and MarkItDown surfaces.
- Privacy/security baseline is hardened: root SQLite/local DB artifacts are ignored/guarded, `headroom_memory.db` was removed from git, `CLAUDE.md` was scrubbed, local-only network proof exists, and public release proof cannot be satisfied by local unsigned evidence.
- Rollback/Doctor local evidence exists: rollback inventory, managed-record domains, Doctor repair disclosure, Off-mode cleanup, local relaunch evidence, and aggregate local evidence runner coverage.
- Connector/native-write readiness is selectively gated. Managed/safe paths are documented for Claude, Codex, Gemini CLI, OpenCode, Windsurf, Zed, Goose, and Grok/xAI; Cursor native provider/editor writes remain disabled until a supported schema and full lifecycle proof exist.
- Cursor, Goose, and Grok/xAI Switchboard-owned sidecar lifecycles are complete with dry-run preview, exact confirmation, sibling backup, disk verification, rollback, and Off cleanup. Goose and Grok/xAI native endpoint adapters write only their documented allowlisted fields; provider credentials, account state, and model selection remain manual.
- CI email noise for the working branch was reduced by narrowing workflow push branches while preserving main/PR CI intent.
- Local evidence stability was improved: default-off message logging tests now isolate env/app-storage state, and Repo Intelligence local smoke has a longer timeout for Rust compile/test reality.
- Fable security plan is committed and reflected in the current roadmap status.
- Public documentation rebrand uses AI Switchboard / Switchboard / AI Switchboard for Mac while preserving Headroom, RTK, Caveman, Ponytail, MarkItDown, and legacy compatibility wording.
- Website/download-flow rebrand copy now positions the product as AI Switchboard / AI Switchboard for Mac while preserving GitHub Release updater URLs and `Mac-AI-Switchboard_<version>.dmg` compatibility artifact names.
- Final rebrand release evidence is recorded in `docs/ai-switchboard-rebrand-release-evidence.md`, including track commits, stale-name review scope, and release evidence commands.
- Public release `v0.0.0` has a verified signed/notarized Apple Silicon DMG and checksum on GitHub. The public DMG was downloaded, checksum-verified, `hdiutil`-verified, installed as `/Applications/AI Switchboard for Mac.app`, accepted by Gatekeeper as Notarized Developer ID, and validated with `xcrun stapler validate`.
- Installed-app trust for `/Applications/AI Switchboard for Mac.app` has been locally validated against the public release app, and local uninstall dry-run proof passes with the longer backend evidence timeout.
- Public release proof now reconciles completed live release assets separately from remaining proof: `npm run release:proof` records the `v0.0.0` signed/notarized DMG, checksum, signed updater bundle, updater `.sig`, and reachable `latest.json` as completed live evidence while machine-checking current checkout static/installed smoke summaries and reboot-level installed proof as separate blockers.
- Amazon Q Developer CLI now has a managed Switchboard-owned sidecar lifecycle with fixture-home apply, Doctor verify/repair, rollback, and Off cleanup coverage while AWS auth/provider/workspace state stays manual.
- Continue now has a managed Switchboard-owned sidecar lifecycle with fixture-home apply, Doctor verify/repair, rollback, and Off cleanup coverage while provider config stays manual.
- Repo Map now has a native macOS/Tauri folder picker, supervised run-status surface, and backend `repo_map_generation_event` streaming for live status/stdout/stderr while map generation is running.
- Repo Intelligence graphing moved to `path-graph-v9` with tree-sitter-assisted multiline imports, AST call-reference edges, task-term graph affinity, and reverse-dependency hub ranking for context packs.
- Caveman, Compact Chinese, Ponytail, and MarkItDown attribution now carry runtime evidence-unit counts into backend counters and frontend session rows, with estimated add-on counters separated from inferred counters.
- Add-on measurement guardrails are complete: measured savings require an independent, complete before/after evidence pair; missing or invalid evidence remains explicitly estimated.
- Progressive-disclosure/accessibility completion is recorded for technical evidence, stable disclosure IDs, and explicit connector setup actions.
- Gateway readiness is complete for the local slice: redacted previews, reversible intent receipts, Doctor evidence, and opt-in loopback LiteLLM preflight are available without credentials or network writes.
- Reboot-proof automation is complete for the local workflow: arm, record, and check commands require a real post-reboot marker and cannot fabricate installed-app proof.

## Left

- Repo Map can still deepen per-tool progress semantics beyond current backend status/stdout/stderr events.
- Repo Intelligence can still deepen parser/call-graph coverage, mixed-language fixtures, and per-tool Repo Map progress semantics beyond the current graph-affinity, reverse-dependency, and supervised-MCP slice.
- Add-on counters can still move from durable estimated file/host evidence toward true before/after token measurements for Caveman, Ponytail, and MarkItDown sessions.
- Native/provider write promotion is complete for Goose and Grok/xAI endpoint fields with verified allowlists and fixture lifecycle coverage. Cursor remains gated until a documented, supported on-disk provider schema exists; provider/account/model state remains manual everywhere.
- Public installed-app smoke and reboot-level signed installed-app Doctor/Rollback/uninstall proof. Current uninstall proof is non-destructive local dry-run evidence, and `npm run smoke:reboot-level:local` now records the proof as blocked unless current installed-app trust, current public installed-app smoke evidence, supporting Doctor/Rollback/uninstall evidence, and a real post-reboot marker are all present.
- Optional gateway/add-on integrations remain guided/gated only: LiteLLM semantic-cache lifecycle, self-hosted Langfuse observability, Cloudflare Gateway, and Kong live evidence require user infrastructure and credentials. Local readiness and rollback guidance are complete.
- RTK command-family persistence and richer provider-specific X-Ray metrics remain pending until those tools/providers expose credible durable evidence.

## Latest Commits

- `e273d2b2` - Add gateway readiness and reboot proof workflow.
- `3be98b2d` - Expand safe connector and gateway evidence.
- `65b96190` - Deepen agent memory and connector safety.
- `c55f1810` - Build agent intelligence and analytics control surfaces.

- `065ebb2` - Stabilize local evidence message logging tests.
- `f0e4094` - Mount Repo Map view in the app and add the mount guard.
- `b71c9c17` - Add AI Switchboard rebrand audit.
- `2fd696e7` - Add AI Switchboard platform rebrand plan.
- `57fd78a1` - Update public docs for AI Switchboard rebrand.
- `39191f02` - Update app copy for AI Switchboard rebrand.
- `0597a72c` - Preserve runtime compatibility during rebrand.
- `03a90a60` - Add Switchboard CLI platform docs.
- `34b01f25` - Reposition website as AI Switchboard.
- `bff1a99c` - Update public support labels for AI Switchboard.

## Current Validation Commands

- `npm run check:repo-map-mounted`
- `npm run smoke:repo-intelligence:local && npm run smoke:repo-intelligence:local:check`
- `npm run test:desktop`
- `npm run evidence:local`
- `npm run smoke:reboot-level:local || true && npm run smoke:reboot-level:local:check`
- `npm run build`
- `npm run test:frontend`
- `npm run check:local-only-network`
- `npm run release:report && npm run release:report:check`
- `npm run release:proof && npm run release:proof:check`
- `npm run check:branding`
- `rg -n "Mac AI Switchboard|Mac-AI-Switchboard|mac-ai-switchboard|Headroom|RTK|Caveman|Ponytail|MarkItDown" README.md docs src src-tauri package.json scripts`
- `git diff --check`
