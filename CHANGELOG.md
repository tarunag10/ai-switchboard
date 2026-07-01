# Changelog

## 2026-07-01

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
