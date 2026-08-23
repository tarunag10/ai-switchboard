import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useMasterActivationController } from "./useMasterActivationController";
import type { RuntimeStatus } from "./types";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  memory: vi.fn(),
  xray: vi.fn(),
  briefing: vi.fn(),
  executeActivation: vi.fn(),
  executeDeactivation: vi.fn(),
  activationPlan: vi.fn((input: unknown) => input),
  deactivationPlan: vi.fn((input: unknown) => input),
  maxPlan: vi.fn(),
  localOptimizations: ["semantic-cache"] as string[],
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mocks.invoke(...args),
}));
vi.mock("./agentMemory", () => ({
  getAgentMemorySnapshot: (...args: unknown[]) => mocks.memory(...args),
}));
vi.mock("./usageAnalytics", () => ({
  loadTokenXraySnapshot: (...args: unknown[]) => mocks.xray(...args),
  loadDailyUsageBriefing: (...args: unknown[]) => mocks.briefing(...args),
}));
vi.mock("./leanctxPromotionGate", () => ({
  resolveMasterActivationLocalOptimizations: () => mocks.localOptimizations,
}));
vi.mock("./switchboardDisplay", () => ({
  deriveSwitchboardMode: () => "off",
}));
vi.mock("./masterActivation", () => ({
  createMasterActivationPlan: (input: unknown) => mocks.activationPlan(input),
  createMasterDeactivationPlan: (input: unknown) =>
    mocks.deactivationPlan(input),
  executeMasterActivation: (plan: unknown, context: unknown) =>
    mocks.executeActivation(plan, context),
  executeMasterDeactivation: (plan: unknown, context: unknown) =>
    mocks.executeDeactivation(plan, context),
}));
vi.mock("./maxCompressionActivation", () => ({
  createMaxCompressionActivationPlan: (input: unknown) => mocks.maxPlan(input),
  createMaxCompressionLifecycleReceipts: vi.fn(() => []),
}));

function setup(runtimeStatus: RuntimeStatus | null = null) {
  const callbacks = {
    setSemanticCacheEnabled: vi.fn(),
    setActiveView: vi.fn(),
    openSettingsFocus: vi.fn(),
    handleSetSwitchboardMode: vi.fn(async () => undefined),
    applyRuntimeStatusIfChanged: vi.fn(),
    refreshRuntimeStatus: vi.fn(async () => undefined),
    refreshConnectors: vi.fn(async () => undefined),
    refreshDoctorReport: vi.fn(async () => undefined),
    prepareRepoMemoryMcp: vi.fn(async () => true),
    setRepoMemoryMcpActive: vi.fn(async () => true),
  };
  const hook = renderHook(() =>
    useMasterActivationController({
      switchboardState: null,
      connectors: [],
      runtimeStatus,
      semanticCacheEnabled: false,
      ...callbacks,
    }),
  );
  return { ...hook, ...callbacks };
}

const healthyRuntime = {
  running: true,
  proxyReachable: true,
  repoMemoryMcpActive: false,
} as RuntimeStatus;

describe("useMasterActivationController", () => {
  beforeEach(() => {
    for (const mock of Object.values(mocks)) {
      if (vi.isMockFunction(mock)) mock.mockReset();
    }
    mocks.activationPlan.mockImplementation((input) => input);
    mocks.deactivationPlan.mockImplementation((input) => input);
    mocks.memory.mockResolvedValue({});
    mocks.xray.mockResolvedValue({});
    mocks.briefing.mockResolvedValue({});
    mocks.maxPlan.mockReturnValue({
      engines: ["semantic-cache", "rtk"],
    });
    mocks.localOptimizations = ["semantic-cache"];
  });

  it("maps every master feature to its corresponding view", () => {
    const { result } = setup();
    expect({
      memory: result.current.masterFeatureView("agent-memory"),
      xray: result.current.masterFeatureView("token-xray"),
      briefing: result.current.masterFeatureView("daily-briefing"),
      session: result.current.masterFeatureView("agent-session"),
      repo: result.current.masterFeatureView("repo-intelligence"),
      addons: result.current.masterFeatureView("addons"),
      mcp: result.current.masterFeatureView("gateway-mcp"),
      doctor: result.current.masterFeatureView("doctor"),
      rollback: result.current.masterFeatureView("rollback"),
    }).toEqual({
      memory: "agentMemory",
      xray: "xray",
      briefing: "briefing",
      session: "optimization",
      repo: "repoIntelligence",
      addons: "addons",
      mcp: "addons",
      doctor: "doctor",
      rollback: "settings",
    });
  });

  it("refreshes individual feature evidence and exposes action failures", async () => {
    const { result } = setup();
    await act(() => result.current.activateMasterFeature("agent-memory"));
    expect(mocks.memory).toHaveBeenCalledOnce();
    expect(result.current.masterFeatureStates["agent-memory"]).toMatchObject({
      status: "complete",
      actionLabel: "Run again",
    });

    mocks.invoke.mockRejectedValueOnce(new Error("repo unavailable"));
    await act(() => result.current.activateMasterFeature("repo-intelligence"));
    expect(mocks.invoke).toHaveBeenCalledWith(
      "get_latest_repo_intelligence_summary",
    );
    expect(result.current.masterFeatureStates["repo-intelligence"]).toMatchObject({
      status: "error",
      detail: "repo unavailable",
    });
  });

  it("refreshes all remaining individual feature paths", async () => {
    const setupResult = setup();
    for (const id of [
      "token-xray",
      "daily-briefing",
      "gateway-mcp",
      "doctor",
      "addons",
    ] as const) {
      await act(() => setupResult.result.current.activateMasterFeature(id));
      expect(setupResult.result.current.masterFeatureStates[id]?.status).toBe(
        "complete",
      );
    }
    expect(mocks.xray).toHaveBeenCalledOnce();
    expect(mocks.briefing).toHaveBeenCalledOnce();
    expect(setupResult.prepareRepoMemoryMcp).toHaveBeenCalledOnce();
    expect(setupResult.refreshDoctorReport).toHaveBeenCalledOnce();
    expect(setupResult.refreshRuntimeStatus).toHaveBeenCalledOnce();
    expect(setupResult.refreshConnectors).toHaveBeenCalledOnce();
  });

  it("reports a failed Repo Memory MCP preparation", async () => {
    const setupResult = setup();
    setupResult.prepareRepoMemoryMcp.mockResolvedValueOnce(false);
    await act(() =>
      setupResult.result.current.activateMasterFeature("gateway-mcp"),
    );
    expect(
      setupResult.result.current.masterFeatureStates["gateway-mcp"],
    ).toMatchObject({
      status: "error",
      detail: "Repo Memory MCP could not be prepared.",
    });
  });

  it("opens session and rollback surfaces without claiming completion", async () => {
    const { result, setActiveView, openSettingsFocus, refreshDoctorReport } = setup();
    await act(() => result.current.activateMasterFeature("agent-session"));
    expect(setActiveView).toHaveBeenCalledWith("optimization");
    expect(result.current.masterFeatureStates["agent-session"]?.status).toBe(
      "partial",
    );

    await act(() => result.current.activateMasterFeature("rollback"));
    expect(refreshDoctorReport).toHaveBeenCalledOnce();
    expect(openSettingsFocus).toHaveBeenCalledWith("rollback-center");
    expect(result.current.masterFeatureStates.rollback?.actionLabel).toBe(
      "Open Settings",
    );
  });

  it("activates everything and wires exact local optimization commands", async () => {
    mocks.localOptimizations = ["semantic-cache", "leanctx-shadow"];
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      if (command === "activate_selected_tools") {
        return { receipt: { runId: "native-activation-1", overallStatus: "succeeded", results: [] } };
      }
      if (command === "get_leanctx_sidecar_status") {
        return { configured: false, promotion: {} };
      }
      return undefined;
    });
    mocks.executeActivation.mockImplementation(
      async (
        _plan: unknown,
        context: {
          callbacks: Record<string, (...args: unknown[]) => Promise<void>>;
        },
      ) => {
        await context.callbacks.refreshAgentMemory();
        await context.callbacks.refreshRepoIntelligence();
        await context.callbacks.refreshTokenXray();
        await context.callbacks.refreshDailyBriefing();
        await context.callbacks.enableLocalOptimization("semantic-cache");
        await context.callbacks.enableLocalOptimization("leanctx-shadow");
        await context.callbacks.prepareRepoMemoryMcp();
        return {
          completed: [
            { id: "agent-memory" },
            { id: "local-optimizations" },
            { id: "repo-memory-mcp" },
          ],
          failed: [],
          receipt: {
            ownedActions: [
              { id: "local-optimizations", optimizationIds: ["semantic-cache"] },
            ],
          },
        };
      },
    );
    const setupResult = setup();
    await act(() => setupResult.result.current.activateEverything());

    expect(setupResult.handleSetSwitchboardMode).toHaveBeenCalledWith("full");
    expect(mocks.invoke).toHaveBeenCalledWith("get_runtime_status");
    expect(mocks.invoke).toHaveBeenCalledWith("activate_selected_tools", {
      selectedToolIds: ["headroom", "rtk", "ponytail", "caveman", "markitdown"],
    });
    expect(mocks.invoke).toHaveBeenCalledWith("set_semantic_cache_enabled", {
      enabled: true,
    });
    expect(mocks.invoke).toHaveBeenCalledWith("install_addon", { id: "leanctx" });
    expect(mocks.invoke).toHaveBeenCalledWith("set_addon_enabled", {
      id: "leanctx",
      enabled: true,
    });
    expect(mocks.invoke).toHaveBeenCalledWith(
      "get_latest_repo_intelligence_summary",
    );
    expect(mocks.xray).toHaveBeenCalledOnce();
    expect(mocks.briefing).toHaveBeenCalledOnce();
    expect(setupResult.prepareRepoMemoryMcp).toHaveBeenCalledOnce();
    expect(setupResult.result.current.masterActivationState).toBe("complete");
    expect(setupResult.result.current.masterActivationReceipt?.previousMode).toBe(
      "off",
    );
  });

  it("publishes immediate progress and suppresses duplicate activation clicks", async () => {
    let releaseMode!: () => void;
    const modePending = new Promise<undefined>((resolve) => {
      releaseMode = () => resolve(undefined);
    });
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      if (command === "activate_selected_tools") {
        return { receipt: { runId: "native-activation-2", overallStatus: "succeeded", results: [] } };
      }
      if (command === "get_leanctx_sidecar_status") return null;
      return undefined;
    });
    mocks.executeActivation.mockResolvedValue({
      completed: [],
      failed: [],
      receipt: { ownedActions: [] },
    });
    const setupResult = setup();
    setupResult.handleSetSwitchboardMode.mockImplementationOnce(
      async () => modePending,
    );

    let first!: Promise<void>;
    let duplicate!: Promise<void>;
    act(() => {
      first = setupResult.result.current.activateEverything();
      duplicate = setupResult.result.current.activateEverything();
    });
    expect(setupResult.result.current.masterActivationState).toBe("running");
    expect(setupResult.result.current.masterFeatureStates["agent-memory"]).toMatchObject({
      status: "running",
      actionLabel: "Working…",
      detail: "Waiting for activation evidence.",
    });
    expect(setupResult.handleSetSwitchboardMode).toHaveBeenCalledTimes(1);

    releaseMode();
    await act(async () => {
      await Promise.all([first, duplicate]);
    });
    expect(mocks.executeActivation).toHaveBeenCalledTimes(1);
    expect(setupResult.result.current.masterActivationState).toBe("complete");
  });

  it("maps failed activation callbacks to retryable visible feature errors", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      return null;
    });
    mocks.executeActivation.mockResolvedValue({
      completed: [],
      failed: [
        { id: "repo-memory-mcp", detail: "MCP preparation failed" },
        { id: "local-optimizations", detail: "Cache activation failed" },
      ],
      receipt: { ownedActions: [] },
    });
    const setupResult = setup();
    await act(() => setupResult.result.current.activateEverything());
    expect(setupResult.result.current.masterActivationState).toBe("partial");
    expect(
      setupResult.result.current.masterFeatureStates["gateway-mcp"],
    ).toMatchObject({
      status: "error",
      actionLabel: "Retry",
      detail: "MCP preparation failed",
    });
    expect(setupResult.result.current.masterFeatureStates.addons).toMatchObject({
      status: "error",
      actionLabel: "Retry",
      detail: "Cache activation failed",
    });
  });

  it("fails closed when Full mode does not produce a reachable runtime", async () => {
    mocks.invoke.mockResolvedValueOnce({ running: true, proxyReachable: false });
    const { result } = setup();
    await act(() => result.current.activateEverything());
    expect(mocks.executeActivation).not.toHaveBeenCalled();
    expect(result.current.masterActivationState).toBe("error");
    expect(result.current.masterFeatureStates.doctor?.detail).toContain(
      "did not bring the Headroom runtime online",
    );
  });

  it("activates max compression with exact engine payloads and refreshes evidence", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      return undefined;
    });
    const setupResult = setup(healthyRuntime);
    await act(() => setupResult.result.current.activateMaxCompression());

    expect(mocks.maxPlan).toHaveBeenCalledWith({
      mode: "full",
      proxyReachable: true,
      semanticCacheEnabled: false,
    });
    expect(mocks.invoke).toHaveBeenCalledWith("set_semantic_cache_enabled", {
      enabled: true,
    });
    expect(mocks.invoke).toHaveBeenCalledWith("set_rtk_enabled", {
      enabled: true,
    });
    expect(setupResult.setSemanticCacheEnabled).toHaveBeenCalledWith(true);
    expect(setupResult.setActiveView).toHaveBeenCalledWith("repoIntelligence");
    expect(setupResult.result.current.maxCompressionBusy).toBe(false);
  });

  it("reports max compression runtime failures and opens its playbook", async () => {
    mocks.invoke.mockResolvedValueOnce({ running: false, proxyReachable: false });
    const setupResult = setup();
    await act(() => setupResult.result.current.activateMaxCompression());
    expect(setupResult.result.current.masterFeatureStates.doctor).toMatchObject({
      status: "error",
      detail: expect.stringContaining("requires a reachable Headroom runtime"),
    });

    vi.useFakeTimers();
    const scrollIntoView = vi.fn();
    const element = document.createElement("div");
    element.id = "doctor-compression-playbook";
    element.scrollIntoView = scrollIntoView;
    document.body.append(element);
    act(() => setupResult.result.current.openCompressionPlaybook());
    expect(setupResult.setActiveView).toHaveBeenCalledWith("home");
    vi.runAllTimers();
    expect(scrollIntoView).toHaveBeenCalledWith({
      behavior: "smooth",
      block: "start",
    });
    element.remove();
    vi.useRealTimers();
  });

  it("deactivates all master-owned state with exact inverse commands", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      if (command === "get_leanctx_sidecar_status") return null;
      return undefined;
    });
    mocks.executeActivation.mockResolvedValue({
      completed: [],
      failed: [],
      receipt: {
        ownedActions: [
          {
            id: "local-optimizations",
            optimizationIds: ["semantic-cache", "leanctx-shadow"],
          },
          { id: "repo-memory-mcp" },
        ],
      },
    });
    mocks.executeDeactivation.mockImplementation(
      async (
        _plan: unknown,
        context: {
          callbacks: Record<string, (...args: unknown[]) => Promise<void>>;
        },
      ) => {
        await context.callbacks.deactivateAgentMemory();
        await context.callbacks.deactivateRepoIntelligence();
        await context.callbacks.deactivateTokenXray();
        await context.callbacks.deactivateDailyBriefing();
        await context.callbacks.disableLocalOptimization("semantic-cache");
        await context.callbacks.disableLocalOptimization("leanctx-shadow");
        await context.callbacks.stopRepoMemoryMcp();
        return { failed: [] };
      },
    );
    const setupResult = setup();
    await act(() => setupResult.result.current.activateEverything());
    await act(() => setupResult.result.current.deactivateEverything());

    expect(mocks.invoke).toHaveBeenCalledWith("set_semantic_cache_enabled", {
      enabled: false,
    });
    expect(mocks.invoke).toHaveBeenCalledWith("set_addon_enabled", {
      id: "leanctx",
      enabled: false,
    });
    expect(setupResult.setRepoMemoryMcpActive).toHaveBeenCalledWith(false);
    expect(setupResult.handleSetSwitchboardMode).toHaveBeenLastCalledWith("off");
    expect(setupResult.result.current.masterActivationState).toBe("ready");
    await waitFor(() =>
      expect(setupResult.result.current.masterActivationReceipt).toBeNull(),
    );
  });

  it("retains and rolls back the master-owned native add-on receipt", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      if (command === "get_leanctx_sidecar_status") return null;
      if (command === "activate_selected_tools") {
        return { receipt: { runId: "native-run-42", overallStatus: "succeeded", results: [] } };
      }
      return undefined;
    });
    mocks.executeActivation.mockResolvedValue({
      completed: [],
      failed: [],
      receipt: { ownedActions: [] },
    });
    mocks.executeDeactivation.mockResolvedValue({ failed: [] });

    const setupResult = setup();
    await act(() => setupResult.result.current.activateEverything());
    expect(setupResult.result.current.masterActivationReceipt).toMatchObject({
      nativeActivationRunId: "native-run-42",
    });

    await act(() => setupResult.result.current.deactivateEverything());
    expect(mocks.invoke).toHaveBeenCalledWith("rollback_selective_activation", {
      runId: "native-run-42",
    });
    expect(setupResult.result.current.masterActivationReceipt).toBeNull();
  });

  it("uses the native receipt from the Addons deactivation action", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      if (command === "get_leanctx_sidecar_status") return null;
      if (command === "activate_selected_tools") {
        return { receipt: { runId: "native-addon-action", overallStatus: "succeeded", results: [] } };
      }
      return undefined;
    });
    mocks.executeActivation.mockResolvedValue({
      completed: [],
      failed: [],
      receipt: { ownedActions: [] },
    });

    const setupResult = setup();
    await act(() => setupResult.result.current.activateEverything());
    await act(() => setupResult.result.current.deactivateMasterFeature("addons"));

    expect(mocks.invoke).toHaveBeenCalledWith("rollback_selective_activation", {
      runId: "native-addon-action",
    });
    expect(setupResult.result.current.masterFeatureStates.addons).toMatchObject({
      status: "ready",
      actionLabel: "Activate",
    });
    expect(setupResult.result.current.masterActivationReceipt).toBeNull();
  });

  it("marks failed deactivation items and preserves partial state", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      return null;
    });
    mocks.executeActivation.mockResolvedValue({
      completed: [],
      failed: [],
      receipt: { ownedActions: [{ id: "repo-memory-mcp" }] },
    });
    mocks.executeDeactivation.mockResolvedValue({
      failed: [
        { id: "repo-memory-mcp", detail: "MCP still running" },
        { id: "local-optimizations", detail: "Cache still enabled" },
      ],
    });
    const setupResult = setup();
    await act(() => setupResult.result.current.activateEverything());
    await act(() => setupResult.result.current.deactivateEverything());
    expect(setupResult.result.current.masterActivationState).toBe("partial");
    expect(
      setupResult.result.current.masterFeatureStates["gateway-mcp"]?.detail,
    ).toBe("MCP still running");
    expect(setupResult.result.current.masterFeatureStates.addons?.detail).toBe(
      "Cache still enabled",
    );
  });

  it("handles per-feature deactivation guards, success, and failure", async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") return healthyRuntime;
      return null;
    });
    mocks.executeActivation.mockResolvedValue({
      completed: [],
      failed: [],
      receipt: {
        ownedActions: [
          { id: "local-optimizations", optimizationIds: ["semantic-cache"] },
        ],
      },
    });
    mocks.executeDeactivation.mockResolvedValueOnce({ failed: [] });
    const setupResult = setup();
    await act(() => setupResult.result.current.activateEverything());

    await act(() => setupResult.result.current.deactivateMasterFeature("doctor"));
    expect(setupResult.result.current.masterFeatureStates.doctor?.detail).toContain(
      "No master-owned backend state",
    );
    await act(() =>
      setupResult.result.current.deactivateMasterFeature("token-xray"),
    );
    expect(
      setupResult.result.current.masterFeatureStates["token-xray"]?.detail,
    ).toContain("left no reversible");
    await act(() => setupResult.result.current.deactivateMasterFeature("addons"));
    expect(setupResult.result.current.masterFeatureStates.addons?.status).toBe(
      "ready",
    );
    await waitFor(() =>
      expect(setupResult.result.current.masterActivationReceipt).toBeNull(),
    );

    mocks.executeActivation.mockResolvedValueOnce({
      completed: [],
      failed: [],
      receipt: { ownedActions: [{ id: "agent-memory" }] },
    });
    mocks.executeDeactivation.mockResolvedValueOnce({
      failed: [{ id: "agent-memory", detail: "undo failed" }],
    });
    await act(() => setupResult.result.current.activateEverything());
    await act(() =>
      setupResult.result.current.deactivateMasterFeature("agent-memory"),
    );
    expect(
      setupResult.result.current.masterFeatureStates["agent-memory"],
    ).toMatchObject({ status: "error", detail: "undo failed" });
  });
});
