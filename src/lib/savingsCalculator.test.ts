import { describe, expect, it } from "vitest";

import {
  buildSavingsCalculatorSummary,
  type SavingsCalculatorScope,
} from "./savingsCalculator";
import type { DashboardState } from "./types";

function dashboardFixture(
  overrides: Partial<DashboardState> = {},
): DashboardState {
  return {
    appVersion: "0.0.0",
    launchExperience: "dashboard",
    bootstrapComplete: true,
    pythonRuntimeInstalled: true,
    lifetimeRequests: 10,
    lifetimeEstimatedSavingsUsd: 4.5,
    lifetimeEstimatedTokensSaved: 2_000,
    sessionRequests: 2,
    sessionEstimatedSavingsUsd: 0.75,
    sessionEstimatedTokensSaved: 300,
    sessionSavingsPct: 30,
    outputReduction: null,
    dailySavings: [
      {
        date: "2026-06-25",
        estimatedSavingsUsd: 4.5,
        estimatedTokensSaved: 2_000,
        actualCostUsd: 2,
        totalTokensSent: 8_000,
      },
    ],
    hourlySavings: [],
    savingsHistoryLoaded: true,
    tools: [],
    clients: [],
    recentUsage: [
      {
        id: "usage-1",
        timestamp: "2026-06-25T10:00:00Z",
        client: "Codex",
        workspace: "repo",
        upstreamTarget: "openai",
        stages: [],
        estimatedInputTokens: 1_000,
        estimatedOutputTokens: 200,
        estimatedCostSavingsUsd: 0.75,
        latencyMs: 120,
        outcome: "success",
      },
    ],
    insights: [],
    requiredTermsVersion: 1,
    acceptedTermsVersion: 1,
    termsUrl: "https://example.com/terms",
    ...overrides,
  };
}

describe("savings calculator", () => {
  it.each<SavingsCalculatorScope>(["session", "overall"])(
    "returns stable %s totals",
    (scope) => {
      const summary = buildSavingsCalculatorSummary(dashboardFixture(), scope);

      expect(summary.scope).toBe(scope);
      expect(summary.savedTokens).toBe(scope === "session" ? 300 : 2_000);
      expect(summary.savedUsd).toBe(scope === "session" ? 0.75 : 4.5);
      expect(summary.conservativeSavedUsd).toBe(scope === "session" ? 0.375 : 2.25);
      expect(summary.requests).toBe(scope === "session" ? 2 : 10);
      expect(summary.savingsPct).toBe(20);
    },
  );

  it("does not invent a percentage before usage exists", () => {
    const summary = buildSavingsCalculatorSummary(
      dashboardFixture({
        lifetimeRequests: 0,
        lifetimeEstimatedSavingsUsd: 0,
        lifetimeEstimatedTokensSaved: 0,
        sessionRequests: 0,
        sessionEstimatedSavingsUsd: 0,
        sessionEstimatedTokensSaved: 0,
        sessionSavingsPct: 0,
        dailySavings: [],
        recentUsage: [],
      }),
      "overall",
    );

    expect(summary.savingsPct).toBeNull();
    expect(summary.beforeTokens).toBe(0);
  });
});
