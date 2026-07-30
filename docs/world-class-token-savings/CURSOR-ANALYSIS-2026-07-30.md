# Cursor Analysis — AI Switchboard Robustness Review

**Date:** 2026-07-30  
**Analyst:** Cursor Agent (Composer)  
**Repository:** `mac-ai-switchboard` @ `main`  
**App version:** `0.0.1` (public release artifact `v0.0.0`)

---

## Executive summary

**Verdict:** AI Switchboard is one of the most evidence-driven, privacy-conscious token-saving control planes in the coding-agent space. It is **not yet robust end-to-end** and **not yet the world’s best token-saving app**.

The product already ships a real multi-layer savings stack (Headroom + RTK + Repo Intelligence), unusually honest measured-vs-estimated attribution, Doctor/Rollback contracts, and heavy local test coverage. Remaining gaps are trust proof (reboot-level public evidence), localhost security boundaries, blocked advanced compressors, thin benchmark fixtures, connector coverage (especially Cursor native), and maintainability debt in multi-thousand-line modules.

**Average robustness score:** ~6.0 / 10

---

## What the app is

AI Switchboard for Mac is a **local-first** Tauri menu-bar app and CLI that turns coding-agent optimizations on and off from one surface:

| Layer | Role |
| --- | --- |
| **Headroom** | Local HTTP intercept (`127.0.0.1:6767`) compresses provider prompts/context |
| **RTK** | Shell/tool output compression (“Rust Token Killer”) |
| **Repo Intelligence / Repo Map** | Read-only repo indexing, bounded context packs, agent handoffs |
| **Modes** | Full / Headroom-only / RTK-only / Off with Off-mode cleanup |
| **Token X-Ray + Daily Briefing** | Confidence-labelled local analytics, 365-day history |
| **Agent Memory** | Screened compaction with SHA-verified backups and rollback |
| **Doctor + Rollback Center** | Dry-run → backup → apply → verify → restore |

Local-first ≠ offline-only: model traffic still reaches provider APIs. Switchboard state, Doctor evidence, add-ons, and repo metadata stay on-device.

### Tray navigation surfaces

Home · Usage · Token X-Ray · Daily Briefing · Agent Memory · Doctor · Optimize · Activity · Repo Map · Repo Intelligence · Addons · Settings

### Tech stack

| Layer | Stack |
| --- | --- |
| Frontend | React + Vite (`src/`) |
| Backend | Tauri 2 / Rust (`src-tauri/`) |
| Runtime | Headroom proxy (~0.27 pin), RTK, managed Python tools |
| Storage | App Support dir, SQLite (cache/analytics), Keychain |
| CLI | `switchboard`, `repo:intelligence` |

**Scale signals (2026-07-30):** ~143k source lines across 327 TS/RS/CSS files · 141 Tauri commands · 926 Rust `#[test]` · 83 frontend test files · 57 release/smoke/check npm scripts · 9 CI workflows

---

## Token-savings architecture

### Live default path (shipped)

1. **Headroom** — prompt/context compression via managed local proxy  
2. **RTK** — command-output compression; family aggregates from RTK history DB  
3. **Repo Intelligence** — task-aware bounded packs instead of whole-repo dumps  
4. **Switchboard modes** — orchestrate which layers are active  

### Opt-in / shadow / blocked layers

| Engine | Status | Notes |
| --- | --- | --- |
| Semantic cache (exact) | Opt-in | SQLite exact replay; fail-open; separate from compression ledger |
| leanctx sidecar | Shadow only | Managed readiness; no live provider routing |
| chonkify | Blocked | License/provenance gate |
| LLMLingua-2 | Blocked | Research fallback only |
| pxpipe | Blocked | Requires upstream Headroom `text_image` capability |

### Measured fixture benchmark (local run 2026-07-30)

| Fixture | Saved % | Fact retention |
| --- | ---: | ---: |
| Noisy test log | 66.0 | 100% |
| Stack trace summary | 67.6 | 100% |
| Task-aware pack vs broad scan | 68.7 | 100% |
| Office/PDF markdown handoff | 54.0 | 100% |

**Caveat:** static fixtures only; no LLM quality judging; toy inputs — not world-class evidence yet.

### Savings honesty (strength)

The product separates **measured**, **estimated**, and **inferred** savings and refuses to merge them into one opaque number. Headroom/RTK are strongest evidence; Repo packs are estimated; several add-ons remain inferred until stronger counters exist.

---

## Connector matrix

| Client | Status | Routing | RTK | Notes |
| --- | --- | ---: | ---: | --- |
| Claude Code | Managed | Yes | Yes | Full reversible lifecycle |
| Codex | Managed | Yes | Partial | Provider block + Doctor repair |
| Gemini CLI | Limited managed | Limited | No | Shell base-url adapter |
| OpenCode | Limited managed | Limited | No | Provider adapter gated |
| Goose / Grok | Managed endpoints | Yes* | No | Allowlisted fields only |
| Continue / Aider / Qwen / Amazon Q / Zed | Sidecar | Yes | No | Provider state manual |
| Cursor / Windsurf | Guided / gated | No* | No | Packs + detection; Cursor native off |

---

## Robustness scorecard

| Area | Score | Evidence |
| --- | ---: | --- |
| Local-first privacy | 9/10 | Redaction defaults, local-only builds, Keychain, secret-path exclusion |
| Rollback / Doctor | 8/10 | Dry-run, backup, verify, Off cleanup; reboot public proof blocked |
| Measured savings honesty | 8/10 | Confidence labels, caveats in Token X-Ray |
| Core compression path | 7/10 | Headroom + RTK live; advanced engines blocked |
| Connector coverage | 6/10 | Claude/Codex strong; Cursor native gated |
| Security boundary | 6/10 | Loopback-only; no per-session proxy auth until P0 slice |
| Code maintainability | 4/10 | God files: `client_adapters.rs` ~9.4k, `App.tsx` ~5.7k, `styles.css` ~11.7k |
| Product maturity | 4/10 | v0.0.x productization branch |
| Cross-platform | 3/10 | macOS polished v1 only |
| Savings proof quality | 5/10 | 4 static fixtures; no golden-task suite |

---

## What is already robust

- Consent → dry-run → backup → apply → verify → rollback connector lifecycle  
- Fail-open on Headroom compression refusal (Codex oversized-request path)  
- Message logging off by default + redaction + purge  
- Heavy automated test surface and release evidence scripts  
- Signed/notarized Apple Silicon public DMG (`v0.0.0`)  
- Mode Inspector with routing rows, MCP lifecycle, stale-shell guidance (partially shipped)  

---

## Critical gaps vs “best in the world”

| Gap | Impact |
| --- | --- |
| Not foolproof | Reboot-level public Doctor/Rollback/uninstall proof blocked |
| Localhost ≠ security boundary | `127.0.0.1:6767` without per-session proxy auth |
| Savings partly estimated | Provider-billed counterfactuals pending |
| Advanced compressors not live | Headroom+RTK only default path |
| God-file architecture | Regression risk in multi-kLOC modules |
| macOS-only polished surface | Windows/Linux second-class |
| Thin benchmarks | Not reproducible golden-task evidence |

---

## Explicit non-goals / limits

- Does not stop provider-side token billing itself  
- Does not guarantee post-compression task quality without benchmarks  
- Does not write into user repositories (by design)  
- Does not fully automate Cursor provider config (gated)  
- Does not ship advanced compressors by default  

---

## Definition of done for #1

### Savings

- Default stack beats baseline agents on a public golden-task suite  
- Measured provider-billed deltas published with confidence labels  
- Wrong-omission rate bounded; fail-open never silent  

### Trust

- Reboot-surviving Doctor/Rollback/uninstall proofs on signed builds  
- Authenticated local proxy or UDS; Mode Inspector proves active reality  
- Connector lifecycle complete for top 5 coding agents  

---

## Bottom line

Keep the honesty, Doctor/Rollback, and multi-layer architecture — they are the foundation of a category leader. To become the best token-saving app, close trust proofs, ship only evidence-gated compressor promotions, deepen connector coverage (especially Cursor), and replace toy fixtures with a rigorous reproducible benchmark suite.

**Next document:** [IMPLEMENTATION-PLAN.md](./IMPLEMENTATION-PLAN.md)
