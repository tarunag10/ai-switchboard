import { describe, expect, it } from "vitest";

import {
  buildAddonSavingsEstimate,
  buildFilteredSavingsLedger,
  buildSavingsCalculatorBreakdown,
  buildSavingsCalculatorSummary,
  buildSavingsLedgerRows,
  filterSavingsLedgerRowsByConfidence,
  formatSavingsLedgerAttributionSummary,
  formatSavingsLedgerConfidenceBreakdown,
  formatSavingsLedgerShareText,
  getSavingsLedgerEmptyState,
  groupSavingsLedgerRowsBySource,
  formatSavingsCalculatorShareText,
  savingsCalculatorScopeDefinition,
  summarizeSavingsLedgerRows,
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
  it.each<SavingsCalculatorScope>(["session", "lifetime"])(
    "returns stable %s totals",
    (scope) => {
      const summary = buildSavingsCalculatorSummary(dashboardFixture(), scope);

      expect(summary.scope).toBe(scope);
      expect(summary.savedTokens).toBe(scope === "session" ? 300 : 2_000);
      expect(summary.savedUsd).toBe(scope === "session" ? 0.75 : 4.5);
      expect(summary.conservativeSavedUsd).toBe(
        scope === "session" ? 0.375 : 2.25,
      );
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
      "lifetime",
    );

    expect(summary.savingsPct).toBeNull();
    expect(summary.beforeTokens).toBe(0);
  });

  it("does not surface lifetime fallbacks until saved local history loads", () => {
    const summary = buildSavingsCalculatorSummary(
      dashboardFixture({
        lifetimeEstimatedSavingsUsd: 4.25,
        lifetimeEstimatedTokensSaved: 6_400_000,
        dailySavings: [],
        savingsHistoryLoaded: false,
      }),
      "lifetime",
    );

    expect(summary).toMatchObject({
      savedTokens: 0,
      savedUsd: 0,
      sentTokens: 0,
      dataLabel: "Waiting for saved local history",
    });
    expect(summary.savingsPct).toBeNull();
  });

  it("builds today, week, and month summaries from saved local history", () => {
    const today = new Date().toISOString().slice(0, 10);
    const weekStart = new Date();
    weekStart.setUTCDate(weekStart.getUTCDate() - 6);
    const weekStartDate = weekStart.toISOString().slice(0, 10);
    const weekStartInCurrentMonth = weekStartDate.startsWith(today.slice(0, 7));
    const expectedWeek = {
      savedTokens: 800,
      savedUsd: 2,
      sentTokens: 3_200,
    };
    const expectedMonth = {
      savedTokens: weekStartInCurrentMonth ? 800 : 600,
      savedUsd: weekStartInCurrentMonth ? 2 : 1.5,
      sentTokens: weekStartInCurrentMonth ? 3_200 : 2_400,
    };
    const dashboard = dashboardFixture({
      dailySavings: [
        {
          date: today,
          estimatedSavingsUsd: 1.5,
          estimatedTokensSaved: 600,
          actualCostUsd: 0.75,
          totalTokensSent: 2_400,
        },
        {
          date: weekStartDate,
          estimatedSavingsUsd: 0.5,
          estimatedTokensSaved: 200,
          actualCostUsd: 0.25,
          totalTokensSent: 800,
        },
        {
          date: "2026-01-01",
          estimatedSavingsUsd: 9,
          estimatedTokensSaved: 9_000,
          actualCostUsd: 4,
          totalTokensSent: 9_000,
        },
      ],
    });

    expect(buildSavingsCalculatorSummary(dashboard, "today")).toMatchObject({
      savedTokens: 600,
      savedUsd: 1.5,
      sentTokens: 2_400,
      requests: 0,
      dataLabel: "Tracked switchboard usage today",
    });
    expect(buildSavingsCalculatorSummary(dashboard, "week")).toMatchObject({
      savedTokens: expectedWeek.savedTokens,
      savedUsd: expectedWeek.savedUsd,
      sentTokens: expectedWeek.sentTokens,
      requests: 0,
      dataLabel: "Tracked switchboard usage this week",
    });
    expect(buildSavingsCalculatorSummary(dashboard, "month")).toMatchObject({
      savedTokens: expectedMonth.savedTokens,
      savedUsd: expectedMonth.savedUsd,
      sentTokens: expectedMonth.sentTokens,
      requests: 0,
      dataLabel: "Tracked switchboard usage this month",
    });
  });

  it("uses measured backend attribution events for the session Headroom ledger row", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture({
        sessionEstimatedTokensSaved: 999,
        sessionEstimatedSavingsUsd: 9.99,
      }),
      "session",
      "2026-06-25T10:00:00Z",
      {
        attributionEvents: [
          {
            schemaVersion: 1,
            id: "event-1",
            observedAt: "2026-06-25T10:01:00Z",
            scope: "session",
            source: "headroom_engine",
            confidence: "measured",
            deltaTokensSaved: 100,
            deltaUsd: 0.25,
            totalTokensSent: 900,
            requestDelta: 1,
            evidence: ["Measured from positive Headroom /stats session deltas."],
          },
          {
            schemaVersion: 1,
            id: "event-2",
            observedAt: "2026-06-25T10:02:00Z",
            scope: "session",
            source: "headroom_engine",
            confidence: "measured",
            deltaTokensSaved: 125,
            deltaUsd: 0.5,
            totalTokensSent: 1_000,
            requestDelta: 2,
            evidence: [],
          },
        ],
      },
    );

    const headroom = rows.find((row) => row.source === "headroom_engine");

    expect(headroom).toMatchObject({
      id: "headroom_attribution_events",
      confidence: "measured",
      savedTokens: 225,
      savedUsd: 0.75,
      recordedAt: "2026-06-25T10:02:00Z",
      caveat: "Observed from append-only backend attribution events.",
    });
    expect(headroom?.detail).toContain("2 measured Headroom session events");
    expect(headroom?.detail).toContain("3 requests");
    expect(rows.filter((row) => row.source === "headroom_engine")).toHaveLength(1);
  });

  it("uses measured backend attribution events for the session RTK ledger row", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "session",
      "2026-06-25T10:00:00Z",
      {
        attributionEvents: [
          {
            schemaVersion: 1,
            id: "rtk-event-1",
            observedAt: "2026-06-25T10:03:00Z",
            scope: "session",
            source: "rtk",
            confidence: "measured",
            deltaTokensSaved: 450,
            deltaUsd: 0,
            totalTokensSent: 0,
            requestDelta: 3,
            evidence: ["Measured from positive RTK gain counter deltas."],
          },
        ],
      },
    );

    const rtk = rows.find((row) => row.source === "rtk");

    expect(rtk).toMatchObject({
      id: "rtk_attribution_events",
      label: "RTK",
      confidence: "measured",
      savedTokens: 450,
      savedUsd: null,
      recordedAt: "2026-06-25T10:03:00Z",
      caveat: "Observed from append-only backend attribution events.",
    });
    expect(rtk?.detail).toContain("1 measured RTK session event");
    expect(rtk?.detail).toContain("3 commands");
    expect(rtk?.detail).toContain("RTK gain counter");
  });

  it("uses measured RTK daily stats for the today ledger row", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "today",
      "2026-06-25T10:00:00Z",
      {
        rtkToday: {
          date: "2026-06-25",
          savedTokens: 1_234,
          commands: 7,
          inputTokens: 2_000,
          outputTokens: 766,
          totalTimeMs: 3_500,
        },
      },
    );

    const rtk = rows.find((row) => row.id === "rtk_today");

    expect(rtk).toMatchObject({
      label: "RTK today",
      source: "rtk",
      confidence: "measured",
      savedTokens: 1_234,
      savedUsd: null,
      recordedAt: "2026-06-25T10:00:00Z",
      caveat: "Observed from local counters for this source.",
    });
    expect(rtk?.detail).toContain("7 command outputs compressed locally today");
    expect(rtk?.detail).toContain(
      "RTK measured 2,000 input tokens, 766 output tokens, and 1,234 saved tokens",
    );
    expect(rtk?.detail).toContain("61.7% saved vs input");
    expect(rtk?.detail).toContain("3.5s total RTK processing time");
  });

  it("rolls RTK daily stats into week and month measured ledger rows", () => {
    const today = new Date().toISOString().slice(0, 10);
    const weekStart = new Date();
    weekStart.setUTCDate(weekStart.getUTCDate() - 6);
    const weekStartDate = weekStart.toISOString().slice(0, 10);
    const weekStartInCurrentMonth = weekStartDate.startsWith(today.slice(0, 7));
    const runtimeStatus = {
      rtk: {
        daily: [
          {
            date: today,
            savedTokens: 500,
            commands: 2,
            inputTokens: 1_000,
            outputTokens: 500,
            totalTimeMs: 1_100,
          },
          {
            date: weekStartDate,
            savedTokens: 250,
            commands: 1,
            inputTokens: 500,
            outputTokens: 250,
            totalTimeMs: 900,
          },
          { date: "2026-01-01", savedTokens: 9_000, commands: 9 },
        ],
      },
    } as never;

    const weekRows = buildSavingsLedgerRows(
      dashboardFixture(),
      "week",
      "2026-06-25T10:00:00Z",
      { runtimeStatus },
    );
    const monthRows = buildSavingsLedgerRows(
      dashboardFixture(),
      "month",
      "2026-06-25T10:00:00Z",
      { runtimeStatus },
    );

    expect(weekRows.find((row) => row.id === "rtk_week")).toMatchObject({
      label: "RTK this week",
      confidence: "measured",
      savedTokens: 750,
    });
    expect(weekRows.find((row) => row.id === "rtk_week")?.detail).toContain(
      "RTK measured 1,500 input tokens, 750 output tokens, and 750 saved tokens",
    );
    expect(weekRows.find((row) => row.id === "rtk_week")?.detail).toContain(
      "2s total RTK processing time",
    );
    expect(monthRows.find((row) => row.id === "rtk_month")).toMatchObject({
      label: "RTK this month",
      confidence: "measured",
      savedTokens: weekStartInCurrentMonth ? 750 : 500,
    });
  });

  it("uses measured backend attribution events for session Repo Intelligence rows", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "session",
      "2026-06-25T10:00:00Z",
      {
        repoSavings: {
          fullScanTokens: 4_000,
          bestPackTokens: 1_500,
          bestPackTokensAvoided: 2_500,
          bestPackSavingsPct: 62.5,
          allPacksTokens: 2_500,
          allPacksTokensAvoided: 1_500,
          allPacksSavingsPct: 37.5,
        },
        attributionEvents: [
          {
            schemaVersion: 1,
            id: "repo-event-1",
            observedAt: "2026-06-25T10:04:00Z",
            scope: "session",
            source: "repo_intelligence",
            confidence: "measured",
            deltaTokensSaved: 1_100,
            deltaUsd: 0,
            totalTokensSent: 0,
            requestDelta: 1,
            evidence: ["Measured from a copied Repo Intelligence context-pack delta."],
          },
        ],
      },
    );

    const repo = rows.find((row) => row.source === "repo_intelligence");

    expect(repo).toMatchObject({
      id: "repo_intelligence_attribution_events",
      label: "Repo Intelligence",
      confidence: "measured",
      savedTokens: 1_100,
      savedUsd: null,
      recordedAt: "2026-06-25T10:04:00Z",
      caveat: "Observed from append-only backend attribution events.",
    });
    expect(repo?.detail).toContain("1 measured Repo Intelligence session event");
    expect(repo?.detail).toContain("1 request");
    expect(rows.filter((row) => row.source === "repo_intelligence")).toHaveLength(1);
  });

  it("uses durable estimated backend attribution events for session add-on ledger rows", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "session",
      "2026-06-25T10:00:00Z",
      {
        cavemanSavings: buildAddonSavingsEstimate(480, 180),
        ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
        attributionEvents: [
          {
            schemaVersion: 1,
            id: "caveman-event-1",
            observedAt: "2026-06-25T10:05:00Z",
            scope: "session",
            source: "caveman",
            confidence: "estimated",
            deltaTokensSaved: 300,
            deltaUsd: 0,
            totalTokensSent: 0,
            requestDelta: 1,
            evidence: [
              "Estimated Caveman managed guidance changed 2 client instruction files.",
            ],
          },
          {
            schemaVersion: 1,
            id: "ponytail-event-1",
            observedAt: "2026-06-25T10:06:00Z",
            scope: "session",
            source: "ponytail",
            confidence: "estimated",
            deltaTokensSaved: 880,
            deltaUsd: 0,
            totalTokensSent: 0,
            requestDelta: 1,
            evidence: [
              "Estimated Ponytail plugin registered with 2 agent hosts: Claude Code, Codex.",
            ],
          },
        ],
      },
    );

    const caveman = rows.find((row) => row.source === "caveman");
    const ponytail = rows.find((row) => row.source === "ponytail");

    expect(caveman).toMatchObject({
      id: "caveman_attribution_events",
      label: "Caveman",
      confidence: "estimated",
      savedTokens: 300,
      savedUsd: null,
      recordedAt: "2026-06-25T10:05:00Z",
      caveat:
        "Estimated from changed Caveman-managed instruction files and the audited terse-output template delta.",
    });
    expect(caveman?.detail).toContain("1 estimated Caveman session event");
    expect(caveman?.detail).toContain("managed guidance changed 2 client instruction files");
    expect(rows.filter((row) => row.source === "caveman")).toHaveLength(1);
    expect(ponytail).toMatchObject({
      id: "ponytail_attribution_events",
      label: "Ponytail",
      confidence: "estimated",
      savedTokens: 880,
      savedUsd: null,
      recordedAt: "2026-06-25T10:06:00Z",
      caveat:
        "Estimated from verified Ponytail plugin registration in connected agent hosts; not runtime-measured output.",
    });
    expect(ponytail?.detail).toContain("1 estimated Ponytail session event");
    expect(ponytail?.detail).toContain("plugin registered with 2 agent hosts");
    expect(rows.filter((row) => row.source === "ponytail")).toHaveLength(1);
  });

  it("keeps lifetime Headroom ledger rows based on saved rollups", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "lifetime",
      "2026-06-25T10:00:00Z",
      {
        attributionEvents: [
          {
            schemaVersion: 1,
            id: "event-1",
            observedAt: "2026-06-25T10:01:00Z",
            scope: "session",
            source: "headroom_engine",
            confidence: "measured",
            deltaTokensSaved: 100,
            deltaUsd: 0.25,
            totalTokensSent: 900,
            requestDelta: 1,
            evidence: [],
          },
        ],
      },
    );

    const headroom = rows.find((row) => row.source === "headroom_engine");

    expect(headroom).toMatchObject({
      id: "headroom",
      confidence: "estimated",
      savedTokens: 2_000,
      savedUsd: 4.5,
    });
  });

  it("keeps repo scope focused on Repo Intelligence estimates", () => {
    const rows = buildSavingsCalculatorBreakdown(dashboardFixture(), "repo", {
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_500,
        bestPackTokensAvoided: 7_500,
        bestPackSavingsPct: 75,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation pack",
          purpose: "Build next slice",
          estimatedTokens: 2_500,
          savingsVsFullScanPct: 75,
          files: [],
        },
      },
    });

    expect(buildSavingsCalculatorSummary(dashboardFixture(), "repo")).toMatchObject({
      savedTokens: 0,
      savedUsd: 0,
      requests: 0,
      dataLabel: "Current repo context estimate",
    });
    expect(rows.map((row) => row.id)).toEqual(["repo_intelligence"]);
    expect(rows[0].confidence).toBe("estimated");
  });

  it("breaks down lifetime savings by runtime, RTK, and repo context", () => {
    const rows = buildSavingsCalculatorBreakdown(
      dashboardFixture(),
      "lifetime",
      {
        runtimeStatus: {
          platform: "darwin",
          supportTier: "supported",
          installed: true,
          running: true,
          starting: false,
          paused: false,
          autoPaused: false,
          proxyReachable: true,
          headroomLearnSupported: true,
          rtk: {
            installed: true,
            enabled: true,
            pathConfigured: true,
            hookConfigured: true,
            totalCommands: 12,
            totalSaved: 900,
            avgSavingsPct: 72,
          },
        },
        repoSavings: {
          fullScanTokens: 10_000,
          bestPackTokens: 2_000,
          bestPackTokensAvoided: 8_000,
          bestPackSavingsPct: 80,
          allPacksTokens: 4_000,
          allPacksTokensAvoided: 6_000,
          allPacksSavingsPct: 60,
          bestPack: {
            id: "implementation",
            title: "Implementation Pack",
            purpose: "Feature work",
            estimatedTokens: 2_000,
            savingsVsFullScanPct: 80,
            files: [],
          },
        },
      },
    );

    expect(rows.map((row) => row.id)).toEqual([
      "headroom",
      "rtk",
      "repo_intelligence",
    ]);
    expect(rows.map((row) => row.confidence)).toEqual([
      "estimated",
      "measured",
      "estimated",
    ]);
    expect(rows[0].savedUsd).toBe(4.5);
    expect(rows[1].savedTokens).toBe(900);
    expect(rows[2].detail).toContain("Implementation Pack");
    expect(rows[2].detail).toContain("graph summary");
  });

  it("appends inferred add-on rows when their estimates avoid tokens", () => {
    const rows = buildSavingsCalculatorBreakdown(dashboardFixture(), "lifetime", {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
          avgSavingsPct: 72,
        },
      },
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_000,
        bestPackTokensAvoided: 8_000,
        bestPackSavingsPct: 80,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation Pack",
          purpose: "Feature work",
          estimatedTokens: 2_000,
          savingsVsFullScanPct: 80,
          files: [],
        },
      },
      cavemanSavings: buildAddonSavingsEstimate(480, 180),
      ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
      markitdownSavings: buildAddonSavingsEstimate(3_200, 900),
    });

    expect(rows.map((row) => row.id)).toEqual([
      "headroom",
      "rtk",
      "repo_intelligence",
      "caveman",
      "ponytail",
      "markitdown",
    ]);
    const addonRows = rows.filter((row) =>
      ["caveman", "ponytail", "markitdown"].includes(row.id),
    );
    expect(addonRows.map((row) => row.confidence)).toEqual([
      "inferred",
      "inferred",
      "estimated",
    ]);
    expect(addonRows.map((row) => row.savedUsd)).toEqual([null, null, null]);
    expect(addonRows.map((row) => row.savedTokens)).toEqual([300, 880, 2_300]);
  });

  it("omits add-on rows when the estimate is missing or avoids no tokens", () => {
    const rows = buildSavingsCalculatorBreakdown(dashboardFixture(), "lifetime", {
      cavemanSavings: buildAddonSavingsEstimate(200, 200),
      ponytailSavings: null,
    });

    expect(rows.map((row) => row.id)).toEqual(["headroom"]);
  });

  it("derives add-on estimates defensively", () => {
    expect(buildAddonSavingsEstimate(500, 200)).toMatchObject({
      tokensAvoided: 300,
      savingsPct: 60,
    });
    expect(buildAddonSavingsEstimate(100, 400)).toMatchObject({
      tokensAvoided: 0,
      savingsPct: 0,
    });
    expect(buildAddonSavingsEstimate(0, 0).savingsPct).toBe(0);
  });

  it("does not show lifetime RTK totals in the session breakdown", () => {
    const rows = buildSavingsCalculatorBreakdown(
      dashboardFixture(),
      "session",
      {
        runtimeStatus: {
          platform: "darwin",
          supportTier: "supported",
          installed: true,
          running: true,
          starting: false,
          paused: false,
          autoPaused: false,
          proxyReachable: true,
          headroomLearnSupported: true,
          rtk: {
            installed: true,
            enabled: true,
            pathConfigured: true,
            hookConfigured: true,
            totalCommands: 12,
            totalSaved: 900,
          },
        },
      },
    );

    expect(rows.map((row) => row.id)).toEqual(["headroom"]);
  });

  it("builds scoped ledger rows with confidence labels", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "lifetime",
      "2026-06-27T10:00:00.000Z",
      {
        runtimeStatus: {
          platform: "darwin",
          supportTier: "supported",
          installed: true,
          running: true,
          starting: false,
          paused: false,
          autoPaused: false,
          proxyReachable: true,
          headroomLearnSupported: true,
          rtk: {
            installed: true,
            enabled: true,
            pathConfigured: true,
            hookConfigured: true,
            totalCommands: 12,
            totalSaved: 900,
          },
        },
      },
    );

    expect(rows.map((row) => row.scope)).toEqual(["lifetime", "lifetime"]);
    expect(rows.map((row) => row.recordedAt)).toEqual([
      "2026-06-27T10:00:00.000Z",
      "2026-06-27T10:00:00.000Z",
    ]);
    expect(rows.map((row) => row.confidence)).toEqual([
      "estimated",
      "measured",
    ]);
    expect(rows.map((row) => row.source)).toEqual(["headroom_engine", "rtk"]);
    expect(rows[0].caveat).toContain("Estimated from saved history");
    expect(rows[1].caveat).toContain("Observed from local counters");
  });

  it("summarizes ledger confidence buckets without upgrading inferred rows", () => {
    const rows = buildSavingsLedgerRows(
      dashboardFixture(),
      "lifetime",
      "2026-06-27T10:00:00.000Z",
      {
        runtimeStatus: {
          platform: "darwin",
          supportTier: "supported",
          installed: true,
          running: true,
          starting: false,
          paused: false,
          autoPaused: false,
          proxyReachable: true,
          headroomLearnSupported: true,
          rtk: {
            installed: true,
            enabled: true,
            pathConfigured: true,
            hookConfigured: true,
            totalCommands: 12,
            totalSaved: 900,
          },
        },
        repoSavings: {
          fullScanTokens: 10_000,
          bestPackTokens: 2_500,
          bestPackTokensAvoided: 7_500,
          bestPackSavingsPct: 75,
          allPacksTokens: 4_000,
          allPacksTokensAvoided: 6_000,
          allPacksSavingsPct: 60,
          bestPack: {
            id: "implementation",
            title: "Implementation pack",
            purpose: "Build next slice",
            estimatedTokens: 2_500,
            savingsVsFullScanPct: 75,
            files: [],
          },
        },
        ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
      },
    );
    const summary = summarizeSavingsLedgerRows(
      rows,
      "lifetime",
      "2026-06-27T10:00:00.000Z",
    );

    expect(rows.map((row) => row.source)).toEqual([
      "headroom_engine",
      "rtk",
      "repo_intelligence",
      "ponytail",
    ]);
    expect(summary).toMatchObject({
      rowCount: 4,
      measuredTokens: 900,
      estimatedTokens: 9_500,
      inferredTokens: 880,
      totalTokens: 11_280,
      estimatedUsd: 4.5,
      measuredUsd: 0,
    });
    expect(formatSavingsLedgerConfidenceBreakdown(summary)).toBe(
      "900 measured · 9,500 estimated · 880 inferred",
    );
  });

  it("groups ledger rows by source and preserves the time window", () => {
    const recordedAt = "2026-06-27T10:00:00.000Z";
    const rows = buildSavingsLedgerRows(dashboardFixture(), "lifetime", recordedAt, {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
        },
      },
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_500,
        bestPackTokensAvoided: 7_500,
        bestPackSavingsPct: 75,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation pack",
          purpose: "Build next slice",
          estimatedTokens: 2_500,
          savingsVsFullScanPct: 75,
          files: [],
        },
      },
    });
    const groups = groupSavingsLedgerRowsBySource(rows, "lifetime", recordedAt);

    expect(groups.map((group) => group.source)).toEqual([
      "headroom_engine",
      "rtk",
      "repo_intelligence",
    ]);
    expect(groups.map((group) => group.scope)).toEqual([
      "lifetime",
      "lifetime",
      "lifetime",
    ]);
    expect(groups.map((group) => group.recordedAt)).toEqual([
      recordedAt,
      recordedAt,
      recordedAt,
    ]);
    expect(groups.map((group) => group.confidence)).toEqual([
      "estimated",
      "measured",
      "estimated",
    ]);
    expect(groups[2]).toMatchObject({
      label: "Repo Intelligence",
      inferredTokens: 0,
      measuredTokens: 0,
      estimatedTokens: 7_500,
    });
  });

  it("returns no source groups for an empty ledger", () => {
    expect(
      groupSavingsLedgerRowsBySource(
        [],
        "session",
        "2026-06-27T10:00:00.000Z",
      ),
    ).toEqual([]);
  });

  it("describes genuinely empty and filter-empty ledgers distinctly", () => {
    expect(getSavingsLedgerEmptyState(0, "all")).toEqual({
      title: "No savings ledger rows yet",
      detail:
        "Run a connected agent, index a repo, or enable an add-on estimate to populate measured, estimated, or inferred rows.",
    });
    expect(getSavingsLedgerEmptyState(3, "measured")).toEqual({
      title: "No matching ledger rows",
      detail:
        "No measured rows match this ledger view. Change the confidence filter to see other sources.",
    });
  });

  it("filters ledger rows by confidence before grouping and copying", () => {
    const recordedAt = "2026-06-27T10:00:00.000Z";
    const rows = buildSavingsLedgerRows(dashboardFixture(), "lifetime", recordedAt, {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
        },
      },
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_500,
        bestPackTokensAvoided: 7_500,
        bestPackSavingsPct: 75,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation pack",
          purpose: "Build next slice",
          estimatedTokens: 2_500,
          savingsVsFullScanPct: 75,
          files: [],
        },
      },
      ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
    });

    expect(filterSavingsLedgerRowsByConfidence(rows, "measured").map((row) => row.source)).toEqual([
      "rtk",
    ]);

    const inferred = buildFilteredSavingsLedger(
      rows,
      "lifetime",
      recordedAt,
      "inferred",
    );
    expect(inferred.summary).toMatchObject({
      rowCount: 1,
      measuredTokens: 0,
      estimatedTokens: 0,
      inferredTokens: 880,
      totalTokens: 880,
    });
    expect(inferred.groups.map((group) => group.source)).toEqual(["ponytail"]);

    const copied = formatSavingsLedgerShareText(
      inferred.rows,
      "lifetime",
      recordedAt,
      "inferred",
    );
    expect(copied).toContain("Rows: 1");
    expect(copied).toContain("Confidence filter: inferred");
    expect(copied).toContain(
      "- ponytail: Ponytail (inferred, lifetime, 2026-06-27T10:00:00.000Z)",
    );
    expect(copied).not.toContain("- repo_intelligence: Repo Intelligence");
    expect(copied).not.toContain("- rtk: RTK");
    expect(copied).not.toContain("- headroom_engine: Headroom");
  });

  it("formats a copyable savings ledger with confidence caveats", () => {
    const recordedAt = "2026-06-27T10:00:00.000Z";
    const rows = buildSavingsLedgerRows(dashboardFixture(), "lifetime", recordedAt, {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
        },
      },
      markitdownSavings: buildAddonSavingsEstimate(3_200, 900),
    });
    const text = formatSavingsLedgerShareText(rows, "lifetime", recordedAt);

    expect(text).toContain("Mac AI Switchboard savings ledger (lifetime)");
    expect(text).toContain("Recorded: 2026-06-27T10:00:00.000Z");
    expect(text).toContain("Confidence filter: all rows");
    expect(text).toContain(
      `Scope definition: ${savingsCalculatorScopeDefinition("lifetime")}`,
    );
    expect(text).toContain("Measured tokens: 900 / $0.00");
    expect(text).toContain("Estimated tokens: 4,300 / $4.50");
    expect(text).toContain("Inferred tokens: 0");
    expect(text).toContain("Attribution: 17.3% measured · 82.7% estimated");
    expect(text).toContain(
      "Equation per row: saved tokens come from each source's before/after or counter delta",
    );
    expect(text).toContain(
      "Confidence labels are not interchangeable: inferred rows are never reported as measured.",
    );
    expect(text).toContain("Evidence: 12 command outputs compressed locally.");
    expect(text).toContain(
      "Evidence: Markdown extract vs re-attaching the full source document each turn; the managed converter is smoke-tested before integration is enabled.",
    );
    expect(text).toContain("Caveat: Observed from local counters");
    expect(text).toContain(
      "Caveat: Estimated from a smoke-tested managed MarkItDown hook or instruction-file change",
    );
    expect(text).toContain(
      "- markitdown: MarkItDown (estimated, lifetime, 2026-06-27T10:00:00.000Z) saved 2,300 tokens.",
    );
  });

  it("formats savings attribution by confidence without overstating empty ledgers", () => {
    expect(
      formatSavingsLedgerAttributionSummary({
        totalTokens: 0,
        measuredTokens: 0,
        estimatedTokens: 0,
        inferredTokens: 0,
      }),
    ).toBe("No attributed savings yet.");

    expect(
      formatSavingsLedgerAttributionSummary({
        totalTokens: 1_000,
        measuredTokens: 250,
        estimatedTokens: 250,
        inferredTokens: 500,
      }),
    ).toBe("25% measured · 25% estimated · 50% inferred");
  });

  it("formats a copyable session savings summary", () => {
    const dashboard = dashboardFixture();
    const summary = buildSavingsCalculatorSummary(dashboard, "session");
    const rows = buildSavingsCalculatorBreakdown(dashboard, "session");
    const text = formatSavingsCalculatorShareText(summary, rows);

    expect(text).toContain("Mac AI Switchboard savings (current app session)");
    expect(text).toContain(
      `Scope definition: ${savingsCalculatorScopeDefinition("session")}`,
    );
    expect(text).toContain("Saved: 300 tokens / $0.75");
    expect(text).toContain(
      "Confidence: measured = observed local counters; estimated = saved history or cost estimate; inferred = modelled template or context-pack delta.",
    );
    expect(text).toContain(
      "- headroom_engine: Headroom (measured) saved 300 tokens / $0.75",
    );
  });

  it("formats a copyable lifetime savings summary with local sources", () => {
    const dashboard = dashboardFixture();
    const summary = buildSavingsCalculatorSummary(dashboard, "lifetime");
    const rows = buildSavingsCalculatorBreakdown(dashboard, "lifetime", {
      runtimeStatus: {
        platform: "darwin",
        supportTier: "supported",
        installed: true,
        running: true,
        starting: false,
        paused: false,
        autoPaused: false,
        proxyReachable: true,
        headroomLearnSupported: true,
        rtk: {
          installed: true,
          enabled: true,
          pathConfigured: true,
          hookConfigured: true,
          totalCommands: 12,
          totalSaved: 900,
          avgSavingsPct: 72,
        },
      },
      repoSavings: {
        fullScanTokens: 10_000,
        bestPackTokens: 2_500,
        bestPackTokensAvoided: 7_500,
        bestPackSavingsPct: 75,
        allPacksTokens: 4_000,
        allPacksTokensAvoided: 6_000,
        allPacksSavingsPct: 60,
        bestPack: {
          id: "implementation",
          title: "Implementation pack",
          purpose: "Build next slice",
          estimatedTokens: 2_500,
          savingsVsFullScanPct: 75,
          files: [],
        },
      },
      cavemanSavings: buildAddonSavingsEstimate(480, 180),
      ponytailSavings: buildAddonSavingsEstimate(1_400, 520),
      markitdownSavings: buildAddonSavingsEstimate(3_200, 900),
    });
    const text = formatSavingsCalculatorShareText(summary, rows);

    expect(text).toContain("Mac AI Switchboard savings (lifetime)");
    expect(text).toContain(
      `Scope definition: ${savingsCalculatorScopeDefinition("lifetime")}`,
    );
    expect(text).toContain("Saved: 2,000 tokens / $4.50");
    expect(text).toContain("Confidence: measured = observed local counters");
    expect(text).toContain(
      "- headroom_engine: Headroom (estimated) saved 2,000 tokens / $4.50",
    );
    expect(text).toContain("- rtk: RTK (measured) saved 900 tokens");
    expect(text).toContain(
      "- repo_intelligence: Repo Intelligence (estimated) saved 7,500 tokens",
    );
    expect(text).toContain("- caveman: Caveman (inferred) saved 300 tokens");
    expect(text).toContain("- ponytail: Ponytail (inferred) saved 880 tokens");
    expect(text).toContain(
      "- markitdown: MarkItDown (estimated) saved 2,300 tokens",
    );
  });

  it("defines savings scopes without mixing session, repo, and history windows", () => {
    expect(savingsCalculatorScopeDefinition("session")).toContain(
      "reset on app restart",
    );
    expect(savingsCalculatorScopeDefinition("session")).toContain(
      "not a repo total",
    );
    expect(savingsCalculatorScopeDefinition("repo")).toContain(
      "Repo Intelligence context-pack estimates",
    );
    expect(savingsCalculatorScopeDefinition("repo")).toContain(
      "Runtime and RTK traffic are excluded",
    );
    expect(savingsCalculatorScopeDefinition("today")).toContain(
      "current UTC date",
    );
    expect(savingsCalculatorScopeDefinition("lifetime")).toContain(
      "all saved local Switchboard history",
    );
  });
});
