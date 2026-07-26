/**
 * Framework-agnostic orchestration for the product's local master action.
 *
 * This module deliberately knows nothing about Tauri, React, or persistence.
 * Callers inject the actions they are prepared to perform and can therefore
 * keep confirmation, permissions, and UI state at the boundary.
 */

export type MasterActivationRuntimeState =
  | "running"
  | "starting"
  | "paused"
  | "offline"
  | "unknown";

export type MasterActivationLocalFeatureId =
  | "runtime"
  | "local-optimizations"
  | "agent-memory"
  | "repo-intelligence"
  | "token-xray"
  | "daily-briefing"
  | "repo-memory-mcp";

export type MasterActivationGuidedFeatureId =
  | "connector-setup"
  | "gateway-deployment"
  | "blocked-optimization-engines"
  | "provider-authentication"
  | "installed-release-proof";

export type MasterActivationActionId =
  | MasterActivationLocalFeatureId
  | MasterActivationGuidedFeatureId;

export interface MasterActivationGuidedItem {
  id: MasterActivationGuidedFeatureId | (string & {});
  label: string;
  reason: string;
  /** Explicitly communicates that the master action will not execute it. */
  status: "guided" | "gated" | "manual";
}

export interface MasterActivationPlanAction {
  id: MasterActivationLocalFeatureId;
  label: string;
  kind: "local";
  reason: string;
  operation?: "start" | "refresh";
}

export interface MasterActivationPlan {
  version: 1;
  actions: readonly MasterActivationPlanAction[];
  supportedLocalOptimizations: readonly string[];
  guided: readonly MasterActivationGuidedItem[];
}

export interface MasterActivationCallbacks {
  startRuntime?: () => void | Promise<void>;
  refreshRuntime?: () => void | Promise<void>;
  enableLocalOptimization?: (id: string) => void | Promise<void>;
  refreshAgentMemory?: () => void | Promise<void>;
  refreshRepoIntelligence?: () => void | Promise<void>;
  refreshTokenXray?: () => void | Promise<void>;
  refreshDailyBriefing?: () => void | Promise<void>;
  prepareRepoMemoryMcp?: () => void | Promise<void>;
}

export interface MasterActivationPlanOptions {
  runtimeState?: MasterActivationRuntimeState;
  /** Only these local optimization IDs may be passed to the enable callback. */
  supportedLocalOptimizations?: readonly string[];
  callbacks?: MasterActivationCallbacks;
  guided?: readonly MasterActivationGuidedItem[];
}

export type MasterActivationActionStatus = "completed" | "failed" | "skipped";

export interface MasterActivationActionResult {
  id: MasterActivationLocalFeatureId;
  status: MasterActivationActionStatus;
  detail: string;
  error?: unknown;
}

export interface MasterActivationResult {
  version: 1;
  attempted: readonly MasterActivationActionResult[];
  completed: readonly MasterActivationActionResult[];
  failed: readonly MasterActivationActionResult[];
  skipped: readonly MasterActivationActionResult[];
  /** Guided/gated/manual work is never invoked by executeMasterActivation. */
  guided: readonly MasterActivationGuidedItem[];
  /** Receipt of the local work this invocation actually completed and owns. */
  receipt: MasterActivationReceipt;
}

export interface MasterActivationReceiptAction {
  id: MasterActivationLocalFeatureId;
  /** Optimization IDs are copied only from the activation plan allowlist. */
  optimizationIds: readonly string[];
}

export interface MasterActivationReceipt {
  version: 1;
  /** Only completed local actions are eligible for the inverse lifecycle. */
  ownedActions: readonly MasterActivationReceiptAction[];
  supportedLocalOptimizations: readonly string[];
}

export interface MasterDeactivationPlanAction {
  id: MasterActivationLocalFeatureId;
  label: string;
  kind: "local";
  reason: string;
  optimizationIds: readonly string[];
}

export interface MasterDeactivationPlan {
  version: 1;
  actions: readonly MasterDeactivationPlanAction[];
  guided: readonly MasterActivationGuidedItem[];
}

export interface MasterDeactivationCallbacks {
  stopRuntime?: () => void | Promise<void>;
  disableLocalOptimization?: (id: string) => void | Promise<void>;
  deactivateAgentMemory?: () => void | Promise<void>;
  deactivateRepoIntelligence?: () => void | Promise<void>;
  deactivateTokenXray?: () => void | Promise<void>;
  deactivateDailyBriefing?: () => void | Promise<void>;
  stopRepoMemoryMcp?: () => void | Promise<void>;
}

export interface MasterDeactivationPlanOptions {
  receipt: MasterActivationReceipt;
  callbacks?: MasterDeactivationCallbacks;
  guided?: readonly MasterActivationGuidedItem[];
}

export type MasterDeactivationActionStatus = "completed" | "failed" | "skipped" | "gated";

export interface MasterDeactivationActionResult {
  id: MasterActivationLocalFeatureId;
  status: MasterDeactivationActionStatus;
  detail: string;
  error?: unknown;
}

export interface MasterDeactivationGatedItem {
  id: MasterActivationGuidedItem["id"];
  status: "gated" | "guided" | "manual";
  detail: string;
}

export interface MasterDeactivationResult {
  version: 1;
  attempted: readonly MasterDeactivationActionResult[];
  completed: readonly MasterDeactivationActionResult[];
  failed: readonly MasterDeactivationActionResult[];
  skipped: readonly MasterDeactivationActionResult[];
  gated: readonly MasterDeactivationGatedItem[];
  /** The receipt remains available for UI/audit; it is not mutated or broadened. */
  receipt: MasterActivationReceipt;
  guided: readonly MasterActivationGuidedItem[];
}

export const defaultMasterActivationGuidedItems: readonly MasterActivationGuidedItem[] = [
  {
    id: "connector-setup",
    label: "Connector setup",
    reason: "Provider configuration and account choices require explicit user action.",
    status: "manual",
  },
  {
    id: "gateway-deployment",
    label: "Gateway deployment",
    reason: "Live gateways require user-owned infrastructure and credentials.",
    status: "gated",
  },
  {
    id: "blocked-optimization-engines",
    label: "Blocked optimization engines",
    reason: "Engines without verified provenance or live support remain gated.",
    status: "gated",
  },
  {
    id: "provider-authentication",
    label: "Provider authentication",
    reason: "The master action must not sign in, choose accounts, or write credentials.",
    status: "manual",
  },
  {
    id: "installed-release-proof",
    label: "Installed release proof",
    reason: "Signed, installed, and reboot-level evidence must be collected separately.",
    status: "guided",
  },
];

const localAction = (
  id: MasterActivationLocalFeatureId,
  label: string,
  reason: string,
): MasterActivationPlanAction => ({ id, label, kind: "local", reason });

export function createMasterActivationPlan(
  options: MasterActivationPlanOptions = {},
): MasterActivationPlan {
  const callbacks = options.callbacks ?? {};
  const actions: MasterActivationPlanAction[] = [];
  const runtimeState = options.runtimeState ?? "unknown";

  if (callbacks.startRuntime || callbacks.refreshRuntime) {
    if (runtimeState === "running" && callbacks.refreshRuntime) {
      actions.push({ ...localAction("runtime", "Refresh runtime", "Runtime is already running."), operation: "refresh" });
    } else if (callbacks.startRuntime) {
      actions.push({ ...localAction("runtime", "Start runtime", "Runtime is not confirmed running."), operation: "start" });
    } else if (runtimeState === "running") {
      actions.push({ ...localAction("runtime", "Refresh runtime", "Runtime is already running."), operation: "refresh" });
    }
  }

  if (options.supportedLocalOptimizations?.length && callbacks.enableLocalOptimization) {
    actions.push(localAction(
      "local-optimizations",
      "Enable supported local optimizations",
      "Only the caller-provided allowlist is eligible; no provider or gated engine is assumed live.",
    ));
  }

  const refreshActions: Array<[
    MasterActivationLocalFeatureId,
    string,
    string,
    keyof Pick<MasterActivationCallbacks, "refreshAgentMemory" | "refreshRepoIntelligence" | "refreshTokenXray" | "refreshDailyBriefing">,
  ]> = [
    ["agent-memory", "Refresh Agent Memory", "Prepare a local memory snapshot.", "refreshAgentMemory"],
    ["repo-intelligence", "Refresh Repo Intelligence", "Prepare local repository context and rankings.", "refreshRepoIntelligence"],
    ["token-xray", "Refresh Token X-Ray", "Refresh the local token/context read model.", "refreshTokenXray"],
    ["daily-briefing", "Refresh Daily Briefing", "Prepare the local usage briefing snapshot.", "refreshDailyBriefing"],
  ];
  for (const [id, label, reason, callback] of refreshActions) {
    if (callbacks[callback]) actions.push(localAction(id, label, reason));
  }

  if (callbacks.prepareRepoMemoryMcp) {
    actions.push(localAction("repo-memory-mcp", "Prepare Repo Memory MCP", "Prepare the app-managed read-only MCP lifecycle."));
  }

  return {
    version: 1,
    actions,
    supportedLocalOptimizations: options.supportedLocalOptimizations ?? [],
    guided: options.guided ?? defaultMasterActivationGuidedItems,
  };
}

export async function executeMasterActivation(
  plan: MasterActivationPlan,
  options: MasterActivationPlanOptions = {},
): Promise<MasterActivationResult> {
  const callbacks = options.callbacks ?? {};
  const results: MasterActivationActionResult[] = [];

  for (const action of plan.actions) {
    const callback = callbackForAction(action, callbacks);
    if (!callback) {
      results.push({ id: action.id, status: "skipped", detail: "No injected callback was supplied." });
      continue;
    }
    try {
      if (action.id === "local-optimizations") {
        for (const optimizationId of plan.supportedLocalOptimizations) {
          await callbacks.enableLocalOptimization?.(optimizationId);
        }
      } else {
        await callback();
      }
      results.push({ id: action.id, status: "completed", detail: "Injected local action completed." });
    } catch (error) {
      results.push({ id: action.id, status: "failed", detail: "Injected local action failed; no live claim was made.", error });
    }
  }

  return {
    version: 1,
    attempted: results,
    completed: results.filter((result) => result.status === "completed"),
    failed: results.filter((result) => result.status === "failed"),
    skipped: results.filter((result) => result.status === "skipped"),
    guided: plan.guided,
    receipt: createMasterActivationReceipt(plan, results),
  };
}

export function createMasterActivationReceipt(
  plan: MasterActivationPlan,
  results: readonly MasterActivationActionResult[],
): MasterActivationReceipt {
  return {
    version: 1,
    ownedActions: plan.actions
      .filter((action) => results.some((result) => result.id === action.id && result.status === "completed"))
      .map((action) => ({
        id: action.id,
        optimizationIds: action.id === "local-optimizations" ? [...plan.supportedLocalOptimizations] : [],
      })),
    supportedLocalOptimizations: [...plan.supportedLocalOptimizations],
  };
}

export function createMasterDeactivationPlan(
  options: MasterDeactivationPlanOptions,
): MasterDeactivationPlan {
  const receipt = options.receipt;
  const allowedOptimizations = new Set(receipt.supportedLocalOptimizations);
  const labels: Record<MasterActivationLocalFeatureId, [string, string]> = {
    runtime: ["Stop runtime", "Stop only the runtime started or refreshed by this receipt."],
    "local-optimizations": ["Disable supported local optimizations", "Disable only optimization IDs owned by this activation receipt."],
    "agent-memory": ["Deactivate Agent Memory", "Deactivate only the Agent Memory state owned by this receipt."],
    "repo-intelligence": ["Deactivate Repo Intelligence", "Deactivate only the Repo Intelligence state owned by this receipt."],
    "token-xray": ["Deactivate Token X-Ray", "Deactivate only the Token X-Ray state owned by this receipt."],
    "daily-briefing": ["Deactivate Daily Briefing", "Deactivate only the Daily Briefing state owned by this receipt."],
    "repo-memory-mcp": ["Stop Repo Memory MCP", "Stop only the app-managed MCP lifecycle owned by this receipt."],
  };

  return {
    version: 1,
    actions: receipt.ownedActions.map((owned) => ({
      id: owned.id,
      label: labels[owned.id][0],
      kind: "local",
      reason: labels[owned.id][1],
      optimizationIds: owned.id === "local-optimizations"
        ? owned.optimizationIds.filter((id) => allowedOptimizations.has(id))
        : [],
    })),
    guided: options.guided ?? defaultMasterActivationGuidedItems,
  };
}

export async function executeMasterDeactivation(
  plan: MasterDeactivationPlan,
  options: MasterDeactivationPlanOptions,
): Promise<MasterDeactivationResult> {
  const callbacks = options.callbacks ?? {};
  const results: MasterDeactivationActionResult[] = [];

  for (const action of plan.actions) {
    const callback = deactivationCallbackForAction(action, callbacks);
    if (!callback) {
      results.push({ id: action.id, status: "skipped", detail: "No injected deactivation callback was supplied." });
      continue;
    }
    try {
      if (action.id === "local-optimizations") {
        for (const optimizationId of action.optimizationIds) {
          await callbacks.disableLocalOptimization?.(optimizationId);
        }
      } else {
        await callback();
      }
      results.push({ id: action.id, status: "completed", detail: "Injected receipt-scoped deactivation completed." });
    } catch (error) {
      results.push({ id: action.id, status: "failed", detail: "Injected deactivation failed; no broader state was touched.", error });
    }
  }

  return {
    version: 1,
    attempted: results,
    completed: results.filter((result) => result.status === "completed"),
    failed: results.filter((result) => result.status === "failed"),
    skipped: results.filter((result) => result.status === "skipped"),
    gated: plan.guided.map((item) => ({ id: item.id, status: item.status, detail: item.reason })),
    receipt: options.receipt,
    guided: plan.guided,
  };
}

function deactivationCallbackForAction(
  action: MasterDeactivationPlanAction,
  callbacks: MasterDeactivationCallbacks,
): (() => void | Promise<void>) | undefined {
  switch (action.id) {
    case "runtime": return callbacks.stopRuntime;
    case "local-optimizations": return callbacks.disableLocalOptimization ? () => undefined : undefined;
    case "agent-memory": return callbacks.deactivateAgentMemory;
    case "repo-intelligence": return callbacks.deactivateRepoIntelligence;
    case "token-xray": return callbacks.deactivateTokenXray;
    case "daily-briefing": return callbacks.deactivateDailyBriefing;
    case "repo-memory-mcp": return callbacks.stopRepoMemoryMcp;
  }
}

function callbackForAction(
  action: MasterActivationPlanAction,
  callbacks: MasterActivationCallbacks,
): (() => void | Promise<void>) | undefined {
  switch (action.id) {
    case "runtime":
      return action.operation === "refresh"
        ? callbacks.refreshRuntime
        : callbacks.startRuntime;
    case "local-optimizations":
      return callbacks.enableLocalOptimization
        ? () => undefined
        : undefined;
    case "agent-memory":
      return callbacks.refreshAgentMemory;
    case "repo-intelligence":
      return callbacks.refreshRepoIntelligence;
    case "token-xray":
      return callbacks.refreshTokenXray;
    case "daily-briefing":
      return callbacks.refreshDailyBriefing;
    case "repo-memory-mcp":
      return callbacks.prepareRepoMemoryMcp;
  }
}
