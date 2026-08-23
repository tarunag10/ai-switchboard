import { describe, expect, it, vi } from "vitest";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import {
  createWorkbenchSession,
  issueWorkbenchProcessStartGrant,
  isWorkbenchDigest,
  listWorkbenchProcessStartGrants,
  prepareWorkbenchRunPlan,
  revokeWorkbenchProcessStartGrant,
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
      replayReferenceId: null,
      presetId: null,
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

it("keeps future process authorizations scoped to an opaque prepared plan", async () => {
  invoke.mockClear();
  const runSpec = {
    sessionId: "workbench:test",
    adapterId: "codex" as const,
    workspaceDigest: `sha256:${"a".repeat(64)}`,
    contextPackDigest: null,
    routerDecisionId: "routing-decision-1",
    replayReferenceId: null,
    presetId: null,
    requiredCapabilityIds: ["router_observe", "adapter_command_readiness"],
    requestedMode: "full" as const,
  };
  invoke.mockResolvedValue({ grantId: "process-grant:test" });

  await issueWorkbenchProcessStartGrant({
    runSpec,
    expectedPlanId: "run-plan:test",
    expectedProcessRunId: "process-run:test",
    confirmationPhrase: "AUTHORIZE FUTURE PROCESS run-plan:test",
  });
  await listWorkbenchProcessStartGrants("workbench:test");
  await revokeWorkbenchProcessStartGrant("process-grant:test");

  expect(invoke).toHaveBeenNthCalledWith(1, "issue_workbench_process_start_grant", {
    input: {
      runSpec,
      expectedPlanId: "run-plan:test",
      expectedProcessRunId: "process-run:test",
      confirmationPhrase: "AUTHORIZE FUTURE PROCESS run-plan:test",
    },
  });
  expect(invoke).toHaveBeenNthCalledWith(2, "list_workbench_process_start_grants", {
    sessionId: "workbench:test",
  });
  expect(invoke).toHaveBeenNthCalledWith(3, "revoke_workbench_process_start_grant", {
    grantId: "process-grant:test",
  });
});
