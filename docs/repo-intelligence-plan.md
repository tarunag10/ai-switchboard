# Repo Intelligence Plan

Repo Intelligence is underway, not complete. The app now has a read-only foundation for local file classification, rough token estimation, bounded implementation, verification, and handoff context packs. It does not yet have a complete Graphy-style symbol graph, call graph, dependency graph, or agent-facing context-pack API.

The goal is to make Mac AI Switchboard useful before an agent burns tokens reading the same files repeatedly: build a local repo graph, turn it into small task-specific context packs, and expose those packs to Claude Code, Codex, Gemini CLI, OpenCode, and future local agents without sending repository contents to a remote graph service.

## Recommended Tools To Evaluate

- **Graphy-style repo graph:** symbol dependency graph view for files, imports, call paths, and test relationships.
- **tree-sitter parsers:** local language-aware extraction for TypeScript, JavaScript, Python, Rust, Swift, Markdown, and shell scripts.
- **Dependency call-graph analyzers:** language-specific helpers such as TypeScript compiler APIs, Rust metadata, Python AST/import scanners, and package manifest readers.
- **repomix-style repo packaging:** bounded, ignore-aware repo summaries for agent context handoff.
- **MCP repo-memory adapters:** local command/API surface so multiple agents request the same graph context without each one rescanning the repo.
- **Existing add-ons:** keep RTK, Ponytail, and MarkItDown separate but connected. RTK compresses command output, Ponytail nudges smaller implementation behavior, MarkItDown prepares documents, and Repo Intelligence prepares codebase context.

## Build Slices

1. **Read-only local indexer**
   - CLI foundation: run `npm run repo:intelligence -- <repo-path>` to produce local file roles, token estimates, and bounded context packs.
   - App workflow: enter a local repo path in the Repo Intelligence add-on card and click **Index**.
   - Persistence: the latest successful summary is saved under Mac AI Switchboard managed config storage, not inside the user repo.
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
   - Expose context packs through local CLI/MCP-style commands that Claude Code, Codex, Gemini CLI, OpenCode, and similar tools can call.
   - Keep first version read-only. Any write, refactor, or auto-repair action must require explicit user action.
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
