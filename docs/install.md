# Installing Mac AI Switchboard

Mac AI Switchboard is designed to be installed as a normal macOS DMG app. Signed, notarized public release artifacts are not published yet, so early testers should build from source or use a locally built unsigned DMG.

The app is local-first, not offline-only. Claude, OpenAI, and other model calls still go to the configured provider accounts. Switchboard mode, client config edits, Doctor repairs, add-ons, telemetry defaults, and Repo Intelligence metadata live on your Mac.

## Recommended User Install

Once releases are published:

1. Download `Mac-AI-Switchboard_<version>.dmg` from GitHub Releases.
2. Open the DMG and drag **Mac AI Switchboard** into **Applications**.
3. Launch the app from Applications.
4. On first launch, approve the local runtime install. The app downloads a self-contained Python runtime and managed tools into `~/Library/Application Support/Headroom`.
5. Choose **Full optimization**, **Headroom only**, **RTK only**, or **Off mode**.

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

Use this path for development and testing only. A source run does not replace a signed and notarized DMG for normal users.

## Local Unsigned DMG

For a local Mac-only test build:

```bash
npm install
npm run build:mac:local-install
```

That script builds a local DMG, installs `/Applications/Mac AI Switchboard.app`, ad-hoc signs the installed bundle for local execution, and runs `npm run smoke:installed:local`.

Local unsigned DMGs are build output, not source artifacts. They are ignored by git under `src-tauri/target/` and should not be committed as a substitute for a GitHub Release.

You can manually verify a local DMG:

```bash
hdiutil verify "src-tauri/target/release/bundle/dmg/Mac AI Switchboard_<version>_aarch64.dmg"
shasum -a 256 "src-tauri/target/release/bundle/dmg/Mac AI Switchboard_<version>_aarch64.dmg"
```

Local unsigned evidence is useful for internal testing, but it is not release-gate evidence and does not replace `npm run smoke:installed -- --confirm` for signed and notarized builds.

## Signed DMG Build

Maintainers with Apple Developer credentials can build a release DMG:

```bash
npm install
npm run build:mac:dmg
```

Run `npm run release:ready -- --strict` before sharing any public build. The readiness check runs branding guards, validates the release report schema, reports concrete blockers, and checks signing/notarization environment requirements.

See [macOS release docs](macos-release.md) for required secrets and release workflow details.

## Shareable Build Checklist

Do not share a public DMG until all gates are true:

1. `npm run release:ready -- --strict` reports no environment blockers.
2. The DMG is signed and notarized with Developer ID credentials.
3. Updater signing is configured with `HEADROOM_UPDATER_PUBLIC_KEY` and `HEADROOM_UPDATER_ENDPOINTS`.
4. `npm run smoke:preflight` passes and writes `dist/smoke-preflight-summary.md`.
5. The DMG is installed as `/Applications/Mac AI Switchboard.app`, with `Contents/Info.plist` present inside the app bundle.
6. `docs/beta-smoke-test.md` is run against the installed app.
7. `npm run smoke:installed -- --confirm` records `dist/installed-smoke-summary.md`, including Switchboard modes, degraded-mode Doctor guidance, planned connector automation gates, manual workflow, Repo Intelligence recipes, Savings calculator copyable summary, per-tool agent handoffs, and Codex compression recovery.

## First-Run Footprint

Mac AI Switchboard may write:

- `~/Library/Application Support/Headroom` for managed runtimes, tools, logs, receipts, backups, caches, and Repo Intelligence summaries.
- `~/.claude/settings.json` plus `~/.claude/hooks/` when Claude Code routing or RTK hooks are enabled.
- `~/.codex/config.toml` and shell profile managed blocks when Codex routing is enabled.
- macOS Keychain entries for app/session secrets.
- `~/Library/LaunchAgents/` only if launch at login is enabled.

Managed edits are fenced and reversible. Use **Off** mode or Doctor repair flows to remove routing hooks and return clients to direct provider behavior.

For oversized Codex payload failures, see [Codex Compression Troubleshooting](codex-compression-troubleshooting.md).

If Codex reports `The '' model is not supported when using Codex with a ChatGPT account`, treat it as a Codex model/provider configuration issue rather than a Headroom compression issue. Use Doctor to repair the Codex provider block, then choose a Codex-supported ChatGPT model before retrying.
