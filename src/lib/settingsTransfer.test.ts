import { describe, expect, it } from "vitest";

import {
  buildSettingsExportBundle,
  formatSettingsExportBundle,
  parseSettingsImport,
} from "./settingsTransfer";
import type {
  ClientConnectorStatus,
  DashboardState,
  ManagedTool,
} from "./types";

function tool(overrides: Partial<ManagedTool>): ManagedTool {
  return {
    id: "rtk",
    name: "RTK",
    description: "Command output compression",
    runtime: "binary",
    required: false,
    enabled: true,
    status: "healthy",
    sourceUrl: "https://example.com/rtk",
    version: "0.0.0",
    metadata: {
      secretPath: "/Users/me/.secret",
      token: "sk-not-real",
    },
    ...overrides,
  };
}

function dashboardFixture(): DashboardState {
  return {
    appVersion: "0.0.0",
    launchExperience: "dashboard",
    bootstrapComplete: true,
    pythonRuntimeInstalled: true,
    lifetimeRequests: 1,
    lifetimeEstimatedSavingsUsd: 12,
    lifetimeEstimatedTokensSaved: 123_456,
    sessionRequests: 1,
    sessionEstimatedSavingsUsd: 1,
    sessionEstimatedTokensSaved: 1_000,
    sessionSavingsPct: 25,
    outputReduction: null,
    dailySavings: [],
    hourlySavings: [],
    savingsHistoryLoaded: true,
    tools: [tool({ id: "rtk" }), tool({ id: "markitdown", runtime: "python" })],
    clients: [],
    recentUsage: [
      {
        id: "usage-1",
        timestamp: "2026-06-30T10:00:00Z",
        client: "Codex",
        workspace: "/Users/me/private-repo",
        upstreamTarget: "openai",
        stages: [],
        estimatedInputTokens: 100_000,
        estimatedOutputTokens: 2_000,
        estimatedCostSavingsUsd: 1,
        latencyMs: 100,
        outcome: "success",
      },
    ],
    insights: [],
    requiredTermsVersion: 1,
    acceptedTermsVersion: 1,
    termsUrl: "https://example.com/terms",
  };
}

function connector(overrides: Partial<ClientConnectorStatus>): ClientConnectorStatus {
  return {
    clientId: "codex",
    name: "Codex",
    installed: true,
    enabled: true,
    verified: true,
    supportStatus: "managed",
    setupPhase: "managed",
    setupHint: "Uses ~/.codex/config.toml",
    ...overrides,
  };
}

describe("settings transfer", () => {
  it("exports only non-secret app preferences and advisory connector state", () => {
    const bundle = buildSettingsExportBundle({
      dashboard: dashboardFixture(),
      connectors: [
        connector({ clientId: "codex" }),
        connector({
          clientId: "claude_code",
          name: "Claude Code",
          supportStatus: "managed",
        }),
      ],
      switchboardMode: "rtk",
      savingsMode: "aggressive",
      exportedAt: "2026-06-30T10:00:00.000Z",
    });
    const text = formatSettingsExportBundle(bundle);

    expect(bundle.preferences).toEqual({
      switchboardMode: "rtk",
      savingsMode: "aggressive",
    });
    expect(bundle.connectors).toEqual([
      { clientId: "claude_code", enabled: true, supportStatus: "managed" },
      { clientId: "codex", enabled: true, supportStatus: "managed" },
    ]);
    expect(bundle.addons).toEqual([
      { id: "markitdown", enabled: true, runtime: "python", status: "healthy" },
      { id: "rtk", enabled: true, runtime: "binary", status: "healthy" },
    ]);
    expect(text).not.toContain("/Users/me");
    expect(text).not.toContain("sk-not-real");
    expect(text).not.toContain("recentUsage");
    expect(text).not.toContain("lifetimeEstimatedTokensSaved");
    expect(text).toContain("No provider API keys");
  });

  it("previews valid imports and keeps connector/add-on state manual", () => {
    const text = formatSettingsExportBundle(
      buildSettingsExportBundle({
        dashboard: dashboardFixture(),
        connectors: [connector({ clientId: "codex" })],
        switchboardMode: "headroom",
        savingsMode: "balanced",
        exportedAt: "2026-06-30T10:00:00.000Z",
      }),
    );

    const preview = parseSettingsImport(text);

    expect(preview).toMatchObject({
      valid: true,
      title: "Settings import ready",
      safePreferences: {
        switchboardMode: "headroom",
        savingsMode: "balanced",
      },
      errors: [],
    });
    expect(preview.manualItems).toContain(
      "Connector codex: enabled in export; review manually before applying config.",
    );
    expect(preview.manualItems).toContain(
      "Add-on rtk: enabled in export; install or enable from Addons if wanted.",
    );
    expect(preview.migrationActions).toEqual(
      expect.arrayContaining([
        {
          id: "preferences",
          label: "App preferences",
          status: "safe",
          detail:
            "Switchboard mode and savings profile can be applied without touching provider config.",
        },
        {
          id: "connector:codex",
          label: "Connector codex",
          status: "manual",
          detail:
            "Managed connector state is advisory; native config changes still require the connector's backup, verify, rollback, Doctor, and Off cleanup gates.",
        },
        {
          id: "addon:rtk",
          label: "Add-on rtk",
          status: "manual",
          detail:
            "Healthy add-on state is advisory; install, enable, or repair it from Addons so local runtime checks stay explicit.",
        },
      ]),
    );
  });

  it("keeps legacy managed connector imports on the managed-gate copy path", () => {
    const preview = parseSettingsImport(
      JSON.stringify({
        schemaVersion: 1,
        preferences: { switchboardMode: "headroom", savingsMode: "balanced" },
        connectors: [
          { clientId: "codex", enabled: true },
          { clientId: "cursor", enabled: true, supportStatus: "planned" },
        ],
        addons: [],
      }),
    );

    expect(preview.migrationActions).toEqual(
      expect.arrayContaining([
        {
          id: "connector:codex",
          label: "Connector codex",
          status: "manual",
          detail:
            "Managed connector state is advisory; native config changes still require the connector's backup, verify, rollback, Doctor, and Off cleanup gates.",
        },
        {
          id: "connector:cursor",
          label: "Connector cursor",
          status: "manual",
          detail:
            "Connector state is advisory and must be reviewed from Connectors before any local config changes.",
        },
      ]),
    );
  });

  it("rejects invalid or unsupported import bundles", () => {
    expect(parseSettingsImport("not-json")).toMatchObject({
      valid: false,
      title: "Settings import is not valid JSON",
    });

    expect(
      parseSettingsImport(
        JSON.stringify({
          schemaVersion: 999,
          preferences: { switchboardMode: "turbo", savingsMode: "balanced" },
        }),
      ),
    ).toMatchObject({
      valid: false,
      errors: [
        "Unsupported schema version: 999.",
        "Missing or invalid switchboard mode.",
      ],
      migrationActions: [
        expect.objectContaining({
          id: "preferences",
          status: "blocked",
        }),
      ],
    });
  });
});
