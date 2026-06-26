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
expect(screen.getByLabelText("Full optimization local footprint")).toBeInTheDocument();
expect(screen.getByText("Client routing")).toBeInTheDocument();
expect(screen.getByText("Shell output")).toBeInTheDocument();
expect(screen.getByText("Repo packs")).toBeInTheDocument();
expect(screen.getByText("Managed through Headroom")).toBeInTheDocument();
expect(screen.getByText("RTK compacts noisy commands")).toBeInTheDocument();
expect(
screen.getByText(
        "Routes supported clients through Headroom and compresses shell output with RTK.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Running several Codex goals?")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Headroom compression is best for one main Codex session. Use RTK only before running several heavy active Codex chats or goals so large requests do not stall behind compression.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Codex parallel-session policy")).toBeInTheDocument();
    expect(screen.getByText("Full: one main Codex session")).toBeInTheDocument();
    expect(screen.getByText("RTK only: 2+ heavy sessions")).toBeInTheDocument();
    expect(
      screen.getByText("Unsupported model: Repair Codex setup"),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Codex multiple-goal prevention steps"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Switch to RTK only before opening several active Codex chats or goals.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "If Codex was bypassed after a 413 compression_refused error, run Doctor to reset the bypass.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "If Codex says the model is unsupported with a ChatGPT account, use Doctor's Repair Codex action instead.",
      ),
    ).toBeInTheDocument();
  expect(
    screen.getByRole("button", {
      name: /Full optimization: Routes supported clients through Headroom/,
    }).querySelector("svg"),
  ).not.toBeNull();
  expect(
    screen.getByRole("button", { name: "Switch to RTK only" }),
  ).toBeInTheDocument();
    expect(
      screen.getByText("No pricing, trial, Clarity, Sentry, or Aptabase calls."),
    ).toBeInTheDocument();
  });

  it("shows off mode safety notes for routing and local metadata", () => {
    renderPanel({
      mode: "off",
      summary: "No optimization layer active right now.",
      proxyStatus: "Stopped",
      headroomDetail: "Client traffic direct",
      rtkStatus: "Disabled",
      rtkDetail: "Shell output unchanged",
    });

    expect(screen.getByLabelText("Off safety notes")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Routing hooks and RTK shell integration are disabled for normal client behavior.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Repo Intelligence summaries remain local until cleared from Addons."),
    ).toBeInTheDocument();
  });

  it("hides Codex parallel-goal guidance outside Headroom routed Codex modes", () => {
    renderPanel({
      mode: "rtk",
      headroomDetail: "Codex, Claude Code",
    });

  expect(screen.queryByText("Running several Codex goals?")).not.toBeInTheDocument();
});

it("switches to RTK only from Codex parallel-goal guidance", async () => {
  const user = userEvent.setup();
  const onSetMode = vi.fn();
  renderPanel({ onSetMode });

  await user.click(screen.getByRole("button", { name: "Switch to RTK only" }));

  expect(onSetMode).toHaveBeenCalledWith("rtk");
});

it("disables Codex parallel-goal action while applying RTK mode", () => {
  renderPanel({ modeBusy: "rtk" });

  expect(screen.getByRole("button", { name: "Applying" })).toBeDisabled();
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
    expect(
      screen.getByText("Account features and optional remote telemetry are enabled."),
    ).toBeInTheDocument();
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
