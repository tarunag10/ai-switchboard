# Fable Security and Product Hardening Plan

## Updated Status - 2026-07-03

Created / updated:
- Security artifact guardrails are in place: repo-root SQLite artifacts are ignored/guarded, and local-only proof is separated from public release proof.
- Fable audit plan is committed in-repo as this implementation plan and tracks privacy/security hardening separately from product roadmap work.
- Connector readiness proof now covers gated native-write lifecycle evidence, including automation-disabled safety for `aider`, `amazon_q`, `continue`, `cursor`, and `grok_cli`.
- Repo Map UX reports install/tooling issues for missing `uv`, `npx`, `cargo`, and Graphviz, and handles partial-success output.
- Savings evidence is classified so fixture/local-only savings cannot masquerade as public release proof.
- Local evidence checks exist for Doctor repair, Rollback Center, Repo Memory MCP, connector readiness, and runtime savings attribution.

Still left:
- Produce real signed/notarized DMG, updater feed, and public installed-app smoke evidence.
- Promote provider/editor native-write connectors only after provider-specific apply, verify, rollback, Off cleanup, and relaunch-survival proof.
- Finish frontend-visible runtime/session attribution for Caveman, Ponytail, and MarkItDown if the UI still hides backend-measured add-on events.
- Refactor large files opportunistically when touched: `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`, `src-tauri/src/tool_manager.rs`, and `src/App.tsx`.

## Current Plan Rollup - 2026-07-03

Created:
- Fable audit implementation plan and repo-owned security/privacy hardening checklist.
- Local artifact guardrails for `headroom_memory.db`, SQLite sidecars, runtime DBs, and raw audit files.
- App-owned remote destination disclosure and provider-traffic/change-control markers.
- Public release proof summary scripts: `npm run release:proof` and `npm run release:proof:check`.

Done:
- Scrubbed committed local/security-sensitive artifacts and added repo-local ignore/check coverage.
- Added macOS CI and deployment guard coverage without re-enabling noisy `tarun/local-switchboard` push CI.
- Replaced static-only savings claims with fixture-backed checks for Caveman, Ponytail, and MarkItDown evidence.
- Strengthened Repo Intelligence and Repo Memory MCP evidence: pack annotations, stale-health surface, local release evidence, and budget/graph proof.
- Added rollback relaunch-survival evidence and seven-stage automation-disabled connector readiness.
- Added Repo Map preflight remediation for missing `uv`, `npx`, `cargo`, Graphviz, and related tools.

Left:
- Measure real runtime/session savings for Caveman, Ponytail, and MarkItDown beyond fixture/proxy evidence.
- Finish native connector apply/verify/rollback/Off cleanup promotion for provider/editor writes.
- Complete signed/notarized/updater public-release proof and public installed-app smoke evidence.
- Continue large-file refactors in `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`, `src-tauri/src/tool_manager.rs`, and `src/App.tsx`.

## Current Status - 2026-07-03

Shipped since this plan was created:

- Protected local/security-sensitive artifacts: `headroom_memory.db`, SQLite sidecars, runtime DBs, and the raw Fable audit file are ignored and guarded by local-artifact checks.
- Scrubbed inherited personal machine details from project guidance.
- Added app-owned privacy and remote-destination disclosures, including provider-owned traffic boundaries.
- Added macOS desktop CI validation without noisy branch push triggers.
- Added measured Headroom `/stats` attribution and a fixture-backed measured savings benchmark for Caveman, Ponytail, and MarkItDown source rows.
- Added `npm run savings:benchmark:check` and wired benchmark evidence into `evidence:local` and release-readiness schema checks.
- Strengthened Repo Intelligence / Repo Memory MCP evidence: budgeted pack retrieval, pack listing, graph summary, symbol lookup, dependent lookup, read-only annotations, stale-health surface, and local release evidence.
- Added rollback relaunch-survival proof: the local rollback smoke now writes a probe artifact and re-reads it from a fresh process before recording `relaunchSurvivalEvidence`.
- Added connector readiness release evidence for Aider and Cursor gated native-write dossiers, including seven-stage lifecycle coverage and automation-disabled proof.
- Added Repo Map preflight install-hint UI so missing `uv`, `npx`, `cargo`, Graphviz, and related tooling show remediation instead of a generic generation failure.
- Committed and pushed each slice on `tarun/local-switchboard`.

Still left:

- Implement real runtime/session counters for Caveman, Ponytail, and MarkItDown beyond fixture/proxy benchmark evidence.
- Promote the next connector native-write path only after real detect, dry-run diff, backup, apply, verify, rollback, Doctor repair, fixture-home restore tests, and Off cleanup are implemented.
- Refactor large files, especially `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`, `src-tauri/src/tool_manager.rs`, and `src/App.tsx`.
- Complete signed/notarized/updater public-release proof and installed-app smoke evidence.

Source: local Fable audit file reviewed from outside the repository. The raw audit is intentionally not committed because it may contain local paths, inherited machine details, and security-sensitive notes.

## Goals

1. Keep local memory and audit artifacts out of git.
2. Remove inherited personal-machine guidance from repo docs.
3. Replace static savings claims with measured before/after token attribution wherever proxy/runtime evidence exists.
4. Add macOS-specific CI coverage for the app surfaces that Linux cannot exercise.
5. Improve privacy disclosures for app-owned network surfaces and unofficial upstream usage endpoints.
6. Turn Repo Intelligence into a stronger MCP-style context service with budget-aware packs and graph queries.
7. Continue connector readiness through one connector at a time: detect, backup, dry-run, apply, verify, rollback, and Off cleanup.

## Workstreams

### P0: Sensitive Local Artifacts

- Keep `headroom_memory.db`, SQLite sidecars, runtime logs, and local audit files ignored.
- Verify no DB files are tracked with `git ls-files '*headroom_memory.db*' '*.db' '*.sqlite' '*.sqlite3'`.
- Keep `CLAUDE.md` sanitized and repo-local.

### P0: Measured Savings

- Use `record_measured_savings_attribution` for real before/after token counts.
- Keep Caveman, Ponytail, and MarkItDown template values as fallback estimates only.
- Add proxy/runtime call sites that submit measured events when both baseline and optimized token counts are known.
- Add tests proving measured events appear as `confidence: measured` and estimated fallbacks remain clearly labeled.

### P1: macOS CI

- Add a non-secret `macos-latest` job for `npm run build`, focused Rust checks, and desktop-safe tests.
- Keep signing/notarization out of normal CI unless secrets are present.
- Avoid noisy branch push triggers for local working branches.

### P1: Privacy Disclosure

- Update privacy docs with local-only boundaries, app-owned remote-service calls, telemetry toggles, and known unofficial upstream usage endpoints.
- Make disclosures match actual code paths and env gates.

### P1: Repo Intelligence MCP

- Expose pack listing, budgeted pack retrieval, graph queries, and stale-index health through MCP-compatible tooling.
- Keep packs read-only, bounded, secret-excluding, and versioned.
- Track pack quality with fixture repos and token/recall evidence.

### P1: Connector Native-Write Readiness

- Promote connectors only after the full lifecycle is proven: detect, backup, dry-run diff, apply, verify, rollback, Off cleanup.
- Keep sidecar-only and MCP-only connectors visibly separate from native provider config writes.
- Prioritize Cursor or Aider as the next small connector slice after current Qwen/Goose status cleanup.

### P2: Maintainability

- Continue shrinking high-risk giant files by extracting cohesive modules from `lib.rs`, `state.rs`, `tool_manager.rs`, and `App.tsx`.
- Preserve tests and release gates while extracting.

## Subagent Assignments

- Security scout: verify ignored artifacts, privacy docs, and tracked secret-like files.
- Savings worker: wire measured token attribution call sites and tests.
- macOS CI worker: add a safe non-secret macOS workflow job.
- Repo Intelligence worker: extend MCP-style commands and fixture coverage.
- Connector worker: implement the next connector lifecycle slice with dry-run, backup, apply, verify, rollback, and Off cleanup.

## Slice Rules

- Every slice must be validated, committed, and pushed before moving on.
- Do not commit the raw local audit file.
- Do not commit local databases, logs, generated runtime state, or private machine paths.
