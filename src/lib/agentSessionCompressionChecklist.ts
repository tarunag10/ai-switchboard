import { recommendExactCacheDefault } from "./exactCacheDefaultPolicy";
import { resolveSwitchboardModeForCache } from "./switchboardModeForCache";
import type { RepoIndexFreshness } from "./repoIntelligence";
import type { RuntimeStatus, SwitchboardMode } from "./types";

export type AgentSessionChecklistStatus = "pass" | "warn" | "blocked";

export interface AgentSessionCompressionChecklistItem {
  id: string;
  label: string;
  status: AgentSessionChecklistStatus;
  detail: string;
  doctorLink?: boolean;
}

export interface AgentSessionCompressionChecklistInput {
  agentId: string;
  packEstimatedTokens: number;
  tokenBudget: number;
  switchboardMode: SwitchboardMode;
  indexFreshness: RepoIndexFreshness;
  runtimeStatus?: RuntimeStatus | null;
  semanticCacheEnabled?: boolean;
}

export interface AgentSessionCompressionChecklist {
  items: AgentSessionCompressionChecklistItem[];
  blocked: boolean;
  hasWarnings: boolean;
  canCopyWithAcknowledgment: boolean;
}

function freshnessItem(
  freshness: RepoIndexFreshness,
): AgentSessionCompressionChecklistItem {
  if (freshness.status === "none") {
    return {
      id: "index-freshness",
      label: "Repo index freshness",
      status: "blocked",
      detail: freshness.detail,
      doctorLink: true,
    };
  }
  if (
    freshness.status === "changed_cache" ||
    freshness.status === "unknown" ||
    freshness.indexHealth !== "healthy"
  ) {
    return {
      id: "index-freshness",
      label: "Repo index freshness",
      status: "warn",
      detail: `${freshness.label}: ${freshness.detail}`,
      doctorLink: true,
    };
  }
  return {
    id: "index-freshness",
    label: "Repo index freshness",
    status: "pass",
    detail: `${freshness.label}: ${freshness.detail}`,
  };
}

function budgetItem(
  packEstimatedTokens: number,
  tokenBudget: number,
): AgentSessionCompressionChecklistItem {
  if (tokenBudget > 0 && packEstimatedTokens > tokenBudget) {
    return {
      id: "pack-budget",
      label: "Pack token budget",
      status: "blocked",
      detail: `Selected pack is ~${packEstimatedTokens.toLocaleString()} tokens against a ${tokenBudget.toLocaleString()} token budget.`,
    };
  }
  return {
    id: "pack-budget",
    label: "Pack token budget",
    status: "pass",
    detail:
      tokenBudget > 0
        ? `Pack fits within the ${tokenBudget.toLocaleString()} token budget.`
        : "No session budget limit is set.",
  };
}

function modeItem(
  mode: SwitchboardMode,
): AgentSessionCompressionChecklistItem {
  if (mode === "off") {
    return {
      id: "switchboard-mode",
      label: "Switchboard mode",
      status: "warn",
      detail: "Off mode bypasses Headroom compression and managed routing.",
      doctorLink: true,
    };
  }
  if (mode === "rtk") {
    return {
      id: "switchboard-mode",
      label: "Switchboard mode",
      status: "warn",
      detail: "RTK-only mode compresses shell output but does not route provider traffic through Headroom.",
    };
  }
  return {
    id: "switchboard-mode",
    label: "Switchboard mode",
    status: "pass",
    detail: `${mode} mode is aligned with compression and routing expectations.`,
  };
}

function cacheItem(
  runtimeStatus: RuntimeStatus | null | undefined,
  semanticCacheEnabled: boolean | undefined,
): AgentSessionCompressionChecklistItem {
  const mode = resolveSwitchboardModeForCache(runtimeStatus);
  const recommendation = recommendExactCacheDefault({
    mode,
    semanticCacheEnabled: semanticCacheEnabled ?? false,
    proxyReachable: runtimeStatus?.proxyReachable ?? false,
  });
  if (recommendation.recommend) {
    return {
      id: "exact-cache",
      label: "Exact cache eligibility",
      status: "warn",
      detail: recommendation.reason,
      doctorLink: true,
    };
  }
  if (semanticCacheEnabled) {
    return {
      id: "exact-cache",
      label: "Exact cache eligibility",
      status: "pass",
      detail: "Exact replay cache is enabled for eligible requests.",
    };
  }
  return {
    id: "exact-cache",
    label: "Exact cache eligibility",
    status: "pass",
    detail: recommendation.reason,
  };
}

function mcpItem(
  runtimeStatus: RuntimeStatus | null | undefined,
): AgentSessionCompressionChecklistItem {
  if (!runtimeStatus?.repoMemoryMcpConfigured) {
    return {
      id: "repo-memory-mcp",
      label: "Repo Memory MCP health",
      status: "warn",
      detail: "Repo Memory MCP is not configured. Prepare MCP before repo-memory tool handoffs.",
      doctorLink: true,
    };
  }
  if (
    runtimeStatus.repoMemoryMcpSupervisionStatus &&
    ["smoke_failed", "service_unhealthy", "stale_health", "restart_required"].includes(
      runtimeStatus.repoMemoryMcpSupervisionStatus,
    )
  ) {
    return {
      id: "repo-memory-mcp",
      label: "Repo Memory MCP health",
      status: "blocked",
      detail: `Repo Memory MCP supervision is degraded (${runtimeStatus.repoMemoryMcpSupervisionStatus}).`,
      doctorLink: true,
    };
  }
  return {
    id: "repo-memory-mcp",
    label: "Repo Memory MCP health",
    status: "pass",
    detail: runtimeStatus.repoMemoryMcpActive
      ? "Repo Memory MCP is active for this app session."
      : "Repo Memory MCP is configured and ready to prepare.",
  };
}

export function buildAgentSessionCompressionChecklist(
  input: AgentSessionCompressionChecklistInput,
): AgentSessionCompressionChecklist {
  const items = [
    freshnessItem(input.indexFreshness),
    budgetItem(input.packEstimatedTokens, input.tokenBudget),
    modeItem(input.switchboardMode),
    cacheItem(input.runtimeStatus, input.semanticCacheEnabled),
    mcpItem(input.runtimeStatus),
  ];

  const blocked = items.some((item) => item.status === "blocked");
  const hasWarnings = items.some((item) => item.status === "warn");

  return {
    items,
    blocked,
    hasWarnings,
    canCopyWithAcknowledgment: blocked || hasWarnings,
  };
}

export function agentSessionChecklistStatusLabel(
  status: AgentSessionChecklistStatus,
): string {
  switch (status) {
    case "pass":
      return "Ready";
    case "warn":
      return "Review";
    case "blocked":
      return "Blocked";
  }
}
