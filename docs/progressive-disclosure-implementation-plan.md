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

Status: in progress.

Scope:

- Add explicit `Enable`, `Disable`, and `Manual setup` buttons beside connector names.
- Keep technical connector details behind existing info/details affordances.
- Make the automatic/manual distinction visible without requiring users to understand the switch control.

Validation:

- `npm run build`
- focused connector/dashboard frontend tests
- `npm run check:connectors`
- `npm run check:deployment`

### Slice 2: Connector Details Collapse

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

### Slice 3: Doctor Action-First

Scope:

- Collapse connector setup details in Doctor.
- Collapse Codex history retagging technical details.
- Keep `Repair all`, specific repairs, and verification actions visible.

Acceptance:

- Doctor first screen shows only issue title, severity, action, and short guidance.
- Evidence/details are one click away.

### Slice 4: Home Mode Inspector

Scope:

- Collapse Mode Inspector evidence rows.
- Show current mode, active mode, and primary repair action first.
- Hide port, shell hook, stale env, remote-service, and row evidence details.

Acceptance:

- Home is usable without reading backend/runtime internals.

### Slice 5: Addons

Scope:

- Collapse add-on health checks, savings sources, verification commands, and long caveats.
- Keep install/enable/open actions visible.

Acceptance:

- Addons list scans as local tools with status and action, not a diagnostic report.

### Slice 6: Repo Intelligence

Scope:

- Collapse runtime/Doctor/connector verification copy in session summaries.
- Collapse graph examples, import edges, reverse hubs, symbols, and generated artifact paths.
- Keep index state, pack actions, and copy buttons visible.

Acceptance:

- Repo Intelligence first view shows “index/copy/use” workflow before graph internals.

### Slice 7: Usage And Savings

Scope:

- Keep totals, scope selector, and top savings sources visible.
- Collapse methodology, confidence, source details, and ledger explanations.

Acceptance:

- Usage explains savings at a glance; methodology is available through `Details`.

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
