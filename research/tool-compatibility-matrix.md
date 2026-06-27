# Mac AI Switchboard Tool Compatibility Matrix

This file is the v1 research gate for add-ons and external coding tools. Mac AI Switchboard only ships automatic adapters when a tool can be detected locally, explained clearly, backed up before writes, verified after setup, rolled back exactly, and cleaned up by Off mode. Tools can appear earlier as planned connectors when detection and manual guidance are useful without mutating user config.

| Tool | Category | Runtime | Local-only fit | Install method | Maintenance view | Decision | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Headroom engine | Prompt optimization runtime | Python | Excellent | Managed Python install | Core dependency | Include | Required engine for local proxy compression. |
| RTK | Shell-output optimization | Rust binary | Strong | Managed binary install + shell/client hooks | Core dependency | Include | Safe for noisy command output and planned connectors because it does not require provider config writes. |
| Repo Intelligence | Repo context optimization | Local indexer | Strong | Built into Switchboard app/backend | Core workflow | Include | Read-only repo graph and context-pack layer for agent handoff before starting Codex, Claude Code, Gemini CLI, and similar tools. |
| Gemini CLI | Planned coding connector | External CLI | Strong for detection and handoff; unproven for automatic provider routing | User-installed CLI detected read-only | First planned connector target | Detection-only | Detect binary, version, and config surfaces without writes. Keep provider routing manual until a stable config surface, model/account compatibility, backup, verify, rollback, and Off cleanup are proven. |
| OpenCode | Planned coding connector | External CLI | Strong for detection and RTK-only usage | User-installed CLI detected read-only | Planned | Manual only | Automatic provider config edits wait for active config-path discovery, dry-run diff, backup, restore, and Off cleanup. |
| Cursor | Planned editor connector | External app | Strong for guided setup and repo packs | User-installed app detected read-only | Planned | Manual only | Account/profile-specific settings stay manual until profile-aware backup and restore are implemented. |
| Aider | Planned agent connector | External CLI | Strong for RTK-only and repo packs | User-installed CLI detected read-only | Planned | Manual only | Prefer reversible environment wrapper before any saved provider config edits. |
| Goose | Planned agent connector | External CLI | Strong for repo packs and future MCP handoff | User-installed CLI detected read-only | Planned | Manual only | Separate provider routing from MCP/Repo Intelligence handoff until backup and Off cleanup exist. |
| claude-cognitive | Workflow enhancement | Outside v1 policy | Weak | Manual external setup | Medium | Defer | Deferred because it breaks the Python-only boundary and assumes user profile edits. |

## Research checklist

- Confirm license compatibility and pin exact versions before bundling.
- Verify each tool can run inside Headroom-managed storage without relying on host-global installs.
- Verify installation/update flow can be fully local after download.
- Verify tooling has a stable CLI or library surface for adapter integration.
- Reject candidates that require unreviewed profile mutation, cloud-only setup, or credential copying.
- For planned connectors, allow read-only detection only when the app can show config surfaces, automation gates, manual workflow, and a disabled setup control.
- Promote a planned connector to automatic setup only after dry-run diff, backup, apply, verify, rollback, and Off cleanup are all implemented and tested.

## Gemini CLI Detection-Only Gate

Gemini CLI is the first planned connector target because its detection path is low risk and useful before automatic setup:

- Detection source: `PATH: gemini`, `~/.gemini`, and `~/.config/gemini`.
- Current evidence: backend reports binary path, `gemini --version` output when available, detected config surfaces, and the routing blocker.
- Safe workflow today: RTK-only shell-output savings plus Repo Intelligence handoff packs.
- Blocked automation: provider/base-url routing stays manual until model/account compatibility can be verified locally without storing credentials.
- Required before writes: stable config surface, dry-run diff, exact backup, apply, verify, rollback, and Off mode cleanup.
- Current decision: keep Gemini as `planned` and `guide`; do not convert to managed setup yet.
