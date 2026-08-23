# jcode harness evaluation

Audit date: 2026-08-23. Decision: **reference architecture only**.

This evaluation covers the `1jehuang/jcode` repository linked from
[`jcode.sh`](https://jcode.sh/). It does not authorize vendoring or installing
jcode, and it does not treat jcode benchmark claims as Switchboard evidence.

## Useful capabilities for Switchboard

| Capability | Switchboard integration | Status | Boundary |
|---|---|---|---|
| Reconnectable multi-session service | Model session ownership as an app-owned service with attach, cancel, resume, and bounded resource state. | Reference | No remote execution or daemon exposure is enabled by this evaluation. |
| Provider profiles | Reuse named provider metadata: endpoint, model family, context limit, and auth-source label. | Partial | Secrets stay in existing Keychain/config boundaries; metadata remains content-free. |
| Adaptive context | Apply already-seen and bounded-result reduction ideas to Repo Intelligence packs. | Partial | Workspace-scoped, provenance-bearing, read-only packs only; semantic memory remains gated. |
| Session search and memory | Consider a later local event-index layer for session discovery and replay. | Not started | No prompt or tool-result content may be persisted without a separate privacy contract. |
| Low-overhead UI | Use upstream measurements as a local profiling target. | Not evidence | Machine-specific upstream measurements are not Switchboard product claims. |

## Licensing and provenance

- The repository advertises an MIT license. Any copied source would require its
  license and copyright notice to be preserved.
- Cargo, npm, native, and UI dependencies require a separate inventory before
  any source or binary is copied.
- This phase copies no jcode source, binary, dependency, credential handling,
  remote transport, or provider implementation.

## Promotion gate

jcode-derived behavior remains a Switchboard reference until each proposed
surface has a version-pinned source, a Switchboard-owned contract, redaction
and workspace-isolation tests, deterministic replay or explicit non-replay
labeling, and a rollback/disable path.

