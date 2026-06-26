# Installing Mac AI Switchboard

Mac AI Switchboard is designed to be installed as a normal macOS DMG app. Release artifacts are not published yet, so early users currently build from source.

## Recommended User Install

Once releases are published:

1. Download `Mac-AI-Switchboard_<version>.dmg` from GitHub Releases.
2. Open the DMG and drag **Mac AI Switchboard** into **Applications**.
3. Launch the app from Applications.
4. On first launch, approve the local runtime install. The app downloads a self-contained Python runtime and Headroom tools into `~/Library/Application Support/Headroom`.
5. Use the Switchboard Home view to choose **Full optimization**, **Headroom only**, **RTK only**, or **Off**.

The app is local-first, not offline-only. Claude/OpenAI model calls still go to your provider account. The switchboard, client config edits, Doctor repairs, add-ons, and telemetry defaults live on the Mac.

## Current Source Install

Until a signed DMG is published:

```bash
git clone https://github.com/tarunag10/mac-ai-switchboard.git
cd mac-ai-switchboard
npm install
cat > .env <<'EOF'
HEADROOM_LOCAL_ONLY="1"
VITE_HEADROOM_LOCAL_ONLY="1"
VITE_HEADROOM_REMOTE_TELEMETRY="0"
EOF
npm run tauri dev
```

Use this path for development and testing only. A source run does not replace a signed/notarized DMG for normal users.

## Building a Local DMG

Maintainers with Apple Developer credentials can build a signed DMG:

```bash
npm install
npm run build:mac:dmg
```

Run `npm run release:ready` before sharing a build. It runs `release:report`, validates the report schema, and prints concrete remaining blockers, warnings, and installed-app smoke actions.
The script validates signing/notarization environment variables and writes `Mac-AI-Switchboard_<version>.dmg` under `src-tauri/target/release/bundle/dmg/`. See [macOS release docs](macos-release.md) for the required secrets and release workflow.

## Shareable Build Checklist

Do not share a public DMG until all five gates are true:

1. `npm run release:ready -- --strict` reports no environment blockers and shows Rust backend validation ready.
2. The DMG is signed and notarized with Developer ID, updater signing, `HEADROOM_UPDATER_PUBLIC_KEY`, and `HEADROOM_UPDATER_ENDPOINTS` configured.
3. `npm run smoke:preflight` passes and writes `dist/smoke-preflight-summary.md`.
4. The DMG is installed as `/Applications/Mac AI Switchboard.app`, with `Contents/Info.plist` present inside the app bundle.
5. `docs/beta-smoke-test.md` has been run against the installed app, then `npm run smoke:installed -- --confirm` records `dist/installed-smoke-summary.md`, including Switchboard modes, degraded-mode Doctor guidance, planned connector automation gates, manual workflow, Repo Intelligence recipes, Savings calculator copyable summary, per-tool agent handoffs, and Codex compression recovery.

## First-Run Footprint

Mac AI Switchboard may write:

- `~/Library/Application Support/Headroom` for managed runtime, tools, logs, receipts, backups, and caches.
- `~/.claude/settings.json` plus `~/.claude/hooks/` when Claude Code routing or RTK hooks are enabled.
- `~/.codex/config.toml` and shell profile managed blocks when Codex routing is enabled.
- macOS Keychain entries for app/session secrets.
- `~/Library/LaunchAgents/` only if launch-at-login is enabled.

Every managed config edit is reversible and should be fenced with `headroom:` markers plus timestamped backups.

## Turning It Off

- **Off mode** removes routing hooks and RTK integration so supported clients behave normally.
- **Pause/Quit** tears down interception while keeping runtime files for faster next launch.
- **Uninstall** removes app-managed LaunchAgent, preferences, caches, keychain entries, routing hooks, and managed config blocks.

If Codex hits a large-request issue such as `413 Payload Too Large: compression_refused`, use **RTK only** mode or compact the conversation, then let Doctor reset the Codex bypass. See [Codex Compression Troubleshooting](codex-compression-troubleshooting.md) for the multiple active chats/goals workflow.

## Local Unsigned Test Build

For a local Mac-only test build, run `npm run build:mac:local-install`. It builds an unsigned/ad-hoc DMG without Apple release secrets, copies it to `dist/release-artifacts`, installs `/Applications/Mac AI Switchboard.app`, ad-hoc signs the installed bundle for local execution, and runs `npm run smoke:installed:local`. The local summary checks the installed bundle, version, running process, DMG checksum, `hdiutil verify`, local code-sign status, and Gatekeeper assessment. It is not release-gate evidence and does not replace `npm run smoke:installed -- --confirm` for signed/notarized builds.

If Codex reports `The '' model is not supported when using Codex with a ChatGPT account`, treat it as a Codex model/provider configuration issue rather than a Headroom compression issue. Use Doctor to repair the Codex provider block, then choose a Codex-supported ChatGPT model before retrying.
