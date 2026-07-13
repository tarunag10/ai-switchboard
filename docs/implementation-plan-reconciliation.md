# Implementation Plan Reconciliation

Updated: 2026-07-12

## Architecture Rule

AI Switchboard is the independent product and user-facing control plane. Headroom remains an accurately attributed underlying compression engine; it is not presented as the product, an MCP service, or a separate user workflow.

## Implemented Foundations

- Managed routing, reversible setup, Doctor, rollback, local-only guards, release reporting, Repo Map, Repo Intelligence v1, and read-only Repo Memory MCP are implemented.
- Connector support is correctly split between managed, sidecar, guided, and gated states.
- The Vite web shell has a verified Vercel contract (`npm ci`, `npm run build`,
  `dist`, SPA fallback, native/local artifact exclusion, and a commit-aware
  ignore step that skips native/docs-only changes). Browser previews
  guard incomplete Tauri globals and event runtimes, so hosted Vercel builds do
  not attempt desktop-only calls.
- Token X-Ray and Daily Briefing have an implemented local/content-free foundation, including persistent daily history, exports, and scoped analytics deletion.
- Platform rebrand documentation/evidence and the signed/notarized v0.0.0 release proof are recorded.

## Completed Plan Slices

The following implementation-plan slices are complete in the current checkout and are reflected in the changelog and status ledger:

- Agent Memory discovery, secret screening, compaction preview, exact-confirmation apply/rollback, session handoff, and attribution.
- Token X-Ray depth and bounded live updates: model/context/cache metadata, projected pressure, event coalescing, recommendation controls, timestamped evidence, and unavailable-state handling.
- Repo Intelligence incremental index reuse and graph-aware ranking, including task affinity and reverse-dependency hubs.
- Repo Memory MCP app-owned read-only process supervision with child/restart/exit evidence and stale-health recovery.
- Cursor, Goose, and Grok/xAI Switchboard-owned sidecar lifecycles with preview, exact confirmation, backup, verification, rollback, and Off cleanup. Goose and Grok/xAI also have allowlisted native endpoint adapters; Cursor native writes remain gated.
- Progressive-disclosure and accessibility completion for technical evidence and connector setup actions.
- Add-on measurement guardrails and explicit evidence inputs. The app records measured savings only for a complete independent baseline/optimized pair and otherwise preserves estimated status.
- Gateway readiness receipts, redacted previews, reversible local intent, and opt-in loopback LiteLLM preflight; no credentials or external configuration are written.
- Reboot-proof automation to arm, record, and check a post-reboot marker without fabricating installed-app evidence.

## Remaining Build Backlog

1. **Evidence depth**: true before/after token measurements for Caveman, Ponytail, and MarkItDown, plus richer provider-specific X-Ray metrics where credible APIs expose them. RTK command-family persistence now reads RTK's local history database read-only and exposes sanitized weighted family aggregates with timestamps.
2. **Repo Intelligence depth**: symbol-level caller-to-callee AST call edges now cover TypeScript/JavaScript/React, Rust, and Python with mixed-language fixtures, and Repo Map now exposes typed per-tool progress/current-tool evidence. Remaining depth is richer language-specific semantic resolution plus cancellation/retry semantics.
3. **Cursor native provider routing gate**: Goose and Grok/xAI endpoint routing are shipped with stable allowlists and full fixture lifecycle proof. Cursor provider/account/model writes remain gated because no supported on-disk schema is published.
4. **Gateway integrations**: LiteLLM semantic-cache lifecycle, self-hosted Langfuse, Cloudflare Gateway, and Kong live verification require user-controlled infrastructure and credentials. Local readiness is complete.
5. **Platform extraction**: continue CLI/Linux/Windows boundary work where it is useful; this is roadmap work rather than a blocker for the macOS product.

## External Validation Gates

- Public signed installed-app smoke and reboot-level Doctor/Rollback/uninstall proof require a real signed installation, current public smoke evidence, and a post-reboot marker. The arm/record/check automation is complete, but local code cannot manufacture that marker.
- Live gateway integration tests require user-controlled infrastructure and credentials.
- A protocol adapter must be implemented before Switchboard claims managed API routing for that provider.

## Delivery Rule

Each active backlog item is delivered in small, independently testable slices. The app must continue to distinguish implemented, guided, gated, and external-proof-required states rather than claiming unsupported automation.
