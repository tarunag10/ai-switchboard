# Repo Intelligence Plan

Repo Intelligence is planned, not complete. The app currently tracks it as a future add-on, but there is no complete Graphy-style integration, graph builder, token-saving graph context layer, or UI workflow yet.

## Recommended Build Path

1. Local repo indexer
   - Build a read-only scanner for symbols, imports, routes, tests, package manifests, and high-churn files.
   - Prefer proven local graph/index libraries where they fit, including Graphy-style code graphs, CodeGraph-style indexes, tree-sitter parsers, and MCP repo-memory adapters.
   - Store the index under Mac AI Switchboard managed storage, not inside user repos.

2. Graph context API
   - Add a backend command that returns compact context packs for a selected repo, file, symbol, or task.
   - Keep the output bounded and token-aware: top files, related symbols, likely tests, risk notes, and links to source paths.
   - Never send repo contents to a remote graph service.

3. UI workflow
   - Add a Repo Intelligence add-on card with install/index/re-index controls.
   - Add a repo picker, index health, last indexed time, file count, symbol count, and estimated context saved.
   - Show generated context packs before any agent uses them.

4. Agent integration
   - Expose context packs through local CLI/MCP-style commands that Claude Code, Codex, and future clients can call.
   - Keep write or auto-repair actions explicit. The first version should be read-only planning.

5. Safety and verification
   - Respect `.gitignore`, denylist secrets, and avoid indexing large binary/vendor folders.
   - Add tests for ignore handling, bounded output, stale-index detection, and no network dependency.
   - Add a beta smoke test proving a repo can be indexed, queried, and disabled without modifying project files.

## Done Means

- The app can index a local repo into managed storage.
- The UI can show index status and generate a compact context pack.
- Supported agents can request that pack through a local interface.
- Turning the feature off removes hooks and stops indexing.
- Tests prove the graph layer is local-only, bounded, and read-only by default.
