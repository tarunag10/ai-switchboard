import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { mockDashboard } from "../lib/mockData";
import { AddonsView, type AddonsViewProps } from "./AddonsView";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("./AddonHealthStrip", () => ({ AddonHealthStrip: () => null }));
vi.mock("./MeasuredAddonSavingsForm", () => ({ MeasuredAddonSavingsForm: () => null }));
vi.mock("./GatewayProfilesCard", () => ({ GatewayProfilesCard: ({ onCopyGuidance }: any) => <button onClick={() => onCopyGuidance("gateway", "Gateway")}>Gateway guidance mock</button> }));
vi.mock("./OptimizationEngineProfilesCard", () => ({ OptimizationEngineProfilesCard: ({ onCopyGuidance }: any) => <button onClick={() => onCopyGuidance("engine", "Engine")}>Engine guidance mock</button> }));
vi.mock("./ProviderBilledCounterfactualCard", () => ({ ProviderBilledCounterfactualCard: () => null }));
vi.mock("./PlannedAddonCard", () => ({ PlannedAddonCard: ({ onOpenRepoIntelligence }: any) => <button onClick={onOpenRepoIntelligence}>Open repo intelligence mock</button> }));
vi.mock("./AddonCard", () => ({
  AddonCard: ({ name, children, onInstall, onToggleEnabled, onUninstall, onOpenSource, onToggleInfo, onDismissResult }: any) => <li>
    <span>{name}</span>{children}
    <button onClick={onInstall}>Install {name}</button>
    <button onClick={onToggleEnabled}>Toggle {name}</button>
    <button onClick={onUninstall}>Uninstall {name}</button>
    <button onClick={onOpenSource}>Source {name}</button>
    <button onClick={onToggleInfo}>Info {name}</button>
    <button onClick={onDismissResult}>Dismiss {name}</button>
  </li>,
}));

function props(overrides: Partial<AddonsViewProps> = {}): AddonsViewProps {
  return {
    activeView: "addons", setActiveView: vi.fn(), addonError: null,
    runtimeStatus: { rtk: { installed: true, enabled: true } } as any,
    dashboard: { ...mockDashboard, tools: [{ id: "rtk", name: "RTK", description: "RTK", required: false, enabled: true, status: "installed", sourceUrl: "https://example.test/rtk", version: "1" }] },
    connectors: [], addonCopy: { rtk: {} as any }, addonInfoId: null, setAddonInfoId: vi.fn(), addonBusyId: null,
    addonBusyLabel: null, addonResult: null, setAddonResult: vi.fn(), rtkAvgSavingsPct: null, rtkBusy: false,
    openExternalLink: vi.fn(), runAddonAction: vi.fn(), handleRtkToggle: vi.fn(), onMeasuredAddonSavingsRecorded: vi.fn(),
    setCavemanLevel: vi.fn(), copyPlannedConnectorCommand: vi.fn(), ...overrides,
  } as AddonsViewProps;
}

describe("AddonsView integration", () => {
  beforeEach(() => invokeMock.mockReset());

  it("wires RTK lifecycle, source, and planned navigation", async () => {
    const user = userEvent.setup();
    const p = props();
    render(<AddonsView {...p} />);
    await user.click(screen.getByRole("button", { name: "Install RTK" }));
    await user.click(screen.getByRole("button", { name: "Toggle RTK" }));
    await user.click(screen.getByRole("button", { name: "Uninstall RTK" }));
    await user.click(screen.getByRole("button", { name: "Source RTK" }));
    await user.click(screen.getAllByRole("button", { name: "Open repo intelligence mock" })[0]);
    expect(p.runAddonAction).toHaveBeenNthCalledWith(1, "install_addon", "rtk");
    expect(p.handleRtkToggle).toHaveBeenCalledWith(false);
    expect(p.runAddonAction).toHaveBeenNthCalledWith(2, "uninstall_addon", "rtk");
    expect(p.openExternalLink).toHaveBeenCalledWith("https://example.test/rtk");
    expect(p.setActiveView).toHaveBeenCalledWith("repoIntelligence");
  });

  it("invokes the exact RTK activity command and renders success and error states", async () => {
    const user = userEvent.setup();
    invokeMock.mockResolvedValueOnce(["saved 42 tokens"]);
    const view = render(<AddonsView {...props()} />);
    await user.click(screen.getByRole("button", { name: "Show RTK activity" }));
    expect(invokeMock).toHaveBeenCalledWith("get_rtk_activity", { maxLines: 80 });
    expect(await screen.findByText("saved 42 tokens")).toBeInTheDocument();

    view.unmount();
    invokeMock.mockRejectedValueOnce(new Error("offline"));
    render(<AddonsView {...props()} />);
    await user.click(screen.getByRole("button", { name: "Show RTK activity" }));
    await waitFor(() => expect(screen.getByText("Failed to load RTK activity.")).toBeInTheDocument());
  });

  it("toggles a regular addon and selects alternate Caveman levels", async () => {
    const user = userEvent.setup();
    const caveman = { id: "caveman", name: "Caveman", description: "Compact prompts", required: false, enabled: true, status: "installed", sourceUrl: "https://example.test/caveman", version: "1", metadata: { level: "scoped" } } as any;
    const markitdown = { id: "markitdown", name: "MarkItDown", description: "Documents", required: false, enabled: false, status: "installed", sourceUrl: "https://example.test/markitdown", version: "1" } as any;
    const p = props({ dashboard: { ...mockDashboard, tools: [caveman, markitdown] }, addonCopy: { caveman: {} as any, markitdown: {} as any } });
    render(<AddonsView {...p} />);
    await user.click(screen.getByRole("button", { name: "Toggle MarkItDown" }));
    await user.click(screen.getByRole("button", { name: "Aggressive" }));
    await user.click(screen.getByRole("button", { name: "Compact Chinese" }));
    expect(p.runAddonAction).toHaveBeenCalledWith("set_addon_enabled", "markitdown", true);
    expect(p.setCavemanLevel).toHaveBeenNthCalledWith(1, "aggressive");
    expect(p.setCavemanLevel).toHaveBeenNthCalledWith(2, "compact_chinese");
  });

  it("copies an RTK task preset and reports clipboard unavailability", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    const view = render(<AddonsView {...props()} />);
    const copy = screen.getAllByRole("button", { name: /Copy .* preset/ })[0];
    await user.click(copy);
    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("RTK"));
    expect(screen.getByRole("status")).toHaveTextContent("Copied");

    view.unmount();
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined });
    render(<AddonsView {...props()} />);
    await user.click(screen.getAllByRole("button", { name: /Copy .* preset/ })[0]);
    expect(screen.getByRole("status")).toHaveTextContent("Clipboard unavailable.");
  });

  it("forwards info, result dismissal, and guidance-copy callbacks", async () => {
    const user = userEvent.setup();
    const p = props({ addonResult: { id: "rtk", message: "Installed" } });
    render(<AddonsView {...p} />);
    await user.click(screen.getByRole("button", { name: "Info RTK" }));
    await user.click(screen.getByRole("button", { name: "Dismiss RTK" }));
    await user.click(screen.getByRole("button", { name: "Gateway guidance mock" }));
    await user.click(screen.getByRole("button", { name: "Engine guidance mock" }));
    expect(p.setAddonInfoId).toHaveBeenCalledWith("rtk");
    expect(p.setAddonResult).toHaveBeenCalledWith(null);
    expect(p.copyPlannedConnectorCommand).toHaveBeenCalledWith("gateway", "Gateway");
    expect(p.copyPlannedConnectorCommand).toHaveBeenCalledWith("engine", "Engine");
  });
});
