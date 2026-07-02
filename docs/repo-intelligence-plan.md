# Repo Intelligence Plan

Repo Intelligence is underway, not complete. The app now has a read-only foundation for local file classification, rough token estimation, bounded implementation, verification, and handoff context packs. It now includes dependency hubs, path-based import/dependency edges, content-derived import references, package-dependency edges from external imports to `package.json`, lightweight call references, reverse dependency hubs, a bounded symbol graph, a read-only local API manifest in the local graph summary, and a repo-memory MCP stdio transport with app-managed install/start/stop smoke coverage. It does not yet complete a full AST call graph, persistent parser index, long-running MCP daemon supervision, or writable agent automation.

The goal is to make Mac AI Switchboard useful before an agent burns tokens reading the same files repeatedly: build a local repo graph, turn it into small task-specific context packs, and expose those packs to Claude Code, Codex, Gemini CLI, OpenCode, Qwen Code, Amazon Q Developer CLI, Cursor, Continue, Windsurf, Zed AI, and future local agents without sending repository contents to a remote graph service.

## Recommended Tools To Evaluate

- **Graphy-style repo graph:** Symbol/call graph view for files, call paths, dependency hubs, and richer test relationships. Current implementation already includes bounded path-based dependency edges, content-derived import references, Python local import-reference edges, lightweight call references, reverse dependency hubs, and symbol extraction for TypeScript, JavaScript, Rust, and Python source.
- **tree-sitter parsers:** local language-aware extraction for TypeScript, JavaScript, Python, Rust, Swift, Markdown, and shell scripts.
- **Dependency call-graph analyzers:** language-specific helpers such as TypeScript compiler APIs, Rust metadata, Python AST/import scanners, shell workflow parsers, Markdown heading scanners, stylesheet/HTML asset parsers, and deeper package manifest readers. Current `path-graph-v8` support links visible test/source relationships, Markdown headings into symbols with parent hierarchy, shell script invocations to local scripts, `package.json` scripts to local shell scripts and script-to-script chains, Python absolute and relative local imports to indexed modules, stylesheet `@import`/`url(...)` references, HTML script/style/image links, and TypeScript, JavaScript, and React external imports back to `package.json`.
- **repomix-style repo packaging:** bounded, ignore-aware repo summaries for agent context handoff.
- **MCP repo-memory adapters:** local command/API surface so multiple agents request the same graph context without each one rescanning the repo.
- **Existing add-ons:** keep RTK, Ponytail, and MarkItDown separate but connected. RTK compresses command output, Ponytail nudges smaller implementation behavior, MarkItDown prepares documents, and Repo Intelligence prepares codebase context.

## Build Slices

1. **Read-only local indexer**
   - CLI foundation: run `npm run repo:intelligence -- <repo-path>` to produce local file roles, token estimates, and bounded context packs.

- App workflow: open the dedicated Repo Intelligence sidebar view, enter a local repo path, and click **Index**.
  - Persistence: the latest successful summary is saved under Mac AI Switchboard managed config storage, not inside the user repo.
  - Handoff: click **Copy pack** to copy a bounded Markdown summary for another local coding agent without copying file contents.
  - Safety: respect common vendor folders, generated outputs, large files, and secret-like paths.

2. **Graph storage freshness**
   - Persist file hashes, parser versions, last indexed time, file count, symbol count, symbols, likely tests, risk notes, and exact local paths.
   - Include estimated tokens avoided by using graph packs instead of full-file discovery.
   - Mark stale indexes clearly instead of returning overconfident context.
3. **UI workflow**
   - Add repo picker, index health, last indexed time, file count, symbol count, skipped files, and estimated context saved.
   - Show generated context packs in app so users can see what an agent received.
   - Add re-index, pause, and remove-index controls.
4. **Agent integrations**
  - Expose context packs through local CLI and repo-memory MCP commands that Claude Code, Codex, Gemini CLI, OpenCode, and similar tools can call. Current CLI contract includes `npm run repo:intelligence -- <repo> --manifest` for pack discovery, read-only `apiQueries`, read-only `agentRecipes`, and per-agent `agentSessionRecipes` with exact `--session` commands for Claude Code, Codex, Gemini CLI, OpenCode, Aider, Goose, Qwen Code, Amazon Q Developer CLI, Cursor, Continue, Windsurf, and Zed AI. It also includes `--list-api` for local API query command names, `--list-agents` for supported handoff ids, `--pack <id> --format markdown` for task-specific handoff, `--agent <claude|codex|gemini|opencode|aider|goose|cursor|continue|grok|qwen|amazonq|windsurf|zed> --format markdown` for ready-to-paste bounded tool handoffs, `--agent <id> --format json` for machine-readable `mac_ai_switchboard.repo_agent_handoff` payloads, and `--session --agent <id> --task <implementation|verification|handoff|risk_review|release_handoff>` for `mac_ai_switchboard.agent_session_preparation` output with freshness, copy status, recommended mode, selected handoff, and managed connector readiness when applicable.
  - Connector-consumption docs live in [Repo Memory MCP](repo-memory-mcp.md), including the Mode Inspector install action, smoke command, read-only tool names, and MCP-aware connector boundary.
  - `get_agent_handoff` and matching CLI handoff exports include managed connector config readiness when the target is Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, or Zed AI. The readiness block carries the next gate, evidence requirements, config path strategy, account caveat, and rollback strategy while provider-specific native config mutation stays gated.
  - App/API query names exposed in manifests: `get_repo_manifest`, `get_repo_pack`, `get_agent_handoff`, `get_index_freshness`, `clear_repo_index`, `search_repo_intelligence_symbols`, and `get_repo_intelligence_dependents`. Each query is read-only except `clear_repo_index`, which only clears Switchboard's saved managed index metadata and never mutates the user repository.
   - Keep first version read-only. Any write, refactor, or auto-repair action must require explicit user action.

- Reuse the managed connector readiness contract for Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI: identify config surfaces, document manual-safe workflows, show promoted routing evidence for Gemini/OpenCode/Windsurf/Zed, show Goose's managed Repo Memory MCP bridge separately from provider routing, keep sidecar/readiness lifecycle coverage visible for unpromoted connectors, and block remaining native provider/editor config mutation behind backup/restore/off-mode gates.
  - Reuse Switchboard on/off behavior: disabling the feature stops indexing and removes routing/hooks without deleting the user's repo.

5. **Doctor repair support**
   - Check parser availability, index freshness, storage permissions, ignored path handling, and local API reachability.
   - Provide repair actions for rebuilding index, clearing corrupt graph storage, and removing stale agent hooks.
6. **Safety privacy verification**
   - Test ignore handling, bounded output, stale-index detection, no network dependency, and no project-file mutation.
   - Add fixture repos for TypeScript, Python, Rust, and mixed-document projects.
   - Add beta smoke test proving a repo can be indexed, queried, disabled, and cleaned up without modifying project files.

## Done Means

- App indexes a local repo into managed storage.
- UI shows index status and generates compact context packs.
- Supported agents request packs through a local interface.
- Turning the feature off stops hooks and indexing without touching project files.
- Tests prove the graph layer is local-only, bounded, and read-only by default.
