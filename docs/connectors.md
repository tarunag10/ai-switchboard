# Connector Support

Mac AI Switchboard treats connector status as a safety boundary. A tool is not advertised as fully managed until setup, verification, rollback, Off mode cleanup, and manual recovery are all implemented and tested for that tool. See [Adapter Lifecycle Contract](adapter-lifecycle.md) for the managed promotion rule.

## Status Labels

| Label | Meaning |
| --- | --- |
| Managed | Switchboard can apply reversible setup, verify it, repair drift, and clean up managed edits. |
| Managed MCP | Switchboard manages a read-only Repo Memory MCP bridge while provider/account mutation remains gated. |
| Limited managed adapter | A bounded adapter exists, but provider/account mutation or some lifecycle evidence remains gated. |
| Guided | Switchboard detects the tool and gives a safe manual workflow, usually with copyable Repo Intelligence packs. |
| Detected | Switchboard can identify the tool or known config locations, but does not guide writes yet. |
| Gated | The tool has detection or handoff coverage, but native/provider writes stay blocked until the full reversible lifecycle is proven. |
| Unsupported | No supported workflow is shipped. |

## Support Matrix

| Tool | Status | Automatic routing | RTK support | Repo packs | Notes |
| --- | --- | ---: | ---: | ---: | --- |
| Claude Code | Managed | Yes | Yes | Yes | First-class managed target with reversible config edits. |
| Codex | Managed | Yes | Partial | Yes | First-class managed target with provider block and bypass handling. |
| Gemini CLI | Managed | Yes | No | Yes | Managed shell/base-url routing with sibling rollback backups, Doctor repair, rollback, and Off cleanup. |
| OpenCode | Managed | Yes | No | Yes | Managed provider routing with backup, verify, rollback, and Off cleanup gates. |
| Cursor | Gated | No | No | Yes | Copyable packs, settings discovery, and dry-run target/marker preview today; native/provider writes remain blocked. |
| Windsurf | Managed | Yes | No | Yes | Managed editor settings routing with backup, Doctor verification, rollback, and Off cleanup. |
| Aider | Managed | Yes | No | Yes | Switchboard-owned sidecar with Doctor verification, rollback, and Off cleanup; provider config remains manual. |
| Continue | Managed | Yes | No | Yes | Switchboard-owned sidecar with Doctor verification, rollback, and Off cleanup; provider config remains manual. |
| Goose | Managed MCP | MCP only | No | Yes | Read-only Repo Memory MCP bridge; native provider routing remains manual and unmodified. |
| Qwen Code | Managed | Yes | No | Yes | Switchboard-owned sidecar with Doctor verification, rollback, and Off cleanup; account/model setup remains manual. |
| Amazon Q Developer CLI | Managed | Yes | No | Yes | Switchboard-owned sidecar with Doctor verification, rollback, and Off cleanup; AWS auth/provider/workspace state remains manual. |
| Zed AI | Managed | Yes | No | Yes | Managed assistant settings routing with backup, Doctor verification, rollback, and Off cleanup. |
| Grok / xAI CLI | Gated | No | No | Yes | Detection and config semantics need more evidence before native/provider writes. |

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

Repo Intelligence packs are safe for every listed tool because they are read-only and copyable. Goose additionally has the managed Repo Memory MCP bridge for read-only context handoff. Provider routing, settings mutation, and account-specific config stay gated connector by connector.
