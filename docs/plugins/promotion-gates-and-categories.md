# Plugin Promotion Gates and Categories

AI Switchboard plugins share a promotion-evidence format, not a universal
execution interface. The six categories are deliberately separate:

- `OptimizationEngine` owns bounded lifecycle and request transformation;
- `OptimizationAddon` contributes one bounded action to a host engine;
- `ContextProvider` produces a versioned, read-only context pack;
- `CodingClientAdapter` owns client-specific detect, plan, apply, and verify;
- `InferenceEndpointProfile` declares one protocol and requires explicit user enrollment;
- `TelemetryExporter` accepts only content-free telemetry and must preserve local-only operation.

Every candidate fails closed unless it provides pinned provenance and artifact
digest, an allowlisted SPDX license with source, an explicit network
declaration, deterministic fixtures with a bundle digest, measured quality,
an exact version pin, and an HTTPS update source. The default quality gate
requires at least 30 measured samples, at least 98% successful tasks, and no
more than 1% wrong omissions.

`no_network` declarations require a deterministic verification fixture.
Declared remote destinations must be HTTPS, enumerate destinations, require
explicit user opt-in, and attest secret redaction. A plugin that writes state
must additionally supply rollback, uninstall, and deterministic post-write
verification contracts. Read-only candidates are not forced to invent those
write-recovery contracts.

The checked-in candidate fixture is synthetic and uses `example.invalid`; it
tests schema and gate behavior and is not a promoted runtime plugin.
