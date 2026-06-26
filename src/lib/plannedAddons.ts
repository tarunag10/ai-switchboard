export interface PlannedAddon {
  id: string;
  name: string;
  statusLabel: string;
  description: string;
  bullets: string[];
  healthChecks: string[];
  savingsSources: string[];
  verificationCommand?: string;
}

export const plannedAddons: PlannedAddon[] = [
  {
    id: "repo_intelligence",
    name: "Repo Intelligence",
    statusLabel: "Planned",
    description:
      "Local repo graph memory layer for symbols, imports, routes, call paths, tests, and repeated context lookups. The read-only context-pack foundation and path-based relationship graph are now in place; AST-backed parsing is next.",
    bullets: [
      "Foundation added: local file classification, token estimates, bounded implementation, verification, and handoff context packs.",
      "Now available: dependency hubs, path-based import/dependency edges, and reverse dependency hubs in local graph summaries.",
      "Not complete yet: full Graphy-style symbol graph, call graph, persistent parser index, richer in-app workflow, and MCP repo-memory API are still planned.",
      "Recommended targets include Graphy-style code graphs, tree-sitter parsers, dependency/call-graph analyzers, repomix-style repo packaging, and MCP repo-memory adapters.",
      "Local-first index stored on Mac, no remote service requirement.",
      "Read-only planning mode first; write auto-repair actions stay explicit.",
    ],
    healthChecks: [
      "Local index exists and can be cleared without touching the repository.",
      "Secret-like paths and generated folders are excluded from context packs.",
      "Manifest includes implementation, verification, and handoff packs with estimated tokens avoided.",
      "Graph summary includes dependency hubs, path-based edges, and reverse dependency hubs.",
    ],
    savingsSources: [
      "Avoided full-repo scans by copying bounded context packs.",
      "Agent handoffs for Claude Code, Codex, Gemini CLI, OpenCode, Qwen Code, Amazon Q, Cursor, Continue, Windsurf, and Zed.",
      "Graph summary lets agents focus on entrypoints, tests, config hubs, and dependency hubs.",
    ],
    verificationCommand: "npm run repo:intelligence -- . --manifest",
  },
  {
    id: "agent_connectors",
    name: "Agent Connectors",
    statusLabel: "Planned",
    description:
      "Future connector layer for popular coding CLIs and editor agents beyond Claude Code and Codex, including Gemini CLI, OpenCode, Cursor, Grok / xAI CLI, Aider, Continue, Goose, Qwen Code, Amazon Q Developer CLI, Windsurf, and Zed AI.",
    bullets: [
      "Start with read-only detection so Switchboard can show installed tools without editing configs.",
      "Add reversible local provider/base-url adapters only after each tool has a stable config surface.",
      "Keep off-mode cleanup, backups, and Doctor repair actions consistent with Claude Code and Codex.",
      "Expose RTK and Repo Intelligence context packs to agent-style tools where direct Headroom routing is not supported.",
    ],
    healthChecks: [
      "Detected tools stay read-only unless a managed adapter is explicitly supported.",
      "Every planned connector shows config surfaces, automation gates, and manual workflow.",
      "Doctor keeps planned connector tasks manual and excludes them from Repair all.",
      "Off mode must remove only Switchboard-owned changes before any future adapter writes config.",
    ],
    savingsSources: [
      "RTK-only shell-output savings for tools without safe LLM routing.",
      "Repo Intelligence handoff packs for copy-only agents and editors.",
      "Manual provider routing until backup, restore, and account/model compatibility checks are proven.",
    ],
    verificationCommand: "npm run smoke:preflight",
  },

{
id: "rtk_hardening",
name: "RTK Hardening",
statusLabel: "Ready to harden",
description:
"Token-optimized command-output layer for shell-heavy agents, with stronger health checks, activity evidence, and per-session savings attribution.",
bullets: [
"Keep RTK install, enable, disable, and uninstall flows reversible through Doctor and Off mode.",
"Surface shell-profile hook status, managed binary path, and recent activity evidence before claiming RTK savings.",
"Treat RTK as the safe fallback for several active Codex goals when Headroom compression risk is high.",
],
healthChecks: [
"RTK binary is installed in managed app storage and appears on the expected shell path.",
"Shell profile contains only Switchboard-owned managed RTK blocks.",
"Recent RTK activity can be loaded from the Addons card without exposing command secrets.",
"Doctor offers Install RTK when Full optimization is degraded by missing command-output compression.",
],
savingsSources: [
"Command output summarized before it reaches Claude Code or terminal-driven agents.",
"Savings calculator source breakdown attributes RTK separately from Headroom and Repo Intelligence.",
"RTK only mode keeps shell-output savings while bypassing Headroom request routing.",
],
verificationCommand: "npm run smoke:preflight",
},

{
id: "ponytail_hardening",
name: "Ponytail Hardening",
statusLabel: "Ready to harden",
description:
"Agent behavior nudge for smaller, more focused implementation slices, tracked as a local add-on instead of hidden prompt drift.",
bullets: [
"Keep Ponytail opt-in so users can choose smaller-change nudges per workflow.",
"Show whether local Ponytail guidance is installed, enabled, and scoped to supported clients.",
"Measure benefit through smaller context packs, fewer broad rewrites, and easier verification handoffs.",
],
healthChecks: [
"Managed Ponytail guidance exists only in Switchboard-owned config blocks.",
"Disable and Off mode remove Ponytail guidance without touching user-authored instructions.",
"Doctor keeps Ponytail failures separate from required Headroom runtime failures.",
],
savingsSources: [
"Smaller implementation slices reduce repeated broad file reads.",
"Fewer unrelated rewrites lower verification and review context.",
"Repo Intelligence handoffs can pair with Ponytail to keep agent work bounded.",
],
verificationCommand: "npm run test:frontend -- src/lib/plannedAddons.test.ts --pool=threads",
},

{
id: "markitdown_hardening",
name: "MarkItDown Hardening",
statusLabel: "Ready to harden",
description:
"Local document-to-Markdown preprocessing add-on for PDFs, Office files, and reference docs before sending compact context to coding agents.",
bullets: [
"Keep document conversion local-first and opt-in from Addons.",
"Add smoke evidence for importable MarkItDown runtime dependencies before exposing conversion workflows.",
"Attribute savings to smaller Markdown extracts instead of repeatedly attaching bulky source documents.",
],
healthChecks: [
"MarkItDown imports inside the managed runtime after install or upgrade.",
"Conversion failures stay warn-only and never block core Headroom runtime boot.",
"Converted outputs exclude private source files unless user explicitly copies them.",
],
savingsSources: [
"Markdown extracts avoid repeated binary document uploads.",
"Prepared reference snippets pair with Repo Intelligence packs for smaller task context.",
"Document preprocessing savings remain separate from RTK shell-output and Headroom compression savings.",
],
verificationCommand: "npm run smoke:preflight",
},
];

export function getPlannedAddon(id: string) {
  return plannedAddons.find((addon) => addon.id === id) ?? null;
}
