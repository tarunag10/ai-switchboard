import { describe, expect, it, vi } from "vitest";

import {
  createMasterDeactivationPlan,
  createMasterActivationPlan,
  defaultMasterActivationGuidedItems,
  executeMasterDeactivation,
  executeMasterActivation,
} from "./masterActivation";

describe("master activation", () => {
  it("plans only injected local actions and chooses runtime refresh when healthy", () => {
    const plan = createMasterActivationPlan({
      runtimeState: "running",
      supportedLocalOptimizations: ["semantic-cache"],
      callbacks: {
        refreshRuntime: vi.fn(),
        enableLocalOptimization: vi.fn(),
        refreshAgentMemory: vi.fn(),
        refreshRepoIntelligence: vi.fn(),
        refreshTokenXray: vi.fn(),
        refreshDailyBriefing: vi.fn(),
        prepareRepoMemoryMcp: vi.fn(),
      },
    });

    expect(plan.actions.map(({ id }) => id)).toEqual([
      "runtime",
      "local-optimizations",
      "agent-memory",
      "repo-intelligence",
      "token-xray",
      "daily-briefing",
      "repo-memory-mcp",
    ]);
    expect(plan.actions[0]).toMatchObject({ label: "Refresh runtime" });
    expect(plan.guided).toEqual(defaultMasterActivationGuidedItems);
  });

  it("executes the allowlisted local work and never executes guided work", async () => {
    const calls: string[] = [];
    const callbacks = {
      startRuntime: vi.fn(async () => { calls.push("runtime"); }),
      enableLocalOptimization: vi.fn(async (id: string) => { calls.push(id); }),
      refreshAgentMemory: vi.fn(async () => { calls.push("memory"); }),
      refreshRepoIntelligence: vi.fn(async () => { calls.push("repo"); }),
      refreshTokenXray: vi.fn(async () => { calls.push("xray"); }),
      refreshDailyBriefing: vi.fn(async () => { calls.push("briefing"); }),
      prepareRepoMemoryMcp: vi.fn(async () => { calls.push("mcp"); }),
    };
    const plan = createMasterActivationPlan({
      runtimeState: "offline",
      supportedLocalOptimizations: ["semantic-cache", "leanctx-shadow"],
      callbacks,
    });

    const result = await executeMasterActivation(plan, {
      supportedLocalOptimizations: ["semantic-cache", "leanctx-shadow"],
      callbacks,
    });

    expect(calls).toEqual(["runtime", "semantic-cache", "leanctx-shadow", "memory", "repo", "xray", "briefing", "mcp"]);
    expect(result.failed).toHaveLength(0);
    expect(result.completed.map(({ id }) => id)).toContain("local-optimizations");
    expect(result.guided).toEqual(defaultMasterActivationGuidedItems);
  });

  it("reports callback failures and missing callbacks without claiming success", async () => {
    const failing = vi.fn(async () => {
      throw new Error("runtime unavailable");
    });
    const plan = createMasterActivationPlan({
      runtimeState: "offline",
      callbacks: { startRuntime: failing, refreshTokenXray: vi.fn() },
      guided: [{ id: "gateway-deployment", label: "Gateway", reason: "Needs credentials.", status: "gated" }],
    });
    const result = await executeMasterActivation(plan, { callbacks: { startRuntime: failing } });

    expect(result.failed).toHaveLength(1);
    expect(result.failed[0]).toMatchObject({ id: "runtime", detail: expect.stringContaining("no live claim") });
    expect(result.skipped).toEqual([{ id: "token-xray", status: "skipped", detail: "No injected callback was supplied." }]);
    expect(result.guided).toEqual([{ id: "gateway-deployment", label: "Gateway", reason: "Needs credentials.", status: "gated" }]);
  });

  it("creates a receipt-scoped inverse plan from completed activation work", async () => {
    const callbacks = { startRuntime: vi.fn(), enableLocalOptimization: vi.fn(), refreshAgentMemory: vi.fn() };
    const plan = createMasterActivationPlan({ runtimeState: "offline", supportedLocalOptimizations: ["semantic-cache"], callbacks });
    const activation = await executeMasterActivation(plan, { callbacks });
    const inverse = createMasterDeactivationPlan({ receipt: activation.receipt });

    expect(inverse.actions.map(({ id }) => id)).toEqual(["runtime", "local-optimizations", "agent-memory"]);
    expect(inverse.actions[1].optimizationIds).toEqual(["semantic-cache"]);
  });

  it("deactivates only receipt-owned work and reports guided items", async () => {
    const calls: string[] = [];
    const receipt = {
      version: 1 as const,
      ownedActions: [
        { id: "local-optimizations" as const, optimizationIds: ["semantic-cache", "unowned"] },
        { id: "agent-memory" as const, optimizationIds: [] },
      ],
      supportedLocalOptimizations: ["semantic-cache"],
    };
    const plan = createMasterDeactivationPlan({ receipt, guided: [{ id: "provider-authentication", label: "Auth", reason: "Manual.", status: "manual" }] });
    const result = await executeMasterDeactivation(plan, {
      receipt,
      callbacks: {
        disableLocalOptimization: async (id) => { calls.push(id); },
        deactivateAgentMemory: async () => { calls.push("memory"); },
      },
    });

    expect(calls).toEqual(["semantic-cache", "memory"]);
    expect(result.completed.map(({ id }) => id)).toEqual(["local-optimizations", "agent-memory"]);
    expect(result.failed).toHaveLength(0);
    expect(result.skipped).toHaveLength(0);
    expect(result.gated).toEqual([{ id: "provider-authentication", status: "manual", detail: "Manual." }]);
  });

  it("reports inverse callback failures and missing callbacks without claiming success", async () => {
    const receipt = { version: 1 as const, ownedActions: [{ id: "runtime" as const, optimizationIds: [] }, { id: "token-xray" as const, optimizationIds: [] }], supportedLocalOptimizations: [] };
    const plan = createMasterDeactivationPlan({ receipt });
    const result = await executeMasterDeactivation(plan, { receipt, callbacks: { stopRuntime: async () => { throw new Error("busy"); } } });

    expect(result.failed[0]).toMatchObject({ id: "runtime", detail: expect.stringContaining("no broader state") });
    expect(result.skipped).toEqual([{ id: "token-xray", status: "skipped", detail: "No injected deactivation callback was supplied." }]);
  });
});
