# Mac AI Switchboard Product Roadmap Plan

This plan expands the rebrand and trust-hardening work into a broader product roadmap for Mac AI Switchboard. The goal is to make the app trustworthy enough to install, clear enough to debug, and useful enough to become the local control center for coding-agent optimization on macOS.

Mac AI Switchboard should stay local-first. The app can route Claude, Codex, and future agent traffic to remote model providers when the user chooses those tools, but switchboard state, config edits, Doctor checks, savings attribution, repo context packs, and add-on health should remain inspectable on the user's Mac.

## Product Principles

- Off means off: no local proxy listener, no client routing, no MCP restore, no LaunchAgent, and no hidden repair that re-enables routing.
- Local-only mode must be provable: no account, pricing, telemetry, support, or analytics network calls unless explicitly enabled.
- App identity belongs to Mac AI Switchboard; Headroom remains visible only as the underlying engine/runtime where technically accurate.
- Every config edit must be reversible, fenced, backed up, and visible in Doctor.
- Savings claims must be explainable by source, scope, and time window.
- Agent connector automation must start with detection and guided setup before mutating third-party config.
- Release readiness should be evidence-driven, not inferred from a successful local build.

## Current Issues and Concerns

### Trust and Identity

- Some docs and runtime surfaces still mix app identity with Headroom engine terminology.
- Terms are bundled, but Privacy Notice and legal review surfaces are not yet fully app-owned.
- Upstream account, pricing, legal, support, telemetry, and update assumptions need a deliberate keep, replace, or remove decision.
- Generated logo provenance and branding guardrails should be documented so inherited assets do not reappear.

### Mode Safety

- Off mode and RTK-only mode are high-trust controls. They need regression tests across launch, bootstrap, repair, relaunch, reboot, and uninstall paths.
- Doctor needs a mode inspector that proves what is active instead of only describing desired state.
- Already-running shells can retain old environment variables; the app should explain this clearly and detect likely stale shells where possible.

### Privacy and Network Boundaries

- Remote destinations need a complete inventory and allowlist.
- Local-only mode needs tests that reject unexpected network calls.
- Account/pricing screens should not imply Mac AI Switchboard owns upstream services unless those services have been replaced or intentionally adopted.
- Keychain service names and storage paths still use compatibility names; renaming them without migration risks losing user state.

### Release and Installation

- Public release readiness requires signed/notarized DMGs, updater keys, installed-app smoke evidence, and uninstall evidence.
- Local ad-hoc install success is useful but does not prove public release readiness.
- Uninstall copy and Doctor cleanup must match the actual filesystem, Keychain, LaunchAgent, shell, Claude, and Codex footprint.

### Observability and Savings Accuracy

- RTK currently reports global and project savings, but not a clean app-facing "this session" ledger.
- Savings should be attributed separately to Headroom engine, RTK, Repo Intelligence, MarkItDown, and other add-ons.
- Users need to know whether savings are measured, estimated, or inferred.

### Connector Expansion Risk

- Planned connectors include Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI.
- Each connector has different config files, provider semantics, account models, and rollback behavior.
- Automation should be gated behind detection, manual-safe instructions, backups, Doctor verification, Off-mode cleanup, and restore tests.

## Roadmap Phases

### Phase 1: Legal, Privacy, and Trust Surfaces

Goal: make the user-facing legal and trust story app-owned, local-readable, and honest about remote dependencies.

Tasks:

- Move Terms copy into `src/lib/legalText.ts` or a similar source module.
- Add a bundled Privacy Notice covering local file access, local config edits, update checks, telemetry, account/pricing calls, diagnostics, Keychain usage, and generated evidence.
- Add a Settings Legal section for Terms, Privacy, license, notices, and asset provenance.
- Remove or clearly label upstream legal, pricing, account, and support links.
- Add tests proving Terms and Privacy render offline.
- Add a terms-version note explaining when `REQUIRED_TERMS_VERSION` must change.

Acceptance checks:

- Terms and Privacy are readable without network access.
- No visible legal UI links to upstream Terms or Privacy pages unless intentionally labeled.
- Tests cover first-run gate, Settings Legal rendering, and version bump behavior.
- Account/pricing copy does not imply Mac AI Switchboard owns a service it does not own.

Suggested commit:

- `Own legal and privacy surfaces`

### Phase 2: Mode Inspector and Off-Mode Regression Gates

Goal: make switchboard modes provable instead of merely selectable.

Tasks:

- Add backend checks for listeners on `127.0.0.1:6767`, `127.0.0.1:8787`, managed shell blocks, Claude hooks, Codex provider blocks, MCP config, and LaunchAgents.
- Add a Doctor "Verify Off mode" action.
- Add a Mode Inspector panel showing requested mode, active mode, Headroom engine status, RTK hook status, Claude routing, Codex routing, MCP state, shell export state, and LaunchAgent state.
- Block repair actions from silently restoring Headroom routing when requested mode is Off or RTK-only.
- Add launch/bootstrap tests proving Off and RTK-only do not start the Headroom engine.
- Document stale shell behavior and restart guidance.

Acceptance checks:

- Fresh launch in Off mode leaves no proxy listener.
- Bootstrap completion in Off mode does not start Headroom.
- Doctor repair in Off mode does not restore routing.
- Mode Inspector makes each routing surface visible.
- Tests fail if a future launch path reintroduces hidden startup.

Suggested commit:

- `Add mode inspector and off-mode gates`

### Phase 3: Network and Local-Only Audit

Goal: make local-only mode testable and remote service use explicit.

Tasks:

- Inventory all remote destinations in code and docs.
- Add a central remote-destination registry for account API, pricing API, auth, telemetry, update feeds, support/contact URLs, Sentry, Clarity, Aptabase, and release feeds.
- Add a local-only test mode that rejects unexpected network calls.
- Hide or disable remote account/pricing/telemetry surfaces when local-only mode is enabled.
- Replace upstream support mailto/contact endpoints with Mac AI Switchboard-owned details or remove those actions.
- Add SSRF and external-link allowlist tests for user-opened URLs.

Acceptance checks:

- Local-only runs make no account, pricing, telemetry, or support requests.
- Every remaining remote URL is documented and intentionally allowed.
- Unexpected network attempts fail tests.
- Settings clearly shows whether remote telemetry/account features are enabled.

Suggested commit:

- `Audit remote service boundaries`

### Phase 4: Session Savings Ledger

Goal: give users an exact, scoped answer to "how many tokens or credits did this session save?"

Tasks:

- Define savings scopes: current app session, current repo, today, week, month, lifetime.
- Persist RTK command summaries with timestamps, project path, command family, input tokens, output tokens, saved tokens, and elapsed time.
- Persist Headroom engine compression events with client, model, request id, before/after tokens, saved tokens, and estimated cost.
- Add a unified savings ledger that merges Headroom, RTK, Repo Intelligence, MarkItDown, and future add-ons.
- Label each row as measured, estimated, or inferred.
- Add copy/export actions for a session summary.

Acceptance checks:

- Dashboard can answer current-session savings without relying on global RTK totals.
- Savings are grouped by source and time window.
- Exported summary includes equations and caveats.
- Tests cover empty session, RTK-only session, Headroom-only session, Full mode session, and mixed repo activity.

Suggested commit:

- `Add session savings ledger`

### Phase 5: Safety Rollback Center

Goal: make every managed config mutation visible and reversible from one place.

Tasks:

- Track each managed edit: target path, marker id, backup path, created time, last verified time, and owning feature.
- Add a Settings or Doctor "Rollback Center" listing Claude, Codex, shell, MCP, LaunchAgent, Keychain, and runtime edits.
- Add restore buttons for individual edits and a guarded "Undo all switchboard changes" action.
- Add dry-run cleanup output for uninstall.
- Add tests using temporary home directories and fixture configs.

Acceptance checks:

- Users can see exactly what the app changed.
- Each config edit can be restored from backup or safely removed.
- Uninstall disclosure matches the actual cleanup list.
- Tests prove rollback does not mutate unrelated config sections.

Suggested commit:

- `Add managed config rollback center`

### Phase 6: Connector Expansion Framework

Goal: add new agent connectors safely and repeatedly.

Tasks:

- Create a connector readiness contract with stages: detected, manual guide available, backup implemented, apply implemented, verify implemented, rollback implemented, Off cleanup implemented.
- Add connector manifests for Gemini CLI, OpenCode, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, Zed AI, and Grok / xAI CLI.
- For each connector, document config paths, provider/base-url semantics, account caveats, and rollback strategy.
- Add UI badges for "manual only", "automation gated", "verified automation", and "unsupported account/model".
- Start automation with one low-risk connector after the framework is tested.

Acceptance checks:

- Planned connectors show useful detection and manual setup without mutating config.
- Automation is disabled until backup, verify, rollback, and Off cleanup exist.
- Doctor can explain why a connector is not safely automatable yet.
- Adding a new connector does not require bespoke UI rewrites.

Suggested commit:

- `Add connector readiness framework`

### Phase 7: Repo Intelligence v2

Goal: turn Repo Intelligence from bounded packs into a graph-aware local context layer.

Tasks:

- Add tree-sitter or language-specific parsers for TypeScript, JavaScript, Python, Rust, Swift, Markdown, and shell scripts.
- Persist file hashes, parser versions, symbol counts, imports, reverse dependencies, likely tests, and stale status.
- Add graph-aware packs for implementation, verification, onboarding, risk review, and release handoff.
- Expose packs through local CLI and MCP-style APIs.
- Add Doctor checks for parser availability, corrupt graph storage, stale index, and missing repo paths.
- Keep indexing read-only and ignore-aware.

Acceptance checks:

- Context packs are smaller than naive file discovery and include token-savings estimates.
- Graph output identifies dependency hubs and likely tests.
- Disabling Repo Intelligence stops indexing without deleting user repos.
- Tests prove no project-file mutation and no network dependency.

Suggested commit:

- `Expand repo intelligence graph packs`

### Phase 8: Branding, Assets, and Release Evidence

Goal: make the app ready for real testers with app-owned assets and recorded evidence.

Tasks:

- Add asset provenance notes for the generated Mac AI Switchboard logo.
- Add a branding guard script for `logoipsum`, removed upstream logo imports, and stale app-name strings.
- Audit DMG artwork, screenshots, README images, release notes, and app icons.
- Build a local DMG, install `/Applications/Mac AI Switchboard.app`, and run installed smoke tests.
- Save evidence in `dist/local-installed-smoke-summary.md` or a release evidence doc.
- Keep signed/notarized release readiness separate from local ad-hoc success.

Acceptance checks:

- Launcher, tray, app icon, DMG, README, and docs use app-owned branding.
- Branding guard fails if old assets return.
- Local installed smoke evidence is fresh.
- Release readiness reports any missing signing/notarization/updater secrets.

Suggested commit:

- `Record product release evidence`

## Feature Backlog

### High Impact

- Session savings ledger with copyable summary.
- Mode Inspector and Verify Off mode.
- Rollback Center for managed config edits.
- Local-only certification mode.
- Connector readiness framework.
- Repo Intelligence graph packs.
- Release readiness dashboard.

### Medium Impact

- Savings anomaly alerts when output unexpectedly grows.
- Per-client savings trends for Claude, Codex, and future connectors.
- Codex large-context advisor that recommends compacting, RTK-only, or connector reset.
- Test relationship view in Repo Intelligence.
- Add-on health cards for RTK, MarkItDown, Ponytail, and Headroom engine.
- Import/export of app settings without secrets.
- Doctor evidence copy button for support/debug handoff.

### Later

- Storage/keychain migration from compatibility Headroom names to Mac AI Switchboard names.
- Signed public release channel and auto-update promotion workflow.
- Optional team/shared policy profiles.
- Optional local MCP server for cross-agent context packs and health checks.
- App Store distribution research, if sandbox and managed runtime constraints can be satisfied.

## Cross-Cutting Test Plan

- Frontend tests for legal panels, Mode Inspector, savings ledger, connector readiness badges, and rollback UI.
- Rust tests for Off mode startup gates, bootstrap gates, repair guards, remote URL allowlists, and cleanup logic.
- Fixture-home tests for shell profiles, Claude settings, Codex config, MCP config, LaunchAgents, and backup restore.
- Local-only tests that fail on unexpected network calls.
- Installed-app smoke tests for first launch, mode switching, RTK-only, Off, uninstall, and release evidence.
- Branding guard tests for removed assets and stale app identity strings.

## Recommended Implementation Order

1. Legal and Privacy surfaces.
2. Mode Inspector and Off-mode regression gates.
3. Network and local-only audit.
4. Session savings ledger.
5. Safety Rollback Center.
6. Connector readiness framework.
7. Repo Intelligence v2.
8. Branding and release evidence.

This order front-loads user trust and safety before expanding automation. It also creates the observability needed to tell whether later connector and savings features are actually working.

