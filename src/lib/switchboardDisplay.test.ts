import { describe, expect, it } from "vitest";

import {
  deriveSwitchboardMode,
  switchboardModeEffect,
  switchboardModeLabel,
  switchboardModeSummary,
} from "./switchboardDisplay";
import type {
  ClientConnectorStatus,
  RuntimeStatus,
  SwitchboardMode,
} from "./types";

function connector(
  overrides: Partial<ClientConnectorStatus> = {},
): ClientConnectorStatus {
  return {
    clientId: "codex",
    name: "Codex",
    installed: true,
    enabled: true,
    verified: true,
    lastConfiguredAt: null,
    ...overrides,
  };
}

function runtime(overrides: Partial<RuntimeStatus> = {}): RuntimeStatus {
  return {
    platform: "macos",
    supportTier: "stable",
    installed: true,
    running: true,
    starting: false,
    paused: false,
    autoPaused: false,
    proxyReachable: true,
    headroomPid: 123,
    mcpConfigured: true,
    mcpError: null,
    mlInstalled: null,
    kompressEnabled: true,
    headroomLearnSupported: true,
    headroomLearnDisabledReason: null,
    startupError: null,
    startupErrorHint: null,
    runtimeUpgradeFailure: null,
    rtk: {
      installed: true,
      enabled: true,
      version: "0.42.4",
      pathConfigured: true,
      hookConfigured: true,
      totalCommands: 10,
      totalSaved: 1000,
      avgSavingsPct: 80,
    },
    ...overrides,
  };
}

describe("switchboardDisplay", () => {
  it.each<[SwitchboardMode, string]>([
    ["off", "Off"],
    ["rtk", "RTK only"],
    ["headroom", "Headroom only"],
    ["full", "Full optimization"],
  ])("labels %s mode", (mode, label) => {
    expect(switchboardModeLabel(mode)).toBe(label);
    expect(switchboardModeSummary(mode).length).toBeGreaterThan(10);
    expect(switchboardModeEffect(mode).length).toBeGreaterThan(10);
  });

  it("derives full mode when Headroom and RTK are both active", () => {
    expect(deriveSwitchboardMode(runtime(), [connector()])).toBe("full");
  });

  it("derives headroom-only mode when RTK is disabled", () => {
    expect(
      deriveSwitchboardMode(
        runtime({ rtk: { ...runtime().rtk, enabled: false } }),
        [connector()],
      ),
    ).toBe("headroom");
  });

  it("derives RTK-only mode when no client is routed through Headroom", () => {
    expect(deriveSwitchboardMode(runtime(), [])).toBe("rtk");
  });

  it("derives off when runtime is paused even with an enabled client", () => {
    expect(
      deriveSwitchboardMode(runtime({ paused: true }), [connector()]),
    ).toBe("rtk");
  });

  it("derives off when neither Headroom nor RTK is active", () => {
    expect(
      deriveSwitchboardMode(
        runtime({ rtk: { ...runtime().rtk, enabled: false } }),
        [],
      ),
    ).toBe("off");
  });
});
