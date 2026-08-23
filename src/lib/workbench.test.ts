import { describe, expect, it, vi } from "vitest";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import {
  admitWorkbenchProcess,
  createWorkbenchSession,
  getWorkbenchCapabilityProjection,
  issueWorkbenchProcessStartGrant,
  isWorkbenchDigest,
  listWorkbenchProcessAdmissions,
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

  it("loads OSS capabilities only through the shared Workbench projection", async () => {
    const projection = {
      schemaVersion: 1,
      executionMode: "plan_only",
      writesEnabled: false,
      providerTraffic: "none",
      registry: {
        schemaVersion: 1,
        registryMode: "metadata_only",
        writesEnabled: false,
        approvalMode: "fail_closed",
        providers: [],
        tools: [],
      },
      presets: [],
      adapterReadiness: [],
      processStartGrantPolicy: {
        schemaVersion: 1,
        capabilityId: "workbench_process_start",
        ttlSeconds: 900,
        confirmationPhraseTemplate: "AUTHORIZE FUTURE PROCESS {planId}",
        executionEnabled: false,
        providerTraffic: "none",
        writesEnabled: false,
      },
    } as const;
    invoke.mockResolvedValueOnce(projection);

    await expect(getWorkbenchCapabilityProjection()).resolves.toEqual(projection);
    expect(invoke).toHaveBeenLastCalledWith("get_workbench_capability_projection");
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

it("submits process admissions through the scoped native command", async () => {
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
  invoke.mockResolvedValueOnce({ admissionId: "process-admission:test" });

  await admitWorkbenchProcess({
    runSpec,
    expectedPlanId: "run-plan:test",
    expectedProcessRunId: "process-run:test",
    grantId: "process-grant:test",
  });

  expect(invoke).toHaveBeenCalledOnce();
  expect(invoke).toHaveBeenCalledWith("admit_workbench_process", {
    input: {
      runSpec,
      expectedPlanId: "run-plan:test",
      expectedProcessRunId: "process-run:test",
      grantId: "process-grant:test",
    },
  });
});

it("lists process admissions through the scoped native command", async () => {
  invoke.mockClear();
  invoke.mockResolvedValueOnce([]);

  await listWorkbenchProcessAdmissions("workbench:test");

  expect(invoke).toHaveBeenCalledOnce();
  expect(invoke).toHaveBeenCalledWith("list_workbench_process_admissions", {
    sessionId: "workbench:test",
  });
});
