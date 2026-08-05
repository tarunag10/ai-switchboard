import type { DoctorIssue } from "./types";
import { doctorRepairLabel } from "./doctorRepairCopy";

export type CompressionPlaybookStageId =
  | "runtime"
  | "routing"
  | "rtk"
  | "cache"
  | "repo-index"
  | "mcp";

export interface CompressionPlaybookStage {
  id: CompressionPlaybookStageId;
  label: string;
  issueIds: readonly string[];
  repairActions: readonly string[];
  guidance: string;
}

export interface CompressionPlaybookStageSummary {
  stage: CompressionPlaybookStage;
  openIssues: DoctorIssue[];
  nextRepairAction: string | null;
  nextRepairLabel: string | null;
}

export interface CompressionPlaybookSummary {
  stages: CompressionPlaybookStageSummary[];
  openIssueCount: number;
  hasOpenCompressionIssues: boolean;
  orderedStageIds: CompressionPlaybookStageId[];
}

export interface CompressionPlaybookOptions {
  issues: readonly DoctorIssue[];
  exactCacheRecommended?: boolean;
  semanticCacheEnabled?: boolean;
}

export const COMPRESSION_PLAYBOOK_STAGES: readonly CompressionPlaybookStage[] = [
  {
    id: "runtime",
    label: "Runtime",
    issueIds: [
      "headroom_runtime_unreachable",
      "headroom_native_compressor_unavailable",
      "headroom_paused",
    ],
    repairActions: ["repair_runtime"],
    guidance: "Restart or repair Headroom before expecting live compression savings.",
  },
  {
    id: "routing",
    label: "Routing",
    issueIds: [
      "switchboard_mode_degraded",
      "codex_direct_bypass",
      "no_headroom_clients",
      "proxy_loopback_unauthenticated",
      "codex_thread_retagging_opt_in_required",
    ],
    repairActions: [
      "reset_codex_bypass",
      "repair_client_setups",
      "repair_codex_setup",
    ],
    guidance: "Align requested mode, managed clients, and proxy routing evidence.",
  },
  {
    id: "rtk",
    label: "RTK",
    issueIds: ["rtk_not_active", "rtk_integration_incomplete"],
    repairActions: ["repair_rtk_runtime", "repair_rtk_integrations"],
    guidance: "Install or repair RTK shell compression before relying on command-output savings.",
  },
  {
    id: "cache",
    label: "Cache",
    issueIds: [],
    repairActions: [],
    guidance:
      "Enable exact replay cache when Full or Headroom mode is healthy. Cache hits stay separate from compression savings.",
  },
  {
    id: "repo-index",
    label: "Repo index",
    issueIds: [
      "repo_intelligence_repo_missing",
      "repo_intelligence_repo_moved",
      "repo_intelligence_stale",
      "repo_intelligence_index_health",
      "repo_intelligence_storage_corrupt",
    ],
    repairActions: ["clear_repo_intelligence_index"],
    guidance: "Refresh Repo Intelligence before copying context packs into agent sessions.",
  },
  {
    id: "mcp",
    label: "Repo Memory MCP",
    issueIds: [
      "repo_memory_mcp_not_configured",
      "repo_memory_mcp_smoke_failed",
      "repo_memory_mcp_stale_config",
      "repo_memory_mcp_service_unhealthy",
      "repo_memory_mcp_needs_verification",
    ],
    repairActions: ["install_repo_memory_mcp"],
    guidance: "Prepare the read-only Repo Memory MCP bridge before agent handoffs.",
  },
] as const;

export const COMPRESSION_PLAYBOOK_ORDER: readonly CompressionPlaybookStageId[] =
  COMPRESSION_PLAYBOOK_STAGES.map((stage) => stage.id);

function issueMatchesStage(issue: DoctorIssue, stage: CompressionPlaybookStage) {
  if (stage.issueIds.includes(issue.id)) return true;
  const repairAction = issue.repairAction ?? "";
  return stage.repairActions.some((action) => repairAction === action);
}

function cacheStageOpen(
  options: CompressionPlaybookOptions,
): DoctorIssue[] {
  if (
    options.exactCacheRecommended &&
    options.semanticCacheEnabled === false
  ) {
    return [
      {
        id: "exact_cache_recommended",
        title: "Exact replay cache is eligible but disabled",
        body: "Full or Headroom mode is healthy enough to recommend exact cache. Enable it from Add-ons or Max compression; cache hits remain separate from compression savings.",
        severity: "warning",
        repairAction: null,
      },
    ];
  }
  return [];
}

export function buildCompressionPlaybookSummary(
  options: CompressionPlaybookOptions,
): CompressionPlaybookSummary {
  const stages = COMPRESSION_PLAYBOOK_STAGES.map((stage) => {
    const openIssues =
      stage.id === "cache"
        ? cacheStageOpen(options)
        : options.issues.filter((issue) => issueMatchesStage(issue, stage));
    const nextRepairAction =
      openIssues.find((issue) => issue.repairAction)?.repairAction ?? null;
    return {
      stage,
      openIssues,
      nextRepairAction,
      nextRepairLabel: nextRepairAction
        ? doctorRepairLabel(nextRepairAction)
        : null,
    };
  });

  const openIssueCount = stages.reduce(
    (count, entry) => count + entry.openIssues.length,
    0,
  );

  return {
    stages,
    openIssueCount,
    hasOpenCompressionIssues: openIssueCount > 0,
    orderedStageIds: [...COMPRESSION_PLAYBOOK_ORDER],
  };
}

export function compressionPlaybookShareText(
  summary: CompressionPlaybookSummary,
): string {
  const lines = [
    "Compression repair playbook",
    `Open issues: ${summary.openIssueCount}`,
    "",
  ];
  for (const entry of summary.stages) {
    if (entry.openIssues.length === 0) continue;
    lines.push(`${entry.stage.label} (${entry.openIssues.length})`);
    lines.push(entry.stage.guidance);
    if (entry.nextRepairLabel) {
      lines.push(`Next repair: ${entry.nextRepairLabel}`);
    }
    lines.push("");
  }
  return lines.join("\n").trim();
}
