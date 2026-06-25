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

The script validates signing/notarization environment variables and writes `Mac-AI-Switchboard_<version>.dmg` under `src-tauri/target/release/bundle/dmg/`. See [macOS release docs](macos-release.md) for the required secrets and release workflow.

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

If Codex hits a large-request issue such as `413 Payload Too Large: compression_refused`, use RTK-only mode or compact the conversation, then let Doctor reset the Codex bypass.

If Codex reports `The '' model is not supported when using Codex with a ChatGPT account`, treat that as a Codex model/provider configuration issue rather than a Headroom compression issue. Use Doctor to repair the Codex provider block, then choose a Codex-supported ChatGPT model before retrying.
