# ADR-0002: Managed coding clients share one adapter lifecycle

- Status: Accepted
- Date: 2026-08-16

## Context

Coding clients store configuration in different files and schemas, but a
Switchboard-managed integration always carries the same user-data risks. A
client-specific shortcut can leave routing active in Off mode, overwrite
unrelated settings, or claim health without evidence.

## Decision

Every managed coding client or agent is represented through a
`CodingClientAdapter`. Promotion to managed requires: secret-safe detection,
dry-run preview, timestamped backup or restore point, explicit consent, apply,
Doctor verification, scoped rollback, Off-mode and uninstall cleanup, a
secret-free managed-footprint report, and manual recovery documentation.
Fixture tests must cover apply, verify, rollback, repair, cleanup, footprint,
and relevant version mismatch behavior.

## Alternatives

- Maintain unrelated setup scripts for each client.
- Support detection and manual instructions only for every client.
- Treat successful file mutation as sufficient verification.

## Consequences

New adapters take longer to promote but have a uniform trust contract. Clients
without a documented writable schema or complete rollback evidence remain
detected or guided rather than managed. Shared lifecycle tooling becomes a
compatibility boundary and must preserve unrelated user configuration.

## Reversal strategy

Demote an adapter to guided or detected immediately if a lifecycle guarantee
fails. Use its stored restore point to roll back managed changes. A replacement
lifecycle requires a superseding ADR and fixture evidence at least as strong as
the contract above.
