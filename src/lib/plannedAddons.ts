import type { ManagedTool, RuntimeStatus } from "./types";

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

export type AddonHealthTone = "healthy" | "warning" | "offline";
export type AddonHealthId = "headroom_engine" | "rtk" | "markitdown" | "ponytail";

export interface AddonHealthCard {
  id: AddonHealthId;
  name: string;
  statusLabel: string;
  tone: AddonHealthTone;
  detail: string;
  evidence: string[];
  nextAction: string;
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

function toolById(tools: ManagedTool[], id: string) {
  return tools.find((tool) => tool.id === id) ?? null;
}

function managedToolHealth(
  tool: ManagedTool | null,
  id: "markitdown" | "ponytail",
  name: string,
  installedAction: string,
): AddonHealthCard {
  if (!tool || tool.status === "not_installed") {
    return {
      id,
      name,
      statusLabel: "Not installed",
      tone: "offline",
      detail: `${name} is not installed in managed app storage yet.`,
      evidence: ["No managed tool record is healthy for this add-on."],
      nextAction: "Install from this Addons page.",
    };
  }

  if (tool.status === "degraded") {
    return {
      id,
      name,
      statusLabel: "Needs attention",
      tone: "warning",
      detail: `${name} is installed but its managed health check is degraded.`,
      evidence: [
        `Tool status: ${tool.status}.`,
        tool.version ? `Version: ${tool.version}.` : "Version is not reported.",
      ],
      nextAction: installedAction,
    };
  }

  if (!tool.enabled) {
    return {
      id,
      name,
      statusLabel: "Disabled",
      tone: "warning",
      detail: `${name} is installed but disabled, so it is not contributing savings right now.`,
      evidence: [
        `Tool status: ${tool.status}.`,
        tool.version ? `Version: ${tool.version}.` : "Version is not reported.",
      ],
      nextAction: "Enable it from this Addons page or leave it off intentionally.",
    };
  }

  return {
    id,
    name,
    statusLabel: "Healthy",
    tone: "healthy",
    detail: `${name} is installed and enabled.`,
    evidence: [
      `Tool status: ${tool.status}.`,
      tool.version ? `Version: ${tool.version}.` : "Version is not reported.",
    ],
    nextAction: "No action needed.",
  };
}

export function buildAddonHealthCards(
  runtimeStatus: RuntimeStatus | null | undefined,
  tools: ManagedTool[] = [],
): AddonHealthCard[] {
  const headroomHealthy =
    runtimeStatus?.installed === true &&
    runtimeStatus.running === true &&
    runtimeStatus.proxyReachable === true &&
    runtimeStatus.paused !== true &&
    runtimeStatus.autoPaused !== true;
  const headroomCard: AddonHealthCard =
    runtimeStatus === null || runtimeStatus === undefined
      ? {
          id: "headroom_engine",
          name: "Headroom engine",
          statusLabel: "Checking",
          tone: "warning",
          detail: "Runtime status has not loaded yet.",
          evidence: ["Waiting for the backend runtime probe."],
          nextAction: "Refresh runtime status or run Doctor.",
        }
      : headroomHealthy
        ? {
            id: "headroom_engine",
            name: "Headroom engine",
            statusLabel: "Healthy",
            tone: "healthy",
            detail: "The local runtime is running and its proxy is reachable.",
            evidence: [
              runtimeStatus.proxyBindAddress
                ? `Proxy listener: ${runtimeStatus.proxyBindAddress}.`
                : "Proxy listener is reachable.",
              runtimeStatus.backendStatus?.reachable
                ? `Backend port: ${runtimeStatus.backendStatus.port}.`
                : "Backend status did not report a port.",
            ],
            nextAction: "No action needed.",
          }
        : {
            id: "headroom_engine",
            name: "Headroom engine",
            statusLabel: runtimeStatus.paused ? "Paused" : "Needs attention",
            tone: "warning",
            detail:
              runtimeStatus.installed === false
                ? "The local runtime is not installed."
                : runtimeStatus.proxyReachable === false
                  ? "The runtime is not reachable through the local proxy."
                  : "The runtime is not in a healthy running state.",
            evidence: [
              `Installed: ${runtimeStatus.installed ? "yes" : "no"}.`,
              `Running: ${runtimeStatus.running ? "yes" : "no"}.`,
              `Proxy reachable: ${runtimeStatus.proxyReachable ? "yes" : "no"}.`,
            ],
            nextAction: "Use Start runtime or run Doctor from Home.",
          };

  const rtk = runtimeStatus?.rtk;
  const rtkCard: AddonHealthCard =
    rtk?.installed && rtk.enabled && rtk.pathConfigured && rtk.hookConfigured
      ? {
          id: "rtk",
          name: "RTK",
          statusLabel: "Healthy",
          tone: "healthy",
          detail: "RTK is installed, enabled, and wired into the managed shell path and hook.",
          evidence: [
            `Commands recorded: ${Math.max(0, rtk.totalCommands ?? 0).toLocaleString()}.`,
            `Tokens saved: ${Math.max(0, rtk.totalSaved ?? 0).toLocaleString()}.`,
          ],
          nextAction: "No action needed.",
        }
      : {
          id: "rtk",
          name: "RTK",
          statusLabel: rtk?.installed ? "Needs attention" : "Not installed",
          tone: rtk?.installed ? "warning" : "offline",
          detail: rtk?.installed
            ? "RTK exists but is not fully enabled or shell-wired."
            : "RTK is not installed in managed app storage yet.",
          evidence: [
            `Installed: ${rtk?.installed ? "yes" : "no"}.`,
            `Enabled: ${rtk?.enabled ? "yes" : "no"}.`,
            `PATH configured: ${rtk?.pathConfigured ? "yes" : "no"}.`,
            `Hook configured: ${rtk?.hookConfigured ? "yes" : "no"}.`,
          ],
          nextAction: rtk?.installed
            ? "Use Enable or run Doctor to repair shell wiring."
            : "Install RTK from this Addons page.",
        };

  return [
    headroomCard,
    rtkCard,
    managedToolHealth(
      toolById(tools, "markitdown"),
      "markitdown",
      "MarkItDown",
      "Run the MarkItDown smoke check or reinstall the add-on.",
    ),
    managedToolHealth(
      toolById(tools, "ponytail"),
      "ponytail",
      "Ponytail",
      "Reinstall Ponytail or run Doctor to refresh managed guidance.",
    ),
  ];
}
