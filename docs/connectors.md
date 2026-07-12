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
| Cursor | Gated | No | No | Yes | Copyable packs, profile-aware settings discovery, and dry-run target/marker preview today; native/provider writes remain blocked because Cursor documents API-key setup through Settings > Models rather than a stable file-backed provider/model/base-url schema. |
| Windsurf | Managed | Yes | No | Yes | Managed editor settings routing with backup, Doctor verification, rollback, and Off cleanup. |
| Aider | Managed | Yes | No | Yes | Switchboard-owned sidecar with Doctor verification, rollback, and Off cleanup; provider config remains manual. |
| Continue | Managed | Yes | No | Yes | Switchboard-owned sidecar with Doctor verification, rollback, and Off cleanup; provider config remains manual. |
| Goose | Managed MCP | MCP only | No | Yes | Read-only Repo Memory MCP bridge; native provider routing remains manual and unmodified. |
| Qwen Code | Managed | Yes | No | Yes | Switchboard-owned sidecar with Doctor verification, rollback, and Off cleanup; account/model setup remains manual. |
| Amazon Q Developer CLI | Managed | Yes | No | Yes | Switchboard-owned sidecar with Doctor verification, rollback, and Off cleanup; AWS auth/provider/workspace state remains manual. |
| Zed AI | Managed | Yes | No | Yes | Managed assistant settings routing with backup, Doctor verification, rollback, and Off cleanup. |
| Grok / xAI CLI | Managed | Yes | No | Yes | Native routing uses the installed Grok Build documented `[endpoints].models_base_url` field in `~/.grok/config.toml`; credentials, account, and model selection remain manual. |

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

### Grok / xAI native routing evidence

The installed Grok Build documentation (`~/.grok/docs/user-guide/11-custom-models.md`) explicitly documents `~/.grok/config.toml`, `[endpoints]`, and the non-secret `models_base_url` field for an OpenAI-compatible `/v1` endpoint. Switchboard manages only that field, routing it to the local Headroom-compatible proxy with fixture-home dry-run, exact confirmation, sibling backup, Doctor verification, rollback, and Off cleanup. It never reads or writes Grok `auth.json`, API keys, account state, or model selection; `XAI_API_KEY` or `grok login` remains a user-managed prerequisite.

### Cursor native-write evidence gate

Cursor's [official API-key documentation](https://cursor.com/help/models-and-usage/api-keys) describes adding provider keys in **Cursor Settings → Models** for OpenAI, Anthropic, Google, Azure OpenAI, and AWS Bedrock. It does not define a supported on-disk provider/model/base-url schema. Switchboard therefore discovers only documented `settings.json`/`settings.jsonc` profile paths and never reads or writes their contents, `globalStorage`, `storage.json`, `state.vscdb`, account data, credentials, or secrets. The native adapter remains disabled until Cursor publishes an allowlisted file schema and Switchboard has fixture-home detect, dry-run, backup, consented apply, verify, rollback, and Off cleanup proof. The isolated Switchboard-owned sidecar lifecycle remains available.
