# Changelog

## 2026-07-01

### Repo Intelligence Doctor cleanup docs

- Corrected architecture docs so stale, missing, moved, corrupt, or health-mismatched Repo Intelligence summaries are documented as automatic Clear index cleanup items instead of manual-only warnings.
- Added deployment readiness signals for the Repair all boundary: it may clear only Switchboard-managed saved index metadata and must not guess replacement repo paths or mutate repositories.

### Doctor repair smoke aggregation evidence

- Updated the local Doctor repair validation summary to explicitly cover aggregated Repair all timeline evidence.
- Added deployment readiness enforcement so future release checks fail if aggregated Repair all failure evidence drops out of the Doctor smoke.

### Repair all timeline evidence

- Added Doctor timeline coverage for aggregated Repair all failures so support exports preserve every failed sub-action while still scrubbing secrets and local paths.
- Guarded the new aggregated failure format from becoming an opaque single-error message in copied Doctor evidence.

### Repair all failure aggregation

- Hardened Doctor Repair all orchestration so independent repair actions keep running after an earlier action fails, then report all failures together.
- Shared the single-action repair dispatcher between individual Doctor repairs and Repair all so behavior stays consistent across runtime, managed connector, RTK, add-on, Repo Intelligence, and MCP repairs.

### Managed client batch repair resilience

- Hardened Doctor's all-managed-client repair flow so it attempts every installed managed connector before reporting failures, preventing one drifted connector from blocking later managed connector repairs.
- Added batch repair helpers that preserve strict failure reporting while allowing partial successful repairs to refresh runtime status.

### Managed connector registry invariant

- Added backend coverage that the manifest-managed connector set is exactly Claude Code, Codex, Gemini CLI, OpenCode, Windsurf, and Zed AI.
- Guarded promoted managed connectors so any manifest-managed tool must have a native or promoted apply/verify/repair setup path before Doctor can advertise it as managed.

### Retained connector Doctor wording

- Aligned planned-connector Doctor body, beta smoke expectations, and deployment readiness checks around retained connector native-routing gates so managed connectors are not described as manual-only.
- Kept the release checklist tied to the current degraded-mode repair contract for runtime, managed clients, and RTK.

### Doctor degraded-mode routing guidance

- Clarified degraded-mode Doctor guidance so managed connectors remain automatic repair targets while only retained connector native-routing gates stay manual until backup, verify, rollback, and Off cleanup evidence is promoted.
- Updated Doctor copy coverage to guard the managed-client versus retained-connector distinction.

### Gemini rollback cleanup copy

- Aligned the Gemini managed-change record with the backend rollback preview so cleanup copy names Switchboard-owned shell and sidecar blocks instead of a vague sidecar dossier.
- Added managed-change coverage to prevent Gemini rollback copy from drifting away from the executable backend cleanup contract.

### Plugin cleanup rollback copy

- Aligned the add-on managed-change record with the backend Ponytail cleanup contract: automatic rollback now promises only Switchboard-receipted plugin registration removal, while backup-file sweeping remains manual until a stricter allowlist exists.
- Added frontend coverage so the rollback plan cannot regress to claiming automatic backup sweeping for add-ons.

### Rollback plan execution copy

- Updated rollback plan safety copy to reflect the current backend preview, exact-confirmation, and dedicated cleanup flows instead of stale manual-only restore wording.

### Gemini backend evidence wording

- Updated backend Gemini connector metadata and compatibility evidence so app/session payloads describe sibling rollback backups instead of stale sidecar evidence.

### Doctor repair evidence coverage

- Expanded local Doctor repair validation to cover the managed repair post-write verification guard and require failed-repair timeline evidence in deployment readiness checks.

### Doctor timeline failed repair evidence

- Added failed Doctor repair details to copied timeline evidence, including scrubbed verification-failure text from automatic repair attempts.

### Doctor repair verification guard

- Made Doctor-managed client repairs fail with verification details when setup writes complete but post-write verification still fails, preventing false-success repair loops.

### Backend smoke-test stability

- Hardened backend process-spawn smoke tests by serializing PATH-mutating Claude CLI coverage and using production-equivalent smoke timeouts in Headroom and markitdown helper tests.

### Doctor repair-all action normalization

- Added backend normalization for Doctor Repair all so duplicate repair actions are skipped and all-managed-client repair takes precedence over individual managed-client repairs.

### Gemini routing evidence wording

- Updated Gemini managed connector copy, compatibility evidence, and handoff script text so routing verification describes shell exports plus sibling rollback backups instead of stale sidecar evidence.

### Codex provider repair-ready row

- Updated the Codex provider-block Mode Inspector fallback so direct provider routing is described as repair-ready instead of a passive not-routed state.

### Managed footprint backup pattern guard

- Added backend regression coverage so managed-footprint evidence keeps the current `*.headroom-backup-*` rollback backup pattern and rejects the stale `*.headroom.bak` suffix.

### Rollback inventory backup pattern

- Updated rollback inventory, uninstall, and managed-footprint copy to use the backend's real `*.headroom-backup-*` backup naming pattern instead of the stale `*.headroom.bak` suffix.

### Native connector dossier wording

- Updated OpenCode, Windsurf, and Zed managed connector dossier copy so promoted native config rows describe provider/settings verification and sibling rollback backups instead of stale sidecar verification.

### Promoted editor rollback backup guard

- Corrected Zed rollback preview evidence to use the real `zed-ai-routing` record id and added fixture-home coverage proving Windsurf and Zed reject rollback backups outside their managed config directories.

### Windsurf safe-apply rollback coverage

- Added fixture-home coverage proving the promoted Windsurf safe-apply flow can preview, apply, verify, and roll back from its sibling backup while preserving unmanaged settings.

### Windsurf native routing regression coverage

- Added fixture-home coverage proving Windsurf managed setup writes, verifies, and Off cleanup removes only the Switchboard-owned native routing keys while preserving user settings.

### Managed connector repair-ready wording

- Updated managed connector dashboard copy so repairable direct routing reads as repair-ready, and clarified the gated manual automation fallback label.

### Codex backend repair-ready Doctor body

- Updated backend Codex Doctor issue copy so detected or drifted Codex routing is described as repair ready while retaining the managed provider evidence details.

### Doctor gated connector action label

- Updated Doctor action labels so gated connector readiness issues show as gated setup instead of a generic manual step.

### First-run no-client Doctor guidance

- Updated the no-managed-client Doctor guidance so first-run setup tells users when automatic repair becomes available instead of showing a generic no-repair message.

### Backend managed connector repair-ready Doctor issue

- Updated backend Doctor issues for detected-but-unrouted managed clients so repairable setup is titled as repair-ready and scoped to that managed client.

### Installed gated connector control reason

- Updated installed gated connector control copy so detected tools describe reversible setup evidence gates instead of saying automatic routing is not available yet.

### Codex provider direct repair guidance

- Updated the Mode Inspector Codex provider-block direct state so it points users back to the repair-ready Codex routing row instead of stopping at a passive not-routed message.

### Managed connector repair-ready routing status

- Updated Mode Inspector managed connector rows so detected-but-direct managed tools show a repair-ready state and point at the one-click repair action.

### Agent connector add-on gated status

- Updated the Agent Connectors add-on card so the visible status and disabled action describe gated readiness instead of planned or coming soon support.

### Doctor no-pending connector copy

- Updated Doctor no-pending connector guidance so promoted setup coverage is described as managed or gated instead of pending.

### Doctor gated connector preview count

- Updated Doctor connector-readiness preview copy and test fixtures so connector evidence is described as gated readiness instead of pending or planned tools.

### Connector dossier gated status states

- Updated retained connector dossier status labels and gated capability-row states so cards no longer present blocked setup work as planned.

### Connector dossier badge gating labels

- Updated connector dossier capability badges so gated setup work is labeled as gated instead of planned or pending.

### Dashboard gated connector control copy

- Updated connector dashboard control states so unavailable non-managed connectors are labeled as gated setup instead of planned or coming soon.

### Implementation plan gated connector language

- Updated the implementation plan and backend connector test diagnostics so roadmap and fixture messages use gated connector language instead of planned connector language.

### Lifecycle docs gated connector language

- Updated install and adapter lifecycle docs so non-managed connector promotion guidance uses gated connector language instead of planned connector language.

### Release and support connector labels

- Updated release readiness report output and connector support docs to describe retained non-managed connectors as gated/guided instead of planned.

### Backend Repo Intelligence gated connector note

- Updated the Tauri Repo Intelligence connector readiness payload so backend handoffs use connector-native gated setup wording instead of planned connector wording.

### Gated connector status badges

- Updated planned-status connector badges and dashboard fallback labels to show gated setup state instead of planned setup state.

### Gated connector unavailable reasons

- Updated Home and Settings unavailable-reason copy for unpromoted connectors so missing tools explain the gated setup evidence instead of saying the adapter is planned.

### Doctor gated connector issue title

- Updated the backend Doctor issue title/body for detected unpromoted coding tools so Doctor reports gated connector readiness instead of planned coding tools.

### Gated connector setup details

- Updated Home, Settings, connector dossier, and connector verifier detail copy so unpromoted connectors are described as gated connector readiness instead of planned connectors.

### Connector roadmap and checklist wording

- Updated connector roadmap accessibility labels, checklist headers, and Repo Intelligence safety notes to avoid stale planned-connector wording in active output.

### Repo Intelligence connector handoff wording

- Updated Repo Intelligence handoff Markdown, session summaries, and architecture copy so agent-facing exports refer to connector readiness instead of planned connectors.

### Connector verifier output wording

- Updated the connector verifier and implementation-plan references so active check output reports gated native-write dossiers instead of pending planned rows.

### Connector checklist copy label

- Updated Home and Settings connector checklist copy labels so success feedback no longer refers to stale planned-tool wording.

### Connector readiness summary labels

- Updated Home and Settings connector readiness summary labels to use connector readiness wording instead of stale planned-tool wording.

### Doctor connector dossier export wording

- Updated Doctor connector dossier exports so the no-pending state refers to connector-native write dossiers instead of stale planned connector dossiers.

### Repo Memory MCP editor connector boundary

- Updated the Repo Memory MCP connector boundary so Windsurf and Zed MCP bridge setup is explicitly separate from Windsurf editor-settings routing and Zed assistant-settings routing.

### Repo Intelligence API editor connector wording

- Synced the Tauri Repo Intelligence handoff API with the frontend and CLI wording for Windsurf editor-settings routing and Zed assistant-settings routing.

### Zed connector routing wording

- Aligned Zed connector docs, setup copy, and frontend readiness dossiers with the backend assistant-settings routing lifecycle, and guarded the docs signal.

### Windsurf connector routing wording

- Aligned Windsurf connector docs and frontend readiness dossiers with the backend editor-settings routing lifecycle, replacing stale provider-routing wording and guarding the docs signal.

### Release verifier connector wording

- Updated release verification output to check connector registry parity without stale planned-connector wording, and added a deployment-readiness guard for the label.

### Release readiness connector evidence wording

- Updated the shareable release gate copy so static smoke preflight requires managed connector readiness evidence instead of the old planned-connector safety wording.

### Frontend Doctor gated-connector guidance

- Aligned frontend Doctor degraded-mode and connector guidance with the backend wording so manual follow-up is limited to connector-specific native routing gates with backup, verify, rollback, and Off cleanup evidence.

### Backend Doctor gated-connector wording

- Clarified backend Doctor degraded-mode and gated-connector copy so unpromoted tools are described as readiness-covered with connector-specific native routing gates, not as generally unfinished setup.

### Doctor connector readiness wording

- Updated Doctor connector readiness copy and exports so promoted managed coverage no longer appears under stale planned-connector headings, while pending connector gates remain explicit when they exist.

### Connector compatibility evidence labels

- Updated connector compatibility cards so promoted managed routing evidence renders as routing evidence instead of being prefixed as blocked, while unpromoted provider/settings blockers still show as blocked.

### Connector readiness summary counts

- Updated Settings connector readiness summary counts and copy to include managed connector dossiers, so promoted managed routes are counted as repairable connector evidence instead of being hidden behind planned-only wording.

### Connector card readiness evidence

- Updated connector cards to use managed connector dossiers as well as planned dossiers, so promoted Gemini/OpenCode/Windsurf/Zed readiness and config-plan evidence can render in the same card surfaces.
- Fixed connector compatibility blocks to render the detected version string instead of a hard-coded `0.0.0`.

### Rollback Center undo-all safety copy

- Updated undo-all rollback preview safety notes so backend-allowlisted Windsurf and Zed native rows are named alongside Codex, Gemini, OpenCode, and retained sidecar rollback paths.

### Mode Inspector repair detail

- Updated managed connector Direct rows in Mode Inspector so repairable routing drift says Repair will re-apply reversible managed setup and verify routing evidence.

### Connector phase baseline wording

- Updated the Agent Control Center phase 6 baseline and roadmap shipped-state copy so connector readiness coverage is distinct from promoted Gemini/OpenCode/Windsurf/Zed managed routing and retained sidecar/readiness coverage for unpromoted tools.

### Doctor connector coverage copy

- Updated Doctor connector guidance and copied dossiers so the no-pending-planned state says managed connector coverage with promoted routing evidence instead of stale managed sidecar coverage.

### Connector roadmap lifecycle wording

- Updated the roadmap, Repo Intelligence plan, and Agent Control Center checkpoint so promoted Gemini/OpenCode/Windsurf/Zed managed routing is distinct from sidecar/readiness coverage for unpromoted connectors.
- Clarified that remaining native/provider writes stay gated for Cursor, Continue, Goose, Aider, Grok / xAI CLI, Qwen Code, and Amazon Q.

### Repo Intelligence managed handoff readiness

- Synced Repo Intelligence handoff metadata across frontend, CLI, and Tauri API paths so Gemini CLI, OpenCode, Windsurf, and Zed AI report promoted managed routing instead of stale manual-provider routing.
- Added managed Windsurf/Zed safety dossiers and readiness coverage so frontend handoffs include enabled automation evidence with backup, apply, verify, rollback, and Off cleanup status.
- Added frontend and Rust regressions for managed handoff readiness.

### Managed connector repair audit

- Fixed Doctor so installed managed connectors that are detected but still Direct now get a repairable issue instead of only a passive warning.
- Added a Codex-specific Doctor repair issue for the detected-but-unrouted state, including missing `OPENAI_BASE_URL` and provider-block setup.
- Added Mode Inspector repair actions for Direct managed connectors.
- Filled managed Zed connector metadata so connector validation covers config surfaces, manual workflow, and Off-mode cleanup wording.
- Validated with frontend, desktop, connector, build, and Doctor-repair smoke checks.

### Connector status documentation sync

- Updated connector support docs, install notes, architecture docs, and the compatibility matrix so Gemini CLI, OpenCode, Windsurf, and Zed AI are documented as managed where their lifecycle is now implemented.

### Backend connector metadata sync

- Updated backend connector readiness metadata for Gemini CLI, OpenCode, Windsurf, and Zed AI so Doctor, Settings, and Repo Intelligence fallback copy no longer inherit manual-era routing language.

### Repo Memory MCP deployment gate sync

- Restored deployment-readiness wording in the Repo Memory MCP guide so Cursor, Windsurf, and Zed bridge setup stays separate from provider routing and the full connector lifecycle evidence gate remains visible.

### Release readiness connector summary

- Changed release readiness reporting to derive managed connector counts and rows from `connectors/manifest.json`, with schema checks that fail if the report drifts from the manifest.

### Connector checker summary clarity

- Updated the connector metadata checker output to distinguish manifest-managed connectors from managed connector dossiers and promoted sidecar dossiers.

### Connector roadmap truth sync

- Updated the active roadmap docs to distinguish fully managed manifest connectors from sidecar/readiness-covered connector dossiers, and to reflect Windsurf/Zed as promoted editor-routing surfaces instead of blocked editor-write plans.

### Managed routing metadata refresh

- Replaced stale Gemini/OpenCode manual-routing backend and Repo Intelligence copy with their promoted managed routing lifecycles and rollback evidence.
- Refreshed Windsurf and Zed backend detector evidence so managed editor-routing connectors no longer describe themselves as unimplemented manual handoffs.

### Repo handoff managed-routing safety

- Updated Repo Intelligence and connector compatibility UI logic so Gemini, OpenCode, Windsurf, and Zed handoffs are no longer marked as manual-provider-routing when their managed connectors are promoted.

### Mode Inspector managed connector rows

- Added individual Mode Inspector routing rows for installed managed connectors beyond Codex and Claude, with repair actions for direct Gemini/OpenCode/Windsurf/Zed-style setup drift.

### Targeted managed connector Doctor repairs

- Added per-connector Doctor repair actions for installed managed connectors so Gemini/OpenCode/Windsurf/Zed-style direct routing drift can be fixed one tool at a time while preserving bulk Repair all behavior.
- Synced backend connector tests and Gemini/Windsurf/Zed evidence copy with the promoted managed routing lifecycle.

### Repo Memory MCP Node resolution

- Hardened Repo Memory MCP setup and status checks to resolve Node from common macOS install paths when the GUI app PATH does not include Homebrew or `/usr/local/bin`, preventing false missing-Node Doctor loops.

### Windsurf setup copy

- Fixed duplicated Windsurf settings-path wording in the connector setup details shown by the app.

### Mode Inspector targeted connector repairs

- Updated Home Mode Inspector repair buttons for installed managed connectors to call per-tool repair actions instead of the bulk client repair path.

### Managed connector smoke evidence wording

- Updated smoke, release, and beta-test evidence wording to require broader managed connector readiness evidence instead of stale Gemini-only dry-run preview wording.

### Targeted verified-connector Doctor repairs

- Updated Doctor issues for enabled managed connectors whose routing no longer verifies to repair the specific connector instead of running the bulk client repair path.

### Mode Inspector unverified connector repairs

- Updated Mode Inspector managed connector rows in the Needs test state to run the same targeted repair action as Doctor instead of only opening connector settings.

### Bulk managed-client repair copy

- Renamed the remaining bulk client repair action to clarify that it reapplies setup for every installed managed client, distinct from per-connector repairs.

### Repo Intelligence session routing checklist

- Updated beta smoke coverage to require promoted managed-routing session handoffs to report non-manual provider routing while preserving manual routing for connectors that still require it.

### Repo Intelligence Doctor API copy

- Updated Doctor support copy for `get_agent_handoff` to describe managed connector readiness dossiers instead of planned-only readiness.

### Implementation plan connector truth

- Refreshed the main implementation plan support matrix and connector expansion order so OpenCode, Windsurf, Zed AI, and Gemini CLI match their current managed status.

### Zed native rollback routing

- Fixed promoted Zed rollback previews and execution to use the native settings backup/restore path before falling back to sidecar cleanup.

### Zed Off cleanup

- Wired Zed disable/Off cleanup to remove only the native Switchboard-managed settings routing keys and markers, preserving unrelated user settings.

### Connector smoke evidence copy

- Updated smoke-preflight and roadmap evidence text to distinguish promoted native-routing connectors from guided connectors whose native config mutation remains gated.

### Rollback Center connector inventory

- Updated Windsurf and Zed Rollback Center records to describe native editor settings routing, backup paths, markers, and restore boundaries instead of stale sidecar dossiers.

### Rollback roadmap truth

- Synced roadmap and Agent Control Center docs with the promoted OpenCode, Windsurf, and Zed native apply/rollback paths and the remaining sidecar-only connector scope.

### Native editor rollback fallback cleanup

- Removed stale Windsurf/Zed entries from the sidecar rollback fallback table so promoted editor connectors always resolve through native settings rollback paths.

### Connector manual-gate copy

- Clarified add-on, roadmap, and beta smoke copy so promoted managed connector routing drift is repairable while only remaining unpromoted native config gates stay manual.
