# Agent Control Center Implementation Plan

This plan turns Mac AI Switchboard into a local-first agent control center: pick a repo, pick an agent, verify local setup, hand over bounded context, and prove what the app changed or saved.

The work should ship in small slices. Each slice must leave the app usable, keep config changes reversible, and preserve the boundary between local ad-hoc evidence and public signed release readiness.

## Current Connector Checkpoint

- Gemini has the deepest current implementation path.
- OpenCode, Grok/xAI CLI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, and Zed already exist in the planned connector registry with config surfaces, manual guides, automation gates, and rollback dossiers.
- Each planned connector must get config-creation work like Gemini, gated behind detection, dry-run diff, backup, verify, rollback, and Off cleanup.
- `npm run check:connectors` enforces the shared config-creation plan contract, including the seven required gated steps and copyable markdown handoff.
- Planned connector cards now surface the same seven config-creation gates in-app before backend-specific detection evidence is available.
- Planned connector cards now provide per-tool Copy config plan actions, so Gemini, OpenCode, Grok/xAI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, and Zed each export the same gated config-creation contract from the Mac app.
- Repo Intelligence agent handoffs now include connector config readiness, next gate, evidence requirements, config path strategy, account caveat, and rollback strategy for each planned connector target.
- The `repo:intelligence` CLI handoff export now mirrors those connector readiness dossiers in Markdown and JSON output.
- `npm run check:connectors` now verifies the CLI connector dossier mirror so planned config metadata cannot silently drift across app, backend, and handoff exports.
- The Tauri `get_agent_handoff` read-only API now returns matching connector config readiness dossiers for planned connector targets while leaving Claude/Codex handoffs unchanged.
- Doctor support copy and Repo Intelligence docs now describe the `get_agent_handoff` connector readiness payload, including next gate, evidence requirements, config path strategy, account caveat, and rollback strategy.
- `npm run check:connectors` now verifies the Tauri `get_agent_handoff` connector readiness response and tests alongside the frontend, backend adapter registry, and CLI mirror.
- Release readiness smoke evidence now requires connector readiness payloads in agent handoffs alongside the planned connector config creation plan.
- `npm run check:deployment` now verifies the connector-readiness release evidence chain across beta smoke docs, smoke preflight, release readiness report generation, and dashboard copy.
- `npm run release:report:check` now rejects release reports that omit connector readiness payload evidence from static or installed smoke evidence.
- Start Agent Session now exposes planned connector config readiness as a first-class app/session field, including the next gate and disabled automation state before copying Gemini, OpenCode, Grok/xAI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, or Zed handoffs.
- The `repo:intelligence --session` CLI export now mirrors that session-level config readiness in JSON and Markdown, so config-creation work stays Gemini-like and gated across app and terminal workflows.
- Repo Intelligence manifests now advertise per-agent Start Agent Session recipes, including the exact `--session` command, default task, read-only safety, manual provider routing state, and planned connector next gate when applicable.
- Doctor timeline copy now includes Repo Intelligence availability gates for `get_index_freshness`, missing/stale/corrupt/moved index states, `clear_repo_index` cleanup boundaries, and the evidence agents need before trusting saved packs.
- The Doctor panel now exposes a Copy timeline action that exports scrubbed Doctor status, issue, repair-success, and Repo Intelligence availability-gate events for support/debugging.
- Doctor now reports moved or replaced Repo Intelligence repo paths explicitly when the saved file map no longer matches the existing folder, with the same managed-index-only clear repair.
- Savings ledger exports now include per-row evidence alongside confidence caveats, so measured, estimated, and inferred savings keep their source equations visible when copied.
- Managed config dry-run exports now label the write path as blocked and spell out the Off-mode cleanup boundary before any config apply path can be promoted.
- Uninstall dry-run exports now state that their managed footprint comes from the Rollback Center inventory and include the item count, with tests guarding against drift.
- Release readiness command copy now includes the strict public-release gate, report path, and local unsigned/ad-hoc evidence boundary even before a report JSON is loaded.
- Start Agent Session now has a dedicated Copy summary action alongside full handoff, selected pack, and JSON copy; sample/demo indexes stay blocked from summary copy too.
- Start Agent Session repo-path validation now uses a shared tested helper, so empty or whitespace-only paths are blocked before the Mac app invokes indexing.
- Start Agent Session freshness detail now includes changed-cache metadata, with tests proving stale indexes stay labeled as changed instead of fresh/current.
- Read-only `get_repo_pack` responses now have Rust coverage for default pack selection, bounded file lists, secret exclusion, freshness safety, verification-pack selection, and unknown-pack errors.
- Savings ledger empty states now distinguish a genuinely empty ledger from confidence filters that hide existing rows, with helper tests covering both copy paths.
- Savings ledger copy payloads now include the active confidence filter, so exported rows remain auditable when users copy measured, estimated, inferred, or all-row views.
- Safe config diff coverage now proves every managed config write path in the rollback inventory can produce a blocked dry-run preview with target, backup, marker, rollback, Off cleanup, and unmanaged-config boundaries.
- Doctor timeline support copies now scrub user paths, token-like values, and common secret assignments before sharing support/debug evidence.
- The tool compatibility matrix now lists every planned connector from the registry, and `npm run check:connectors` fails if the matrix omits a connector or the Gemini detection-only gate.
- Release readiness dashboard slices already shipped report loading and copyable report snapshots; continue shipping the remaining roadmap slice by slice with a commit and push after each validated slice.

## Product Goals

- Make Repo Intelligence the primary workflow before starting Codex, Claude Code, Gemini CLI, Cursor, Aider, OpenCode, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, Zed AI, or similar tools.
- Expose read-only local repo context through app UI, CLI, and a future MCP-style interface so agents stop rediscovering the same codebase.
- Prove savings by source: Headroom engine compression, RTK output reduction, Repo Intelligence avoided reads, MarkItDown preprocessing, and Ponytail smaller-change guidance.
- Make every managed config mutation visible before and after it happens.
- Convert planned connectors into supported connectors only after detection, dry-run diff, backup, verify, rollback, and Off-mode cleanup exist.
- Surface release readiness in the app without confusing local unsigned install success with signed/notarized public release readiness.

## Non-Negotiable Constraints

- Local-first by default; no repo contents leave the Mac through Repo Intelligence.
- Off means off: no proxy listener, no routing hook, no hidden repair, no LaunchAgent restore.
- First API surfaces are read-only and bounded.
- Config edits require explicit user action, backup, managed markers, and rollback.
- Planned connectors stay manual or detect-only until their safety contract is complete.
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
- Add per-change rollback where safe.
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

- Timeline survives app relaunch.
- Sensitive values are scrubbed.
- Rollback does not touch unrelated config.
- Doctor copy output is complete enough for debugging.

Suggested commit:

- `Add Doctor timeline and rollback center`

## Phase 6: First Real Planned Connector

Goal: convert one planned connector into a supported adapter without weakening safety.

Recommended first target: Gemini CLI.

Deliverables:

- Detect Gemini CLI binary and version.
- Detect stable provider/config surface without writing.
- Show compatibility report in planned connector UI.
- Add dry-run diff, backup, apply, verify, rollback, and Off cleanup only after config surface is proven.
- Keep provider routing manual if model/account compatibility cannot be verified locally.

Connector config track:

- Gemini CLI is the first full pattern: detect, report compatibility, then add safe config creation only after the provider surface is proven.
- OpenCode follows the same path for provider config: detect `opencode`, find the active config path, preview the local proxy entry, back up, apply, verify, rollback, and clean up in Off mode.
- Grok / xAI CLI follows the same path after `grok` or `xai` detection, with model/account guardrails before any base-url or provider config is offered.
- Cursor follows a profile-aware editor path: detect the app/profile settings, show a dry-run settings diff, back up profile settings, apply only user-approved routing, verify, rollback, and clean up in Off mode.
- Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI get matching config-creation plans in their connector dossiers before any adapter is promoted from planned to supported.
- No connector may create or modify config until its manifest has config paths, account caveats, rollback strategy, Doctor verification, and fixture-home restore tests.

Implementation areas:

- `src/lib/plannedConnectors.ts`
- `src-tauri/src/client_adapters.rs`
- `src-tauri/src/lib.rs`
- `research/tool-compatibility-matrix.md`
- `docs/architecture.md`

Acceptance gates:

- Detection never mutates config.
- Automation is disabled until all safety gates pass.
- Doctor explains each blocked automation gate.
- OpenCode, Grok/xAI, Cursor, and every other planned connector have an explicit config-creation dossier before implementation starts.
- `npm run check:connectors` verifies the manifest contract.

Suggested commit:

- `Add Gemini CLI connector detection`

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

Goal: map planned connector contracts and define the first Gemini CLI detection-only slice.

Questions:

- What metadata does `plannedConnectors.ts` already require?
- What backend detection functions already exist for planned tools?
- What would a no-write Gemini detection result look like?
- Which script validates connector manifests?

Output:

- Connector contract map.
- Gemini detection-only proposal.
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
