import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ModelRoutingExperimentCard } from "./ModelRoutingExperimentCard";
import { OptimizationActionPanel, PreemptiveCompactionButton } from "./OptimizationActionControls";
import { OptimizationStatusIcon, PromptCacheClientProofList, RoutingValidationPanel } from "./OptimizationValidationPanels";

const mocks = vi.hoisted(() => ({
  loadAction: vi.fn(), saveAction: vi.fn(), compact: vi.fn(), validate: vi.fn(),
  loadRouting: vi.fn(), saveRouting: vi.fn(),
}));

vi.mock("../lib/optimization", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/optimization")>()),
  loadOptimizationActionPolicy: mocks.loadAction,
  saveOptimizationActionPolicy: mocks.saveAction,
  runPreemptiveCompaction: mocks.compact,
  validateModelRouting: mocks.validate,
  loadModelRoutingExperimentPolicy: mocks.loadRouting,
  saveModelRoutingExperimentPolicy: mocks.saveRouting,
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

describe("optimization supporting panels", () => {
  beforeEach(() => {
    for (const mock of Object.values(mocks)) mock.mockReset();
    mocks.loadAction.mockResolvedValue(actionPolicy);
    mocks.saveAction.mockImplementation(async (value) => value);
    mocks.loadRouting.mockResolvedValue(routingPolicy);
    mocks.saveRouting.mockImplementation(async (value) => value);
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
    expect(await screen.findByRole("status")).toHaveTextContent("saved locally");
  });

  it("surfaces routing policy persistence errors", async () => {
    mocks.saveRouting.mockRejectedValue("storage blocked");
    render(<ModelRoutingExperimentCard />);
    fireEvent.click(await screen.findByRole("button", { name: "Save routing policy" }));
    expect(await screen.findByRole("status")).toHaveTextContent("storage blocked");
  });
});
