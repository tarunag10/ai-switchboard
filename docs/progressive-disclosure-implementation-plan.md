# Progressive Disclosure Implementation Plan

## Goal

Make AI Switchboard for Mac action-first by default. Each tab should answer:

- Is this working?
- What can I do now?
- Is setup automatic or manual?
- Where do I open deeper evidence if I need it?

Detailed evidence, file paths, config plans, backup/rollback notes, and provider-routing caveats should move behind explicit `Details`, `Why?`, `Verification`, or `Show setup steps` controls.

## Design Rules

1. Keep primary rows short.
   - Name, status, one short sentence, one primary action, optional compact secondary action.
2. Use button-driven inline disclosure.
   - Prefer real buttons with `aria-expanded` and `aria-controls`.
   - Avoid nested cards.
3. Match the label to the content.
   - `Details`: technical evidence or file paths.
   - `Why?`: short reason for a blocked/manual state.
   - `Show setup steps`: manual workflow.
   - `Verification`: commands, proof, logs, or smoke evidence.
4. Hide technical terms by default.
   - Examples: config gates, dossiers, dry-run previews, rollback evidence, watched paths, global storage, SQLite, sibling backups.
5. Preserve copy/export power tools.
   - Technical evidence can stay in copied reports, diagnostics, and release evidence.
   - The visible UI should not expose all of it at once.

## Existing Patterns To Reuse

- Row-level inline expansion from `ActivityFeed` expandable rows.
- Short icon help from connector help buttons.
- Popover-style help from `OutputReductionChip` for metric explanations.
- Link-button reveal pattern from Headroom logs for heavier diagnostics.

## Slice Plan

### Slice 1: Connector Rows Action First

Status: complete (2026-07-13).

Scope:

- Add explicit `Enable`, `Disable`, and `Manual setup` buttons beside connector names.
- Keep technical connector details behind existing info/details affordances.
- Make the automatic/manual distinction visible without requiring users to understand the switch control.

Validation:

- `npm run build`
- focused connector/dashboard frontend tests
- `npm run check:connectors`
- `npm run check:deployment`

Evidence: `SettingsConnectorPanel` renders an explicit Enable/Disable/Manual
setup action beside each connector, keeps the switch as an equivalent control,
and exposes an accessible setup-details button with a stable `aria-controls`
target. Covered by `src/components/SettingsConnectorPanel.test.tsx`.

### Slice 2: Connector Details Collapse

Status: complete (2026-07-13).

Scope:

- In Home/Settings connector rows, hide compatibility report, config checks, backend evidence, readiness stages, and capability rows behind `Details`.
- Default row should show:
  - connector name
  - status badge
  - automatic/manual setup label
  - primary action
  - one sentence

Acceptance:

- Cursor/Grok/Amazon Q rows no longer show config paths or safety-check paragraphs by default.
- Managed connectors still show an obvious `Enable` or `Repair` action.

Evidence: Settings connector rows render only status, setup mode, and actions by
default. Compatibility reports, config gates, backend checks, readiness stages,
capabilities, setup commands, gated config paths, and safety reasons render only
after the per-row details control is expanded. The collapsed/expanded contract is covered by
`src/components/SettingsConnectorPanel.test.tsx` and the connector helper
tests.

### Slice 3: Doctor Action-First

Status: complete (2026-07-13).

Scope:

- Collapse connector setup details in Doctor.
- Collapse Codex history retagging technical details.
- Keep `Repair all`, specific repairs, and verification actions visible.

Acceptance:

- Doctor first screen shows only issue title, severity, action, and short guidance.
- Evidence/details are one click away.

Evidence: `SwitchboardDoctorPanel` keeps repair, Verify Off, and approval
actions in the primary triage view while retagging and connector dossiers use
accessible details controls. Doctor UI/manual tests cover the collapsed state,
action grouping, and expansion behavior.

### Slice 4: Home Mode Inspector

Status: complete (2026-07-13).

Scope:

- Collapse Mode Inspector evidence rows.
- Show current mode, active mode, and primary repair action first.
- Hide port, shell hook, stale env, remote-service, and row evidence details.

Acceptance:

- Home is usable without reading backend/runtime internals.

Evidence: `SwitchboardPanel` shows mode status and repair actions first; port,
shell-hook, stale-env, remote-service, and row evidence are behind the Mode
Inspector Details control (`aria-expanded`/`aria-controls`). Covered by
`src/components/SwitchboardPanel.test.tsx`.

### Slice 5: Addons

Status: complete (2026-07-13).

Scope:

- Collapse add-on health checks, savings sources, verification commands, and long caveats.
- Keep install/enable/open actions visible.

Acceptance:

- Addons list scans as local tools with status and action, not a diagnostic report.

Evidence: Addon cards keep Install/Enable/Disable/Uninstall visible while
descriptions, RTK activity, readiness, and planned-addon evidence use info or
Learn more disclosures. `AddonCard`, `PlannedAddonCard`, and measured-savings
tests cover the action-first contract.

### Slice 6: Repo Intelligence

Status: complete (2026-07-13).

Scope:

- Collapse runtime/Doctor/connector verification copy in session summaries.
- Collapse graph examples, import edges, reverse hubs, symbols, and generated artifact paths.
- Keep index state, pack actions, and copy buttons visible.

Acceptance:

- Repo Intelligence first view shows “index/copy/use” workflow before graph internals.

Evidence: `RepoIntelligencePreview` keeps Index, copy, clear, session, and pack
actions visible; verification/mode reasoning and graph diagnostics are hidden
until Details/Learn more controls are expanded. `RepoMapView` keeps artifact
open/copy actions visible while generated paths and run output stay collapsed.
`RepoIntelligencePreview.test.tsx` and `RepoMapView.test.tsx` assert the
collapsed ARIA state and expansion.

### Slice 7: Usage And Savings

Status: complete (2026-07-13).

Scope:

- Keep totals, scope selector, and top savings sources visible.
- Collapse methodology, confidence, source details, and ledger explanations.

Acceptance:

- Usage explains savings at a glance; methodology is available through `Details`.

Evidence: `SavingsCalculatorCard` keeps scope, totals, equation, and copy
visible while source breakdown, confidence, ledger, and anomaly methodology
stay behind the Source details disclosure. `OptimizationView.test.tsx` and
savings calculator tests cover the action-first and copy contracts.

## Audit Findings

High-priority dense surfaces:

- Home Mode Inspector evidence.
- Doctor connector preview and Codex retagging details.
- Settings connector cards.
- Addons evidence grids and verification commands.
- Repo Intelligence session/graph details.
- Usage savings source breakdown.

Medium-priority dense surfaces:

- Home shortcut copy.
- Settings import/export notes.
- Optimize “Where this lives” and empty-state path details.
- Repo Map artifact paths.

## Commit Strategy

Ship one bounded slice at a time. Each slice should:

1. Keep unrelated copy and layout unchanged.
2. Add or reuse a disclosure control.
3. Run focused tests plus build.
4. Commit and push before moving to the next slice.
