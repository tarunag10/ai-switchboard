# Plan Status Ledger

Updated: 2026-07-03

This file is the short status index for the active Mac AI Switchboard plans. It
tracks what has been created, what was updated, and what is still left.

## Created / Built

- Repo Map one-click generation: Graphify-style graph output, Madge,
  dependency-cruiser, Cargo metadata, Tauri invoke scan, artifact links,
  multi-repo history, stale/fresh warnings, tool preflight checks, partial
  success handling, and token-savings artifacts.
- Repo Intelligence integration: Repo Map freshness, graph-input paths,
  selected packs, handoffs, CLI exports, stale-map warnings, and MCP smoke
  evidence are wired into Repo Intelligence and Agent Session flows.
- Repo Memory MCP: read-only bounded repo manifest, context pack, handoff,
  freshness, clear-index, and stale-health evidence paths exist with local
  smoke checks.
- Savings evidence: runtime/session attribution exists for RTK, Repo
  Intelligence, Caveman, Ponytail, and MarkItDown, with estimated and measured
  confidence separated instead of presenting static constants as measured truth.
- Privacy/security baseline: committed local SQLite/database artifacts are
  ignored/guarded, `CLAUDE.md` was scrubbed, local artifact checks exist, and
  generated evidence summaries are excluded from privacy false positives.
- Local-only network proof: app-owned remote destinations are documented,
  provider traffic is separated, local-only validation emits schema-versioned
  JSON/Markdown, and `smoke:local-only:local:check` verifies the summary.
- Connector readiness baseline: connector manifests and readiness evidence
  exist for the supported/gated provider/editor set; Gemini CLI, OpenCode,
  Windsurf, Zed, and Goose Repo Memory MCP paths have promoted or bridged
  evidence where safe.
- Rollback/Doctor evidence: rollback inventory, managed-record boundaries,
  connector cleanup domains, Doctor repair copy, uninstall disclosure, and
  relaunch-survival local evidence checks exist.
- Release evidence scaffolding: release-readiness, deployment, local evidence,
  public proof schema checks, local install smoke, and unsigned/ad-hoc local
  DMG evidence paths exist.

## Updated In This Slice

- `docs/remote-destinations.md` was restored to readable Markdown and now
  includes the exact local-only boundary and guard signals required by the
  network certifier.
- `scripts/local-only-network-validation-summary.mjs` emits
  `schemaVersion: 1`.
- `scripts/check-local-only-network-summary.mjs` verifies the generated
  local-only network evidence summary.
- `package.json`, `scripts/run-local-evidence.mjs`,
  `scripts/release-readiness-report.mjs`, and
  `scripts/check-release-report-schema.mjs` include the new local-only network
  summary check in aggregate evidence paths.
- `scripts/check-local-build-privacy.mjs` ignores generated evidence summary
  artifacts so the scan checks source/build privacy surfaces instead of its own
  proof output.

## Left

- Produce signed and notarized public DMG artifacts, updater feed proof,
  checksums/SBOM, and public installed-app smoke evidence.
- Promote remaining native/provider editor writes only after provider-specific
  backup, apply, verify, rollback, Off-mode cleanup, and relaunch-survival
  evidence exists.
- Finish public uninstall proof for a signed/notarized installed build, not
  just local or unsigned/ad-hoc evidence.
- Deepen language-aware Repo Intelligence parsers beyond the current graph and
  pack foundations, especially richer Rust/Python/Swift symbol and dependency
  edges.
- Expand savings accuracy with more real before/after token measurements and
  alerting for output growth, low savings, or cost growth.
- Complete gateway/add-on roadmap items that remain guided/gated: LiteLLM
  semantic cache lifecycle, Langfuse self-hosted observability, Cloudflare AI
  Gateway, and Kong enterprise documentation/evidence.
- Keep release readiness blocked until signing/notarization/updater/public
  installed-smoke proof is real.

## Validation

- `npm run smoke:local-only:local`
- `npm run smoke:local-only:local:check`
