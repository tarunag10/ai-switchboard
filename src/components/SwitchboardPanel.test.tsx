import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SwitchboardPanel } from "./SwitchboardPanel";

function renderPanel(
  overrides: Partial<React.ComponentProps<typeof SwitchboardPanel>> = {},
) {
  const props: React.ComponentProps<typeof SwitchboardPanel> = {
    mode: "full",
    summary:
      "Headroom proxy routing and RTK command compression are both active.",
    localOnly: true,
    proxyStatus: "Running",
    headroomDetail: "Codex, Claude Code",
    rtkStatus: "Enabled",
    rtkDetail: "82.5% average savings",
    remoteServicesEnabled: false,
    paused: false,
    resuming: false,
    modeBusy: null,
    modeError: null,
    onSetMode: vi.fn(),
    onResume: vi.fn(),
    onManageClients: vi.fn(),
    onManageRtk: vi.fn(),
    ...overrides,
  };
  return { ...render(<SwitchboardPanel {...props} />), props };
}

describe("SwitchboardPanel", () => {
  it("renders the current mode and local-only status", () => {
    renderPanel();

    expect(
      screen.getByRole("heading", { name: "Full optimization" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Local-only Mac setup")).toBeInTheDocument();
    expect(screen.getByText("Codex, Claude Code")).toBeInTheDocument();
    expect(screen.getByText("82.5% average savings")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Routes supported clients through Headroom and compresses shell output with RTK.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("No pricing, trial, Clarity, or Sentry calls"),
    ).toBeInTheDocument();
  });

  it("renders cloud availability when remote services are enabled", () => {
    renderPanel({
      localOnly: false,
      remoteServicesEnabled: true,
      mode: "headroom",
    });

    expect(
      screen.getByRole("heading", { name: "Headroom only" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Headroom cloud setup")).toBeInTheDocument();
    expect(screen.getByText("Account features enabled")).toBeInTheDocument();
  });

  it("shows and disables resume action while resuming", () => {
    renderPanel({ paused: true, resuming: true, proxyStatus: "Paused" });

    expect(screen.getByRole("button", { name: "Restarting…" })).toBeDisabled();
  });

  it("calls action handlers", async () => {
    const user = userEvent.setup();
    const onResume = vi.fn();
    const onManageClients = vi.fn();
    const onManageRtk = vi.fn();
    const onSetMode = vi.fn();
    renderPanel({
      paused: true,
      onResume,
      onManageClients,
      onManageRtk,
      onSetMode,
    });

    await user.click(screen.getByRole("button", { name: /RTK only:/ }));
    await user.click(screen.getByRole("button", { name: "Resume Headroom" }));
    await user.click(screen.getByRole("button", { name: "Manage clients" }));
    await user.click(screen.getByRole("button", { name: "Manage RTK" }));

    expect(onSetMode).toHaveBeenCalledWith("rtk");
    expect(onResume).toHaveBeenCalledOnce();
    expect(onManageClients).toHaveBeenCalledOnce();
    expect(onManageRtk).toHaveBeenCalledOnce();
  });

  it("shows busy and error states for mode changes", () => {
    renderPanel({
      modeBusy: "headroom",
      modeError: "Could not switch optimization mode.",
    });

    expect(screen.getByRole("button", { name: /Applying/ })).toBeDisabled();
    expect(
      screen.getByText("Could not switch optimization mode."),
    ).toBeInTheDocument();
  });

  it("shows the effective mode when the requested mode needs attention", () => {
    renderPanel({
      mode: "full",
      effectiveMode: "rtk",
      needsAttention: true,
      summary: "Full optimization requested, but RTK only is currently active.",
    });

    expect(
      screen.getByRole("heading", { name: "Full optimization" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Active now: RTK only. Connect a supported client or repair Headroom routing in Doctor.")).toBeInTheDocument();
  });
});
