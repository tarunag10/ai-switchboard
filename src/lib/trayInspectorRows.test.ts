import { describe, expect, it, vi } from "vitest";

import { buildSwitchboardInspectorRows } from "./trayInspectorRows";
import type { ClientConnectorStatus, RuntimeStatus } from "./types";

function connector(overrides: Record<string, unknown>): ClientConnectorStatus {
  return {
    clientId: "codex",
    name: "Codex",
    installed: true,
    enabled: false,
    verified: false,
    ...overrides,
  } as ClientConnectorStatus;
}

function build(overrides: Record<string, unknown> = {}) {
  const handleDoctorRepair = vi.fn();
  const openSettingsFocus = vi.fn();
  const setRepoMemoryMcpActive = vi.fn(async () => true);
  const prepareRepoMemoryMcp = vi.fn(async () => true);
  const rows = buildSwitchboardInspectorRows({
    runtimeStatus: null,
    switchboardState: null,
    switchboardConnectors: [],
    doctorRepairBusy: null,
    handleDoctorRepair,
    openSettingsFocus,
    repoMemoryLifecycle: { state: "inactive" } as never,
    addonBusyId: null,
    addonBusyLabel: null,
    setRepoMemoryMcpActive,
    prepareRepoMemoryMcp,
    ...overrides,
  });
  return {
    rows,
    handleDoctorRepair,
    openSettingsFocus,
    setRepoMemoryMcpActive,
    prepareRepoMemoryMcp,
  };
}

describe("buildSwitchboardInspectorRows", () => {
  it("describes unavailable runtime evidence conservatively", () => {
    const { rows } = build();
    expect(rows.find((row) => row.label === "Proxy listener")).toMatchObject({
      status: "Unreachable",
      detail: expect.stringContaining("127.0.0.1:6767"),
    });
    expect(rows.find((row) => row.label === "Backend port")).toMatchObject({
      status: "Unreachable",
      detail: "Internal backend port evidence is unavailable.",
    });
    expect(rows.find((row) => row.label === "Client routing")?.status).toBe(
      "Direct",
    );
  });

  it("reports reachable and fallback backend ports", () => {
    const runtimeStatus = {
      proxyReachable: true,
      proxyBindAddress: "127.0.0.1:7000",
      proxyAuthDetail: "Session token enforced.",
      backendStatus: {
        reachable: true,
        port: 7001,
        defaultPort: 7000,
        bindAddress: "127.0.0.1:7001",
      },
      rtk: { pathConfigured: true, hookConfigured: true, installed: true },
    } as RuntimeStatus;
    const { rows } = build({ runtimeStatus });
    expect(rows.find((row) => row.label === "Proxy listener")).toMatchObject({
      status: "Reachable",
      detail: expect.stringContaining("Session token enforced"),
    });
    expect(rows.find((row) => row.label === "Backend port")?.detail).toContain(
      "fallback internal backend port",
    );
    expect(rows.find((row) => row.label === "Shell export")?.status).toBe(
      "Configured",
    );
  });

  it("wires managed connector repair actions with exact IDs", () => {
    const codex = connector({
      supportsAutomaticSetup: true,
      setupMode: "managed",
    });
    const { rows, handleDoctorRepair } = build({
      switchboardConnectors: [codex],
      doctorRepairBusy: "repair_codex_setup",
    });
    const row = rows.find((item) => item.label === "Codex routing");
    expect(row).toMatchObject({
      status: "Repair ready",
      actionLabel: "Repair Codex",
      actionBusyLabel: "Repairing",
      actionDisabled: true,
    });
    row?.onAction?.();
    expect(handleDoctorRepair).toHaveBeenCalledWith("repair_codex_setup");
  });

  it("derives shell and provider proof from connector verification", () => {
    const codex = connector({
      enabled: true,
      verified: true,
      setupVerification: {
        checks: ["Managed shell block found", "Provider block found"],
        failures: [],
      },
    });
    const { rows } = build({ switchboardConnectors: [codex] });
    expect(rows.find((row) => row.label === "Managed shell blocks")?.status).toBe(
      "Verified",
    );
    expect(rows.find((row) => row.label === "Codex provider block")?.status).toBe(
      "Verified",
    );
  });

  it("wires settings and each Repo Memory MCP lifecycle action", async () => {
    const configured = build({
      runtimeStatus: {
        repoMemoryMcpConfigured: true,
        rtk: {},
      } as RuntimeStatus,
      repoMemoryLifecycle: { state: "inactive" } as never,
    });
    configured.rows.find((row) => row.label === "Proxy session auth")?.onAction?.();
    expect(configured.openSettingsFocus).toHaveBeenCalledWith(
      "proxy-session-auth",
    );
    configured.rows.find((row) => row.label === "Repo Memory MCP")?.onAction?.();
    expect(configured.setRepoMemoryMcpActive).toHaveBeenCalledWith(true);

    const active = build({ repoMemoryLifecycle: { state: "active" } as never });
    active.rows.find((row) => row.label === "Repo Memory MCP")?.onAction?.();
    expect(active.setRepoMemoryMcpActive).toHaveBeenCalledWith(false);

    const unconfigured = build();
    unconfigured.rows.find((row) => row.label === "Repo Memory MCP")?.onAction?.();
    expect(unconfigured.prepareRepoMemoryMcp).toHaveBeenCalledOnce();
  });

  it("flags legacy launch agents ahead of current launch state", () => {
    const runtimeStatus = {
      launchAgentStatus: {
        installed: true,
        loaded: true,
        legacyInstalled: true,
        legacyLoaded: true,
        legacyPath: "/tmp/Headroom.plist",
      },
      rtk: {},
    } as RuntimeStatus;
    const row = build({ runtimeStatus }).rows.find(
      (item) => item.label === "Launch at login",
    );
    expect(row).toMatchObject({
      status: "Legacy found",
      detail: expect.stringContaining("/tmp/Headroom.plist"),
    });
  });

  it("describes paused runtime, default backend, installed RTK, and MCP states", () => {
    const runtimeStatus = {
      paused: true,
      proxyBindAddress: "127.0.0.1:6767",
      backendStatus: {
        reachable: false,
        port: 6768,
        defaultPort: 6768,
        bindAddress: "127.0.0.1:6768",
      },
      rtk: { installed: true, pathConfigured: false, hookConfigured: false },
      mcpConfigured: false,
      mcpError: "Config missing",
    } as RuntimeStatus;
    const { rows } = build({ runtimeStatus });
    expect(rows.find((row) => row.label === "Proxy listener")?.status).toBe(
      "Paused",
    );
    expect(rows.find((row) => row.label === "Backend port")).toMatchObject({
      status: "Paused",
      detail: expect.stringContaining("default internal Headroom backend port"),
    });
    expect(rows.find((row) => row.label === "RTK shell hook")?.detail).toContain(
      "installed, but",
    );
    expect(rows.find((row) => row.label === "Headroom MCP")).toMatchObject({
      status: "Not configured",
      detail: "Config missing",
    });
  });

  it("covers verified, needs-test, direct, and missing connector rows", () => {
    const verified = connector({ enabled: true, verified: true });
    const needsTest = connector({
      clientId: "claude_code",
      name: "Claude Code",
      enabled: true,
      verified: false,
    });
    const { rows } = build({
      switchboardConnectors: [verified, needsTest],
    });
    expect(rows.find((row) => row.label === "Codex routing")?.status).toBe(
      "Verified",
    );
    expect(rows.find((row) => row.label === "Claude routing")?.status).toBe(
      "Needs test",
    );
    const directRows = build({
      switchboardConnectors: [
        connector({ enabled: false, supportStatus: "unsupported" }),
      ],
    }).rows;
    expect(directRows.find((row) => row.label === "Codex routing")?.status).toBe(
      "Direct",
    );
    const missingRows = build({ switchboardConnectors: [] }).rows;
    expect(missingRows.find((row) => row.label === "Claude routing")?.detail).toContain(
      "not detected",
    );
  });

  it("reports missing shell/provider proof and active launch-agent states", () => {
    const codex = connector({
      enabled: true,
      setupVerification: {
        checks: [],
        failures: ["shell profiles missing", "provider block missing"],
      },
    });
    const runtimeStatus = {
      rtk: {},
      mcpConfigured: true,
      launchAgentStatus: {
        installed: true,
        loaded: true,
        legacyInstalled: false,
        legacyLoaded: false,
        path: "/tmp/current.plist",
      },
    } as RuntimeStatus;
    const { rows } = build({ runtimeStatus, switchboardConnectors: [codex] });
    expect(rows.find((row) => row.label === "Managed shell blocks")?.status).toBe(
      "Missing",
    );
    expect(rows.find((row) => row.label === "Codex provider block")?.status).toBe(
      "Missing",
    );
    expect(rows.find((row) => row.label === "Headroom MCP")?.status).toBe(
      "Configured",
    );
    expect(rows.find((row) => row.label === "Launch at login")?.status).toBe(
      "Loaded",
    );
  });

  it("shows no-proof provider status when enabled without verification", () => {
    const codex = connector({ enabled: true, setupVerification: undefined });
    const { rows } = build({ switchboardConnectors: [codex] });
    expect(rows.find((row) => row.label === "Managed shell blocks")?.status).toBe(
      "No proof",
    );
    expect(rows.find((row) => row.label === "Codex provider block")?.status).toBe(
      "No proof",
    );
  });
});
