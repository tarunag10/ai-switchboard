# Agent Control Center Implementation Plan

This plan turns Mac AI Switchboard into a local-first agent control center: pick a repo, pick an agent, verify local setup, hand over bounded context, and prove what the app changed or saved.

The work should ship in small slices. Each slice must leave the app usable, keep config changes reversible, and preserve the boundary between local ad-hoc evidence and public signed release readiness.

## Shipped Vs Left

Current checkpoint: Start Agent Session, managed sidecar connector coverage, Gemini/OpenCode managed routing, read-only Repo Intelligence APIs, repo-memory MCP lifecycle/supervision evidence, durable savings attribution, guarded Rollback Center native restore/cleanup, native undo-all for ready rows, release-readiness local evidence, Caveman Compact Chinese, and trust-hardening guards are shipped. Remaining work centers on provider-specific native config mutation, long-running repo-memory MCP process supervision, deeper Repo Intelligence graphs, stronger measured add-on counters, real approved config writes, Rollback Center relaunch-survival evidence, signed/notarized public release evidence, and final trust hardening.

### Shipped

- Start Agent Session has a first usable flow with repo-path validation, freshness detail, selected-pack/full-handoff/summary/JSON copy actions, sample-pack copy blocking, managed connector readiness in session payloads, and matching `repo:intelligence --session` CLI exports.
- Managed connector sidecar coverage exists for Gemini CLI, OpenCode, Grok/xAI CLI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, and Zed, including config surfaces, manual guides, automation gates, rollback dossiers, copyable config-plan actions, compatibility-matrix checks, dry-run previews for detected or documented config surfaces, and a seven-stage automation path for detect, dry-run, backup, apply, verify, rollback, and Off cleanup.
- Read-only Repo Intelligence API coverage exists for manifests, packs, handoffs, freshness, parser/index health fields, stale/moved/corrupt index states, safe clear behavior, bounded pack output, and secret-like path exclusion.
- Repo Intelligence parser coverage now indexes modern JavaScript/TypeScript exported arrow functions, async assignment functions, generic arrow helpers, and Python async defs in addition to existing functions/classes/types/consts, improving local symbol search and graph edges without changing the read-only boundary.
- Repo-memory MCP is advertised in CLI help and smoke-tested through `npm run check:repo-memory-mcp`, including read-only tool annotations and a real `repo_context_pack` call that checks safety text and seeded secret exclusion.
- Repo-memory MCP now has an app-facing lifecycle status contract in the Mode Inspector, Doctor repair flow, and Doctor timeline copy, including direct install/start/stop actions, smoke-check command, read-only tool names, persisted app-session active state, supervision status/last-check evidence, stale-config detection, and secret-exclusion safety boundary.
- Savings ledger has shipped rows/copy exports with measured, estimated, and inferred confidence labels, empty-state distinction, active-filter copy disclosure, per-row evidence/caveats, and attribution percentages by confidence.
- Durable savings attribution has started with append-only Headroom engine, RTK, Repo Intelligence, MarkItDown, Ponytail, Caveman, and Compact Chinese session events exposed through a read-only backend command; the frontend session ledger now consumes durable measured, estimated, and inferred backend events when available.
- Safe config dry-run coverage proves managed write paths can produce blocked previews with target, backup, marker, rollback, Off cleanup, and unmanaged-config boundaries; confirmed apply plans now require the exact preview phrase before producing write-ready next text. OpenCode now has the first promoted native safe-apply path with backend preview, exact confirmation, sibling backup, write, verify, and Rollback Center restore coverage.
- Doctor support/timeline copy now includes scrubbed status, issue, repair-success, Repo Intelligence availability-gate events, parser/index health evidence, moved-repo detail, token/path/secret scrubbing, uninstall dry-run managed-footprint counts, and a Doctor-visible Rollback Center inventory copy with per-change rollback plans, restore mode, evidence, and cleanup boundaries.
- Rollback Center now has a guarded execution-preview contract for every managed record, plus native undo-all preview/execution for ready allowlisted rows that separates config-backed and sidecar-backed native rows from manual or cleanup-only rows, including stable undo-all order, exact confirmation phrase, backend action classification, cleanup-only boundaries, disk re-read relaunch-survival evidence for native rows, and native-write status while unsupported restore commands remain blocked.
- Backend Rollback Center execution now has allowlisted `codex-routing` and `opencode-routing` restore paths plus Gemini and sidecar-only managed-block cleanup for Cursor, Grok/xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, and Zed: preview validates marker/backup readiness where required, execute requires the exact confirmation phrase, backup restores create a fresh safety backup, cleanup removes only Switchboard-owned sidecar blocks, and fixture-home tests cover the native rows.
- Rollback Center UI now exposes the allowlisted Codex, Gemini, OpenCode, and sidecar-backed native flows: preview readiness, target/backup/marker evidence, no-backup cleanup readiness, exact confirmation input, guarded restore/cleanup button, native undo-all preview/execution for ready rows, and result/error copy.
- Release readiness surfaces have shipped report loading, one-click fresh report refresh through `npm run release:ready -- --json`, executable desktop validation, static smoke-preflight, local unsigned DMG build/install, and local installed-smoke evidence generation, copyable snapshots, command copy, source labels that distinguish guidance from report proof, evidence coverage summary, next blocked release action, connector-readiness evidence checks, and separate signing/notarization/updater blocker categories.
- Caveman supports scoped, aggressive, and experimental opt-in Compact Chinese profiles; Compact Chinese is limited to private internal planning notes and handoffs while user-facing, legal, safety, debugging, and release-readiness content stays in the requested language with full detail.

### Left To Build

- Promote provider-specific config mutation beyond managed sidecars, connector by connector, only after each native surface has real backup, apply, verify, rollback, Doctor repair, fixture-home restore tests, and Off cleanup. Priority surfaces include Cursor/Windsurf/Zed editor settings, Continue multi-provider config, Goose MCP/provider config, Aider wrapper/env config, Grok/Qwen account and model guardrails, and Amazon Q credential-safe integration.
- Turn repo-memory MCP from a smoke-tested CLI transport into a fully app-managed local service. The status/copy contract, backend install/start/stop commands, Mode Inspector actions, Doctor repair integration, connector-consumption matrix, persisted supervision status, last-check evidence, and stale-config detection exist; remaining work is real long-running process supervision and connector-specific MCP bridge setup docs as native/MCP-aware agents are promoted.
- Deepen Repo Intelligence v2 with tree-sitter or language-specific parsers, richer symbols/imports/reverse dependencies, and graph-aware packs. Persistent parser/index version fields, health fields, Doctor health checks, and expanded lightweight symbol extraction for modern JS/TS/Python async declarations are shipped.
- Finish exact live/session savings attribution by replacing inferred add-on template events with stronger measured counters where profile-specific evidence exists. Headroom and RTK measured session events are shipped; Repo Intelligence records estimated best-pack token-avoidance events after successful index builds; MarkItDown, Ponytail, Caveman, and Compact Chinese record durable inferred template events when enabled; the frontend session ledger consumes all durable backend event confidences.
- Promote confirmed safe config apply plans into real user-approved file writes, verification, and rollback flows for supported connectors without touching unmanaged config. OpenCode provider config is promoted; remaining connectors still need the same safe path before native writes are enabled.
- Complete persistent Rollback Center execution with actual per-change restore actions, relaunch survival, and a guarded "undo all Switchboard changes" flow. The Doctor copy surface, inventory model, execution-preview contract, copyable undo-all preview, Codex/OpenCode backup restore paths, Gemini managed cleanup path, sidecar-backed connector cleanup paths, UI execution gate for no-backup cleanup rows, guarded native undo-all orchestration for ready rows, and disk re-read relaunch-survival evidence are shipped; remaining work is installed-app relaunch smoke evidence, broader undo-all coverage for dedicated cleanup domains, and promotion to deeper provider-specific config-backed rows.
- Complete release readiness execution by wiring signed/notarized public DMG install and confirmed public installed-smoke evidence generation into the in-app panel. Fresh report refresh from `npm run release:ready -- --json`, desktop validation, static smoke-preflight execution, local unsigned DMG build/install, and local installed-smoke evidence recording are shipped.
- Finish trust hardening from the product roadmap: app-owned legal/privacy surfaces, local-only network audit, branding/asset provenance, and public-release evidence cleanup. The remote destination registry is shipped in `docs/remote-destinations.md` and enforced by governance/deployment checks. Main in-app support actions now route to this repository's GitHub Issues instead of inherited upstream support mailto links. External link opening now has backend SSRF/injection guards for schemes, credentials, line breaks, and loopback/private/local hosts. Local-only backend refusal now blocks account activation, checkout, plan change, reactivation, billing portal, and contact commands before auth or HTTP setup. Branding provenance now names the app-owned iconset source folder, and the guard blocks the inherited `headroom.iconset` name.

## Current Connector Checkpoint

- Gemini and OpenCode are no longer only registry plans: Gemini has managed shell base-url env routing, and OpenCode has provider config routing plus the shared sidecar lifecycle.
- Gemini CLI, OpenCode, Grok/xAI CLI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, and Zed now have managed sidecar lifecycle coverage with config surfaces, manual guides, automation gates, rollback dossiers, dry-run previews, backup/apply/verify/rollback/Off cleanup stages, and release-readiness evidence.
- `npm run check:connectors` reports `0 pending planned, 11 managed, 9 retained compatibility dossiers` and enforces the shared config-creation contract, including the seven required gated steps and copyable markdown handoff.
- Managed connector cards now surface the same seven config-creation gates in-app, with provider-specific native mutation still gated behind compatibility dossiers where needed.
- Managed connector cards now provide per-tool Copy config plan actions, so Gemini, OpenCode, Grok/xAI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, and Zed each export the same gated config-creation contract from the Mac app.
- Detected managed connector cards now receive dry-run previews with target, marker, backup path, proposed state, rollback preview, confirmation phrase, apply-blocked reason when native mutation is not promoted, and seven-stage automation path status.
- Repo Intelligence agent handoffs now include connector config readiness, next gate, evidence requirements, config path strategy, account caveat, and rollback strategy for each managed connector target.
- The `repo:intelligence` CLI handoff export now mirrors those connector readiness dossiers in Markdown and JSON output.
- `npm run check:connectors` now verifies the CLI connector dossier mirror so managed config metadata cannot silently drift across app, backend, and handoff exports.
- The Tauri `get_agent_handoff` read-only API now returns matching connector config readiness dossiers for managed connector targets while leaving Claude/Codex handoffs unchanged.
- Doctor support copy and Repo Intelligence docs now describe the `get_agent_handoff` connector readiness payload, including next gate, evidence requirements, config path strategy, account caveat, and rollback strategy.
- `npm run check:connectors` now verifies the Tauri `get_agent_handoff` connector readiness response and tests alongside the frontend, backend adapter registry, and CLI mirror.
- Release readiness smoke evidence now requires connector readiness payloads in agent handoffs alongside the managed connector config creation plan.
- `npm run check:deployment` now verifies the connector-readiness release evidence chain across beta smoke docs, smoke preflight, release readiness report generation, and dashboard copy.
- `docs/remote-destinations.md` now inventories app-owned remote destinations, provider-traffic boundaries, tool download surfaces, local-only behavior, and change-control requirements, with governance/deployment checks requiring the registry before release.
- Remote-services UI copy now mirrors the backend/frontend Sentry env split and routes support actions to this repository's GitHub Issues, with tests preventing the inherited upstream support mailbox from returning.
- Backend external-link opening now validates browser/mail launches before shelling out, rejecting unsupported schemes, embedded credentials, line-break injection, loopback, private, and local hosts with focused Rust coverage.
- Local-only backend guards now reject account, billing, and contact command entrypoints before auth or HTTP setup, with Rust coverage for pricing and contact guard paths.
- Branding hardening now renames the tracked Tauri iconset source folder to `src-tauri/icons/mac-ai-switchboard.iconset/` and extends `npm run check:branding` to reject the inherited iconset name.
- Repo Memory MCP docs now include a connector consumption matrix for Claude Code, Codex, Gemini CLI, OpenCode, Goose, Cursor, Windsurf, Zed, Aider, Continue, Grok / xAI CLI, Qwen Code, and Amazon Q Developer CLI, while keeping MCP context separate from provider/editor config mutation.
- Durable savings attribution now records estimated Repo Intelligence best-pack token-avoidance events after successful index builds, inferred MarkItDown, Ponytail, Caveman, and Compact Chinese template events when those add-ons are enabled, and session ledger rows for all durable backend event confidences.
- `npm run release:report:check` now rejects release reports that omit connector readiness payload evidence from static or installed smoke evidence.
- Start Agent Session now exposes managed connector config readiness as a first-class app/session field, including the next gate and automation state before copying Gemini, OpenCode, Grok/xAI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, or Zed handoffs.
- The `repo:intelligence --session` CLI export now mirrors that session-level config readiness in JSON and Markdown, so config-creation work stays Gemini-like and gated across app and terminal workflows.
- Repo Intelligence manifests now advertise per-agent Start Agent Session recipes, including the exact `--session` command, default task, read-only safety, provider routing state, and managed connector next gate when applicable.
- Doctor timeline copy now includes Repo Intelligence availability gates for `get_index_freshness`, missing/stale/corrupt/moved index states, `clear_repo_index` cleanup boundaries, and the evidence agents need before trusting saved packs.
- The Doctor panel now exposes a Copy timeline action that exports scrubbed Doctor status, issue, repair-success, and Repo Intelligence availability-gate events for support/debugging.
- Doctor now reports moved or replaced Repo Intelligence repo paths explicitly when the saved file map no longer matches the existing folder, with the same managed-index-only clear repair.
- Savings ledger exports now include per-row evidence alongside confidence caveats, so measured, estimated, and inferred savings keep their source equations visible when copied.
- Managed config dry-run exports now label the write path as blocked and spell out the Off-mode cleanup boundary before any config apply path can be promoted.
- Uninstall dry-run exports now state that their managed footprint comes from the Rollback Center inventory and include the item count, with tests guarding against drift.
- Doctor now exposes a Copy Rollback Center action that exports the full no-write rollback inventory from the app, including target summaries, markers, backup expectations, restore modes, evidence, and Off cleanup boundaries.
- Release readiness command copy now includes the strict public-release gate, report path, and local unsigned/ad-hoc evidence boundary even before a report JSON is loaded.
- Start Agent Session now has a dedicated Copy summary action alongside full handoff, selected pack, and JSON copy; sample/demo indexes stay blocked from summary copy too.
- Start Agent Session repo-path validation now uses a shared tested helper, so empty or whitespace-only paths are blocked before the Mac app invokes indexing.
- Start Agent Session freshness detail now includes changed-cache metadata, with tests proving stale indexes stay labeled as changed instead of fresh/current.
- Read-only `get_repo_pack` responses now have Rust coverage for default pack selection, bounded file lists, secret exclusion, freshness safety, verification-pack selection, and unknown-pack errors.
- Savings ledger empty states now distinguish a genuinely empty ledger from confidence filters that hide existing rows, with helper tests covering both copy paths.
- Savings ledger copy payloads now include the active confidence filter, so exported rows remain auditable when users copy measured, estimated, inferred, or all-row views.
- Safe config diff coverage now proves every managed config write path in the rollback inventory can produce a blocked dry-run preview with target, backup, marker, rollback, Off cleanup, and unmanaged-config boundaries.
- Doctor timeline support copies now scrub user paths, token-like values, and common secret assignments before sharing support/debug evidence.
- The tool compatibility matrix now lists every managed connector, and `npm run check:connectors` fails if the matrix omits a connector or required Gemini/OpenCode routing gates.
- Release readiness report copies now categorize signing, notarization, and updater blockers separately so missing secrets stay release blockers, not app failures.
- Release readiness source labels now state that checklist defaults are guidance, not release proof, until `npm run release:ready` produces the report JSON.
- Repo Intelligence CLI help now advertises the read-only `--mcp-serve` repo-memory transport, and deployment readiness checks guard its no-mutation MCP tool contract.
- Repo-memory MCP tools now declare `readOnlyHint` annotations for context packs, symbol lookup, and dependent-edge queries, with deployment readiness guarding the annotation.
- `npm run check:repo-memory-mcp` now smoke-tests the repo-memory MCP stdio tool list and runs in the release verifier before connector parity checks.
- Mode Inspector now labels Repo Memory MCP separately from the generic Headroom MCP state and explains configured, needs-attention, and unknown lifecycle states.
- Mode Inspector now offers a one-click Prepare MCP action for unconfigured Repo Memory MCP state, backed by `install_repo_memory_mcp` followed by `start_repo_memory_mcp`; configured states still expose Start MCP and Stop MCP.
- Doctor now reports `repo_memory_mcp_not_configured` as an automatic Prepare MCP repair; the repair installs the app-managed config, runs the start/smoke check, and is included in Repair all.
- Doctor timeline copy now includes Repo Memory MCP lifecycle evidence: `install_repo_memory_mcp`, `npm run check:repo-memory-mcp`, read-only tool names, and the secret-like path exclusion boundary.
- Repo Memory MCP docs now explain supported-agent consumption, MCP-aware connector boundaries, smoke verification, and troubleshooting.
- Cursor and Windsurf now surface existing editor settings files as detection evidence, while native provider writes remain blocked until settings parse, dry-run diff, backup, verification, rollback, and Off cleanup are promoted.
- Release readiness dashboard slices already shipped report loading, copyable report snapshots, and a Run local evidence action that sequences desktop validation, smoke preflight, local DMG build/install, and local installed smoke without touching signing, notarization, or public publication gates.

## Product Goals

- Make Repo Intelligence the primary workflow before starting Codex, Claude Code, Gemini CLI, Cursor, Aider, OpenCode, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, Zed AI, or similar tools.
- Expose read-only local repo context through app UI, CLI, and the repo-memory MCP interface so agents stop rediscovering the same codebase; remaining MCP work is long-running supervision and connector-specific bridge docs.
- Prove savings by source: Headroom engine compression, RTK output reduction, Repo Intelligence avoided reads, MarkItDown preprocessing, Ponytail smaller-change guidance, and Caveman terse-output guidance.
- Add Caveman Compact Chinese as an experimental opt-in profile only for private internal planning notes and handoffs; user-facing, legal, safety, debugging, and release-readiness content must stay in the user's requested language with full required detail.
- Make every managed config mutation visible before and after it happens.
- Promote provider-specific native config mutation only after detection, dry-run diff, backup, verify, rollback, and Off-mode cleanup exist for that native surface.
- Surface release readiness in the app without confusing local unsigned install success with signed/notarized public release readiness.

## Non-Negotiable Constraints

- Local-first by default; no repo contents leave the Mac through Repo Intelligence.
- Off means off: no proxy listener, no routing hook, no hidden repair, no LaunchAgent restore.
- First API surfaces are read-only and bounded.
- Config edits require explicit user action, backup, managed markers, and rollback.
- Native provider/editor config mutation stays manual or detect-only until its connector-specific safety contract is complete.
- Savings rows must be labeled as measured, estimated, or inferred.
- Release gates must use existing scripts as authority instead of UI inference.

## Phase 1: Start Agent Session

Goal: give users one workflow that prepares a coding-agent session from local state.

Deliverables:

- Add a "Start Agent Session" flow in the Repo Intelligence area.
- Inputs: repo path, agent target, and task type.
- Refresh or reuse the latest local index, then show freshness, skipped files, secret exclusions, estimated tokens avoided, and selected packs.
- Generate agent-specific handoffs for Codex, Claude Code, Gemini CLI, OpenCode, Aider, Goose, Cursor, Continue, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI.
- Recommend a Switchboard mode from current local state:
  - Full optimization when Headroom and RTK are healthy.
  - Headroom only when shell-output compression is unavailable.
  - RTK only when provider routing is unsafe or Headroom compression is risky.
  - Off when the user wants clean pass-through/debugging.
- Add copy actions for summary, selected pack, and full handoff.

Implementation areas:

- `src/App.tsx`
- `src/lib/repoIntelligence.ts`
- `src/lib/trayHelpers.ts`
- `src-tauri/src/repo_intelligence.rs`
- `src/components/*Repo*`
- `docs/repo-intelligence-plan.md`

Acceptance gates:

- `npm run repo:intelligence -- . --manifest` includes agent recipes and token estimates.
- The UI refuses to copy sample/demo packs as real repo context.
- Stale indexes are visible and never presented as current.
- Tests cover empty repo path, stale index, valid index, agent selection, and copy payload generation.

Suggested commit:

- `Add agent session preparation flow`

## Phase 2: Read-Only Local Agent API

Goal: let local agents request prepared context without manual copy-paste.

Deliverables:

- Add a read-only command/API contract:
  - `get_repo_manifest`
  - `get_repo_pack`
  - `get_agent_handoff`
  - `get_index_freshness`
  - `clear_repo_index`
- Return bounded JSON and Markdown.
- Include safety flags: read-only, ignored secret paths, skipped generated/vendor paths, token budget, graph freshness, parser version.
- Document how each agent can consume the API today, even before automatic connector setup exists.
- Add Doctor checks for stale index, missing index, corrupt index, moved repo paths, and local API availability.
- Treat `get_index_freshness` as the availability gate for local agents: it must expose API availability, graph status, parser/indexer metadata, and missing/stale/corrupt index state before an agent trusts a pack.

Implementation areas:

- `src-tauri/src/repo_intelligence.rs`
- `src-tauri/src/lib.rs`
- `src/lib/repoIntelligence.ts`
- `src/lib/doctorRepairCopy.ts`
- `docs/repo-intelligence-plan.md`

Acceptance gates:

- API calls are read-only and do not mutate user repositories.
- Oversized outputs are bounded.
- Secret-like files remain excluded by default.
- Missing, stale, corrupt, or moved repo indexes remain visible in Doctor until cleared or re-indexed.
- Tests cover manifest, pack, handoff, stale, clear, and corrupt-storage paths.

Suggested commit:

- `Expose read-only repo handoff API`

## Phase 3: Live Savings Ledger

Goal: make product value visible and credible.

Deliverables:

- Add a unified savings ledger with source, scope, timestamp, amount, and confidence.
- Sources:
  - Headroom engine: measured request compression where available.
  - RTK: measured shell-output reduction where available.
  - Repo Intelligence: estimated avoided discovery reads.
  - MarkItDown: estimated binary-document-to-Markdown reduction.
  - Ponytail: inferred smaller-change guidance savings.
  - Caveman: inferred terse-output guidance savings.
  - Caveman Compact Chinese: experimental inferred prompt and handoff compaction, counted separately only after profile-specific evidence exists.
- Scopes: current session, repo, today, month, lifetime.
- Add UI filters and a "copy savings summary" action.
- Keep equations and caveats visible in exported summaries.

Implementation areas:

- `src/lib/savingsCalculator.ts`
- `src/lib/dashboardHelpers.ts`
- `src-tauri/src/analytics.rs`
- `src-tauri/src/state.rs`
- `src/components/OptimizePanel.tsx`
- `src/App.tsx`

Acceptance gates:

- Empty ledger renders clearly.
- RTK-only, Headroom-only, Full, and mixed sessions show separate rows.
- Estimated and inferred rows are never shown as measured.
- Tests cover aggregation, formatting, copy payloads, and edge cases.

Suggested commit:

- `Add live savings ledger`

## Phase 4: Safe Config Diff Viewer

Goal: make managed edits inspectable before applying them.

Deliverables:

- Add dry-run diffs for Claude Code, Codex, and future connectors.
- Show target file, existing managed block, proposed managed block, backup path, and rollback behavior.
- Add a confirmation step before applying changes.
- Add tests proving unmanaged user config survives apply, repair, Off cleanup, and rollback.

Implementation areas:

- `src-tauri/src/client_adapters.rs`
- `src/lib/managedChanges.ts`
- `src/components/SwitchboardDoctorPanel.tsx`
- `src/components/SwitchboardPanel.tsx`

Acceptance gates:

- Every write path can produce a dry-run summary.
- Backups are created before writes.
- Off mode removes only Switchboard-owned changes.
- Tests use temporary fixture homes and protect unrelated config sections.

Suggested commit:

- `Show safe config diffs before edits`

## Phase 5: Doctor Timeline And Rollback Center

Goal: show what changed, when it changed, and how to undo it.

Deliverables:

- Persist local events for install, enable, disable, repair, backup, rollback, failed repair, index refresh, and connector setup.
- Add a Doctor timeline with scrubbed paths and statuses.
- Add a Doctor-visible Rollback Center copy/export for every managed change.
- Add per-change rollback execution where safe.
- Add "copy Doctor timeline" for support/debugging.
- Add uninstall dry-run output that matches the actual managed footprint.

Implementation areas:

- `src-tauri/src/storage.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/client_adapters.rs`
- `src/lib/doctorRepairCopy.ts`
- `src/components/SwitchboardDoctorPanel.tsx`
- `src/lib/uninstallDisclosure.ts`

Acceptance gates:

- Timeline and rollback inventory survive app relaunch.
- Sensitive values are scrubbed.
- Rollback does not touch unrelated config.
- Doctor copy output is complete enough for debugging.

Suggested commit:

- `Add Doctor timeline and rollback center`

## Phase 6: Managed Connector Baseline And Native Config Promotion

Goal: keep the shipped managed sidecar baseline for all connector targets and promote native provider/editor config mutation only where the connector-specific safety contract is proven.

Current baseline: Gemini CLI, OpenCode, Grok/xAI CLI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI all have managed sidecar lifecycle coverage. `npm run check:connectors` should continue to report `0 pending planned, 11 managed, 9 retained compatibility dossiers`.

Deliverables:

- Preserve binary/version detection and compatibility reporting for every managed connector.
- Keep sidecar dry-run diff, backup, apply, verify, rollback, and Off cleanup available for every managed connector.
- Promote native config writes connector by connector only after the provider/editor surface is proven locally.
- Keep provider routing manual if model/account compatibility cannot be verified locally.

Connector config track:

- Gemini CLI keeps managed shell base-url env routing as the current safe path; native prompt-routing behavior remains dependent on Gemini CLI honoring those env vars at runtime.
- OpenCode keeps provider config routing as the closest Codex/Claude-style path and remains the reference implementation for connector-specific config promotion.
- Grok / xAI CLI requires `grok` or `xai` detection plus model/account guardrails before any base-url or provider config is offered.
- Cursor, Windsurf, and Zed follow profile-aware editor paths: detect app/profile settings, show a dry-run settings diff, back up profile settings, apply only user-approved routing, verify, rollback, and clean up in Off mode.
- Aider, Continue, Goose, Qwen Code, and Amazon Q Developer CLI retain compatibility dossiers until their wrapper, provider, MCP, account, or credential-safe native config contracts are backed by fixture-home restore tests.
- No connector may create or modify native config until its manifest has config paths, account caveats, rollback strategy, Doctor verification, and fixture-home restore tests.

Implementation areas:

- `src/lib/connectorReadiness.ts`
- `src-tauri/src/client_adapters.rs`
- `src-tauri/src/lib.rs`
- `research/tool-compatibility-matrix.md`
- `docs/architecture.md`

Acceptance gates:

- Detection never mutates native config.
- Native config automation is disabled until all connector-specific safety gates pass.
- Doctor explains each blocked automation gate.
- OpenCode, Grok/xAI, Cursor, and every other managed connector retain explicit config-creation dossiers before native config mutation is promoted.
- `npm run check:connectors` verifies the manifest contract.

Suggested commit:

- `Promote native connector config safely`

## Phase 7: Release Readiness Dashboard

Goal: make release state understandable from the app.

Deliverables:

- Add a release readiness panel showing:
  - frontend build status
  - desktop test status
  - local DMG status
  - installed-app smoke status
  - signing environment status
  - notarization environment status
  - updater configuration status
  - final release gate result
- Add "copy release report."
- Keep local unsigned/ad-hoc install success separate from signed/notarized release readiness.

Implementation areas:

- `src/lib/releaseReadiness.ts`
- `scripts/check-release-readiness.mjs`
- `scripts/release-readiness-report.mjs`
- `src/App.tsx`
- `docs/macos-release.md`

Acceptance gates:

- UI uses release scripts as the source of truth.
- Missing secrets are reported as blockers, not app failures.
- Local unsigned DMG evidence is labeled local-only.
- Tests cover ready, blocked, and missing-artifact states.

Suggested commit:

- `Surface release readiness in app`

## Subagent Work Plan

The main agent owns edits, integration, tests, commits, and pushes. Subagents are used for bounded discovery or disjoint implementation only.

### Subagent 1: Repo Workflow Scout

Goal: identify the minimal code path for the first "Start Agent Session" slice.

Questions:

- Where is the current Repo Intelligence UI implemented?
- What type contracts already exist for agent handoffs?
- What is the smallest UI addition that can reuse existing pack builders?
- Which tests should be extended first?

Output:

- File map.
- Recommended first implementation slice.
- Test list.
- Risks.

### Subagent 2: Backend API Scout

Goal: map the current Rust repo-intelligence command surface and identify safe read-only API additions.

Questions:

- Which Tauri commands already call `repo_intelligence.rs`?
- Where is latest-summary persistence handled?
- Which command names and payload shapes should be added first?
- What backend tests or fixtures exist?

Output:

- Command map.
- Proposed read-only API shape.
- Test list.
- Risks.

### Subagent 3: Savings Ledger Scout

Goal: map existing savings sources and find the smallest credible ledger slice.

Questions:

- Where do current savings calculations live?
- Which values are measured versus estimated?
- Which UI surface should show source attribution first?
- Which tests already cover formatting and aggregation?

Output:

- Source map.
- First ledger model.
- Test list.
- Risks.

### Subagent 4: Connector Safety Scout

Goal: map managed connector contracts and define the next native config promotion slice.

Questions:

- What metadata does `connectorReadiness.ts` already require?
- What backend detection functions already exist for managed tools?
- Which native provider/editor config surface is safest to promote after OpenCode?
- Which script validates connector manifests?

Output:

- Connector contract map.
- Native config promotion proposal.
- Test list.
- Risks.

## First Build Slice

Start with Phase 1, but keep the first commit narrow:

- Add pure helper types/functions for an agent-session preparation model.
- Reuse existing Repo Intelligence summary and handoff builders.
- Add tests for recommended mode, stale index copy, and agent handoff labels.
- Wire UI only after helper tests pass.

First-slice acceptance:

- `npm run test:frontend -- src/lib/repoIntelligence.test.ts --pool=threads`
- `npm run repo:intelligence -- . --manifest`
- No config writes.
- No release behavior changes.
