export type RepoMemoryMcpSupervisionScope = "app_session" | "relaunch_verified";

export type RepoMemoryMcpRelaunchSurvivalStatus =
  | "not_applicable"
  | "pending"
  | "verified"
  | "failed"
  | "os_daemon_not_supported";

export interface RepoMemoryMcpSupervisionEvidence {
  supervisionStatus: string;
  relaunchSurvivalStatus?: RepoMemoryMcpRelaunchSurvivalStatus | null;
  supervisionScope?: RepoMemoryMcpSupervisionScope | null;
  active?: boolean;
}

export function deriveRepoMemoryMcpSupervisionSummary(
  evidence: RepoMemoryMcpSupervisionEvidence,
): { tone: "success" | "warning" | "danger"; summary: string } {
  const relaunch = evidence.relaunchSurvivalStatus ?? "not_applicable";
  const scope = evidence.supervisionScope ?? "app_session";

  if (evidence.supervisionStatus === "verified_active" && relaunch === "verified") {
    return {
      tone: "success",
      summary:
        "Repo Memory MCP survived app relaunch with a fresh read-only smoke check.",
    };
  }

  if (relaunch === "pending") {
    return {
      tone: "warning",
      summary:
        "Repo Memory MCP was active before relaunch; app is re-verifying read-only smoke evidence.",
    };
  }

  if (relaunch === "failed") {
    return {
      tone: "danger",
      summary:
        "Repo Memory MCP failed relaunch smoke verification. Prepare MCP again before agent handoffs.",
    };
  }

  if (scope === "app_session" && evidence.active) {
    return {
      tone: "warning",
      summary:
        "Repo Memory MCP is supervised for this app session only. OS-level daemon/reboot survival is not claimed.",
    };
  }

  if (
    ["restart_required", "stale_health", "service_unhealthy", "smoke_failed"].includes(
      evidence.supervisionStatus,
    )
  ) {
    return {
      tone: "warning",
      summary: `Repo Memory MCP supervision is degraded (${evidence.supervisionStatus}).`,
    };
  }

  return {
    tone: "success",
    summary: "Repo Memory MCP supervision is healthy for the current app session.",
  };
}
