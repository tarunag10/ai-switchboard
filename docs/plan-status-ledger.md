# Plan Status Ledger

Updated: 2026-07-03

This is the current done/left ledger for the Mac AI Switchboard roadmap, including the Repo Map/token-compression work, Fable security hardening, local evidence gates, and release-readiness work.

## Done

- Repo Map one-click generation is built for the app repo and local repo paths: Graphify, Madge, dependency-cruiser, Cargo metadata, Tauri invoke/handler scan, tool preflight, partial-success handling, generated artifacts, and estimated token-savings output.
- Repo Map UI is reachable from the sidebar. The existing `RepoMapView` is mounted, and `check:repo-map-mounted` is wired into `evidence:local` so the sidebar route cannot silently disappear again.
- Repo Map artifact controls are built: open `GRAPH_TREE.html`, `README.md`, `COMPACT_CONTEXT.md`, and the generated map folder.
- Repo Map history and staleness/freshness warnings are built for generated local maps.
- Repo Intelligence consumes Repo Map context: freshness, graph-input paths, selected packs, handoffs, CLI exports, stale-map warnings, and MCP smoke evidence are wired through local checks.
- Repo Memory MCP local proof exists: manifest, context pack retrieval, symbol lookup, clear-index path, stale-health surface, app-managed descriptor recheck, and read-only/no-mutation evidence.
- Token-savings evidence is no longer just static dashboard constants. Runtime/session attribution, measured benchmark fixtures, anomaly warnings, and source caveats exist for RTK, Repo Intelligence, Caveman, Ponytail, and MarkItDown surfaces.
- Privacy/security baseline is hardened: root SQLite/local DB artifacts are ignored/guarded, `headroom_memory.db` was removed from git, `CLAUDE.md` was scrubbed, local-only network proof exists, and public release proof cannot be satisfied by local unsigned evidence.
- Rollback/Doctor local evidence exists: rollback inventory, managed-record domains, Doctor repair disclosure, Off-mode cleanup, local relaunch evidence, and aggregate local evidence runner coverage.
- Connector/native-write readiness is gated. Managed/safe paths are documented for Claude, Codex, Gemini CLI, OpenCode, Windsurf, Zed, and Goose Repo Memory MCP; higher-risk provider/editor writes remain disabled until their full lifecycle proof exists.
- CI email noise for the working branch was reduced by narrowing workflow push branches while preserving main/PR CI intent.
- Local evidence stability was improved: default-off message logging tests now isolate env/app-storage state, and Repo Intelligence local smoke has a longer timeout for Rust compile/test reality.
- Fable security plan is committed and reflected in the current roadmap status.

## Left

- Native repo folder picker for Repo Map. Current UI accepts a path text field; next slice should add a deliberate macOS/Tauri folder picker dependency or backend picker command.
- Streaming/background Repo Map job UX. Current progress is step/status based; long runs still need live logs or event polling per tool.
- Deeper language-aware Repo Intelligence parsers and graph analyzers beyond the current tree-sitter/fallback symbol extraction and graph-input evidence.
- More real runtime/session counters for Caveman, Ponytail, and MarkItDown beyond current fixture/proxy/local attribution contracts.
- Native/provider write promotion for Cursor, Aider, Continue, Qwen, Amazon Q, and Grok/xAI. Each needs provider-specific fixture-home apply, verify, rollback, Doctor repair, Off cleanup, and relaunch-survival proof before enabling.
- Signed/notarized public release lane: Developer ID signing, notarized DMG, updater feed proof, checksums/SBOM, public installed-app smoke, public uninstall proof, and public release gate evidence.
- Reboot-level signed installed-app Doctor/Rollback/uninstall proof. Current proof is local-only/ad-hoc.
- Optional gateway/add-on integrations remain guided/gated only: LiteLLM semantic cache lifecycle, self-hosted Langfuse observability, Cloudflare Gateway, and Kong evidence.

## Latest Commits

- `065ebb2` - Stabilize local evidence message logging tests.
- `f0e4094` - Mount Repo Map view in the app and add the mount guard.

## Current Validation Commands

- `npm run check:repo-map-mounted`
- `npm run smoke:repo-intelligence:local && npm run smoke:repo-intelligence:local:check`
- `npm run test:desktop`
- `npm run evidence:local`
- `npm run build`
- `npm run release:report && npm run release:report:check`
- `git diff --check`
