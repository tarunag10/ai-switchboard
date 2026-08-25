import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { WorkbenchView } from "./WorkbenchView";

const listSessions = vi.fn();
const projection = vi.fn();
const createSession = vi.fn();
const preparePlan = vi.fn();
const getPlanHeadCorrelationSummary = vi.fn();
const issueProcessStartGrant = vi.fn();
const listProcessStartGrants = vi.fn();
const revokeProcessStartGrant = vi.fn();
const admitProcess = vi.fn();
const listProcessAdmissions = vi.fn();
const deriveAdmissionEligibility = vi.fn();
const listRouterDecisionReferences = vi.fn();
const getEffectiveRoutingStageReceipt = vi.fn();
const listReplayReferences = vi.fn();

vi.mock("../lib/tauriRuntime", () => ({ hasTauriRuntime: () => true }));
vi.mock("../lib/optimization", async () => {
  const actual = await vi.importActual<typeof import("../lib/optimization")>("../lib/optimization");
  return {
    ...actual,
    getModelRoutingEffectiveStageReceipt: (...args: unknown[]) => getEffectiveRoutingStageReceipt(...args),
    listModelRoutingDecisionReferences: (...args: unknown[]) => listRouterDecisionReferences(...args),
  };
});
vi.mock("../lib/ossHarnessReplay", async () => {
  const actual = await vi.importActual<typeof import("../lib/ossHarnessReplay")>("../lib/ossHarnessReplay");
  return {
    ...actual,
    listOssHarnessReplayReferences: (...args: unknown[]) => listReplayReferences(...args),
  };
});
vi.mock("../lib/workbench", async () => {
  const actual = await vi.importActual<typeof import("../lib/workbench")>("../lib/workbench");
  return {
    ...actual,
    listWorkbenchSessions: (...args: unknown[]) => listSessions(...args),
    getWorkbenchCapabilityProjection: (...args: unknown[]) => projection(...args),
    getWorkbenchPlanHeadCorrelationSummary: (...args: unknown[]) => getPlanHeadCorrelationSummary(...args),
    createWorkbenchSession: (...args: unknown[]) => createSession(...args),
    prepareWorkbenchRunPlan: (...args: unknown[]) => preparePlan(...args),
    issueWorkbenchProcessStartGrant: (...args: unknown[]) => issueProcessStartGrant(...args),
    listWorkbenchProcessStartGrants: (...args: unknown[]) => listProcessStartGrants(...args),
    revokeWorkbenchProcessStartGrant: (...args: unknown[]) => revokeProcessStartGrant(...args),
    admitWorkbenchProcess: (...args: unknown[]) => admitProcess(...args),
    listWorkbenchProcessAdmissions: (...args: unknown[]) => listProcessAdmissions(...args),
    deriveWorkbenchProcessAdmissionEligibility: (...args: unknown[]) => deriveAdmissionEligibility(...args),
    exportWorkbenchSession: vi.fn(),
    forkWorkbenchSession: vi.fn(),
    transitionWorkbenchSession: vi.fn(),
  };
});

const workspaceDigest = `sha256:${"a".repeat(64)}`;
const routerDigest = `sha256:${"b".repeat(64)}`;
const routerDecisionReference = {
  schemaVersion: 1 as const,
  decisionId: "routing-decision-1",
  runId: "routing-run-1",
  capturedAt: "2026-08-23T00:00:00Z",
  taskClass: "formatting",
  decisionStage: "observe" as const,
  routingMode: "observe_only" as const,
  evidenceDigest: routerDigest,
};
const userApprovedRouterDecisionReference = {
  schemaVersion: 1 as const,
  decisionId: "routing-decision-2",
  runId: "routing-run-2",
  capturedAt: "2026-08-23T01:00:00Z",
  taskClass: "review",
  decisionStage: "userApproved" as const,
  routingMode: "observe_only" as const,
  evidenceDigest: `sha256:${"e".repeat(64)}`,
};
const replayReference = {
  schemaVersion: 1 as const,
  replayId: "replay-reference-00000000-0000-4000-8000-000000000001",
  validatedAt: "2026-08-23T00:00:00Z",
  replayMode: "redacted_observe_only" as const,
  automaticPromotion: "disabled" as const,
  providerTraffic: "none" as const,
  eventCount: 2,
  replayDigest: `sha256:${"c".repeat(64)}`,
  receiptDigest: `sha256:${"d".repeat(64)}`,
};
const workbenchPreset = {
  schemaVersion: 1,
  presetId: "adapter-plan-review",
  label: "Adapter plan review",
  description: "Draft only",
  requiredCapabilityIds: ["router_observe", "client_adapter_plan"],
  evidenceSource: "native_router_decision_receipt" as const,
  routingMode: "observe_only" as const,
  executionMode: "plan_only" as const,
  providerTraffic: "none" as const,
  writesEnabled: false as const,
};
const session = {
  schemaVersion: 1,
  sessionId: "workbench:test",
  workspaceDigest,
  taskClass: "coding" as const,
  status: "active" as const,
  parentSessionId: null,
  forkEventId: null,
  createdAt: "2026-08-23T00:00:00Z",
  updatedAt: "2026-08-23T00:00:00Z",
  executionMode: "plan_only" as const,
  providerTraffic: "none" as const,
  events: [{
    eventId: "workbench:test:0",
    sessionId: "workbench:test",
    sequence: 0,
    kind: "started",
    parentEventId: null,
    occurredAt: "2026-08-23T00:00:00Z",
  }],
};

const planHeadSummary = {
  schemaVersion: 1,
  headId: "plan-head:test",
  sessionId: session.sessionId,
  planId: "run-plan:test",
  generation: 2,
  sessionSnapshotDigest: `sha256:${"e".repeat(64)}`,
  planSnapshotDigest: `sha256:${"f".repeat(64)}`,
  predecessorHeadId: "plan-head:previous",
  predecessorRecordDigest: `sha256:${"1".repeat(64)}`,
};

const capabilityProjection = {
  schemaVersion: 1,
  executionMode: "plan_only" as const,
  writesEnabled: false as const,
  providerTraffic: "none" as const,
    registry: {
    schemaVersion: 1,
    registryMode: "metadata_only" as const,
    writesEnabled: false as const,
    approvalMode: "fail_closed" as const,
    providers: [{ id: "local", label: "Local", modelFamilies: ["test"], contextLimit: 1, authSource: "none" as const }],
      tools: [{ id: "router", label: "Router", providerId: "local", capabilities: ["observe"], requiresApproval: true, writesEnabled: false as const }],
    },
    presets: [workbenchPreset],
    adapterReadiness: [
      {
        schemaVersion: 1,
        adapterId: "codex" as const,
        adapterContractVersion: 1,
        logicalBinary: "codex" as const,
        knownCandidatePresent: false,
        discoveryMode: "fixed_known_location_metadata_only" as const,
        cliVersionProbeState: "not_probed" as const,
        versionProbeReason: "CLI version probing is deferred because it would start a process.",
        processStartEnabled: false as const,
        providerTraffic: "none" as const,
        writesEnabled: false as const,
      },
      {
        schemaVersion: 1,
        adapterId: "claude_code" as const,
        adapterContractVersion: 1,
        logicalBinary: "claude" as const,
        knownCandidatePresent: false,
        discoveryMode: "fixed_known_location_metadata_only" as const,
        cliVersionProbeState: "not_probed" as const,
        versionProbeReason: "CLI version probing is deferred because it would start a process.",
        processStartEnabled: false as const,
        providerTraffic: "none" as const,
        writesEnabled: false as const,
      },
    ],
    processStartGrantPolicy: {
      confirmationTemplate: "AUTHORIZE FUTURE PROCESS {planId}",
      ttlSeconds: 900,
      executionEnabled: false as const,
      providerTraffic: "none" as const,
      writesEnabled: false as const,
    },
};

describe("WorkbenchView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listSessions.mockResolvedValue([session]);
    projection.mockResolvedValue(capabilityProjection);
    listRouterDecisionReferences.mockResolvedValue([routerDecisionReference]);
    listReplayReferences.mockResolvedValue([replayReference]);
    getEffectiveRoutingStageReceipt.mockImplementation(async (policy: { stage: "observe" | "userApproved" | "automaticAllowlisted" }) => ({
      configuredStage: policy.stage,
      effectiveStage: "observe",
      automaticRouting: "observe_only",
      reason: policy.stage === "observe"
        ? "Evidence collection is active; no model route is executed automatically."
        : "This stage is saved as configuration only. The current completion path remains observe-only until trusted completion evidence is wired.",
    }));
    listProcessStartGrants.mockResolvedValue([]);
    listProcessAdmissions.mockResolvedValue([]);
    getPlanHeadCorrelationSummary.mockResolvedValue(planHeadSummary);
    deriveAdmissionEligibility.mockResolvedValue({
      schemaVersion: 1,
      sessionId: session.sessionId,
      evaluatedAt: "2026-08-23T00:01:00Z",
      currentPlanId: null,
      currentProcessRunId: null,
      receipts: [],
      executionEnabled: false,
      providerTraffic: "none",
      writesEnabled: false,
    });
  });

  it("surfaces labelled routing, CLI, and harness readiness without enabling execution", async () => {
    const user = userEvent.setup();
    const onOpenHarnessReplay = vi.fn();
    render(
      <WorkbenchView
        hidden={false}
        onOpenHarnessReplay={onOpenHarnessReplay}
      />,
    );

    expect(await screen.findByText(/approval mode: fail_closed/i)).toBeInTheDocument();
    expect(screen.getByText(/provider traffic: none/i)).toBeInTheDocument();
    expect(screen.getByText(/writes: disabled/i)).toBeInTheDocument();
    expect(screen.getByText(/CLI versions are not probed/i)).toBeInTheDocument();
    const readiness = screen.getByRole("article", {
      name: "Harness, CLI & routing readiness",
    });
    const routing = within(readiness).getByLabelText("Operational routing status");
    expect(within(routing).getByLabelText("Configured routing value")).toHaveTextContent("observe");
    expect(within(routing).getByLabelText("Effective routing value")).toHaveTextContent("observe");
    expect(within(routing).getByLabelText("Automatic routing value")).toHaveTextContent("observe_only");

    const adaptersList = within(readiness).getByRole("list", { name: "CLI adapter readiness" });
    expect(within(adaptersList).getByLabelText("Codex adapter status")).toHaveTextContent("No candidate metadata");
    expect(within(adaptersList).getByLabelText("Codex version status")).toHaveTextContent("Not probed");
    expect(within(adaptersList).getByLabelText("Codex process status")).toHaveTextContent("Disabled");
    expect(within(adaptersList).getByLabelText("Claude Code adapter status")).toHaveTextContent("No candidate metadata");
    expect(within(adaptersList).getByLabelText("Claude Code version status")).toHaveTextContent("Not probed");
    expect(within(adaptersList).getByLabelText("Claude Code process status")).toHaveTextContent("Disabled");
    expect(within(adaptersList).getByLabelText("Gemini CLI adapter status")).toHaveTextContent("Plan only");
    expect(within(adaptersList).getByLabelText("Gemini CLI version status")).toHaveTextContent("Not available");
    expect(within(adaptersList).getByLabelText("Gemini CLI process status")).toHaveTextContent("Disabled");
    expect(within(readiness).getByLabelText("Harness replay availability value")).toHaveTextContent(
      "1 validated receipt available",
    );
    await user.click(within(readiness).getByRole("button", { name: "Open harness replay" }));
    expect(onOpenHarnessReplay).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: /execute|start/i })).not.toBeInTheDocument();
    expect(screen.getAllByText("workbench:test").length).toBeGreaterThan(0);
  });

  it("explains the empty local ledger without suggesting a hidden execution path", async () => {
    listSessions.mockResolvedValue([]);
    render(<WorkbenchView hidden={false} />);

    expect(await screen.findByText(/no workbench sessions yet/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /execute/i })).not.toBeInTheDocument();
  });

  it("surfaces native ledger failures as an actionable error", async () => {
    listSessions.mockRejectedValue(new Error("session ledger lock is unavailable"));
    render(<WorkbenchView hidden={false} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(/ledger lock is unavailable/i);
  });

  it("keeps historical admission receipts visible without presenting them as current", async () => {
    listProcessAdmissions.mockResolvedValue([{
      schemaVersion: 1,
      admissionId: "process-admission:historical",
      sessionId: session.sessionId,
      planId: "run-plan:historical",
      processRunId: "process-run:historical",
      grantId: "process-grant:historical",
      adapterId: "codex",
      admittedAt: "2026-08-23T00:01:00Z",
      state: "authorized_not_started",
      executionEnabled: false,
      providerTraffic: "none",
      writesEnabled: false,
      receiptDigest: `sha256:${"1".repeat(64)}`,
    }]);

    render(<WorkbenchView hidden={false} />);

    expect(await screen.findByText("Session receipt center")).toBeInTheDocument();
    expect(await screen.findByText((_, element) =>
      element?.tagName === "SPAN"
      && element.textContent?.includes("not currently evaluated")
      && element.textContent.includes("historical authorized not started"),
    )).toBeInTheDocument();
    expect(deriveAdmissionEligibility).not.toHaveBeenCalled();
  });

  it("creates a local session from a digest rather than a workspace path", async () => {
    const user = userEvent.setup();
    createSession.mockResolvedValue({ ...session, sessionId: "workbench:new" });
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.type(screen.getByLabelText("Workspace SHA-256 digest"), workspaceDigest);
    await user.click(screen.getByRole("button", { name: "Create local session" }));

    expect(createSession).toHaveBeenCalledWith({
      workspaceDigest,
      taskClass: "coding",
    });
    expect((await screen.findAllByText("workbench:new")).length).toBeGreaterThan(0);
  });

  it("prepares an observe-only adapter plan without exposing execution controls", async () => {
    const user = userEvent.setup();
    preparePlan.mockResolvedValue({
      schemaVersion: 1,
      planId: "run-plan:test",
      sessionId: session.sessionId,
      adapterId: "codex",
      workspaceDigest,
      contextPackDigest: null,
      routerDecision: { decisionId: "routing-decision-1", decisionStage: "observe", routingMode: "observe_only", evidenceDigest: routerDigest },
      replayReference: null,
      preset: null,
      requestedMode: "full",
      adapterPlanId: "adapter-plan:test",
      adapterAction: "apply_managed_routing",
      adapterReversible: true,
      commandReadiness: null,
      processContainment: null,
      capabilityRequests: [],
      executionMode: "plan_only",
      providerTraffic: "none",
      writesEnabled: false,
    });
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.selectOptions(screen.getByLabelText("Observe-only Router decision"), "routing-decision-1");
    await user.click(screen.getByRole("button", { name: "Prepare plan only" }));

    expect(preparePlan).toHaveBeenCalledWith(expect.objectContaining({
      sessionId: "workbench:test",
      adapterId: "codex",
      routerDecisionId: "routing-decision-1",
      replayReferenceId: null,
      presetId: null,
    }));
    const preparedPlan = screen.getByRole("heading", { name: "Prepared plan" }).closest("article");
    expect(preparedPlan).not.toBeNull();
    expect(within(preparedPlan as HTMLElement).getByText("Plan ID: run-plan:test", { exact: true })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /execute/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Observe-only Router decision ID")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Router evidence SHA-256 digest")).not.toBeInTheDocument();
  });

  it("renders the current plan-head binding without exposing execution or fabricated metrics", async () => {
    const user = userEvent.setup();
    preparePlan.mockResolvedValue({
      schemaVersion: 1,
      planId: planHeadSummary.planId,
      sessionId: session.sessionId,
      adapterId: "codex",
      workspaceDigest,
      contextPackDigest: null,
      routerDecision: { decisionId: "routing-decision-1", decisionStage: "observe", routingMode: "observe_only", evidenceDigest: routerDigest },
      replayReference: null,
      preset: null,
      requestedMode: "full",
      adapterPlanId: "adapter-plan:test",
      adapterAction: "apply_managed_routing",
      adapterReversible: true,
      commandReadiness: null,
      processContainment: null,
      capabilityRequests: [],
      executionMode: "plan_only",
      providerTraffic: "none",
      writesEnabled: false,
    });
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.selectOptions(screen.getByLabelText("Observe-only Router decision"), "routing-decision-1");
    await user.click(screen.getByRole("button", { name: "Prepare plan only" }));

    expect(getPlanHeadCorrelationSummary).toHaveBeenCalledWith({
      sessionId: session.sessionId,
      runPlan: expect.objectContaining({
        planId: planHeadSummary.planId,
        sessionId: session.sessionId,
      }),
    });
    expect(await screen.findByRole("heading", { name: "Current plan-head correlation" })).toBeInTheDocument();
    const correlation = screen.getByRole("heading", { name: "Current plan-head correlation" }).closest("section");
    expect(correlation).not.toBeNull();
    const correlationSection = within(correlation as HTMLElement);
    expect(correlationSection.getByText(/plan id: run-plan:test.*head id: plan-head:test/i)).toBeInTheDocument();
    const sessionDigest = correlationSection.getByText("Session snapshot digest:", { exact: true }).parentElement;
    const planDigest = correlationSection.getByText("Plan snapshot digest:", { exact: true }).parentElement;
    const predecessor = correlationSection.getByText("Predecessor:", { exact: true }).parentElement;
    expect(sessionDigest).not.toBeNull();
    expect(planDigest).not.toBeNull();
    expect(predecessor).not.toBeNull();
    expect(correlation as HTMLElement).toHaveTextContent(/Generation:\s*2/);
    expect(sessionDigest as HTMLElement).toHaveTextContent(planHeadSummary.sessionSnapshotDigest);
    expect(planDigest as HTMLElement).toHaveTextContent(planHeadSummary.planSnapshotDigest);
    expect(predecessor as HTMLElement).toHaveTextContent(planHeadSummary.predecessorHeadId);
    expect(screen.queryByRole("button", { name: /execute/i })).not.toBeInTheDocument();
  });

  it("discards a late prepared plan after a visible plan input changes", async () => {
    const user = userEvent.setup();
    let resolvePlan!: (value: Record<string, unknown>) => void;
    preparePlan.mockReturnValue(new Promise((resolve) => { resolvePlan = resolve; }));
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.selectOptions(screen.getByLabelText("Observe-only Router decision"), "routing-decision-1");
    await user.click(screen.getByRole("button", { name: "Prepare plan only" }));
    await waitFor(() => expect(preparePlan).toHaveBeenCalledOnce());
    await user.selectOptions(screen.getByLabelText("Requested Switchboard mode"), "headroom");
    await act(async () => {
      resolvePlan({
        schemaVersion: 1,
        planId: "run-plan:stale",
        sessionId: session.sessionId,
        adapterId: "codex",
        workspaceDigest,
        contextPackDigest: null,
        routerDecision: { decisionId: "routing-decision-1", decisionStage: "observe", routingMode: "observe_only", evidenceDigest: routerDigest },
        replayReference: null,
        preset: null,
        requestedMode: "full",
        adapterPlanId: "adapter-plan:stale",
        adapterAction: "apply_managed_routing",
        adapterReversible: true,
        commandReadiness: null,
        processContainment: null,
        capabilityRequests: [],
        executionMode: "plan_only",
        providerTraffic: "none",
        writesEnabled: false,
      });
    });

    expect(screen.queryByText(/plan id: run-plan:stale/i)).not.toBeInTheDocument();
    expect(deriveAdmissionEligibility).not.toHaveBeenCalled();
  });

  it("attaches a native replay receipt only when redacted replay is selected", async () => {
    const user = userEvent.setup();
    preparePlan.mockResolvedValue({
      schemaVersion: 1,
      planId: "run-plan:replay",
      sessionId: session.sessionId,
      adapterId: "codex",
      workspaceDigest,
      contextPackDigest: null,
      routerDecision: { decisionId: "routing-decision-1", decisionStage: "observe", routingMode: "observe_only", evidenceDigest: routerDigest },
      replayReference,
      preset: null,
      requestedMode: "full",
      adapterPlanId: "adapter-plan:test",
      adapterAction: "apply_managed_routing",
      adapterReversible: true,
      commandReadiness: null,
      processContainment: null,
      capabilityRequests: [],
      executionMode: "plan_only",
      providerTraffic: "none",
      writesEnabled: false,
    });
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    expect(screen.getByLabelText("Validated redacted replay")).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: /redacted replay/i }));
    await user.selectOptions(screen.getByLabelText("Observe-only Router decision"), "routing-decision-1");
    await user.selectOptions(screen.getByLabelText("Validated redacted replay"), replayReference.replayId);
    await user.click(screen.getByRole("button", { name: "Prepare plan only" }));

    expect(preparePlan).toHaveBeenCalledWith(expect.objectContaining({
      replayReferenceId: replayReference.replayId,
      requiredCapabilityIds: expect.arrayContaining(["redacted_replay"]),
    }));
    expect(await screen.findByText((_, element) =>
      element?.tagName === "P" && element.textContent?.includes(replayReference.replayId),
    )).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /execute/i })).not.toBeInTheDocument();
  });

  it("loads native Workbench presets as drafts without preparing or executing a plan", async () => {
    const user = userEvent.setup();
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.selectOptions(screen.getByLabelText("Workbench plan preset"), workbenchPreset.presetId);

    expect(await screen.findByText(/loaded as a plan draft/i)).toBeInTheDocument();
    expect(preparePlan).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: /execute/i })).not.toBeInTheDocument();
  });

  it("prepares canonical adapter command readiness without probing or starting a process", async () => {
    const user = userEvent.setup();
    preparePlan.mockResolvedValue({
      schemaVersion: 1,
      planId: "run-plan:readiness",
      sessionId: session.sessionId,
      adapterId: "codex",
      workspaceDigest,
      contextPackDigest: null,
      routerDecision: { decisionId: "routing-decision-1", decisionStage: "observe", routingMode: "observe_only", evidenceDigest: routerDigest },
      replayReference: null,
      preset: null,
      requestedMode: "full",
      adapterPlanId: "codex-1234567890ab",
      adapterAction: "apply_managed_routing",
      adapterReversible: true,
      commandReadiness: {
        ...capabilityProjection.adapterReadiness[0],
        adapterPlanId: "codex-1234567890ab",
      },
      processContainment: {
        schemaVersion: 1,
        runId: "process-run:1234567890abcdef1234567890abcdef",
        sessionId: session.sessionId,
        adapterPlanId: "codex-1234567890ab",
        adapterId: "codex",
        adapterContractVersion: 1,
        workspaceDigest,
        owner: "workbench_native",
        state: "not_started",
        startAuthorization: "not_granted",
        launchMode: "native_adapter_only",
        processGroup: "required_on_unix",
        stdin: "null",
        output: "piped_bounded_redacted",
        timeoutPolicy: "native_fixed_policy_required",
        cancellation: "group_sigterm_then_sigkill",
        providerTraffic: "none",
        writesEnabled: false,
      },
      capabilityRequests: [],
      executionMode: "plan_only",
      providerTraffic: "none",
      writesEnabled: false,
    });
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.click(screen.getByRole("checkbox", { name: /adapter command readiness/i }));
    await user.selectOptions(screen.getByLabelText("Observe-only Router decision"), "routing-decision-1");
    await user.click(screen.getByRole("button", { name: "Prepare plan only" }));

    expect(preparePlan).toHaveBeenCalledWith(expect.objectContaining({
      adapterId: "codex",
      requiredCapabilityIds: expect.arrayContaining(["adapter_command_readiness"]),
    }));
    expect(await screen.findByText(/CLI version not probed/i)).toBeInTheDocument();
    expect(screen.getByText((_, element) =>
      element?.tagName === "P" && element.textContent?.includes("Native containment: not started"),
    )).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /execute|start/i })).not.toBeInTheDocument();
  });

  it("records and revokes a non-executable future process authorization from the saved plan input", async () => {
    const user = userEvent.setup();
    preparePlan.mockResolvedValue({
      schemaVersion: 1,
      planId: "run-plan:grant",
      sessionId: session.sessionId,
      adapterId: "codex",
      workspaceDigest,
      contextPackDigest: null,
      routerDecision: { decisionId: "routing-decision-1", decisionStage: "observe", routingMode: "observe_only", evidenceDigest: routerDigest },
      replayReference: null,
      preset: null,
      requestedMode: "full",
      adapterPlanId: "codex-1234567890ab",
      adapterAction: "apply_managed_routing",
      adapterReversible: true,
      commandReadiness: {
        ...capabilityProjection.adapterReadiness[0],
        adapterPlanId: "codex-1234567890ab",
      },
      processContainment: {
        schemaVersion: 1,
        runId: "process-run:1234567890abcdef1234567890abcdef",
        sessionId: session.sessionId,
        adapterPlanId: "codex-1234567890ab",
        adapterId: "codex",
        adapterContractVersion: 1,
        workspaceDigest,
        owner: "workbench_native",
        state: "not_started",
        startAuthorization: "not_granted",
        launchMode: "native_adapter_only",
        processGroup: "required_on_unix",
        stdin: "null",
        output: "piped_bounded_redacted",
        timeoutPolicy: "native_fixed_policy_required",
        cancellation: "group_sigterm_then_sigkill",
        providerTraffic: "none",
        writesEnabled: false,
      },
      capabilityRequests: [],
      executionMode: "plan_only",
      providerTraffic: "none",
      writesEnabled: false,
    });
    const activeGrant = {
      schemaVersion: 1,
      grantId: "process-grant:test",
      sessionId: session.sessionId,
      planId: "run-plan:grant",
      processRunId: "process-run:1234567890abcdef1234567890abcdef",
      capabilityId: "adapter_process_start",
      issuedAt: "2026-08-23T00:00:00Z",
      expiresAt: "2099-08-23T00:15:00Z",
      effectiveState: "active",
      executionEnabled: false,
      providerTraffic: "none",
      writesEnabled: false,
      receiptDigest: `sha256:${"e".repeat(64)}`,
    } as const;
    const revokedGrant = {
      ...activeGrant,
      effectiveState: "revoked",
      receiptDigest: `sha256:${"f".repeat(64)}`,
    } as const;
    const admission = {
      schemaVersion: 1,
      admissionId: "process-admission:test",
      sessionId: session.sessionId,
      planId: "run-plan:grant",
      processRunId: "process-run:1234567890abcdef1234567890abcdef",
      grantId: "process-grant:test",
      adapterId: "codex",
      admittedAt: "2026-08-23T00:01:00Z",
      state: "authorized_not_started",
      executionEnabled: false,
      providerTraffic: "none",
      writesEnabled: false,
      receiptDigest: `sha256:${"1".repeat(64)}`,
    } as const;
    const eligibilitySnapshot = (currentEligibility: "active" | "revoked") => ({
      schemaVersion: 1,
      sessionId: session.sessionId,
      evaluatedAt: "2026-08-23T00:01:00Z",
      currentPlanId: "run-plan:grant",
      currentProcessRunId: "process-run:1234567890abcdef1234567890abcdef",
      receipts: [{
        ...admission,
        currentEligibility,
        reason: currentEligibility === "active" ? "bound_and_current" : "grant_revoked",
        grantEffectiveState: currentEligibility,
        evaluatedAt: "2026-08-23T00:01:00Z",
        requiresStartRevalidation: true,
        executionEnabled: false,
        providerTraffic: "none",
        writesEnabled: false,
      }],
      executionEnabled: false,
      providerTraffic: "none",
      writesEnabled: false,
    });
    issueProcessStartGrant.mockResolvedValue(activeGrant);
    revokeProcessStartGrant.mockResolvedValue(revokedGrant);
    admitProcess.mockResolvedValue(admission);
    listProcessStartGrants
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([activeGrant])
      .mockResolvedValueOnce([activeGrant])
      .mockResolvedValueOnce([revokedGrant]);
    listProcessAdmissions
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([admission])
      .mockResolvedValueOnce([admission]);
    deriveAdmissionEligibility
      .mockResolvedValueOnce({ ...eligibilitySnapshot("active"), receipts: [] })
      .mockResolvedValueOnce({ ...eligibilitySnapshot("active"), receipts: [] })
      .mockResolvedValueOnce(eligibilitySnapshot("active"))
      .mockResolvedValueOnce(eligibilitySnapshot("revoked"));
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.click(screen.getByRole("checkbox", { name: /adapter command readiness/i }));
    await user.selectOptions(screen.getByLabelText("Observe-only Router decision"), "routing-decision-1");
    await user.click(screen.getByRole("button", { name: "Prepare plan only" }));

    expect(await screen.findByText("Time-limited future process authorization")).toBeInTheDocument();
    const record = screen.getByRole("button", { name: "Record 15-minute authorization" });
    expect(record).toBeDisabled();
    await user.type(
      screen.getByLabelText("Future process authorization phrase"),
      "AUTHORIZE FUTURE PROCESS run-plan:grant",
    );
    await user.click(record);

    expect(issueProcessStartGrant).toHaveBeenCalledWith(expect.objectContaining({
      expectedPlanId: "run-plan:grant",
      expectedProcessRunId: "process-run:1234567890abcdef1234567890abcdef",
      confirmationPhrase: "AUTHORIZE FUTURE PROCESS run-plan:grant",
      runSpec: expect.objectContaining({
        sessionId: session.sessionId,
        adapterId: "codex",
        workspaceDigest,
        routerDecisionId: "routing-decision-1",
        requiredCapabilityIds: expect.arrayContaining(["adapter_command_readiness"]),
      }),
    }));
    expect(await screen.findByText((_, element) =>
      element?.tagName === "SPAN" && element.textContent?.includes("active") && element.textContent.includes("non-executable"),
    )).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Validate executor eligibility" }));
    expect(admitProcess).toHaveBeenCalledWith(expect.objectContaining({ grantId: "process-grant:test" }));
    expect(await screen.findByText((_, element) =>
      element?.tagName === "SPAN" && element.textContent?.includes("authorized not started") && element.textContent.includes("non-executable"),
    )).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Revoke authorization" }));
    expect(revokeProcessStartGrant).toHaveBeenCalledWith("process-grant:test");
    expect((await screen.findAllByText((_, element) =>
      element?.tagName === "SPAN" && element.textContent?.includes("revoked") && element.textContent.includes("non-executable"),
    )).length).toBeGreaterThanOrEqual(2);
    expect(screen.queryByRole("button", { name: /execute|start/i })).not.toBeInTheDocument();
  });

  it("shows selected router summary and toggles adapter command readiness between Codex and Gemini", async () => {
    const user = userEvent.setup();
    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.selectOptions(screen.getByLabelText("Observe-only Router decision"), "routing-decision-1");
    const routerSummary = screen.getByText(/Selected Router decision:/i);
    expect(routerSummary).toHaveTextContent(
      "task class formatting",
    );
    expect(routerSummary).toHaveTextContent(
      "stage observe",
    );
    expect(routerSummary).toHaveTextContent(
      "routing mode observe_only",
    );
    expect(routerSummary).toHaveTextContent(
      routerDigest,
    );
    expect(routerSummary).toHaveClass("workbench-long-value");
    const routingStatus = await screen.findByLabelText("Operational routing status");
    expect(within(routingStatus).getByLabelText("Configured routing value")).toHaveTextContent("observe");
    expect(within(routingStatus).getByLabelText("Effective routing value")).toHaveTextContent("observe");
    expect(within(routingStatus).getByLabelText("Automatic routing value")).toHaveTextContent("observe_only");
    expect(routingStatus).toHaveTextContent(
      "Evidence collection is active; no model route is executed automatically.",
    );
    expect(screen.getByRole("checkbox", { name: /adapter command readiness/i })).toBeEnabled();

    await user.selectOptions(screen.getByLabelText("Client adapter"), "gemini_cli");

    expect(screen.getByRole("checkbox", { name: /adapter command readiness/i })).toBeDisabled();
    expect(screen.getByText(/Gemini remains adapter-plan-only/i)).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Client adapter"), "codex");

    expect(screen.getByRole("checkbox", { name: /adapter command readiness/i })).toBeEnabled();
    expect(screen.queryByText(/Gemini remains adapter-plan-only/i)).not.toBeInTheDocument();
    expect(preparePlan).not.toHaveBeenCalled();
  });

  it("falls back to a local routing preview when the receipt helper fails", async () => {
    const user = userEvent.setup();
    getEffectiveRoutingStageReceipt.mockRejectedValueOnce(new Error("routing stage unavailable"));
    listRouterDecisionReferences.mockResolvedValue([userApprovedRouterDecisionReference]);

    render(<WorkbenchView hidden={false} />);
    await screen.findAllByText("workbench:test");

    await user.selectOptions(screen.getByLabelText("Observe-only Router decision"), "routing-decision-2");

    const status = await screen.findByLabelText("Operational routing status");
    expect(within(status).getByLabelText("Configured routing value")).toHaveTextContent("userApproved");
    expect(within(status).getByLabelText("Effective routing value")).toHaveTextContent("observe");
    expect(within(status).getByLabelText("Automatic routing value")).toHaveTextContent("observe_only");
    expect(status).toHaveTextContent(
      "This stage is saved as configuration only. The current completion path remains observe-only until trusted completion evidence is wired.",
    );
    expect(screen.getByText(/Selected Router decision:/i)).toHaveTextContent("task class review");
    expect(screen.getByText(/Selected Router decision:/i)).toHaveTextContent("stage userApproved");
    expect(screen.getByText(/Selected Router decision:/i)).toHaveTextContent("evidence digest");
  });
});
