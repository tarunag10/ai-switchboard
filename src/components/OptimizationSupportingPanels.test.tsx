import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ModelRoutingExperimentCard } from "./ModelRoutingExperimentCard";
import { ModelRoutingEvidenceCapture } from "./ModelRoutingEvidenceCapture";
import { OptimizationActionPanel, PreemptiveCompactionButton } from "./OptimizationActionControls";
import { OptimizationStatusIcon, PromptCacheClientProofList, RoutingValidationPanel } from "./OptimizationValidationPanels";

const mocks = vi.hoisted(() => ({
  loadAction: vi.fn(), saveAction: vi.fn(), compact: vi.fn(), validate: vi.fn(),
  loadRouting: vi.fn(), saveRouting: vi.fn(),
  listRoutingPresets: vi.fn(), getEffectiveRoutingStage: vi.fn(),
  exportEvidenceForHandle: vi.fn(),
  recordEvidence: vi.fn(),
  issueCompletionHandle: vi.fn(), completeCompletion: vi.fn(),
}));

vi.mock("../lib/optimization", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/optimization")>()),
  loadOptimizationActionPolicy: mocks.loadAction,
  saveOptimizationActionPolicy: mocks.saveAction,
  runPreemptiveCompaction: mocks.compact,
  validateModelRouting: mocks.validate,
  loadModelRoutingExperimentPolicy: mocks.loadRouting,
  saveModelRoutingExperimentPolicy: mocks.saveRouting,
  listModelRoutingPolicyPresets: mocks.listRoutingPresets,
  getModelRoutingEffectiveStageReceipt: mocks.getEffectiveRoutingStage,
  exportModelRoutingEvidenceForHandle: mocks.exportEvidenceForHandle,
  recordModelRoutingEvidence: mocks.recordEvidence,
  issueModelRoutingCompletionHandle: mocks.issueCompletionHandle,
  completeModelRoutingCompletion: mocks.completeCompletion,
}));

const actionPolicy = {
  promptCacheReorderEnabled: false,
  preemptiveCompactionEnabled: false,
  modelRoutingEnabled: false,
};

const routingPolicy = {
  stage: "observe" as const,
  globalEnabled: false,
  disabledClients: [] as string[],
  automaticTaskAllowlist: [] as string[],
  thresholds: {
    minimumSampleSize: 100,
    maximumSuccessRegressionBps: 50,
    maximumQualityRegressionBps: 50,
    minimumCostImprovementBps: 1_000,
    maximumReworkRateBps: 500,
    maximumLatencyRegressionMs: 50,
  },
};

const routingPreset = {
  schemaVersion: 1,
  presetId: "pause-experiments",
  label: "Pause experiments",
  description: "Draft only",
  policy: { ...routingPolicy, globalEnabled: false },
  routingMode: "observe_only" as const,
  providerTraffic: "none" as const,
  writesEnabled: false as const,
};

describe("optimization supporting panels", () => {
  beforeEach(() => {
    for (const mock of Object.values(mocks)) mock.mockReset();
    mocks.loadAction.mockResolvedValue(actionPolicy);
    mocks.saveAction.mockImplementation(async (value) => value);
    mocks.loadRouting.mockResolvedValue(routingPolicy);
    mocks.saveRouting.mockImplementation(async (value) => value);
    mocks.listRoutingPresets.mockResolvedValue([routingPreset]);
    mocks.getEffectiveRoutingStage.mockImplementation(async (policy) => ({
      configuredStage: policy.stage,
      effectiveStage: "observe",
      automaticRouting: "observe_only",
      reason: "observe-only",
    }));
  });

  it("toggles individual action policy controls and enables all", async () => {
    render(<OptimizationActionPanel />);
    fireEvent.click(await screen.findByRole("button", { name: "Prompt cache reorder: off" }));
    await waitFor(() => expect(mocks.saveAction).toHaveBeenCalledWith(expect.objectContaining({ promptCacheReorderEnabled: true })));
    fireEvent.click(screen.getByRole("button", { name: "Enable all" }));
    await waitFor(() => expect(mocks.saveAction).toHaveBeenLastCalledWith(expect.objectContaining({
      promptCacheReorderEnabled: true,
      preemptiveCompactionEnabled: true,
      modelRoutingEnabled: true,
    })));
  });

  it("runs preemptive compaction and renders its receipt", async () => {
    mocks.compact.mockResolvedValue({ action: "queued", contextUsedPercent: 82, thresholdPercent: 80 });
    render(<PreemptiveCompactionButton />);
    fireEvent.click(screen.getByRole("button", { name: /Run compaction/i }));
    expect(await screen.findByRole("status")).toHaveTextContent("queued 82% used");
  });

  it("renders cache proofs and both validation success and failure", async () => {
    const { rerender } = render(<PromptCacheClientProofList clients={[]} />);
    expect(screen.getByText("No provider cache telemetry yet.")).toBeInTheDocument();
    rerender(<PromptCacheClientProofList clients={[{ client: "Codex", provider: "OpenAI", efficiencyPercent: 75, proof: "measured", promptTokens: 2_000, cacheReadTokens: 1_250, cacheCreationTokens: 250 }]} />);
    expect(screen.getByText("1,250 cache hits")).toBeInTheDocument();
    expect(OptimizationStatusIcon({ status: "blocked" })).toBeTruthy();
    expect(OptimizationStatusIcon({ status: "passed" })).toBeTruthy();

    mocks.validate.mockResolvedValue({ checks: [{ client: "Codex", task: "formatting", status: "passed", selectedModel: "small", fallbackModel: "large", reason: "gate passed" }] });
    const success = render(<RoutingValidationPanel />);
    fireEvent.click(screen.getByRole("button", { name: /Validate routing/i }));
    expect(await screen.findByText("passed: small")).toBeInTheDocument();
    success.unmount();
    mocks.validate.mockRejectedValue(new Error("probe unavailable"));
    render(<RoutingValidationPanel />);
    fireEvent.click(screen.getByRole("button", { name: /Validate routing/i }));
    expect(await screen.findByText("probe unavailable")).toBeInTheDocument();
  });

  it("edits and saves the model-routing experiment policy", async () => {
    render(<ModelRoutingExperimentCard />);
    await waitFor(() => expect(mocks.loadRouting).toHaveBeenCalled());
    fireEvent.change(screen.getByLabelText("Model-routing experiment stage"), { target: { value: "userApproved" } });
    fireEvent.click(screen.getByRole("checkbox", { name: /Enable model-routing experiments globally/i }));
    fireEvent.change(screen.getByLabelText("Clients excluded from model routing"), { target: { value: "codex, claude_code" } });
    fireEvent.click(screen.getByRole("checkbox", { name: "formatting" }));
    fireEvent.click(screen.getByRole("button", { name: "Save routing policy" }));
    await waitFor(() => expect(mocks.saveRouting).toHaveBeenCalledWith(expect.objectContaining({
      stage: "userApproved",
      globalEnabled: true,
      disabledClients: ["codex", "claude_code"],
      automaticTaskAllowlist: ["formatting"],
    })));
    expect(await screen.findByText("Model-routing experiment policy saved locally.")).toBeInTheDocument();
  });

  it("surfaces routing policy persistence errors", async () => {
    mocks.saveRouting.mockRejectedValue("storage blocked");
    render(<ModelRoutingExperimentCard />);
    fireEvent.click(await screen.findByRole("button", { name: "Save routing policy" }));
    expect(await screen.findByText("storage blocked")).toBeInTheDocument();
  });

  it("loads a native policy preset as a draft without saving or validating routes", async () => {
    render(<ModelRoutingExperimentCard />);
    await screen.findByRole("button", { name: "Load Pause experiments" });
    fireEvent.click(screen.getByRole("button", { name: "Load Pause experiments" }));

    expect(await screen.findByText(/unsaved draft/i)).toBeInTheDocument();
    expect(mocks.saveRouting).not.toHaveBeenCalled();
    expect(mocks.validate).not.toHaveBeenCalled();
    expect(mocks.issueCompletionHandle).not.toHaveBeenCalled();
  });

  it("disables explicit evidence recording until an observation is supplied", async () => {
    render(<ModelRoutingEvidenceCapture />);

    const recordButton = screen.getByRole("button", { name: "Record supplied observation" });
    expect(recordButton).toBeDisabled();
    expect(screen.getByText("No explicit routing observation supplied yet.")).toBeInTheDocument();
    fireEvent.click(recordButton);
    expect(mocks.recordEvidence).not.toHaveBeenCalled();
  });

  it("forwards an explicitly supplied observation without adding defaults", async () => {
    const observation = {
      runId: "native-run-2",
      capturedAt: "2026-08-24T10:00:00Z",
      taskClass: "diff_summary",
      arm: "candidate" as const,
      baselineModel: "frontier",
      candidateModel: "fast/local",
      succeeded: false,
      successfulTaskCostMicrounits: null,
      qualityScoreBps: 9123,
      latencyMs: 456,
      followUpRework: true,
    };
    mocks.recordEvidence.mockResolvedValueOnce(undefined);

    render(<ModelRoutingEvidenceCapture observation={observation} />);

    const recordButton = screen.getByRole("button", { name: "Record supplied observation" });
    expect(recordButton).toBeEnabled();
    expect(screen.getByText(/Supplied observation ready: native-run-2 · diff_summary · candidate/)).toBeInTheDocument();

    fireEvent.click(recordButton);

    await waitFor(() => expect(mocks.recordEvidence).toHaveBeenCalledWith(observation));
    expect(screen.getByText("Supplied routing observation recorded exactly as provided.")).toBeInTheDocument();
    expect(mocks.recordEvidence).toHaveBeenCalledTimes(1);
  });

  it("lets the parent pass an explicit observation through to the capture panel", async () => {
    const observation = {
      runId: "native-run-3",
      capturedAt: "2026-08-24T10:15:00Z",
      taskClass: "formatting",
      arm: "baseline" as const,
      baselineModel: "frontier",
      candidateModel: "fast/local",
      succeeded: true,
      successfulTaskCostMicrounits: 800,
      qualityScoreBps: 9900,
      latencyMs: 500,
      followUpRework: false,
    };
    mocks.recordEvidence.mockResolvedValueOnce(undefined);

    render(<ModelRoutingExperimentCard evidenceObservation={observation} />);

    const recordButton = screen.getByRole("button", { name: "Record supplied observation" });
    expect(recordButton).toBeEnabled();
    fireEvent.click(recordButton);

    await waitFor(() => expect(mocks.recordEvidence).toHaveBeenCalledWith(observation));
  });

  it("keeps completion disabled until the explicit success, quality, latency, and cost inputs are supplied", async () => {
    mocks.issueCompletionHandle.mockResolvedValue({
      handleId: "handle-1",
      runId: "native-run-1",
      issuedAt: "2026-08-23T00:00:00Z",
      expiresAt: "2026-08-23T00:10:00Z",
      decision: { stage: "observe", selectedModel: "fast/local" },
    });
    mocks.completeCompletion.mockResolvedValue({
      schemaVersion: 1,
      decisionId: "routing-decision-1",
      runId: "native-run-1",
      capturedAt: "2026-08-23T00:00:00Z",
      taskClass: "formatting",
      decisionStage: "observe",
      routingMode: "observe_only",
      evidenceDigest: `sha256:${"a".repeat(64)}`,
    });
    mocks.exportEvidenceForHandle.mockResolvedValue({
      evidenceClass: "local_runtime_observation",
      promotionEligible: false,
      provenance: { runId: "native-run-1" },
    });
    render(<ModelRoutingExperimentCard />);
    fireEvent.change(screen.getByLabelText("Routing requested model"), { target: { value: "frontier" } });
    fireEvent.change(screen.getByLabelText("Routing cheap model"), { target: { value: "fast/local" } });
    fireEvent.change(screen.getByLabelText("Routing capable model"), { target: { value: "frontier" } });
    fireEvent.click(screen.getByRole("button", { name: "Issue completion handle" }));
    await waitFor(() => expect(mocks.issueCompletionHandle).toHaveBeenCalledWith(expect.objectContaining({
      client: "codex", task: "formatting", requestedModel: "frontier", cheapModel: "fast/local", capableModel: "frontier",
    })));
    const completeButton = await screen.findByRole("button", { name: "Complete provider outcome" });
    expect(completeButton).toBeDisabled();
    fireEvent.click(completeButton);
    expect(mocks.completeCompletion).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText("Successful task outcome"), { target: { value: "succeeded" } });
    fireEvent.change(screen.getByLabelText("Quality score"), { target: { value: "9800" } });
    fireEvent.change(screen.getByLabelText("Latency"), { target: { value: "700" } });
    expect(screen.getByRole("button", { name: "Complete provider outcome" })).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Successful task cost"), { target: { value: "900" } });

    await waitFor(() => expect(screen.getByRole("button", { name: "Complete provider outcome" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "Complete provider outcome" }));
    await waitFor(() => expect(mocks.completeCompletion).toHaveBeenCalledWith("handle-1", expect.objectContaining({
      succeeded: true,
      successfulTaskCostMicrounits: 900,
      qualityScoreBps: 9800,
      latencyMs: 700,
    })));
    expect(await screen.findByText(/routing-decision-1/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Export completion evidence" }));
    await waitFor(() => expect(mocks.exportEvidenceForHandle).toHaveBeenCalledWith("handle-1", "formatting"));
  });

  it("allows failed outcomes to complete without a successful-task cost once the remaining fields are explicit", async () => {
    mocks.issueCompletionHandle.mockResolvedValue({
      handleId: "handle-2",
      runId: "native-run-2",
      issuedAt: "2026-08-23T00:00:00Z",
      expiresAt: "2026-08-23T00:10:00Z",
      decision: { stage: "observe", selectedModel: "fast/local" },
    });
    mocks.completeCompletion.mockResolvedValue({
      schemaVersion: 1,
      decisionId: "routing-decision-2",
      runId: "native-run-2",
      capturedAt: "2026-08-23T00:00:00Z",
      taskClass: "formatting",
      decisionStage: "observe",
      routingMode: "observe_only",
      evidenceDigest: `sha256:${"b".repeat(64)}`,
    });
    render(<ModelRoutingExperimentCard />);
    fireEvent.click(screen.getByRole("button", { name: "Issue completion handle" }));
    await waitFor(() => expect(mocks.issueCompletionHandle).toHaveBeenCalled());

    fireEvent.change(screen.getByLabelText("Successful task outcome"), { target: { value: "failed" } });
    fireEvent.change(screen.getByLabelText("Quality score"), { target: { value: "9123" } });
    fireEvent.change(screen.getByLabelText("Latency"), { target: { value: "456" } });

    const completeButton = screen.getByRole("button", { name: "Complete provider outcome" });
    await waitFor(() => expect(completeButton).toBeEnabled());
    fireEvent.click(completeButton);

    await waitFor(() => expect(mocks.completeCompletion).toHaveBeenCalledWith("handle-2", expect.objectContaining({
      succeeded: false,
      successfulTaskCostMicrounits: null,
      qualityScoreBps: 9123,
      latencyMs: 456,
    })));
  });
});
