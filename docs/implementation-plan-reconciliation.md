# Implementation Plan Reconciliation

Updated: 2026-07-11

## Architecture Rule

AI Switchboard is the independent product and user-facing control plane. Headroom remains an accurately attributed underlying compression engine; it is not presented as the product, an MCP service, or a separate user workflow.

## Implemented Foundations

- Managed routing, reversible setup, Doctor, rollback, local-only guards, release reporting, Repo Map, Repo Intelligence v1, and read-only Repo Memory MCP are implemented.
- Connector support is correctly split between managed, sidecar, guided, and gated states.
- Token X-Ray and Daily Briefing have an implemented local/content-free foundation, including persistent daily history, exports, and scoped analytics deletion.
- Platform rebrand documentation/evidence and the signed/notarized v0.0.0 release proof are recorded.

## Active Build Backlog

1. **Agent Memory**: read-only source discovery, security scanning, compaction preview, approved apply/rollback, session handoff, and attribution.
2. **Token X-Ray and Daily Briefing depth**: provider fixtures, model/cache/context metadata, projected pressure, event coalescing, recommendation controls, and local evidence scripts.
3. **Repo Intelligence v2**: persistent/incremental parser index, richer graph ranking, mixed-language fixtures, and genuine MCP process supervision.
4. **Connector promotion**: Cursor, Continue, Goose provider routing, and Grok/xAI only through detect, dry-run, backup, exact confirmation, verify, rollback, Off cleanup, and fixture-home proof.
5. **Progressive Disclosure completion**: explicit setup actions and screen-by-screen accessibility/default-detail audit.
6. **Add-on measurement**: true before/after evidence where a credible baseline exists; retain estimates otherwise.
7. **Gateway integrations**: local guided profiles can be built, but LiteLLM, Langfuse, Cloudflare, and Kong live verification need user infrastructure and credentials.

## External Validation Gates

- Public signed installed-app smoke and reboot-level Doctor/Rollback/uninstall proof require a real signed installation and reboot marker.
- Live gateway integration tests require user-controlled infrastructure and credentials.
- A protocol adapter must be implemented before Switchboard claims managed API routing for that provider.

## Delivery Rule

Each active backlog item is delivered in small, independently testable slices. The app must continue to distinguish implemented, guided, gated, and external-proof-required states rather than claiming unsupported automation.
