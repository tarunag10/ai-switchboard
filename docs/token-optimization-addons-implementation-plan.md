# Token Optimization Add-ons Implementation Plan

Status: active implementation. Existing Headroom and RTK behavior remains the
default while the add-ons below are implemented behind explicit, reversible
gates.

Evidence and current repository status are recorded in
[token-optimization-addons-research.md](token-optimization-addons-research.md).

This plan turns the current priority order into an implementation sequence:

1. `leanctx` or raw LLMLingua-2 as a pluggable text-compression engine.
2. `chonkify` for Switchboard-generated repository/context packs.
3. Semantic caching as a separate, explicitly labelled cost-saving feature.
4. pxpipe as an experimental visual-compression option.
5. Do not bundle small universal token proxies until their evidence is strong.

## Implementation status — 2026-07-18

The first local-first slice is now in the checkout:

- Shared engine contracts, lifecycle receipts, secret-safe previews, and
  governance validation are present in the Addons surface.
- Leanctx has a guided, user-configured local sidecar lifecycle: registration,
  loopback-only `/health` verification, shadow mode, disable, uninstall, log
  capture, and app-exit cleanup. It does not install an unpinned package, does
  not forward provider traffic, and does not replace Headroom.
- Repo Intelligence emits the native-pack disclosure and has a deterministic,
  provenance-preserving chonkify adapter seam. Chonkify remains blocked while
  license/provenance evidence is incomplete.
- Semantic cache now has a local SQLite backend and an opt-in exact replay path
  inside the existing Switchboard intercept. It is separate from compression,
  disabled by default, restricted to bounded non-streaming JSON requests
  without tools/MCP/protected-content markers, and fail-open on every
  read/write error. The prompt is hashed for the key; status, hit/miss counts,
  clear, and disable actions are exposed to Addons and Doctor.
- pxpipe and raw LLMLingua-2 remain blocked/design-only pending the upstream
  and quality gates below.

The leanctx sidecar is therefore a real managed shadow/readiness path, not yet
a live request transformer. Headroom 0.27.0 remains the only live request
compressor; the pinned managed runtime was verified locally to expose its
native compressor and documented user-message/output-shaper controls. The
next promotion gate for any third-party text engine is still a verified
Headroom-owned compressor seam plus protected-content and fail-open fixtures.

### Guided leanctx configuration

The current lifecycle intentionally accepts only explicit local configuration:

- `LEANCTX_EXECUTABLE`: absolute path to the user-installed leanctx executable;
- `LEANCTX_BASE_URL`: `http://localhost`, `127.0.0.1`, or `::1` endpoint;
- `LEANCTX_ARGS_JSON`: optional JSON array of argv strings, never a shell
  command; and
- `LEANCTX_VERSION`: optional user-supplied version label for the receipt.

Registration stores these values in a Switchboard-owned receipt, and enablement
starts the executable with `LEANCTX_SWITCHBOARD_MODE=shadow`, checks `/health`,
and records no raw prompt content. Disable stops the child before writing the
off receipt. The configured process is never made the provider endpoint.

## 1. Product goals

- Reduce provider input tokens without creating a second competing localhost
  proxy.
- Keep Headroom as the sole managed provider-routing owner.
- Preserve exact bytes for secrets, identifiers, hashes, paths, tool arguments,
  JSON values, current instructions, and open tool state.
- Make every optimization inspectable, measurable, reversible, and independently
  disableable.
- Separate measured token savings from estimated context avoidance, semantic
  cache hits, and external vendor claims.
- Keep context processing local by default. Any remote gateway or hosted model
  used for optimization must be an explicit, separately disclosed choice.

## 2. Non-goals

- Do not add a chain of independent proxies such as
  `pxpipe -> Headroom -> LiteLLM -> provider` as the default architecture.
- Do not rewrite the system prompt, tool definitions, current user turn, or
  open tool calls until provider-specific cache and tool-state tests prove it is
  safe.
- Do not claim that compression is lossless.
- Do not make a cache hit look like token compression.
- Do not install a large ML model or a cache backend during normal app startup.
- Do not bundle small, weakly evidenced “universal token proxy” projects.

## 3. Target architecture

```text
Client
  |
  v
Switchboard-managed Headroom proxy
  |
  +-- request safety classifier
  +-- native byte-preserving protection zone
  +-- selected compression engine
  |     +-- existing Headroom compressors
  |     +-- optional leanctx / LLMLingua-2 text engine
  |     +-- optional pxpipe text-image engine
  +-- provider adapter and cache policy
  +-- measured savings receipt
  |
  +--> provider

Repo Intelligence context pack path
  |
  +-- task-aware ranking and graph signals
  +-- optional chonkify extraction
  +-- bounded Markdown/JSON pack
  +-- estimated pack-avoidance receipt

Optional semantic cache
  |
  +-- separate local profile and namespace
  +-- exact/similar request policy
  +-- hit/miss/invalidated receipt
```

Headroom owns live request transformation, provider compatibility, cache
interaction, fail-open behavior, and request-level measurements. Switchboard
owns installation, profile state, consent, Doctor, rollback, UI disclosure,
and release evidence. Repo Intelligence owns read-only repository scanning and
context-pack generation.

## 4. Shared contract

Every engine must expose an equivalent capability and receipt contract. The
exact implementation may live in Python, Rust, or a managed sidecar, but the
Switchboard read model should normalize it to:

```ts
type OptimizationEngineId =
  | "headroom-native"
  | "rtk"
  | "leanctx"
  | "llmlingua-2"
  | "chonkify"
  | "semantic-cache"
  | "pxpipe-text-image";

type OptimizationEvidence = "measured" | "estimated" | "external" | "none";

interface OptimizationReceipt {
  id: string;
  engine: OptimizationEngineId;
  scope: "live-request" | "repo-pack" | "cache-hit" | "shadow";
  beforeTokens?: number;
  afterTokens?: number;
  savedTokens?: number;
  savedUsd?: number;
  latencyMs?: number;
  fallbackReason?: string;
  protectedBytes?: number;
  evidence: OptimizationEvidence;
  provider?: string;
  model?: string;
  createdAt: string;
}
```

Required engine capability fields:

- local or remote processing boundary
- prompt/output visibility
- supported providers and clients
- lossiness and exact-recall caveat
- model/runtime requirements
- enable/disable state
- fallback behavior
- Doctor checks
- rollback and Off-mode cleanup
- evidence source and measurement method

## 5. Phase 0 — Baseline and compatibility gate

Before adding an engine, capture a reproducible baseline for the current
Headroom + RTK path.

### Work

- Freeze the current Headroom wheel and RTK versions in the evidence fixture.
- Record representative Claude Code, Codex, and Repo Intelligence workloads.
- Capture original input tokens, provider-billed input/cache/output tokens,
  latency, request size, compression refusal, and task outcome.
- Split results by tool result, old history, user prose, code, JSON, logs, and
  repository packs.
- Add an explicit `engine: headroom-native` source to the savings report when
  current data does not already provide it.
- Define a byte-sensitive fixture containing paths, UUIDs, hashes, secrets,
  line numbers, JSON values, and tool arguments.

### Gate

No new engine is enabled until the baseline can answer:

- what was sent before and after optimization;
- which source produced the saving;
- whether the provider cache was preserved or busted;
- whether the agent task still succeeded; and
- whether an unsafe block was left untouched.

## 6. Phase 1 — Pluggable text-compression engine

Priority: highest.

Start with `leanctx` as an opt-in local HTTP/Python sidecar when its current
release is compatible with the managed Python runtime. Keep raw LLMLingua-2 as
a research/reference engine, not as a direct Rust/Tauri gateway dependency.

### Scope

Initial live-request eligibility:

- large, stale tool results;
- old closed-history prose and logs;
- retrieved documents that are not byte-sensitive;
- content above a configurable token threshold.

Keep native text:

- system prompt and tool definitions;
- current user turn;
- newest turns and open tool calls;
- tool arguments and structured protocol state;
- code/JSON/identifier blocks unless the engine explicitly classifies them as
  safe and a verbatim fixture passes.

### Runtime design

- Add a Headroom compressor interface rather than a new proxy. Until that
  upstream seam exists, keep leanctx outside the live Headroom path and use
  shadow/benchmark traffic only.
- Install the optional model/runtime only after explicit user enablement.
- Use a managed, versioned environment under Switchboard storage.
- Keep model downloads local and show disk, memory, and latency requirements.
- Use a bounded worker/sidecar so compression cannot block proxy startup.
- Fail open to the original request on timeout, model failure, unsupported
  provider, malformed message, or negative profitability estimate.
- Keep provider prompt caching behavior visible in the receipt.

### Switchboard surfaces

Add a `Text compression` profile with:

- states: Off, Available, Installing, Shadow, Enabled, Needs repair;
- engine selector: leanctx, LLMLingua-2;
- threshold and protected-content policy summary;
- model/runtime readiness;
- last measured savings and fallback reason;
- explicit lossiness warning;
- Disable, Repair, and Bypass Headroom actions.

### Acceptance criteria

- Disabled mode is byte-identical to current Headroom behavior.
- Shadow mode measures the transformed request but forwards the original.
- Compression never changes tool-call pairing or open-call state.
- Protected fixtures remain byte-exact.
- Sparse prose and small blocks are passed through.
- A failure never blocks the provider request.
- Before/after token counts and fallback reasons are persisted without raw
  prompt logging.
- A live coding-agent fixture passes with no regression in task completion.

## 7. Phase 2 — chonkify for repository/context packs

Priority: second.

The intended project is [`thom-heinrich/chonkify`](https://github.com/thom-heinrich/chonkify).
It is distinct from the similarly named [`feyninc/chonkie`](https://github.com/feyninc/chonkie)
RAG chunking library. chonkify should initially be used only in the read-only
Repo Intelligence path, not in the live Headroom proxy. This makes the output
inspectable and avoids silently altering a running agent conversation.

### Scope

Before any bundling or redistribution, confirm chonkify's license. GitHub
currently reports `NOASSERTION`, so the adapter must remain non-shipped or
user-installed until license evidence is complete.

Add an optional pack stage:

```text
repo scan -> graph/task ranking -> candidate pack -> chonkify -> bounded pack
```

Use it for:

- implementation packs;
- verification/test packs;
- handoff packs;
- dense engineering notes and quantitative logs.

Do not use it for:

- secrets or secret-like paths;
- generated artifacts that require exact reproduction;
- patches, hashes, lockfiles, or byte-sensitive fixtures;
- files whose omission would make a requested edit unsafe.

### Output contract

Each generated pack must retain:

- source file and line/range references;
- included and omitted file counts;
- original and resulting token estimates;
- extraction method and version;
- skipped-file reasons;
- safety flags;
- a recoverable “open source file” path for every excerpt.

Record pack reduction as `estimated`, not provider-billed `measured`, unless a
matched provider request proves the before/after token count.

### Acceptance criteria

- Existing Repo Intelligence ranking remains the source of file selection.
- Chonkify cannot read excluded secret paths.
- The adapter must emit repository-relative source spans, content hashes,
  compressor/version/configuration, and deterministic output hashes.
- A pack can be regenerated deterministically from the same index and task.
- Every excerpt points back to a source file and line context.
- `--no-compression` reproduces the current pack path.
- UI and MCP responses label chonkify output and estimated savings explicitly.

## 8. Phase 3 — Semantic caching

Priority: third. This is a separate optimization category, not a compressor.

### Initial implementation

Use the Switchboard-owned local cache service as the first managed topology.
Start with exact caching for deterministic, completed text requests; add
semantic similarity only behind an explicit opt-in after the local lifecycle
and false-hit gates are proven. LiteLLM remains a research/reference option,
not a second runtime proxy in the shipped path.

The first supported topology should be one documented chain, for example:

```text
client -> Headroom -> local semantic cache -> provider
```

Do not support arbitrary proxy chains in the first managed release.

### Safety policy

- Cache namespaces must include provider, model, account/workspace, and task
  sensitivity policy.
- Disable caching for rapidly changing repositories, secrets, authentication,
  tool calls, live state, and requests with explicit no-cache markers.
- Never return a semantically similar response for an open tool call.
- Bypass semantic replay for streaming, MCP/tool requests, high-temperature
  requests, sensitive data, and rapidly changing repository state.
- Prefer exact or strongly scoped cache keys before semantic matching.
- Make TTL, invalidation, and cache-hit reason visible.
- Disclose that eligible response bodies are stored locally until TTL/clear;
  prompt text itself is not stored as a cache key.

### Evidence

Record:

- cache hit, miss, bypass, and invalidation counts;
- avoided upstream request count;
- avoided provider tokens only when the request would otherwise have been
  sent and a matched counterfactual exists;
- false-hit/rollback reports;
- cache backend and namespace.

Label cache savings as `estimated` until provider-billed counterfactual evidence
exists. UI copy must say that a hit reuses a previous response; it must not say
that the prompt was compressed.

## 9. Phase 4 — pxpipe visual compression

Priority: fourth and experimental.

The existing [pxpipe Headroom plan](pxpipe-headroom-integration-plan.md) is the
source of truth for the text-to-image design. The implementation must land in
Headroom first or use a local Headroom source checkout; Switchboard must not
run pxpipe as a second proxy.

### Required upstream feature

Headroom must expose an opt-in `text_image` compressor with:

- Off, shadow, and enabled modes;
- exact model allowlist;
- image-token profitability gate;
- recent/open/system/tool protection;
- adjacent protected factsheet;
- fail-open fallback;
- visual savings and exact-recall metrics.

Initial provider scope is Anthropic Messages only. OpenAI Responses history
rewriting waits for equivalent tool-pair and token-measurement coverage.

### Switchboard release treatment

- Experimental profile only; never part of Full optimization initially.
- Clear lossiness and exact-string warning before enablement.
- Shadow mode before any image blocks are forwarded.
- Dedicated kill switch and Headroom bypass.
- Separate visual savings row in Token X-Ray and the savings ledger.
- Beta cohort and fixture-based task-quality gate before wider release.

## 10. Phase 5 — shared UI, Doctor, and rollback

Once the first engine exists, unify management without conflating behavior.

### Doctor checks

For every engine:

- installed/version/capability state;
- active mode and owning process;
- provider/model compatibility;
- local/remote boundary;
- prompt/output visibility;
- protected-content policy;
- last successful transformation;
- fallback/error count;
- measured or estimated evidence state;
- Off-mode cleanup status.

### Rollback

- Stop sidecars and workers before removing their managed files.
- Restore only Switchboard-owned config blocks.
- Preserve user-owned Headroom/RTK settings.
- Reset the active engine to passthrough before cleanup.
- Keep a receipt of the rollback result.
- Ensure `Off` means no hidden transformer, cache, model worker, or routing
  hook remains active.

## 11. Phase 6 — benchmark and beta gates

### Required benchmark suites

1. **Token/cost:** before/after provider token and cost measurement.
2. **Exact recall:** paths, hashes, IDs, numbers, JSON, and tool arguments.
3. **Semantic recall:** decisions, negations, names, values, and state changes.
4. **Agent task:** representative coding tasks with tests and edits.
5. **Latency:** cold start, warm request, timeout, and fallback.
6. **Cache safety:** exact hit, semantic hit, invalidation, stale state, and
   open-tool bypass.
7. **Privacy:** no raw prompt export in default receipts or telemetry.

### Promotion gates

- `leanctx/LLMLingua-2`: experimental live profile only after protected-content
  and fail-open tests pass.
- `chonkify`: Repo Intelligence opt-in after deterministic pack and source
  traceability tests pass.
- semantic cache: guided until lifecycle, namespace, invalidation, and false-hit
  evidence pass.
- pxpipe: shadow mode until exact-recall and model-quality evidence passes;
  enabled mode remains opt-in even after that.
- tiny universal proxies: no product integration without maintained tests,
  pinned releases, security review, license review, and reproducible benchmarks.

## 12. Delivery order

### Milestone A — contract and baseline

- Add normalized optimization receipt/capability types.
- Add baseline fixtures and source labels.
- Add governance checks for local/remote, visibility, lossiness, evidence, and
  lifecycle metadata.

### Milestone B — text engine

- Validate leanctx against the managed Python runtime.
- Implement Headroom compressor seam or upstream contribution.
- Add shadow mode, protected spans, fail-open behavior, and measurements.
- Add Switchboard profile, Doctor, rollback, and tests.

### Milestone C — repository packs

- Add chonkify as a Repo Intelligence pack stage.
- Add deterministic metadata and source references.
- Add UI/MCP disclosure and estimated-savings receipts.

### Milestone D — semantic cache

- Harden guided local profile and evidence.
- Implement one managed topology only after lifecycle tests pass.
- Add namespace, invalidation, no-cache, and false-hit controls.

### Milestone E — visual compression

- Implement or consume upstream Headroom `text_image` support.
- Add shadow mode and model capability receipt.
- Run visual quality and exact-recall beta suite.
- Keep enabled mode experimental and independently disableable.

### Milestone F — release readiness

- Run full benchmark matrix.
- Publish measured vs estimated savings separately.
- Verify clean Off mode and direct bypass.
- Confirm no second proxy, remote destination, repo secret, or unowned config
  mutation was introduced.

## 13. Definition of done

An optimization engine is shippable only when it has:

- a clear scope and owner;
- local/remote and prompt/output visibility disclosure;
- explicit lossiness and exact-recall caveats;
- versioned capability detection;
- disabled, shadow, and enabled behavior where relevant;
- protected-content rules;
- fail-open behavior;
- measured or honestly labelled estimated evidence;
- Doctor verification;
- rollback and Off cleanup;
- focused unit, integration, and fixture tests;
- no raw prompt logging by default;
- a documented manual bypass;
- release evidence that distinguishes shipped behavior from planned behavior.

## Current status

Headroom and RTK remain the only default live compression path. The managed
leanctx path is shadow-only and requires explicit local configuration. The
separate semantic cache is implemented but opt-in, exact-only, and never part
of the Full optimization default. The pxpipe design is documented in
[pxpipe-headroom-integration-plan.md](pxpipe-headroom-integration-plan.md).
Raw LLMLingua-2 and chonkify remain blocked for live/shipped transformation.
