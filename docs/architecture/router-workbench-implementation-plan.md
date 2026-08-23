# Switchboard Router and Workbench implementation plan

Updated: 2026-08-24

## Product decision

AI Switchboard has two first-class product surfaces backed by one kernel:

- **Switchboard Router** is the local, headless routing and optimization layer.
  It remains usable from existing coding clients and other local applications
  without starting an agent loop.
- **Switchboard Workbench** is the all-in-one control surface for sessions,
  agent-run plans, capability grants, tools, replay, and eventually managed
  execution. It calls the Router as a capability; it never replaces or runs a
  second routing policy.

```mermaid
flowchart TB
    UI["Switchboard UI and CLI"] --> W["Workbench: sessions and run plans"]
    W --> K["Shared kernel: events, grants, receipts"]
    K --> R["Router: existing policy and route decision authority"]
    K --> A["Agent adapters: existing client lifecycle contracts"]
    R --> P["Providers and local runtimes"]
    A --> C["Codex, Claude Code, and future clients"]
```

The Workbench starts **non-autonomous**. It may create and replay local plans,
but cannot launch a shell, call a provider, modify a workspace, or persist a
prompt/tool result until a later execution backend passes its own consent,
redaction, rollback, and release gates.

## Audit basis and status vocabulary

This status was reconciled against committed `main` through `09bff975` plus the
admission-hardening change recorded in this commit, and against the visible
frontend/native command wiring on 2026-08-24. Unrelated concurrent unstaged
work is not counted as shipped. A check mark therefore means the capability is
in the committed product boundary, not merely described in another plan or
present in an unrelated local diff.

- **Done** — implemented on `main`, reachable through its intended product
  surface, and covered by focused deterministic checks.
- **Prepared / partial** — useful contracts, UI, or receipts exist, but they do
  not yet perform the end-to-end user outcome.
- **Remaining build** — implementation and tests are still required.
- **Intentionally gated** — the safe behavior is to remain unavailable until
  the named evidence or consent gate passes.
- **External gate** — completion depends on a pinned upstream runtime,
  licence/compatibility evidence, provider access, or release environment that
  this repository must not fabricate.

Current verification snapshot:

- `34` focused frontend tests pass across selective activation, Workbench
  bridge/view, model-routing controls, supporting panels, and advanced Settings.
- `46` focused native `workbench_kernel` tests pass.
- The model-routing evidence gate passes: `13` Node contract tests, `35`
  native model-routing tests, and `18` native telemetry-store tests.
- `npm run build` passes after preserving the authoritative
  `RepoPackCompressionMode` union in the preference refresh.
- `npm run check:oss-harness-integrations` passes after its stale deleted
  registry-test reference was replaced with the shared Workbench projection,
  Addons integration, and native registry checks.

## Executive implementation status

| Area | Already done | Prepared or partial | Still left |
|---|---|---|---|
| Router authority | Observe-only route planning, endpoint eligibility, bounded task classes, native completion handles, redacted evidence, decision receipts, presets, and visible Routing UI | `userApproved` and `automaticAllowlisted` can be saved and deterministically evaluated, but the operational receipt truthfully reports effective `observe` | Bind one real request/session lifecycle to outcome evidence; then add per-request approved routing and, only after evidence, automatic allowlisted routing |
| Workbench kernel | Content-free durable sessions/events, lifecycle/fork/export, capability projection, replay/Router receipt resolution, adapter dry-run plans, containment intent, 15-minute grants, and durable Codex admission | Admission reaches only `authorized_not_started`; no binary, task payload, workspace handle, PID, output, or provider request exists | Native supervisor, version probe, process ownership, timeout/cancel, ephemeral task channel, workspace revalidation, execution receipts, recovery, and orchestration |
| Workbench UI | Navigation, session timeline, presets, plan inspection, grant/revoke, admission validation, truthful no-traffic/no-write badges, and hidden-view refresh guard | Execution is deliberately absent and admissions are informational | Add live run status/cancel/recovery only when the native supervisor exists; never add a renderer-owned shell or command field |
| Selective optimization | A production Addons card lets the user choose exactly five of ten tools and activate them in one click; native validation, preflight, single-run locking, per-tool results, receipts, and drift-safe rollback cover the managed actions | A run can end `partial`; the UI remembers only the current component's last run ID even though native state is persisted | Restore native selection/last receipt after restart, expose receipt history, and add safe retry/resume for failed tools without reapplying successful tools |
| Ponytail, Caveman, RTK, MarkItDown | Visible Addons and selective activation paths; exact created/changed artifact fingerprints; narrow restore/removal; external-drift blocking; receipt preservation | Runtime installation or client prerequisites can still fail on a particular machine | Add end-to-end disposable-home matrix tests and an app-visible repair path for partial managed runtimes |
| Chonkify-labelled pack mode | Promotion gate currently passes (`MIT`, wrong-omission gate `0%`); read-only pack preference and selective activation are usable | The current `switchboard-chonkify` adapter is Switchboard-authored head/tail compaction, not verified upstream Chonkify code | Rename it to Switchboard Pack Compaction and remove upstream attribution, or integrate a pinned upstream adapter with parity/provenance tests |
| Leanctx | Loopback-only shadow setup and selective activation/rollback are visible | Requires an already configured executable despite UI copy saying install-and-enable; remains shadow-only | Correct the copy, pin a supported runtime/version if distribution is desired, and pass health/containment/promotion evidence before live routing |
| OSS harness reuse | Internal pinned DeepSeek Harness preview adapter, maturity audit/context prototype, redacted replay, deterministic strategy fixtures, session-event prototype, and shared metadata-only registry | DeepSeek is not in the normal connector UI; Switchyard and JCode are evaluated references; `twaldin/harness` contributes only a contract idea | Expose DeepSeek honestly as Experimental or remove prototype-only production modules; then choose and prove one optional pinned workflow |
| UI visibility | Workbench and Routing are top-level routes; selective activation is in Addons; advanced Headroom settings are in Settings; inactive routes use `hidden` as navigation state rather than product concealment | Some controls correctly remain disabled because their backend is absent or unsafe | Maintain a reachability test for every production component and ensure every mounted hidden route suspends polling/subscriptions |

The largest product gap is therefore not another card or policy schema. It is
the narrow native execution seam between a valid Workbench admission and one
owned, cancellable Codex child process. The largest Router gap is the separate
seam between observe-only decisions and a real request completion lifecycle.
Neither gap should be filled by exposing arbitrary commands, paths, prompts,
or provider credentials to the renderer.

## Current inventory

| Capability | Current authority | Status | Workbench action |
|---|---|---|---|
| Model/provider route policy and evidence thresholds | `src-tauri/src/optimization/model_routing.rs` | Done as a pure planner/gate | Reuse as the Router authority; live model substitution is not implemented. |
| Endpoint/route plan and transport observations | `src-tauri/src/route_plan.rs`, `transport_observations.rs` | Planner and live observations done; lifecycle correlation open | Reuse their content-free evidence; call the route planner from one live shadow path before any substitution. |
| Coding-client detect/plan/consent/apply/verify/rollback | `src-tauri/src/client_adapter_contract.rs` | Done | Wrap as a planning adapter; keep it the only configuration mutation authority. |
| OSS strategy fixtures and redacted route replay | `oss_harness_replay.rs`, `scripts/oss-harness-strategies.mjs` | Done | Promote their schema into the kernel event/replay boundary. |
| Session-event prototype | `scripts/oss-session-events.mjs` | Done, prototype | Port its contiguous lifecycle/fork rules to native persistent storage. |
| Context packs and agent memory | `src/lib/agentSessionPacks.ts`, `src-tauri/src/agent_memory/` | Done, bounded | Reference pack IDs/digests only; do not persist prompts or source content. |
| OSS capability metadata/promotion gate | `oss_capabilities.rs`, `plugin_promotion_gate.rs` | Done, metadata only | Project from Kernel registry/grants while retaining fail-closed promotion. |
| Selective activation receipts | `activation_commands.rs` | Done | Use the same ownership/rollback model for future capability changes. |
| Durable Workbench session/run authority | `src-tauri/src/workbench_kernel/` | Done, plan-only | Persist opaque sessions and prepare non-executable router/adapter plans. |
| Process authorization and admission | `capability_grant.rs`, `process_supervisor.rs` | Prepared, non-executing | Revalidate a plan, issue/revoke a fixed-expiry grant, and record Codex `authorized_not_started`; do not describe this as process supervision. |
| Select-five optimization UI | `SelectiveActivationCard.tsx`, `activationTools.ts` | Done, restart follow-up open | Keep exactly five of the ten explicit tools and show each native result; add native last-receipt recovery. |
| DeepSeek Harness preview adapter | `deepseek_harness.rs`, `dsh_plugin_maturity.rs` | Prepared, internal experimental pin | Expose its normal consent/verify/rollback lifecycle as an explicit Experimental connector or remove production-only prototypes; retain guided-only fallback for unknown versions. |
| Real agent execution backend | — | Remaining build, deliberately gated | Add only after the non-autonomous kernel and UI are verified. |

## OSS reuse and provenance policy

| Upstream | Licence/state | Reuse | Explicit non-reuse |
|---|---|---|---|
| [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) | MIT; developer preview with breaking-change risk | Plugin capability vocabulary, durable session/event distinction, scoped capability concepts | No vendored runtime, plugin binary, scheduler, credential flow, or automatic execution. |
| [NVIDIA NeMo Switchyard](https://github.com/NVIDIA-NeMo/Switchyard) | Apache-2.0; reviewed as optional/interoperability-only | Protocol-translation and typed strategy ideas; benchmark fixtures | No second live router, embedded server, automatic install, or configuration rewrite. |
| JCode provenance conflict: [plan target](https://github.com/Ravi-bit-app/jcode), [evaluation target](https://github.com/1jehuang/jcode) | Unresolved; local documents name different repositories | Attach/resume, multi-session lifecycle, adaptive context and resource profiling ideas only | No source, binary, attribution, or claimed pin until one repository/commit/licence is selected and the other reference is corrected. |
| [twaldin/harness](https://github.com/twaldin/harness) | Evaluate and pin before any dependency use | `RunSpec -> RunResult` adapter boundary and per-CLI isolation | No subprocess execution or instruction-file mutation in the initial kernel. |

Every future OSS addition needs: pinned URL/commit, licence and attribution,
compatibility matrix, capability declaration, privacy/redaction review,
rollback/disable path, deterministic tests, and a visible UI disclosure. The
existing `plugin_promotion_gate.rs` remains the promotion authority.

## Kernel contracts

```text
workbench_kernel/
  session.rs            durable content-free identity, status, lineage, timestamps
  events.rs             bounded versioned lifecycle ledger and deterministic forks
  run_contract.rs       RunSpec, RunPlan, Router/replay references, capabilities
  presets.rs            native-owned observe-only plan drafts
  adapter_readiness.rs  process-free canonical adapter metadata
  process_run_spec.rs   deterministic future-containment intent, never a command
  capability_grant.rs   expiry-bound future-process authorization receipts
  process_supervisor.rs durable historical admissions, not a process supervisor yet
  storage.rs            atomic local session persistence and bounded retention
```

Invariants:

- A session event contains only bounded identifiers, event kind, sequence,
  parent reference, local timestamps, route/adapter/capability IDs, and opaque
  digests. Prompts, model messages, outputs, headers, endpoint URLs,
  credentials, filesystem paths, and tool arguments are forbidden.
- Session events are contiguous, append-only, bounded, and transition through
  `active -> paused -> active|completed|cancelled`; forking is explicit and
  deterministic.
- A `RunSpec` is declarative and non-executable by default. It holds an
  existing workspace/reference digest, adapter ID, context-pack digest, route
  decision reference, capability grants, and receipt IDs—not content.
- `RunPlan` may expose configuration dry-run evidence from the existing
  `CodingClientAdapter` contract but cannot apply it. Existing adapter consent
  and rollback rules remain unchanged.
- Capability requests are default-deny, session scoped, visible in the UI, and
  non-executable. The first expiry-bound, content-free future-process
  authorization receipt is locally recorded and revocable; it is not an
  execution endpoint and does not alter `ProcessRunSpec` authorization.
- The Router has one decision owner: existing model routing. A Workbench plan
  may link to the decision but cannot alter a live provider request.

## Phased delivery

### Phase 0 — foundations already shipped — Done

- [x] Local Router policy/planner, observe-only route plans, model-routing
  evidence, and live content-free transport observations. Live model
  substitution is not part of this completed foundation.
- [x] Client-adapter lifecycle contracts, config consent, verification and
  rollback.
- [x] Redacted replay, deterministic strategy fixtures, session-event
  prototype, static OSS metadata, and promotion gates.
- [x] Receipt-owned, drift-safe activation rollback for Headroom, RTK,
  Ponytail, Caveman, Leanctx, Chonkify, MarkItDown, and master native add-ons.
  MarkItDown rollback removes only run-created artifacts after their exact
  post-activation fingerprints match; broad Addons cleanup remains explicit.
  Here “Chonkify” names the current local pack preference only; upstream
  Chonkify implementation provenance is unresolved and is tracked below.

### Phase 1 — consolidated architecture and provenance — Done

Deliverables:

- [x] This single Router/Workbench architecture and OSS-reuse plan.
- [x] Explicit reuse/non-reuse decision for DeepSeek Harness, Switchyard,
  JCode, and a unified CLI-harness contract reference.
- [x] Link this plan from the master implementation ledger.

Acceptance: the plan names one authority for routing, client mutation,
promotion, and rollback; no external runtime is silently copied or enabled.

### Phase 2 — native Workbench Kernel — Done

Deliverables:

- [x] `WorkbenchSession` ledger with atomic persistence, retention cap,
  migration/version rejection, content-free validation, and deterministic
  `forkAtEvent` lineage.
- [x] Typed `WorkbenchEvent`, `RunSpec`, `RunPlan`, `CapabilityRequest`, and
  `RouterDecisionReference` contracts.
- [x] Native commands to create, inspect, transition, fork, and export an
  observe-only session/run plan.
- [x] Projection of the existing static OSS registry and client-adapter dry
  run into the kernel without duplicate configuration logic.
- [x] Rust tests for forbidden keys, transition failures, persistence,
  idempotency, retention, and adapter/router boundary failures.

Acceptance met: a user can create, inspect, transition, fork, list, and
prepare an inspectable local plan without provider traffic, shell launch,
workspace mutation, or secret/content persistence. The visible UI follows in
Phase 3.

### Phase 3 — visible Workbench core UI — Done

Deliverables:

- [x] Workbench navigation surface beside Routing, with visible plan-only,
  provider-traffic, and write-state badges.
- [x] Content-free session creation, selection, timeline, explicit lifecycle,
  deterministic latest-event fork, and copy-only ledger export controls.
- [x] Observe-only Router decision references, adapter dry-run plans, bounded
  capability request controls, and truthful desktop/empty/error states.
- [x] Focused UI/bridge tests for navigation, loaded state, digest-only input,
  plan preparation, and the absence of an execution control.

Acceptance met: the non-autonomous Workbench kernel is visible and usable from
the desktop UI; unavailable execution is labelled as unavailable rather than
hidden.

### Phase 3.1 — Router and replay receipt integrity — Done; runtime provenance open

Deliverables:

- [x] Native observe-only completion handles atomically persist redacted
  metrics and one durable Router decision receipt. The receipt has an opaque
  ID, bounded metadata, and a SHA-256 digest over canonical content-free
  metrics; prompts, raw task text, responses, paths, provider payloads, and
  replay inputs remain excluded. Current success, cost, quality, latency, and
  rework metrics are entered through the evidence UI; their provider-request
  provenance is not yet automatic.
- [x] Bounded native list and resolver commands return only receipt-backed
  decisions. A manually recorded evidence event, replay digest, unknown ID,
  missing source event, altered receipt, or malformed policy fails closed.
- [x] The Router evidence screen visibly reports a completed receipt, and the
  Workbench replaces manual Router ID/digest fields with a native picker. Plan
  preparation resolves the selected ID again in Rust rather than trusting the
  renderer.
- [x] Replace the Addons-local OSS registry fetch with the typed shared
  Workbench projection. Native equality and UI tests preserve provider/tool
  labels and fail-closed plan-only/no-traffic/no-write state; registry rows
  remain display-only and cannot trigger Addons lifecycle actions.
- [x] Reuse the existing native redacted-replay validator as the sole parser,
  then issue a separate bounded content-free replay receipt. Its atomic local
  ledger contains only an opaque ID, validation time, counts, fixed
  observe-only flags, and verifiable source/receipt digests—never a path, raw
  event, task class, route/outcome data, prompt, response, or credential.
- [x] Add native replay receipt list/resolve commands and a Workbench picker
  that sends only `replayReferenceId`; native plan creation resolves it again,
  includes it in the plan digest, and rejects capability/receipt mismatches or
  caller-supplied replay metadata. Replay remains provider-free, plan-only,
  non-promoting, and non-executable.
- [x] Add native-owned Router and Workbench presets only as inspectable drafts:
  Router presets load existing observe-only policy into the form without saving;
  Workbench presets compose only existing Router/replay receipts and adapter
  plan capabilities. Preset evidence source, no-write/no-traffic state, and
  plan-only execution state are visible. Presets cannot issue handles, validate
  routes, promote routes, save policy, or execute a plan.
- [x] Harden Workbench plan contracts so every plan declares the existing
  `router_observe` and `client_adapter_plan` capabilities, while replay and
  repository-context capabilities are paired exactly with their native receipt
  or digest inputs. Native preset IDs must resolve and match their capability
  composition exactly.

Acceptance met for integrity and composition: an observe-only Router completion
supplies a digest-verified durable Workbench selection without creating a
second Router or exposing route content. It does **not** yet prove that manually
entered outcome metrics came from a specific intercepted request. A separately
verified redacted replay can be selected with the same native re-resolution
boundary; presets are native-issued plan/policy drafts with no promotion path.

Gate: do not collapse existing Addons, Router, or replay authorities into a
new Workbench copy. Each remaining link must retain its current promotion and
rollback rules.

### Phase 3.2 — live Router lifecycle binding — Remaining build

Deliverables:

- [ ] Call the canonical route-plan authority from one real, bounded proxy or
  session request path. Today `build_route_plan()` is contract/test-only and
  always returns `ObserveOnlyShadow`.
- [ ] Issue a request-bound decision ID before forwarding and complete it with
  native transport/model identity, cost, and latency evidence after the same
  request ends. Quality and follow-up rework must remain explicit benchmark or
  user evidence; they must not be inferred from HTTP success.
- [ ] Use one native Router run identity across ingress, policy decision,
  selected transport, provider completion, and evidence receipt. Do not close
  one transport observation and open an uncorrelated later observation.
- [ ] Reject mismatched, duplicate, expired, or incomplete request/completion
  pairs and preserve the existing content-free event boundary. Add a SQLite
  uniqueness constraint and immediate transaction around evidence validation,
  retention, and insertion so concurrent completions cannot race.
- [ ] Keep the selected live model equal to the requested model throughout the
  initial shadow period. Compare the runtime receipt with the existing manual
  harness before deprecating manual capture.
- [ ] Add a per-request `userApproved` execution path only after the shadow
  receipt proves correct binding and rollback. Replace the policy engine's
  Boolean approval input at the live boundary with a one-shot native receipt
  bound to run, baseline/candidate model, endpoint, policy digest, required
  capabilities, cost/timeout ceilings, expiry, and exact attempt.
- [ ] For substitution, select and canonicalize the candidate model first, then
  choose an endpoint that supports that exact model and its context/tool/vision/
  streaming requirements. Fail closed if model and endpoint cannot be proven
  together.
- [ ] Add `automaticAllowlisted` only after trusted evidence includes freshness,
  model/version/endpoint identity, paired or randomized samples, confidence
  bounds, canary percentage, daily cost budget, drift detection, automatic
  demotion, and receipt-owned return to observe-only.

Acceptance: a captured Router outcome can be traced to exactly one native
request lifecycle without storing request or response content, and observe mode
never substitutes the requested model.

### Phase 4 — execution adapter readiness — In progress, gated

Deliverables:

- [x] Metadata-only Codex and Claude Code compatibility matrix: fixed known
  candidate-location presence is returned without a path; CLI version state is
  explicitly `not_probed` because a version probe would start a process.
- [x] A command-builder-only `RunSpec -> RunPlan` readiness projection using
  the existing dry-run client adapter contract. It has a logical binary and
  adapter-plan ID only—no executable path, argv, shell, environment, working
  directory, instruction, timeout, prompt, credential, provider traffic, or
  start capability. Canonical `codex` and `claude_code` only are accepted.
- [x] A native-owned, content-free `ProcessRunSpec` is attached only to those
  command-ready plans. It is deterministically bound to the session, adapter
  plan, adapter contract, and workspace digest; records `not_started` and
  `not_granted`; and requires a future native fixed timeout, app-owned Unix
  process group, null stdin, bounded redacted output, and TERM-then-KILL group
  cleanup. It has no command path, argv, shell, environment, cwd, prompt,
  credential, PID/PGID, user-controlled timeout, process registry, or start
  endpoint.
- [x] An explicit, fixed-15-minute future-process authorization receipt is
  issued only after native re-preparation of the saved `RunSpec` exactly
  matches the displayed plan and containment IDs. The exact plan-bound phrase,
  active-session state, plan-only/no-traffic/no-write invariants, receipt
  digest, bounded local retention, expiry, terminal-session revocation, and
  manual revocation all fail closed. The UI exposes this receipt lifecycle and
  calls it non-executable; it cannot launch a CLI or change `not_granted`.
- [x] A visible, durable executor-admission receipt is limited to canonical
  Codex with already-verified existing routing. It re-prepares the submitted
  plan, requires the active bound grant, and records only
  `authorized_not_started`; it cannot resolve a binary, launch a child, apply
  configuration, access a workspace, or produce provider traffic.
- [x] Require `session.status == active` independently in both the admission
  command and core admission function. Terminal session state denies admission
  even if a pre-existing grant record still appears active.
- [ ] Repeat the authoritative session/grant/plan revalidation immediately
  before future execution. No execution boundary exists yet.
- [x] Make terminal transition and grant retirement fail-safe across the two
  ledgers, or make session state the authoritative denial and treat revocation
  as cleanup. The terminal transition now remains successful and authoritative
  when injected grant cleanup fails, and a focused test reloads the persisted
  terminal session.
- [ ] Treat admission as a historical receipt, not current eligibility. Derive
  `active`, `expired`, `revoked`, `session_terminal`, and `superseded` status by
  rechecking the grant/session/plan; never trust stored
  `authorized_not_started` as a launch capability.
- [x] Add `deny_unknown_fields` or equivalent explicit persisted-schema
  rejection for sessions, events, grants, and admissions, with corruption and
  forbidden prompt/path/credential/argv/output-field tests. Ledger envelopes
  also reject unknown fields rather than silently retaining future or injected
  content.
- [x] Add direct native `process_supervisor.rs` tests and bridge tests for
  valid/idempotent admission, paused/terminal session, expired/revoked/clock-
  rollback grants, plan drift, unknown adapter, corrupt digest, full ledger,
  restart, and concurrent issue attempts.
- [ ] Add a command-level fake-adapter test for the already-present
  verified-routing prerequisite; do not depend on a developer machine's real
  Codex installation or configuration.
- [ ] Move authorization/admission history to a session-level receipt center,
  refresh at grant expiry, clear stale data on session changes/errors, and
  invalidate or freeze a prepared plan when any visible input changes.
- [ ] Version probing or runnable-binary validation, only after an explicit
  process-start capability and containment/receipt model are available.
- [ ] Process registry, actual bounded timeout/cancel enforcement,
  content-free metrics, receipt-backed cleanup, and a separately gated native
  executor. The current authorization receipt is intentionally insufficient to
  start or supervise a process.
- [ ] Deterministic fake-adapter tests, then a separate opt-in local manual
  test. No provider credentials are read by the planning path.

Immediate Phase 4 order:

1. **4.1 Admission correctness** — active-session enforcement, cross-ledger
   failure handling, strict schemas, direct native/bridge tests, derived
   eligibility states, and bounded retention/reclamation.
2. **4.2 Receipt and consent UX** — session-level history, expiry refresh,
   immutable prepared-plan summary, input-change invalidation, and truthful
   historical-versus-current labels.
3. **4.3 Owned process controller** — Workbench-specific fixed command catalog,
   version probe, app-owned process group, null stdin, environment allowlist,
   bounded redacted buffers, fixed timeout, idempotent cancellation, reaping,
   and TERM-then-KILL cleanup. Reuse process-group and Leanctx shutdown ideas,
   not the generic runner's argument/stdout/stderr-bearing error surface.
4. **4.4 One-adapter opt-in executor** — canonical Codex only, behind a new
   explicit execution capability. Revalidate session, grant, admission,
   adapter/routing verification, binary identity, and workspace digest at the
   final start boundary.

Gate: no arbitrary shell, terminal, browser, provider, or workspace write can
be promoted without per-capability approval, process ownership, cancel/resume,
event redaction, rollback, and local/manual evidence.

### Phase 5 — guarded execution and orchestration — Remaining build

Deliverables:

- [ ] Opt-in local execution backend for one approved adapter at a time. The
  renderer chooses an adapter/task class, never a binary, shell, argv,
  environment, timeout, PID, or process group.
- [ ] Transient app-owned workspace handle that resolves an already selected
  directory only inside the native start boundary. Recompute and compare its
  identity/digest immediately before launch; do not persist the path in the
  content-free session ledger.
- [ ] Bounded ephemeral task envelope, separately consented and digest-bound to
  the grant/admission. Do not persist prompt/task text in session, grant,
  admission, telemetry, crash, or execution-receipt ledgers.
- [ ] Native run state machine (`starting`, `running`, `cancelling`, terminal),
  immutable content-free exit receipt, restart reconciliation, stale-process
  cleanup, and user-visible diagnostics that cannot disclose command output or
  credentials.
- [ ] Goal queue, bounded subagent scheduler, workspace lock/conflict model,
  and human approval checkpoints.
- [ ] Attach/resume/cancel and replay/fork semantics with execution receipts.
- [ ] Budget and concurrency limits, deterministic completion/failure tests,
  and visible per-session resource/evidence status.
- [ ] Separate capability grants for workspace read, workspace write, tool use,
  provider traffic, publishing, and external side effects. A process-start
  grant must never imply all of these capabilities.

Gate: subagents may propose or run only granted capabilities. They cannot
escalate privileges, publish, apply external changes, or absorb private prompt
content into the session ledger.

### Phase 6 — optional upstream interoperability — Prepared, externally gated

- [ ] A specific pinned DeepSeek/Switchyard/JCode workflow with compatibility,
  licence attribution, privacy, rollback, operational ownership and release
  evidence.
- [ ] A dedicated optional profile that is disabled by default and removable
  through a receipt-owned rollback.
- [ ] One machine-readable OSS inventory is the source for product labels and
  release notices: source repository, exact commit/version, licence,
  distribution mode, checksums/lockfile, copied code, notices, runtime owner,
  and rollback authority.
- [ ] Resolve JCode's conflicting repository references before reuse; expose the
  existing DeepSeek adapter as an explicit Experimental connector with preview,
  consent, apply, verify, rollback, and separate configuration-versus-runtime
  health—or remove its prototype-only modules from the production graph.
- [ ] Pin Ponytail instead of `latest`, lock/review MarkItDown's `[all]`
  dependency set, and classify Caveman as Switchboard built-in guidance rather
  than an external runtime/plugin source.

The existing DeepSeek adapter is the first experimental integration candidate:
it already pins `dsh 0.1.0-rc.5` and upstream commit
`47f943859bef60e4160492346772ded9b24f765a`, uses the normal adapter
plan/consent/apply/verify/rollback lifecycle, and becomes guided-only for an
unknown or ambiguous version. It must not silently become a Workbench executor.
Switchyard remains an external endpoint/protocol interoperability candidate,
and JCode remains a session/context design reference until an exact workflow is
selected. Do not combine their routing authority with Switchboard's Router.

## Prioritized improvements

| Priority | Improvement | Why it matters | Acceptance evidence |
|---|---|---|---|
| P0 — Admission done; execution gate remains | Enforce active session at admission/execution | A terminal session can be persisted before separate grant revocation fails | Injected cleanup failure proves the persisted session is authoritative for admission; repeat the same check at the future start boundary |
| P0 — Done | Strict persisted schemas | Serde previously tolerated unknown fields, weakening the content-free claim | Unknown `prompt`, path, credential, argv, output, and arbitrary envelope fields are rejected from every durable Workbench ledger |
| P0 — Done | Direct admission tests | `process_supervisor.rs` previously had no native test module | Native and TypeScript bridge matrices cover valid, corrupt, expired, revoked, terminal, drifted, unknown-adapter, full, restart, and concurrent cases |
| P0 — Done | Restore repository gates | The full build and OSS integration checker were red | Preserve the compression-mode union and validate the shared Workbench capability projection; both gates now pass |
| P0 | Correct Chonkify identity | The promoted adapter is local head/tail compaction while fixtures attribute upstream Chonkify | Rename/localize the feature and attribution, or pin and integrate upstream code with parity, licence, omission, and source-span tests |
| P0 | Split planner from live Router status | The policy and endpoint planners are complete but have no production model-routing caller | Inventory and UI say planner/evidence done, live correlation remaining, approval remaining, automatic routing gated |
| P1 | Historical versus current eligibility | Stored `authorized_not_started` remains visible after grant/session validity changes | Session receipt center derives current state and refreshes at expiry; launch always revalidates native state |
| P1 | Immutable plan consent | Form edits can visually diverge from the saved plan snapshot | Any plan-input edit clears the plan or inputs are frozen with an exact immutable summary and plan ID |
| P1 | Restart-safe selective rollback | The persisted selective run cannot be rediscovered by the current card after restart | Native selection and last receipt load on mount; safe retry and drift-aware rollback remain available after relaunch |
| P1 | Workbench-specific process controller | The generic runner can retain arguments/stdout/stderr and does not implement the declared graceful cleanup contract | Fake-process tests prove fixed command catalog, null stdin, allowlisted environment, bounded redaction, timeout, cancel, TERM/KILL, reaping, and restart cleanup |
| P1 | Live Router provenance | Manual metrics are integrity-checked but not request-bound | One decision and completion receipt pair is generated by the same intercepted request lifecycle while model substitution stays off |
| P1 | Transactional routing evidence | Duplicate/retention checks can race across SQLite connections | Unique index plus immediate transaction rejects concurrent duplicate and 128/129-boundary inserts deterministically |
| P1 | Joint model/endpoint decision | Current endpoint planning proves only the requested model before any candidate substitution | Canonical candidate model is selected first and only an endpoint supporting that exact model/capability/cost/latency envelope is eligible |
| P1 | Workspace and task privacy seam | Execution needs a working directory and task while durable ledgers prohibit both | Native transient workspace handle and ephemeral task envelope are digest-bound, bounded, consented, and absent from persisted artifacts |
| P1 | Reproducible add-on supply chain | Ponytail tracks `latest`; MarkItDown pins the top package but `[all]` transitives are not locked | Pin reviewed versions/commits/checksums or lockfiles, verify downloads/install set, and record notices/SBOM without pretending runtime proof |
| P1 | OSS identity and UI reconciliation | JCode references disagree; DeepSeek is internal; Caveman is presented as external despite having no external runtime | One machine-readable inventory drives docs/UI and records source, exact pin, licence, distribution, checksums, copied code, notices, and built-in versus external classification |
| P2 | Receipt retention and repair | Grant/admission ledgers fail when they reach 128 records; partial add-on runtimes need recovery | Terminal/inactive record reclamation is deterministic and auditable; UI offers non-destructive export/repair rather than silent deletion |
| P2 | Route/component reachability contract | A production component can regress into an imported-only or polling-while-hidden state | Test maps every top-level route to navigation and asserts inactive mounted views suspend timers/subscriptions |
| P2 | OSS optional profile | Evaluations alone do not create user value | One disabled-by-default pinned profile has attribution, compatibility, diagnostics, consent, and receipt-owned uninstall evidence |

## Remove or keep out

These are not backlog items; adding them would weaken the architecture:

- Remove any duplicate Workbench router, provider registry, client mutation
  implementation, or rollback engine. Reuse the current Router, shared OSS
  projection, `CodingClientAdapter`, and activation receipts.
- Do not expose renderer-controlled executable paths, arbitrary argv/shell,
  environment variables, working directories, timeout values, or raw output.
- Do not label a saved configuration stage, historical admission, HTTP 2xx, or
  fixture result as live routing/execution eligibility.
- Do not persist prompts, model messages, tool arguments/results, source paths,
  credentials, headers, or provider payloads in Workbench, Router, replay,
  telemetry, crash, or diagnostic ledgers.
- Remove genuinely unreachable production components after a reachability
  audit. Keep safety-gated controls visible with the missing prerequisite and
  repair action; a safety gate is not a reason to hide a useful surface.
- Do not vendor DeepSeek Harness, Switchyard, JCode, or another scheduler merely
  to duplicate local capabilities. Reuse pinned contracts/adapters and keep
  optional runtimes removable.

## Edge-case and harness matrix

| Boundary | Required behavior | Required test |
|---|---|---|
| Paused/completed/cancelled session | Deny new plan authorization, admission, and execution even if a grant record still says active | Inject terminal grant-revocation failure and attempt admission/start |
| Expired/revoked grant | Historical receipt remains inspectable; current eligibility is denied | Fake clock at boundary, clock rollback, manual revoke, and restart |
| Plan or form drift | Never authorize what is merely visible after the saved snapshot changed | Modify every RunSpec input after prepare and assert plan invalidation/native mismatch |
| Router/replay receipt tampering | Fail closed on unknown ID, digest mismatch, missing source event, duplicate completion, or cross-session reuse | Corrupt each persisted field and re-resolve in native code |
| Binary missing/version mismatch | No launch and no fallback PATH search outside the fixed catalog | Fake resolver covers absent, symlink swap, unsupported, ambiguous, and changed-after-probe binary |
| Task/workspace swap | Final native digest revalidation denies launch | Replace workspace identity or task-envelope digest between approval and start |
| Environment/credential leakage | Only fixed safe variables reach child; no inherited provider secrets by default | Fake child enumerates environment and stdin; persisted receipts are content-free |
| Output flood/invalid UTF-8/secret-like text | Bounded buffer, deterministic redaction/drop counters, no raw durable output | Chunk boundary, binary data, huge line, ANSI/control, and credential-pattern fixtures |
| Timeout/cancel race | One terminal receipt, idempotent cancel, TERM grace then KILL group, all children reaped | Fake process tree covers before-start, during-start, simultaneous-exit, hung child, and repeated cancel |
| App crash/restart | Reconcile only app-owned process identities; never kill an unrelated reused PID | Persist opaque ownership token, simulate PID reuse, orphan, completed child, and corrupt registry |
| Concurrent agents/workspace writes | Read-only can share; write grants require an exclusive workspace lock | Competing session/subagent tests, stale-lock recovery, and external file-drift detection |
| Ledger corruption/full retention | No silent reset or overwrite; safe export/repair and deterministic inactive-record reclamation | Unknown schema/field, partial JSON, digest mismatch, 128/129 record, and interrupted atomic rename |
| Selective activation partial failure | Preserve successes and exact ownership; support safe retry/rollback without broad cleanup | Fail each of ten actions in sequence and relaunch before retry/rollback |
| Add-on external drift | Preserve user/external changes and report the exact blocked artifact | Mutate every recorded post-activation fingerprint before rollback |
| Hidden mounted route | No background polling, provider calls, or stale state overwrite while inactive | Fake timers and delayed response generation guard for every top-level route |
| Model-routing evidence drift | Never infer quality/rework from transport or mix task/model identities | Mixed arm/model/task, duplicate run, manual/runtime provenance, and insufficient sample tests |
| Concurrent routing completion | At most one observation for a run/time/task/arm and deterministic retention | Two SQLite connections race at duplicate and retention boundaries inside an immediate transaction |
| Candidate model/endpoint mismatch | No substituted model reaches an endpoint selected only for the baseline model | Candidate unavailable, missing capability, context overflow, quota/rate, and endpoint health changes after planning |
| Approval replay | One approval authorizes one exact route attempt and then expires/consumes | Changed model/endpoint/policy/cost/timeout, duplicate use, clock rollback, and cross-client reuse |
| Upstream OSS change | Profile stays disabled when commit/licence/schema differs from its pin | Compatibility fixture for known pin plus unknown version/commit/licence failures |
| Add-on dependency drift | Do not install `latest` or an unreviewed transitive set as reproducible release behavior | Lock/pin mismatch, checksum failure, unavailable pin, changed dependency graph, and offline repair tests |

## Dependency-ordered implementation and commit sequence

Each row is a separately reviewable commit and push. Later rows must not be
started by weakening an earlier gate.

1. **Roadmap checkpoint — Done (`43bcbcc0`)** — this
   audit/status/edge-case plan.
2. **Repository gate repair — Done** — frontend compression-mode type fix and
   shared Workbench OSS capability checker coverage; full build and aggregate
   OSS integration gate pass.
3. **Admission hardening — Done** — authoritative active-session check, strict
   ledger schemas, cross-ledger failure behavior, direct native and bridge
   tests. Historical/current eligibility remains deliberately in Receipt UX.
4. **Receipt UX** — derived eligibility, expiry refresh, session receipt center,
   immutable-plan display/invalidation, restart-safe selective receipt loading.
5. **Fake process controller** — no real CLI; deterministic resolver, registry,
   output/redaction, timeout/cancel/cleanup state machine and tests.
6. **Codex probe and opt-in manual harness** — fixed binary catalog/version,
   disposable workspace, no provider credential, no workspace write.
7. **Single Codex executor** — new explicit execution capability, ephemeral task
   and workspace handle, final revalidation, content-free terminal receipts.
8. **Live Router shadow binding** — request/completion receipt pair in the real
   path with actual model unchanged.
9. **User-approved Router stage** — per-request consent and rollback; no
   automatic promotion.
10. **Evidence-gated automatic Router stage** — only allowlisted task classes
    with trusted passing benchmark/runtime evidence and global/client kill
    switches.
11. **Goal/subagent orchestration** — queue, locks, budgets, checkpoints,
    attach/resume/cancel/fork, and capability-separated tools.
12. **Optional OSS profile** — one pinned, attributed, disabled-by-default,
    receipt-removable integration.
13. **Release evidence** — clean build/test/check suite, signed package, and
    separate manual runtime/accessibility/security acceptance.

## Verification and publication

Each phase is committed and pushed separately. Required evidence is scoped to
the phase: deterministic Rust/TypeScript tests, type checking where the
existing checkout permits it, no sensitive fields in persisted artifacts,
read-only/no-network plan-mode proof, and a clean staged diff. Existing user
work remains unmodified and unstaged.

Minimum automated gates by area:

```bash
npm test -- --run src/components/SelectiveActivationCard.test.tsx \
  src/lib/activationTools.test.ts src/lib/workbench.test.ts \
  src/components/WorkbenchView.test.tsx \
  src/components/ModelRoutingExperimentCard.test.tsx \
  src/components/OptimizationSupportingPanels.test.tsx \
  src/components/SettingsView.integration.test.tsx
cargo test --manifest-path src-tauri/Cargo.toml workbench_kernel --lib -- --test-threads=1
npm run check:model-routing-evidence
npm run check:oss-harness-integrations
npm run check:chonkify-promotion-gate
npm run build
git diff --check
```

Current gate truth on 2026-08-24:

- Focused frontend minimum gate: `34 passed` across all seven listed suites.
- Native Workbench: `46 passed`.
- Model-routing evidence: `13` Node, `35` native routing, and `18` native
  telemetry tests pass.
- Chonkify-labelled promotion fixture: passes with MIT metadata and zero
  wrong-omission rate; this does not prove that the local adapter implements or
  vendors upstream Chonkify.
- Full frontend build: passes (`tsc && vite build`).
- OSS harness integration: passes `13` strategy/session/provider Node tests,
  `22` shared Workbench/Addons frontend tests, `2` native registry tests, the
  exact native Workbench projection test, and the required-file/observe-only
  boundary checker.

Manual acceptance remains separate from automated proof. Before execution is
called usable, verify cancellation and full child-tree cleanup in a disposable
workspace, app restart reconciliation, no credential/output persistence,
VoiceOver/keyboard status and consent flows, and truthful behavior with Codex
missing, unsupported, unauthenticated, offline, or returning malformed output.
