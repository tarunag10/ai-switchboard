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
      "Local-first index stored on Mac, with no remote service requirement.",
      "Read-only planning mode first; write and auto-repair actions stay explicit.",
    ],
  },
];

export function getPlannedAddon(id: string) {
  return plannedAddons.find((addon) => addon.id === id) ?? null;
}
