# ADR-0007: Model routing remains observe-only until benchmark promotion

- Status: Accepted
- Date: 2026-08-16

## Context

Automatic model selection can reduce cost or latency, but a wrong selection can
degrade coding quality, tool use, or reliability. Current routing suggestions
do not yet have sufficient benchmark evidence to change the user's selected
model safely.

## Decision

Model routing remains observe-only. Switchboard may record request facts, the
proposed cheap or capable model, confidence, reasons, and estimated effects, but
must not alter the selected model. Promotion requires representative offline
and opt-in live benchmarks with quality/success, tool correctness, latency,
cost, and cache-impact measures; explicit thresholds; explainable deterministic
rules; user-visible opt-in; per-route kill switches; and rollback evidence.
Failure to meet quality thresholds leaves routing observe-only.

## Alternatives

- Enable automatic model routing immediately with heuristics.
- Remove routing observations until the policy is production-ready.
- Use an ML router without deterministic promotion gates.

## Consequences

The product gathers comparison evidence without silently changing outcomes.
Savings remain hypothetical until a route is actually promoted and measured.
Promotion takes longer and requires representative fixtures, but regressions
are detectable and reversible.

## Reversal strategy

Any promoted route can be returned to observe-only globally or per endpoint
without changing coding-client configuration. Preserve the user's prior manual
model selection, disable the routing rule, and compare post-rollback benchmarks
before attempting promotion again.
