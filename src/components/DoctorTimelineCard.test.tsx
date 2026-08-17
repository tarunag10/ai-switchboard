import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DoctorTimelineCard } from "./DoctorTimelineCard";

const event = { id: "1", title: "Doctor ran", body: "No issues", kind: "doctor", status: "complete" } as never;

describe("DoctorTimelineCard", () => {
  it("renders singular evidence and copies the safe timeline", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    render(<DoctorTimelineCard events={[event]} />);
    expect(screen.getByText("1 event")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Copy timeline" }));
    expect(writeText).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Copied timeline." })).toBeVisible();
  });

  it("handles empty history and unavailable clipboard", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined });
    render(<DoctorTimelineCard events={[]} />);
    expect(screen.getByText("0 events")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Copy timeline" }));
    expect(screen.getByRole("button", { name: "Clipboard unavailable." })).toBeVisible();
  });
});
