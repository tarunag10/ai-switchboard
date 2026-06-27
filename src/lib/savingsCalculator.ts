import type { DashboardState } from "./types";
import type { RuntimeStatus } from "./types";
import type { RepoSavingsEstimate } from "./repoIntelligence";

export type SavingsCalculatorScope = "session" | "overall";

export type SavingsCalculatorBreakdownKind =
  | "runtime"
  | "command_output"
  | "repo_context";
export type SavingsCalculatorConfidence = "measured" | "estimated" | "inferred";

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
  confidence: SavingsCalculatorConfidence;
  savedTokens: number;
  savedUsd: number | null;
  detail: string;
}

export interface SavingsCalculatorBreakdownOptions {
  runtimeStatus?: RuntimeStatus | null;
  repoSavings?: RepoSavingsEstimate | null;
}

function formatUsd(value: number) {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

function formatTokens(value: number) {
  return new Intl.NumberFormat("en-US").format(Math.round(value));
}

function formatPercent(value: number | null) {
  if (value === null) {
    return "waiting for usage";
  }

  return `${new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(value)}%`;
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
      confidence: scope === "session" ? "measured" : "estimated",
      savedTokens: summary.savedTokens,
      savedUsd: summary.savedUsd,
      detail:
        scope === "session"
          ? "Runtime compression measured in this app session."
          : "Runtime compression recorded across saved history.",
    },
  ];

  const rtkSaved = Math.max(0, options.runtimeStatus?.rtk.totalSaved ?? 0);
  const rtkCommands = Math.max(
    0,
    options.runtimeStatus?.rtk.totalCommands ?? 0,
  );
  if (scope === "overall" && rtkSaved > 0) {
    rows.push({
      id: "rtk",
      label: "RTK",
      kind: "command_output",
      confidence: "measured",
      savedTokens: rtkSaved,
      savedUsd: null,
      detail:
        rtkCommands > 0
          ? `${rtkCommands.toLocaleString()} command outputs compressed locally.`
          : "Command-output tokens compressed locally.",
    });
  }

  const repoSaved = Math.max(
    0,
    options.repoSavings?.bestPackTokensAvoided ?? 0,
  );
  if (repoSaved > 0) {
    rows.push({
      id: "repo_intelligence",
      label: "Repo Intelligence",
      kind: "repo_context",
      confidence: "inferred",
      savedTokens: repoSaved,
      savedUsd: null,
      detail: `${options.repoSavings?.bestPack?.title ?? "Best context pack"} avoids a broad full-repo scan before agent work.`,
    });
  }

  return rows;
}

export function formatSavingsCalculatorShareText(
  summary: SavingsCalculatorSummary,
  rows: SavingsCalculatorBreakdownRow[],
) {
  const scopeLabel =
    summary.scope === "session" ? "current app session" : "overall history";
  const sourceLines = rows.map((row) => {
    const usdPart =
      row.savedUsd === null ? "" : ` / ${formatUsd(row.savedUsd)}`;
    return `- ${row.label} (${row.confidence}): ${formatTokens(row.savedTokens)} tokens${usdPart}`;
  });

  return [
    `Mac AI Switchboard savings (${scopeLabel})`,
    `Saved: ${formatTokens(summary.savedTokens)} tokens / ${formatUsd(summary.savedUsd)}`,
    `Requests: ${formatTokens(summary.requests)}`,
    `Reduction: ${formatPercent(summary.savingsPct)}`,
    `Likely at least: ${formatUsd(summary.conservativeSavedUsd)}`,
    `Equation: before ${formatTokens(summary.beforeTokens)} - sent ${formatTokens(summary.sentTokens)} = saved ${formatTokens(summary.savedTokens)}`,
    "Sources:",
    ...(sourceLines.length > 0 ? sourceLines : ["- Waiting for usage"]),
  ].join("\n");
}
