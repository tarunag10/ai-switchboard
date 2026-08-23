import { describe, expect, it, vi } from "vitest";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import {
  createWorkbenchSession,
  isWorkbenchDigest,
  prepareWorkbenchRunPlan,
  transitionWorkbenchSession,
} from "./workbench";

describe("workbench bridge", () => {
  it("only recognizes SHA-256 references for content-free workspace inputs", () => {
    expect(isWorkbenchDigest(`sha256:${"a".repeat(64)}`)).toBe(true);
    expect(isWorkbenchDigest("/Users/alice/project")).toBe(false);
    expect(isWorkbenchDigest("sha256:short")).toBe(false);
  });

  it("sends session lifecycle and plan requests through their scoped native commands", async () => {
    invoke.mockResolvedValue({ sessionId: "workbench:test" });
    await createWorkbenchSession({
      workspaceDigest: `sha256:${"a".repeat(64)}`,
      taskClass: "coding",
    });
    await transitionWorkbenchSession("workbench:test", "pause");
    await prepareWorkbenchRunPlan({
      sessionId: "workbench:test",
      adapterId: "codex",
      workspaceDigest: `sha256:${"a".repeat(64)}`,
      contextPackDigest: null,
      routerDecisionId: "routing-decision-1",
      requiredCapabilityIds: ["router_observe"],
      requestedMode: "full",
    });

    expect(invoke).toHaveBeenNthCalledWith(1, "create_workbench_session", {
      input: {
        workspaceDigest: `sha256:${"a".repeat(64)}`,
        taskClass: "coding",
      },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "transition_workbench_session", {
      input: { sessionId: "workbench:test", action: "pause" },
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "prepare_workbench_run_plan", {
      input: expect.objectContaining({
        sessionId: "workbench:test",
        adapterId: "codex",
        requiredCapabilityIds: ["router_observe"],
      }),
    });
  });
});
