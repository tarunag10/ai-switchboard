# Connector Support

Mac AI Switchboard treats connector status as a safety boundary. A tool is not advertised as fully managed until setup, verification, rollback, Off mode cleanup, and manual recovery are all implemented and tested for that tool. See [Adapter Lifecycle Contract](adapter-lifecycle.md) for the managed promotion rule.

## Status Labels

| Label | Meaning |
| --- | --- |
| Managed | Switchboard can apply reversible setup, verify it, repair drift, and clean up managed edits. |
| Limited managed adapter | A bounded adapter exists, but provider/account mutation or some lifecycle evidence remains gated. |
| Guided | Switchboard detects the tool and gives a safe manual workflow, usually with copyable Repo Intelligence packs. |
| Detected | Switchboard can identify the tool or known config locations, but does not guide writes yet. |
| Planned | The tool is on the roadmap, but automation is not ready. |
| Unsupported | No supported workflow is shipped. |

## Support Matrix

| Tool | Status | Automatic routing | RTK support | Repo packs | Notes |
| --- | --- | ---: | ---: | ---: | --- |
| Claude Code | Managed | Yes | Yes | Yes | First-class managed target with reversible config edits. |
| Codex | Managed | Yes | Partial | Yes | First-class managed target with provider block and bypass handling. |
| Gemini CLI | Managed | Yes | No | Yes | Managed shell/base-url routing with sidecar evidence, Doctor repair, rollback, and Off cleanup. |
| OpenCode | Managed | Yes | No | Yes | Managed provider routing with backup, verify, rollback, and Off cleanup gates. |
| Cursor | Guided | No | No | Yes | Copyable packs and settings detection today. |
| Windsurf | Managed | Yes | No | Yes | Managed editor settings routing with backup, Doctor verification, rollback, and Off cleanup. |
| Aider | Guided | No | No | Yes | CLI handoffs and manual workflow before managed routing. |
| Continue | Guided | No | No | Yes | Editor/extension config remains manual. |
| Goose | Guided | No | No | Yes | MCP/repo handoff fit before native routing. |
| Qwen Code | Detected | No | No | Yes | Detection and handoff only until lifecycle tests exist. |
| Amazon Q Developer CLI | Detected | No | No | Yes | Detection and handoff only until lifecycle tests exist. |
| Zed AI | Managed | Yes | No | Yes | Managed assistant settings routing with backup, Doctor verification, rollback, and Off cleanup. |
| Grok / xAI CLI | Planned | No | No | Yes | Detection and config semantics need more evidence. |

## Automation Gates

Before a connector moves to Managed, it must have:

- Detection that avoids reading secrets.
- Setup dry run with target paths and before/after preview.
- Timestamped backup before edits.
- Idempotent apply.
- Doctor verification.
- Rollback or restore action.
- Off mode cleanup that removes only Switchboard-owned edits.
- Fixture-home tests for apply, repair, rollback, and cleanup.
- Manual recovery docs.

Repo Intelligence packs are safe for every listed tool because they are read-only and copyable. Provider routing, settings mutation, and account-specific config stay gated connector by connector.
