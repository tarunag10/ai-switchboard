# AI Switchboard Master Implementation Plan

Updated: 2026-08-21

This is the authoritative reconciliation of the repository's implementation
plans. Older plans remain useful as design history, but their individual status
labels may be stale. A slice is marked **done** only when the implementation,
tests or deterministic gate, and the documented safety boundary are present in
the current checkout.

## Source plans reconciled

- `docs/mac-ai-switchboard-implementation-plan.md` — trust, identity, CI,
  runtime safety, connectors, release, and recovery foundations.
- `docs/agent-control-center-implementation-plan.md` — Agent Session,
  connectors, savings ledger, Doctor, rollback, and release readiness.
- `docs/product-roadmap-plan.md` — product phases 1–8 and the remaining
  installed-app, metrics, connector, and Repo Intelligence gaps.
- `docs/plan-status-ledger.md` — current shipped-vs-left evidence ledger.
- `docs/implementation-plan-reconciliation.md` — architecture and external
  validation boundaries.
- `docs/world-class-token-savings/IMPLEMENTATION-PLAN.md` — P0–P3 trust,
  savings, coverage, and maintainability program.
- `docs/world-class-token-savings/COMPREHENSIVE-TOKEN-COMPRESSION-IMPLEMENTATION-PLAN.md`
  — C0–C5 compression product program.
- `docs/architecture/phase-3-policy-brain.md`,
  `docs/architecture/phase-4-response-cache.md`,
  `docs/architecture/phase-5-tenant-policy.md`, and
  `docs/architecture/phase-6-release-hardening.md` — policy and release
  contracts.
- `docs/benchmarks/phase-3-endpoint-routing-and-cache-compression.md` and
  `docs/benchmarks/phase-4-semantic-cache-and-lmcache.md` — evidence gates.

## Status vocabulary

- **Done** — shipped and locally verified in this checkout.
- **Prepared / externally blocked** — code, schema, and checker are ready;
  completion requires a real signed installation, provider, infrastructure, or
  reboot that local code must not fabricate.
- **Remaining build** — a scoped implementation slice still needs code and
  tests.
- **Intentionally gated** — supported as a safe contract, but promotion is
  prohibited until its evidence requirements pass.

## Reconciled status

### Completed in the current execution cycle

- Redacted model-routing baseline/candidate aggregation with deterministic
  quality, latency, cost, success, and rework derivation.
- Release-evidence documentation reconciliation plus
  `npm run check:release-documentation-drift`, keeping historical asset notes
  distinct from current installed/reboot proof.
- Connector lifecycle evidence linkage: every declared lifecycle evidence name
  now resolves to an approved Rust `#[test]` in
  `src-tauri/src/client_adapters_tests.rs`.
- Repo Intelligence ambiguity guard: `path-graph-v11` suppresses ambiguous
  duplicate cross-file name edges and object-member false positives while
  preserving same-file and static-import resolution.

### Done — prepared and shipped

- Trust seal: proxy session authentication, Mode Inspector verdicts, local-only
  network checks, and trust-seal aggregation.
- Release truth: app/public-version separation, release schema/checkers, and
  truthful README/release claims.
- Agent Session: budget enforcement, cheapest-valid pack recommendation,
  cacheable-prefix tie-break, task affinity, auto-selection, and compression
  checklist.
- Compression shell: unified dashboard, master activation allowlist, Doctor
  compression playbook, profile presets, provider profiles, content-class
  breakdown, RTK presets, and parallel-session guidance.
- Cache: exact response cache policy, namespace diagnostics, clear/restore
  contract, and semantic-cache-v2 opt-in/fail-closed policy.
- Evidence: deterministic 12-fixture benchmark suite, four-variant compression
  proof, model-routing quality/latency thresholds, provider-billed measurement
  scaffolding, and benchmark regression checks.
- Endpoint and policy foundations: generic OpenAI-compatible endpoints,
  endpoint verification, tenant isolation contracts, content-free telemetry,
  rollback/update/storage migration, and release-hardening checkers.
- Connector safety: lifecycle matrix for 12 managed connectors, sidecar
  promotion gates, Goose/Grok allowlisted endpoint writes, and Cursor native
  writes remaining safely disabled.
- Repo Intelligence: bounded AST/call-graph indexing, ranking, MCP supervision,
  progress, cancellation, retry, and context-pack budget controls.
- Maintainability: god-file registry/budgets, modularization, CLI parity
  boundary, local benchmark export, and progressive disclosure/accessibility
  slices.

### Prepared but externally blocked

- Public installed-app smoke and reboot-level Doctor/Rollback/uninstall proof.
  Required inputs are a current signed/notarized install, public release
  artifacts, installed smoke summary, and a real post-reboot marker.
- Public release proof, updater feed/signature metadata, and strict notarized
  distribution evidence. Missing credentials or unreachable GitHub are blockers;
  local unsigned/ad-hoc output is not a substitute.
- Live LiteLLM semantic-cache, Langfuse, Cloudflare Gateway, and Kong evidence.
  These require user-owned infrastructure and credentials.
- Durable provider-billed counterfactuals for providers that do not expose a
  credible read-only usage API.

### Remaining build work

1. **Fresh quality evidence loop:** the machine-checked evidence contract and
   deterministic redacted baseline/candidate aggregation are shipped in
   `benchmarks/fixtures/model-routing-quality-evidence.json`,
   `src-tauri/src/optimization/model_routing.rs`, and
   `npm run check:model-routing-evidence`. Importing successful-task, rework,
   quality, and latency observations from real approved runs remains pending;
   automatic routing stays observe-only until that evidence exists.
2. **Release evidence operator path:** the documentation/checker drift guard is
   shipped; executing the external checklist still requires signing
   credentials, a current public artifact, and a real reboot.
3. **Repo Intelligence depth:** ambiguity handling and indexer versioning are
   shipped; remaining work is deeper bounded semantic resolution only where it
   remains deterministic. Whole-program type inference and dynamic dispatch
   stay out of scope unless a separate evidence-backed design is approved.
4. **Connector coverage:** lifecycle evidence linkage is now machine-checked;
   continue only with documented schemas and full
   detect/preview/backup/apply/verify/rollback/off/uninstall proof. Cursor
   native provider writes remain gated; Continue, Aider, Qwen Code, and Amazon
   Q remain guided or sidecar paths until their schemas are proven.
5. **Provider-specific metrics:** add a provider adapter only when a stable,
   read-only usage API supports complete before/after attribution.

## Execution order

1. Keep the master plan and status checker synchronized with evidence.
2. Build the fresh quality-evidence capture/reconciliation slice.
3. Run the full local gates and commit/push that slice.
4. Prepare, but do not fabricate, the signed-install/reboot operator handoff.
5. Reassess promotion only after current external evidence is present.

## Definition of done

- Every slice is classified in this document as done, externally blocked,
  intentionally gated, or remaining build.
- Done claims point to code plus a test/gate; blocked claims name the required
  external artifact or authority.
- Automatic routing, semantic replay, native connector writes, and release
  trust never become enabled merely because a fixture or local build passes.
- Each completed implementation slice has its own commit and is pushed to
  `main`.
