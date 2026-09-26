# Portfolio-wide Codex Plan

Status: planning baseline, 2026-09-26

This is the global coordination plan for Tarun Agarwal's app and tooling portfolio. Each repository has a `CODEX_PLAN.md` with its local priority and acceptance gates.

## Portfolio goals

1. Reduce duplicated product work by choosing canonical products in each family.
2. Make release, privacy, accessibility, testing, and evidence status visible and repeatable.
3. Keep local-first and user-data boundaries explicit; do not add cloud/LLM processing without consent and tests.
4. Use Codex for bounded, reviewable slices with a test or inspection gate before commit.

## Global workstreams

### A. Portfolio inventory and truth

- Build a machine-readable catalogue of repository, product family, platform, owner, branch, version, tests, CI, deployment, privacy URL, support URL, and release evidence.
- Mark every fact as verified locally, verified remotely, documented only, stale, or blocked.
- Detect duplicate remotes, renamed products, forks, policy satellites, dirty worktrees, and unpushed branches.
- Add a weekly read-only audit report; no automatic commits, releases, submissions, or public comments.

### B. Shared quality baseline

- Standardise README, license, security contact, privacy/support links, contribution rules, and an explicit data-flow section.
- Add the smallest reliable CI gate for each stack: format/lint, typecheck/build, focused tests, accessibility smoke check where applicable, and secret/dependency scan.
- Use stable fixtures for imports, exports, calculations, OCR, deadlines, and malformed input.
- Record evidence in a release-truth file; local tests do not prove App Review, notarisation, deployment, or live-store status.

### C. Canonical product families

- Document intelligence: PageLumen, Verity, DocBeam, QuickScanToPDF, ExactDraft, LexForm, ClausePilot.
- Legal/civic: Open Access UK, AccessLaw UK, AccessLaw MCP, GDPR Request Handler, Legal Templates, MoveReady UK.
- Finance: Ledger IRR, Salary Sacrifice Calculator, and one selected personal-finance product from CashWise, Spend Light, and PulseBoard.
- Health/wellness: select a lead product among GluPath, SoftGaze, Breathe+, and FitTrack; keep others maintenance-only until evidence supports expansion.
- Local utilities/accessibility: AI Switchboard, Sightline, MacCareStudio, GhostKey, Universal Copy, Accessibility Keyboard, and a11ysnap.

### D. Reusable infrastructure

- Share design tokens and accessible interaction patterns through `design-system`.
- Share Apple release checks through `apple-app-store-release-factory-skill` and `apple-app-foundation-TA`.
- Share extension listing/privacy generation through `chrome-store-kit`.
- Share evidence, source citation, deadline, and export contracts across the legal/civic family.
- Keep shared infrastructure additive and versioned; do not force a monorepo migration before duplication is measured.

### E. Release and safety gates

- Before implementation: verify active repo, branch, bundle/package identity, and intended scope.
- Before commit: run focused tests, typecheck/build where cheap, diff check, and inspect generated files.
- Before push: review staged diff and confirm no credentials, `.DS_Store`, caches, build output, or unrelated user work.
- Before distribution: separately verify archive/export, signing, upload, processing, review submission, notarisation/Gatekeeper, and live status.

## Delivery phases

1. Inventory: re-authenticate GitHub, reconcile all remotes, classify 249 public-profile entries into owned products, forks, satellites, and experiments.
2. Foundations: add per-repo plans, standard checks, release truth, and portfolio catalogue.
3. Flagships: finish AI Switchboard, then one document product and one legal/civic product with complete evidence.
4. Consolidation: remove duplicate product work, promote shared contracts, and make maintenance/archive decisions.
5. Distribution: verify store/deployment state, documentation, onboarding, privacy, accessibility, and rollback.

## Explicit non-goals

- No bulk feature expansion across every repository.
- No automatic routing, cloud OCR, remote LLM processing, or sync without measured need, consent, and rollback.
- No claim that local tests or signatures prove public release readiness.
- No deletion or archiving without a reviewed inventory and recoverable history.

## Definition of done

Each completed slice has: exact scope, changed files, focused verification, known external gates, a clean intentional diff, and a pushed commit whose status is reported separately from unverified deployment or store state.
