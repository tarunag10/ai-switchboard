# ADR-0003: Headroom is the first OptimizationEngine

- Status: Accepted
- Date: 2026-08-16

## Context

AI Switchboard currently relies on Headroom for request-path compression, but
the product also owns client lifecycle, policy, routing, observability, and
other optimization surfaces. Treating Headroom as the entire product prevents
additional engines and bypass modes from sharing a stable contract.

## Decision

Define an `OptimizationEngine` boundary and represent Headroom as its first
implementation. Switchboard owns selection and policy; Headroom owns its engine
behavior. Initial wrapping preserves current Headroom behavior and telemetry
before any engine expansion. Product and UI naming must say **AI Switchboard**
for the control plane and **Headroom** only for the engine.

## Alternatives

- Keep Headroom calls embedded throughout the product.
- Rename AI Switchboard to Headroom and make the engine the product boundary.
- Build a new optimization engine before extracting the current behavior.

## Consequences

Headroom remains the proven default while future engines can be evaluated
without rewriting coding-client adapters. The interface must expose honest
capabilities, lifecycle, health, bypass, and optimization evidence; it must not
pretend engines are interchangeable where behavior differs.

## Reversal strategy

The Headroom implementation can remain the sole engine or be selected directly
behind a compatibility facade if abstraction costs exceed demonstrated value.
Preserve current configuration, mode, and bypass behavior so removing the
interface does not require client reconfiguration.
