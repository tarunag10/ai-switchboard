# World-Class Token Savings — Implementation Plan

**Status:** active  
**Created:** 2026-07-30  
**Source analysis:** [CURSOR-ANALYSIS-2026-07-30.md](./CURSOR-ANALYSIS-2026-07-30.md)  
**Progress ledger:** [slice-status.json](./slice-status.json)

---

## 1. Program goals

1. **Maximum real tokens saved per task** with quality preserved — not the highest marketing percentage.
2. **Provable trust** — reboot-level evidence, Mode Inspector that reflects reality, authenticated local proxy boundary.
3. **Honest attribution** — measured vs estimated vs inferred stays separate in UI, exports, and benchmarks.
4. **Universal agent coverage** — top coding agents get managed or documented sidecar lifecycles with rollback.
5. **Maintainable codebase** — split god files; file-size budgets enforced repo-wide.

## 2. Non-goals

- Replacing Headroom with a chain of independent localhost proxies.
- Promoting leanctx/chonkify/pxpipe without passing evidence gates.
- Claiming cache hits as compression savings.
- Fabricating reboot-level or public installed-app proof from local unsigned builds.

## 3. Phase overview

| Phase | Theme | Horizon | Exit gate |
| --- | --- | --- | --- |
| **P0** | Trust seal | 2–4 weeks | Public reboot proof + session proxy auth + Mode Inspector truth |
| **P1** | Savings supremacy | 4–8 weeks | Golden benchmarks + exact-cache policy + leanctx promotion path |
| **P2** | Universal coverage | 6–10 weeks | Top-5 connector lifecycle + cheapest-correct Agent Session |

---

## P0 — Trust seal

### P0.1 Public installed-app and reboot proof

**Goal:** Close release blockers listed in `docs/plan-status-ledger.md` and `docs/ai-switchboard-rebrand-release-evidence.md`.

**Tasks:**

1. Run `npm run smoke:installed:local` against `/Applications/AI Switchboard for Mac.app` from public `v0.0.0` DMG.
2. Arm reboot proof: `npm run smoke:reboot-level:arm` → reboot → `npm run smoke:reboot-level:record`.
3. Verify `npm run smoke:reboot-level:local:check` passes with real marker (not blocked summary).
4. Record Doctor/Rollback/uninstall evidence in `dist/` summaries consumed by `npm run release:proof`.

**Files:**

- `scripts/reboot-level-installed-proof-summary.mjs`
- `scripts/check-reboot-level-installed-proof-summary.mjs`
- `scripts/public-release-proof-summary.mjs`
- `docs/install.md`

**Acceptance:**

- `release:proof` no longer lists `reboot-level installed proof` as blocker when marker is valid.
- Public installed smoke summary status is `ready`, not `blocked`.

**Status:** `blocked` — requires physical reboot on signed install; automation shipped, evidence pending.

---

### P0.2 Mode Inspector proves active reality

**Goal:** Mode Inspector shows **requested vs active** mode and live evidence for listeners, hooks, MCP, LaunchAgent — not desired state only.

**Already shipped (baseline):**

- `src/components/SwitchboardPanel.tsx` — Mode Inspector UI
- Stale-shell restart guidance, proxy bind address, RTK PATH vs hook rows, Repo Memory MCP lifecycle

**Remaining tasks:**

1. Add **aggregate inspector verdict** row: `aligned` | `attention` | `blocked` derived from row evidence.
2. Surface **proxy session auth** status and token fingerprint (see P0.3).
3. Add Doctor cross-link when verdict ≠ `aligned`.
4. Frontend tests in `SwitchboardPanel.test.tsx` for verdict states.

**Files:**

- `src/components/SwitchboardPanel.tsx`
- `src/lib/modeInspectorVerdict.ts` (new)
- `src/lib/modeInspectorVerdict.test.ts` (new)

**Acceptance:**

- Inspector verdict matches fixture runtime payloads in unit tests.
- Doctor panel references inspector verdict when routing drift detected.

**Status:** `done` — verdict module, proxy auth surfacing, Doctor cross-link, and panel tests shipped 2026-08-06.

---

### P0.3 Per-session proxy auth token

**Goal:** Generate an app-session proxy token; validate optional `X-Switchboard-Proxy-Session` header on intercept; support advisory (default) and enforce modes.

**Threat model alignment:** `docs/threat-model.md` — localhost is local-process trust until bearer or UDS exists.

**Tasks:**

1. Add `src-tauri/src/proxy_session_auth.rs` — token generation, validation, redacted fingerprint.
2. Store `Arc<ProxySessionAuth>` on `AppState`; pass into `proxy_intercept::spawn`.
3. Intercept: after loopback/Origin checks, validate session header; return `401` when enforce mode on and header missing/invalid.
4. Persist enforce flag in `config/proxy-session-auth.json` under app storage.
5. Expose `get_proxy_session_auth_status` Tauri command for Mode Inspector.
6. Update `runtime_lifecycle.rs` `proxy_auth_status` / `proxy_auth_detail`.
7. Update `doctor.rs` — downgrade warning when `session_token_available`; error only when enforce expected but broken.

**Files:**

- `src-tauri/src/proxy_session_auth.rs` (new)
- `src-tauri/src/proxy_intercept.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/state/runtime_lifecycle.rs`
- `src-tauri/src/doctor.rs`
- `src-tauri/src/lib.rs` (command registration)
- `src/lib/types.ts` (RuntimeStatus fields if needed)

**Acceptance:**

- Unit tests: valid/missing/invalid header; enforce vs advisory; Debug never leaks token.
- Intercept returns `401` in enforce mode without header.
- Mode Inspector shows fingerprint + mode label.

**Status:** `done` — module, intercept validation, Settings card, and Mode Inspector fingerprint shipped 2026-08-06.

---

### P0.4 Local-only network audit completion

**Goal:** Extend `check-local-only-network.mjs` to cover world-class plan surfaces and proxy session auth config paths.

**Tasks:**

1. Add `scripts/check-p0-trust-seal.mjs` aggregating reboot summary schema, plan docs, proxy auth module presence.
2. Wire `npm run check:world-class-plan` and `npm run smoke:world-class-plan:local`.

**Acceptance:**

- `npm run check:world-class-plan` passes on CI checkout.
- Fails clearly when plan docs or slice ledger missing.

**Status:** `done` — `check:p0-trust-seal` aggregates plan docs, proxy auth surfaces, reboot schema checker, and local-only network guard 2026-08-06.

---

## P1 — Savings supremacy

### P1.1 Golden benchmark suite

**Goal:** Expand `benchmarks/fixtures.json` beyond 4 toy fixtures; add schema version and category minimums.

**Tasks:**

1. Add fixtures: long CI log, multi-file repo scan, agent transcript excerpt, JSON API error blob.
2. Add `benchmarks/schema.json` and `scripts/check-world-class-benchmarks.mjs`.
3. Document quality gate: wrong-omission rate must stay 0 on fixtures; retention ≥ 95%.

**Files:**

- `benchmarks/fixtures.json`
- `benchmarks/schema.json` (new)
- `scripts/check-world-class-benchmarks.mjs` (new)
- `scripts/run-benchmarks.mjs`

**Acceptance:**

- ≥ 8 fixtures across ≥ 4 categories.
- `npm run benchmarks` + `npm run check:world-class-benchmarks` pass.

**Status:** `in_progress`

---

### P1.2 Exact cache default policy

**Goal:** Recommend enabling exact semantic cache for safe deterministic requests in Full/Headroom modes without conflating cache hits with compression.

**Tasks:**

1. Add `src/lib/exactCacheDefaultPolicy.ts` — recommendation logic aligned with `semanticCachePolicy.ts`.
2. Surface recommendation in `OptimizationEngineProfilesCard.tsx` when cache disabled in Full mode.
3. Tests in `exactCacheDefaultPolicy.test.ts`.

**Acceptance:**

- Recommends enable in Full mode when cache disabled and runtime reachable.
- Never recommends in Off/RTK-only modes.

**Status:** `in_progress`

---

### P1.3 leanctx evidence-gated promotion

**Goal:** Keep leanctx shadow-only until capability, protected-content, and fail-open fixtures pass.

**Tasks:**

1. Add `scripts/check-leanctx-promotion-gate.mjs` reading `optimization_addons_readiness.rs` signals.
2. Block master activation allowlist promotion until gate passes (existing promotion matrix UI).

**Acceptance:**

- Script exits non-zero when required evidence files missing.
- Documented in Addons promotion matrix copy.

**Status:** `in_progress` — `leanctxPromotionGate.ts`, `check:leanctx-promotion-gate.mjs`, and master activation allowlist shipped 2026-07-30.

---

### P1.4 Provider-billed counterfactual measurement

**Goal:** Where provider usage APIs expose billed tokens, record measured before/after deltas in savings ledger.

**Tasks:**

1. Extend Token X-Ray normalization for Codex/Claude usage endpoints (read-only).
2. Label ledger rows `measured` only with complete counterfactual pair.

**Status:** `planned` — depends on provider API stability.

---

## P2 — Universal agent coverage

### P2.1 Cheapest-correct Agent Session pack

**Goal:** One-click Agent Session selects the **cheapest pack that fits budget** and maximizes cacheable prefix tokens.

**Tasks:**

1. Add `recommendAgentSessionPackId()` to `src/lib/agentSessionPacks.ts`.
2. Auto-select in `AgentSessionPanel.tsx` when budget or agent changes.
3. Show recommendation reason in UI.

**Files:**

- `src/lib/agentSessionPacks.ts`
- `src/lib/agentSessionPacks.test.ts`
- `src/components/AgentSessionPanel.tsx`

**Acceptance:**

- Unit tests: budget constraints, cacheable-token tie-break, task affinity hook point.
- Panel defaults to recommended pack.

**Status:** `in_progress`

---

### P2.2 Cursor native write gate (unchanged policy)

**Goal:** No native Cursor provider writes until documented on-disk schema + full lifecycle proof.

**Reference:** `docs/connectors.md` Cursor native-write evidence gate.

**Tasks:**

1. Add `get_cursor_native_schema_assessment` Tauri command wrapping `cursor_native::assess_native_schema`.
2. Add `src/lib/cursorNativeGate.ts` and `scripts/check-cursor-native-gate.mjs`.
3. Keep `supported: false` until Cursor publishes an allowlisted on-disk schema.

**Status:** `in_progress` — assessment command + frontend gate shipped 2026-07-30; native writes remain blocked.

---

### P2.3 OS-level Repo Memory MCP supervision

**Goal:** MCP survives app relaunch with read-only smoke recheck; document reboot survival limits honestly.

**Tasks:**

1. Extend `repo_memory_mcp_supervision_status` with relaunch survival evidence.
2. Doctor warns when MCP active but supervision degraded.

**Status:** `in_progress` — relaunch survival evidence + Doctor warnings shipped 2026-07-30; reboot/OS daemon survival is not claimed.

---

### P2.4 Connector promotion past sidecar

**Goal:** Promote connectors only when dry-run, backup, apply, verify, rollback, Off cleanup proven on fixture homes.

**Status:** `ongoing` — per-connector in `docs/plan-status-ledger.md`.

---

## P3 — Platform and maintainability (ongoing)

| Slice | Action | Status |
| --- | --- | --- |
| God-file split | Extract `client_adapters.rs` domains; split `App.tsx` views; modularize `styles.css` | `done` |
| God-file registry | `fixtures/god-file-registry.json`, growth caps, `check:god-file-registry`, `godFileRegistry.ts` | `done` |
| File-size budget | Extend `check-file-size-budget.mjs` god-file exemptions + `check:phase3-maintainability.mjs` | `done` |
| Cross-platform | CLI parity on Windows/Linux before tray apps | `done` |
| Public leaderboard | `export:benchmark-leaderboard` local export without secrets | `done` |

---

## 4. Test and release gates

### Per-slice

- Rust: `cargo test --manifest-path src-tauri/Cargo.toml --lib <filter>`
- Frontend: `vitest run <file>`
- Plan: `npm run check:world-class-plan`

### Pre-release

```bash
npm run test:all
npm run check:world-class-plan
npm run benchmarks
npm run check:world-class-benchmarks
npm run release:ready:strict
npm run release:proof
```

---

## 5. Delivery order (this program)

1. **P0.3** proxy session auth + runtime status  
2. **P0.2** Mode Inspector verdict module  
3. **P0.4** trust-seal check scripts  
4. **P1.1** benchmark expansion  
5. **P1.2** exact cache default policy  
6. **P2.1** Agent Session pack ranking  
7. **P0.1** public reboot proof (manual, signed install)  
8. **P1.3–P1.4** and **P2.2–P2.4** as evidence allows  

---

## 6. Definition of done (program)

- [ ] All P0 slices `done` in `slice-status.json`
- [ ] `release:proof` passes without reboot/installed blockers
- [ ] Benchmark suite ≥ 8 fixtures, CI-gated
- [ ] Exact cache recommendation shipped; semantic remains opt-in
- [ ] Agent Session auto-recommends cheapest valid pack
- [ ] Average robustness score ≥ 8/10 on re-analysis
