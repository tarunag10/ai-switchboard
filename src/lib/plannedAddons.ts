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
      "Future local repo graph and memory layer for symbols, imports, routes, call paths, tests, and repeated context lookups. It should reduce repeated file reads and help agents choose smaller, safer edits before spending tokens.",
    bullets: [
      "Not fully added yet: no complete Graphy-style integration, graph builder, token-saving graph context layer, or UI workflow exists today.",
      "Recommended targets include Graphy-style code graphs, tree-sitter parsers, dependency/call-graph analyzers, repomix-style repo packaging, and MCP repo-memory adapters.",
      "Local-first index stored on the Mac, with no remote service requirement.",
      "Read-only planning mode first; write and auto-repair actions stay explicit.",
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
      "Add reversible local provider/base-url adapters only when each tool has a stable config surface.",
      "Keep off-mode cleanup, backups, and Doctor repair actions consistent with Claude Code and Codex.",
      "Expose RTK and future Repo Intelligence context packs for agent-style tools where direct Headroom routing is not supported.",
    ],
  },
];

export function getPlannedAddon(id: string) {
  return plannedAddons.find((addon) => addon.id === id) ?? null;
}
