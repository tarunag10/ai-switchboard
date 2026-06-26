import { describe, expect, it } from "vitest";

import {
  deriveSwitchboardMode,
  formatSwitchboardModeShareText,
  switchboardAttentionCopy,
  switchboardModeEffect,
  switchboardModeFootprint,
  switchboardModeLabel,
  switchboardModeSafetyNotes,
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

  it("describes switchboard mode effects clearly", () => {
    expect(switchboardModeSummary("full")).toBe(
      "Headroom proxy routing and RTK command compression are both active.",
    );
    expect(switchboardModeEffect("full")).toBe(
      "Routes supported clients through Headroom and compresses shell output with RTK.",
    );
    expect(switchboardModeEffect("rtk")).toBe(
      "Keeps client traffic direct and compresses shell output with RTK.",
    );
    expect(switchboardModeEffect("off")).toBe(
      "Removes routing hooks and leaves client traffic and shell commands unmodified.",
    );
  });

  it("explains off mode safety without deleting local repo intelligence state", () => {
    expect(switchboardModeSafetyNotes("off")).toEqual([
      "Routing hooks and RTK shell integration are disabled for normal client behavior.",
      "Repo Intelligence summaries remain local until cleared from Addons.",
    ]);
  });

  it("describes each mode local footprint", () => {
    expect(switchboardModeFootprint("full")).toEqual([
      {
        label: "Client routing",
        state: "on",
        detail: "Managed through Headroom",
      },
      {
        label: "Shell output",
        state: "on",
        detail: "RTK compacts noisy commands",
      },
      { label: "Repo packs", state: "local", detail: "Local copy/export only" },
    ]);
    expect(switchboardModeFootprint("rtk").map((item) => item.state)).toEqual([
      "off",
      "on",
      "local",
    ]);
    expect(
      switchboardModeFootprint("off").map((item) => item.detail),
    ).toContain("Saved locally until cleared");
  });

  it("explains requested/effective mode mismatches", () => {
    expect(switchboardAttentionCopy("full", "rtk")).toBe(
      "Active now: RTK only. Connect a supported client or repair Headroom routing in Doctor.",
    );
    expect(switchboardAttentionCopy("full", "off")).toBe(
      "Active now: Off. Run Doctor to restore Headroom and RTK together.",
    );
    expect(switchboardAttentionCopy("rtk", "off")).toBe(
      "Active now: Off. Install or enable RTK from Doctor or Addons.",
    );
    expect(switchboardAttentionCopy("off", "full")).toBe(
      "Active now: Full optimization. Use Doctor if local routing hooks need cleanup.",
    );
    expect(switchboardAttentionCopy("headroom", "headroom")).toBe("");
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

  it("formats a copyable mode state", () => {
    const text = formatSwitchboardModeShareText({
      requestedMode: "rtk",
      summary: "RTK command compression is active.",
    });

    expect(text).toContain("Mac AI Switchboard mode state");
    expect(text).toContain("Requested mode: RTK only");
    expect(text).toContain("Active mode: RTK only");
    expect(text).toContain("Needs attention: no");
    expect(text).toContain("Client routing: off");
    expect(text).toContain("Shell output: on");
  });

  it("formats requested and active mode mismatch", () => {
    const text = formatSwitchboardModeShareText({
      requestedMode: "full",
      effectiveMode: "rtk",
      needsAttention: true,
      summary: "Full optimization requested, RTK only active.",
    });

    expect(text).toContain("Requested mode: Full optimization");
    expect(text).toContain("Active mode: RTK only");
    expect(text).toContain("Needs attention: yes");
    expect(text).toContain("Attention: Active now: RTK only");
  });
});
