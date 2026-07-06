# AI Switchboard Rebrand Release Evidence

Updated: 2026-07-06

Scope: slice 8 from `docs/ai-switchboard-platform-rebrand-execution-tracks.md`.

## Track Reflection

- Slice 1 planning/audit: `b71c9c17`, `2fd696e7`.
- Slice 2 public docs: `57fd78a1`.
- Slice 3 app copy: `39191f02`.
- Slice 4 compatibility/runtime safety: `0597a72c`.
- Slice 5 CLI/platform docs: `03a90a60`.
- Slice 6 support labels: `bff1a99c`.
- Slice 7 website/download flow: `34b01f25`.
- Slice 8 evidence: this document plus `docs/plan-status-ledger.md`.

## Name Gate

Current product surfaces should use AI Switchboard, Switchboard, or AI Switchboard for Mac. Allowed legacy names are compatibility references for current bundle paths, storage paths, release asset names, historical plans, or tests that assert compatibility.

Review command:

```bash
rg -n "Mac AI Switchboard|Mac-AI-Switchboard|mac-ai-switchboard|Headroom|RTK|Caveman|Ponytail|MarkItDown" README.md docs src src-tauri package.json scripts
```

## Release Evidence Commands

Run before public handoff:

```bash
npm run check:branding
npm run release:report && npm run release:report:check
npm run evidence:local
npm run build
git diff --check
```

Release blockers that still need public installed evidence remain outside this docs slice: public installed-app smoke, reboot-level public Doctor/Rollback/uninstall proof, and final public release gate evidence.

## Public Release Truth - 2026-07-06

Verified live release:

- GitHub release: `v0.0.0` (`AI Switchboard for Mac 0.0.0`)
- Public DMG asset: `Mac-AI-Switchboard_0.0.0-signed-notarized-aarch64.dmg`
- Public checksum asset: `Mac-AI-Switchboard_0.0.0-signed-notarized-aarch64.dmg.sha256`
- Asset digest: `sha256:705b90e69deffbadf11e5f9566e40b393debec440d37b89636b621306e549eab`
- Updater feed asset: `latest.json`
- Updater bundle asset: `AI.Switchboard.for.Mac.app.tar.gz`
- Updater signature asset: `AI.Switchboard.for.Mac.app.tar.gz.sig`
- Updater endpoint: `https://github.com/tarunag10/ai-switchboard/releases/latest/download/latest.json`
- Local verification: downloaded the GitHub release asset, verified the checksum, ran `hdiutil verify`, installed `/Applications/AI Switchboard for Mac.app`, and verified Gatekeeper/Developer ID notarization plus `xcrun stapler validate`.
- Static smoke evidence: `npm run smoke:preflight` generated a fresh local `dist/smoke-preflight-summary.md`.

Remaining public-release blockers:

- Public installed-app smoke: the beta checklist still needs explicit tester confirmation before `npm run smoke:installed -- --confirm` can be treated as public release evidence.
- Reboot-level public Doctor/Rollback/uninstall proof: local non-destructive uninstall proof passes, but reboot-level destructive/uninstall survival remains intentionally separate.
