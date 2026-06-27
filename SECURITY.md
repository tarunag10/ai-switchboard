# Security Policy

## Supported Versions

This repository is under active productization. Until the first tagged release, security fixes target the active development branch.

## Reporting a Vulnerability

Please report security issues privately before opening a public issue.

Use GitHub private vulnerability reporting if it is enabled for this repository.
If it is not enabled, contact the maintainer through a private channel before
sharing technical details publicly.

Include:

- Affected commit or release.
- macOS version and chip architecture.
- Whether local-only mode was enabled.
- Exact client involved, such as Codex, Claude Code, Gemini CLI, OpenCode, Cursor, or another connector.
- Steps to reproduce.
- Relevant logs with secrets removed.

Do not submit security fixes from forks that expose exploit details, secrets, or
private infrastructure in public CI logs. Coordinate with the maintainer first.

## Areas of Interest

Security-sensitive areas include:

- Reversible client config edits.
- Local proxy routing and bypass behavior.
- Keychain usage.
- LaunchAgent setup.
- Update signing and release artifacts.
- Local-only telemetry and remote-service guards.
- Managed runtime installation, repair, and uninstall cleanup.

Do not include API keys, model-provider tokens, Apple signing credentials, update signing keys, or private repository contents in reports.
