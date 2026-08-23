import { createRef } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { mockDashboard } from "../lib/mockData";
import { SettingsView, type SettingsViewProps } from "./SettingsView";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("./SettingsLegalPanel", () => ({ SettingsLegalPanel: () => null }));
vi.mock("./ProxySessionAuthCard", () => ({ ProxySessionAuthCard: () => null }));
vi.mock("./RollbackCenter", () => ({ RollbackCenter: () => null }));
vi.mock("./SettingsTransferCard", () => ({
  SettingsTransferCard: ({ onCopyExport, onImportTextChange, onPreviewImport, onApplyImport }: any) => <div>
    <button onClick={onCopyExport}>Export mock</button>
    <button onClick={() => onImportTextChange("payload")}>Import text mock</button>
    <button onClick={onPreviewImport}>Preview mock</button>
    <button onClick={onApplyImport}>Apply mock</button>
  </div>,
}));
vi.mock("./SettingsConnectorPanel", () => ({ SettingsConnectorPanel: ({ toggleConnector, copyPlannedConnectorCommand }: any) => <div>
  <button onClick={() => toggleConnector({ clientId: "codex" }, true)}>Toggle connector mock</button>
  <button onClick={() => copyPlannedConnectorCommand("plan", "Codex")}>Copy connector mock</button>
</div> }));
vi.mock("./SettingsReleaseReadinessCard", () => ({ SettingsReleaseReadinessCard: ({ copyReleaseReadinessReport, refreshReleaseReadinessReport, runReleaseEvidenceCommand, runLocalReleaseEvidenceSequence }: any) => <div>
  <button onClick={copyReleaseReadinessReport}>Copy release mock</button>
  <button onClick={refreshReleaseReadinessReport}>Refresh release mock</button>
  <button onClick={() => runReleaseEvidenceCommand("signed-app")}>Evidence command mock</button>
  <button onClick={runLocalReleaseEvidenceSequence}>Evidence sequence mock</button>
</div> }));
vi.mock("./SettingsOpenLoginCard", () => ({ SettingsOpenLoginCard: ({ onToggle }: any) => <button onClick={() => onToggle(true)}>Autostart mock</button> }));
vi.mock("./SettingsUninstallCard", () => ({
  SettingsUninstallCard: ({ onOpenUninstallDialog }: any) => <button onClick={onOpenUninstallDialog}>Uninstall mock</button>,
}));
vi.mock("./SettingsRuntimeStatusCard", () => ({
  SettingsRuntimeStatusCard: ({ onOpenHeadroomDashboard, onToggleHeadroomDetails, checkForAppUpdate }: any) => <div>
    <button onClick={onOpenHeadroomDashboard}>Dashboard mock</button>
    <button onClick={onToggleHeadroomDetails}>Logs mock</button>
    <button onClick={checkForAppUpdate}>Update check mock</button>
  </div>,
}));
vi.mock("./SettingsHeadroomAdvancedCard", () => ({
  SettingsHeadroomAdvancedCard: () => <article>Advanced Headroom settings mock</article>,
}));

function props(overrides: Partial<SettingsViewProps> = {}): SettingsViewProps {
  return {
    hidden: false, readinessSignals: [], dashboard: mockDashboard,
    switchboardMode: "off", savingsMode: "estimated", connectors: [], appSemver: "1.0.0",
    settingsTransferNotice: null, setSettingsImportText: vi.fn(), setSettingsImportPreview: vi.fn(),
    setSettingsTransferNotice: vi.fn(), settingsImportText: "", settingsImportPreview: null,
    settingsImportBusy: false, copySettingsExport: vi.fn(), previewSettingsImport: vi.fn(), applySettingsImport: vi.fn(),
    plannedConnectorReadiness: { headline: "Ready", detail: "", detectedCount: 0, manualOnlyCount: 0, notDetectedCount: 0, safeTodayCount: 0, automationGateCount: 0 },
    plannedConnectorCopyNotice: null, connectorsBusy: false, connectorsError: null, openConnectorHelpId: null,
    setOpenConnectorHelpId: vi.fn(), toggleConnector: vi.fn(), verifyConnectors: vi.fn(), copyPlannedConnectorCommand: vi.fn(),
    autostartEnabled: false, autostartBusy: false, handleAutostartToggle: vi.fn(),
    showHeadroomDetails: false, setShowHeadroomDetails: vi.fn(), setHeadroomLogLines: vi.fn(), headroomLogLines: [],
    headroomLogRef: createRef<HTMLPreElement>(), headroomVersion: "1.0", headroomLifetimeSavingsPct: null,
    runtimeStatus: null, kompressWarming: false, appUpdateConfig: null, appUpdateBusy: false,
    appUpdateInstallBusy: false, appUpdateStatusCopy: null, checkForAppUpdate: vi.fn(),
    releaseReadinessRefreshing: false, releaseEvidenceBusyId: null, releaseEvidenceResult: null,
    releaseReadinessCommand: "check", releaseReadinessReport: null, releaseReadinessEvidence: { copy: "" },
    releaseReadinessAction: null, releaseReadinessError: null, releaseReadinessCounts: { ready: 0, blocked: 0, "local-only": 0 },
    releaseReadinessRows: [], releaseLocalEvidenceRows: [], releaseReadinessCopyNotice: null,
    copyReleaseReadinessReport: vi.fn(), refreshReleaseReadinessReport: vi.fn(), runReleaseEvidenceCommand: vi.fn(),
    runLocalReleaseEvidenceSequence: vi.fn(), formatLocalReleaseEvidenceSequenceCopy: vi.fn(() => ""),
    setUninstallError: vi.fn(), setShowUninstallDialog: vi.fn(), SUPPORT_ISSUES_URL: "https://example.test/issues",
    ...overrides,
  } as SettingsViewProps;
}

describe("SettingsView integration", () => {
  beforeEach(() => invokeMock.mockReset());

  it("wires transfer and uninstall controls", async () => {
    const user = userEvent.setup();
    const p = props();
    render(<SettingsView {...p} />);
    await user.click(screen.getByRole("button", { name: "Export mock" }));
    await user.click(screen.getByRole("button", { name: "Import text mock" }));
    await user.click(screen.getByRole("button", { name: "Preview mock" }));
    await user.click(screen.getByRole("button", { name: "Apply mock" }));
    await user.click(screen.getByRole("button", { name: "Uninstall mock" }));
    expect(p.copySettingsExport).toHaveBeenCalledOnce();
    expect(p.setSettingsImportText).toHaveBeenCalledWith("payload");
    expect(p.setSettingsImportPreview).toHaveBeenCalledWith(null);
    expect(p.setSettingsTransferNotice).toHaveBeenCalledWith(null);
    expect(p.previewSettingsImport).toHaveBeenCalledOnce();
    expect(p.applySettingsImport).toHaveBeenCalledOnce();
    expect(p.setUninstallError).toHaveBeenCalledWith(null);
    expect(p.setShowUninstallDialog).toHaveBeenCalledWith(true);
  });

  it("mounts the advanced Headroom settings card in production Settings", () => {
    render(<SettingsView {...props()} />);
    expect(screen.getByText("Advanced Headroom settings mock")).toBeVisible();
  });

  it("invokes exact native commands and loads logs", async () => {
    const user = userEvent.setup();
    invokeMock.mockResolvedValueOnce(undefined).mockResolvedValueOnce(undefined).mockResolvedValueOnce(["line one"]);
    const p = props();
    render(<SettingsView {...p} />);
    await user.click(screen.getByRole("button", { name: "Contact us" }));
    await user.click(screen.getByRole("button", { name: "Quit AI Switchboard for Mac" }));
    await user.click(screen.getByRole("button", { name: "Logs mock" }));
    expect(invokeMock).toHaveBeenNthCalledWith(1, "open_external_link", { url: "https://example.test/issues" });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "quit_headroom");
    expect(invokeMock).toHaveBeenNthCalledWith(3, "get_headroom_logs", { maxLines: 80 });
    await waitFor(() => expect(p.setHeadroomLogLines).toHaveBeenCalledWith(["line one"]));
    expect(p.setShowHeadroomDetails).toHaveBeenCalledWith(true);
  });

  it("surfaces a native log loading failure", async () => {
    const user = userEvent.setup();
    invokeMock.mockRejectedValueOnce(new Error("offline"));
    const p = props();
    render(<SettingsView {...p} />);
    await user.click(screen.getByRole("button", { name: "Logs mock" }));
    await waitFor(() => expect(p.setHeadroomLogLines).toHaveBeenCalledWith(["Failed to load headroom logs."]));
  });

  it("forwards connector, autostart, updater, and release evidence actions", async () => {
    const user = userEvent.setup();
    const p = props();
    render(<SettingsView {...p} />);
    for (const name of ["Toggle connector mock", "Copy connector mock", "Autostart mock", "Update check mock", "Copy release mock", "Refresh release mock", "Evidence command mock", "Evidence sequence mock"]) {
      await user.click(screen.getByRole("button", { name }));
    }
    expect(p.toggleConnector).toHaveBeenCalledWith({ clientId: "codex" }, true);
    expect(p.copyPlannedConnectorCommand).toHaveBeenCalledWith("plan", "Codex");
    expect(p.handleAutostartToggle).toHaveBeenCalledWith(true);
    expect(p.checkForAppUpdate).toHaveBeenCalledOnce();
    expect(p.copyReleaseReadinessReport).toHaveBeenCalledOnce();
    expect(p.refreshReleaseReadinessReport).toHaveBeenCalledOnce();
    expect(p.runReleaseEvidenceCommand).toHaveBeenCalledWith("signed-app");
    expect(p.runLocalReleaseEvidenceSequence).toHaveBeenCalledOnce();
  });
});
