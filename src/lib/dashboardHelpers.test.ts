import { afterEach, describe, expect, it, vi } from "vitest";
import {
  aggregateClientConnectors,
  buildHourlySavingsChartData,
  buildHourlySavingsWindow,
  buildMonthlySavingsChartData,
  buildMonthlySavingsWindow,
  compactNumber,
  connectorControlState,
  connectorCompatibilityReport,
  connectorDashboardStatus,
  connectorSupportsAutomaticSetup,
  currency,
  currencyExact,
  dayOfMonthTickFormatter,
  earliestHourlyDay,
  earliestSavingsMonth,
  formatConnectorConfigDryRunPreview,
  formatDateTime,
  formatDayKey,
  formatLearnStatus,
  getEnabledSupportedConnectors,
  formatPlannedConnectorConfigGateSummary,
  hasEnabledConnector,
  hourOfDayTickFormatter,
  mergeProviderSavingsForDisplay,
  percent1,
  plannedConnectorCompatibilityReportConfigs,
  sortClientConnectors,
  summarizePlannedConnectorReadiness,
} from "./dashboardHelpers";
import { managedConnectorDossiers, plannedConnectors } from "./plannedConnectors";

import type {
  ClientConnectorStatus,
  DailySavingsPoint,
  HourlySavingsPoint,
} from "./types";

const expectedConfigCreationGates = [
  { id: "detect", label: "Detect config surface" },
  { id: "dryRunDiff", label: "Show dry-run diff" },
  { id: "backup", label: "Create backup" },
  { id: "apply", label: "Apply with consent" },
  { id: "verify", label: "Verify in Doctor" },
  { id: "rollback", label: "Rollback safely" },
  { id: "offCleanup", label: "Clean up in Off mode" },
];

describe("dashboard helpers", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("formats stable numeric summaries", () => {
    expect(currencyExact(12.345)).toBe("$12.35");
    expect(currency(9999)).toBe("$9,999");
    expect(currency(15_432)).toContain("K");
    expect(compactNumber(12_345)).toBe("12.3K");
    expect(percent1(18)).toBe("18.0");
  });

  it("builds full monthly windows with zero-filled gaps", () => {
    const data: DailySavingsPoint[] = [
      {
        date: "2024-02-02",
        estimatedSavingsUsd: 2.5,
        estimatedTokensSaved: 250,
        actualCostUsd: 1.5,
        totalTokensSent: 1000,
      },
    ];

    const month = new Date(2024, 1, 18);
    const windowed = buildMonthlySavingsWindow(data, month);

    expect(windowed).toHaveLength(29);
    expect(windowed[0]).toEqual({
      date: "2024-02-01",
      estimatedSavingsUsd: 0,
      estimatedTokensSaved: 0,
      actualCostUsd: 0,
      totalTokensSent: 0,
    });
    expect(windowed[1]).toEqual(data[0]);
    expect(windowed[28].date).toBe("2024-02-29");
  });

  it("builds hourly windows and chart data with derived totals", () => {
    const data: HourlySavingsPoint[] = [
      {
        hour: "2024-03-05T04:00",
        estimatedSavingsUsd: 1.25,
        estimatedTokensSaved: 125,
        actualCostUsd: 0.75,
        totalTokensSent: 500,
        byProvider: [
          {
            provider: "anthropic",
            estimatedSavingsUsd: 1.25,
            estimatedTokensSaved: 125,
            actualCostUsd: 0.75,
            totalTokensSent: 500,
          },
        ],
      },
    ];

    const windowed = buildHourlySavingsWindow(data, new Date(2024, 2, 5, 12));
    const chartData = buildHourlySavingsChartData(windowed);

    expect(windowed).toHaveLength(24);
    expect(windowed[4]).toEqual(data[0]);
    expect(windowed[3].hour).toBe("2024-03-05T03:00");
    expect(chartData[4]).toMatchObject({
      bucketKey: "2024-03-05T04:00",
      estimatedSavingsUsd: 1.25,
      estimatedTokensSaved: 125,
      actualCostUsd: 0.75,
      totalTokensSent: 500,
      totalCostBeforeOptimization: 2,
      totalTokensBeforeOptimization: 625,
    });
    // Per-provider breakdown carries through; padded hours default to empty.
    expect(chartData[4].byProvider).toEqual(data[0].byProvider);
    expect(chartData[3].byProvider).toEqual([]);
  });

  it("builds monthly chart data and finds earliest visible history", () => {
    const dailyData: DailySavingsPoint[] = [
      {
        date: "2024-01-30",
        estimatedSavingsUsd: 1,
        estimatedTokensSaved: 100,
        actualCostUsd: 3,
        totalTokensSent: 1000,
      },
      {
        date: "2024-03-01",
        estimatedSavingsUsd: 2,
        estimatedTokensSaved: 200,
        actualCostUsd: 4,
        totalTokensSent: 2000,
      },
    ];
    const hourlyData: HourlySavingsPoint[] = [
      {
        hour: "2024-02-14T21:00",
        estimatedSavingsUsd: 0.5,
        estimatedTokensSaved: 50,
        actualCostUsd: 1,
        totalTokensSent: 300,
        byProvider: [],
      },
    ];

    const chartData = buildMonthlySavingsChartData(dailyData);

    expect(chartData[0]).toMatchObject({
      bucketKey: "2024-01-30",
      totalCostBeforeOptimization: 4,
      totalTokensBeforeOptimization: 1100,
    });
    expect(formatDayKey(earliestSavingsMonth(dailyData) as Date)).toBe(
      "2024-01-01",
    );
    expect(formatDayKey(earliestHourlyDay(hourlyData) as Date)).toBe(
      "2024-02-14",
    );
  });

  it("formats chart ticks predictably", () => {
    expect(dayOfMonthTickFormatter("2024-02-01")).toBe("1");
    expect(dayOfMonthTickFormatter("2024-02-02")).toBe("");
    expect(dayOfMonthTickFormatter("2024-02-29")).toBe("29");
    expect(hourOfDayTickFormatter("2024-02-01T04:00")).toBe("04");
    expect(hourOfDayTickFormatter("2024-02-01T05:00")).toBe("");
    expect(hourOfDayTickFormatter("2024-02-01T23:00")).toBe("23");
  });

  it("filters and sorts client connectors", () => {
    const connectors: ClientConnectorStatus[] = [
      {
        clientId: "zed",
        name: "Zed",
        installed: false,
        enabled: false,
        verified: false,
      },
      {
        clientId: "claude_code",
        name: "Claude Code",
        installed: true,
        enabled: true,
        verified: true,
      },
      {
        clientId: "cursor",
        name: "Cursor",
        supportStatus: "planned",
        installed: true,
        enabled: false,
        verified: false,
      },
    ];

    expect(aggregateClientConnectors(connectors)).toEqual([
      connectors[1],
      connectors[2],
    ]);
    expect(
      sortClientConnectors(connectors).map((connector) => connector.clientId),
    ).toEqual(["claude_code", "cursor", "zed"]);
  });

  it("keeps managed and planned switchboard connectors visible", () => {
    const connectors: ClientConnectorStatus[] = [
      {
        clientId: "codex",
        name: "Codex",
        installed: true,
        enabled: false,
        verified: false,
      },
      {
        clientId: "claude_code",
        name: "Claude Code",
        installed: true,
        enabled: true,
        verified: true,
      },
      {
        clientId: "gemini_cli",
        name: "Gemini CLI",
        supportStatus: "planned",
        installed: true,
        enabled: false,
        verified: false,
      },
      {
        clientId: "opencode",
        name: "OpenCode",
        supportStatus: "planned",
        installed: false,
        enabled: false,
        verified: false,
      },
      {
        clientId: "cursor",
        name: "Cursor",
        supportStatus: "planned",
        installed: true,
        enabled: false,
        verified: false,
      },
      {
        clientId: "grok_cli",
        name: "Grok / xAI CLI",
        supportStatus: "planned",
        installed: false,
        enabled: false,
        verified: false,
      },
      {
        clientId: "aider",
        name: "Aider",
        supportStatus: "planned",
        installed: true,
        enabled: false,
        verified: false,
      },
      {
        clientId: "continue",
        name: "Continue",
        supportStatus: "planned",
        installed: false,
        enabled: false,
        verified: false,
      },
      {
        clientId: "goose",
        name: "Goose",
        supportStatus: "planned",
        installed: true,
        enabled: false,
        verified: false,
      },
      {
        clientId: "zed",
        name: "Zed",
        installed: true,
        enabled: false,
        verified: false,
      },
    ];

    expect(
      aggregateClientConnectors(connectors)
        .map((connector) => connector.clientId)
        .sort(),
    ).toEqual([
      "aider",
      "claude_code",
      "codex",
      "continue",
      "cursor",
      "gemini_cli",
      "goose",
      "grok_cli",
      "opencode",
    ]);
  });

  it("reports enabled supported connectors regardless of which tool", () => {
    const connectors: ClientConnectorStatus[] = [
      {
        clientId: "claude_code",
        name: "Claude Code",
        installed: true,
        enabled: false,
        verified: false,
      },
      {
        clientId: "codex",
        name: "Codex",
        installed: true,
        enabled: true,
        verified: true,
      },
      {
        clientId: "gemini_cli",
        name: "Gemini CLI",
        supportStatus: "planned",
        installed: true,
        enabled: true,
        verified: false,
      },
      {
        clientId: "cursor",
        name: "Cursor",
        supportStatus: "planned",
        setupPhase: "guide",
        installed: true,
        enabled: true,
        verified: true,
      },
      {
        clientId: "goose",
        name: "Goose",
        setupPhase: "adapt",
        installed: true,
        enabled: true,
        verified: true,
      },
    ];

    expect(
      getEnabledSupportedConnectors(connectors).map((c) => c.clientId),
    ).toEqual(["codex"]);
    expect(connectorSupportsAutomaticSetup(connectors[1])).toBe(true);
    expect(connectorSupportsAutomaticSetup(connectors[4])).toBe(false);
    expect(hasEnabledConnector(connectors)).toBe(true);
    expect(
      hasEnabledConnector([
        {
          clientId: "claude_code",
          name: "Claude Code",
          installed: true,
          enabled: false,
          verified: false,
        },
      ]),
    ).toBe(false);
  });

  it("disables planned connector controls with RTK-only guidance", () => {
    expect(
      connectorControlState({
        clientId: "cursor",
        name: "Cursor",
        supportStatus: "planned",
        setupHint:
          "Manual guide only. Reversible Cursor profile routing is planned.",
        installed: true,
        enabled: false,
        verified: false,
      }),
    ).toEqual({
      disabled: true,
      reason:
        "Cursor is detected, but automatic routing is not available yet. Manual guide only. Reversible Cursor profile routing is planned.",
    });

    expect(
      connectorControlState({
        clientId: "grok_cli",
        name: "Grok / xAI CLI",
        supportStatus: "planned",
        installed: false,
        enabled: false,
        verified: false,
      }),
    ).toEqual({
      disabled: true,
      reason:
        "Grok / xAI CLI support is planned for a later release. Use RTK-only mode for command output savings today.",
    });

    expect(
      connectorControlState({
        clientId: "codex",
        name: "Codex",
        installed: false,
        enabled: false,
        verified: false,
      }),
    ).toEqual({ disabled: false, reason: null });
  });

  it("keeps compatibility report config coverage for every managed and planned connector", () => {
    expect(
      Object.keys(plannedConnectorCompatibilityReportConfigs).sort(),
    ).toEqual(
      [...managedConnectorDossiers, ...plannedConnectors]
        .map((connector) => connector.id)
        .sort(),
    );
  });

  it("exposes config creation gates for every planned connector compatibility report", () => {
    for (const connector of plannedConnectors) {
      const report = connectorCompatibilityReport({
        clientId: connector.id,
        name: connector.name,
        supportStatus: "planned",
        detectionEvidence: [
          `${plannedConnectorCompatibilityReportConfigs[connector.id]?.pathPrefix} /tmp/${connector.id}`,
        ],
        installed: true,
        enabled: false,
        verified: false,
      });

      expect(report?.automationEnabled).toBe(false);
      expect(report?.configCreationGates).toEqual(expectedConfigCreationGates);
    }
  });

  it("formats config creation gate summaries for planned connector cards", () => {
    const summary = formatPlannedConnectorConfigGateSummary({
      clientId: "aider",
      name: "Aider",
      supportStatus: "planned",
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(summary).toEqual({
      title: "Config creation gates",
      detail:
        "7 gates required before automatic setup: Detect config surface -> Show dry-run diff -> Create backup -> Apply with consent -> Verify in Doctor -> Rollback safely -> Clean up in Off mode",
      nextGateLabel: "Detect config surface",
      automationEnabled: false,
      safetyNote:
        "Config creation remains gated until every step has tests and Doctor evidence.",
    });
    expect(
      formatPlannedConnectorConfigGateSummary({
        clientId: "claude_code",
        name: "Claude Code",
        installed: true,
        enabled: true,
        verified: true,
      }),
    ).toBeNull();
  });

  it("formats Gemini compatibility evidence for managed connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "gemini_cli",
      name: "Gemini CLI",
      supportStatus: "planned",
      setupPhase: "guide",
      detectionEvidence: [
        "Gemini binary: /opt/homebrew/bin/gemini",
        "Gemini version: gemini 0.2.1",
        "Gemini config surface: /Users/test/.gemini",
        "Provider routing blocked until stable config surface, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but the Headroom adapter is not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Gemini compatibility report",
      primaryPathLabel: "Binary",
      binaryPath: "/opt/homebrew/bin/gemini",
      version: "gemini 0.2.1",
      configSurface: "/Users/test/.gemini",
      routingBlocker:
        "Provider routing blocked until stable config surface, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: [],
      otherEvidence: ["Detected, but the Headroom adapter is not implemented yet."],
    });
  });

  it("formats a Gemini config dry-run preview from detected config evidence", () => {
    expect(
      formatConnectorConfigDryRunPreview({
        clientId: "gemini_cli",
        name: "Gemini CLI",
        supportStatus: "planned",
        detectionEvidence: [
          "Gemini binary: /opt/homebrew/bin/gemini",
          "Gemini config surface: /Users/test/.gemini",
        ],
        installed: true,
        enabled: false,
        verified: false,
      }),
    ).toContain(
      [
        "## Dry-run diff preview",
        "- Target: /Users/test/.gemini",
        "- Marker: mac-ai-switchboard:gemini_cli",
        "- Backup: /Users/test/.gemini.mac-ai-switchboard.bak",
        "- Current managed block: none detected",
        "- Proposed managed block: Mac AI Switchboard provider routing for Gemini CLI",
        "- Apply blocked: detection, dry-run diff, backup, verify, rollback, and Off cleanup evidence are incomplete",
        "- Writes: none; preview only; apply stays disabled",
        "- Rollback: Restore the previous provider settings or remove only Switchboard-managed shell routing.",
        `- Gates: ${expectedConfigCreationGates.map((gate) => gate.label).join(" -> ")}`,
      ].join("\n"),
    );
  });

  it("prefers backend-owned Gemini dry-run preview when present", () => {
    expect(
      formatConnectorConfigDryRunPreview({
        clientId: "gemini_cli",
        name: "Gemini CLI",
        supportStatus: "planned",
        detectionEvidence: [
          "Gemini binary: /opt/homebrew/bin/gemini",
          "Gemini config surface: /Users/test/.gemini",
        ],
        configDryRunPreview: {
          target: "/Users/test/.gemini",
          marker: "mac-ai-switchboard:gemini_cli",
          backupPath: "/Users/test/.gemini.mac-ai-switchboard.bak",
          currentState: "No Switchboard-managed Gemini provider routing detected.",
          proposedState:
            "Add Mac AI Switchboard local provider routing for Gemini CLI after explicit consent.",
          applyBlockedReason:
            "Gemini CLI automation is disabled until backup, verify, rollback, and Off cleanup gates pass.",
          rollbackPreview:
            "Restore the Gemini config backup or remove only the Switchboard-managed provider block.",
          confirmationPhrase: "APPLY GEMINI CLI CONFIG",
          writes: [],
        },
        installed: true,
        enabled: false,
        verified: false,
      }),
    ).toContain(
      [
        "## Dry-run diff preview",
        "- Target: /Users/test/.gemini",
        "- Marker: mac-ai-switchboard:gemini_cli",
        "- Backup: /Users/test/.gemini.mac-ai-switchboard.bak",
        "- Current managed block: No Switchboard-managed Gemini provider routing detected.",
        "- Proposed managed block: Add Mac AI Switchboard local provider routing for Gemini CLI after explicit consent.",
        "- Apply blocked: Gemini CLI automation is disabled until backup, verify, rollback, and Off cleanup gates pass.",
        "- Writes: none; preview only; apply stays disabled",
        "- Rollback: Restore the Gemini config backup or remove only the Switchboard-managed provider block.",
        "- Confirmation phrase: APPLY GEMINI CLI CONFIG",
      ].join("\n"),
    );
  });

  it("formats OpenCode compatibility evidence for managed connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "opencode",
      name: "OpenCode",
      supportStatus: "planned",
      setupPhase: "adapt",
      detectionEvidence: [
        "OpenCode binary: /opt/homebrew/bin/opencode",
        "OpenCode version: opencode 1.0.0",
        "OpenCode config surface: /Users/test/.config/opencode",
        "Provider routing blocked until active config path, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but the Headroom adapter is not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "OpenCode compatibility report",
      primaryPathLabel: "Binary",
      binaryPath: "/opt/homebrew/bin/opencode",
      version: "opencode 1.0.0",
      configSurface: "/Users/test/.config/opencode",
      routingBlocker:
        "Provider routing blocked until active config path, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: [],
      otherEvidence: ["Detected, but the Headroom adapter is not implemented yet."],
    });
  });

  it("formats Grok/xAI compatibility evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "grok_cli",
      name: "Grok / xAI CLI",
      supportStatus: "planned",
      setupPhase: "detect",
      detectionEvidence: [
        "Grok / xAI binary: /opt/homebrew/bin/xai",
        "Grok / xAI version: xai 0.4.0",
        "Grok / xAI config surface: /Users/test/.config/xai",
        "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Grok / xAI compatibility report",
      primaryPathLabel: "Binary",
      binaryPath: "/opt/homebrew/bin/xai",
      version: "xai 0.4.0",
      configSurface: "/Users/test/.config/xai",
      routingBlocker:
        "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter not implemented yet."],
    });
  });

  it("formats Cursor profile evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "cursor",
      name: "Cursor",
      supportStatus: "planned",
      setupPhase: "guide",
      detectionEvidence: [
        "Cursor app: /Applications/Cursor.app",
        "Cursor profile settings: /Users/test/Library/Application Support/Cursor",
        "Settings routing blocked until active profile detection, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Cursor compatibility report",
      primaryPathLabel: "App",
      binaryPath: "/Applications/Cursor.app",
      version: null,
      configSurface: "/Users/test/Library/Application Support/Cursor",
      routingBlocker:
        "Settings routing blocked until active profile detection, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter not implemented yet."],
    });
  });

  it("formats Aider compatibility evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "aider",
      name: "Aider",
      supportStatus: "planned",
      setupPhase: "adapt",
      detectionEvidence: [
        "Aider binary: /opt/homebrew/bin/aider",
        "Aider version: aider 0.84.0",
        "Aider config surface: /Users/test/.aider.conf.yml",
        "Provider routing blocked until reversible environment wrapper, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Aider compatibility report",
      primaryPathLabel: "Binary",
      binaryPath: "/opt/homebrew/bin/aider",
      version: "aider 0.84.0",
      configSurface: "/Users/test/.aider.conf.yml",
      routingBlocker:
        "Provider routing blocked until reversible environment wrapper, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter not implemented yet."],
    });
  });

  it("formats Continue config-folder evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "continue",
      name: "Continue",
      supportStatus: "planned",
      setupPhase: "guide",
      detectionEvidence: [
        "Continue command: /opt/homebrew/bin/continue",
        "Continue config folder: /Users/test/.continue",
        "Settings routing blocked until multi-provider parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Continue compatibility report",
      primaryPathLabel: "Command",
      binaryPath: "/opt/homebrew/bin/continue",
      version: null,
      configSurface: "/Users/test/.continue",
      routingBlocker:
        "Settings routing blocked until multi-provider parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter not implemented yet."],
    });
  });

  it("formats Goose compatibility evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "goose",
      name: "Goose",
      supportStatus: "planned",
      setupPhase: "adapt",
      detectionEvidence: [
        "Goose binary: /opt/homebrew/bin/goose",
        "Goose version: goose 1.2.0",
        "Goose config surface: /Users/test/.config/goose",
        "Provider routing blocked until MCP handoff shape, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Goose compatibility report",
      primaryPathLabel: "Binary",
      binaryPath: "/opt/homebrew/bin/goose",
      version: "goose 1.2.0",
      configSurface: "/Users/test/.config/goose",
      routingBlocker:
        "Provider routing blocked until MCP handoff shape, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter not implemented yet."],
    });
  });

  it("formats Qwen Code compatibility evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "qwen_code",
      name: "Qwen Code",
      supportStatus: "planned",
      setupPhase: "guide",
      detectionEvidence: [
        "Qwen Code binary: /opt/homebrew/bin/qwen-code",
        "Qwen Code version: qwen-code 0.9.0",
        "Qwen Code config surface: /Users/test/.qwen",
        "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter is not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Qwen Code compatibility report",
      primaryPathLabel: "Binary",
      binaryPath: "/opt/homebrew/bin/qwen-code",
      version: "qwen-code 0.9.0",
      configSurface: "/Users/test/.qwen",
      routingBlocker:
        "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter is not implemented yet."],
    });
  });

  it("formats Amazon Q compatibility evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "amazon_q",
      name: "Amazon Q Developer CLI",
      supportStatus: "planned",
      setupPhase: "detect",
      detectionEvidence: [
        "Amazon Q binary: /opt/homebrew/bin/q",
        "Amazon Q version: q 1.11.0",
        "Amazon Q config surface: /Users/test/.aws/amazonq",
        "Provider routing blocked until AWS/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter is not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Amazon Q compatibility report",
      primaryPathLabel: "Binary",
      binaryPath: "/opt/homebrew/bin/q",
      version: "q 1.11.0",
      configSurface: "/Users/test/.aws/amazonq",
      routingBlocker:
        "Provider routing blocked until AWS/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter is not implemented yet."],
    });
  });

  it("formats Windsurf settings evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "windsurf",
      name: "Windsurf",
      supportStatus: "planned",
      setupPhase: "guide",
      detectionEvidence: [
        "Windsurf app: /Applications/Windsurf.app",
        "Windsurf settings: /Users/test/Library/Application Support/Windsurf",
        "Settings routing blocked until profile-aware discovery, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter is not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Windsurf compatibility report",
      primaryPathLabel: "App",
      binaryPath: "/Applications/Windsurf.app",
      version: null,
      configSurface: "/Users/test/Library/Application Support/Windsurf",
      routingBlocker:
        "Settings routing blocked until profile-aware discovery, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter is not implemented yet."],
    });
  });

  it("formats Zed settings evidence for planned connector UI", () => {
    const report = connectorCompatibilityReport({
      clientId: "zed_ai",
      name: "Zed AI",
      supportStatus: "planned",
      setupPhase: "guide",
      detectionEvidence: [
        "Zed app: /Applications/Zed.app",
        "Zed assistant settings: /Users/test/.config/zed",
        "Settings routing blocked until lossless settings parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.",
        "Detected, but Headroom adapter is not implemented yet.",
      ],
      installed: true,
      enabled: false,
      verified: false,
    });

    expect(report).toEqual({
      title: "Zed AI compatibility report",
      primaryPathLabel: "App",
      binaryPath: "/Applications/Zed.app",
      version: null,
      configSurface: "/Users/test/.config/zed",
      routingBlocker:
        "Settings routing blocked until lossless settings parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.",
      automationEnabled: false,
      configCreationGates: expectedConfigCreationGates,
      otherEvidence: ["Detected, but Headroom adapter is not implemented yet."],
    });
  });

  it("derives a dashboard status label/tone per connector state", () => {
    expect(
      connectorDashboardStatus({
        clientId: "codex",
        name: "Codex",
        installed: false,
        enabled: false,
        verified: false,
      }),
    ).toEqual({ label: "Not installed", tone: "idle" });
    expect(
      connectorDashboardStatus({
        clientId: "codex",
        name: "Codex",
        installed: true,
        enabled: false,
        verified: false,
      }),
    ).toEqual({ label: "Off", tone: "idle" });
    expect(
      connectorDashboardStatus({
        clientId: "codex",
        name: "Codex",
        installed: true,
        enabled: true,
        verified: false,
      }),
    ).toEqual({ label: "Verifying", tone: "pending" });
    expect(
      connectorDashboardStatus({
        clientId: "codex",
        name: "Codex",
        installed: true,
        enabled: true,
        verified: true,
      }),
    ).toEqual({ label: "Active", tone: "active" });
    expect(
      connectorDashboardStatus({
        clientId: "amazon_q",
        name: "Amazon Q Developer CLI",
        supportStatus: "planned",
        installed: true,
        enabled: false,
        verified: false,
      }),
    ).toEqual({ label: "Planned", tone: "pending" });
    expect(
      connectorDashboardStatus({
        clientId: "cursor",
        name: "Cursor",
        supportStatus: "planned",
        setupPhase: "guide",
        installed: true,
        enabled: false,
        verified: false,
      }),
    ).toEqual({ label: "guide", tone: "pending" });
    expect(
      connectorDashboardStatus({
        clientId: "grok_cli",
        name: "Grok / xAI CLI",
        supportStatus: "planned",
        installed: false,
        enabled: false,
        verified: false,
      }),
    ).toEqual({ label: "Coming soon", tone: "idle" });
  });

  it("formats timestamps and learn recency with clear fallbacks", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-27T12:00:00Z"));

    expect(formatDateTime(null)).toBe("Never");
    expect(formatDateTime("not-a-date")).toBe("Unknown");
    expect(formatLearnStatus({ lastLearnRanAt: null })).toBe("never scan");
    expect(formatLearnStatus({ lastLearnRanAt: "invalid" })).toBe("never scan");
    expect(formatLearnStatus({ lastLearnRanAt: "2026-03-27T08:00:00Z" })).toBe(
      "last scan: today",
    );
    expect(formatLearnStatus({ lastLearnRanAt: "2026-03-26T08:00:00Z" })).toBe(
      "last scan: yesterday",
    );
    expect(formatLearnStatus({ lastLearnRanAt: "2026-03-22T08:00:00Z" })).toBe(
      "last scan: 5 days ago",
    );
  });
});

describe("mergeProviderSavingsForDisplay", () => {
  it("folds anthropic and unknown into Claude Code (listed first) and openai into Codex", () => {
    const merged = mergeProviderSavingsForDisplay([
      {
        provider: "openai",
        estimatedSavingsUsd: 0.04,
        estimatedTokensSaved: 40,
        actualCostUsd: 0.16,
        totalTokensSent: 80,
      },
      {
        provider: "anthropic",
        estimatedSavingsUsd: 0.1,
        estimatedTokensSaved: 100,
        actualCostUsd: 0.24,
        totalTokensSent: 120,
      },
      {
        provider: "unknown",
        estimatedSavingsUsd: 0.01,
        estimatedTokensSaved: 15,
        actualCostUsd: 0.03,
        totalTokensSent: 20,
      },
    ]);

    expect(merged).toEqual([
      {
        label: "Claude Code",
        estimatedSavingsUsd: 0.1 + 0.01,
        estimatedTokensSaved: 115,
        actualCostUsd: 0.24 + 0.03,
        totalTokensSent: 140,
      },
      {
        label: "Codex",
        estimatedSavingsUsd: 0.04,
        estimatedTokensSaved: 40,
        actualCostUsd: 0.16,
        totalTokensSent: 80,
      },
    ]);
  });

  it("omits a connector with no attributed providers", () => {
    const merged = mergeProviderSavingsForDisplay([
      {
        provider: "anthropic",
        estimatedSavingsUsd: 0.1,
        estimatedTokensSaved: 100,
        actualCostUsd: 0.24,
        totalTokensSent: 120,
      },
    ]);

    expect(merged).toHaveLength(1);
    expect(merged[0].label).toBe("Claude Code");
  });

  it("returns nothing for an empty breakdown", () => {
    expect(mergeProviderSavingsForDisplay([])).toEqual([]);
  });

  it("summarizes planned connector readiness without enabling automation", () => {
    const connectors: ClientConnectorStatus[] = [
      {
        clientId: "claude_code",
        name: "Claude Code",
        installed: true,
        enabled: true,
        verified: true,
      },
      {
        clientId: "grok_cli",
        name: "Grok / xAI CLI",
        supportStatus: "planned",
        setupPhase: "detect",
        installed: true,
        enabled: false,
        verified: false,
      },
      {
        clientId: "aider",
        name: "Aider",
        supportStatus: "planned",
        setupPhase: "adapt",
        installed: false,
        enabled: false,
        verified: false,
      },
      {
        clientId: "cursor",
        name: "Cursor",
        supportStatus: "planned",
        installed: true,
        enabled: false,
        verified: false,
      },
    ];

    expect(summarizePlannedConnectorReadiness(connectors)).toEqual({
      detectedCount: 2,
      manualOnlyCount: 3,
      notDetectedCount: 1,
      safeTodayCount: 9,
      plannedCapabilityCount: 10,
      automationGateCount: 27,
      detectedNames: ["Grok / xAI CLI", "Cursor"],
      notDetectedNames: ["Aider"],
      headline: "2 planned tools detected locally",
      detail:
        "Grok / xAI CLI, Cursor are read-only today. Not found: Aider. 9 safe capabilities are available now; 10 remain gated behind 27 backup, restore, and Off mode checks. Automatic routing stays locked until backup, restore, and Off mode cleanup ship.",
    });
  });
});
