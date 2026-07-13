# AI Switchboard Rebrand Audit

Status: refreshed 2026-07-13; native UI/config/error copy now uses AI Switchboard while compatibility identifiers remain legacy by design.

Use this file as the rename guardrail before changing public copy, bundle names, runtime paths, or connector wording.

## Product Name Rules

- Public parent product: **AI Switchboard**.
- Short product name: **Switchboard**.
- macOS desktop app: **AI Switchboard for Mac** where platform specificity matters.
- CLI name: `switchboard`.
- Repo/package slug: keep `mac-ai-switchboard` until redirects, CI, release artifacts, and package consumers are proven.
- Legacy names: keep compatibility with **Mac AI Switchboard** paths, receipts, logs, and prior release artifacts.

## Keep As Technical Names

These strings should not be blindly renamed:

- **Headroom** when referring to the local optimization engine, runtime folders, proxy logs, compatibility shim, or upstream-derived engine behavior.
- **RTK**, **Caveman**, **Ponytail**, **MarkItDown**, and **Compact Chinese** as attributed integrated tools/profiles.
- Existing app support, keychain, backup, receipt, and rollback names until migration tests prove aliases.
- Existing bundle/update identifiers until signing, updater, installed-app smoke, and rollback evidence are updated.

## Public Copy Hotspots

Known user-facing references still using the old product name:

- `README.md`: title, intro, privacy copy, screenshot alt text, release-readiness wording.
- `docs/beta-smoke-test.md`: installed bundle path, launch instructions, local-first layer wording.
- `docs/install.md`, `docs/macos-release.md`, and release docs: app name and download/install wording.
- `docs/repo-memory-mcp.md`: agent handoff instructions that should say Switchboard while preserving macOS app steps.
- `docs/architecture.md`: title and overview can move to AI Switchboard while retaining legacy path notes.

## Desktop UI Hotspots

Known app-visible references:

- `index.html`: boot title and launch heading still say Headroom. Public shell should become Switchboard; runtime events may remain `headroom:*`.
- `src/App.tsx`: update notices, engine issue emails, connector explanations, and release copy still use Mac AI Switchboard in several places.
- `src/components/TermsGate.tsx` and related tests: legal titles still use Mac AI Switchboard.
- `src/components/SettingsView.tsx`: quit label and connector descriptions use old product wording.
- `src/components/SavingsCalculatorCard.tsx`: user-facing usage copy uses old product wording.
- `src/lib/appUpdate.test.ts`, `src/lib/releaseReadiness.test.ts`, and notification helpers need copy updates with matching tests.

## Runtime Compatibility Hotspots

These areas need caution and tests before renaming:

- `src-tauri/tauri.conf.json`: current product and bundle metadata.
- `package.json`: npm/package slug remains `mac-ai-switchboard` for now.
- `src-tauri/src/client_adapters.rs`: managed block tags such as `mac-ai-switchboard:{client}` and legacy text serialization tests.
- `src-tauri/src/logging.rs`: log path compatibility under `Library/Logs/Headroom`.
- `src-tauri/src/state.rs` and `src-tauri/src/lib.rs`: app support paths, Doctor copy, install/repair labels, and release evidence.
- `scripts/check-branding-assets.mjs`: icon and asset names need staged updates only after asset migration.

## First Safe Slices

1. Update public docs and README copy, with explicit legacy path notes.
2. Update desktop UI copy and matching frontend tests.
3. Add runtime compatibility aliases before changing any app support, keychain, bundle, or updater identifier.
4. Update release evidence and website/download copy after local installed-app smoke remains green.

## Current Native Copy Evidence

- Tray menus, runtime tooltips, startup/recovery errors, pricing notices, watchdog notifications, Doctor guidance, provider-config descriptions, uninstall confirmations, and managed sidecar text use **AI Switchboard** or **AI Switchboard for Mac**.
- Headroom remains named only when the copy identifies the local optimization engine, runtime, proxy, or engine logs; it is not presented as a Switchboard product or service connection.
- Legacy `Mac AI Switchboard` application-support/log paths, bundle IDs, keychain labels, updater identifiers, and serialized compatibility fixtures remain unchanged and are covered by existing cleanup/rollback tests.
- Remaining public proof gates are recorded separately in `docs/ai-switchboard-rebrand-release-evidence.md`; no native copy change claims a signed install or reboot marker that has not been observed.

## Audit Commands

```bash
rg -n "Mac AI Switchboard|Launching Headroom|<title>Headroom" README.md docs src index.html
rg -n "mac-ai-switchboard|Mac AI Switchboard|Headroom" package.json src-tauri scripts
```
