import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { WorkbenchView } from "./WorkbenchView";

const listSessions = vi.fn();
const projection = vi.fn();
const createSession = vi.fn();
const preparePlan = vi.fn();
const listRouterDecisionReferences = vi.fn();
const listReplayReferences = vi.fn();

vi.mock("../lib/tauriRuntime", () => ({ hasTauriRuntime: () => true }));
vi.mock("../lib/optimization", async () => {
  const actual = await vi.importActual<typeof import("../lib/optimization")>("../lib/optimization");
  return {
    ...actual,
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
    createWorkbenchSession: (...args: unknown[]) => createSession(...args),
    prepareWorkbenchRunPlan: (...args: unknown[]) => preparePlan(...args),
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
};

describe("WorkbenchView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listSessions.mockResolvedValue([session]);
    projection.mockResolvedValue(capabilityProjection);
    listRouterDecisionReferences.mockResolvedValue([routerDecisionReference]);
    listReplayReferences.mockResolvedValue([replayReference]);
  });

  it("surfaces the local plan-only boundary and shared capability registry", async () => {
    render(<WorkbenchView hidden={false} />);

    expect(await screen.findByText(/approval mode: fail_closed/i)).toBeInTheDocument();
    expect(screen.getByText(/provider traffic: none/i)).toBeInTheDocument();
    expect(screen.getByText(/writes: disabled/i)).toBeInTheDocument();
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
      requestedMode: "full",
      adapterPlanId: "adapter-plan:test",
      adapterAction: "apply_managed_routing",
      adapterReversible: true,
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
    }));
    expect(await screen.findByText(/plan id: run-plan:test/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /execute/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Observe-only Router decision ID")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Router evidence SHA-256 digest")).not.toBeInTheDocument();
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
      requestedMode: "full",
      adapterPlanId: "adapter-plan:test",
      adapterAction: "apply_managed_routing",
      adapterReversible: true,
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
});
