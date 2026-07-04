# Refactor And Platform Readiness Plan

## Goals

- Shrink oversized files into focused modules with stable ownership.
- Keep every slice behavior-preserving unless the slice explicitly states a UI change.
- Preserve Mac app behavior while creating seams for Windows, Linux, and CLI reuse.
- Commit and push each verified slice independently.

## Current Hotspots

### Frontend

- `src/App.tsx`: about 11.7k lines before refactor. It mixes shell state, views, duplicate component implementations, Tauri calls, pricing, add-ons, Repo Intelligence, release tooling, and settings flows.
- `src/components/SettingsView.tsx`: about 1.9k lines. It mixes settings import/export, connectors, release readiness, rollback, autostart, and legal/support links.
- `src/components/ActivityFeed.tsx`, `RepoIntelligencePreview.tsx`, `RepoMapView.tsx`, `OptimizationView.tsx`: large feature views with direct Tauri calls.

### Backend

- `src-tauri/src/client_adapters.rs`: connector registry, setup, verification, rollback, macOS cleanup, shell mutation, and compatibility evidence.
- `src-tauri/src/lib.rs`: Tauri setup plus many command bodies.
- `src-tauri/src/state.rs`: runtime lifecycle, launch warming, upgrade flow, boot validation, and process control.
- `src-tauri/src/tool_manager.rs`: managed runtime layout, receipts, installers, plugin setup, RTK/headroom/MCP flows.

## Frontend Slices

1. Reuse already extracted UI components in `App.tsx`.
   - Start with `PlannedAddonCard` and `PlannedConnectorRoadmap`.
   - Then move to `OutputReductionChip`, `ClientSavingsTrendsCard`, `DailySavingsChart`, `AddonHealthStrip`, `AddonCard`, and `DoctorTimelineCard`.
   - Validation: focused component tests, `npm run build`, `npm run check:deployment`.

2. Extract parity versions of still-inline feature cards.
   - `SavingsCalculatorCard` should be migrated carefully because the extracted component already contains progressive-disclosure behavior.
   - `RepoIntelligencePreview` should preserve `onSummaryChange={setLatestRepoIntelligenceSummary}` and Repo Map callback wiring.

3. Introduce a frontend client seam.
   - Add a `SwitchboardClient` interface and Tauri implementation.
   - Migrate one feature at a time, starting with Repo Map.
   - Keep React components transport-agnostic so future Windows/Linux shells and browser tests can use the same UI logic.

## Backend Slices

1. Connector manifest and planning core.
   - Move pure connector catalog/status planning from `client_adapters.rs` into connector-focused modules.
   - Keep setup/write behavior unchanged.

2. Client setup backends.
   - Split per-client setup and verification into `claude`, `codex`, `opencode`, `windsurf`, `zed`, and shared managed-block/json helpers.

3. Platform paths and cleanup.
   - Create a `PlatformPaths` seam with macOS implementation first.
   - Move LaunchAgent, `~/Library`, shell profile, and cleanup target modeling behind explicit platform modules.

4. Tool manager core.
   - Separate managed runtime layout, receipts, install flows, and RTK/headroom/MCP installers.

5. Tauri command surface.
   - Move command bodies into `commands/*` modules and leave `lib.rs` as app setup plus command registration.

## Cross-Platform Target Boundaries

- `switchboard-core`: pure modes, Doctor findings, connector registry, savings attribution, repo intelligence summaries, rollback plans.
- `switchboard-platform`: OS-specific filesystem, shell, secure storage, autostart, process, and notification adapters.
- `switchboard-api`: use-case layer consumed by desktop apps and CLI.
- `switchboard-ui`: React views that depend on a client interface rather than direct Tauri imports.
- `switchboard-cli`: CLI commands backed by the same API/use cases as the desktop app.

## Validation Policy

- Frontend extraction: focused tests plus `npm run build` and `npm run check:deployment`.
- Backend extraction: focused Rust module tests plus `npm run test:desktop`.
- Connector/platform changes: add `npm run check:connectors`.
- Release-sensitive changes: add `npm run release:ready` and document expected signing/notarization blockers.
