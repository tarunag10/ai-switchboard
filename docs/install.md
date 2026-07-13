# Installing AI Switchboard for Mac

AI Switchboard for Mac is designed to be installed as a normal macOS DMG app. Signed, notarized public release artifacts are not published yet, so early testers should build from source or use a locally built unsigned DMG.

The app is local-first, not offline-only. Claude, OpenAI, and other model calls still go to the configured provider accounts. Switchboard mode, client config edits, Doctor repairs, add-ons, telemetry defaults, and Repo Intelligence metadata live on your Mac.

## Recommended User Install

Once releases are published:

1. Download the current AI Switchboard for Mac DMG from GitHub Releases. During the compatibility window, artifacts may still be named `Mac-AI-Switchboard_<version>.dmg`.
2. Open the DMG and drag **AI Switchboard for Mac** into **Applications**. Current compatibility bundles may still appear as **Mac AI Switchboard** in Finder and `/Applications`.
3. Launch the app from Applications.
4. On first launch, approve the local runtime install. The app downloads a self-contained Python runtime and managed tools into `~/Library/Application Support/Mac AI Switchboard`.
5. Choose **Full optimization**, **Headroom only**, **RTK only**, or **Off mode**.

Public website/download copy should say **Download AI Switchboard for Mac** and link to the current GitHub Release. Keep the compatibility note visible anywhere the raw asset filename is shown: DMGs may still be named `Mac-AI-Switchboard_<version>.dmg` until updater, smoke-test, and install automation have a proven migration path.

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

For a local macOS-only test build:

```bash
npm install
npm run build:mac:local-install
```

That script builds a local DMG, installs the current compatibility bundle at `/Applications/Mac AI Switchboard.app`, ad-hoc signs the installed bundle for local execution, runs `npm run smoke:installed:local`, and opens the installed app. Set `MAC_AI_SWITCHBOARD_SKIP_OPEN=1` when you want the same local evidence without launching the app window.

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

For read-only agent context sharing, see [Repo Memory MCP](repo-memory-mcp.md). It explains the Mode Inspector install action, `npm run check:repo-memory-mcp`, and how supported coding agents should consume bounded Repo Intelligence packs without mutating repos or connector config.

For per-tool support status, see [Connector Support](connectors.md). Claude
Code, Codex, Gemini CLI, OpenCode, Windsurf, Zed AI, Goose's allowlisted
endpoint fields, and Grok/xAI's documented endpoint are managed routing
targets; Goose also has a managed read-only Repo Memory MCP bridge. Aider,
Continue, Qwen Code, and Amazon Q remain managed sidecars with native provider
state manual, while Cursor stays schema-gated until its reversible native
lifecycle is fully proven.

## Shareable Build Checklist

Do not share a public DMG until all gates are true:

1. `npm run release:ready -- --strict` reports no environment blockers.
2. The DMG is signed and notarized with Developer ID credentials.
3. Rust backend validation ready: `npm run fmt:desktop` and `npm run test:desktop` pass locally or in CI for the release commit.
4. Updater signing is configured with `HEADROOM_UPDATER_PUBLIC_KEY` and `HEADROOM_UPDATER_ENDPOINTS`.
5. `npm run smoke:preflight` passes and writes `dist/smoke-preflight-summary.md`.
6. The DMG is installed as the current compatibility bundle at `/Applications/Mac AI Switchboard.app`, with `Contents/Info.plist` present inside the app bundle.
7. `docs/beta-smoke-test.md` is run against the installed app.
8. `npm run smoke:installed -- --confirm` records `dist/installed-smoke-summary.md`, including Switchboard modes, degraded-mode Doctor guidance, managed connector automation gates, manual workflow, config creation plan, Repo Intelligence recipes, Savings calculator copyable summary, per-tool agent handoffs, and Codex compression recovery.
9. Record reboot-level proof without fabricating it: run `npm run smoke:reboot-level:arm`, reboot the Mac, then run `npm run smoke:reboot-level:record`. The record command refuses to create a marker unless the macOS boot session changed and the app installed in `/Applications` passes codesign, Gatekeeper, and notarization-stapler validation. If the release DMG is still available locally, set `MAC_AI_SWITCHBOARD_PUBLIC_ARTIFACT_PATH=/absolute/path/to/release.dmg` for the record command to verify and checksum it. Finally run `npm run smoke:reboot-level:local` and `npm run smoke:reboot-level:local:check`.

## First-Run Footprint

AI Switchboard for Mac may write:

- `~/Library/Application Support/Mac AI Switchboard` for managed runtimes, tools, logs, receipts, backups, caches, and Repo Intelligence summaries.
- `~/Library/Application Support/Headroom` may remain as preserved legacy storage after first-launch migration.
- `~/.claude/settings.json` plus `~/.claude/hooks/` when Claude Code routing or RTK hooks are enabled.
- `~/.codex/config.toml` and shell profile managed blocks when Codex routing is enabled.
- macOS Keychain entries for app/session secrets.
- `~/Library/LaunchAgents/` only if launch at login is enabled.

Managed edits are fenced and reversible. Use **Off** mode or Doctor repair flows to remove routing hooks and return clients to direct provider behavior.

Before uninstalling, use **Settings -> Uninstall -> Copy dry-run** or run
`mac-ai-switchboard --uninstall-dry-run` from a local build to preview the
managed cleanup targets. The report includes legacy Headroom and current
Switchboard bundle IDs, app support storage, LaunchAgents, macOS WebKit/cache
data, managed shell/config blocks, managed backups, and Switchboard-owned
Keychain service metadata. See [Recovery and Uninstall](recovery.md) for the
full cleanup boundary.

Connector support is intentionally explicit: Claude Code, Codex, Gemini CLI, OpenCode, Windsurf, and Zed AI have managed setup/verify/repair coverage for routing, and Goose has managed setup/verify/repair coverage for the read-only Repo Memory MCP bridge. Cursor has guided settings discovery plus dry-run target/marker preview while native/provider writes stay blocked. Aider, Continue, Qwen Code, Amazon Q Developer CLI, and Grok / xAI CLI rely on guided or detected workflows unless [Connector Support](connectors.md) says a lifecycle is managed.

For oversized Codex payload failures such as `413 Payload Too Large`, see [Codex Compression Troubleshooting](codex-compression-troubleshooting.md).

If Codex reports `The '' model is not supported when using Codex with a ChatGPT account`, treat it as a Codex model/provider configuration issue rather than a Headroom compression issue. Use Doctor to repair the Codex provider block, then choose a Codex-supported ChatGPT model before retrying.

## CLI Preview

The `switchboard` CLI is a repo-local preview for cross-platform Repo Intelligence workflows:

```bash
npm run switchboard -- repo-intelligence <repo-path> --manifest
npm run switchboard -- repo-intelligence <repo-path> --agent codex --format markdown
```

The compatibility path remains supported:

```bash
npm run repo:intelligence -- <repo-path> --manifest
```

Linux and Windows support is CLI-preview only. Desktop app packaging, runtime management, repair, uninstall, bundle, and keychain workflows remain macOS-only. See [Platform support](platform-support.md).
