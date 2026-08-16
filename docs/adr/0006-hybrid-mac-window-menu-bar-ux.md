# ADR-0006: Use a full Mac window with a menu-bar companion

- Status: Accepted
- Date: 2026-08-16

## Context

AI Switchboard has outgrown a menu-bar-only interface. Onboarding, connectors,
endpoint configuration, benchmarks, Doctor evidence, and rollback need more
space and navigation, while mode, health, warnings, and pause controls benefit
from immediate menu-bar access.

## Decision

AI Switchboard for Mac ships as a full macOS application with a persistent
menu-bar companion. The main window owns onboarding and detailed workflows.
The menu bar owns glanceable mode and health, warnings, quick pause/Off and
restart actions, brief savings, and an action to open the main window. Closing
the main window does not silently disable active background optimization.

## Alternatives

- Keep the menu bar as the only interface.
- Replace the menu bar with a conventional full-window app.
- Create separate applications for background control and configuration.

## Consequences

Complex workflows gain accessible layouts and evidence views while frequent
controls stay fast. Both surfaces must share one state model, make background
activity obvious, support keyboard navigation, and avoid conflicting controls
or lifecycle behavior.

## Reversal strategy

Either surface can be reduced behind a feature flag while retaining the shared
state and command layer. If the full window is unavailable, the menu bar keeps
safe mode, health, Off, and recovery access; if menu-bar integration fails, the
main window exposes the same safety controls.
