# AI Switchboard Master Implementation Plan

Updated: 2026-08-25

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
- `docs/integrations/oss-harness-integration-plan.md` — DeepSeek Harness,
  NVIDIA NeMo Switchyard, and jcode integration boundaries.
- `docs/architecture/router-workbench-implementation-plan.md` — shared Router
  and Workbench product boundary, OSS reuse policy, kernel contracts, and
  phased execution gates.

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
- Repo Intelligence ambiguity guard: `path-graph-v13` suppresses ambiguous
  duplicate cross-file name edges and object-member false positives while
  preserving same-file and static-import resolution.
- Repo Intelligence now preserves a direct same-file call when the same file
  also contains a member call with that name; the object-member cross-file
  guard remains in place.
- Repo Intelligence now resolves bounded three-hop local TypeScript/JavaScript
  named and wildcard re-exports, while dynamic exports and deeper chains remain
  unresolved; indexer version is `path-graph-v13`.
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
- Repo Intelligence now exposes a bounded, read-only relationship explorer for
  test/source links, imports, and reverse-dependency hubs, with search,
  relationship-type filters, and explicit no-index/no-match states; the UI
  caps the rendered view at 40 rows and never exposes file contents.
- Cursor now exposes a usable Switchboard-owned routing-intent sidecar from
  Settings with reversible apply, verification, rollback, and Off cleanup;
  Cursor native provider/editor writes remain separately gated by the missing
  upstream schema and profile-aware lifecycle proof.
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
- Model-routing completion-handle cleanup now uses the same monotonic expiry
  clock as issuance and validation, so wall-clock adjustments cannot retain
  unusable handles until the bounded pending-handle cap is reached.
- Provider usage parsing now captures OpenAI Responses nested cached-input
  metrics from `usage.input_tokens_details.cached_tokens`, while retaining
  top-level compatibility and content-free cache attribution.
- Backend response-body and streaming reads now share a bounded idle timeout;
  stalled local providers produce truthful timeout outcomes instead of holding
  proxy tasks indefinitely.
- Repo Intelligence native fallback call traversal now includes Swift with the
  same ambiguity and receiver-qualified suppression already used by CLI and
  frontend surfaces; type inference and dynamic dispatch remain out of scope.
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
- Routing evidence export now compares parsed RFC3339 instants rather than
  timestamp strings, so timezone-offset observations report the true latest
  completion while retaining the observe-only boundary.
- Public proof and model-routing checkers now fail closed with concise
  diagnostics for parseable malformed arrays and corrupt JSON fixtures instead
  of continuing into type errors or exposing parser stack traces.
- Continue and Aider dry-run previews now redact YAML fields whose keys imply
  keys, tokens, secrets, passwords, credentials, or auth; raw state remains
  internal for confirmation hashing and apply preservation.
- Model-routing disabled-client matching and policy uniqueness now trim
  surrounding whitespace, closing a kill-switch bypass and whitespace-variant
  duplicate gap.
- Local Repo Intelligence summary validation now reports corrupt generated JSON
  with a concise operator error, and its local check command runs the
  corruption regression harness before accepting a report.
- Local release artifact selection now requires an allowlisted Switchboard
  filename, a regular file, and an exact semantic version token; ambiguous
  duplicate exact-version candidates fail closed before freshness/tie-break
  selection.
- The local DMG install harness now uses that same selector instead of taking
  the first matching or fallback DMG, and its CLI has deterministic ambiguity,
  regular-file, and exact-version regression coverage.
- Native managed-config previews now redact secret-keyed values in JSON and
  TOML surfaces as well as YAML, while raw state remains internal for
  confirmation hashing and apply preservation.
- Operational install, smoke, release, and deployment guidance now derives
  from the canonical `/Applications/AI Switchboard.app` identity; legacy names
  remain confined to historical evidence or compatibility artifact notes.
- Confirmed installed-app smoke now records an explicit public artifact
  SHA-256, and release readiness requires it to match the selected DMG before
  installed evidence can be considered ready.
- Routing evidence persistence now rejects reuse of one run ID across task
  classes or baseline/candidate model pairs, preventing cross-experiment
  aggregation while keeping local evidence observe-only.
- Installed-smoke artifact input now fails closed before writing evidence when
  the optional public artifact path is relative, missing, non-DMG, or not a
  regular file; the contract has a standalone self-test command.
- A canonical content-free route-plan contract now composes endpoint eligibility
  with model-routing identity and promotion stage without changing live proxy
  traffic; no fallback, retry, body translation, or automatic execution is
  enabled.
- A bounded, exactly-once, content-free transport observation recorder now
  exists as a separate contract; truthful terminal outcomes are wired through
  direct, cache, Headroom, ingress, and local-rejection proxy paths. The
  backend request-body splice now has a bounded idle-read timeout and preserves
  timeout/disconnect outcomes. Transport observations remain separate from
  model-routing evidence.

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
- OSS harness governance: DeepSeek Harness maturity audit, Switchyard optional
  interoperability evaluation, jcode reference evaluation, and a machine-
  checked provenance/promotion boundary are shipped; runtime integration remains
  staged and fail-closed.
- Router/Workbench architecture: one shared-kernel plan now separates the
  headless Router from the all-in-one Workbench. Existing routing, adapters,
  replay, registry, and promotion contracts are reused; the durable native
  session/run kernel, visible plan-only Workbench, and native Router
  decision-receipt picker are shipped. The Workbench also selects only
  receipt-backed, source-path-free redacted replays through the existing native
  validator. Native Router and Workbench presets now load only observe-only
  drafts or receipt-backed plan templates; every execution backend remains
  separately gated build.

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
- CLI and frontend Repo Intelligence now resolve bounded three-hop named and
  wildcard local re-exports for static imported calls; dynamic, namespace,
  default, cyclic, and deeper-than-three-hop inference remain intentionally
  unresolved.
- Connector lifecycle evidence now requires adjacent Rust
  `lifecycle-intent` markers for every fixture-linked stage; combined tests may
  declare multiple stages, but unknown or undeclared stage intent fails the
  harness.
- Public release proof now validates uploaded checksum state and fetched
  checksum content against the signed DMG digest; a blocked proof with no
  external release snapshot remains a valid blocked artifact instead of being
  rejected as malformed.
- Repo Intelligence CLI/frontend/native parity now has fixtures proving bounded
  three-hop named/wildcard resolution while dynamic, unresolved, cyclic, and
  deeper-than-three-hop re-exports produce no false call edges.
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

### Cross-platform harness and UI phase — started 2026-08-24

Status: in progress. The first UI/CLI slice and the first Workbench core
extraction slices are implemented and locally verifiable; the remaining core
contracts, native Linux/Windows adapters, and live execution gates remain.

Done in this slice:

- The public `switchboard` command now exposes `harness status`,
  `harness session`, `router`, and `optimize` provider-neutral local planning paths on
  macOS, Linux, and Windows Node environments.
- The CLI contract explicitly reports that live provider traffic and process
  start remain disabled in the cross-platform preview until the shared core and
  runtime adapters are extracted.
- The Session Ready UI now exposes a visible Harness / CLI entry point into the
  Workbench, and its step cards use the Switchboard theme, responsive sizing,
  wrapping labels, and overflow-safe detail text.
- The cross-screen UI hardening slice now removes conflicting legacy sidebar
  widths, gives the shell one geometry owner, disables OS light-control leakage,
  removes forced 360px narrow-content overflow, defines the missing semantic
  hover/strong tokens, and stacks connector readiness/actions on narrow windows.

Remaining in this phase:

1. Extract `switchboard-core` from `src-tauri/src` for route plans, Workbench
   sessions, grants, receipts, capability projection, and OSS contracts.
2. Introduce `switchboard-runtime` traits for filesystem, process supervision,
   clock/cancellation, provider transport, and secrets/keychain boundaries.
3. Make the desktop Tauri crate a thin adapter over the shared core and replace
   the Node preview router/session aliases with the real core-backed CLI.
4. Add macOS, Linux, and Windows runtime adapters plus CI compile/smoke jobs;
   package desktop installers only after those adapters have passing evidence.
5. Add UI integration coverage for Harness / CLI visibility, Workbench routing,
   live runtime status, narrow windows, long labels, and browser-preview guards.
6. Consolidate the Workbench information architecture: rename the sidebar/page
   to Harness / CLI Workbench, embed replay validation beside router decisions,
   add Settings and capability-detail actions, and refresh Workbench references
   after validation without enabling unauthorised process execution.

Core extraction progress:

- `crates/switchboard-core` now exists as a standalone, dependency-light path
  crate. It owns versioned provider-neutral execution, planning, and harness
  status contracts with no Tauri, filesystem, process, network, or OS imports.
- The existing Tauri route-plan boundary consumes the shared core's strategy and
  execution-mode types. The existing `src-tauri/Cargo.lock` remains the release
  lockfile while the new crate is incrementally extracted.
- The full Rust workspace split is intentionally not claimed yet: Workbench
  persistence, capability grants, OSS registry projection, and runtime traits
  still depend on the Tauri application crate and are next.
- The Workbench event vocabulary (`WorkbenchEventKind`, session status, and
  session actions) now lives in `switchboard-core`; Tauri retains only event
  persistence and platform/application wiring.
- The provider-neutral Workbench event/session model and fail-closed lifecycle
  validation now live in `crates/switchboard-core/src/workbench.rs`. The Tauri
  `events.rs` and `session.rs` modules are compatibility facades, preserving
  existing command and storage imports. This slice was verified with 10 core
  tests, 211 Workbench-focused Tauri tests, and 2 route-plan tests; it does not
  yet extract grants, receipts, capability projection, persistence, or runtime
  clock injection.
- `crates/switchboard-runtime` now defines the platform-neutral runtime adapter
  boundary with clock and capability contracts plus a fail-closed portable
  implementation. It does not start processes, touch files, access secrets, or
  send provider traffic. It also exposes a deterministic, non-negative
  `FixedClock` for contract tests and future injected Workbench timestamps;
  its additive `try_unix_millis` path preserves the existing infallible clock
  API while allowing production mutations to fail closed on clock errors.
  Production Workbench creation still uses its existing compatibility path
  until full clock/identity injection is extracted.
- `crates/switchboard-cli` now provides a native, read-only `switchboard`
  binary for `harness status` and bounded Workbench session serialization,
  backed by the shared core/runtime contracts. Its source-purity and binary
  contract tests reject filesystem, process, network, provider, Tauri, and
  content-bearing surfaces. A cross-platform workflow draft is prepared
  locally to run locked format, test, and Clippy checks for core, runtime, and
  CLI on Ubuntu, macOS, and Windows; publishing that workflow is blocked by the
  current GitHub OAuth credential lacking the `workflow` scope. This does not
  yet replace the Node preview command or enable live routing/process
  execution.
- The native CLI boundary now classifies malformed JSON, unsupported fields,
  unsupported enum values, and validation failures into stable content-free
  errors; adversarial tests cover unknown keys, enum values, malformed input,
  and control characters without echoing user data. The three standalone
  crates are pinned to the verified Rust 1.96 toolchain through
  `rust-toolchain.toml` and `rust-version` metadata.
- The native CLI harness-status writer now accepts an injected
  `RuntimeAdapter`; `run_cli` still supplies `PortableRuntime`, while fake
  runtime tests prove provider transport and process-start capabilities are
  rejected fail-closed. The complete CLI package suite passes (2 unit, 2
  binary, 10 contract, and 2 source-purity tests) with Clippy and formatting.
- `switchboard-core::plan_head` now owns the first provider-neutral plan-head
  receipt contract: strict serde shape, deterministic ledger-scoped identity,
  tamper-evident receipt digest, and unknown-field rejection. The Tauri plan
  store still owns snapshot decoding, filesystem persistence, locking, and
  ledger publication; full receipt/ledger adapter integration remains next.
- Tauri plan-head identity derivation now delegates to the shared core helper;
  the existing Tauri record/ledger digests and storage behavior remain the
  compatibility boundary. The focused plan-head storage suite (10 tests) and
  full Workbench suite (211 tests) pass after this integration.
- `switchboard-core::process_grant` now owns the first provider-neutral process
  grant contract: strict persisted schema, fixed TTL/status/revocation rules,
  plan-only flags, deterministic receipt digest, and effective-state checks.
  Grant ID/time generation, authority transactions, filesystem persistence,
  and process admission remain Tauri-owned for the next adapter phase.
- Tauri grant receipt digest generation now delegates through an explicit
  conversion to the shared process-grant contract, while Tauri keeps its
  compatibility validation, effective-state behavior, authority locking, and
  durable ledger unchanged. The grant-focused suite (5 tests) and full
  Workbench suite (211 tests) pass with the updated lockfile.
- `switchboard-core::presets` now owns the provider-neutral Workbench preset
  schema, strict plan-only validation, capability allowlist, and replay
  evidence binding. Tauri retains the native catalog and resolution behavior
  through a compatibility facade; the focused preset tests (2 core and 2
  Tauri) pass without enabling execution.
- `switchboard-core::process_admission` now owns the provider-neutral
  admission receipt schema, deterministic identity, receipt digest, strict
  timestamp validation, and non-executing boundary. Cross-object binding to
  sessions, plans, process specs, and grants remains Tauri-owned. Tauri now
  delegates admission identity and receipt digest generation through an
  explicit conversion adapter while retaining cross-object validation,
  lifecycle, persistence, and locking. The focused process-supervisor suite
  (5 tests) and full Workbench suite (211 tests) pass after this integration.
- The Workbench frontend now consumes a shared adapter-command-readiness policy
  helper for availability, disclosure copy, and checkbox gating instead of
  duplicating the Gemini exception in the component. Focused bridge/component
  tests (21 tests total) and TypeScript typechecking pass; live process start
  and provider traffic remain disabled.
- Tauri process-admission reload validation now delegates the complete
  converted receipt to `switchboard-core`, including deterministic
  `admissionId` binding. A regression test rewrites the ID and refreshes the
  digest, and persistence rejects it; the focused process-supervisor suite
  remains 5/5 after the fix.
- `switchboard-core::process_run_spec` now owns the provider-neutral,
  content-free containment schema, deterministic run identity, and exact
  snapshot digest contract. Tauri remains unchanged for this core-only slice;
  the shared core has 29 passing tests including compatibility golden values,
  identity sensitivity, and tamper/containment checks. The later Tauri adapter
  delegation is still to be completed.
- Tauri `ProcessRunSpec` now delegates neutral validation, deterministic run
  identity, and snapshot digest generation through an explicit conversion to
  the shared core contract, while retaining adapter allowlisting, contract
  version policy, plan assembly, persistence, and lifecycle behavior. Focused
  process-run, run-contract, and Codex-preflight suites pass (29 tests total).
- `switchboard-core::workbench` now exposes additive deterministic lifecycle
  constructors for explicit session IDs and Unix-millisecond timestamps, with
  one timestamp per logical create/transition/fork mutation and validation
  before mutation. Existing constructors remain compatibility wrappers; all
  34 core tests and all-target Clippy pass. Tauri storage has not yet been
  switched to injected runtime clocks; that remains the next adapter phase.
- Tauri `WorkbenchStore` now depends on `switchboard-runtime` and supplies
  `PortableRuntime` through compatibility wrappers, while testable
  `create_with_clock`, `transition_with_clock`, and `fork_with_clock` paths use
  `RuntimeClock::try_unix_millis()` and the core lifecycle seam. Fixed-clock
  and failing-clock tests cover one-sample mutations and no-write-on-failure;
  the storage/run-plan-head suite passes 15/15 and the full Workbench suite
  passes 213/213. Grant/admission clocks remain a separate follow-up.
- Grant issuance and process admission now have additive RuntimeClock-backed
  orchestration paths. The runtime timestamp is sampled once after plan
  preparation, reused for grant/admission binding and expiry, and failures
  occur before verifier or ledger mutation. Focused grant/admission tests pass
  14/14, and the full Workbench suite passes 217/217 after this integration.
  Listing, revoke, and terminal-cleanup clock paths remain intentionally
  separate because they can persist expiry state.
- Workbench now exposes the selected Router decision's existing content-free
  metadata (`taskClass`, `decisionStage`, `routingMode`, and `evidenceDigest`)
  and consumes one combined adapter-readiness policy object for disclosure,
  checkbox gating, and plan-preparation guards. The focused frontend suite
  passes 21/21 and TypeScript typechecking passes; this does not promote Node
  preview routing or enable process/provider execution.
- The public Node CLI now has an explicit `--native` bridge for only
  `harness status` and `workbench session serialize`, using the configured
  `SWITCHBOARD_NATIVE_CLI` path with inherited streams and no guessed-path or
  fallback behavior. Native opt-in is rejected for `router` and `optimize`;
  four bridge tests, the dedicated `check:native-cli-bridge` script, and the
  existing Switchboard CLI checker pass. Startup failures are content-free.
  This is discovery/delegation only, not native router parity or installer
  packaging.
- `switchboard-core::router` now owns a strict, content-free endpoint route
  plan contract: deterministic candidate selection, stable endpoint-ID
  tie-breaking, requested/actual model equality, bounded inputs,
  duplicate/unknown-field rejection, and hard-gate fail-closed behavior. The
  planner is always observe-only with provider traffic and process start
  disabled. Cost and latency values are bounded to JavaScript's safe integer
  range, and unknown rank measurements serialize as `null` rather than an
  overflowing sentinel. The native Rust CLI now exposes `switchboard router endpoint plan`
  with one bounded stdin JSON request, deterministic compact JSON output, and
  content-free malformed/unsafe-input errors. Core verification passes 42/42
  tests; CLI verification passes 21/21 tests and all-target Clippy. The Tauri
  endpoint-routing module now delegates to the same core planner, preserving
  its internal infallible compatibility signature while failing closed on
  invalid legacy input; endpoint and route-plan parity tests pass 11/11. The
  exact Node opt-in bridge now exposes only `router endpoint plan --native`
  alongside the existing harness and Workbench native commands. It forwards
  inherited stdio with `shell: false`, requires the explicit native executable
  environment variable, and keeps legacy router/optimize native shapes
  rejected. Bridge verification passes 5/5 tests plus the existing CLI check.
  Workbench now visibly renders the bounded operational routing status beside
  the selected Router decision, using the existing effective-stage receipt and
  a local fail-safe preview when the native receipt is unavailable. The UI
  slice is strictly observe-only; focused Workbench, Session Ready, and shared
  Workbench tests pass 24/24 with TypeScript clean.
- The cross-platform runtime audit confirmed that Windows artifact URLs alone
  are insufficient: the existing Python, RTK, Headroom, extraction, and path
  layers are Unix-oriented. Runtime distribution now uses explicit target
  selectors with matrix tests and fails closed for every Windows architecture;
  no unsupported Windows runtime is advertised. Six focused selector tests
  pass. Full Windows runtime support remains a separate implementation phase
  requiring archive-kind metadata, Windows executable/venv paths, Headroom
  wheel selection, and platform-safe process code.
- CLI discovery now has an explicit platform boundary: Windows uses PATH and
  PATHEXT-aware resolution only, never Unix login-shell probing, and emits a
  stable fail-closed diagnostic when no candidate is usable. Unix shell and
  candidate ordering remain unchanged. Nineteen focused discovery tests pass;
  the Windows-only PATHEXT integration test remains CI-gated because this
  checkout runs on macOS.
- Managed process execution now has the same explicit boundary: Unix retains
  process-group setup and descendant cleanup, PATH augmentation uses native
  `OsString` path joining, and non-Unix execution fails closed instead of
  silently running without containment. Streaming output overflow is handled
  before timeout cleanup, preserving the capture-limit contract. Six focused
  process-runner tests pass; Windows Job Object containment remains required
  before enabling Windows process execution.
- Unix-only Workbench filesystem, launcher-chain, Mach-O, and related authority
  test modules are now explicitly `cfg(unix)`-gated. The descriptor-relative,
  raw-byte, symlink-safe implementation is unchanged; unsupported platforms do
  not receive a guessed path abstraction or accidental partial runtime. The
  focused Unix regression set remains green: 10 filesystem tests and 29
  authority tests.
- The portable CLI now has a canonical `npm run build:native-cli` source-build
  helper. It resolves the repository root, invokes the locked Cargo manifest
  with `shell: false`, propagates build failures, and has three contract tests
  covering arguments, working directory, failure status, and missing Cargo. It
  does not install, discover, configure, or publish a native artifact. On this
  checkout the pinned rustup toolchain lacks its Cargo component, while the
  explicit installed stable toolchain builds successfully; the helper leaves
  toolchain selection to the caller rather than mutating it silently.
- The tracked `check:switchboard-cli` CI gate now also runs locked, offline
  contract tests for `switchboard-core`, `switchboard-runtime`, and
  `switchboard-cli`. The standalone runtime lockfile was repaired to include
  the current `serde_json` dependency graph, and the combined gate passes with
  `RUSTUP_TOOLCHAIN=stable CARGO_NET_OFFLINE=true`. The repository's prepared
  cross-platform workflow draft remains intentionally untracked because the
  current GitHub credential cannot publish workflow changes without the
  `workflow` OAuth scope; no workflow file is claimed as shipped.
- Executable candidate planning is now a pure, bounded contract in
  `switchboard-runtime`: PATH entries, binary names, platform, and PATHEXT are
  injected; invalid names, unsupported platforms, and expansion limits fail
  closed before allocation. Tauri retains environment reads, filesystem
  checks, and process ownership, while Unix ordering and Windows PATHEXT
  behavior remain compatible. Runtime and focused Tauri discovery tests pass;
  Windows managed runtime execution remains disabled.
- The Node bridge now detects the exact `router endpoint plan` shape before
  legacy Repo Intelligence routing. Missing or misplaced `--native` fails
  closed with a stable exit-2 usage error and no native or repository
  invocation; the exact native command and legacy `router <repo-path>` path
  remain covered by seven focused bridge tests.
- Workbench now exposes a compact Harness/CLI/Routing readiness card with
  separate configured, effective, and automatic routing states; explicit
  Codex, Claude Code, and Gemini CLI readiness; truthful version/process
  boundaries; and a direct path to the existing redacted harness replay.
  Responsive styles preserve opaque IDs and long labels at the supported
  760px minimum, while focused Workbench and TrayApp tests verify the single
  Add-ons replay surface and fail-closed status copy.
- The native `record_model_routing_evidence` command is now registered in the
  Tauri bridge and has a typed TypeScript wrapper with exact invoke-contract
  tests. This exposes validated, observe-only evidence persistence to the app
  without fabricating provider metrics or enabling automatic routing; native
  validation and the TS contract suite both pass.
- JavaScript and native OSS harness replay now consume the shared bounded
  `tests/fixtures/oss-harness/replay-golden.json` contract and assert identical
  redacted output, observe-only flags, counters, p95 latency, and SHA-256
  digest. This closes cross-language replay drift without adding provider
  traffic, process execution, or promotion behavior.
- The local `evidence:local` operator chain now ends at `release-report`,
  matching the app/Tauri local-only allowlist and its explicit no-public-gate
  boundary. Public release proof remains a separate command; script-level
  execution and summary tests verify that neither proof command is invoked by
  the local chain.
- Model-routing evidence capture no longer silently fabricates completion
  observations: success/failure, quality, latency, and applicable cost are
  explicit before completion; direct recording is disabled without a caller-
  supplied observation and forwards supplied payloads unchanged. Focused
  supporting-panel coverage and TypeScript validation pass while routing stays
  observe-only.
- The native CLI bridge now enforces the documented absolute executable
  boundary before spawning: bare and relative `SWITCHBOARD_NATIVE_CLI` values
  fail closed and cannot resolve through PATH, while configured absolute paths
  retain the existing behavior. Nine bridge tests cover the boundary and
  existing command compatibility.
- Native CLI dispatch now uses exact raw argument shapes for the three
  supported native commands. Misplaced, duplicated, leading, or trailing
  `--native`, unsupported commands consuming the flag, trailing harness
  arguments, and Workbench serialization without `--native` fail closed before
  either Node fallback or native execution. The bridge matrix now passes 13/13.
- Windows CLI discovery now preserves the complete bounded PATH/PATHEXT
  candidate order until `first_runnable` validation. Broken files, directories,
  or earlier extensions no longer mask later runnable candidates; metadata-only
  callers retain their compatibility lookup. Runtime executable-search tests,
  focused Tauri discovery tests, and the existing metadata regression pass on
  macOS; real Windows integration remains CI-gated.
- The shared Repo Intelligence golden graph now proves namespace-member calls
  through a named-alias re-export barrel (`api.execute()` -> exported
  `normalize`) and rejects the private sibling (`api.hidden()`) consistently
  across CLI, frontend, and native projections. Resolver scope remains bounded
  static analysis; no type inference or dynamic dispatch was added.
- The standalone Rust contract runner is now cache-neutral: it no longer
  forces `CARGO_NET_OFFLINE=true`, preserves a caller-supplied Cargo policy,
  and still uses exact locked manifests, repository-root execution,
  `shell:false`, and fail-closed diagnostics. The contract self-test passes
  4/4 and the real core/runtime/CLI gate passes with 77 Rust tests.

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
   successful-task cost inputs before producing a store-ready observation; a
   native-issued, bounded, expiring, one-shot completion handle now owns the
   run identity and route decision, while wiring it into a central production
   provider/client completion hook remains pending.
   Native and TypeScript defaults now align with the 100-sample evidence
   contract; transport-only proxy completion still lacks arm, task-class,
   quality, rework, and provider-billed cost context.
   The completion adapter now also rejects invalid run identity, task class,
   timestamp metadata, and out-of-range latency before persistence.
   Native evidence storage canonicalizes RFC3339 instants before duplicate
   detection, so equivalent timezone-offset representations cannot inflate a
   run in the harness.
   The handle store uses monotonic expiry and rejects malformed route identity;
   the frontend exposes issue/complete wrappers without accepting caller
   supplied decisions, run IDs, timestamps, or arms.
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
   native provider writes remain gated; Qwen Code and Amazon Q remain guided
   or sidecar paths until their schemas are proven. Aider and Continue now
   satisfy the promoted allowlisted provider/config contract.
5. **Provider-specific metrics:** add a provider adapter only when a stable,
   read-only usage API supports complete before/after attribution.
6. **OSS harness integration:** the local redacted replay, route-decision
   strategies, bounded session events, provider/tool registry, native
   capability command, and frontend loader are complete and covered by the OSS
   integration gate. Remaining work is optional external interoperability only;
   it stays gated until a concrete upstream workflow has compatibility,
   rollback, attribution, and release evidence.

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
