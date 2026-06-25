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
      "Future local repo graph and memory layer for symbols, imports, routes, call paths, and repeated context lookups. It should reduce repeated file reads and help agents choose smaller, safer edits before spending tokens.",
    bullets: [
      "Future adapter targets include Graphy-style code graphs, CodeGraph-style indexes, MCP repo memory.",
      "Local-first index stored on Mac, with no remote service requirement.",
      "Read-only planning mode first; write or auto-repair actions stay explicit.",
    ],
  },
];

export function getPlannedAddon(id: string) {
  return plannedAddons.find((addon) => addon.id === id) ?? null;
}
