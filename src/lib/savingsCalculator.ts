import type { DashboardState } from "./types";

export type SavingsCalculatorScope = "session" | "overall";

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
