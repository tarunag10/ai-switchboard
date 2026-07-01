# Changelog

## 2026-07-01

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
