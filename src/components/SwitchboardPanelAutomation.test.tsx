import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";

import { SwitchboardPanel } from "./SwitchboardPanel";

const baseProps: ComponentProps<typeof SwitchboardPanel> = {
  mode: "full",
  effectiveMode: "full",
  summary: "Full optimization is managing supported agents automatically.",
  localOnly: true,
  proxyStatus: "Running",
  headroomDetail: "Codex and Claude traffic is routed.",
  rtkStatus: "Enabled",
  rtkDetail: "82.5% token reduction",
  inspectorRows: [
    {
      label: "Codex routing",
      status: "Verified",
      detail: "Codex provider block is configured.",
    },
  ],
  remoteServicesEnabled: false,
  savingsMode: "balanced",
  savingsModeBusy: null,
  paused: false,
  resuming: false,
  modeBusy: null,
  modeError: null,
  onSetMode: vi.fn(),
  onSetSavingsMode: vi.fn(),
  onResume: vi.fn(),
  onManageClients: vi.fn(),
  onManageRtk: vi.fn(),
};

describe("SwitchboardPanel automation", () => {
  it("offers one-click setup repair and explains managed automation", async () => {
    const user = userEvent.setup();
    const onAutoFixSetup = vi.fn();

    render(
      <SwitchboardPanel
        {...baseProps}
        onAutoFixSetup={onAutoFixSetup}
      />,
    );

    expect(screen.getByText("One-click automation")).toBeInTheDocument();
    expect(
      screen.getByText(/Switchboard writes, verifies, backs up/),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Auto-fix setup" }));

    expect(onAutoFixSetup).toHaveBeenCalledTimes(1);
  });

  it("shows progress while auto-fix is running", () => {
    render(
      <SwitchboardPanel
        {...baseProps}
        autoFixBusy
        onAutoFixSetup={vi.fn()}
      />,
    );

    const autoFixButton = screen.getByRole("button", { name: "Auto-fixing" });
    expect(autoFixButton).toBeDisabled();
    expect(autoFixButton).toHaveAttribute("aria-disabled", "true");
    expect(autoFixButton).toHaveAttribute("aria-busy", "true");
  });
});
