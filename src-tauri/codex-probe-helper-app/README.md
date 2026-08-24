# AI Switchboard Codex Probe Helper App

This MIT-licensed crate is intended for AI Switchboard's private, non-commercial research use. It is a deliberately non-executing protocol-v1 helper boundary.

The binary reads one bounded preparation frame from standard input, waits for closed input, validates the frame with the local `codex-probe-helper` protocol crate, and writes one shape-consistent no-process response to standard output. It does not authenticate a host, establish freshness, grant launch authority, claim support, inspect a Codex installation, execute `codex --version`, or start any process.

The helper does not read process arguments or environment values and opens no filesystem, path, network, shell, Tauri, platform, provider, workspace, or logging surface. Its only production dependency is the local protocol crate.

The protocol deliberately waits for the host to close standard input and has no internal wall-clock or process-control authority. Any future native parent integration must own a fixed deadline, close the pipe, concurrently drain the bounded output pipes, terminate and reap the exact child on timeout, clear inherited environment values, and treat a missing response as failure. No such launcher exists in this phase.

The included property lists define the future nested macOS app identity and an exact sandbox-only entitlement. This directory defines no bundling, installation, independent signing, launch, or parent-app connection. A local macOS compiler output may be linker ad-hoc signed; that development artifact is not independent or release signing, nested-code trust, notarization, or distribution evidence. Parent bundle integration and independent release verification remain a separate phase.
