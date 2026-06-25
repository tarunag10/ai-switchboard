export interface PlannedAddon {
  id: string;
  name: string;
  statusLabel: string;
  description: string;
  bullets: string[];
}

export const plannedAddons: PlannedAddon[] = [
  {
    id: "repo_intelligence",
    name: "Repo Intelligence",
    statusLabel: "Planned",
    description:
      "Local repo graph memory layer for symbols, imports, routes, call paths, tests, and repeated context lookups. The first read-only context-pack foundation is now in place; graph parsing and app UI are next.",
    bullets: [
      "Foundation added: local file classification, token estimates, and bounded implementation, verification, and handoff context packs.",
      "Not complete yet: no full Graphy-style symbol graph, call graph, dependency graph, persistent index, or in-app context-pack workflow exists today.",
      "Recommended targets include Graphy-style code graphs, tree-sitter parsers, dependency/call-graph analyzers, repomix-style repo packaging, MCP repo-memory adapters.",
      "Local-first index stored on Mac, no remote service requirement.",
      "Read-only planning mode first; write auto-repair actions stay explicit.",
    ],
  },
  {
    id: "agent_connectors",
    name: "Agent Connectors",
    statusLabel: "Planned",
    description:
      "Future connector layer for popular coding CLIs and editor agents beyond Claude Code and Codex, including Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, and Goose.",
    bullets: [
      "Start with read-only detection so Switchboard can show installed tools without editing configs.",
      "Add reversible local provider/base-url adapters only after each tool has a stable config surface.",
      "Keep off-mode cleanup, backups, and Doctor repair actions consistent with Claude Code and Codex.",
      "Expose RTK and future Repo Intelligence context packs to agent-style tools where direct Headroom routing is not supported.",
    ],
  },
];

export function getPlannedAddon(id: string) {
  return plannedAddons.find((addon) => addon.id === id) ?? null;
}
