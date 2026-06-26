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
      id: "headroom_runtime_unreachable",
      title: "Headroom runtime is not reachable",
      body: "Repair will restart the Headroom runtime.",
      severity: "error",
      repairAction: "repair_runtime",
    },
    {
      id: "codex_direct_bypass",
      title: "Codex is bypassing Headroom",
      body: "Compact the conversation context, then reset this bypass.",
      severity: "warning",
      repairAction: "reset_codex_bypass",
    },
    {
      id: "codex_provider_mismatch",
      title: "Codex routing config needs repair",
      body: "Repair will re-apply the reversible Codex setup.",
      severity: "warning",
      repairAction: "repair_codex_setup",
    },
    {
      id: "no_headroom_clients",
      title: "No clients are routed through Headroom",
      body: "Repair will re-apply reversible client setup.",
      severity: "warning",
      repairAction: "repair_client_setups",
    },
    {
      id: "rtk_not_active",
      title: "RTK is not active",
      body: "Repair will install RTK and enable local shell compression.",
      severity: "warning",
      repairAction: "repair_rtk_runtime",
    },
    {
      id: "rtk_integration_incomplete",
      title: "RTK integration is incomplete",
      body: "Repair will re-apply the local RTK integration.",
      severity: "warning",
      repairAction: "repair_rtk_integrations",
    },
  ],
};

describe("SwitchboardDoctorPanel", () => {
  it("hides when the report is healthy", () => {
    const { container } = render(
      <SwitchboardDoctorPanel
        report={{ status: "ok", summary: "No issues.", issues: [] }}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("shows successful repair message when report is healthy", () => {
    render(
      <SwitchboardDoctorPanel
        report={{ status: "ok", summary: "No issues.", issues: [] }}
        busyAction={null}
        error={null}
        successMessage="Repair complete. Switchboard looks ready."
        onRepair={vi.fn()}
      />,
    );

    expect(screen.getByRole("heading", { name: "Ready" })).toBeInTheDocument();
    expect(screen.getByLabelText("Switchboard Doctor")).toHaveClass(
      "switchboard-doctor--ok",
    );
    expect(
      screen.getByText("Repair complete. Switchboard looks ready."),
    ).toBeInTheDocument();
  });

  it("renders issues and runs repair actions", async () => {
    const user = userEvent.setup();
    const onRepair = vi.fn();
    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={onRepair}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "Needs attention" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Switchboard Doctor")).toHaveClass(
      "switchboard-doctor--warning",
    );
    expect(screen.getByText("6 automatic")).toBeInTheDocument();
    expect(screen.getByText("0 manual")).toBeInTheDocument();
    expect(
      screen.queryByText("Repair all will leave manual steps visible."),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Codex is bypassing Headroom")).toBeInTheDocument();
    expect(
      screen.getByText("Codex routing config needs repair"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Compact the Codex conversation, switch to RTK only for parallel heavy goals, then reset the Codex bypass when you want Headroom routing again.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Repair Codex setup to re-apply the managed provider block, then choose a Codex-supported ChatGPT model before retrying.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Installs or enables RTK in managed storage for local shell-output compression.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Restores RTK PATH and hook wiring without reinstalling the binary.",
      ),
    ).toBeInTheDocument();

    expect(
      screen.getByRole("button", { name: "Repair all" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Restart Headroom" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Repair clients" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Repair Codex" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Install RTK" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Repair RTK" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Reset Codex" }));
    expect(onRepair).toHaveBeenCalledWith("reset_codex_bypass");
    await user.click(screen.getByRole("button", { name: "Repair Codex" }));
    expect(onRepair).toHaveBeenCalledWith("repair_codex_setup");
    await user.click(screen.getByRole("button", { name: "Install RTK" }));
    expect(onRepair).toHaveBeenCalledWith("repair_rtk_runtime");
    await user.click(screen.getByRole("button", { name: "Repair all" }));
    expect(onRepair).toHaveBeenCalledWith("repair_all");
  });

  it("shows busy and error states", () => {
    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction="repair_all"
        error="Could not repair."
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Repairing all" }),
    ).toBeDisabled();
    expect(screen.getByText("Could not repair.")).toBeInTheDocument();
  });

  it("copies a shareable Doctor report", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(
      <SwitchboardDoctorPanel
        report={warningReport}
        busyAction={null}
        error={null}
        successMessage={null}
        onRepair={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Copy report" }));

    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain(
      "Mac AI Switchboard Doctor report",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "Action: automatic / Reset Codex",
    );
    expect(
      screen.getByRole("button", { name: "Copied report." }),
    ).toBeInTheDocument();
  });
});
