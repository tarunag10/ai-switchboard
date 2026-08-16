# Phase 3 policy brain and model-routing gate

Phase 3 uses deterministic rules, not a learned router. Every optimization
score exposes the same six terms and their common integer unit:

```text
net value = input cost saved
          + prefill compute saved
          + context headroom value
          - optimization latency cost
          - cache-break cost
          - quality risk
```

Negative inputs are rejected by the unsigned serialized contract. Saturating
arithmetic prevents malformed extreme estimates from wrapping into a favorable
decision. A net value must be positive to be favorable, and the serialized
score includes the full calculation as an explanation.

## Model-routing promotion

The default stage is `observe`. It records the proposed model, actual requested
model, deterministic task class, reason, success, successful-task cost, and
follow-up rework without changing the live route.

`userApproved` requires approval for the current decision. Approval is never
remembered as permission for later requests.

`automaticAllowlisted` requires all of the following:

- global routing and routing for the current client are enabled;
- the deterministic task class is explicitly allowlisted;
- the benchmark has the configured minimum sample size;
- success-rate regression is within its basis-point limit;
- average successful-task cost improves by the configured minimum;
- follow-up rework remains within its basis-point limit.

Missing or failing evidence returns to observation and keeps the requested
model. The decision exposes each measured value beside its threshold. Global
and case-insensitive per-client kill switches also preserve the requested
model. These gates deliberately do not inspect prompt content or model output.
