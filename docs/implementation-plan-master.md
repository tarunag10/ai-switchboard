# AI Switchboard Master Implementation Plan

Updated: 2026-08-25

This is the **single canonical implementation plan** for AI Switchboard. It merges
every prior implementation plan, roadmap, and reconciliation document into one
phase-wise program with consolidated status. Older plans remain available as
design history or technical appendices; their status labels are not
authoritative. A slice is marked **done** only when implementation, tests or a
deterministic gate, and the documented safety boundary are all present in the
current checkout.

## 1. Source plans merged

| Document | Role after merge |
|---|---|
| `AI-SWITCHBOARD-PHASED-IMPLEMENTATION-PLAN.md` (root) | Program specification for the Phase 0–6 spine below; status lives here. |
| `docs/product-roadmap-plan.md`, `docs/mac-ai-switchboard-implementation-plan.md`, `docs/agent-control-center-implementation-plan.md`, `docs/implementation-roadmap-2026-08-21.md` | Design history; open items absorbed into §4/§5. Their phase numberings are retired. |
| `docs/architecture/router-workbench-implementation-plan.md` | Technical appendix owning Router/Workbench kernel detail; open items mirrored in R6–R9. |
| `docs/refactor-platform-readiness-plan.md`, five-crate decomposition | Technical appendix for cross-platform layering; mirrored in R10. |
| Feature plans: Agent Memory, Token X-Ray/Briefing, progressive disclosure, Repo Intelligence | Feature appendices; done slices verified once here, depth work in R11–R13. |
| `docs/world-class-token-savings/*`, `docs/token-optimization-addons-implementation-plan.md`, `docs/gateway-addons-implementation-plan.md` | Trust/savings/add-on appendices; gated engines and live gateway evidence in R14–R15. |
| Integration plans: DeepSeek Harness, Switchyard, jcode, vLLM/SGLang/TensorRT/LiteLLM/Dynamo/llama.cpp evaluations under `docs/integrations/` | Source-specific evidence appendices; only locally integrated, gated capabilities are product commitments. |
| Security/trust/release docs: Fable plan, threat model, rebrand trust hardening, platform rebrand, terminology, design system | Policy and release appendices; release truth owned by §5 and `docs/release-truth.json`. |

## 2. Status vocabulary and evidence rules

Exactly four states:

- **Done** — shipped and locally verified in this checkout (code + test/gate + safety boundary).
- **Prepared / externally blocked** — code, schema, and checker ready; completion requires a real signed installation, provider, infrastructure, or reboot that local code cannot fabricate.
- **Remaining build** — scoped implementation slice still needs code and tests (items R1–R18).
- **Intentionally gated** — supported as a safe contract, but promotion prohibited until stated evidence passes.

Rules: done claims must point at code plus a test or deterministic gate; blocked
claims must name the required external artifact. Automatic routing, semantic
replay, native connector writes, and release trust never become enabled merely
because a fixture or local build passed. Each completed slice ships as its own
commit on `main`.

## 3. Unified phase map

Three legacy numbering systems are retired into one spine:

| Unified phase | Root phased plan | Legacy roadmap 1–8 | Roadmap 2026-08-21 |
|---|---|---|---|
| P0 Baseline + hardening | Phase 0 | 1–4 | 1–4 |
| P1 Interfaces + Mac app shell | Phase 1 (incl. §15 UX) | 5–6 | — |
| P2 vLLM + DeepSeek Harness | Phase 2 | — | 6 |
| P3 SGLang + benchmark-gated policy | Phase 3 | — | 5 |
| P4 Local inference + cache maturity | Phase 4 | 7 (partial) | — |
| P5 Enterprise / hyperscale | Phase 5 | — | — |
| P6 Plugin ecosystem + GA hardening | Phase 6 | 8 | — |

Cross-cutting streams (not phase-bound): Router/Workbench execution (R6–R9),
cross-platform extraction (R10), savings/attribution depth (R13),
maintainability (R18).

## 4. Phase status

### Phase 0 — Baseline + hardening: **DONE**

Terminology locked (`docs/terminology.md`); ADRs 0001–0007 complete with
context/decision/alternatives/consequences/reversal; benchmark manifest +
stored baseline + threshold comparison wired through `benchmarks:check` in CI;
`LiveBenchmarkTarget` trait with deterministic 2×2 (B00/B10/B01/B11) mock proof
(`src-tauri/src/live_benchmark.rs`); 13-case request-path regression fixture
locked by test (`proxy_intercept.rs`); connector lifecycle matrix generated from
fixtures and fail-closed (`check-connector-lifecycle-matrix.mjs`); security
baseline green (`scripts/check-phase-0-security-baseline.mjs`, 15 named tests).

### Phase 1 — Interfaces + Mac app shell: **DONE except R5a**

Done: `CodingClientAdapter` contract with structured detection/plan/receipt/
verify/rollback/footprint (claude_code, codex, gemini_cli, deepseek_harness);
`OptimizationEngine` wrapping Headroom; read-only `ContextProvider`;
`InferenceEndpoint` with exactly two Phase-1 classes plus URL validation;
`EndpointCapabilities` as data; unified `RequestFacts…UserPolicy → RouteDecision`
policy facade (conservative, observe-only). Main window + sidebar + tray
companion ship; complex controls are main-window-first; one Rust state source.

Remaining: R5a (remaining connectors behind the adapter) and the sidebar
"Agents & Connectors" destination (folded into R5a).

### Phase 2 — vLLM + DeepSeek Harness: **DONE**

Generic OpenAI-compatible endpoints with allowlist/explicit consent/classification;
vLLM verified profile (health/server_info/models probes, capability mapping,
AIPerf-tagged benchmark metadata, manual select/disable without client rewrites);
benchmark adapter capturing TTFT/ITL/TPOT/e2e/throughput/prefix-cache/queue/GPU
(developer mode); `DeepSeekHarnessAdapter` full lifecycle with pinned upstream
version/SHA, guided-mode degradation on unknown versions; dsh routes through its
documented Cordis patch seam (no core patching); native context prototype ships
offline-deterministic evidence. Live dsh seam stays experimental until upstream
stabilizes (externally gated).

### Phase 3 — SGLang + benchmark-gated policy: **DONE except R2**

SGLang verified profile proves endpoint independence (test asserts no
CodingClientAdapter change between vLLM/SGLang); capability states
supported/unsupported/unknown/configured/observed kept distinct; rule-based
net_value scoring in `optimization/action_policy.rs`; routing stages
Observe → UserApproved → AutomaticAllowlisted with persisted policy, evidence
store, and threshold gating; endpoint routing separate from model routing with
fail-closed no-fallback; cache-aware four-variant compression gate enforced.

Remaining: capability field completeness (R2).

### Phase 4 — Local inference + cache maturity: **DONE except R3/R12/R15 tails**

llama.cpp verified loopback/local-network profile with quantization metadata;
Switchyard deliberately evaluated-not-promoted (documented anti-goal compliance);
LiteLLM readiness with secret redaction and opt-in loopback probes; Exact
Response Cache naming/UI contract; true semantic cache experiment fail-closed
with isolation/stale-code/task-type restrictions; LMCache promotion gate with
paired-arm thresholds.

### Phase 5 — Enterprise / hyperscale: **DONE except R14 tail**

Content-free OTel/Prometheus-style telemetry schema with forbidden-field tests;
Envoy AI Gateway enterprise profile (connect-not-manage); Dynamo selected as the
one hyperscale proof (frontend health/models only, no cluster control);
TensorRT-LLM endpoint with enterprise-gateway rule; multi-tenant policy
(roles, scopes, quotas, `AutomaticRoutingDenied`) plus content-free enterprise
evidence chain validation.

### Phase 6 — Plugin ecosystem + GA hardening: **DONE except externally blocked proof**

Promotion-gate framework (provenance, SPDX license, network declaration,
deterministic fixtures, quality/wrong-omission thresholds, version pin, update
source) with all six plugin categories; DSH plugin maturity audit correctly
fail-closed during upstream developer preview; release-proof automation
(installed smoke, reboot-level arm/record/check, `release:proof`,
`check:phase6-hardening`) complete locally — public external evidence remains
blocked (see §5).

## 5. Prepared but externally blocked

Local code cannot manufacture these artifacts; each names its requirement:

- **Public installed-app smoke and reboot-level Doctor/Rollback/uninstall proof** — requires a current signed/notarized install, public release artifacts, and a real post-reboot marker. Arm/record/check automation is complete; `release:proof` stays red until then.
- Public release proof, updater feed/signature metadata, strict notarized distribution — requires signing credentials and reachable release infrastructure.
- Live LiteLLM managed lifecycle, Langfuse export, Cloudflare passthrough, Kong evidence — requires user-controlled infrastructure and credentials (guided readiness shipped).
- Durable provider-billed counterfactual measurement where providers expose no usage API.
- dsh native context prototype against a live plugin seam; optional external OSS interoperability — both wait on stable pinned upstream workflows.
- Windows runtime execution — requires Job Object containment and full runtime layout support.

## 6. Intentionally gated / frozen

Automatic model/endpoint routing stays observe-only; more native provider
writes; semantic-cache similarity replay; enterprise gateways as defaults;
vendored runtimes or network replay of harness sessions; "tiny universal token
proxy" integrations; Chonkify/LLMLingua-2/pxpipe activation profiles (license/
provenance/quality seams pending); leanctx live flip; Cursor native writes
until Cursor publishes a documented supported on-disk schema. Per the routing
invariant, automatic routing stays observe-only until that evidence exists
(R1/R7).

## 7. Remaining build work

Prioritized, deduplicated across every merged source. One commit per slice.

- **R1 — Fresh quality-evidence loop + Class C coding-agent task benchmark.** Define the Class C schema (`task_id/repo_fixture/task/success_command/allowed_files/quality_assertions/expected_risk`), fixtures, runner recording tokens/files/tool calls/retries/test result/model/endpoint/profile/cache reads; import successful-task/rework/quality/latency observations through a central production provider/client completion hook. Feeds Stage-A→C routing promotion and compression gates.
- **R2 — Capability normalization completeness.** Add `parallelism`, `max_context`, `tool_calling` to `NormalizedRuntimeCapabilities`; keep configured-vs-supported distinct; update vLLM/SGLang/llama.cpp mappings and fixtures.
- **R3 — Security metadata backlog (P1/P2).** `trust_remote_code=false` recommendation/default metadata on endpoint profiles; model provenance + SPDX license inventory; response-cache body-at-rest protection design (ADR: encrypt-or-accepted-risk) with namespace tests.
- **R4 — Add-on measurement matrix.** Scripted 5-arm runner (baseline / Ponytail / Caveman scoped / Caveman aggressive / combined) over Class A + C fixtures measuring success/LOC/unnecessary LOC/tool calls/tokens/rework; promote live runtime counters for Caveman/Ponytail/MarkItDown above inferred fallbacks; advertise only evidence-matched claims.
- **R5 — Connector completion.** (a) Migrate remaining managed connectors (OpenCode, Grok/xAI, Aider, Continue, Goose, Qwen, Amazon Q, Windsurf, Zed) behind `CodingClientAdapter`; add the sidebar "Agents & Connectors" destination consuming structured status. (b) Promote native writes connector-by-connector only with documented schemas + full lifecycle proof; Cursor remains gated; Qwen Code/Amazon Q stay guided until schemas are proven.
- **R6 — Workbench execution foundations.** Bind v2 grants/admissions to plan-head id/generation/digest under the shared transaction; introduce protected monotonic state with crash-repair semantics; authenticated parent/helper IPC (owner epochs, freshness proof, payload lease, atomic reservation).
- **R7 — Live Router shadow binding.** Call `build_route_plan()` from a real bounded request path; request-bound decision ids completed with transport/model/cost/latency evidence; one run identity ingress→completion; SQLite uniqueness under racing completions; requested model preserved during shadow; user-approved stage next; `automaticAllowlisted` only after freshness/canary/confidence/drift/auto-demotion evidence.
- **R8 — Guarded execution backend.** Opt-in single-adapter execution; transient workspace handles; digest-bound task envelopes; real owned-process controller with timeout/cancel/reap and TERM-then-KILL group cleanup; goal queue, bounded subagent scheduler, workspace locks, human approval checkpoints, budget/concurrency limits, capability-separated grants.
- **R9 — OSS migration remainder.** Full upstream LICENSE/NOTICE bundling; pick one pinned DeepSeek/Switchyard/JCode workflow with full evidence; resolve JCode dual-repository provenance; expose or remove prototype DeepSeek production modules; MarkItDown locked bundled wheels; bundle leanctx no-model core; RTK built from reviewed revision as sidecar; remove Headroom/DeepSeek runtime downloads after hash/licence/offline/parity tests; helper app Developer ID signing path.
- **R10 — Cross-platform extraction.** Finish `switchboard-core` extraction (route plans, Workbench sessions, grants, receipts, capability projection, OSS contracts); `switchboard-runtime` traits (filesystem/process/clock/provider transport/secrets); Tauri crate as thin adapter; macOS/Linux/Windows runtime adapters + CI compile/smoke jobs (publish needs `workflow` scope); package installers only after passing evidence; CLI command-surface parity (`status|doctor|proxy start/stop|optimize|session start|xray|cache report|redundancy report`).
- **R11 — VS Code/Codex session-history import.** Read-only discovery → versioned source adapters → preview → fail-closed bounded input → immutable provenance-receipted records → Workbench/Agent Memory integration → CLI parity → edge-case verification matrix.
- **R12 — Repo Intelligence depth.** Persistent parser index beyond latest-summary reuse; deeper language-specific analyzers where deterministic; long-running MCP supervision depth. Whole-program type inference and dynamic dispatch stay out of scope absent an approved evidence-backed design.
- **R13 — Savings/attribution depth.** Provider-specific X-Ray metrics where credible durable APIs exist; savings-anomaly trend thresholds (day/week); per-client exact request-count history; dedicated Caveman health card rollups.
- **R14 — Gateway addons live evidence.** LiteLLM managed lifecycle promotion, Langfuse self-hosted export, Cloudflare passthrough verification, Kong dossier conditions — each requires user infrastructure; Doctor + Rollback rows must appear in release JSON when enabled.
- **R15 — Gated engine profiles.** leanctx live-eligibility evidence (protected-content byte-exact fixtures, fail-open, no task regression); upstream Chonkify license resolution; LLMLingua-2 benchmark pass; pxpipe waits on the upstream Headroom `text_image` contract.
- **R16 — Release proof operator path.** Execute the external checklist for §5 item 1: signing credentials, current public artifact, real reboot; then refresh `release-truth.json`, updater metadata, launch-at-login test, legacy-storage migration test, and the Repo Memory MCP signed-relaunch survival sub-proof.
- **R17 — Product decisions.** Upstream account/pricing surface keep/replace/remove; legacy Headroom-storage compatibility sunset after a migration-evidenced release; signed public channel + auto-update promotion workflow; App Store distribution research; staged repo/package slug rename to `ai-switchboard`.
- **R18 — Maintainability.** God-file registry upkeep and scoped refactors (`lib.rs`, `state.rs`, `tool_manager.rs`, `App.tsx`) touched-area-only; Mode Inspector active-state completeness; broaden local-only network certification; settings-migration auto-apply after native-gate restore evidence.

## 8. Execution order

1. R1 → R7 (evidence loop unblocks routing promotion; highest leverage).
2. R2, R3 (small, independent security/capability wins).
3. R5 (connector completion; mechanical delegation onto the proven contract).
4. R16 preparation (operator handoff checklist; do not fabricate).
5. R6 → R8 → R9 (Workbench execution stream), R10 in parallel.
6. R4, R11–R15, R17, R18 as capacity allows, respecting stop rules.

Stop rules still apply verbatim from the root program spec (§23): boundary
violations stop endpoint work; failed benchmarks keep routing observe-only;
rollback-less connectors stay unmanaged; valueless proxy hops get removed;
isolation-less caches stay disabled; UI controls that cannot explain recovery
never become one-click automation.

## 9. Definition of done

Unchanged: every done slice needs code + focused tests or a deterministic gate +
documented safety boundary in this checkout; blocked slices name the missing
external artifact; gated slices name the promotion evidence; each completed
slice ships as its own commit on `main`. `npm run check:implementation-plan-master`
mechanically validates this file's boundaries against release truth.
