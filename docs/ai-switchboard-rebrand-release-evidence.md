# AI Switchboard Rebrand Release Evidence

Updated: 2026-07-04

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

Release blockers that still need signed/public infrastructure remain outside this docs slice: Developer ID signing, notarization, updater feed proof, checksums/SBOM, public installed-app smoke, public uninstall proof, and public release gate evidence.
