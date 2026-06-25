import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { DoctorReport } from "../lib/types";
import { SwitchboardDoctorPanel } from "./SwitchboardDoctorPanel";

const warningReport: DoctorReport = {
  status: "warning",
  summary: "Doctor found switchboard items that may need attention.",
  issues: [
    {
      id: "codex_direct_bypass",
      title: "Codex is bypassing Headroom",
      body: "Compact the conversation context, then reset this bypass.",
      severity: "warning",
      repairAction: "reset_codex_bypass"
    }
  ]
};

describe("SwitchboardDoctorPanel", () => {
  it("hides when the report is healthy", () => {
    const { container } = render(
      <SwitchboardDoctorPanel
        report={{ status: "ok", summary: "No issues.", issues: [] }}
        busyAction={null}
        error={null}
        onRepair={vi.fn()}
      />
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("renders issues and runs repair actions", async () => {
    const user = userEvent.setup();
    const onRepair = vi.fn();
    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction={null}
        error={null}
        onRepair={onRepair}
      />
    );

    expect(screen.getByRole("heading", { name: "Needs attention" })).toBeInTheDocument();
    expect(screen.getByText("Codex is bypassing Headroom")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Repair" }));
    expect(onRepair).toHaveBeenCalledWith("reset_codex_bypass");
  });

  it("shows busy and error states", () => {
    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction="reset_codex_bypass"
        error="Could not repair."
        onRepair={vi.fn()}
      />
    );

    expect(screen.getByRole("button", { name: "Repairing" })).toBeDisabled();
    expect(screen.getByText("Could not repair.")).toBeInTheDocument();
  });
});
