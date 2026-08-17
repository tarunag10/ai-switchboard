import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ProviderBilledCounterfactualCard } from "./ProviderBilledCounterfactualCard";

const mocks = vi.hoisted(() => ({
  load: vi.fn(),
  record: vi.fn(),
}));

vi.mock("../lib/providerBilledCounterfactual", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/providerBilledCounterfactual")>()),
  loadProviderBilledUsageSnapshot: mocks.load,
  recordProviderBilledCounterfactual: mocks.record,
}));

describe("ProviderBilledCounterfactualCard", () => {
  beforeEach(() => {
    localStorage.clear();
    mocks.load.mockReset();
    mocks.record.mockReset();
  });

  it("captures both provider readings and records the measured pair", async () => {
    const onRecorded = vi.fn().mockResolvedValue(undefined);
    mocks.load
      .mockResolvedValueOnce({
        provider: "headroom_stats",
        billedInputTokens: 3_000,
        sourceEndpoint: "/stats",
        observedAt: "2026-08-16T00:00:00.000Z",
      })
      .mockResolvedValueOnce({
        provider: "headroom_stats",
        billedInputTokens: 1_200,
        sourceEndpoint: "/stats",
        observedAt: "2026-08-16T00:05:00.000Z",
      });
    mocks.record.mockResolvedValue({
      recorded: true,
      tokensSaved: 1_800,
      requestDelta: 2,
      confidence: "measured",
    });
    render(<ProviderBilledCounterfactualCard onRecorded={onRecorded} />);

    fireEvent.click(screen.getByRole("button", { name: "Capture Headroom baseline" }));
    await waitFor(() => expect(screen.getByLabelText("Baseline billed tokens")).toHaveValue("3000"));
    fireEvent.click(screen.getByRole("button", { name: "Capture Headroom optimized" }));
    await waitFor(() => expect(screen.getByLabelText("Optimized billed tokens")).toHaveValue("1200"));
    fireEvent.change(screen.getByLabelText("Matched request count"), { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: "Record measured savings" }));

    await waitFor(() => expect(onRecorded).toHaveBeenCalledTimes(1));
    expect(mocks.record).toHaveBeenCalledWith(expect.objectContaining({
      provider: "headroom_stats",
      baselineTokens: 3_000,
      optimizedTokens: 1_200,
      requestDelta: 2,
    }));
    expect(screen.getByRole("status")).toHaveTextContent("1,800");
  });

  it("reports unavailable captures, validation failures, thrown errors, and persists opt-in", async () => {
    mocks.load.mockResolvedValue(null);
    mocks.record
      .mockResolvedValueOnce({ recorded: false, tokensSaved: 0, requestDelta: 1, confidence: "estimated", reason: "empty_delta" })
      .mockRejectedValueOnce(new Error("ledger unavailable"));
    render(<ProviderBilledCounterfactualCard onRecorded={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Capture Headroom baseline" }));
    await screen.findByText(/counters are unavailable/i);
    fireEvent.click(screen.getByRole("button", { name: "Record measured savings" }));
    await screen.findByText(/empty_delta/i);
    fireEvent.click(screen.getByRole("button", { name: "Record measured savings" }));
    await screen.findByText("ledger unavailable");
    fireEvent.click(screen.getByRole("checkbox", { name: /Opt in to weekly sampling/i }));
    expect(localStorage.getItem("ai-switchboard.provider-billed-sampling.v1")).toContain('"enabled":true');
  });
});
