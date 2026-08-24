# AI Switchboard Codex Probe Helper App

This MIT-licensed crate is intended for AI Switchboard's private, non-commercial research use. It is a deliberately non-executing protocol-v2 helper boundary.

The binary reads one bounded preparation frame from standard input, waits for closed input, validates the frame with the local `codex-probe-helper` protocol crate, and writes one shape-consistent no-process response to standard output. It does not authenticate a host, establish freshness, grant launch authority, claim support, inspect a Codex installation, execute `codex --version`, or start any process.

The helper does not read process arguments or environment values and opens no filesystem, path, network, shell, Tauri, platform, provider, workspace, or logging surface. Its only production dependency is the local protocol crate.

The protocol deliberately waits for the host to close standard input and has no internal wall-clock or process-control authority. Any future native parent integration must own a fixed deadline, close the pipe, concurrently drain the bounded output pipes, terminate and reap the exact child on timeout, clear inherited environment values, and treat a missing response as failure. No such launcher exists in this phase.

The included property lists define the nested macOS app identity and an exact sandbox-only entitlement. AI Switchboard's packaging scripts now build this locked crate for the requested Apple target, assemble it under `Contents/Helpers`, sign the helper before the parent with this entitlement, and verify the staged and packaged signatures. Developer ID packaging must use the same team as the parent; local packaging is explicitly ad-hoc and is not release, notarization, Gatekeeper, or sandbox-enforcement evidence.

Packaging adds no parent runtime command, launcher, IPC, LaunchServices registration, or Codex execution path. The helper remains non-executing and unreachable from the running app. Runtime authentication, freshness, launch reservation, enforced containment, and any manual probe remain a separate phase.
