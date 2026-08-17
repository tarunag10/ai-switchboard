import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { mockDashboard } from "../lib/mockData";
import { HomeView, type HomeViewProps } from "./HomeView";

vi.mock("./MasterActivationCard", () => ({
  MasterActivationCard: ({ onActivateAll, onDeactivateAll, onActivateFeature, onDeactivateFeature, onOpenFeature, onActivateMaxCompression }: any) => <div>
    <button onClick={onActivateAll}>Activate everything mock</button>
    <button onClick={onDeactivateAll}>Deactivate everything mock</button>
    <button onClick={() => onActivateFeature("exactCache")}>Activate feature mock</button>
    <button onClick={() => onDeactivateFeature("rtk")}>Deactivate feature mock</button>
    <button onClick={() => onOpenFeature("doctor")}>Open feature mock</button>
    <button onClick={onActivateMaxCompression}>Max compression mock</button>
  </div>,
}));
vi.mock("./SwitchboardPanel", () => ({
  SwitchboardPanel: ({ onSetMode, onSetSavingsMode, onAutoFixSetup, onManageClients, onManageRtk, onResume, onOpenCompressionPlaybook }: any) => (
    <div>
      <button onClick={() => onSetMode("max_compression")}>Set max mode mock</button>
      <button onClick={() => onSetSavingsMode("measured")}>Set measured savings mock</button>
      <button onClick={onAutoFixSetup}>Auto fix mock</button>
      <button onClick={onManageClients}>Manage clients mock</button>
      <button onClick={onManageRtk}>Manage RTK mock</button>
      <button onClick={onResume}>Panel resume mock</button>
      <button onClick={onOpenCompressionPlaybook}>Compression playbook mock</button>
    </div>
  ),
}));
vi.mock("./SwitchboardDoctorPanel", () => ({ SwitchboardDoctorPanel: ({ onRepair }: any) => <button onClick={() => onRepair("repair_connector")}>Doctor repair mock</button> }));
vi.mock("./SavingsCalculatorCard", () => ({ SavingsCalculatorCard: () => null }));
vi.mock("./ClientSavingsTrendsCard", () => ({ ClientSavingsTrendsCard: () => null }));
vi.mock("./DailySavingsChart", () => ({ DailySavingsChart: () => null }));
vi.mock("./OutputReductionChip", () => ({ OutputReductionChip: () => null }));

function props(overrides: Partial<HomeViewProps> = {}): HomeViewProps {
  return {
    tierMismatch: null,
    upgradeActionError: null,
    upgradeActionBusy: null,
    handleUpgradeAction: vi.fn(),
    calloutBanner: { tone: "paused", title: "Paused" },
    calloutTitle: "Runtime paused",
    platformPreviewNotice: null,
    showRuntimeRestartAction: true,
    handleResumeRuntime: vi.fn(),
    resuming: false,
    resumeError: null,
    connectorPhase: "disabled",
    beginProxyVerificationStep: vi.fn(),
    connectors: [],
    pricingStatus: null,
    codexNudgeDismissed: false,
    localOnlyMode: true,
    connectorsBusy: false,
    toggleConnector: vi.fn(),
    dismissCodexNudge: vi.fn(),
    switchboardMode: "off",
    switchboardEffectiveMode: "off",
    switchboardNeedsAttention: false,
    switchboardModeCopy: "Off",
    switchboardLocalOnly: true,
    switchboardProxyStatus: "stopped",
    switchboardHeadroomLabel: "Stopped",
    switchboardRtkLabel: "Stopped",
    switchboardRtkDetail: "Not active",
    switchboardConnectors: [],
    dashboard: { ...mockDashboard },
    savingsMode: "estimated",
    savingsModeBusy: null,
    runtimeStatus: null,
    switchboardModeBusy: null,
    switchboardModeError: null,
    switchboardInspectorRows: [],
    switchboardRemoteServicesEnabled: false,
    handleSetSwitchboardMode: vi.fn(),
    handleSetSavingsMode: vi.fn(),
    setActiveView: vi.fn(),
    doctorReport: null,
    doctorRepairBusy: null,
    doctorRepairError: null,
    doctorRepairSuccess: null,
    managedFootprintReport: null,
    handleDoctorRepair: vi.fn(),
    chartMode: "usd",
    setChartMode: vi.fn(),
    setShowSavingsInfo: vi.fn(),
    savingsDashboard: { ...mockDashboard },
    savingsCalculatorRepoEstimate: null,
    activityFeed: { tiles: { rtkToday: null }, events: [] } as any,
    savingsAttributionEvents: [],
    cavemanSavingsEstimate: null,
    ponytailSavingsEstimate: null,
    markitdownSavingsEstimate: null,
    savingsCalculatorScope: "today",
    setSavingsCalculatorScope: vi.fn(),
    historyLoadTimedOut: true,
    chartResetSignal: 0,
    masterActivationState: "idle",
    masterActivationProgress: { completed: 0, total: 0 },
    masterFeatureStates: {},
    onActivateEverything: vi.fn(),
    onDeactivateEverything: vi.fn(),
    onActivateMasterFeature: vi.fn(),
    onDeactivateMasterFeature: vi.fn(),
    onOpenMasterFeature: vi.fn(),
    masterActivationIsActive: false,
    masterOperation: "activate",
    ...overrides,
  } as HomeViewProps;
}

describe("HomeView integration", () => {
  it("wires runtime, mode, repair, activation, and sector navigation actions", async () => {
    const user = userEvent.setup();
    const p = props();
    render(<HomeView {...p} />);

    await user.click(screen.getByRole("button", { name: "Resume" }));
    await user.click(screen.getByRole("button", { name: "Set max mode mock" }));
    await user.click(screen.getByRole("button", { name: "Auto fix mock" }));
    await user.click(screen.getByRole("button", { name: "Activate everything mock" }));
    await user.click(screen.getByRole("button", { name: /HealthDoctor/ }));

    expect(p.handleResumeRuntime).toHaveBeenCalledOnce();
    expect(p.handleSetSwitchboardMode).toHaveBeenCalledWith("max_compression");
    expect(p.handleDoctorRepair).toHaveBeenCalledWith("repair_all");
    expect(p.onActivateEverything).toHaveBeenCalledOnce();
    expect(p.setActiveView).toHaveBeenCalledWith("doctor");
  });

  it("shows failure and busy states and prevents a duplicate upgrade action", async () => {
    const user = userEvent.setup();
    const p = props({
      tierMismatch: {
        paidTier: "individual",
        recommendedTier: "pro",
        recommendedSource: "codex",
        clamped: true,
      } as any,
      upgradeActionError: "Checkout failed",
      upgradeActionBusy: "pro" as any,
      resuming: true,
      resumeError: "Runtime failed",
    });
    render(<HomeView {...p} />);

    expect(screen.getByText("Runtime failed")).toBeInTheDocument();
    const upgrade = screen.getByRole("button", { name: "Updating…" });
    expect(upgrade).toBeDisabled();
    await user.click(upgrade);
    expect(p.handleUpgradeAction).not.toHaveBeenCalled();
  });

  it("wires verification, savings controls, and every home sector", async () => {
    const user = userEvent.setup();
    const p = props({ calloutBanner: { tone: "starting", title: "Starting" }, connectorPhase: "verifying" });
    render(<HomeView {...p} />);
    await user.click(screen.getByRole("button", { name: "Test setup" }));
    await user.click(screen.getByRole("button", { name: "How savings are calculated" }));
    await user.click(screen.getByRole("button", { name: /All-time input tokens saved/ }));
    for (const name of [/Savings\$0/, /ToolsAdd-ons/, /LearnOptimization/]) {
      await user.click(screen.getByRole("button", { name }));
    }
    expect(p.beginProxyVerificationStep).toHaveBeenCalledOnce();
    expect(p.setShowSavingsInfo).toHaveBeenCalledWith(true);
    expect(p.setChartMode).toHaveBeenCalledWith("tokens");
    expect(p.setActiveView).toHaveBeenCalledWith("usage");
    expect(p.setActiveView).toHaveBeenCalledWith("addons");
    expect(p.setActiveView).toHaveBeenCalledWith("optimization");
  });

  it("offers and dismisses Codex automatic setup and reports upgrade errors", async () => {
    const user = userEvent.setup();
    const connector = { clientId: "codex", name: "Codex", installed: true, enabled: false, verified: false, supportStatus: "managed" } as any;
    const p = props({
      connectors: [connector], localOnlyMode: false, pricingStatus: { optimizationAllowed: true } as any,
      tierMismatch: { paidTier: "pro", recommendedTier: "max5x", recommendedSource: "codex", clamped: false } as any,
      upgradeActionError: "Upgrade unavailable",
    });
    render(<HomeView {...p} />);
    expect(screen.getByText("Upgrade unavailable")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Turn on Codex" }));
    await user.click(screen.getByRole("button", { name: "Dismiss Codex suggestion" }));
    await user.click(screen.getByRole("button", { name: /Upgrade to/ }));
    expect(p.toggleConnector).toHaveBeenCalledWith(expect.objectContaining({ clientId: "codex" }), true);
    expect(p.dismissCodexNudge).toHaveBeenCalledOnce();
    expect(p.handleUpgradeAction).toHaveBeenCalledWith("max5x");
  });

  it("forwards all master activation, panel, and doctor callbacks", async () => {
    const user = userEvent.setup();
    const p = props({ onActivateMaxCompression: vi.fn(), onOpenCompressionPlaybook: vi.fn() });
    render(<HomeView {...p} />);
    for (const name of [
      "Deactivate everything mock", "Activate feature mock", "Deactivate feature mock", "Open feature mock", "Max compression mock",
      "Set measured savings mock", "Manage clients mock", "Manage RTK mock", "Panel resume mock", "Compression playbook mock", "Doctor repair mock",
    ]) await user.click(screen.getByRole("button", { name }));
    expect(p.onDeactivateEverything).toHaveBeenCalledOnce();
    expect(p.onActivateMasterFeature).toHaveBeenCalledWith("exactCache");
    expect(p.onDeactivateMasterFeature).toHaveBeenCalledWith("rtk");
    expect(p.onOpenMasterFeature).toHaveBeenCalledWith("doctor");
    expect(p.onActivateMaxCompression).toHaveBeenCalledOnce();
    expect(p.handleSetSavingsMode).toHaveBeenCalledWith("measured");
    expect(p.setActiveView).toHaveBeenCalledWith("settings");
    expect(p.setActiveView).toHaveBeenCalledWith("addons");
    expect(p.handleResumeRuntime).toHaveBeenCalledOnce();
    expect(p.onOpenCompressionPlaybook).toHaveBeenCalledOnce();
    expect(p.handleDoctorRepair).toHaveBeenCalledWith("repair_connector");
  });
});
