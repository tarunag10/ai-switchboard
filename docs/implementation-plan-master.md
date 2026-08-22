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
- Repo Intelligence now preserves a direct same-file call when the same file
  also contains a member call with that name; the object-member cross-file
  guard remains in place.
- Repo Intelligence now resolves one-hop local TypeScript/JavaScript named
  and wildcard re-exports, while dynamic exports remain unresolved; indexer
  version is `path-graph-v13`.
- Repo Intelligence CommonJS fallback parsing now extracts only the quoted
  argument of `require(...)`, preventing later unrelated string literals from
  becoming false dependency edges; CommonJS symbol binding remains
  intentionally unresolved.
- Connector inventory refreshes now use a monotonic generation guard so
  out-of-order frontend responses cannot restore stale enabled state or stale
  errors; the same guard covers launcher verification refreshes.
- Add-on attribution writes now suppress exact duplicate estimated evidence
  events while retaining changed evidence and leaving measured Headroom/RTK
  events unaffected, preventing repeated local setup callbacks from inflating
  savings history.
- Connector promotion now requires non-empty native evidence, a null native
  blocked-stage marker, and all lifecycle stages unblocked before returning
  `native_promoted`; contradictory contracts fail closed.
- Repo Intelligence task context-pack selection now never admits a file that
  exceeds the requested token budget, including zero and undersized budgets;
  ranking remains deterministic for positive budgets.
- Local Repo Memory MCP evidence now validates a fresh, timezone-qualified
  `generatedAt`, preventing stale passing bridge summaries from being reused
  indefinitely.
- Release-readiness measured-savings status now requires both positive saved
  tokens and fresh evidence; consistency tests reject stale passing reports.
- Model-routing evidence persistence now rejects successful observations that
  omit successful-task cost instead of silently dropping them.
- Local release readiness now requires complete Markdown/JSON evidence pairs;
  one-sided or missing pairs cannot be fresh or passing.
- Release readiness now selects only the newest unambiguous DMG matching the
  configured app version and records its modification time and SHA-256.
- Release report status is now an explicit `ready`/`blocked` enum consistent
  with the underlying environment, validation, smoke, and DMG gates.
- The model-routing evidence gate now runs the native routing and telemetry
  persistence test modules in addition to the JavaScript evidence checker.
- Native evidence aggregation now rejects empty, identical, or mixed baseline/
  candidate model identities before producing benchmark metrics.
- Repo Intelligence now offers a native folder chooser while preserving manual
  path entry, cancellation, and read-only indexing behavior.
- Repo Intelligence now resolves same-file identifier-form default exports,
  with frontend and CLI parity coverage; dynamic and multi-hop exports remain
  intentionally unresolved.
- Repo Intelligence now resolves same-file aliased named exports without
  broadening dynamic or multi-hop re-export inference.
- CLI Repo Intelligence now matches the frontend for same-file aliased named
  exports, preserving the existing parity boundary.
- Release readiness now has a deterministic blocked no-refresh rehearsal that
  verifies exact action mapping without rewriting the supplied report.
- Managed connector listings now expose read-only config previews for the
  already-tested sidecar/native lifecycle adapters; preview writes remain empty
  and explicit backup/apply/verify/rollback/Off actions remain required.
- Model-routing completion metrics now have a typed content-free Tauri bridge
  through validation and redacted persistence; automatic routing remains
  observe-only and raw proxy completion is not treated as benchmark evidence.
- A deterministic completion harness now exercises one baseline/candidate pair,
  export reconciliation, duplicate rejection, and the permanently observe-only
  local-runtime result without network or provider traffic.
- Repo Intelligence now has a shared bounded JavaScript/TypeScript golden graph
  corpus with normalized CLI, frontend, and native projections; the parity gate
  runs all three surfaces instead of only the CLI.
- Repo Intelligence frontend, CLI, and native traversal now treat case-variant
  dependency/generated directories as ignored, with parity fixtures for
  `Node_modules`, `Vendor`, and `DIST`.
- The shared Repo Intelligence golden graph now requires an exact, sorted,
  duplicate-free normalized call-edge projection across CLI, frontend, and
  native implementations; unexpected or repeated edges fail the parity gate.
- The same golden contract now requires exact symbol projections and matching
  total/indexed/skipped file counts across CLI, frontend metadata, and native
  summaries, closing indexing-drift gaps in the parity harness.
- Release operator action validation now rejects null, array, empty-label,
  and non-string blocker entries before action mapping, preserving the
  no-refresh report without rewriting it.
- Connector dry-run preview tests now require non-empty target, marker,
  backup, rollback, and confirmation fields while retaining the zero-write
  invariant for gated previews.
- Model-routing evidence now carries explicit `costAttribution`: offline and
  local-runtime artifacts are permanently `local_estimate`, while approved
  live runs require `provider_declared` attribution plus a provider identity;
  the eligibility thresholds are unchanged.
- Connector listing tests now apply the dry-run payload safety contract to
  every emitted preview, not only representative managed and gated rows;
  target, marker, backup, state, rollback, confirmation, and zero-write
  invariants all fail closed.
- Public release proof generation and checking now reject corrupt local JSON
  inputs with concise operator diagnostics before writing or accepting proof;
  the corruption harness verifies the generated proof remains untouched.

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

### Completed current cycle: edge-case and harness hardening

- Model-routing promotion now fails closed for zero-sample, impossible success
  counts, zero-success approved runs, inconsistent success-rate/count pairs,
  out-of-range basis-point metrics, and unsigned latency arithmetic overflow;
  the JS and native evidence artifacts expose explicit successful-task counts.
- The model-routing evidence harness now accepts an explicit fixture path and
  rejects non-positive minimum samples, unequal baseline/candidate arms, and
  future or invalid approved-run timestamps. The canonical offline fixture
  remains observe-only.
- Added `npm run check:implementation-plan-master`, which validates that the
  merged plan's evidence paths and external-proof boundaries still exist and
  agrees with release truth, connector gating, and the offline routing fixture.
- Verified with `npm run check:model-routing-evidence`,
  `npm run check:phase3-routing`, and `npm run build`.
- The evidence checker now independently recomputes the Rust promotion
  thresholds for approved-live fixtures; checked-in offline evidence remains
  explicitly ineligible.
- Added `npm run release:ready:selftest` and a non-mutating
  `--no-refresh --report <path>` mode so release action mapping can be tested
  against an existing report without regenerating local evidence artifacts.
- Connector lifecycle fixtures now have a pure schema gate that rejects
  duplicate IDs, unknown stages, and malformed required-stage declarations;
  Cursor's gated null lifecycle remains valid.
- World-class benchmark fixtures now have explicit shape, numeric, identity,
  and allowed-success-proxy validation before quality gates run.
- Connector lifecycle gates now expose the canonical fixture-to-runtime stage
  mapping (`preview` to `dryRunDiff`, `off` to `offCleanup`).
- Deployment-readiness wiring now tracks the extracted release action module,
  and no-refresh readiness reports fail closed on malformed structure or
  missing report arguments.
- Gemini and Qwen connector fixture tests now cover drift detection, repair,
  and byte-stable repeated Off cleanup.
- Release report schema validation now rejects malformed, timezone-free, stale,
  and materially future-dated `generatedAt` evidence; deterministic timestamp
  edge-case tests cover the seven-day freshness window and five-minute clock
  skew allowance.
- Model-routing live evidence now uses the same bounded timestamp contract and
  tests stale/timezone-free runs plus large-cost arithmetic; promotion remains
  observe-only for the checked-in offline fixture.
- Local release-readiness sections now carry structured source `generatedAt`
  lineage; the schema requires fresh timestamps for present or passed local
  evidence while allowing absent blocked evidence to remain explicit.
- Local readiness status now fails closed when JSON and Markdown source
  timestamps are missing, stale, future-dated, or inconsistent; freshness
  details are emitted in the report for operator diagnosis.
- CLI and frontend Repo Intelligence fallback call matching now suppresses
  duplicate cross-file callable names and receiver-qualified calls while
  retaining unique direct-call edges; parity fixtures cover both surfaces.
- Gemini connector verification now has a deterministic negative-path test for
  each managed shell export; the test derives the actual apply targets so a
  future target-surface change cannot silently make drift coverage ineffective.
- Native Repo Intelligence re-export scanning now uses Tree-sitter export
  nodes, handles semicolonless declarations, and continues after unresolved
  external exports; focused native coverage is 41 tests.
- CLI and frontend Repo Intelligence now resolve bounded one-hop named and
  wildcard local re-exports for static imported calls; dynamic, namespace,
  default, and multi-hop inference remain intentionally unresolved.
- Connector lifecycle evidence now requires adjacent Rust
  `lifecycle-intent` markers for every fixture-linked stage; combined tests may
  declare multiple stages, but unknown or undeclared stage intent fails the
  harness.
- Public release proof now validates uploaded checksum state and fetched
  checksum content against the signed DMG digest; a blocked proof with no
  external release snapshot remains a valid blocked artifact instead of being
  rejected as malformed.
- Repo Intelligence CLI/frontend parity now has negative fixtures proving
  dynamic, unresolved, and two-hop re-exports produce no false call edges;
  the bounded one-hop contract remains explicit.
- Public checksum blockers now depend on verified digest content, not merely
  an uploaded asset name; an uploaded-but-mismatched checksum is covered by a
  fail-closed regression.
- CLI and frontend re-export resolution now rejects ambiguous wildcard targets,
  private named targets, and default wildcard names instead of choosing the
  first matching symbol.
- Native Repo Intelligence now applies the same fail-closed re-export contract
  and suppresses legacy name-matching edges for unresolved imported bindings;
  native coverage is 42 focused tests.
- Automatic model-routing task allowlists now trim surrounding whitespace and
  compare case-insensitively, with a regression fixture for persisted policy
  formatting drift.
- Model-routing evidence now requires explicit task-class, baseline-model,
  candidate-model, and source provenance; approved live evidence must identify
  itself separately from the offline fixture class.
- Rollback local evidence now labels its fresh-process persistence probe as
  serialization/process isolation only and keeps installed-app relaunch proof
  explicitly unverified.
- Repo Intelligence namespace imports now resolve exported member calls with
  visibility checks across CLI, native Rust, and frontend parity fixtures;
  private namespace members remain unresolved.
- Connector lifecycle fixtures now fail closed when any canonical stage is
  omitted or reordered; explicit `null` remains the only valid gated-stage
  declaration.
- CLI Repo Intelligence now excludes unknown files from default indexing like
  the native and frontend implementations, with an indexer-version bump and
  classification parity coverage.
- Native Repo Intelligence now excludes singular `secret` and `private_key`
  path segments consistently with CLI/frontend secret-path policy.
- Mode-relaunch summaries now have a strict checker and explicitly classify
  their evidence as config persistence/process state, not app-internal mode
  observation or public release proof.
- Local connector readiness now compares command output against an independent
  authoritative gated-native-write inventory and rejects empty, duplicate,
  missing, or extra connector lists.
- Connector smoke verification now requires a successful process exit plus the
  exact expected model response; mismatched or empty output stays unverified.
- Frontend one-click connector rows now require that exact smoke result before
  request-counter activity can mark them verified, preventing unrelated or
  mismatched model traffic from producing a false green.
- Successful one-click smoke attempts now transition directly to verified from
  the exact native result; request-counter polling cannot upgrade a pending or
  failed one-click attempt.
- Connector smoke async results now carry the proxy-verification session
  identity, so a late result from a prior launcher session or another client
  cannot mutate the current row state.
- Connector lifecycle validation now rejects any support status outside the
  explicit `managed`/`planned` vocabulary before managed counts or lifecycle
  evidence are computed.
- Mode-relaunch validation now requires both intercept and proxy listeners to
  be down for each persistence-only result, and release readiness reuses that
  contract instead of trusting `passed` alone.
- CLI Repo Intelligence now matches native scan safety for sorted traversal,
  dependency-directory exclusions, the 1 MB default-index boundary, and
  secret/size role precedence.
- Native, CLI, and frontend Repo Intelligence now normalize and globally sort
  candidate paths before applying the 2,500-file cap, preventing directory
  traversal or caller input order from changing indexed membership; boundary
  tests cover all three surfaces.
- Installed-app evidence scripts now select only the canonical
  `/Applications/AI Switchboard.app` path, with a shared bundle identity
  contract rejecting legacy-name or wrong-bundle metadata; this preserves the
  external signed/notarized/reboot proof boundary while removing app-selection
  ambiguity.
- Repo Intelligence classification now aligns native, CLI, and frontend
  precedence for case-insensitive secret paths, ignored/generated directories,
  large files, shell source files, and nested documentation; direct and
  parity fixtures cover the matrix.
- Reboot proof marker and summary now validate canonical bundle identifier,
  product name, and version before treating installed-app trust as ready; the
  restricted-runner arm test skips only when boot identity capability is
  unavailable, while the production arm command remains fail-closed.
- Public installed-smoke readiness now requires a fresh, timestamped summary
  in addition to current checklist content and hash, preventing stale tester
  confirmations from being treated as current evidence.
- Static smoke preflight now uses the same presence-and-freshness contract,
  and public installed-smoke evidence is bound to canonical installed-app
  metadata before it can be considered ready.
- Local connector readiness now reads the canonical promoted native-config
  inventory name and rejects an empty or duplicate promoted-ID report, avoiding
  silent drift from a stale extractor regex.
- Local model-routing evidence capture now persists only bounded, redacted,
  run-scoped arm metrics in SQLite and exports the existing checker shape as a
  permanently observe-only `local_runtime_observation`; Tauri and frontend
  command boundaries expose record/export without accepting prompts or
  responses.
- Native harness tests now detect unavailable loopback sockets and unwritable
  app storage in test-only capability guards, preserving real assertions when
  those capabilities exist while preventing restricted runners from reporting
  environment failures as product regressions; the full library suite is
  `1080 passed, 2 ignored` in this checkout.
- Routing evidence ingestion now enforces a bounded 10,000-event run limit and
  derives exported `minimumSamples` from the persisted routing policy rather
  than duplicating a threshold constant.
- Native routing evidence ingestion applies the same explicit timezone,
  seven-day freshness, and five-minute future-skew contract as the JavaScript
  checker, preventing stale local rows from entering a reconciliation run.
- The model-routing experiment card now provides an operator-facing redacted
  capture/export path wired to the native evidence store; it exposes outcome
  metrics only and labels every local export observe-only.
- Evidence capture invalidates a displayed export whenever run/task inputs
  change, preventing stale artifact JSON from being mistaken for the current
  capture session.
- Release report schema validation now enforces cross-field consistency for
  summary presence, freshness, identity, checklist hashes, and readiness
  booleans, with adversarial blocked/contradictory fixtures.
- Shareable DMG readiness now also requires its `ready` flag to equal the
  conjunction of environment, artifact, backend, updater, static-smoke, and
  installed-smoke gates, preventing contradictory public-release status.

### Remaining build work

1. **Fresh quality evidence loop:** the machine-checked evidence contract and
   deterministic redacted baseline/candidate aggregation are shipped in
   `benchmarks/fixtures/model-routing-quality-evidence.json`,
   `src-tauri/src/optimization/model_routing.rs`, and
   `npm run check:model-routing-evidence`, and malformed-evidence fail-closed
   guards are shipped. Importing successful-task, rework,
   quality, and latency observations from a real approved benchmark producer
   remains pending; the local runtime exporter is ready but automatic routing
   stays observe-only until that evidence and approval exists. A content-free
   completed-route adapter now enforces explicit quality, rework, and
   successful-task cost inputs before producing a store-ready observation; the
   typed persistence bridge is shipped, while wiring it into a central
   production completion hook remains pending.
   Native and TypeScript defaults now align with the 100-sample evidence
   contract; transport-only proxy completion still lacks arm, task-class,
   quality, rework, and provider-billed cost context.
   The completion adapter now also rejects invalid run identity, task class,
   timestamp metadata, and out-of-range latency before persistence.
   automatic routing stays observe-only until that evidence exists.
2. **Release evidence operator path:** the documentation/checker drift guard,
   canonical app identity, report freshness contract, local source-lineage
   fields, and the shared public-proof/shareable-DMG gate consistency contract
   are shipped; executing the external checklist still requires signing
   credentials, a current public artifact, and a real reboot.
3. **Repo Intelligence depth:** ambiguity handling, indexer versioning,
   case-insensitive generated-directory exclusion, the shared bounded golden
   graph contract, and bounded named default-import/default-re-export
   resolution are shipped;
   remaining work is deeper bounded semantic resolution only where it remains
   deterministic. Whole-program type inference and dynamic dispatch stay out
   of scope unless a separate evidence-backed design is approved.
4. **Connector coverage:** lifecycle evidence linkage is now machine-checked;
   promotion inventories are also fail-closed for canonical ordering,
   duplication, overlap, and Cursor gating; managed sidecar/native adapters now
   expose truthful read-only previews in the connector listing;
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
