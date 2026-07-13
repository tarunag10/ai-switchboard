# AI Switchboard Platform Rebrand Execution Tracks

Status: shipped for copy, documentation, compatibility, and evidence slices; live installed-app/reboot proof remains external
Parent plan: `docs/ai-switchboard-platform-rebrand-implementation-plan.md`

This file breaks the platform rebrand into sub-agent-ready tracks and implementation slices. Sub-agents are not launched from this side conversation, but the main implementation thread can use these tracks directly.

## Sub-Agent Tracks

### Track A: Brand and Copy Audit

Goal: identify every user-visible naming surface.

Inputs:

- `README.md`
- `docs/`
- `src/`
- `src-tauri/`
- website/deployment repo if separate

Tasks:

- Classify every reference as visible brand, technical identifier, migration note, or third-party attribution.
- Produce a rename map.
- Flag risky technical identifiers.

Deliverable:

- `docs/ai-switchboard-rebrand-audit.md`

### Track B: Runtime and Migration Safety

Goal: prevent broken installs during renaming.

Tasks:

- Inventory storage paths, keychain names, launch labels, updater config, receipts, and local DB paths.
- Design compatibility aliases for old Mac AI Switchboard paths.
- Add tests for legacy path discovery.
- Add repair/uninstall coverage for both old and new names.

Deliverable:

- migration implementation PR with tests and rollback evidence.

### Track C: UI and Desktop Rebrand

Goal: update visible app naming without breaking macOS-specific clarity.

Tasks:

- Replace visible Mac AI Switchboard copy with AI Switchboard or Switchboard.
- Label Mac-only controls where needed.
- Refresh screenshots.
- Verify focus, contrast, and layout after text changes.

Deliverable:

- UI rebrand PR with screenshots and Playwright/Tauri smoke evidence.

### Track D: CLI and Cross-Platform Surface

Goal: make Switchboard usable from CLI-first workflows.

Tasks:

- Define CLI command contract.
- Add or update command help.
- Add cross-platform smoke checks where possible.
- Document Linux/Windows support status.

Deliverable:

- CLI visibility PR with help snapshots and CI checks.

### Track E: Website and Release

Goal: make the public brand match the product direction.

Tasks:

- Update website hero, metadata, download copy, and roadmap.
- Add rename note for existing users.
- Update release checklist.
- Confirm DMG/download links remain correct.

Deliverable:

- website/release PR with live deployment proof.

### Track F: Legal, Attribution, and Licensing

Goal: keep the rebrand credible and compliant.

Tasks:

- Audit third-party tool mentions.
- Preserve license notices.
- Create consistent "integrates with" language.
- Ensure no copy implies Switchboard created upstream tools.

Deliverable:

- attribution and license PR.

## Implementation Slice Detail

### Slice 1: Planning and Audit

Files:

- `docs/ai-switchboard-platform-rebrand-implementation-plan.md`
- `docs/ai-switchboard-platform-rebrand-execution-tracks.md`
- `docs/ai-switchboard-rebrand-audit.md`
- `docs/plan-status-ledger.md`

Checks:

- `rg "Mac AI Switchboard|Headroom|RTK|Caveman|Ponytail|MarkItDown"`
- docs spelling/link check if available.

Commit:

- `Add AI Switchboard platform rebrand plan`

### Slice 2: Public Copy and Documentation

Files:

- `README.md`
- `docs/install.md`
- `docs/beta-smoke-test.md`
- `docs/remote-destinations.md`
- release docs.

Rules:

- Use AI Switchboard for product.
- Use Switchboard for short references.
- Use AI Switchboard for Mac for macOS-only install instructions.
- Keep legacy names in migration/troubleshooting sections.

Checks:

- docs grep for stale visible names.
- link check.

Commit:

- `Rebrand public docs to AI Switchboard`

### Slice 3: App Copy and UI

Files:

- React components.
- settings/legal/onboarding copy.
- screenshots.

Checks:

- frontend tests.
- UI screenshot review.
- text overflow review.

Commit:

- `Update app copy for AI Switchboard`

### Slice 4: Technical Identifier Compatibility

Files:

- Tauri config.
- app path helpers.
- launch/keychain/updater helpers.
- rollback/doctor tests.

Rules:

- Do not rename persistent identifiers without alias tests.
- Prefer visible rename first, technical migration second.
- Keep old-path discovery until at least one stable release after migration.

Checks:

- Rust tests for old and new path discovery.
- rollback/doctor smoke.
- installed-app smoke if available.

Commit:

- `Add legacy identity compatibility for AI Switchboard`

### Slice 5: CLI Brand and Commands

Files:

- CLI entrypoint.
- command help.
- CLI docs.
- CI smoke workflows.

Checks:

- `switchboard --help`
- `switchboard status`
- platform-specific smoke scripts.

Commit:

- `Make Switchboard CLI a first-class surface`

### Slice 6: Cross-Platform Support Matrix

Files:

- `docs/platform-support.md`
- README support table.
- UI platform labels.

Checks:

- docs link check.
- grep for misleading Mac-only claims.

Commit:

- `Document AI Switchboard platform support`

### Slice 7: Website and Download Flow

Files:

- website repo or deployment source.
- release notes.
- DMG/download metadata.

Checks:

- local website build.
- live deployment inspect.
- download link test.

Commit:

- `Reposition website as AI Switchboard`

### Slice 8: Release Evidence

Files:

- `dist/`
- release reports.
- smoke summaries.
- plan ledger.

Checks:

- local app smoke.
- CLI smoke.
- docs checks.
- CI green.
- download page proof.

Commit:

- `Record AI Switchboard rebrand release evidence`
