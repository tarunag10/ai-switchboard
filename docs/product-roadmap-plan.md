# Mac AI Switchboard Product Roadmap Plan

This plan expands the rebrand and trust-hardening work into a broader product roadmap for Mac AI Switchboard. The goal is to make the app trustworthy enough to install, clear enough to debug, and useful enough to become the local control center for coding-agent optimization on macOS.

Mac AI Switchboard should stay local-first. The app can route Claude, Codex, and future agent traffic to remote model providers when the user chooses those tools, but switchboard state, config edits, Doctor checks, savings attribution, repo context packs, and add-on health should remain inspectable on the user's Mac.

## Current Checkpoint - 2026-06-29

Recent verified build: `/Applications/Mac AI Switchboard.app` was rebuilt and locally reinstalled from `tarun/local-switchboard` at commit `0a96d26 Guard bootstrap in off mode`. Local installed smoke and codesign verification passed for the ad-hoc app. Public release readiness is still blocked by signing, notarization, updater, and public installed-smoke evidence.

Shipped:

- Off and RTK-only modes now gate launch warmup, threaded bootstrap, and legacy synchronous bootstrap so optimization does not silently restart after the user turns it off.
- Doctor already reports Off-mode violations when Headroom, managed client routing, or RTK remain active against a requested Off mode.
- Managed connector sidecar coverage exists for Gemini CLI, OpenCode, Grok/xAI CLI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, and Zed.
- Gemini has managed shell base-url routing. OpenCode has the first promoted provider-config write path with preview, exact confirmation, backup, apply, verify, and rollback coverage.
- Cursor and Windsurf now surface detected editor settings files as evidence while native writes remain blocked.
- Repo Intelligence Start Agent Session, read-only packs, handoffs, CLI exports, and repo-memory MCP smoke transport are usable.
- Savings ledger rows now distinguish measured, estimated, and inferred events across Headroom, RTK, Repo Intelligence, MarkItDown, Ponytail, Caveman, and Compact Chinese.
- Rollback Center has guarded preview/execution for ready allowlisted rows, including Codex/OpenCode restore paths and managed sidecar cleanup.
- Remote destination registry, support-link routing, external-link SSRF guards, local-only backend refusal for account/billing/contact commands, branding iconset provenance, local DMG build, and local installed smoke evidence are shipped.

Left:

- App-owned Terms, Privacy Notice, Settings Legal surfaces, and terms-version policy are bundled and covered by focused tests.
- Complete the Mode Inspector as a user-facing proof surface for every listener, hook, MCP, shell block, provider block, LaunchAgent, and stale-shell caveat.
- Add installed-app/relaunch/reboot smoke evidence for Off mode, RTK-only mode, Doctor repair guards, and Rollback Center survival.
- Promote native config mutation connector by connector beyond Gemini/OpenCode only after parse, dry-run diff, exact backup, apply, verify, rollback, Doctor repair, fixture-home restore tests, and Off cleanup are proven.
- Turn repo-memory MCP into a real app-supervised long-running local service instead of only persisted app-session state plus smoke-tested stdio transport.
- Replace inferred Ponytail, Caveman, Compact Chinese, MarkItDown, and some Repo Intelligence savings rows with stronger measured counters where trustworthy evidence exists.
- Deepen Repo Intelligence with tree-sitter or language-aware parser/index versions, imports, reverse dependencies, graph-aware packs, and richer Doctor health checks.
- Complete public release readiness with signed/notarized DMG, updater artifacts, public installed-smoke proof, uninstall evidence, and release-panel wiring.

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

- Some runtime/file-path surfaces still use compatibility Headroom naming where migration would risk user state; visible app identity should continue moving to Mac AI Switchboard while keeping "Headroom engine" for the optimizer.
- Terms, Privacy Notice, Settings Legal, and terms-version policy are app-owned and readable without network access.
- Upstream account, pricing, telemetry, and update assumptions need a deliberate keep, replace, or remove decision before public release. Support actions now route to this repository's GitHub Issues.
- Generated logo provenance and branding guardrails are partly shipped; keep them enforced as release assets and screenshots change.

### Mode Safety

- Off mode and RTK-only mode now gate launch and bootstrap startup paths, including the legacy synchronous bootstrap command.
- Remaining mode-safety work is installed-app/relaunch/reboot evidence, stronger Doctor repair guards, LaunchAgent/MCP/listener proof, and stale-shell detection/copy.
- Doctor still needs a complete Mode Inspector that proves what is active instead of only describing desired state.
- Already-running shells can retain old environment variables; the app should explain this clearly and detect likely stale shells where possible.

### Privacy and Network Boundaries

- Remote destinations are inventoried in `docs/remote-destinations.md` and guarded by governance/deployment checks; keep the registry current as release, update, telemetry, account, and provider surfaces change.
- Local-only backend guards reject account, billing, and contact entrypoints before auth or HTTP setup; broader unexpected-network tests still need completion.
- Account/pricing screens should not imply Mac AI Switchboard owns upstream services unless those services have been replaced or intentionally adopted.
- Keychain service names and storage paths still use compatibility names; renaming them without migration risks losing user state.

### Release and Installation

- Public release readiness requires signed/notarized DMGs, updater keys, installed-app smoke evidence, and uninstall evidence.
- Local ad-hoc build/install and local installed-smoke evidence are shipped and useful, but they do not prove public release readiness.
- Uninstall copy and Doctor cleanup must match the actual filesystem, Keychain, LaunchAgent, shell, Claude, and Codex footprint.

### Observability and Savings Accuracy

- A session savings ledger exists and labels rows as measured, estimated, or inferred.
- Headroom and RTK measured events are the strongest evidence today; Repo Intelligence has estimated best-pack events; MarkItDown, Ponytail, Caveman, and Compact Chinese still rely mostly on inferred template events.
- Remaining work is to make add-on counters more exact and to explain current session, repo, today, and all-time scopes without mixing them.

### Connector Expansion Risk

- Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI now have managed sidecar lifecycle coverage.
- The remaining connector risk is provider-specific native config mutation: Gemini and OpenCode are furthest along, while Cursor/Windsurf/Zed/Continue/Goose/Aider/Grok/Qwen/Amazon Q still need connector-specific safe writes.
- Native config automation should stay gated behind detection, manual-safe instructions, dry-run diffs, backups, Doctor verification, Off-mode cleanup, and fixture-home restore tests.

## Roadmap Phases

### Phase 1: Legal, Privacy, and Trust Surfaces

Goal: make the user-facing legal and trust story app-owned, local-readable, and honest about remote dependencies.

Tasks:

- Move Terms copy into `src/lib/legalText.ts` or a similar source module: shipped.
- Add a bundled Privacy Notice covering local file access, local config edits, update checks, telemetry, account/pricing calls, diagnostics, Keychain usage, and generated evidence: shipped.
- Add a Settings Legal section for Terms, Privacy, license, notices, and asset provenance: Terms and Privacy shipped; license/notices/asset provenance links remain to be folded into Settings.
- Remove or clearly label upstream legal, pricing, account, and support links: account and paid API copy now states those APIs are not included.
- Add tests proving Terms and Privacy render offline: shipped.
- Add a terms-version note explaining when `REQUIRED_TERMS_VERSION` must change: shipped in Settings Legal.

Acceptance checks:

- Terms and Privacy are readable without network access.
- No visible legal UI links to upstream Terms or Privacy pages unless intentionally labeled.
- Tests cover first-run gate, Settings Legal rendering, and version bump behavior.
- Account/pricing copy does not imply Mac AI Switchboard owns a service it does not own.

Suggested commit:

- `Own legal and privacy surfaces`

### Phase 2: Mode Inspector and Off-Mode Regression Gates

Goal: make switchboard modes provable instead of merely selectable.

Status: partially shipped. Off/RTK-only launch and bootstrap guards are in place, and Doctor can flag active routing evidence while Off is requested. The remaining work is the full proof surface, repair blocking, and installed-app persistence evidence.

Tasks:

- Add backend checks for listeners on `127.0.0.1:6767`, `127.0.0.1:8787`, managed shell blocks, Claude hooks, Codex provider blocks, MCP config, and LaunchAgents.
- Add a Doctor "Verify Off mode" action.
- Add a Mode Inspector panel showing requested mode, active mode, Headroom engine status, RTK hook status, Claude routing, Codex routing, Repo Memory MCP lifecycle state, shell export state, and LaunchAgent state.
- Block repair actions from silently restoring Headroom routing when requested mode is Off or RTK-only.
- Extend launch/bootstrap tests into installed-app restart, relaunch, reboot, and Doctor-repair smoke evidence.
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

Status: partially shipped. The ledger UI and backend durable events exist with measured, estimated, and inferred confidence labels. The remaining work is stronger live counters and clearer scoped rollups.

Tasks:

- Define savings scopes: current app session, current repo, today, week, month, lifetime.
- Persist RTK command summaries with timestamps, project path, command family, input tokens, output tokens, saved tokens, and elapsed time.
- Persist Headroom engine compression events with client, model, request id, before/after tokens, saved tokens, and estimated cost.
- Keep the unified savings ledger accurate across Headroom, RTK, Repo Intelligence, MarkItDown, Ponytail, Caveman, Compact Chinese, and future add-ons.
- Replace inferred add-on rows with measured counters when runtime evidence is trustworthy.
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

Status: partially shipped. Rollback inventory, guarded previews, selected native restore paths, sidecar cleanup, and undo-all orchestration for ready rows exist. The remaining work is broader native restore coverage and installed-app survival evidence.

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

Status: framework shipped; native writes are intentionally gated. Managed sidecar lifecycle, readiness metadata, dry-run previews, handoff dossiers, and release evidence checks exist across the managed connector set. OpenCode is the first promoted provider-config write path; Gemini has managed shell routing.

Tasks:

- Preserve the shipped connector readiness contract with stages: detected, manual guide available, backup implemented, apply implemented, verify implemented, rollback implemented, Off cleanup implemented.
- Keep connector manifests current for Gemini CLI, OpenCode, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, Zed AI, and Grok / xAI CLI.
- For each connector, document native config paths, provider/base-url semantics, account caveats, credential boundaries, and rollback strategy.
- Keep UI badges for "manual only", "automation gated", "verified automation", and "unsupported account/model".
- Promote native config automation connector by connector after the managed sidecar framework and fixture-home restore tests prove the path.

Acceptance checks:

- Managed connectors show useful detection, sidecar lifecycle state, and native config gates without mutating native config.
- Automation is disabled until backup, verify, rollback, and Off cleanup exist.
- Doctor can explain why a connector is not safely automatable yet.
- Adding a new connector does not require bespoke UI rewrites.

Suggested commit:

- `Add connector readiness framework`

### Phase 7: Repo Intelligence v2

Goal: turn Repo Intelligence from bounded packs into a graph-aware local context layer.

Status: v1 is usable. Start Agent Session, CLI handoffs, read-only packs, parser/index health fields, modern JS/TS/Python symbol extraction, and repo-memory MCP smoke transport are shipped. The remaining work is deeper graphing and supervised service behavior.

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

Status: local evidence is shipped; public evidence is not. The app has app-owned iconset provenance, branding guards, local unsigned DMG build/install, local installed smoke, and release-readiness reports. The remaining work is signed/notarized public release evidence and uninstall proof.

Tasks:

- Add asset provenance notes for the generated Mac AI Switchboard logo.
- Add a branding guard script for `logoipsum`, removed upstream logo imports, and stale app-name strings.
- Audit DMG artwork, screenshots, README images, release notes, and app icons.
- Keep building local DMGs, installing `/Applications/Mac AI Switchboard.app`, and running installed smoke tests after app-behavior slices.
- Add signed/notarized public DMG install evidence, updater artifact evidence, and uninstall proof before broad testers.
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

- Complete Mode Inspector and Verify Off mode proof surface.
- Legal/Privacy Settings surface and local-only network certification.
- Connector-native config promotion after safe parse/dry-run/backup/apply/verify/rollback/Off cleanup.
- Repo Memory MCP long-running app supervision.
- Public signed/notarized release readiness and uninstall proof.
- Stronger measured savings counters for inferred add-ons.
- Local-only certification mode.
- Repo Intelligence graph packs.

### Medium Impact

- Savings anomaly alerts when output unexpectedly grows.
- Per-client savings trends for Claude, Codex, and future connectors.
- Codex large-context advisor that recommends compacting, RTK-only, or connector reset.
- Test relationship view in Repo Intelligence.
- Add-on health cards for RTK, MarkItDown, Ponytail, and Headroom engine.
- Import/export of app settings without secrets.
- Broader Rollback Center native restore coverage beyond Codex/OpenCode/Gemini/sidecar rows.

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

1. Legal and Privacy Settings surfaces.
2. Complete Mode Inspector, Verify Off mode, and installed-app restart/relaunch/reboot smoke evidence.
3. Public local-only/network certification tests on top of the shipped remote destination registry and backend guards.
4. Promote the next native connector write path with full parse, dry-run, backup, apply, verify, rollback, Doctor repair, fixture-home restore, and Off cleanup.
5. Add real repo-memory MCP long-running supervision and connector-specific MCP bridge docs.
6. Replace inferred add-on savings events with measured counters where possible.
7. Expand Repo Intelligence graph packs with language-aware parser/index versions, imports, and reverse dependencies.
8. Finish public signed/notarized release readiness, updater evidence, uninstall proof, and broad tester handoff.

This order front-loads user trust and safety before expanding automation. It also creates the observability needed to tell whether later connector and savings features are actually working.
