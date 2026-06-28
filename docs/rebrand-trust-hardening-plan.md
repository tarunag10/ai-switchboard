# Mac AI Switchboard Rebrand and Trust Hardening Plan

Mac AI Switchboard has started moving away from the upstream Headroom desktop shell, but the product still contains mixed identity, legal, runtime, account, and support surfaces. This plan turns that cleanup into a staged implementation roadmap.

The goal is not to hide Headroom. Headroom is still a real underlying optimization engine and should remain visible where that is technically accurate. The goal is to make the app identity, legal surfaces, user controls, and release evidence belong to Mac AI Switchboard.

## Current State

- Product name and bundle identity are Mac AI Switchboard.
- The visible Terms gate now uses bundled Mac AI Switchboard Terms of Use.
- The old Logoipsum/Headroom SVG has been removed from the app UI.
- A generated Mac AI Switchboard logo is now used in the launcher, tray, and app icon assets.
- Off/RTK-only mode now prevents automatic Headroom intercept/proxy startup on launch and bootstrap.
- Remote destinations are now inventoried in `docs/remote-destinations.md`, and governance/deployment checks require that registry before release.
- In-app support actions now route to this repository's GitHub Issues instead of the inherited upstream support mailbox.
- External link opening now rejects unsupported schemes, embedded credentials, line-break injection, and loopback/private/link-local hosts before launching a browser or mail client.
- Many user-facing strings, release docs, runtime labels, support links, pricing/account flows, keychain labels, and file paths still refer to Headroom or upstream Extra Headroom services.

## Principles

- Use "Mac AI Switchboard" for the desktop app, tray UI, legal copy, docs, release notes, support, and install/uninstall surfaces.
- Use "Headroom engine" only when referring to the underlying runtime, proxy, package, or compatibility layer.
- Make Off mode literal: no listener, no client routing, no MCP server, no LaunchAgent, no hidden restore.
- Keep every integration reversible and auditable.
- Keep local-only mode honest: no remote calls, telemetry, pricing checks, or account sync in that mode.
- Do not rename storage paths or keychain services until a migration plan exists.

## Phase 1: Visible Product Identity Cleanup

Replace visible upstream identity where it describes the app rather than the runtime.

Tasks:

- Replace legacy tray menu labels that name Headroom as the app with Mac AI Switchboard wording.
- Replace update banners that name Headroom when they mean the desktop app update.
- Replace onboarding copy that says users are launching Headroom when they are launching Mac AI Switchboard.
- Replace support/contact copy that points to upstream support addresses unless those services are intentionally still used.
- Keep labels such as "Headroom engine", "Headroom proxy", and `headroom-ai` where they refer to the underlying optimizer.
- Update launcher, settings, release-readiness, smoke-test, and docs copy to make the app/runtime split explicit.

Acceptance checks:

- A user can open the app without seeing upstream Headroom as the app name.
- A user can still understand that Headroom is the optional optimization engine.
- App-identity grep checks across `src`, `src-tauri`, and `docs` have no legacy tray/update/app-name leftovers.
- Existing runtime diagnostics still mention Headroom where technically accurate.

Suggested commit:

- `Rebrand visible app shell copy`

## Phase 2: Legal and Privacy Surfaces

Move from upstream legal assumptions to app-owned local legal surfaces.

Tasks:

- Move Terms copy out of `TermsGate.tsx` into a small source module such as `src/lib/legalText.ts`.
- Add a bundled Privacy Notice covering local file access, local config edits, remote account/pricing calls, update checks, telemetry, and generated diagnostics.
- Add a Settings/legal area showing Terms of Use and Privacy Notice after first launch.
- Decide whether any remote account or pricing feature is still owned by upstream Extra Headroom. If not, disable or replace those flows before release.
- Replace any upstream Terms/Privacy/support links in pricing, sign-in, settings, release docs, and error messages.
- Add a terms-version note explaining why `REQUIRED_TERMS_VERSION` was bumped and when future bumps are required.

Acceptance checks:

- Terms and Privacy are readable without network access.
- No visible legal UI links to `extraheadroom.com/terms` or `extraheadroom.com/privacy`.
- Tests cover the Terms gate, legal panel rendering, and version bump behavior.
- Remote account/pricing copy does not imply Mac AI Switchboard owns a service that it does not own.

Suggested commit:

- `Own legal and privacy surfaces`

## Phase 3: Off Mode Regression Hardening

Off mode must stay off across app launch, bootstrap, repair, relaunch, and reboot.

Tasks:

- Add backend tests for saved `SwitchboardMode::Off` preventing intercept spawn, Python proxy startup, MCP restore, and client setup restore.
- Add a Doctor item for Off verification:
  - no `127.0.0.1:6767` listener
  - no `127.0.0.1:8787` listener
  - no Headroom MCP config in Codex
  - no Headroom LaunchAgent loaded
  - no managed client routing blocks
- Add a one-click "Verify Off mode" action in Doctor.
- Ensure repair actions cannot silently re-enable Headroom while desired mode is Off or RTK-only.
- Add an app restart smoke test that saves Off mode, quits, relaunches, and verifies no proxy.
- Document that already-running shells may retain old environment variables until those shells restart.

Acceptance checks:

- Fresh launch in Off mode leaves no listener on `6767`.
- Bootstrap completion in Off mode does not start Headroom.
- Doctor repair in Off mode does not restore Headroom routing.
- Tests fail if a future launch path reintroduces hidden Headroom startup.

Suggested commit:

- `Add Off mode regression gates`

## Phase 4: Runtime Boundary and Naming

Make product architecture understandable.

Tasks:

- Introduce consistent terminology:
  - Mac AI Switchboard: desktop control app
  - Headroom engine: local optimization runtime/proxy
  - RTK: command-output compression
  - Repo Intelligence: local context-pack builder
- Create a small copy helper or constants file for product names and runtime labels.
- Replace generic "Headroom" in UI states with "Headroom engine" when the runtime is meant.
- Rename "Headroom only" mode if needed to "Engine routing" or keep it but explain it in subcopy.
- Keep Rust module and package names stable unless renaming is low-risk.

Acceptance checks:

- Mode labels are understandable without upstream context.
- The user can distinguish app controls from engine status.
- No source-level rename causes migration, updater, or path breakage.

Suggested commit:

- `Clarify app and engine naming`

## Phase 5: Branding Assets and Release Artifacts

Finish replacing inherited visual identity.

Tasks:

- Rename `src-tauri/icons/headroom.iconset` to a Mac AI Switchboard-owned name if Tauri tooling and scripts allow it safely.
- Check DMG artwork, generated release artifacts, docs screenshots, README images, and app store/public listing assets for old branding.
- Consider creating a vector-friendly source logo derived from the generated PNG for long-term maintainability.
- Add an asset provenance note: generated with ChatGPT image generation, copied into the repo, edited only by resizing/format conversion.
- Add an asset guard script that fails on `logoipsum`, `headroom-logo.svg`, and old upstream logo imports.

Acceptance checks:

- DMG, app icon, launcher, tray, and docs all use the new logo.
- Old Logoipsum asset cannot reappear unnoticed.
- Bundle size remains reasonable after replacing image assets.

Suggested commit:

- `Harden app-owned branding assets`

## Phase 6: Network, Account, and Privacy Audit

Audit every remote call and decide what belongs in this fork.

Status: remote destination registry and release gates are shipped. Runtime
local-only backend rejection for account, billing, and contact commands is
shipped. Final account/pricing ownership decisions are still open.

Tasks:

- Inventory all remote destinations: shipped in `docs/remote-destinations.md`.
  - account API
  - pricing API
  - auth code flows
  - contact/support endpoints
  - update feeds
  - telemetry/Sentry/Clarity
  - external legal/support/mail links
- Add a `LOCAL_ONLY` test mode that rejects unexpected network calls: shipped for account activation, checkout, plan change, reactivation, billing portal, and support/contact command entrypoints.
- Ensure local-only mode disables or hides account/pricing/telemetry flows: backend refusal is shipped for account/billing/contact commands, with frontend hiding already present.
- Decide whether upstream account/pricing should be removed, replaced, or clearly labeled as upstream.
- Replace upstream support mailto and endpoint copy with Mac AI Switchboard-owned support details or remove those actions: shipped for the main in-app support and runtime-upgrade failure actions.
- Audit Keychain service names before renaming; create migration logic if they change.

Acceptance checks:

- A local-only run performs no remote account, pricing, telemetry, or support requests. Account activation, checkout, plan change, reactivation, billing portal, and contact commands now refuse in local-only mode before auth or HTTP setup.
- Every remaining remote URL is documented and intentionally allowed.
- SSRF/url allowlist tests cover link-opening and contact/payment flows. Link-opening coverage is shipped for unsupported schemes, credentialed URLs, newline injection, loopback, private, and link-local hosts.

Suggested commit:

- `Audit remote service boundaries`

## Phase 7: Installer, Storage, and Uninstall Migration

Clean up app-owned footprint without breaking installed users.

Tasks:

- Inventory current storage:
  - `~/Library/Application Support/Headroom`
  - `~/Library/Logs/Headroom`
  - `~/Library/LaunchAgents/com.headroom.*`
  - Keychain services under `com.extraheadroom.*`
  - webview storage paths
- Decide whether to migrate storage to `Mac AI Switchboard` paths or keep compatibility paths with clearer UI wording.
- If renaming storage, write an idempotent migration:
  - copy/move state
  - preserve backups
  - avoid losing user settings
  - clean old LaunchAgents
  - keep rollback possible
- Update uninstall disclosure to match actual paths.
- Add installed-app smoke tests for uninstall cleanup.

Acceptance checks:

- Existing users keep settings after upgrade.
- Off mode and uninstall do not leave stale LaunchAgents or proxy listeners.
- Doctor can detect and clean old upstream leftovers.

Suggested commit:

- `Plan and test storage migration`

## Phase 8: Release Readiness Evidence

Create evidence that the fork is ready for testers.

Tasks:

- Build a fresh local DMG.
- Install it into `/Applications/Mac AI Switchboard.app`.
- Verify first launch:
  - new app icon
  - new logo in launcher
  - Mac AI Switchboard Terms of Use
  - no upstream legal links
- Verify mode behavior:
  - Off stays off after relaunch
  - RTK-only does not start Headroom
  - Full mode explicitly starts the engine and configures clients
- Verify uninstall:
  - app-owned files disclosed
  - stale upstream LaunchAgents detected or removed
  - no listener remains
- Save evidence in `dist/local-installed-smoke-summary.md` or a dedicated release evidence file.

Acceptance checks:

- `npm run release:ready -- --strict` passes or lists only intentional blockers.
- Local installed smoke summary is fresh.
- Manual screenshots or notes confirm no upstream terms/logo regression.

Suggested commit:

- `Record rebrand release evidence`

## Test Additions

Add these tests as the roadmap is implemented:

- Terms gate renders bundled Mac AI Switchboard Terms of Use.
- Terms version bump gates older accepted versions.
- No visible legal UI links to upstream Terms/Privacy.
- Launcher and tray import only app-owned logo assets.
- Off mode launch does not bind intercept.
- Off mode bootstrap does not start Python proxy.
- Doctor detects stale Headroom MCP/LaunchAgent leftovers.
- Local-only mode rejects account/pricing/telemetry calls.
- Branding guard catches `logoipsum` and removed asset imports.

## Risks

- Renaming storage and Keychain services can strand existing user state.
- Removing upstream pricing/account flows before replacement can break paid/trial UI paths.
- Keeping Headroom runtime terminology too hidden can make debugging harder.
- Generated bitmap logo assets can bloat the bundle if used at full generation size.
- Tests that touch shell profiles, LaunchAgents, or user config need isolation to avoid mutating developer machines.

## Suggested Implementation Order

1. Visible app copy rebrand.
2. Legal/Privacy local document extraction.
3. Off mode regression tests.
4. Runtime boundary terminology cleanup.
5. Branding asset guard.
6. Remote service audit.
7. Storage/keychain migration plan.
8. Release evidence.

This order keeps user-facing trust issues first while avoiding the riskiest migration work until the app/runtime boundary is well understood.
