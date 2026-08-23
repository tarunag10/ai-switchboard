import { invoke } from "@tauri-apps/api/core";
import { useRef, useState } from "react";

import type {
  MasterFeatureId,
  MasterFeatureState,
  MasterFeatureStatus,
} from "../components/MasterActivationCard";
import { getAgentMemorySnapshot } from "./agentMemory";
import { resolveMasterActivationLocalOptimizations } from "./leanctxPromotionGate";
import {
  createMasterActivationPlan,
  createMasterDeactivationPlan,
  executeMasterActivation,
  executeMasterDeactivation,
  type MasterActivationLocalFeatureId,
  type MasterActivationReceipt,
  type MasterDeactivationCallbacks,
} from "./masterActivation";
import {
  createMaxCompressionActivationPlan,
  createMaxCompressionLifecycleReceipts,
} from "./maxCompressionActivation";
import type { TrayView } from "./trayHelpers";
import type { RuntimeStatus, SwitchboardMode, SwitchboardState, ClientConnectorStatus } from "./types";
import {
  loadDailyUsageBriefing,
  loadTokenXraySnapshot,
} from "./usageAnalytics";
import { deriveSwitchboardMode } from "./switchboardDisplay";

export interface MasterActivationReceiptState {
  activation: MasterActivationReceipt;
  previousMode: SwitchboardMode;
  mcpWasActive: boolean;
}

export interface UseMasterActivationControllerOptions {
  switchboardState: SwitchboardState | null;
  connectors: ClientConnectorStatus[];
  runtimeStatus: RuntimeStatus | null;
  semanticCacheEnabled: boolean;
  setSemanticCacheEnabled: (enabled: boolean) => void;
  setActiveView: (view: TrayView) => void;
  openSettingsFocus: (targetId: string) => void;
  handleSetSwitchboardMode: (mode: SwitchboardMode) => Promise<void>;
  applyRuntimeStatusIfChanged: (status: RuntimeStatus) => void;
  refreshRuntimeStatus: () => Promise<void>;
  refreshConnectors: () => Promise<void>;
  refreshDoctorReport: () => Promise<void>;
  prepareRepoMemoryMcp: () => Promise<boolean>;
  setRepoMemoryMcpActive: (active: boolean) => Promise<boolean>;
}

export function useMasterActivationController({
  switchboardState,
  connectors,
  runtimeStatus,
  semanticCacheEnabled,
  setSemanticCacheEnabled,
  setActiveView,
  openSettingsFocus,
  handleSetSwitchboardMode,
  applyRuntimeStatusIfChanged,
  refreshRuntimeStatus,
  refreshConnectors,
  refreshDoctorReport,
  prepareRepoMemoryMcp,
  setRepoMemoryMcpActive,
}: UseMasterActivationControllerOptions) {
  const [masterActivationState, setMasterActivationState] =
    useState<MasterFeatureStatus>("ready");
  const [masterFeatureStates, setMasterFeatureStates] = useState<
    Partial<Record<MasterFeatureId, MasterFeatureState>>
  >({});
  const [masterActivationProgress, setMasterActivationProgress] = useState({
    completed: 0,
    total: 9,
  });
  const [masterActivationReceipt, setMasterActivationReceipt] =
    useState<MasterActivationReceiptState | null>(null);
  const [masterOperation, setMasterOperation] = useState<
    "activate" | "deactivate"
  >("activate");
  const [maxCompressionBusy, setMaxCompressionBusy] = useState(false);
  const masterOperationInFlightRef = useRef(false);
  const featureOperationsInFlightRef = useRef(new Set<MasterFeatureId>());

  function setMasterFeature(id: MasterFeatureId, state: MasterFeatureState) {
    setMasterFeatureStates((current) => ({ ...current, [id]: state }));
  }

  function removeMasterOwnedFeature(id: MasterFeatureId) {
    setMasterActivationReceipt((current) => {
      if (!current) return current;
      const ownedId =
        id === "gateway-mcp"
          ? "repo-memory-mcp"
          : id === "addons"
            ? "local-optimizations"
            : id;
      const ownedActions = current.activation.ownedActions.filter(
        (action) => action.id !== ownedId,
      );
      if (ownedActions.length === 0) return null;
      return {
        ...current,
        activation: { ...current.activation, ownedActions },
      };
    });
  }

  function masterFeatureView(id: MasterFeatureId): TrayView {
    switch (id) {
      case "agent-memory":
        return "agentMemory";
      case "token-xray":
        return "xray";
      case "daily-briefing":
        return "briefing";
      case "agent-session":
        return "optimization";
      case "repo-intelligence":
        return "repoIntelligence";
      case "addons":
      case "gateway-mcp":
        return "addons";
      case "doctor":
        return "doctor";
      case "rollback":
        return "settings";
    }
  }

  async function activateMasterFeature(id: MasterFeatureId) {
    if (
      masterOperationInFlightRef.current ||
      featureOperationsInFlightRef.current.has(id)
    ) {
      return;
    }
    featureOperationsInFlightRef.current.add(id);
    setMasterFeature(id, { status: "running", actionLabel: "Working…" });
    try {
      switch (id) {
        case "agent-memory":
          await getAgentMemorySnapshot();
          break;
        case "token-xray":
          await loadTokenXraySnapshot();
          break;
        case "daily-briefing":
          await loadDailyUsageBriefing();
          break;
        case "repo-intelligence":
          await invoke("get_latest_repo_intelligence_summary");
          break;
        case "gateway-mcp":
          if (!(await prepareRepoMemoryMcp())) {
            throw new Error("Repo Memory MCP could not be prepared.");
          }
          break;
        case "doctor":
          await refreshDoctorReport();
          break;
        case "addons":
          await Promise.all([refreshRuntimeStatus(), refreshConnectors()]);
          break;
        case "rollback":
          await refreshDoctorReport();
          openSettingsFocus("rollback-center");
          setMasterFeature(id, {
            status: "partial",
            actionLabel: "Open Settings",
            detail: "Rollback inventory is in Settings below.",
          });
          return;
        case "agent-session":
          setActiveView("optimization");
          setMasterFeature(id, {
            status: "partial",
            actionLabel: "Open",
            detail: "Prepare and copy the session payload before launch.",
          });
          return;
      }
      setMasterFeature(id, {
        status: "complete",
        actionLabel: "Run again",
        detail: "Local evidence refreshed.",
      });
    } catch (error) {
      setMasterFeature(id, {
        status: "error",
        actionLabel: "Retry",
        detail: error instanceof Error ? error.message : "Action failed.",
      });
    } finally {
      featureOperationsInFlightRef.current.delete(id);
    }
  }

  async function activateEverything() {
    if (masterOperationInFlightRef.current) return;
    masterOperationInFlightRef.current = true;
    const enabledSwitchboardConnectors = connectors.filter(
      (connector) => connector.enabled,
    );
    const previousMode =
      switchboardState?.mode ??
      deriveSwitchboardMode(runtimeStatus, enabledSwitchboardConnectors);
    const mcpWasActive = runtimeStatus?.repoMemoryMcpActive === true;
    setMasterOperation("activate");
    setMasterActivationState("running");
    setMasterFeatureStates(
      Object.fromEntries(
        [
          "agent-memory",
          "token-xray",
          "daily-briefing",
          "agent-session",
          "repo-intelligence",
          "addons",
          "gateway-mcp",
          "doctor",
          "rollback",
        ].map((id) => [
          id,
          {
            status: "running",
            actionLabel: "Working…",
            detail: "Waiting for activation evidence.",
          },
        ]),
      ) as Partial<Record<MasterFeatureId, MasterFeatureState>>,
    );
    setMasterActivationProgress({ completed: 0, total: 9 });
    try {
      await handleSetSwitchboardMode("full");
      const activatedRuntime = await invoke<RuntimeStatus>("get_runtime_status");
      applyRuntimeStatusIfChanged(activatedRuntime);
      if (!activatedRuntime.running || !activatedRuntime.proxyReachable) {
        throw new Error(
          "The full local mode did not bring the Headroom runtime online.",
        );
      }
      let managedAddonDetail = "Runtime and connector health refreshed.";
      let managedAddonStatus: MasterFeatureStatus = "complete";
      try {
        const managedActivation = await invoke<{
          receipt?: {
            overallStatus?: string;
            results?: Array<{ toolId: string; state: string; detail: string }>;
          };
        }>("activate_selected_tools", {
          selectedToolIds: ["headroom", "rtk", "ponytail", "caveman", "markitdown"],
        });
        const results = managedActivation?.receipt?.results ?? [];
        const failed = results.filter((item) => item.state === "failed");
        managedAddonDetail = failed.length > 0
          ? `Managed add-on activation was partial: ${failed.map((item) => `${item.toolId}: ${item.detail}`).join("; ")}`
          : managedActivation?.receipt?.overallStatus === "succeeded"
            ? "RTK, Ponytail, Caveman, and MarkItDown activation was applied through the native receipt path."
            : "Managed add-on activation returned no native receipt; health was refreshed without claiming completion.";
        if (failed.length > 0 || !managedActivation?.receipt) managedAddonStatus = "partial";
      } catch (error) {
        managedAddonStatus = "partial";
        managedAddonDetail = `Managed add-on activation needs attention: ${error instanceof Error ? error.message : "native activation failed."}`;
      }
      let leanctxSidecar: {
        configured: boolean;
        promotion?: {
          status: string;
          capabilityVersionOk: boolean;
          protectedContentOk: boolean;
          failOpenOk: boolean;
          shadowContractOk: boolean;
          livePromotionAllowed: boolean;
          reasons: string[];
        };
      } | null = null;
      try {
        leanctxSidecar = await invoke("get_leanctx_sidecar_status");
      } catch {
        leanctxSidecar = null;
      }
      const supportedLocalOptimizations =
        resolveMasterActivationLocalOptimizations(
          leanctxSidecar?.promotion ?? null,
        );
      const callbacks = {
        refreshAgentMemory: async () => {
          await getAgentMemorySnapshot();
          setMasterFeature("agent-memory", {
            status: "complete",
            detail: "Local memory metadata refreshed.",
          });
        },
        refreshRepoIntelligence: async () => {
          await invoke("get_latest_repo_intelligence_summary");
          setMasterFeature("repo-intelligence", {
            status: "complete",
            detail: "Latest local repository evidence checked.",
          });
        },
        refreshTokenXray: async () => {
          await loadTokenXraySnapshot();
          setMasterFeature("token-xray", {
            status: "complete",
            detail: "Local token evidence refreshed.",
          });
        },
        refreshDailyBriefing: async () => {
          await loadDailyUsageBriefing();
          setMasterFeature("daily-briefing", {
            status: "complete",
            detail: "Local briefing evidence refreshed.",
          });
        },
        enableLocalOptimization: async (optimizationId: string) => {
          if (optimizationId === "semantic-cache") {
            await invoke("set_semantic_cache_enabled", { enabled: true });
          } else if (optimizationId === "leanctx-shadow") {
            if (!leanctxSidecar?.configured) {
              await invoke("install_addon", { id: "leanctx" });
            }
            await invoke("set_addon_enabled", { id: "leanctx", enabled: true });
          }
          setMasterFeature("addons", {
            status: "partial",
            detail: `Enabled ${optimizationId} from the evidence-gated allowlist.`,
          });
        },
        ...(mcpWasActive
          ? {}
          : {
              prepareRepoMemoryMcp: async () => {
                if (!(await prepareRepoMemoryMcp())) {
                  throw new Error("Repo Memory MCP could not be prepared.");
                }
                setMasterFeature("gateway-mcp", {
                  status: "complete",
                  detail: "Read-only Repo Memory MCP prepared.",
                });
              },
            }),
      };
      const plan = createMasterActivationPlan({
        runtimeState:
          runtimeStatus?.running && runtimeStatus.proxyReachable
            ? "running"
            : "offline",
        supportedLocalOptimizations,
        callbacks,
      });
      const result = await executeMasterActivation(plan, { callbacks });
      await Promise.all([
        refreshRuntimeStatus(),
        refreshConnectors(),
        refreshDoctorReport(),
      ]);
      setMasterFeature("addons", {
        status: managedAddonStatus,
        detail: managedAddonDetail,
      });
      setMasterFeature("doctor", {
        status: "complete",
        detail: "Doctor report refreshed.",
      });
      setMasterFeature("rollback", {
        status: "complete",
        actionLabel: "Open Settings",
        detail: "Rollback inventory is in Settings.",
      });
      setMasterFeature("agent-session", {
        status: "partial",
        actionLabel: "Open",
        detail: "Prepare and copy a payload before launch.",
      });
      setMasterFeatureStates((current) =>
        Object.fromEntries(
          Object.entries(current).map(([id, state]) => [
            id,
            state?.status === "running"
              ? {
                  status: "complete",
                  actionLabel: "Run again",
                  detail: "Activation plan completed for this feature.",
                }
              : state,
          ]),
        ),
      );
      const completed = new Set(result.completed.map((item) => item.id));
      if (result.receipt.ownedActions.length > 0) {
        setMasterActivationReceipt({
          activation: result.receipt,
          previousMode,
          mcpWasActive,
        });
      }
      setMasterActivationProgress({
        completed: Math.min(9, completed.size + 3),
        total: 9,
      });
      setMasterActivationState(result.failed.length ? "partial" : "complete");
      for (const item of result.failed) {
        const featureId: MasterFeatureId =
          item.id === "repo-memory-mcp"
            ? "gateway-mcp"
            : item.id === "local-optimizations"
              ? "addons"
              : (item.id as MasterFeatureId);
        setMasterFeature(featureId, {
          status: "error",
          actionLabel: "Retry",
          detail: item.detail,
        });
      }
    } catch (error) {
      setMasterActivationState("error");
      setMasterFeature("doctor", {
        status: "error",
        actionLabel: "Retry",
        detail:
          error instanceof Error ? error.message : "Master activation failed.",
      });
    } finally {
      masterOperationInFlightRef.current = false;
    }
  }

  function masterFeatureToOwnedActionId(
    id: MasterFeatureId,
  ): MasterActivationLocalFeatureId | null {
    switch (id) {
      case "agent-memory":
        return "agent-memory";
      case "token-xray":
        return "token-xray";
      case "daily-briefing":
        return "daily-briefing";
      case "repo-intelligence":
        return "repo-intelligence";
      case "addons":
        return "local-optimizations";
      case "gateway-mcp":
        return "repo-memory-mcp";
      case "agent-session":
      case "doctor":
      case "rollback":
        return null;
    }
  }

  async function activateMaxCompression() {
    if (masterOperationInFlightRef.current || maxCompressionBusy) return;
    masterOperationInFlightRef.current = true;
    setMaxCompressionBusy(true);
    try {
      const plan = createMaxCompressionActivationPlan({
        mode: "full",
        proxyReachable: runtimeStatus?.proxyReachable ?? false,
        semanticCacheEnabled,
      });
      void createMaxCompressionLifecycleReceipts(plan);
      await handleSetSwitchboardMode("full");
      const activatedRuntime = await invoke<RuntimeStatus>("get_runtime_status");
      applyRuntimeStatusIfChanged(activatedRuntime);
      if (!activatedRuntime.running || !activatedRuntime.proxyReachable) {
        throw new Error(
          "Max compression requires a reachable Headroom runtime in Full mode.",
        );
      }
      if (plan.engines.includes("semantic-cache")) {
        await invoke("set_semantic_cache_enabled", { enabled: true });
        setSemanticCacheEnabled(true);
      }
      if (plan.engines.includes("rtk")) {
        await invoke("set_rtk_enabled", { enabled: true });
      }
      setActiveView("repoIntelligence");
      await Promise.all([
        refreshRuntimeStatus(),
        refreshConnectors(),
        refreshDoctorReport(),
      ]);
      setMasterFeature("doctor", {
        status: "complete",
        detail: "Doctor refreshed after max compression activation.",
      });
      setMasterFeature("repo-intelligence", {
        status: "partial",
        actionLabel: "Open",
        detail:
          "Index the active repository before starting an agent session.",
      });
    } catch (error) {
      setMasterFeature("doctor", {
        status: "error",
        actionLabel: "Retry",
        detail:
          error instanceof Error
            ? error.message
            : "Max compression activation failed.",
      });
    } finally {
      setMaxCompressionBusy(false);
      masterOperationInFlightRef.current = false;
    }
  }

  function openCompressionPlaybook() {
    setActiveView("home");
    window.setTimeout(() => {
      document
        .getElementById("doctor-compression-playbook")
        ?.scrollIntoView({ behavior: "smooth", block: "start" });
    }, 0);
  }

  function createMasterDeactivationCallbacks(
    receipt: MasterActivationReceiptState,
  ): MasterDeactivationCallbacks {
    return {
      deactivateAgentMemory: async () => undefined,
      deactivateRepoIntelligence: async () => undefined,
      deactivateTokenXray: async () => undefined,
      deactivateDailyBriefing: async () => undefined,
      disableLocalOptimization: async (optimizationId: string) => {
        if (optimizationId === "semantic-cache") {
          await invoke("set_semantic_cache_enabled", { enabled: false });
        } else if (optimizationId === "leanctx-shadow") {
          await invoke("set_addon_enabled", { id: "leanctx", enabled: false });
        }
      },
      ...(receipt.mcpWasActive
        ? {}
        : {
            stopRepoMemoryMcp: async () => {
              if (!(await setRepoMemoryMcpActive(false))) {
                throw new Error("Repo Memory MCP could not be stopped.");
              }
            },
          }),
    };
  }

  async function deactivateMasterFeature(id: MasterFeatureId) {
    if (
      masterOperationInFlightRef.current ||
      featureOperationsInFlightRef.current.has(id)
    ) {
      return;
    }
    const receipt = masterActivationReceipt;
    if (!receipt) return;
    featureOperationsInFlightRef.current.add(id);
    setMasterFeature(id, { status: "running", actionLabel: "Working…" });
    const ownedActionId = masterFeatureToOwnedActionId(id);

    if (!ownedActionId) {
      setMasterFeature(id, {
        status: "ready",
        actionLabel: "Activate",
        detail: "No master-owned backend state remains active for this feature.",
      });
      featureOperationsInFlightRef.current.delete(id);
      return;
    }

    const owned = receipt.activation.ownedActions.find(
      (action) => action.id === ownedActionId,
    );
    if (!owned) {
      setMasterFeature(id, {
        status: "ready",
        actionLabel: "Activate",
        detail:
          "This feature was refreshed during activation but left no reversible master-owned state.",
      });
      featureOperationsInFlightRef.current.delete(id);
      return;
    }

    try {
      const partialReceipt: MasterActivationReceipt = {
        ...receipt.activation,
        ownedActions: [owned],
      };
      const plan = createMasterDeactivationPlan({ receipt: partialReceipt });
      const result = await executeMasterDeactivation(plan, {
        receipt: partialReceipt,
        callbacks: createMasterDeactivationCallbacks(receipt),
      });
      if (result.failed.length > 0) {
        throw new Error(result.failed[0]?.detail ?? "Deactivation failed.");
      }
      await refreshRuntimeStatus();
      setMasterFeature(id, {
        status: "ready",
        actionLabel: "Activate",
        detail: "Master-owned state for this feature was reversed.",
      });
      removeMasterOwnedFeature(id);
    } catch (error) {
      setMasterFeature(id, {
        status: "error",
        actionLabel: "Retry deactivation",
        detail: error instanceof Error ? error.message : "Deactivation failed.",
      });
    } finally {
      featureOperationsInFlightRef.current.delete(id);
    }
  }

  async function deactivateEverything() {
    const receipt = masterActivationReceipt;
    if (!receipt || masterOperationInFlightRef.current) return;
    masterOperationInFlightRef.current = true;
    setMasterOperation("deactivate");
    setMasterActivationState("running");
    try {
      const callbacks = createMasterDeactivationCallbacks(receipt);
      const plan = createMasterDeactivationPlan({
        receipt: receipt.activation,
        callbacks,
      });
      const result = await executeMasterDeactivation(plan, {
        receipt: receipt.activation,
        callbacks,
      });
      if (receipt.previousMode !== "full") {
        await handleSetSwitchboardMode(receipt.previousMode);
      }
      await Promise.all([
        refreshRuntimeStatus(),
        refreshConnectors(),
        refreshDoctorReport(),
      ]);
      const failed = result.failed.length > 0;
      if (!failed) {
        setMasterActivationReceipt(null);
        setMasterActivationProgress({ completed: 0, total: 9 });
        setMasterFeatureStates({});
        setMasterActivationState("ready");
      } else {
        setMasterActivationState("partial");
        for (const item of result.failed) {
          setMasterFeature(
            item.id === "repo-memory-mcp" ? "gateway-mcp" : "addons",
            {
              status: "error",
              actionLabel: "Retry deactivation",
              detail: item.detail,
            },
          );
        }
      }
    } catch (error) {
      setMasterActivationState("partial");
      setMasterFeature("doctor", {
        status: "error",
        actionLabel: "Retry deactivation",
        detail:
          error instanceof Error ? error.message : "Master deactivation failed.",
      });
    } finally {
      masterOperationInFlightRef.current = false;
    }
  }

  return {
    masterActivationState,
    masterFeatureStates,
    masterActivationProgress,
    masterActivationReceipt,
    masterOperation,
    maxCompressionBusy,
    masterActivationIsActive: masterActivationReceipt !== null,
    activateEverything,
    deactivateEverything,
    activateMasterFeature,
    deactivateMasterFeature,
    activateMaxCompression,
    openCompressionPlaybook,
    masterFeatureView,
  };
}
