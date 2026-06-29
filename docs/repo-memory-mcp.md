# Repo Memory MCP

Repo Memory MCP is the read-only agent-consumption surface for Repo Intelligence. It lets supported local coding agents request bounded repo context without rescanning the project or copying whole files into every session.

## Current Status

- Transport: stdio MCP served by `scripts/repo-intelligence.mjs --mcp-serve`.
- Install action: **Install MCP** in the Mode Inspector, backed by `install_repo_memory_mcp`.
- Session controls: **Start MCP** and **Stop MCP** in the Mode Inspector, backed by `start_repo_memory_mcp` and `stop_repo_memory_mcp`. Start verifies the read-only smoke contract and records the current app process before marking MCP active; these controls do not claim a separate background daemon is running.
- Verification: `npm run check:repo-memory-mcp`.
- Tools: `repo_context_pack`, `repo_symbol_lookup`, and `repo_dependents_of`.
  Switchboard-compatible aliases are also exposed:
  `switchboard.list_context_packs`, `switchboard.build_context_pack`, and
  `switchboard.get_repo_graph_summary`.
- Safety: read-only tools, secret-like paths excluded, generated/vendor paths skipped, and pack output bounded by Repo Intelligence budgets.

## Agent Consumption

Use Repo Memory MCP after indexing a real local repo from the Repo Intelligence view. Agents should treat MCP output as context only; it is not an approval to write files, mutate config, or bypass Switchboard safety gates.

Recommended agent flow:

1. Index or refresh the repo in Mac AI Switchboard.
2. Install Repo Memory MCP from the Mode Inspector if it is not configured.
3. Start Repo Memory MCP from the Mode Inspector when the agent session should be allowed to consume app-managed repo context; the app runs the same read-only smoke before marking it active.
4. Run `npm run check:repo-memory-mcp` manually when you want an extra terminal proof of the read-only tool contract.
5. Ask the agent to request a bounded context pack with `repo_context_pack`.
6. Use `repo_symbol_lookup` or `repo_dependents_of` only for targeted follow-up.
7. Stop Repo Memory MCP from the Mode Inspector when the app session should no longer advertise active MCP context.

The same handoff information is also available without MCP:

```bash
npm run repo:intelligence -- <repo-path> --manifest
npm run repo:intelligence -- <repo-path> --pack implementation --format markdown
npm run repo:intelligence -- <repo-path> --agent codex --format markdown
npm run repo:intelligence -- <repo-path> --session --agent codex --task verification --format markdown
```

## Connector Notes

Claude Code, Codex, Gemini CLI, OpenCode, Aider, Goose, Cursor, Continue, Grok / xAI CLI, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI should consume Repo Intelligence as read-only context. Managed connector readiness dossiers may describe config paths and rollback strategy, but provider/editor native config mutation stays gated behind explicit backup, apply, verify, rollback, Doctor, and Off cleanup evidence.

Goose and other MCP-aware tools should keep Repo Memory MCP separate from provider routing. MCP context can be enabled while provider config remains manual or sidecar-only.

### Connector Consumption Matrix

| Connector | Preferred Repo Memory path | Boundary |
| --- | --- | --- |
| Claude Code | Use Repo Memory MCP tools after **Install MCP**, **Start MCP**, and `npm run check:repo-memory-mcp` pass. | MCP context is read-only and separate from Claude Code shell routing or Headroom engine mode. |
| Codex | Prefer Start Agent Session or `repo:intelligence --session`; use MCP only when the active Codex environment can call configured MCP tools. | MCP context does not change Codex provider config, model selection, or `OPENAI_BASE_URL` routing. |
| Gemini CLI | Use Start Agent Session handoffs first; use MCP only when Gemini is running in an MCP-capable wrapper or environment. | Gemini base-url/env routing remains managed separately and must keep its backup/rollback evidence. |
| OpenCode | Use Start Agent Session handoffs first; use MCP only when OpenCode's configured runtime can call MCP tools. | OpenCode provider config routing remains guarded by native backup, verify, rollback, and Off cleanup. |
| Goose | Use Repo Memory MCP as the preferred read-only context path once configured. | Keep MCP server setup separate from Goose provider, model, and credential configuration. |
| Cursor, Windsurf, Zed | Use copied context packs or session handoffs until each editor's MCP bridge is explicitly configured and verified. | Editor settings mutation stays gated behind connector-specific dry-run, backup, apply, verify, rollback, Doctor repair, and Off cleanup. |
| Aider, Continue | Use copied packs/session handoffs by default; use MCP only when the local tool installation advertises compatible MCP consumption. | Multi-provider config and wrapper/env changes remain manual or sidecar-only until native promotion evidence exists. |
| Grok / xAI CLI, Qwen Code, Amazon Q Developer CLI | Use copied packs/session handoffs until each CLI's MCP/provider capability is detected and documented. | Account, credential, and model guardrails must be proven before native provider config writes are promoted. |

Do not treat MCP availability as permission to mutate provider/editor configuration. The Switchboard rollback inventory and connector readiness dossier remain the source of truth for config writes.

## Troubleshooting

- If the Mode Inspector says **Unknown**, install MCP and run the smoke check before relying on agent MCP handoffs.
- If it says **Needs attention**, copy the Doctor timeline; it includes `install_repo_memory_mcp`, `npm run check:repo-memory-mcp`, tool names, and the read-only safety boundary.
- If it says **Start required**, click **Start MCP** again. A previous app process verified the tools, but this app session has not.
- If `npm run check:repo-memory-mcp` fails, do not ask agents to use MCP context until the tool list and `repo_context_pack` smoke pass.
- If a repo was moved, deleted, or became stale, clear or refresh the Repo Intelligence index before using MCP output.

## Remaining Work

- Long-running service supervision beyond the stdio MCP app-session active marker.
- Connector-specific MCP bridge setup docs as native config mutation is promoted.
