import type { DashboardState } from "./types";
import type { RuntimeStatus } from "./types";
import type { RepoSavingsEstimate } from "./repoIntelligence";

export type SavingsCalculatorScope = "session" | "overall";

export type SavingsCalculatorBreakdownKind =
  | "runtime"
  | "command_output"
  | "repo_context";

export interface SavingsCalculatorSummary {
  scope: SavingsCalculatorScope;
  requests: number;
  savedTokens: number;
  savedUsd: number;
  conservativeSavedUsd: number;
  sentTokens: number;
  beforeTokens: number;
  savingsPct: number | null;
  dataLabel: string;
}

export interface SavingsCalculatorBreakdownRow {
  id: string;
  label: string;
  kind: SavingsCalculatorBreakdownKind;
  savedTokens: number;
  savedUsd: number | null;
  detail: string;
}

export interface SavingsCalculatorBreakdownOptions {
  runtimeStatus?: RuntimeStatus | null;
  repoSavings?: RepoSavingsEstimate | null;
}

export function buildSavingsCalculatorSummary(
  dashboard: DashboardState,
  scope: SavingsCalculatorScope,
): SavingsCalculatorSummary {
  if (scope === "session") {
    const sentTokens = dashboard.recentUsage.reduce(
      (sum, event) =>
        sum + event.estimatedInputTokens + event.estimatedOutputTokens,
      0,
    );
    const beforeTokens = sentTokens + dashboard.sessionEstimatedTokensSaved;

    return {
      scope,
      requests: dashboard.sessionRequests,
      savedTokens: dashboard.sessionEstimatedTokensSaved,
      savedUsd: dashboard.sessionEstimatedSavingsUsd,
      conservativeSavedUsd: dashboard.sessionEstimatedSavingsUsd * 0.5,
      sentTokens,
      beforeTokens,
      savingsPct:
        beforeTokens > 0
          ? (dashboard.sessionEstimatedTokensSaved / beforeTokens) * 100
          : dashboard.sessionSavingsPct > 0
            ? dashboard.sessionSavingsPct
            : null,
      dataLabel: "Current app session",
    };
  }

  const sentTokens = dashboard.dailySavings.reduce(
    (sum, point) => sum + point.totalTokensSent,
    0,
  );
  const beforeTokens = sentTokens + dashboard.lifetimeEstimatedTokensSaved;
  const savingsPct =
    beforeTokens > 0
      ? (dashboard.lifetimeEstimatedTokensSaved / beforeTokens) * 100
      : null;

  return {
    scope,
    requests: dashboard.lifetimeRequests,
    savedTokens: dashboard.lifetimeEstimatedTokensSaved,
    savedUsd: dashboard.lifetimeEstimatedSavingsUsd,
    conservativeSavedUsd: dashboard.lifetimeEstimatedSavingsUsd * 0.5,
    sentTokens,
    beforeTokens,
    savingsPct,
    dataLabel: dashboard.savingsHistoryLoaded
      ? "All tracked switchboard usage"
      : "All recorded usage",
  };
}

export function buildSavingsCalculatorBreakdown(
  dashboard: DashboardState,
  scope: SavingsCalculatorScope,
  options: SavingsCalculatorBreakdownOptions = {},
): SavingsCalculatorBreakdownRow[] {
  const summary = buildSavingsCalculatorSummary(dashboard, scope);
  const rows: SavingsCalculatorBreakdownRow[] = [
    {
      id: "headroom",
      label: "Headroom",
      kind: "runtime",
      savedTokens: summary.savedTokens,
      savedUsd: summary.savedUsd,
      detail:
        scope === "session"
          ? "Runtime compression measured in this app session."
          : "Runtime compression recorded across saved history.",
    },
  ];

  const rtkSaved = Math.max(0, options.runtimeStatus?.rtk.totalSaved ?? 0);
  const rtkCommands = Math.max(0, options.runtimeStatus?.rtk.totalCommands ?? 0);
  if (scope === "overall" && rtkSaved > 0) {
    rows.push({
      id: "rtk",
      label: "RTK",
      kind: "command_output",
      savedTokens: rtkSaved,
      savedUsd: null,
      detail:
        rtkCommands > 0
          ? `${rtkCommands.toLocaleString()} command outputs compressed locally.`
          : "Command-output tokens compressed locally.",
    });
  }

  const repoSaved = Math.max(0, options.repoSavings?.bestPackTokensAvoided ?? 0);
  if (repoSaved > 0) {
    rows.push({
      id: "repo_intelligence",
      label: "Repo Intelligence",
      kind: "repo_context",
      savedTokens: repoSaved,
      savedUsd: null,
      detail: `${options.repoSavings?.bestPack?.title ?? "Best context pack"} avoids a broad full-repo scan before agent work.`,
    });
  }

  return rows;
}
