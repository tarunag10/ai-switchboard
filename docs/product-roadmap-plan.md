# Mac AI Switchboard Product Roadmap Plan

This plan expands the rebrand and trust-hardening work into a broader product roadmap for Mac AI Switchboard. The goal is to make the app trustworthy enough to install, clear enough to debug, and useful enough to become the local control center for coding-agent optimization on macOS.

Mac AI Switchboard should stay local-first. The app can route Claude, Codex, and future agent traffic to remote model providers when the user chooses those tools, but switchboard state, config edits, Doctor checks, savings attribution, repo context packs, and add-on health should remain inspectable on the user's Mac.

## Current Checkpoint - 2026-06-29

Recent verified build: `/Applications/Mac AI Switchboard.app` was rebuilt and locally reinstalled from `tarun/local-switchboard` at commit `739dd49 Clarify mode inspector one-click actions`. Local installed smoke, codesign verification, and Off/RTK-only local mode relaunch smoke passed for the ad-hoc app. Public release readiness is still blocked by signing, notarization, updater, and public installed-smoke evidence.

Shipped:

- Off and RTK-only modes now gate launch warmup, threaded bootstrap, and legacy synchronous bootstrap so optimization does not silently restart after the user turns it off.
- Doctor already reports Off-mode violations when Headroom, managed client routing, or RTK remain active against a requested Off mode.
- Connector sidecar/readiness coverage exists for Gemini CLI, OpenCode, Grok/xAI CLI, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q, Windsurf, and Zed, while the fully managed manifest set is Claude Code, Codex, Gemini CLI, OpenCode, Windsurf, and Zed AI.
- Gemini has managed shell base-url routing. OpenCode has promoted provider-config routing with preview, exact confirmation, backup, apply, verify, rollback, and Off cleanup coverage.
- Cursor now surfaces detected editor settings files as evidence while native writes remain blocked. Windsurf and Zed have promoted managed editor-settings routing with backup, verification, rollback, and Off cleanup.
- Repo Intelligence Start Agent Session, read-only packs, handoffs, CLI exports, and repo-memory MCP smoke transport are usable.
- Repo Intelligence graph version `path-graph-v8` adds stylesheet and HTML asset dependency edges plus CSS/HTML symbols on top of visible test/source relationships, Markdown heading symbols with parent hierarchy, shell script invocation edges, Python local import-reference edges, and TypeScript/JavaScript/React package-dependency edges back to `package.json`, so context packs can show web entrypoint links, verification links, docs anchors, shell workflow relationships, Python module relationships, and source-to-installed-package relationships in addition to local imports and call references.
- Doctor now treats stale Repo Intelligence indexer versions as index-health issues, so graph analyzer upgrades ask users to clear/re-index before relying on context packs.
- Repo Memory MCP active state is now process-bound but self-healing: after app relaunch, the app automatically re-runs the read-only smoke check for a previously verified app-managed descriptor. For new setup, Mode Inspector's Prepare MCP action installs, starts, and smoke-checks the app-managed server in one click.
- Repo Memory MCP runtime status now exposes the managed stdio service descriptor separately from app-process smoke supervision, including command, descriptor path, read-only flag, and app-managed ownership.
- Repo Memory MCP service evidence now includes descriptor presence, repo-memory script path/presence, and Node command availability, so Mode Inspector can downgrade broken MCP command wiring before agents rely on handoffs.
- Savings ledger rows now distinguish measured, estimated, and inferred events across Headroom, RTK, Repo Intelligence, MarkItDown, Ponytail, Caveman, and Compact Chinese.
- Rollback Center has guarded preview/execution for backend-allowlisted rows, including Codex/OpenCode restore paths, Gemini managed-block cleanup, managed sidecar cleanup rows, dedicated managed-storage cleanup, Repo Intelligence summary cleanup, LaunchAgent/login-item cleanup, app-state cleanup, and Ponytail receipt/plugin cleanup.
- Doctor repair actions that can restore Headroom routing are now blocked while the saved mode is Off or RTK-only; non-Headroom repairs such as RTK, Caveman, Ponytail, Repo Intelligence, and Repo Memory MCP stay available. Doctor also exposes Verify Off as a primary one-click action when Off-mode evidence remains.
- Remote destination registry, support-link routing, external-link SSRF guards, local-only backend refusal for account/billing/contact commands, branding iconset provenance, local DMG build, local installed smoke evidence, and the in-app Run local evidence sequence are shipped.
- Local-only builds now suppress urgent account, billing, checkout, and upgrade notifications at the shared notification boundary, so stale pricing states cannot reopen remote-service prompts.
- Contact and payment URL allowlist coverage now rejects credentialed, newline-injected, loopback/private, and untrusted checkout/billing redirect hosts before opening or posting to remote endpoints.
- Local mode relaunch smoke now backs up and restores `client-setup.json`, launches the installed app in saved Off and RTK-only modes, and verifies the app process returns while intercept and Headroom proxy listeners stay down.
- Mode Inspector now surfaces stale-shell restart guidance when requested and active mode evidence disagree, including old `ANTHROPIC_BASE_URL`, `OPENAI_BASE_URL`, and `PATH` exports.
- Mode Inspector now reports app-managed launch-at-login plist evidence separately from runtime process/proxy status, including legacy `Headroom.plist` leftovers when present.
- Mode Inspector now shows the client-facing proxy listener address, loopback-only auth/detail evidence, and selected internal backend port/fallback evidence instead of hiding listener proof behind a generic engine label.
- Mode Inspector now distinguishes RTK PATH export evidence from RTK shell hook evidence, so command-rewrite setup is visible separately from install/enabled state.
- Mode Inspector now reuses connector verification details to show managed shell-block and Codex provider-block proof when those checks have run.
- Mode Inspector now separates generic Headroom MCP config evidence from Repo Memory MCP lifecycle evidence.
- Mode Inspector now probes launchd load state for the app-managed LaunchAgent and legacy `Headroom.plist` label instead of only checking plist files.
- One-click local evidence now includes local Off/RTK installed-app relaunch smoke after local DMG build/install and local installed smoke.
- One-click local evidence now includes focused Rollback Center validation for frontend inventory/copy plus native undo-all, Gemini cleanup survival, and dedicated cleanup backend cases, with durable local summary artifacts.
- One-click local evidence now includes focused Doctor repair validation for UI/copy behavior and backend Off/RTK Headroom-restoring repair guards, with durable local summary artifacts.
- Repo Memory MCP active sessions are now app-supervised with periodic read-only smoke rechecks while the same app process owns the session.
- Repo Memory MCP relaunch recovery now auto-verifies previously active app-managed read-only descriptors during runtime refresh, so repeat launches do not require a manual Start MCP click unless smoke fails.
- One-click local evidence now records Repo Memory MCP as its own validation row, including read-only stdio tools, bounded pack response, seeded secret exclusion, app-managed descriptor recheck evidence, and connector bridge recipe proof instead of burying MCP proof inside Repo Intelligence.
- The home runtime banner now offers a primary Start runtime action when the Headroom runtime is offline, degraded, or proxy-unreachable, reusing the same restart-and-refresh path as paused recovery.
- Caveman and Compact Chinese savings attribution now records durable estimated events only when managed guidance actually changes client instruction files, including changed-file and backup evidence instead of unconditional inferred template rows.
- Local-only network certification now has a repo-owned gate, `npm run check:local-only-network`, that verifies the remote destination registry, frontend/backend local-only guards, and documented app-owned remote-service surfaces.
- Settings Legal now includes bundled license, notice, trademark, and asset-provenance summaries offline, including the logo provenance and branding guard command, without external legal links.
- Ponytail savings attribution now records durable estimated events only when plugin registration is verified in connected agent hosts, including host evidence instead of unconditional inferred template rows.
- Savings ledger caveats now use source-specific evidence language for estimated Repo Intelligence, MarkItDown, Ponytail, Caveman, and Compact Chinese rows instead of collapsing all estimates into a generic history/model warning.
- The savings calculator now exposes the roadmap week scope in the Optimize UI, backed by saved local daily history over the trailing seven-day window alongside session, repo, today, month, and lifetime scopes.
- The savings ledger now uses RTK daily gain rows as measured today/week/month evidence, so scoped command-output savings are no longer hidden behind lifetime-only RTK totals.
- RTK gain rows and durable attribution events now preserve measured input tokens, output tokens, savings percentage, and processing time when the local RTK binary exposes them, for scoped daily rows, all-time summary rows, and session ledger evidence, so RTK rows explain more than command count and saved-token totals.
- Savings ledger scope definitions are now visible in-app and included in copied summaries, making it explicit that session resets on app restart, repo is a Repo Intelligence context estimate, and today/week/month/lifetime use saved local history.
- Savings ledger copy and the Optimize UI now flag session output-growth anomalies when backend attribution events report negative saved-token deltas, so regressions are visible instead of being hidden behind positive-only totals.
- Release readiness reports now ingest the local Rollback Center and Doctor repair validation summaries as explicit local-only evidence, including the required refresh commands and pass/fail status, while keeping signed/public installed-smoke gates separate.
- The in-app Run local evidence action now finishes by regenerating the release readiness report, so one click produces fresh local validation summaries and a fresh report snapshot without running signing, notarization, updater publication, or the strict public-release gate.
- Local-only network validation now writes durable local-only evidence from the remote destination registry check and local-free build privacy scan, and the release readiness report ingests it alongside other local validation summaries.
- Local uninstall validation now writes durable non-destructive dry-run evidence from frontend disclosure and backend target-inventory checks, and the release readiness report ingests it alongside Doctor and Rollback local evidence.
- Local Repo Intelligence validation now writes durable read-only evidence for pack generation, backend read-only API payloads, and repo-memory MCP smoke access, and the release readiness report ingests it as local-only proof.
- Repo Intelligence now aligns the frontend preview, CLI/MCP script, and Tauri backend on Swift source classification/symbol extraction, Rust module/import-reference edges, and `package.json` script graph edges, so one-click packs cover more macOS app and build/release workflow code without broad file dumps.
- Repo Intelligence frontend and CLI/MCP graphing now include CSS `@import`/`url(...)`, HTML script/style/image links, and CSS/HTML symbols, so one-click packs can follow web asset entrypoints without broad file dumps.
- Doctor now treats corrupt Repo Intelligence saved summaries as one-click Clear index repairs, with backend proof that only Switchboard managed index metadata is removed before re-indexing.
- Add-on health cards now include bounded trend evidence: Headroom uses recent optimized usage or saved local history, RTK uses per-day command-output history, and MarkItDown/Ponytail explicitly show current-only history until durable counters exist.
- Codex context-pressure guidance now uses saved local daily token history as fallback evidence after relaunch, while still labeling session-only Codex events separately from broader local history.
- Savings anomaly alerts now cover output growth, low savings ratio on high-token sessions, and session cost-growth estimates instead of only showing backend output-growth warnings.
- Doctor now surfaces Repo Memory MCP smoke-failed, stale-config, service-unhealthy, and unverified-active supervision states as Prepare MCP repairs, so unsafe MCP handoffs are visible instead of relying on configured-state alone.
- Repo Memory MCP now has connector-specific bridge setup recipes for Claude Code, Goose, Cursor/Windsurf/Zed, Continue/Aider, Gemini/OpenCode/Grok/Qwen/Amazon Q, and deployment checks keep those read-only boundaries documented.
- The Addons page now has live health cards for Headroom engine, RTK, MarkItDown, and Ponytail, with status, evidence, and next-action copy derived from runtime/tool health instead of static checklist text alone.
- The Home Switchboard panel now includes a Codex large-context advisor that grades recent Codex token pressure, explains when to compact, when to switch to RTK-only, and when to reset/repair Codex through Doctor.
- Settings now exports and previews imports for a secret-free settings bundle covering safe app preferences plus advisory connector/add-on state, while keeping connector config writes and add-on installs manual.
- Settings import previews now classify migration actions as safe, manual, or blocked: preferences can apply directly, while connector and add-on entries show the native gates or Addons/Doctor actions required before any config or runtime changes.
- Home and Usage savings surfaces now include per-client trends for Claude Code, Codex, and future connected tools, showing requests, spent tokens, saved tokens, estimated USD, and reduction from current app-session usage events with a saved provider-history fallback after relaunch.
- Session savings breakdown rows now consume durable backend attribution events before template fallback rows, so Caveman, Ponytail, MarkItDown, and other add-on source cards use measured/estimated event evidence when available instead of contradicting the ledger with inferred enabled-only estimates.

Left:

- Add reboot-level and signed installed-app smoke evidence for full Doctor repair and Rollback Center survival; local summary artifacts are now reported, but they remain local-only proof.
- Promote native config mutation connector by connector beyond Gemini/OpenCode only after parse, dry-run diff, exact backup, apply, verify, rollback, Doctor repair, fixture-home restore tests, and Off cleanup are proven.
- Add signed installed-app relaunch survival evidence for Repo Memory MCP's app-supervised stdio service as native config mutation is promoted; local one-click MCP smoke and connector bridge recipe proof are now split into their own report row.
- Replace remaining fallback-only inferred add-on rows with stronger counters where trustworthy evidence exists; session breakdown and ledger rows now prefer durable backend attribution events, Ponytail uses host-registration estimated events, Caveman and Compact Chinese use changed-file estimated events, MarkItDown uses changed hook/nudge artifact evidence after smoke-tested integration, and Repo Intelligence fallback rows are estimated from graph-pack evidence.
- Deepen Repo Intelligence beyond the shipped parser/index health checks, corrupt-index Doctor cleanup, local imports, package-dependency edges, reverse dependencies, graph-input evidence, graph-aware packs, Swift symbols, Rust module edges, local validation artifact, and read-only MCP smoke proof with richer language-specific dependency analyzers and broader Doctor health checks.
- Complete public release readiness with signed/notarized DMG, updater artifacts, public installed-smoke proof, installed-app uninstall proof, and release-panel wiring; local non-destructive uninstall dry-run evidence is now reported.

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

- Runtime storage now lives under `~/Library/Application Support/Mac AI Switchboard`; the app preserves legacy Headroom storage during migration for one-release compatibility while still naming the optimizer engine accurately as Headroom where technically relevant.
- Terms, Privacy Notice, Settings Legal, and terms-version policy are app-owned and readable without network access.
- Upstream account, pricing, telemetry, and update assumptions need a deliberate keep, replace, or remove decision before public release. Support actions now route to this repository's GitHub Issues.
- Generated logo provenance and branding guardrails are partly shipped; keep them enforced as release assets and screenshots change.

### Mode Safety

- Off mode and RTK-only mode now gate launch and bootstrap startup paths, including the legacy synchronous bootstrap command.
- Remaining mode-safety work is reboot evidence, Doctor-repair installed smoke evidence, and deeper LaunchAgent/MCP/listener proof.
- Doctor still needs a complete Mode Inspector that proves what is active instead of only describing desired state.
- Already-running shells can retain old environment variables; Mode Inspector now explains the restart requirement when mode evidence needs attention. Deeper per-process stale-shell detection can be added later if macOS exposes reliable evidence.

### Privacy and Network Boundaries

- Remote destinations are inventoried in `docs/remote-destinations.md` and guarded by governance/deployment checks; keep the registry current as release, update, telemetry, account, and provider surfaces change.
- Local-only backend guards reject account, billing, and contact entrypoints before auth or HTTP setup; broader unexpected-network tests still need completion.
- Account/pricing screens should not imply Mac AI Switchboard owns upstream services unless those services have been replaced or intentionally adopted.
- Keychain service names are already Mac AI Switchboard-owned, runtime storage migrates from legacy Headroom storage on first launch, and cleanup disclosures must stay aligned with the actual files, Keychain entries, LaunchAgents, shell profile blocks, Claude config, and Codex config touched by the app.

### Release and Installation

- Public release readiness requires signed/notarized DMGs, updater keys, installed-app smoke evidence, and uninstall evidence.
- Local ad-hoc build/install and local installed-smoke evidence are shipped and useful, but they do not prove public release readiness.
- Uninstall copy and Doctor cleanup must match the actual filesystem, Keychain, LaunchAgent, shell, Claude, and Codex footprint.

### Observability and Savings Accuracy

- A session savings ledger exists and labels rows as measured, estimated, or inferred.
- Headroom and RTK measured events are the strongest evidence today; Repo Intelligence has estimated best-pack events; MarkItDown now uses changed hook/nudge artifact evidence after the managed conversion path is smoke-tested; Caveman and Compact Chinese use changed-file estimated events when managed guidance is written; Ponytail uses host-registration estimated events when the plugin is verified in connected agent hosts.
- Remaining work is to make add-on counters more exact and to explain current session, repo, today, and all-time scopes without mixing them.

### Connector Expansion Risk

- Gemini CLI, OpenCode, Windsurf, and Zed AI now have promoted managed routing surfaces with backup, verification, rollback, and Off cleanup evidence. Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, and Amazon Q Developer CLI retain sidecar/readiness coverage and guided setup while their native/provider writes stay gated.
- The remaining connector risk is provider-specific native config mutation for the tools that are not yet managed: Cursor, Continue, Goose, Aider, Grok / xAI CLI, Qwen Code, and Amazon Q still need connector-specific safe writes. Gemini CLI, OpenCode, Windsurf, and Zed AI remain reference paths for promoted managed routing.
- Native config automation should stay gated behind detection, manual-safe instructions, dry-run diffs, backups, Doctor verification, Off-mode cleanup, and fixture-home restore tests.

## Roadmap Phases

### Phase 1: Legal, Privacy, and Trust Surfaces

Goal: make the user-facing legal and trust story app-owned, local-readable, and honest about remote dependencies.

Tasks:

- Move Terms copy into `src/lib/legalText.ts` or a similar source module: shipped.
- Add a bundled Privacy Notice covering local file access, local config edits, update checks, telemetry, account/pricing calls, diagnostics, Keychain usage, and generated evidence: shipped.
- Add a Settings Legal section for Terms, Privacy, license, notices, and asset provenance: shipped.
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

Status: partially shipped. Off/RTK-only launch and bootstrap guards are in place, Doctor can flag active routing evidence while Off is requested, Headroom-restoring Doctor repairs are blocked while Off or RTK-only is requested, Mode Inspector exposes Codex/Claude/RTK/MCP/LaunchAgent rows, and local installed relaunch smoke proves saved Off and RTK-only modes do not start the Headroom proxy. The remaining work is reboot evidence, deeper listener/hook proof, and Doctor-repair smoke evidence.

Tasks:

- Add backend checks for listeners on `127.0.0.1:6767`, the selected internal backend port (`6768` or fallback `6769..=6790`), managed shell blocks, Claude hooks, Codex provider blocks, MCP config, and LaunchAgents.
- Add a Doctor "Verify Off mode" action: shipped as a primary Doctor action when Off-mode evidence remains.
- Add a Mode Inspector panel showing requested mode, active mode, Headroom engine status, RTK hook status, Claude routing, Codex routing, Repo Memory MCP lifecycle state, shell export state, and LaunchAgent state.
- Block repair actions from silently restoring Headroom routing when requested mode is Off or RTK-only: shipped for Headroom-restoring Doctor actions.
- Extend launch/bootstrap tests into installed-app reboot and Doctor-repair smoke evidence.
- Document stale shell behavior and restart guidance: shipped in the Mode Inspector attention state.

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

Status: partially shipped. The ledger UI and backend durable events exist with measured, estimated, and inferred confidence labels, the calculator now covers session, repo, today, week, month, and lifetime scopes, RTK gain rows feed measured today/week/month/lifetime ledger rows with input/output/time evidence when available, and scope definitions are visible in-app/copyable so repo and current-session rollups are not conflated. The remaining work is stronger live counters and command-family persistence once RTK exposes command-family data.

Tasks:

- Define savings scopes: current app session, current repo, today, week, month, lifetime: shipped in the calculator and ledger UI.
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

Status: partially shipped. Rollback inventory, guarded previews, selected native restore paths, Gemini managed-block cleanup, and undo-all orchestration for backend-allowlisted ready rows exist. The remaining work is broader native restore coverage for the remaining sidecar connectors and installed-app survival evidence.

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

Status: framework shipped; native writes are intentionally gated per connector. Readiness metadata, dry-run previews, handoff dossiers, sidecar lifecycle evidence for unpromoted connectors, and release evidence checks exist across the connector set. OpenCode has provider-config routing, Gemini has managed shell routing, and Windsurf/Zed have managed editor settings routing with backup, verification, rollback, and Off cleanup coverage.

Tasks:

- Preserve the shipped connector readiness contract with stages: detected, manual guide available, backup implemented, apply implemented, verify implemented, rollback implemented, Off cleanup implemented.
- Keep connector manifests current for Gemini CLI, OpenCode, Cursor, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, Zed AI, and Grok / xAI CLI.
- For each connector, document native config paths, provider/base-url semantics, account caveats, credential boundaries, and rollback strategy.
- Keep UI badges for "manual only", "automation gated", "verified automation", and "unsupported account/model".
- Promote native config automation connector by connector after the shared readiness framework and fixture-home restore tests prove the path.

Acceptance checks:

- Managed and guided connectors show useful detection, sidecar/readiness lifecycle state where applicable, native config gate status, and promoted routing lifecycle evidence without mutating unsupported native config.
- Automation is disabled until backup, verify, rollback, and Off cleanup exist; promoted routes expose repair/rollback only after those gates are proven.
- Doctor can explain why a connector is not safely automatable yet and can repair promoted managed routing drift.
- Adding a new connector does not require bespoke UI rewrites.

Suggested commit:

- `Add connector readiness framework`

### Phase 7: Repo Intelligence v2

Goal: turn Repo Intelligence from bounded packs into a graph-aware local context layer.

Status: v1 is usable. Start Agent Session, CLI handoffs, read-only packs, parser/index health fields, corrupt saved-index Doctor cleanup, modern JS/TS/Python/Rust/Swift symbol extraction, Rust module/import-reference edges, `package.json` script graphing, repo-memory MCP smoke transport, and connector-specific MCP bridge recipes are shipped. The remaining work is deeper graphing and long-running supervised service behavior.

Tasks:

- Add tree-sitter or language-specific parsers for TypeScript, JavaScript, Python, Rust, Swift, Markdown, and shell scripts: shipped for the current tree-sitter-backed symbol extraction contract; deeper language-specific dependency analyzers remain.
- Persist file hashes, parser versions, symbol counts, imports, reverse dependencies, graph-input paths, likely tests, and stale status: shipped for the current graph index contract.
- Add graph-aware packs for implementation, verification, onboarding, risk review, and release handoff: shipped with bounded graph brief and graph-input evidence; deeper task-specific graph ranking remains.
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
- Repo Memory MCP signed installed-app relaunch survival evidence beyond the local one-click MCP smoke row.
- Public signed/notarized release readiness and uninstall proof.
- Stronger measured savings counters for inferred add-ons.
- Broader local-only certification mode beyond the shipped static registry, backend guard, local-free bundle scan, and local summary evidence.
- Repo Intelligence graph packs.

### Medium Impact

- Broader savings anomaly thresholds beyond the shipped session output-growth, low-savings, and cost-growth alerts, especially durable day/week trends once backend attribution can persist richer baselines.
- Broader per-client savings history beyond the shipped saved provider-history fallback, including exact request counts once backend daily/hourly history can persist client counts directly.
- Broader Codex advisor history beyond the shipped saved daily-history fallback, including per-Codex durable history once backend attribution can separate clients across launches.
- Test relationship view in Repo Intelligence.
- Deeper add-on health history beyond the shipped bounded trend cards, especially durable MarkItDown/Ponytail counters.
- Broader settings migration that can apply connector/add-on changes after each native config gate has restore evidence; preview classification is shipped, but connector/add-on application remains manual.
- Broader Rollback Center native restore coverage for provider-specific connector configs beyond the shipped Codex/OpenCode/Windsurf/Zed native rows, Gemini managed cleanup, sidecar connector cleanup, managed-storage, Repo Intelligence summary, LaunchAgent, app-state, and Ponytail receipt/plugin rows.

### Later

- Legacy storage compatibility sunset after a release with migration evidence and user-state preservation proof.
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
3. Broaden public local-only/network certification tests on top of the shipped remote destination registry, backend guards, local-free bundle scan, and local summary evidence.
4. Promote the next native connector write path with full parse, dry-run, backup, apply, verify, rollback, Doctor repair, fixture-home restore, and Off cleanup.
5. Add signed installed-app relaunch survival evidence for Repo Memory MCP as native config mutation is promoted.
6. Replace inferred add-on savings events with measured counters where possible.
7. Expand Repo Intelligence graph packs with language-aware parser/index versions, imports, and reverse dependencies.
8. Finish public signed/notarized release readiness, updater evidence, uninstall proof, and broad tester handoff.

This order front-loads user trust and safety before expanding automation. It also creates the observability needed to tell whether later connector and savings features are actually working.
