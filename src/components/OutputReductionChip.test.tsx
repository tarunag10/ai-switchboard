import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { OutputReductionChip } from "./OutputReductionChip";

const measured = {
  method: "measured" as const,
  reductionPercent: 25,
  ciLowPercent: 20,
  ciHighPercent: 30,
  requests: 1_250,
};

describe("OutputReductionChip", () => {
  it("opens measured details and closes on Escape", () => {
    render(<OutputReductionChip reduction={measured} />);
    const trigger = screen.getByRole("button", { name: "Output token reduction details" });
    fireEvent.click(trigger);
    expect(screen.getByRole("dialog", { name: "Output reduction details" })).toHaveTextContent(
      "measured",
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("describes estimated evidence and closes on an outside pointer", () => {
    render(<OutputReductionChip reduction={{ ...measured, method: "estimated" }} />);
    fireEvent.click(screen.getByRole("button", { name: "Output token reduction details" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("counterfactual");
    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
