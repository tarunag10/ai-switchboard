# Repo Intelligence Plan

Repo Intelligence is planned, not complete. The app currently tracks it as a future add-on, but there is no complete Graphy-style integration, graph builder, token-saving graph context layer, or UI workflow yet.

The goal is to make Mac AI Switchboard useful before an agent burns tokens reading the same files repeatedly: build a local repo graph, turn it into small task-specific context packs, and expose those packs to Claude Code, Codex, Gemini CLI, OpenCode, and future local agents without sending repository contents to a remote graph service.

## Recommended Tools To Evaluate

- **Graphy-style repo graph:** symbol and dependency graph view for files, imports, call paths, and test relationships.
- **tree-sitter parsers:** local language-aware extraction for TypeScript, JavaScript, Python, Rust, Swift, Markdown, and shell scripts.
- **Dependency and call-graph analyzers:** language-specific helpers such as TypeScript compiler APIs, Rust metadata, Python AST/import scanners, and package manifest readers.
- **repomix-style repo packaging:** bounded, ignore-aware repo summaries for agent context handoff.
- **MCP repo-memory adapters:** local command/API surface so multiple agents can request the same graph context without each one rescanning the repo.
- **Existing add-ons:** keep RTK, Ponytail, and MarkItDown separate but connected. RTK compresses command output, Ponytail nudges smaller implementation behavior, MarkItDown prepares documents, and Repo Intelligence should prepare codebase context.

## Build Slices

1. **Read-only local indexer**
   - Scan symbols, imports, routes, tests, package manifests, high-churn files, and ownership hints.
   - Respect `.gitignore`, common vendor folders, large binary files, generated outputs, and secret-like paths.
   - Store indexes under Mac AI Switchboard managed storage, not inside user repos.

2. **Graph storage and freshness**
   - Persist file hashes, parser versions, last indexed time, file count, symbol count, and skipped-file reasons.
   - Re-index incrementally when files change.
   - Mark stale indexes clearly instead of returning overconfident context.

3. **Token-saving context pack API**
   - Add a backend command and CLI entry point that accepts repo path, task text, optional file path, symbol, and target agent.
   - Return bounded packs with relevant files, related symbols, likely tests, risk notes, and exact local paths.
   - Include estimated tokens avoided by using the graph pack instead of full-file discovery.

4. **UI workflow**
   - Add a Repo Intelligence add-on card with install, index, re-index, pause, and remove-index controls.
   - Add repo picker, index health, last indexed time, file count, symbol count, skipped files, and estimated context saved.
   - Show generated context packs in the app so users can see what an agent received.

5. **Agent integrations**
   - Expose context packs through local CLI/MCP-style commands that Claude Code, Codex, Gemini CLI, OpenCode, and similar tools can call.
   - Keep the first version read-only. Any write, refactor, or auto-repair action must require an explicit user action.
   - Reuse Switchboard on/off behavior: disabling the feature stops indexing and removes routing/hooks without deleting the user's repo.

6. **Doctor and repair support**
   - Add checks for parser availability, index freshness, storage permissions, ignored path handling, and local API reachability.
   - Provide repair actions for rebuilding an index, clearing corrupt graph storage, and removing stale agent hooks.

7. **Safety and privacy verification**
   - Add tests for ignore handling, bounded output, stale-index detection, no network dependency, and no project-file mutation.
   - Add fixture repos for TypeScript, Python, Rust, and mixed-document projects.
   - Add a beta smoke test proving a repo can be indexed, queried, disabled, and cleaned up without modifying project files.

## Done Means

- The app can index a local repo into managed storage.
- The UI can show index status and generate a compact context pack.
- Supported agents can request that pack through a local interface.
- Turning the feature off stops hooks and indexing without touching project files.
- Tests prove the graph layer is local-only, bounded, and read-only by default.
