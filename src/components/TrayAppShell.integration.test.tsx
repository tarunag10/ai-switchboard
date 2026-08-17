import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { mockDashboard } from "../lib/mockData";
import { TrayAppShell } from "./TrayAppShell";

vi.mock("./TraySidebar", () => ({ TraySidebar: ({ onSelectView }: any) => <button onClick={() => onSelectView("doctor")}>Doctor navigation mock</button> }));
vi.mock("./HomeView", () => ({ HomeView: ({ handleResumeRuntime }: any) => <button onClick={handleResumeRuntime}>Resume home mock</button> }));
vi.mock("./DoctorView", () => ({ DoctorView: ({ onRepair }: any) => <button onClick={() => onRepair("repair_all")}>Repair doctor mock</button> }));
vi.mock("./ActivityFeed", () => ({ ActivityFeed: ({ onNavigateToOptimize }: any) => <button onClick={onNavigateToOptimize}>Optimize event mock</button> }));
vi.mock("./RepoMapView", () => ({ RepoMapView: ({ onOpenDoctor }: any) => <button onClick={onOpenDoctor}>Repo doctor mock</button> }));
vi.mock("./DailyUsageBriefingView", () => ({ DailyUsageBriefingView: ({ onNavigate }: any) => <button onClick={() => onNavigate("usage")}>Briefing usage mock</button> }));
vi.mock("./SavingsInfoDialog", () => ({ SavingsInfoDialog: ({ onClose }: any) => <button onClick={onClose}>Close savings mock</button> }));
vi.mock("./AddonsView", () => ({ AddonsView: () => null }));
vi.mock("./AgentMemoryInspector", () => ({ AgentMemoryInspector: () => null }));
vi.mock("./OptimizationView", () => ({ OptimizationView: () => null }));
vi.mock("./RepoIntelligencePreview", () => ({ RepoIntelligencePreview: () => null, repoIntelligencePreview: {} }));
vi.mock("./RoutingModelsView", () => ({ RoutingModelsView: () => null }));
vi.mock("./TokenXrayView", () => ({ TokenXrayView: () => null }));
vi.mock("./UpgradeView", () => ({ UpgradeView: () => null }));
vi.mock("./UsageSavingsView", () => ({ UsageSavingsView: () => null }));

function shellProps(overrides: Record<string, unknown> = {}) {
  return {
    upgradeOverlay: null, settingsView: null, pricingAuthCard: <div>Auth</div>, activeView: "home", setActiveView: vi.fn(), localOnlyMode: true,
    tierMismatch: null, upgradeActionError: null, upgradeActionBusy: null, handleUpgradeAction: vi.fn(), calloutBanner: { tone: "healthy", title: "Healthy" },
    calloutTitle: "Healthy", platformPreviewNotice: null, showRuntimeRestartAction: false, handleResumeRuntime: vi.fn(), resuming: false, resumeError: null,
    connectorPhase: "healthy", beginProxyVerificationStep: vi.fn(), connectors: [], pricingStatus: null, codexNudgeDismissed: false, connectorsBusy: false,
    toggleConnector: vi.fn(), dismissCodexNudge: vi.fn(), switchboardMode: "off", switchboardEffectiveMode: "off", switchboardNeedsAttention: false,
    switchboardModeCopy: "Off", switchboardLocalOnly: true, switchboardProxyStatus: "stopped", switchboardHeadroomLabel: "Stopped", switchboardRtkLabel: "Stopped",
    switchboardRtkDetail: "Stopped", switchboardConnectors: [], dashboard: mockDashboard, savingsMode: "estimated", savingsModeBusy: null, runtimeStatus: null,
    switchboardModeBusy: null, switchboardModeError: null, switchboardInspectorRows: [], switchboardRemoteServicesEnabled: false, handleSetSwitchboardMode: vi.fn(),
    handleSetSavingsMode: vi.fn(), doctorReport: null, doctorRepairBusy: null, doctorRepairError: null, doctorRepairSuccess: null, managedFootprintReport: null,
    handleDoctorRepair: vi.fn(), chartMode: "usd", setChartMode: vi.fn(), setShowSavingsInfo: vi.fn(), savingsDashboard: mockDashboard,
    savingsCalculatorRepoEstimate: { estimatedTokensSaved: 0, estimatedSavingsUsd: 0 } as any, activityFeed: { tiles: { rtkToday: null }, events: [] },
    savingsAttributionEvents: [], cavemanSavingsEstimate: null, ponytailSavingsEstimate: null, markitdownSavingsEstimate: null, savingsCalculatorScope: "today",
    setSavingsCalculatorScope: vi.fn(), historyLoadTimedOut: true, chartResetSignal: 0, masterActivationState: "idle", masterActivationProgress: { completed: 0, total: 0 },
    masterFeatureStates: {}, onActivateEverything: vi.fn(), onDeactivateEverything: vi.fn(), onActivateMasterFeature: vi.fn(), onDeactivateMasterFeature: vi.fn(),
    onOpenMasterFeature: vi.fn(), masterActivationIsActive: false, masterOperation: "activate", headroomLearnSupported: false, headroomLearnDisabledReason: null,
    headroomLearnPrereq: null, headroomLearnStatus: null, headroomLearnBusy: false, claudeLearnEnabled: false, codexLearnEnabled: false, claudeProjectsBusy: false,
    claudeProjects: [], visibleClaudeProjects: [], sortedClaudeProjects: [], showAllClaudeProjects: false, setShowAllClaudeProjects: vi.fn(), handleRunHeadroomLearn: vi.fn(),
    copyLearnInstallCommand: vi.fn(), openLearnInstallDocsLink: vi.fn(), refreshHeadroomLearnPrereq: vi.fn(), learnInstallCopyNotice: null, optimizeAppliedByProject: null,
    setOptimizeAppliedRefreshTick: vi.fn(), claudeProjectsError: null, learnBlurb: "", activityFeedError: null, activityFeedLoaded: true,
    setLatestRepoIntelligenceSummary: vi.fn(), addonError: null, addonCopy: {}, addonInfoId: null, setAddonInfoId: vi.fn(), addonBusyId: null, addonBusyLabel: null,
    addonResult: null, setAddonResult: vi.fn(), rtkAvgSavingsPct: null, rtkBusy: false, openExternalLink: vi.fn(), runAddonAction: vi.fn(),
    onMeasuredAddonSavingsRecorded: vi.fn(), handleRtkToggle: vi.fn(), setCavemanLevel: vi.fn(), copyPlannedConnectorCommand: vi.fn(), pricingAudience: "individual",
    setPricingAudience: vi.fn(), setUpgradeActionError: vi.fn(), billingPeriod: "annual", setBillingPeriod: vi.fn(), upgradeTrialCallout: { tone: "info", message: "" },
    authRequestBusy: false, authVerifyBusy: false, upgradePlansState: { featuredPlanId: "pro", plans: [] }, visibleUpgradePlans: [], activeHeadroomPlanId: null,
    handleContactSubmit: vi.fn(), contactEmail: "", setContactEmail: vi.fn(), contactSubmitError: null, setContactSubmitError: vi.fn(), contactSubmitSuccess: null,
    setContactSubmitSuccess: vi.fn(), contactMessage: "", setContactMessage: vi.fn(), contactEmailValid: false, contactSubmitBusy: false,
    handleReactivateSubscription: vi.fn(), reactivateBusy: false, hasHiddenUpgradePlans: false, showAllUpgradePlans: false, setShowAllUpgradePlans: vi.fn(), reactivateError: null,
    showSavingsInfo: false, showUninstallDialog: false, setShowUninstallDialog: vi.fn(), uninstallBusy: false, uninstallDisclosureTitle: "Uninstall",
    uninstallDisclosureItems: [], uninstallDisclosureFooter: "Done", uninstallCopyNotice: null, uninstallError: null, copyUninstallDryRunReport: vi.fn(), handleUninstall: vi.fn(),
    pendingPlanChange: null, cancelPlanChange: vi.fn(), confirmPlanChange: vi.fn(), planChangeError: null, planChangeBusy: false, showAppUpdateDialog: false,
    setShowAppUpdateDialog: vi.fn(), appUpdateAvailable: null, appUpdateReadyToRestart: false, appUpdateInstallBusy: false, restartIntoInstalledUpdate: vi.fn(),
    installAvailableUpdate: vi.fn(), ...overrides,
  } as any;
}

describe("TrayAppShell integration", () => {
  it("connects navigation and child actions to shell handlers", async () => {
    const user = userEvent.setup();
    const p = shellProps();
    render(<TrayAppShell {...p} />);
    await user.click(screen.getByRole("button", { name: "Doctor navigation mock" }));
    await user.click(screen.getByRole("button", { name: "Resume home mock" }));
    await user.click(screen.getByRole("button", { name: "Repair doctor mock" }));
    await user.click(screen.getByText("Optimize event mock"));
    expect(p.setActiveView).toHaveBeenNthCalledWith(1, "doctor");
    expect(p.handleResumeRuntime).toHaveBeenCalledOnce();
    expect(p.handleDoctorRepair).toHaveBeenCalledWith("repair_all");
    expect(p.setActiveView).toHaveBeenNthCalledWith(2, "optimization");
  });

  it("wires confirmation modal actions and disables destructive controls while busy", async () => {
    const user = userEvent.setup();
    const p = shellProps({ showUninstallDialog: true });
    const { rerender } = render(<TrayAppShell {...p} />);
    await user.click(screen.getByRole("button", { name: "Copy dry-run" }));
    await user.click(screen.getByRole("button", { name: "Uninstall and quit" }));
    expect(p.copyUninstallDryRunReport).toHaveBeenCalledOnce();
    expect(p.handleUninstall).toHaveBeenCalledOnce();

    rerender(<TrayAppShell {...shellProps({ showUninstallDialog: true, uninstallBusy: true })} />);
    expect(screen.getByRole("button", { name: "Uninstalling…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Copy dry-run" })).toBeDisabled();
  });

  it("closes savings, returns from auth, and cancels an uninstall dialog", async () => {
    const user = userEvent.setup();
    const p = shellProps({ activeView: "upgradeAuth", showSavingsInfo: true, showUninstallDialog: true });
    render(<TrayAppShell {...p} />);
    await user.click(screen.getByRole("button", { name: "Close savings mock" }));
    await user.click(screen.getByRole("button", { name: "Back to upgrade plans" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(p.setShowSavingsInfo).toHaveBeenCalledWith(false);
    expect(p.setActiveView).toHaveBeenCalledWith("upgrade");
    expect(p.setShowUninstallDialog).toHaveBeenCalledWith(false);
  });

  it("confirms upgrade and downgrade plan changes and exposes busy/error states", async () => {
    const user = userEvent.setup();
    const upgrade = shellProps({
      pendingPlanChange: { fromTier: "pro", toTier: "max5x", billingPeriod: "monthly" },
      planChangeError: "Plan service unavailable",
    });
    const view = render(<TrayAppShell {...upgrade} />);
    expect(screen.getByText("Plan service unavailable")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Confirm upgrade" }));
    expect(upgrade.confirmPlanChange).toHaveBeenCalledOnce();

    const downgrade = shellProps({ pendingPlanChange: { fromTier: "max5x", toTier: "pro", billingPeriod: "annual" }, planChangeBusy: true });
    view.rerender(<TrayAppShell {...downgrade} />);
    expect(screen.getByRole("button", { name: "Downgrading…" })).toBeDisabled();
    expect(screen.getByText(/prorated credit/)).toBeInTheDocument();
  });

  it("installs, defers, and restarts application updates through the modal", async () => {
    const user = userEvent.setup();
    const available = { version: "2.0.0", currentVersion: "1.0.0", publishedAt: "2026-08-17T00:00:00Z", notes: "Safer routing" };
    const install = shellProps({ showAppUpdateDialog: true, appUpdateAvailable: available });
    const view = render(<TrayAppShell {...install} />);
    await user.click(screen.getByRole("button", { name: "Install 2.0.0" }));
    expect(install.installAvailableUpdate).toHaveBeenCalledOnce();
    await user.click(screen.getByRole("button", { name: "Later" }));
    expect(install.setShowAppUpdateDialog).toHaveBeenCalledWith(false);

    const restart = shellProps({ showAppUpdateDialog: true, appUpdateAvailable: available, appUpdateReadyToRestart: true });
    view.rerender(<TrayAppShell {...restart} />);
    await user.click(screen.getByRole("button", { name: "Restart now" }));
    expect(restart.restartIntoInstalledUpdate).toHaveBeenCalledOnce();
  });
});
