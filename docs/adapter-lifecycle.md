# Adapter Lifecycle Contract

Every managed connector must satisfy the same safety lifecycle before it can be
advertised as managed.

## Required Stages

1. Detect without reading secrets.
2. Show dry-run diff or setup preview.
3. Create a timestamped backup or reversible restore point.
4. Apply only after explicit consent.
5. Verify setup in Doctor.
6. Roll back without touching unrelated user config.
7. Clean up in Off mode and uninstall previews.
8. Report managed footprint without secret values.
9. Provide manual recovery docs.

## Promotion Rule

Gated, guided, or detected connectors must not become managed until fixture
tests cover apply, verify, rollback, Doctor repair, Off cleanup, and managed
footprint reporting. The connector manifest must include automation gates for
backup, verify, rollback, and Off cleanup.

Claude Code and Codex are first-class managed targets. Gemini CLI, OpenCode,
Windsurf, and Zed AI have promoted managed adapters. Goose and Grok / xAI CLI
now also have promoted, allowlisted native endpoint adapters alongside their
Switchboard-owned sidecar or Repo Memory MCP lifecycle. Aider, Continue, Qwen
Code, and Amazon Q have promoted Switchboard-owned sidecar adapters that leave
provider, account, credential, and model config manual. Cursor native provider
writes remain guided or gated because no supported on-disk provider schema has
been published; its isolated Switchboard sidecar remains reversible.
