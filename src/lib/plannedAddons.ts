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
    statusLabel: "Local tool",
    description:
      "Local repo graph memory layer for indexing a repository, estimating context tokens, and copying bounded implementation, verification, handoff, and agent-specific packs without sending repo contents to a remote graph service.",
    bullets: [
      "Available now: local file classification, token estimates, dependency hubs, path-based edges, import references, call references, reverse hubs, symbol summaries, and bounded context packs.",
      "Open the Repo Intelligence sidebar view to index a repo, review graph and savings signals, then copy Markdown or JSON handoffs.",
      "Sample preview stays non-copyable until a real local index exists, so users do not paste demo context into agents.",
      "Still planned: deeper AST-backed parsing, full call graph, persistent parser index, direct repo-memory MCP UI controls, and Doctor repair integration.",
      "Local-first index stored on Mac, with secret-like paths, generated outputs, and vendor folders excluded from default packs.",
      "Read-only by default; write or auto-repair actions remain explicit user actions.",
    ],
    healthChecks: [
      "Local index exists can be cleared without touching repository.",
      "Secret-like paths generated folders excluded context packs.",
      "Manifest includes implementation, verification, handoff packs estimated tokens avoided.",
      "Graph summary includes dependency hubs, path-based edges, import references, call references, and reverse dependency hubs.",
    ],
    savingsSources: [
      "Avoided full-repo scans by copying bounded context packs.",
      "Agent handoffs Claude Code, Codex, Gemini CLI, OpenCode, Qwen Code, Amazon Q, Cursor, Continue, Windsurf, Zed.",
      "Graph summary lets agents focus on entrypoints, tests, config hubs, dependency hubs.",
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
      "Add reversible local provider/base-url adapters only after tool stable config surface.",
      "Keep Off-mode cleanup, backups, and Doctor repair actions consistent with Claude Code and Codex.",
      "Expose RTK and Repo Intelligence context packs to agent-style tools where direct Headroom routing is not supported.",
    ],
    healthChecks: [
      "Detected tools stay read-only unless a managed adapter is explicitly supported.",
      "Every managed connector shows config surfaces, automation gates, manual workflow, and native config gate state.",
      "Doctor keeps native connector config tasks manual and excludes them from Repair all until restore coverage is proven.",
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
      "Token-optimized command-output layer for shell-heavy agents, stronger health checks, activity evidence, per-session savings attribution.",
    bullets: [
      "Keep RTK install, enable, disable, uninstall flows reversible through Doctor Off mode.",
      "Surface shell-profile hook status, managed binary path, recent activity evidence before claiming RTK savings.",
      "Treat RTK as safe fallback several active Codex goals when Headroom compression risk is high.",
    ],
    healthChecks: [
      "RTK binary is installed in managed app storage appears on expected shell path.",
      "Shell profile contains only Switchboard-owned managed RTK blocks.",
      "Recent RTK activity can be loaded from Addons card without exposing command secrets.",
      "Doctor offers Install RTK when Full optimization is degraded by missing command-output compression.",
    ],
    savingsSources: [
      "Command output summarized before it reaches Claude Code or terminal-driven agents.",
      "Savings calculator source breakdown attributes RTK separately Headroom and Repo Intelligence.",
      "RTK only mode keeps shell-output savings while bypassing Headroom request routing.",
    ],
    verificationCommand: "npm run smoke:preflight",
  },
  {
    id: "ponytail_hardening",
    name: "Ponytail Hardening",
    statusLabel: "Ready to harden",
    description:
      "Agent behavior nudge smaller, more focused implementation slices, tracked as local add-on instead hidden prompt drift.",
    bullets: [
      "Keep Ponytail opt-in so users can choose smaller-change nudges per workflow.",
      "Show whether local Ponytail guidance installed, enabled, scoped to supported clients.",
      "Measure benefit through smaller context packs, fewer broad rewrites, easier verification handoffs.",
    ],
    healthChecks: [
      "Managed Ponytail guidance exists only in Switchboard-owned config blocks.",
      "Disable and Off mode remove Ponytail guidance without touching user-authored instructions.",
      "Doctor keeps Ponytail failures separate required Headroom runtime failures.",
    ],
    savingsSources: [
      "Smaller implementation slices reduce repeated broad file reads.",
      "Fewer unrelated rewrites lower verification review context.",
      "Savings calculator shows inferred Ponytail row from bounded-versus-unbounded change template delta while add-on enabled.",
    ],
    verificationCommand:
      "npm run test:frontend -- src/lib/plannedAddons.test.ts --pool=threads",
  },
  {
    id: "markitdown_hardening",
    name: "MarkItDown Hardening",
    statusLabel: "Ready to harden",
    description:
      "Local document-to-Markdown preprocessing add-on for PDFs, Office files, reference docs before sending compact context to coding agents.",
    bullets: [
      "Keep document conversion local-first opt-in from Addons.",
      "Add smoke evidence that MarkItDown runtime dependencies importable before exposing conversion workflows.",
      "Attribute savings smaller Markdown extracts instead repeatedly attaching bulky source documents.",
    ],
    healthChecks: [
      "MarkItDown imports inside managed runtime after install or upgrade.",
      "Conversion failures stay warn-only and never block core Headroom runtime boot.",
      "Converted outputs exclude private source files unless user explicitly copies them.",
    ],
    savingsSources: [
      "Markdown extracts avoid repeated binary document uploads.",
      "Prepared reference snippets pair with Repo Intelligence packs for smaller task context.",
      "Savings calculator shows an estimated MarkItDown row from extract-versus-full-document template delta plus managed smoke evidence, separate from RTK shell-output and Headroom compression savings.",
    ],
    verificationCommand: "npm run smoke:preflight",
  },
];

export function getPlannedAddon(id: string) {
  return plannedAddons.find((addon) => addon.id === id) ?? null;
}
