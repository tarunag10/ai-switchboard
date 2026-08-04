# Comprehensive Token Compression — Implementation Plan

**Status:** active  
**Created:** 2026-08-04  
**Design reference:** [COMPREHENSIVE-TOKEN-COMPRESSION-DESIGN.md](./COMPREHENSIVE-TOKEN-COMPRESSION-DESIGN.md)  
**Program ledger:** [slice-status.json](./slice-status.json)  
**Parent program:** [IMPLEMENTATION-PLAN.md](./IMPLEMENTATION-PLAN.md) (P0–P3 trust and coverage)

---

## 1. Program goals

1. Ship a **coherent compression product** — users can see, enable, measure, and roll back every token-saving layer from one control plane.
2. **Preserve architecture** — Headroom remains the sole managed live-request routing owner; extensions plug in as engines, workflows, or guided gateways.
3. **Honest attribution** — measured, estimated, inferred, and cache-hit savings stay separate in UI, exports, and release evidence.
4. **Evidence-gated promotion** — experimental engines (leanctx live, chonkify, pxpipe, LLMLingua-2) do not ship as default until fixtures pass.
5. **Universal workflows** — compression benefits reach BYOK and sidecar-only agents (RTK, Repo Intelligence, MCP) even when Headroom does not own provider routing.

## 2. Non-goals

- Replacing Headroom with a chain of independent localhost proxies.
- Rewriting `proxy_intercept.rs`, `state.rs`, or Tauri bootstrap as a greenfield compression gateway.
- Bundling large ML models or remote cache backends during normal app startup.
- Claiming LiteLLM, Cloudflare, or Kong savings as Switchboard-measured without external evidence labels.
- Fabricating provider-billed counterfactuals when usage APIs are incomplete.

## 3. Phase overview

| Phase | Theme | Horizon | Exit gate |
| --- | --- | --- | --- |
| **C0** | Compression product shell | 2–3 weeks | Unified dashboard + master activation + Doctor playbook |
| **C1** | Headroom depth | 3–5 weeks | Presets + upstream profiles + content-class stats |
| **C2** | Context and shell layers | 4–6 weeks | Chonkify + session budget + RTK presets + MCP budget |
| **C3** | Cache and measurement | 3–5 weeks | Cache inspector + semantic v2 opt-in + benchmark expansion |
| **C4** | Engine promotion | 6–10 weeks | leanctx live + pxpipe (if gates pass) |
| **C5** | Coverage and BYOK | 4–8 weeks | BYOK dossier + connector promotion + cc-switch toggle |

Phases may overlap. C0 and C1 are prerequisites for marketing a “comprehensive compression app.” C4 depends on C3 benchmark and quality gates.

---

## C0 — Compression product shell

**Goal:** Users experience Switchboard as a compression application even before new engines ship.

### C0.1 Unified compression dashboard

**Design ID:** E1

**Tasks:**

1. Add `src/lib/compressionDashboard.ts` — normalizes Headroom `/stats`, RTK daily stats, semantic cache counters, Repo Intelligence estimates, and add-on inferred rows into one read model with `confidence: measured | estimated | inferred | external`.
2. Extend `OptimizationDashboard.tsx` with a **Compression overview** section: today/session totals, per-source breakdown, and caveats copy from savings ledger rules.
3. Link rows to existing drill-downs (Token X-Ray, Addons engine cards, Repo Intelligence view).
4. Add `compressionDashboard.test.ts` with fixture payloads for empty, partial, and full attribution.

**Files:**

- `src/lib/compressionDashboard.ts` (new)
- `src/lib/compressionDashboard.test.ts` (new)
- `src/components/OptimizationDashboard.tsx`
- `src/styles.css` (overview panel tokens only)

**Acceptance:**

- Dashboard shows at least four source families when data exists: Headroom, RTK, cache, Repo Intelligence.
- No source without data is shown as zero measured savings.
- Unit tests cover confidence labeling.

**Dependencies:** P1.4 provider-billed counterfactual (done) for optional measured row.

---

### C0.2 Max compression master activation

**Design ID:** F1

**Tasks:**

1. Add `src/lib/maxCompressionActivation.ts` — allowlist: `headroom-native`, `semantic-cache`, `rtk`, repo index prompt; explicitly excludes `leanctx`, `llmlingua-2`, `chonkify`, `pxpipe-text-image` until `canActivateOptimizationEngine` passes.
2. Extend `MasterActivationCard.tsx` with **Max compression** preset: enables Full optimization, exact cache (if recommended), RTK install/enable, and opens Repo Intelligence index prompt.
3. Record lifecycle receipts per engine via `createOptimizationLifecycleReceipt`.
4. Copy must state what is *not* enabled (experimental engines).

**Files:**

- `src/lib/maxCompressionActivation.ts` (new)
- `src/lib/maxCompressionActivation.test.ts` (new)
- `src/components/MasterActivationCard.tsx`

**Acceptance:**

- Activation never enables experimental/blocked engines.
- Doctor re-run suggested after activation completes.
- Tests assert allowlist matches `filterActivatableOptimizationEngineIds`.

---

### C0.3 Doctor compression repair playbook

**Design ID:** F4

**Tasks:**

1. Add ordered repair stages in `src/lib/doctorCompressionPlaybook.ts`: `runtime` → `routing` → `rtk` → `cache` → `repo-index` → `mcp`.
2. Map each stage to existing Doctor issue ids and repair actions in `doctorRepairCopy.ts`.
3. Surface playbook summary in Doctor when any compression-related issue is open.
4. Link from Mode Inspector verdict when P0.2 aggregate verdict ≠ `aligned`.

**Files:**

- `src/lib/doctorCompressionPlaybook.ts` (new)
- `src/lib/doctorCompressionPlaybook.test.ts` (new)
- `src/lib/doctorRepairCopy.ts`
- `src/components/DoctorPanel.tsx` (or equivalent Doctor surface)

**Acceptance:**

- Playbook order is stable in tests.
- No new repair actions that bypass existing consent/backup gates.

---

### C0.4 Start Agent Session compression checklist

**Design ID:** F2

**Tasks:**

1. Add checklist model to `AgentSessionPanel.tsx`: index freshness, pack token estimate vs budget, switchboard mode, exact cache eligibility, Repo Memory MCP health.
2. Reuse `resolveAgentSessionPreferredPackId` and `get_index_freshness` backend contract where available.
3. Block or warn on copy when checklist has `blocked` items; allow override with explicit acknowledgment.

**Files:**

- `src/components/AgentSessionPanel.tsx`
- `src/lib/agentSessionPacks.ts`
- `src/lib/agentSessionCompressionChecklist.ts` (new)

**Acceptance:**

- Checklist renders for all supported `--list-agents` targets.
- Stale index shows Doctor link, not silent pass.

**C0 exit gate:** `npm run build` passes; new lib tests pass; Optimization dashboard shows unified overview.

---

## C1 — Headroom depth

**Goal:** Expose and manage Headroom compression levers without forking the proxy.

### C1.1 Compression profile presets

**Design ID:** A3

**Tasks:**

1. Define presets in `src-tauri/src/tool_manager/compression_profiles.rs`: `balanced`, `aggressive`, `codex-heavy`, `claude-cache-safe` mapping to `HEADROOM_MODE`, `HEADROOM_COMPRESS_USER_MESSAGES`, `HEADROOM_OUTPUT_SHAPER`, `HEADROOM_VERBOSITY_LEVEL`, and `apply_savings_mode_env` fields.
2. Persist selected preset in `config/compression-profile.json` under app storage.
3. Apply preset env in `headroom_runtime.rs` on proxy spawn; restart proxy on change with user consent.
4. UI: Optimization Engine Profiles card — preset picker with factual description of tradeoffs (latency vs savings vs cache risk).
5. Include preset id in savings attribution metadata when available.

**Files:**

- `src-tauri/src/tool_manager/compression_profiles.rs` (new)
- `src-tauri/src/tool_manager/headroom_runtime.rs`
- `src-tauri/src/lib.rs` (commands: `get_compression_profile`, `set_compression_profile`)
- `src/components/OptimizationEngineProfilesCard.tsx`
- `src/lib/types.ts`

**Acceptance:**

- Changing preset restarts Headroom and is reversible via Rollback/off defaults.
- Default preset matches current shipped env (no behavior change on upgrade until user selects).
- Rust unit tests for preset → env mapping.

---

### C1.2 Per-provider upstream profiles

**Design ID:** A2, G1

**Tasks:**

1. Add `src-tauri/src/provider_upstream_profiles.rs` — validated overrides for `OPENAI_TARGET_API_URL`, `ANTHROPIC_TARGET_API_URL`; refuse non-HTTPS except loopback; no secrets stored.
2. Settings → **Provider upstream** card: enable override, URL field, test connection (harmless `GET` or provider-specific health), Doctor row.
3. Inject overrides into Headroom spawn env in `headroom_runtime.rs`.
4. Document BYOK pattern (DeepSeek, Azure, Together) in guided copy; link from `plannedConnectors.ts` dossier stub for `byok_openai_compatible`.
5. Rollback: clear overrides restores OpenAI/Anthropic defaults.

**Files:**

- `src-tauri/src/provider_upstream_profiles.rs` (new)
- `src-tauri/src/tool_manager/headroom_runtime.rs`
- `src/components/SettingsProviderUpstreamCard.tsx` (new)
- `src/lib/plannedConnectors.ts` (BYOK dossier)
- `docs/connectors.md` (BYOK subsection)

**Acceptance:**

- Overrides never written to git-managed files.
- Doctor shows misconfigured URL before user routes production traffic.
- Fixture test: spawn env includes override when configured.

---

### C1.3 Content-class compression breakdown

**Design ID:** A4, E1

**Tasks:**

1. Parse Headroom `/stats` and `/stats-history` for tool-result vs history vs user-message buckets where exposed.
2. Add Token X-Ray metrics: `compressionToolResultTokens`, `compressionHistoryTokens`, `compressionUserMessageTokens` (names aligned with upstream schema).
3. Show breakdown in Token X-Ray and compression dashboard.

**Files:**

- `src-tauri/src/token_xray.rs`
- `src/lib/usageAnalytics.ts`
- `src/components/TokenXrayView.tsx`
- `src/lib/compressionDashboard.ts`

**Acceptance:**

- Missing upstream fields render as “unavailable,” not zero.
- Rust tests with fixture `/stats` JSON.

---

### C1.4 Tool-result vs history UI toggles

**Design ID:** A4

**Tasks:**

1. Map UI toggles to existing Headroom env flags where supported upstream; block toggles unsupported by pinned Headroom version with Doctor explanation.
2. Toggles live under Optimization → Headroom Native advanced section; require explicit expand.

**Acceptance:**

- Off mode removes only Switchboard-owned preference file, not user Headroom install.

**C1 exit gate:** Presets + upstream profiles ship behind consent; `npm run check:world-class-plan` still passes.

---

## C2 — Context and shell layers

**Goal:** Reduce tokens before they reach Headroom and compress shell output aggressively.

### C2.1 Chonkify promotion for Repo Intelligence packs

**Design ID:** C1

**Tasks:**

1. Complete license/provenance evidence in `fixtures/chonkify-provenance-evidence.json`.
2. Add `scripts/check-chonkify-promotion-gate.mjs`.
3. Implement chonkify adapter invocation in `scripts/repo-intelligence.mjs` when `--compression chonkify` (or manifest flag).
4. Unblock `chonkify` in `optimizationEngines.ts` when gate passes.
5. UI: Repo Intelligence pack copy dialog shows native vs chonkify size estimate.

**Files:**

- `fixtures/chonkify-provenance-evidence.json` (new)
- `scripts/check-chonkify-promotion-gate.mjs` (new)
- `scripts/repo-intelligence.mjs`
- `src/lib/repoIntelligence.ts`
- `src/lib/optimizationEngines.ts`

**Acceptance:**

- Provenance preserved in pack metadata.
- Gate script fails when evidence missing.
- Wrong-omission benchmark ≤ plan threshold on fixtures.

---

### C2.2 Session budget enforcer

**Design ID:** C2

**Tasks:**

1. Add default budget field to Agent Session UI (token estimate ceiling).
2. `repo-intelligence.mjs --start-session` rejects or warns when selected pack exceeds budget; suggest smaller pack from `resolveAgentSessionPreferredPackId` ranking.
3. Persist last-used budget in local settings (non-secret).

**Files:**

- `src/components/AgentSessionPanel.tsx`
- `scripts/repo-intelligence.mjs`
- `src/lib/agentSessionPacks.ts`

**Acceptance:**

- Budget 0 means no limit (backward compatible).
- Tests for over-budget pack selection.

---

### C2.3 Repo Memory MCP budget parameters

**Design ID:** C3

**Tasks:**

1. Extend MCP tool schemas with optional `maxTokens` or `depth` on read-only tools.
2. Enforce bounds server-side in repo-memory MCP handler; refuse excessive requests with actionable error.
3. Document in `docs/repo-memory-mcp.md`.

**Acceptance:**

- No repo writes; read-only contract preserved.

---

### C2.4 RTK preset library

**Design ID:** D1

**Tasks:**

1. Expand `scripts/optimization-report.mjs --rtk-presets` with task presets: `test`, `build`, `grep`, `git-log`.
2. Addons card: copy preset env block for shell profile.
3. Attribute preset id in RTK receipts when detectable.

**Files:**

- `scripts/optimization-report.mjs`
- `scripts/rtk-presets.node-test.mjs`
- `src/components/AddonsView.tsx`

**Acceptance:**

- `npm run rtk:presets:check` passes.

---

### C2.5 Codex parallel-session guard

**Design ID:** D3

**Tasks:**

1. Detect multiple active Codex goals (existing codex thread signals).
2. Show non-blocking banner: suggest RTK-only mode with link to troubleshooting doc.
3. Do not auto-change mode without user consent.

**Acceptance:**

- Banner suppressed in Off mode.

**C2 exit gate:** Chonkify gate script in CI; session budget in Agent Session; RTK presets documented.

---

## C3 — Cache and measurement

**Goal:** Mature caching and prove savings scientifically.

### C3.1 Exact cache Doctor integration

**Design ID:** B1

**Tasks:**

1. Doctor issue when `recommendExactCacheDefault` is true but cache disabled.
2. One-click enable from Doctor (same path as Addons card).

**Files:**

- `src/lib/exactCacheDefaultPolicy.ts`
- `src-tauri/src/doctor.rs`

**Acceptance:**

- No recommendation in RTK-only or Off modes (existing tests extended).

---

### C3.2 Cache namespace inspector

**Design ID:** B4

**Tasks:**

1. Tauri commands: `get_semantic_cache_stats`, `clear_semantic_cache_namespace` (existing or extend).
2. UI table: provider, model, hits, misses, last hit time — no prompt bodies.
3. Export row in diagnostics obeys secret-free rules.

**Files:**

- `src-tauri/src/semantic_cache.rs`
- `src/components/OptimizationEngineProfilesCard.tsx`

**Acceptance:**

- Clear requires confirmation phrase.

---

### C3.3 Semantic similarity cache v2 (opt-in)

**Design ID:** B2

**Tasks:**

1. Extend `semanticCachePolicy.ts` with `semantic-v2` policy version and stricter bypass rules.
2. Implement similarity keying in `semantic_cache.rs` behind feature flag default off.
3. Label all hits `estimated` until counterfactual pair exists.
4. Add `check:semantic-cache-v2-gate.mjs` requiring embedding model consent and local-only boundary proof.

**Acceptance:**

- Default install: exact cache only.
- Semantic v2 cannot enable without explicit UI toggle + disclosure.

---

### C3.4 Benchmark and quality expansion

**Design ID:** E2, E5

**Tasks:**

1. Add fixtures: tool-heavy transcript, streaming bypass, cache-bust, Headroom refusal path.
2. Wire `npm run check:comprehensive-compression-plan` aggregating C-phase doc presence and benchmark minimums.
3. Export leaderboard fields for preset and engine id (`export:benchmark-leaderboard`).

**Files:**

- `benchmarks/fixtures.json`
- `scripts/check-comprehensive-compression-plan.mjs` (new)
- `package.json` scripts

**Acceptance:**

- ≥ 12 fixtures across ≥ 6 categories.
- Wrong-omission rate 0 on required fixtures.

---

### C3.5 Provider-billed scheduled sampling

**Design ID:** E4

**Tasks:**

1. Optional weekly counterfactual sample when user opts in; store in savings ledger.
2. Never run without explicit consent; local-only storage.

**Acceptance:**

- Ledger rows labeled `measured` only with complete pair (extends P1.4).

**C3 exit gate:** Cache inspector shipped; benchmark check in CI; semantic v2 remains off by default.

---

## C4 — Engine promotion

**Goal:** Promote third-party compressors only through Headroom seams.

### C4.1 leanctx live routing

**Design ID:** A1

**Tasks:**

1. Define Headroom-owned compressor seam contract (version gate on pinned `headroom-ai` wheel).
2. Flip `live_request_routing: true` in `leanctx.rs` only when `check:leanctx-promotion-gate` and new live fixtures pass.
3. Protected-content byte-identical tests for tool calls, JSON, paths, secrets.
4. Fail-open timeout tests.

**Acceptance:**

- Shadow mode remains available when live disabled.
- Master activation allowlist uses `leanctxPromotionGate.ts`.

---

### C4.2 PXPipe text/image

**Design ID:** A5

**Tasks:**

1. Add `fixtures/pxpipe-promotion-evidence.json` and `check:pxpipe-promotion-gate.mjs`.
2. Integrate via Headroom `text_image` capability when pinned version exposes seam.
3. Keep `pxpipe-text-image` experimental until visual quality checklist signed.

---

### C4.3 LLMLingua-2 blocked path

**Design ID:** A6

**Tasks:**

1. Document install size, memory, and quality baseline requirements.
2. Remain `activationMode: experimental` until C3 benchmarks pass with engine enabled.

**C4 exit gate:** At least one of leanctx live or pxpipe promoted with CI gates; LLMLingua-2 stays blocked or shadow-only.

---

## C5 — Coverage and external integration

**Goal:** Compression workflows for all major agents and optional external caches.

### C5.1 BYOK connector dossier and Doctor checks

**Design ID:** G1

**Tasks:**

1. Add `byok_openai_compatible` entry to `plannedConnectors.ts` with manual setup steps.
2. Doctor: verify loopback proxy healthy + upstream override configured + client base URL points at `6767`.

---

### C5.2 LiteLLM guided wizard enrichment

**Design ID:** B3

**Tasks:**

1. Extend `GatewayProfilesCard` for `litellm-local-cache` with step checklist and env template download.
2. Read-only preflight: loopback port responds (user-started).

---

### C5.3 cc-switch reconciler toggle

**Design ID:** G4

**Tasks:**

1. Advanced Settings: `HEADROOM_CC_SWITCH_RECONCILE` with disclosure linking to Headroom cc-switch reconciler docs.
2. Inject into Headroom spawn env; default off.

---

### C5.4 Connector native routing promotion

**Design ID:** G2, G3

**Tasks:**

1. Continue Aider/Continue promotion via `connectorPromotionGate.ts` when fixture-home lifecycle complete.
2. Cursor promotion remains blocked until `cursorNativeGate` passes.

**C5 exit gate:** BYOK dossier + Doctor checks shipped; LiteLLM wizard copy complete.

---

## 4. Cross-cutting requirements

### 4.1 Attribution rules (all phases)

| Source | Label | UI family |
| --- | --- | --- |
| Headroom `/stats` delta | measured | Compression |
| RTK gain stats | measured | Shell output |
| Exact/semantic cache hit | estimated or measured | Cache (never compression) |
| Repo Intelligence pack | estimated | Context avoidance |
| MarkItDown/Ponytail/Caveman | inferred | Add-on |
| LiteLLM/Cloudflare | external | Gateway |

### 4.2 Rollback and Off mode

Every C-phase feature must document:

- Switchboard-owned files and markers
- Backup expectation
- Off-mode cleanup behavior
- Rollback Center row id (if applicable)

### 4.3 Release evidence

Add to `npm run check:deployment` or `check:comprehensive-compression-plan`:

- Design doc and implementation plan present
- Slice ledger entries for C0–C5 (optional `comprehensive-compression` section in `slice-status.json`)
- Benchmark minimums for shipped phases

---

## 5. File map (new artifacts summary)

| Artifact | Phase |
| --- | --- |
| `src/lib/compressionDashboard.ts` | C0 |
| `src/lib/maxCompressionActivation.ts` | C0 |
| `src/lib/doctorCompressionPlaybook.ts` | C0 |
| `src/lib/agentSessionCompressionChecklist.ts` | C0 |
| `src-tauri/src/tool_manager/compression_profiles.rs` | C1 |
| `src-tauri/src/provider_upstream_profiles.rs` | C1 |
| `src/components/SettingsProviderUpstreamCard.tsx` | C1 |
| `fixtures/chonkify-provenance-evidence.json` | C2 |
| `scripts/check-chonkify-promotion-gate.mjs` | C2 |
| `scripts/check-comprehensive-compression-plan.mjs` | C3 |
| `fixtures/pxpipe-promotion-evidence.json` | C4 |

---

## 6. Test and CI commands

```bash
# Existing
npm run check:world-class-plan
npm run check:p1-savings-supremacy
npm run benchmarks
npm run check:world-class-benchmarks

# New (add in C3)
npm run check:comprehensive-compression-plan

# Per-phase
npm run check:chonkify-promotion-gate      # C2
npm run check:leanctx-promotion-gate       # C4
npm run check:pxpipe-promotion-gate        # C4
```

Frontend/Rust tests as listed per slice. Prefer focused tests:

```bash
npm run test -- compressionDashboard
cargo test --manifest-path src-tauri/Cargo.toml --lib compression_profiles
```

---

## 7. Suggested execution order

```text
C0.1 → C0.2 → C0.3 → C0.4     (product shell — parallelizable 0.3/0.4)
C1.1 → C1.2 → C1.3 → C1.4     (Headroom depth)
C2.2 → C2.4 → C2.5 → C2.1     (quick wins before chonkify gate)
C3.1 → C3.2 → C3.4 → C3.3     (cache + benchmarks before semantic v2)
C5.1 → C5.2 → C5.3            (coverage docs — parallel with C1)
C4.*                          (only after C3.4 benchmarks green)
```

---

## 8. Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Upstream Headroom env renames break presets | Pin wheel; preset module reads version; Doctor shows mismatch |
| BYOK misconfiguration sends keys to wrong host | HTTPS-only overrides; test connection; explicit disclosure |
| Chonkify omits critical context | Provenance + wrong-omission benchmarks; default off |
| Semantic v2 false cache hits | Strict bypass rules; opt-in only; estimated labeling |
| leanctx live breaks tool calls | Protected-content fixtures; fail-open; shadow fallback |

---

## 9. Success criteria (program complete)

1. **User can answer** “How much did compression save me today?” from one dashboard with confidence labels.
2. **User can enable** max safe compression in one guided flow without enabling experimental engines.
3. **User can route** BYOK OpenAI-compatible providers through Headroom with managed upstream profiles.
4. **User can compress** repo packs (chonkify) and shell output (RTK presets) with measured/estimated attribution.
5. **CI proves** benchmark and promotion gates for every enabled engine.
6. **Doctor repairs** compression stack in deterministic order.

---

## 10. Related documents

- [COMPREHENSIVE-TOKEN-COMPRESSION-DESIGN.md](./COMPREHENSIVE-TOKEN-COMPRESSION-DESIGN.md)
- [IMPLEMENTATION-PLAN.md](./IMPLEMENTATION-PLAN.md)
- [token-optimization-addons-implementation-plan.md](../token-optimization-addons-implementation-plan.md)
- [gateway-addons-implementation-plan.md](../gateway-addons-implementation-plan.md)
- [plan-status-ledger.md](../plan-status-ledger.md)
