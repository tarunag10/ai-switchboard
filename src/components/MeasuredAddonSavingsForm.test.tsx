import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { MeasuredAddonSavingsForm } from "./MeasuredAddonSavingsForm";

const recordMeasuredAddonSavings = vi.fn();

vi.mock("../lib/measuredSavingsAttribution", () => ({
  recordMeasuredAddonSavings: (...args: unknown[]) =>
    recordMeasuredAddonSavings(...args),
}));

describe("MeasuredAddonSavingsForm", () => {
  it("requires independently described baseline and optimized evidence", async () => {
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
        measurementEvidence: {
          baseline: "Local request counter before Ponytail",
          optimized: "Local request counter after Ponytail",
        },
      }),
    );
  });
});
