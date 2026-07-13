# Repo Memory MCP

Repo Memory MCP is the read-only agent-consumption surface for Repo Intelligence. It lets supported local coding agents request bounded repo context without rescanning the project or copying whole files into every session.

Goose is managed for this bridge and for its separately allowlisted native endpoint fields: Switchboard manages the read-only Repo Memory MCP descriptor, smoke-checks it before handoff, and leaves Goose credentials, account state, and model selection manual.

## Current Status

- Transport: stdio MCP served by `scripts/repo-intelligence.mjs --mcp-serve`.
- Service descriptor: the app writes `repo-memory.json` with the managed stdio
  command, descriptor path, repo-memory script path, descriptor presence,
  script presence, Node command availability, read-only flag, healthy flag,
  issue codes, and app-managed ownership so Mode Inspector can show durable
  service wiring separately from active supervision. If descriptor evidence is
  not app-managed, not read-only, not runnable, or reports
  `descriptor_missing`, `script_missing`, or `node_missing`, Mode Inspector
  downgrades Repo Memory MCP to **Needs attention** before agent handoffs.
- Prepare action: **Prepare MCP** in the Mode Inspector, backed by `install_repo_memory_mcp` followed by `start_repo_memory_mcp`. It installs the app-managed config, runs the read-only smoke contract, and records the current app process before marking MCP active.
- Session controls: **Start MCP** and **Stop MCP** in the Mode Inspector, backed by `start_repo_memory_mcp` and `stop_repo_memory_mcp`. Start re-runs the read-only smoke contract for configured or failed states; these controls do not claim a separate background daemon is running.
- Relaunch recovery: when the previous app process had already verified an app-managed read-only Repo Memory MCP descriptor, the next app launch automatically re-runs the smoke check during runtime refresh before advertising MCP as active again. Unsafe, missing, or never-prepared descriptors still require **Prepare MCP**.
- Supervision: while the same app process owns an active Repo Memory MCP session, runtime status polling periodically re-runs the read-only smoke check. A failed recheck downgrades the lifecycle to **Smoke failed** so agents do not rely on stale MCP handoffs. Runtime status also checks the managed descriptor, script, Node command evidence, healthy flag, and issue codes before telling agents the MCP service is safe to use; broken service evidence downgrades to **Needs attention** even if a previous smoke check passed.
- Optional terminal verification: `npm run check:repo-memory-mcp`.
- One-click evidence verification: the app's **Run local evidence** flow now runs `npm run smoke:repo-memory-mcp:local` as a separate local-only report row. It writes `dist/local-repo-memory-mcp-validation-summary.md` and `.json` with read-only tool, bounded response, seeded secret exclusion, app-managed descriptor recheck, and connector bridge recipe evidence.
- Tools: `repo_context_pack`, `repo_symbol_lookup`, and `repo_dependents_of`.
  Switchboard-compatible aliases are also exposed:
  `switchboard.list_context_packs`, `switchboard.build_context_pack`, and
  `switchboard.get_repo_graph_summary`.
- Safety: read-only tools, secret-like paths excluded, generated/vendor paths skipped, and pack output bounded by Repo Intelligence budgets.

## Agent Consumption

Use Repo Memory MCP after indexing a real local repo from the Repo Intelligence view. Agents should treat MCP output as context only; it is not an approval to write files, mutate config, or bypass Switchboard safety gates.

Recommended agent flow:

1. Index or refresh the repo in AI Switchboard for Mac.
2. Click **Prepare MCP** in the Mode Inspector if Repo Memory MCP is not configured; the app installs it, starts it, and runs the same read-only smoke before marking it active.
3. Relaunch AI Switchboard for Mac normally; previously verified app-managed read-only MCP descriptors are smoke-checked again automatically. Use **Start MCP** only when you want to retry immediately after a failed or stale check.
4. Run `npm run check:repo-memory-mcp` manually only when you want extra terminal proof of the read-only tool contract.
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

Claude Code, Codex, Gemini CLI, OpenCode, Aider, Goose, Cursor, Continue, Grok / xAI CLI, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI should consume Repo Intelligence as read-only context. Managed connector readiness dossiers may describe config paths and rollback strategy; Goose and Grok/xAI native adapters write only their documented endpoint fields, while unsupported provider/editor mutation stays gated behind explicit dry-run, backup, apply, verify, rollback, Doctor repair, and Off cleanup evidence.

Goose and other MCP-aware tools should keep Repo Memory MCP separate from provider routing. MCP context can be enabled independently; Goose's allowlisted endpoint fields can be managed while credentials, account state, and model selection remain manual.

### Connector Consumption Matrix

| Connector | Preferred Repo Memory path | Boundary |
| --- | --- | --- |
| Claude Code | Use Repo Memory MCP tools after **Prepare MCP** marks the app-managed smoke check active; run `npm run check:repo-memory-mcp` only for extra terminal proof. | MCP context is read-only and separate from Claude Code shell routing or Headroom engine mode. |
| Codex | Prefer Start Agent Session or `repo:intelligence --session`; use MCP only when the active Codex environment can call configured MCP tools. | MCP context does not change Codex provider config, model selection, or `OPENAI_BASE_URL` routing. |
| Gemini CLI | Use Start Agent Session handoffs first; use MCP only when Gemini is running in an MCP-capable wrapper or environment. | Gemini base-url/env routing remains managed separately and must keep its backup/rollback evidence. |
| OpenCode | Use Start Agent Session handoffs first; use MCP only when OpenCode's configured runtime can call MCP tools. | OpenCode provider config routing remains guarded by native backup, verify, rollback, and Off cleanup. |
| Goose | Use Repo Memory MCP as the preferred read-only context path once configured. | Keep MCP server setup separate from Goose provider, model, and credential configuration. |
| Cursor | Use copied context packs or session handoffs until Cursor's MCP bridge is explicitly configured and verified. | Cursor editor settings mutation stays gated behind connector-specific dry-run, backup, apply, verify, rollback, Doctor repair, and Off cleanup. |
| Windsurf, Zed | Use copied context packs or session handoffs until each editor's MCP bridge is explicitly configured and verified. | Repo Memory MCP setup is separate from managed Windsurf editor-settings routing and Zed assistant-settings routing; those routes remain governed by the connector lifecycle and rollback inventory. Cursor, Windsurf, Zed bridge setup is never provider-routing permission. |
| Aider, Continue | Use copied packs/session handoffs by default; use MCP only when the local tool installation advertises compatible MCP consumption. | Multi-provider config and wrapper/env changes remain manual or sidecar-only until native promotion evidence exists. |
| Grok / xAI CLI, Qwen Code, Amazon Q Developer CLI | Use copied packs/session handoffs until each CLI's MCP/provider capability is detected and documented. | Account, credential, and model guardrails must be proven before native provider config writes are promoted. |

Do not treat MCP availability as permission to mutate provider/editor configuration. The Switchboard rollback inventory and connector readiness dossier remain the source of truth for config writes.

### Bridge Setup Recipes

Use these recipes only after **Prepare MCP** reports an app-managed, read-only, smoke-tested descriptor. They are consumption guides for agents that already support MCP; they are not provider setup instructions.

#### Claude Code

- Preferred path: let AI Switchboard for Mac install the app-managed `repo-memory` descriptor, then restart Claude Code so it reloads MCP servers.
- Verification: open a fresh Claude Code session and ask it to call `repo_context_pack` for a small implementation pack. If the tool is unavailable, run `npm run check:repo-memory-mcp` and use **Prepare MCP** again.
- Boundary: do not edit Claude model, subscription, shell routing, or Headroom proxy settings while testing Repo Memory MCP.

#### Goose

- Preferred path: register the app-managed `repo-memory.json` descriptor as a read-only MCP server in the Goose MCP surface, or paste the same command/args from Mode Inspector when Goose asks for a stdio server.
- Verification: ask Goose for `repo_symbol_lookup` on a known symbol before requesting broader context.
- Boundary: Goose provider/model credentials, account state, and model selection remain manual. Repo Memory MCP only supplies repository context.

#### Cursor, Windsurf, and Zed

- Preferred path: keep using Start Agent Session and copied context packs until the editor's MCP bridge UI is explicitly configured by the user.
- Verification: once the editor advertises MCP tools, request `repo_context_pack` and compare the pack title/path against the current Repo Intelligence index.
- Boundary: Cursor settings writes remain blocked until Switchboard has connector-specific lifecycle evidence. Windsurf and Zed provider-routing writes are managed separately; MCP bridge setup must not be treated as permission to alter routing, model, or account settings.

#### Continue and Aider

- Preferred path: copied packs/session handoffs by default. Use Repo Memory MCP only when the local Continue or Aider installation documents stdio MCP tool consumption.
- Verification: request `repo_dependents_of` for a small file before asking for a task pack.
- Boundary: multi-provider config and wrapper/env changes remain manual or sidecar-only.

#### Gemini CLI, OpenCode, Grok / xAI CLI, Qwen Code, and Amazon Q Developer CLI

- Preferred path: Start Agent Session handoffs first. Use MCP only in a wrapper or environment that already exposes MCP tools to the CLI.
- Verification: request a bounded `repo_context_pack`; if the CLI cannot call MCP, fall back to `npm run repo:intelligence -- <repo-path> --session --agent <id> --task implementation --format markdown`.
- Boundary: account, credential, model, and provider-routing guardrails must be proven before native provider config writes are promoted.

## Troubleshooting

- If the Mode Inspector says **Unknown**, use **Prepare MCP** before relying on agent MCP handoffs.
- If it says **Needs attention**, click **Prepare MCP** or copy the Doctor timeline; it includes `install_repo_memory_mcp`, `start_repo_memory_mcp`, `npm run check:repo-memory-mcp`, tool names, and the read-only safety boundary.
- If descriptor detail says **not app-managed** or **not read-only**, click
  **Prepare MCP**. Do not ask agents to use MCP context until the app restores
  the app-managed read-only descriptor and the smoke check passes.
- If it says **Verifying**, AI Switchboard for Mac is re-running the read-only smoke check for a previous app session. Click **Start MCP** only if you want to retry immediately.
- If `npm run check:repo-memory-mcp` fails, do not ask agents to use MCP context until the tool list and `repo_context_pack` smoke pass.
- If a repo was moved, deleted, or became stale, clear or refresh the Repo Intelligence index before using MCP output.

## Remaining Work

- Signed installed-app relaunch survival evidence for the app-supervised stdio service as native config mutation is promoted. Local one-click smoke evidence is recorded separately by `npm run smoke:repo-memory-mcp:local`.
- Connector-specific MCP bridge setup docs beyond Goose as native config mutation is promoted.
