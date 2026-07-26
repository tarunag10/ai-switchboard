import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { MeasuredAddonSavingsForm } from "./MeasuredAddonSavingsForm";

const recordMeasuredAddonSavings = vi.fn();
const loadTokenXraySnapshot = vi.fn();

vi.mock("../lib/measuredSavingsAttribution", () => ({
  recordMeasuredAddonSavings: (...args: unknown[]) =>
    recordMeasuredAddonSavings(...args),
}));

vi.mock("../lib/usageAnalytics", () => ({
  loadTokenXraySnapshot: (...args: unknown[]) => loadTokenXraySnapshot(...args),
}));

describe("MeasuredAddonSavingsForm", () => {
  it("requires independently described baseline and optimized evidence", async () => {
    loadTokenXraySnapshot.mockResolvedValue({
      sessionId: "session-123",
      provider: "local-provider",
      model: "local-model",
      generatedAt: Date.parse("2026-07-26T10:00:00.000Z"),
      metrics: {
        inputTokens: {
          value: 1200,
          confidence: "measured",
          source: "local Token X-Ray",
          observedAt: Date.parse("2026-07-26T09:59:00.000Z"),
        },
      },
    });
    recordMeasuredAddonSavings.mockResolvedValue({
      recorded: true,
      tokensSaved: 800,
    });
    const user = userEvent.setup();
    render(
      <MeasuredAddonSavingsForm
        source="ponytail"
        label="Ponytail"
        onRecorded={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    const submit = screen.getByRole("button", {
      name: /record measured sample/i,
    });
    expect(submit).toBeDisabled();
    expect(
      screen.getByText(/savings remain estimated/i),
    ).toBeInTheDocument();

    await user.type(screen.getByLabelText("Before"), "1200");
    await user.type(screen.getByLabelText("After"), "400");
    await user.clear(screen.getByLabelText("Request count / delta"));
    await user.type(screen.getByLabelText("Request count / delta"), "3");
    expect(submit).toBeDisabled();

    await user.type(
      screen.getByLabelText("Baseline evidence"),
      "Local request counter before Ponytail",
    );
    expect(submit).toBeDisabled();

    await user.type(
      screen.getByLabelText("Optimized evidence"),
      "Local request counter after Ponytail",
    );
    expect(submit).toBeEnabled();
    await user.click(submit);

    expect(recordMeasuredAddonSavings).toHaveBeenCalledWith(
      expect.objectContaining({
        baselineTokens: 1200,
        optimizedTokens: 400,
        requestDelta: 3,
        measurementEvidence: {
          baseline: "Local request counter before Ponytail",
          optimized: "Local request counter after Ponytail",
        },
      }),
    );
  });

  it("captures local X-Ray evidence into either side with provenance", async () => {
    loadTokenXraySnapshot.mockResolvedValue({
      sessionId: "session-123",
      provider: "provider-a",
      model: "model-a",
      generatedAt: Date.parse("2026-07-26T10:00:00.000Z"),
      metrics: {
        inputTokens: {
          value: 840,
          confidence: "measured",
          source: "local Token X-Ray",
          observedAt: Date.parse("2026-07-26T09:59:00.000Z"),
        },
      },
    });
    const user = userEvent.setup();
    render(
      <MeasuredAddonSavingsForm
        source="caveman"
        label="Caveman"
        onRecorded={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    const beforeCapture = await screen.findByRole("button", {
      name: "Capture X-Ray into Before",
    });
    expect(beforeCapture).toBeEnabled();
    await user.click(beforeCapture);
    expect(screen.getByLabelText("Before")).toHaveValue(840);
    expect(screen.getByLabelText("Baseline evidence").getAttribute("value")).toContain(
      "session session-123",
    );
    expect(screen.getByLabelText("Baseline evidence").getAttribute("value")).toContain(
      "provider provider-a",
    );
    expect(screen.getByLabelText("Baseline evidence").getAttribute("value")).toContain(
      "observed 2026-07-26T09:59:00.000Z",
    );

    await user.click(screen.getByRole("button", { name: "Capture X-Ray into After" }));
    expect(screen.getByLabelText("After")).toHaveValue(840);
    expect(screen.getByLabelText("Optimized evidence").getAttribute("value")).toContain(
      "model model-a",
    );
  });

  it("keeps capture disabled when X-Ray input tokens are unavailable", async () => {
    loadTokenXraySnapshot.mockResolvedValue({
      sessionId: null,
      provider: null,
      model: null,
      generatedAt: 0,
      metrics: {
        inputTokens: {
          value: null,
          confidence: "unavailable",
          source: "local Token X-Ray",
          observedAt: null,
        },
      },
    });
    render(
      <MeasuredAddonSavingsForm
        source="markitdown"
        label="MarkItDown"
        onRecorded={vi.fn().mockResolvedValue(undefined)}
      />,
    );
    expect(
      await screen.findByRole("button", { name: "Capture X-Ray into Before" }),
    ).toBeDisabled();
    expect(
      screen.getByText(/credible local or external counters may be entered manually/i),
    ).toBeInTheDocument();
  });
});
