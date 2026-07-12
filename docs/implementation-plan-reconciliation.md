# Implementation Plan Reconciliation

Updated: 2026-07-12

## Architecture Rule

AI Switchboard is the independent product and user-facing control plane. Headroom remains an accurately attributed underlying compression engine; it is not presented as the product, an MCP service, or a separate user workflow.

## Implemented Foundations

- Managed routing, reversible setup, Doctor, rollback, local-only guards, release reporting, Repo Map, Repo Intelligence v1, and read-only Repo Memory MCP are implemented.
- Connector support is correctly split between managed, sidecar, guided, and gated states.
- Token X-Ray and Daily Briefing have an implemented local/content-free foundation, including persistent daily history, exports, and scoped analytics deletion.
- Platform rebrand documentation/evidence and the signed/notarized v0.0.0 release proof are recorded.

## Completed Plan Slices

The following implementation-plan slices are complete in the current checkout and are reflected in the changelog and status ledger:

- Agent Memory discovery, secret screening, compaction preview, exact-confirmation apply/rollback, session handoff, and attribution.
- Token X-Ray depth and bounded live updates: model/context/cache metadata, projected pressure, event coalescing, recommendation controls, timestamped evidence, and unavailable-state handling.
- Repo Intelligence incremental index reuse and graph-aware ranking, including task affinity and reverse-dependency hubs.
- Repo Memory MCP app-owned read-only process supervision with child/restart/exit evidence and stale-health recovery.
- Cursor, Goose, and Grok/xAI Switchboard-owned sidecar lifecycles with preview, exact confirmation, backup, verification, rollback, and Off cleanup. Native provider/account/model writes remain gated.
- Progressive-disclosure and accessibility completion for technical evidence and connector setup actions.
- Add-on measurement guardrails and explicit evidence inputs. The app records measured savings only for a complete independent baseline/optimized pair and otherwise preserves estimated status.
- Gateway readiness receipts, redacted previews, reversible local intent, and opt-in loopback LiteLLM preflight; no credentials or external configuration are written.
- Reboot-proof automation to arm, record, and check a post-reboot marker without fabricating installed-app evidence.

## Remaining Build Backlog

1. **Evidence depth**: true before/after token measurements for Caveman, Ponytail, and MarkItDown, plus RTK command-family persistence and richer provider-specific X-Ray metrics where credible APIs expose them.
2. **Repo Intelligence depth**: richer language-specific parser/call-graph coverage, mixed-language fixtures, and deeper per-tool Repo Map progress semantics.
3. **Native provider routing**: Cursor, Goose, and Grok/xAI provider/account/model writes remain gated until stable, allowlisted schemas and full lifecycle fixtures are verified. Their safe sidecars are complete.
4. **Gateway integrations**: LiteLLM semantic-cache lifecycle, self-hosted Langfuse, Cloudflare Gateway, and Kong live verification require user-controlled infrastructure and credentials. Local readiness is complete.
5. **Platform extraction**: continue CLI/Linux/Windows boundary work where it is useful; this is roadmap work rather than a blocker for the macOS product.

## External Validation Gates

- Public signed installed-app smoke and reboot-level Doctor/Rollback/uninstall proof require a real signed installation, current public smoke evidence, and a post-reboot marker. The arm/record/check automation is complete, but local code cannot manufacture that marker.
- Live gateway integration tests require user-controlled infrastructure and credentials.
- A protocol adapter must be implemented before Switchboard claims managed API routing for that provider.

## Delivery Rule

Each active backlog item is delivered in small, independently testable slices. The app must continue to distinguish implemented, guided, gated, and external-proof-required states rather than claiming unsupported automation.
