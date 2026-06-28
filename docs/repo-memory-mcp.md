# Repo Memory MCP

Repo Memory MCP is the read-only agent-consumption surface for Repo Intelligence. It lets supported local coding agents request bounded repo context without rescanning the project or copying whole files into every session.

## Current Status

- Transport: stdio MCP served by `scripts/repo-intelligence.mjs --mcp-serve`.
- Install action: **Install MCP** in the Mode Inspector, backed by `install_repo_memory_mcp`.
- Session controls: **Start MCP** and **Stop MCP** in the Mode Inspector, backed by `start_repo_memory_mcp` and `stop_repo_memory_mcp`. These controls persist app-session active state for the stdio MCP path; they do not claim a separate background daemon is running.
- Verification: `npm run check:repo-memory-mcp`.
- Tools: `repo_context_pack`, `repo_symbol_lookup`, and `repo_dependents_of`.
- Safety: read-only tools, secret-like paths excluded, generated/vendor paths skipped, and pack output bounded by Repo Intelligence budgets.

## Agent Consumption

Use Repo Memory MCP after indexing a real local repo from the Repo Intelligence view. Agents should treat MCP output as context only; it is not an approval to write files, mutate config, or bypass Switchboard safety gates.

Recommended agent flow:

1. Index or refresh the repo in Mac AI Switchboard.
2. Install Repo Memory MCP from the Mode Inspector if it is not configured.
3. Start Repo Memory MCP from the Mode Inspector when the agent session should be allowed to consume app-managed repo context.
4. Run `npm run check:repo-memory-mcp` to verify the read-only tool contract.
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

## Troubleshooting

- If the Mode Inspector says **Unknown**, install MCP and run the smoke check before relying on agent MCP handoffs.
- If it says **Needs attention**, copy the Doctor timeline; it includes `install_repo_memory_mcp`, `npm run check:repo-memory-mcp`, tool names, and the read-only safety boundary.
- If `npm run check:repo-memory-mcp` fails, do not ask agents to use MCP context until the tool list and `repo_context_pack` smoke pass.
- If a repo was moved, deleted, or became stale, clear or refresh the Repo Intelligence index before using MCP output.

## Remaining Work

- Long-running service supervision beyond the stdio MCP app-session active marker.
- Doctor repair integration for failed or missing repo-memory MCP setup.
- Broader connector-specific docs as native config mutation is promoted.
