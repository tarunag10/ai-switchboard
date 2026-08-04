# Comprehensive AI Token Compression Application — Design Recommendations

**Status:** design reference  
**Created:** 2026-08-04  
**Program:** world-class-token-savings  
**Implementation plan:** [COMPREHENSIVE-TOKEN-COMPRESSION-IMPLEMENTATION-PLAN.md](./COMPREHENSIVE-TOKEN-COMPRESSION-IMPLEMENTATION-PLAN.md)  
**Related:** [IMPLEMENTATION-PLAN.md](./IMPLEMENTATION-PLAN.md), [token-optimization-addons-implementation-plan.md](../token-optimization-addons-implementation-plan.md)

---

## 1. Purpose

This document defines how AI Switchboard evolves from a Headroom-enabled control plane into a **comprehensive, local-first AI token compression application** without replacing the existing architecture.

Headroom remains the **sole managed provider-routing owner** on `127.0.0.1:6767`. Every recommendation below extends, complements, or measures compression — it does not introduce a competing default proxy chain.

---

## 2. Design principles

1. **One routing owner** — Headroom on loopback; no default `proxy → proxy → provider` stacks.
2. **Separate attribution** — compression, cache hits, context avoidance, and inferred add-on savings stay distinct in UI and exports.
3. **Reversible by default** — markers, backups, Off cleanup, and Rollback Center apply to every new surface.
4. **Fail-open** — compression never blocks coding; bypass on refusal, timeout, or oversize.
5. **Local-first** — remote gateways (Cloudflare, Kong, hosted observability) remain guided/manual with disclosure.
6. **Evidence before promotion** — experimental engines stay shadow or blocked until fixtures and gates pass.

---

## 3. Compression stack overview

```text
Before the API call          At the API call              Shell / environment
───────────────────          ───────────────              ───────────────────
Repo Intelligence            Headroom proxy               RTK
Repo Memory MCP              + native compressors
MarkItDown / Ponytail        + Exact Replay Cache
Caveman handoffs             + optional leanctx / pxpipe
chonkify (repo packs)        (output shaper, verbosity)
Session budget enforcer
Agent Memory compaction
```

Each layer saves tokens differently. Savings compound in practice but must not compound in accounting.

---

## 4. Live request compression (Headroom layer)

Extensions inside the managed proxy path.

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| A1 | **Promote leanctx to live compressor** | Move leanctx from shadow-only to a Headroom-owned text engine for safe prose and logs after protected-content gates pass. | `optimizationEngines.ts`, `leanctx.rs`, `check:leanctx-promotion-gate`; enable `live_request_routing` only after fixtures pass. |
| A2 | **Per-provider upstream profile UI** | Let users set `OPENAI_TARGET_API_URL` and `ANTHROPIC_TARGET_API_URL` (DeepSeek, Azure, etc.) without hacking Headroom spawn env. | Settings panel; inject in `headroom_runtime.rs`; Doctor verifies reachability; Rollback restores defaults. |
| A3 | **Compression profile presets** | Named profiles (`balanced`, `aggressive`, `codex-heavy`, `claude-cache-safe`) mapping to `HEADROOM_*` env. | `proxy_runtime.rs` `apply_savings_mode_env`; Optimization Dashboard + Mode Inspector; receipts tag profile id. |
| A4 | **Tool-result vs history toggles** | Separate controls for old turns, fresh tool output, and user messages. | Surface spawn flags in Add-ons; Token X-Ray breaks down savings by content class. |
| A5 | **PXPipe text/image promotion** | Visual/screenshot compression via Headroom `text_image` seam when upstream and quality gates pass. | `pxpipe-text-image` engine registry; shadow → enabled path mirrors leanctx. |
| A6 | **LLMLingua-2 research slot** | Optional local model for maximum text reduction on non-tool prose; blocked until quality baselines exist. | `optimizationEngines.ts`; install consent, disk check, benchmark gate before `activationMode: supported`. |

---

## 5. Caching (separate from compression)

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| B1 | **Exact Replay Cache auto-enable policy** | Recommend and one-click enable when mode is Headroom/full and proxy is healthy. | `switchboardModeForCache.ts`, `OptimizationEngineProfilesCard`; Doctor row when eligible but disabled. |
| B2 | **Semantic similarity cache (opt-in v2)** | Embedding-based near-duplicate replay for low-temperature, non-tool requests. | `semantic_cache.rs`, `semanticCachePolicy.ts`; explicit opt-in; savings labeled `cache-hit`. |
| B3 | **LiteLLM guided connector wizard** | Step-by-step local LiteLLM + cache backend for BYOK/multi-provider users. | `litellm-local-cache` gateway profile; Doctor health preflight; copyable env template. |
| B4 | **Cache namespace inspector** | View hit/miss, TTL, clear per namespace without exposing prompt text. | Addons card + semantic cache stats; secret-free export policy. |

---

## 6. Pre-request context optimization

Reduces tokens **entering** Headroom.

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| C1 | **Chonkify for Repo Intelligence packs** | Compress generated context packs offline with provenance preserved. | Unblock `chonkify` engine; `repoIntelligence.ts` pack path; estimated savings in manifest. |
| C2 | **Session budget enforcer** | Default `--start-session --budget N` with UI slider; warn before copy when over budget. | Agent Session panel; `agentSessionPacks.ts`; `repo:intelligence --start-session`. |
| C3 | **Repo Memory MCP compression-aware queries** | MCP tools accept budget param (symbol-only, dependents depth limit). | Read-only MCP contract; Doctor supervision (P2.3). |
| C4 | **Agent Memory structural compaction** | Structural summaries for long session handoffs before they become prompt history. | Agent Memory slice; `estimated` attribution; separate from live Headroom path. |
| C5 | **MarkItDown pre-ingest pipeline** | Offer conversion when agent attaches PDF/Office; show before/after token estimate. | MarkItDown add-on; Start Agent Session workflow; inferred attribution. |

---

## 7. Shell and environment compression (RTK layer)

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| D1 | **RTK preset library per task** | Curated filters for `test`, `build`, `grep`, `git log`. | `npm run rtk:presets`; Add-ons surface; measured RTK attribution. |
| D2 | **Per-connector RTK-only shortcut** | One-click bypass Headroom for Aider/Continue/BYOK workflows. | `plannedConnectors.ts`; Settings quick action; Doctor copy. |
| D3 | **Codex parallel-session guard** | Proactive RTK-only suggestion when multiple heavy Codex goals are active. | Extend 413 preflight; mode suggestion banner. |

---

## 8. Measurement, attribution, and proof

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| E1 | **Unified compression dashboard** | Single view: Headroom `/stats`, RTK, cache, Repo Intelligence, add-ons — with confidence labels. | `OptimizationDashboard`, Token X-Ray, savings ledger. |
| E2 | **Golden benchmark CI gate** | Expand fixtures: tool-heavy, streaming, cache-bust, refusal paths. | `benchmarks/fixtures.json`, `check:p1-savings-supremacy`, `export:benchmark-leaderboard`. |
| E3 | **Per-engine before/after receipts** | Every optimization appends `OptimizationReceipt` with correct scope. | `optimizationEngines.ts`; UI export; daily briefing. |
| E4 | **Provider-billed A/B mode** | Counterfactual on vs off using real billing endpoints where available. | `provider_billed_counterfactual.rs`; scheduled sampling. |
| E5 | **Compression quality scorecard** | Wrong-omission rate and fact retention on local fixtures. | `docs/benchmarks.md`; optional LLM-judge off by default. |

---

## 9. User workflows

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| F1 | **Max compression master activation** | One consent flow: Headroom profile + exact cache + RTK + repo index + session budget. | `MasterActivationCard`; `leanctxPromotionGate` allowlist pattern. |
| F2 | **Start Agent Session compression checklist** | Pre-flight: index freshness, pack size, mode alignment, cache eligibility, MCP health. | Agent Session panel; `repo:intelligence --session`. |
| F3 | **Daily compression briefing** | Yesterday's measured vs estimated savings; top leak categories. | `live-token-xray-daily-briefing-implementation-plan.md`; content-free. |
| F4 | **Doctor compression repair playbook** | Ordered repairs: runtime → routing → RTK → cache → index → MCP. | `doctorRepairCopy.ts`; Mode Inspector verdict (P0.2). |
| F5 | **Off / compare mode** | Disable all compression layers while app runs for baseline measurement. | Off mode + per-engine toggles in Add-ons. |

---

## 10. Connector and provider coverage

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| G1 | **BYOK OpenAI-compatible profile** | Client → `127.0.0.1:6767/v1` + upstream override for DeepSeek, Together, Groq, etc. | Guided dossier in `plannedConnectors.ts`; Doctor checks proxy + upstream. |
| G2 | **Promote Aider/Continue native routing** | Managed base-URL writes when schema is proven. | `connectorPromotionGate.ts`; P2.4 pattern. |
| G3 | **Cursor native write promotion** | Managed routing when Cursor publishes stable on-disk schema. | `cursorNativeGate.ts`; blocked until schema exists. |
| G4 | **cc-switch + Headroom reconciler toggle** | Opt-in `HEADROOM_CC_SWITCH_RECONCILE` for multi-provider Claude users. | Advanced Settings → Headroom spawn env; gateway governance disclosure. |

---

## 11. Safety, trust, and governance

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| H1 | **Protected-content classifier dashboard** | Show what Headroom refused to compress (redacted). | Headroom `/stats` + intercept logs; `docs/threat-model.md`. |
| H2 | **Fail-open latency budget** | Auto-bypass when compression exceeds N ms; log `fallbackReason` on receipt. | Settings threshold; Token X-Ray. |
| H3 | **Mode Inspector compression verdict** | Aggregate: proxy up, engines active, cache aligned, routing matches mode. | P0.2; compression-specific rows. |
| H4 | **Secret-safe config previews** | Redacted config before every engine apply. | `previewOptimizationEngineConfig`; gateway profiles pattern. |

---

## 12. Context-shaping add-ons (indirect compression)

| ID | Recommendation | Description | Integration |
| --- | --- | --- | --- |
| I1 | **Ponytail measured savings counter** | Upgrade from inferred to measured lines/files avoided. | `plannedAddons.ts`; savings ledger source. |
| I2 | **Caveman profile picker in session start** | Terse handoff profile per task type. | Caveman add-on; Agent Session. |
| I3 | **Ponytail + RTK joint preset** | Small diff + compressed test output for verification tasks. | Master Activation workflow preset. |

---

## 13. Architecture guardrails (do not violate)

- Do not replace Headroom with independent localhost proxies as the default path.
- Do not claim cache hits as compression savings.
- Do not promote leanctx, chonkify, or pxpipe without passing evidence gates.
- Do not rewrite `proxy_intercept.rs` routing ownership without preserving loopback bind and bypass contracts.
- Do not store provider secrets in repo files or Switchboard receipts.

---

## 14. Highest-impact near-term items

If prioritizing a “comprehensive compression app” feel within the current architecture:

1. **E1** — Unified compression dashboard  
2. **A3** — Compression profile presets  
3. **A2** — Upstream provider profiles (unlocks BYOK / DeepSeek-through-Headroom)  
4. **F1** — Max compression master activation  
5. **C1** — Chonkify on repo packs  

---

## 15. Related documents

| Document | Role |
| --- | --- |
| [COMPREHENSIVE-TOKEN-COMPRESSION-IMPLEMENTATION-PLAN.md](./COMPREHENSIVE-TOKEN-COMPRESSION-IMPLEMENTATION-PLAN.md) | Execution slices, files, gates |
| [IMPLEMENTATION-PLAN.md](./IMPLEMENTATION-PLAN.md) | P0–P3 trust and coverage program |
| [token-optimization-addons-implementation-plan.md](../token-optimization-addons-implementation-plan.md) | Engine contracts and add-on phases |
| [gateway-addons-implementation-plan.md](../gateway-addons-implementation-plan.md) | Guided gateway profiles |
| [architecture.md](../architecture.md) | V1 boundaries and connector model |
