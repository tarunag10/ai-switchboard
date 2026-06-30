import type {
  DailySavingsPoint,
  ManagedTool,
  RtkDailyStats,
  RuntimeStatus,
  UsageEvent,
} from "./types";

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
  trend: AddonHealthTrend;
  nextAction: string;
}

export interface AddonHealthTrendPoint {
  label: string;
  value: number;
}

export interface AddonHealthTrend {
  label: string;
  value: string;
  detail: string;
  points: AddonHealthTrendPoint[];
}

export interface AddonHealthHistoryInputs {
  recentUsage?: UsageEvent[];
  dailySavings?: DailySavingsPoint[];
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
      "Still planned: deeper AST-backed parsing, full call graph, and persistent parser indexes; direct repo-memory MCP controls and Doctor repair integration are available now.",
      "Local-first index stored on Mac, with secret-like paths, generated outputs, and vendor folders excluded from default packs.",
      "Read-only by default; write or auto-repair actions remain explicit user actions.",
    ],
    healthChecks: [
      "Local index exists can be cleared without touching repository.",
      "Secret-like paths generated folders excluded context packs.",
      "Manifest includes implementation, verification, handoff packs estimated tokens avoided.",
      "Graph summary includes dependency hubs, path-based edges, import references, call references, reverse dependency hubs, stylesheet links, and HTML asset entrypoints.",
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

function formatCount(value: number) {
  return new Intl.NumberFormat("en-US", {
    notation: value >= 10_000 ? "compact" : "standard",
    maximumFractionDigits: value >= 10_000 ? 1 : 0,
  }).format(Math.max(0, value));
}

function shortDateLabel(dateKey: string) {
  const parsed = new Date(`${dateKey}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) {
    return dateKey;
  }
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
  }).format(parsed);
}

function noDurableHistoryTrend(addonName: string): AddonHealthTrend {
  return {
    label: "Health history",
    value: "Current only",
    detail: `${addonName} has live install health; durable per-day trend evidence is not available yet.`,
    points: [],
  };
}

function buildHeadroomHealthTrend(
  history: AddonHealthHistoryInputs,
): AddonHealthTrend {
  const events = history.recentUsage ?? [];
  const headroomEvents = events
    .map((event) => {
      const tokensSaved = event.stages
        .filter(
          (stage) =>
            stage.applied &&
            /headroom|kompress/i.test(`${stage.stageId} ${stage.stageName}`),
        )
        .reduce(
          (sum, stage) => sum + Math.max(0, stage.estimatedTokensSaved),
          0,
        );
      return {
        label: new Intl.DateTimeFormat(undefined, {
          hour: "2-digit",
          minute: "2-digit",
        }).format(new Date(event.timestamp)),
        value: tokensSaved,
      };
    })
    .filter((point) => point.value > 0);

  if (headroomEvents.length > 0) {
    const tokensSaved = headroomEvents.reduce(
      (sum, point) => sum + point.value,
      0,
    );
    const requestNoun = headroomEvents.length === 1 ? "request" : "requests";
    const requestVerb = headroomEvents.length === 1 ? "includes" : "include";
    return {
      label: "Recent Headroom trend",
      value: `${formatCount(tokensSaved)} tokens`,
      detail: `${headroomEvents.length} recent optimized ${requestNoun} ${requestVerb} Headroom compression evidence.`,
      points: headroomEvents.slice(-6),
    };
  }

  const savingsDays = (history.dailySavings ?? [])
    .filter((point) => Math.max(0, point.estimatedTokensSaved) > 0)
    .slice(-7)
    .map((point) => ({
      label: shortDateLabel(point.date),
      value: Math.max(0, point.estimatedTokensSaved),
    }));

  if (savingsDays.length > 0) {
    const tokensSaved = savingsDays.reduce((sum, point) => sum + point.value, 0);
    return {
      label: "Saved history trend",
      value: `${formatCount(tokensSaved)} tokens`,
      detail: `${savingsDays.length} saved local history days include optimization savings.`,
      points: savingsDays,
    };
  }

  return {
    label: "Recent Headroom trend",
    value: "No traffic yet",
    detail: "Send traffic through a connected tool to build durable Headroom trend evidence.",
    points: [],
  };
}

function buildRtkHealthTrend(daily: RtkDailyStats[] = []): AddonHealthTrend {
  const points = daily
    .filter(
      (point) =>
        Math.max(0, point.commands) > 0 || Math.max(0, point.savedTokens) > 0,
    )
    .slice(-7)
    .map((point) => ({
      label: shortDateLabel(point.date),
      value: Math.max(0, point.savedTokens),
      commands: Math.max(0, point.commands),
    }));

  if (points.length === 0) {
    return {
      label: "RTK history trend",
      value: "No commands yet",
      detail: "Run shell commands through RTK to build per-day command-output savings history.",
      points: [],
    };
  }

  const tokensSaved = points.reduce((sum, point) => sum + point.value, 0);
  const commands = points.reduce((sum, point) => sum + point.commands, 0);
  return {
    label: "RTK history trend",
    value: `${formatCount(tokensSaved)} tokens`,
    detail: `${formatCount(commands)} commands across ${points.length} local RTK history days.`,
    points: points.map(({ label, value }) => ({ label, value })),
  };
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
      trend: noDurableHistoryTrend(name),
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
      trend: noDurableHistoryTrend(name),
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
      trend: noDurableHistoryTrend(name),
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
    trend: noDurableHistoryTrend(name),
    nextAction: "No action needed.",
  };
}

export function buildAddonHealthCards(
  runtimeStatus: RuntimeStatus | null | undefined,
  tools: ManagedTool[] = [],
  history: AddonHealthHistoryInputs = {},
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
          trend: buildHeadroomHealthTrend(history),
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
            trend: buildHeadroomHealthTrend(history),
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
            trend: buildHeadroomHealthTrend(history),
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
          trend: buildRtkHealthTrend(rtk.daily ?? []),
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
          trend: buildRtkHealthTrend(rtk?.daily ?? []),
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
