import { render, screen, within } from "@testing-library/react";
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
    inspectorRows: [
      {
        label: "Proxy listener",
        status: "Reachable",
        detail:
          "127.0.0.1:6767 is accepting loopback traffic. The listener is local-only.",
      },
      {
        label: "Backend port",
        status: "Reachable",
        detail:
          "127.0.0.1:6768 is the default internal Headroom backend port.",
      },
      {
        label: "Codex routing",
        status: "Verified",
        detail: "Codex is routed through Headroom and verified.",
      },
      {
        label: "Claude routing",
        status: "Needs test",
        detail:
          "Claude Code routing is configured; send a test prompt from Connectors.",
        actionLabel: "Open Connectors",
        onAction: vi.fn(),
      },
      {
        label: "Client routing",
        status: "Managed",
        detail: "Codex, Claude Code",
      },
      {
        label: "Managed shell blocks",
        status: "Verified",
        detail: "Connector verification found managed shell routing blocks.",
      },
      {
        label: "Codex provider block",
        status: "Verified",
        detail:
          "Connector verification found the Headroom-managed provider block in ~/.codex/config.toml.",
      },
      {
        label: "Shell export",
        status: "Configured",
        detail: "Managed RTK PATH export is present.",
      },
      {
        label: "RTK shell hook",
        status: "Configured",
        detail: "Managed RTK command-rewrite hook is present.",
      },
      {
        label: "Headroom MCP",
        status: "Configured",
        detail: "Claude MCP config includes the local Headroom server.",
      },
      {
        label: "Repo Memory MCP",
        status: "Configured",
        detail:
          "Repo Memory MCP is app-managed, read-only, and available to supported agents.",
      },
      {
        label: "Launch at login",
        status: "Loaded",
        detail:
          "Launch at login plist exists at ~/Library/LaunchAgents/com.tarunagarwal.mac-ai-switchboard.plist. launchctl reports gui/501/com.tarunagarwal.mac-ai-switchboard is loaded.",
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
    expect(screen.getByLabelText("Mode Inspector")).toBeInTheDocument();
    const inspector = within(screen.getByLabelText("Mode Inspector"));
    expect(inspector.getByText("Requested")).toBeInTheDocument();
    expect(inspector.getByText("Active")).toBeInTheDocument();
    expect(inspector.getByText("Headroom engine")).toBeInTheDocument();
    expect(inspector.getByText("RTK hook")).toBeInTheDocument();
    expect(inspector.queryByText("Stale shells")).not.toBeInTheDocument();
    expect(inspector.getByText("Proxy listener")).toBeInTheDocument();
    expect(inspector.getByText("Backend port")).toBeInTheDocument();
    expect(inspector.getByText("Codex routing")).toBeInTheDocument();
    expect(inspector.getByText("Claude routing")).toBeInTheDocument();
    expect(inspector.getByText("Client routing")).toBeInTheDocument();
    expect(inspector.getByText("Managed shell blocks")).toBeInTheDocument();
    expect(inspector.getByText("Codex provider block")).toBeInTheDocument();
    expect(inspector.getByText("Shell export")).toBeInTheDocument();
    expect(inspector.getByText("RTK shell hook")).toBeInTheDocument();
    expect(inspector.getByText("Headroom MCP")).toBeInTheDocument();
    expect(inspector.getByText("Repo Memory MCP")).toBeInTheDocument();
    expect(inspector.getByText("Launch at login")).toBeInTheDocument();
    expect(screen.getAllByText("Codex, Claude Code").length).toBeGreaterThan(0);
    expect(screen.getAllByText("82.5% average savings").length).toBeGreaterThan(
      0,
    );
    expect(
      inspector.getByText("Managed RTK PATH export is present."),
    ).toBeInTheDocument();
    expect(
      inspector.getByText("Managed RTK command-rewrite hook is present."),
    ).toBeInTheDocument();
    expect(
      inspector.getByText("Claude MCP config includes the local Headroom server."),
    ).toBeInTheDocument();
    expect(
      inspector.getByText("Connector verification found managed shell routing blocks."),
    ).toBeInTheDocument();
    expect(
      inspector.getByText(
        "Connector verification found the Headroom-managed provider block in ~/.codex/config.toml.",
      ),
    ).toBeInTheDocument();
    expect(
      inspector.getByText(
        "127.0.0.1:6767 is accepting loopback traffic. The listener is local-only.",
      ),
    ).toBeInTheDocument();
    expect(
      inspector.getByText(
        "127.0.0.1:6768 is the default internal Headroom backend port.",
      ),
    ).toBeInTheDocument();
    expect(
      inspector.getByText(
        "Repo Memory MCP is app-managed, read-only, and available to supported agents.",
      ),
    ).toBeInTheDocument();
    expect(
      inspector.getByText(
        "Launch at login plist exists at ~/Library/LaunchAgents/com.tarunagarwal.mac-ai-switchboard.plist. launchctl reports gui/501/com.tarunagarwal.mac-ai-switchboard is loaded.",
      ),
    ).toBeInTheDocument();
    expect(
      inspector.getByText("Codex is routed through Headroom and verified."),
    ).toBeInTheDocument();
    expect(
      inspector.getByText(
        "Claude Code routing is configured; send a test prompt from Connectors.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Savings profile")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Balanced" })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Aggressive" }),
    ).toBeInTheDocument();
    const footprint = within(
      screen.getByLabelText("Full optimization local footprint"),
    );
    expect(footprint.getByText("Client routing")).toBeInTheDocument();
    expect(footprint.getByText("Shell output")).toBeInTheDocument();
    expect(footprint.getByText("Repo packs")).toBeInTheDocument();
    expect(screen.getByText("Managed through Headroom")).toBeInTheDocument();
    expect(screen.getByText("RTK compacts noisy commands")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Routes supported clients through Headroom and compresses shell output with RTK.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Running several Codex goals?"),
    ).toBeInTheDocument();
    expect(screen.getByText("Preventive guidance")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Headroom compression is best for one main Codex session. Use RTK only before running several heavy active Codex chats or goals so large requests do not stall behind compression.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Codex context pressure evidence"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("No recent Codex token events in this app session yet."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("No saved local token history is available yet."),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Codex parallel-session policy"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Full: one main Codex session"),
    ).toBeInTheDocument();
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
      screen
        .getByRole("button", {
          name: /Full optimization: Routes supported clients through Headroom/,
        })
        .querySelector("svg"),
    ).not.toBeNull();
    expect(
      screen.getByRole("button", { name: "Switch to RTK only" }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/endpoints stay paused/)[0]).toBeInTheDocument();
  });

  it("runs optional Mode Inspector row actions", async () => {
    const user = userEvent.setup();
    const onAction = vi.fn();
    renderPanel({
      inspectorRows: [
        {
          label: "Repo Memory MCP",
          status: "Needs attention",
          detail: "Repo Memory MCP is not configured.",
          actionLabel: "Prepare MCP",
          onAction,
        },
      ],
    });

    await user.click(screen.getByRole("button", { name: "Prepare MCP" }));

    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it("renders individual managed connector routing repair rows", async () => {
    const user = userEvent.setup();
    const onAction = vi.fn();
    renderPanel({
      inspectorRows: [
        {
          label: "Gemini CLI routing",
          status: "Repair ready",
          detail:
            "Gemini CLI routing is repair ready. Use Repair managed setup to re-apply reversible managed setup and verify routing evidence.",
          actionLabel: "Repair managed setup",
          onAction,
        },
      ],
    });

    const inspector = within(screen.getByLabelText("Mode Inspector"));
    expect(inspector.getByText("Gemini CLI routing")).toBeInTheDocument();
    expect(
      inspector.getByText(
        "Gemini CLI routing is repair ready. Use Repair managed setup to re-apply reversible managed setup and verify routing evidence.",
      ),
    ).toBeInTheDocument();

    await user.click(
      inspector.getByRole("button", { name: "Repair managed setup" }),
    );

    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it("points direct Codex provider rows back to repair-ready routing", () => {
    renderPanel({
      inspectorRows: [
        {
          label: "Codex provider block",
          status: "Direct",
          detail:
            "Codex provider routing is repair ready. Use the Codex routing repair-ready row to re-apply the managed provider block.",
        },
      ],
    });

    const inspector = within(screen.getByLabelText("Mode Inspector"));
    expect(
      inspector.getByText(
        "Codex provider routing is repair ready. Use the Codex routing repair-ready row to re-apply the managed provider block.",
      ),
    ).toBeInTheDocument();
  });

  it("disables busy Mode Inspector row actions", () => {
    const onAction = vi.fn();
    renderPanel({
      inspectorRows: [
        {
          label: "Repo Memory MCP",
          status: "Unknown",
          detail: "Repo Memory MCP lifecycle has not been verified.",
          actionLabel: "Prepare MCP",
          actionBusyLabel: "Preparing Repo Memory MCP...",
          actionDisabled: true,
          onAction,
        },
      ],
    });

    expect(
      screen.getByRole("button", { name: "Preparing Repo Memory MCP..." }),
    ).toBeDisabled();
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
      screen.getByText(
        "Repo Intelligence summaries remain local until cleared from Addons.",
      ),
    ).toBeInTheDocument();
  });

  it("surfaces stale shell restart guidance when mode evidence needs attention", () => {
    renderPanel({
      mode: "off",
      effectiveMode: "full",
      needsAttention: true,
      summary: "Off requested, but routing evidence is still active.",
      proxyStatus: "Running",
      headroomDetail: "Headroom engine is still reachable",
      rtkStatus: "Enabled",
      rtkDetail: "Old shell hook still active",
    });

    const inspector = within(screen.getByLabelText("Mode Inspector"));
    expect(inspector.getByText("Stale shells")).toBeInTheDocument();
    expect(inspector.getByText("Restart shells")).toBeInTheDocument();
    expect(
      inspector.getByText(/ANTHROPIC_BASE_URL, OPENAI_BASE_URL, or PATH/),
    ).toBeInTheDocument();
  });

  it("hides Codex parallel-goal guidance outside Headroom routed Codex modes", () => {
    renderPanel({
      mode: "rtk",
      headroomDetail: "Codex, Claude Code",
    });

    expect(
      screen.queryByText("Running several Codex goals?"),
    ).not.toBeInTheDocument();
  });

  it("renders high context-pressure evidence from recent Codex usage", () => {
    renderPanel({
      recentUsage: [
        {
          id: "codex-large",
          timestamp: "2026-06-30T10:00:00Z",
          client: "Codex",
          workspace: "repo",
          upstreamTarget: "openai",
          stages: [],
          estimatedInputTokens: 125_000,
          estimatedOutputTokens: 6_000,
          estimatedCostSavingsUsd: 0,
          latencyMs: 1_200,
          outcome: "success",
        },
      ],
    });

    expect(screen.getByText("High context pressure")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Codex or saved local token history is large enough that Headroom compression can stall. Compact the largest conversation or switch to RTK only before opening more heavy Codex work.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("1 recent Codex request.")).toBeInTheDocument();
    expect(
      screen.getByText("Largest recent Codex request: 131,000 tokens."),
    ).toBeInTheDocument();
  });

  it("renders high context-pressure evidence from saved local history", () => {
    renderPanel({
      recentUsage: [],
      savedHistory: [
        {
          date: "2026-06-29",
          estimatedSavingsUsd: 0.7,
          estimatedTokensSaved: 70_000,
          actualCostUsd: 2.0,
          totalTokensSent: 320_000,
        },
      ],
    });

    expect(screen.getByText("High context pressure")).toBeInTheDocument();
    expect(
      screen.getByText("1 saved local history day with token traffic."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Largest saved history day: 320,000 tokens sent."),
    ).toBeInTheDocument();
  });

  it("switches to RTK only from Codex parallel-goal guidance", async () => {
    const user = userEvent.setup();
    const onSetMode = vi.fn();
    renderPanel({ onSetMode });

    await user.click(
      screen.getByRole("button", { name: "Switch to RTK only" }),
    );

    expect(onSetMode).toHaveBeenCalledWith("rtk");
  });

  it("switches the savings profile", async () => {
    const user = userEvent.setup();
    const onSetSavingsMode = vi.fn();
    renderPanel({ onSetSavingsMode });

    await user.click(screen.getByRole("button", { name: "Aggressive" }));

    expect(onSetSavingsMode).toHaveBeenCalledWith("aggressive");
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
    expect(screen.getByText("Mac AI Switchboard cloud setup")).toBeInTheDocument();
    expect(
      screen.getAllByText(
        /Update, support, and optional telemetry destinations are enabled/,
      )[0],
    ).toBeInTheDocument();
  });

  it("shows and disables resume action while resuming", () => {
    renderPanel({ paused: true, resuming: true, proxyStatus: "Paused" });

    expect(screen.getByRole("button", { name: "Restarting…" })).toBeDisabled();
  });

  it("offers a primary start runtime action when the engine is offline", async () => {
    const user = userEvent.setup();
    const onResume = vi.fn();
    renderPanel({
      proxyStatus: "Offline",
      headroomDetail: "127.0.0.1:6767 is not accepting traffic.",
      runtimeActionVisible: true,
      runtimeActionLabel: "Start runtime",
      onResume,
    });

    await user.click(screen.getByRole("button", { name: "Start runtime" }));

    expect(onResume).toHaveBeenCalledOnce();
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
    await user.click(screen.getByRole("button", { name: "Resume runtime" }));
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
    expect(
      screen.getByText(
        "Active now: RTK only. Connect a supported client or repair Headroom routing in Doctor.",
      ),
    ).toBeInTheDocument();
  });

  it("copies a shareable switchboard state", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    renderPanel({
      mode: "full",
      effectiveMode: "rtk",
      needsAttention: true,
      summary: "Full optimization requested, RTK only active.",
    });

    await user.click(screen.getByRole("button", { name: "Copy state" }));

    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain(
      "Mac AI Switchboard mode state",
    );
    expect(writeText.mock.calls[0][0]).toContain(
      "Requested mode: Full optimization",
    );
    expect(writeText.mock.calls[0][0]).toContain("Active mode: RTK only");
    expect(writeText.mock.calls[0][0]).toContain("Client routing: on");
    expect(
      screen.getByRole("button", { name: "Copied state." }),
    ).toBeInTheDocument();
  });
});
