# ADR-0001: AI Switchboard owns the stable local intercept

- Status: Accepted
- Date: 2026-08-16

## Context

Coding-client configurations need a durable local destination even when the
active optimization engine, route policy, or downstream inference endpoint
changes. Rewriting every client configuration for each backend would multiply
risk and weaken rollback. The current small Rust intercept already provides a
stable edge in front of the dynamically managed Headroom backend.

## Decision

AI Switchboard owns one stable, loopback-only local intercept. Coding clients
target that boundary. The intercept authenticates and normalizes the request,
collects request facts, applies the selected policy result, and fails safely. It
delegates optimization to an `OptimizationEngine` and delivery to an
`InferenceEndpoint`; provider- or runtime-specific behavior does not accumulate
in the intercept.

## Alternatives

- Configure every coding client directly for each engine or endpoint.
- Make Headroom own the permanent public intercept.
- Add provider-specific conditionals directly to the intercept.

## Consequences

Client configuration stays stable and engine or endpoint changes remain
internal to Switchboard. The intercept becomes a safety-critical component that
requires strict loopback binding, redaction, compatibility fixtures, bounded
responsibilities, and fail-open or bypass behavior where policy permits.

## Reversal strategy

Introduce a versioned replacement intercept behind an opt-in migration. Keep
the prior listener and client configuration available until parity, rollback,
and Off-mode cleanup fixtures pass. Restore backed-up client settings if the
stable boundary is ever removed.
