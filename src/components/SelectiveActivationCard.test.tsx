import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StrictMode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SelectiveActivationCard } from "./SelectiveActivationCard";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

describe("SelectiveActivationCard", () => {
  beforeEach(() => {
    window.localStorage.clear();
    invoke.mockReset();
    invoke.mockImplementation((command: string) => {
      if (command === "get_selective_activation_selection" || command === "get_selective_activation_recovery") return Promise.resolve(null);
      if (command === "activate_selected_tools") return Promise.resolve({ dashboard: {}, receipt: { runId: "run-1", overallStatus: "succeeded", results: ["headroom", "rtk", "repo-intelligence", "token-xray", "ponytail"].map((toolId) => ({ toolId, state: toolId === "repo-intelligence" || toolId === "token-xray" ? "refreshed" : "enabled", detail: `${toolId} ready` })) } });
      if (command === "rollback_selective_activation") return Promise.resolve({ dashboard: {}, receipt: { runId: "run-1", overallStatus: "succeeded", results: [{ toolId: "headroom", state: "restored", detail: "mode restored" }] } });
      return Promise.resolve({});
    });
  });

  it("requires five tools and activates the selected tools in one click", async () => {
    render(<SelectiveActivationCard />);
    await screen.findByRole("button", { name: "Activate selected 5" });
    expect(screen.getByRole("button", { name: "Activate selected 5" })).toBeDisabled();

    const tools = screen.getAllByRole("button", { pressed: false });
    tools.slice(0, 5).forEach((tool) => fireEvent.click(tool));
    expect(screen.getByText("5/5")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Activate selected 5" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Activate selected 5" }));
    await waitFor(() => expect(screen.getByText("Activated all 5 selected tools.")).toBeInTheDocument());
    expect(invoke).toHaveBeenCalledWith("activate_selected_tools", { selectedToolIds: ["headroom", "rtk", "repo-intelligence", "token-xray", "ponytail"] });
    expect(invoke.mock.calls.filter(([command]) => command === "activate_selected_tools")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "Undo last native tool activation" }));
    await waitFor(() => expect(screen.getByText(/Last native tool activation was rolled back/)).toBeInTheDocument());
    expect(invoke).toHaveBeenCalledWith("rollback_selective_activation", { runId: "run-1" });
  });

  it("caps selection at five and persists the chosen ids", async () => {
    render(<SelectiveActivationCard />);
    await screen.findByRole("button", { name: "Activate selected 5" });
    screen.getAllByRole("button", { pressed: false }).slice(0, 6).forEach((tool) => fireEvent.click(tool));
    expect(screen.getByText("5/5")).toBeInTheDocument();
    expect(JSON.parse(window.localStorage.getItem("ai-switchboard.selective-activation.v1") ?? "{}").selectedToolIds).toHaveLength(5);
  });

  it("keeps native rollback available when the dashboard refresh callback fails", async () => {
    const onComplete = vi.fn().mockRejectedValue(new Error("refresh failed"));
    render(<SelectiveActivationCard onComplete={onComplete} />);
    await screen.findByRole("button", { name: "Activate selected 5" });
    screen.getAllByRole("button", { pressed: false }).slice(0, 5).forEach((tool) => fireEvent.click(tool));
    fireEvent.click(screen.getByRole("button", { name: "Activate selected 5" }));

    await waitFor(() => expect(screen.getByText(/Activation completed and remains undoable/)).toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Undo last native tool activation" })).toBeEnabled();
  });

  it("restores native selection and rollback access without automatically retrying", async () => {
    const selectedToolIds = ["headroom", "rtk", "repo-intelligence", "token-xray", "ponytail"];
    invoke.mockImplementation((command: string) => {
      if (command === "get_selective_activation_selection") return Promise.resolve({ version: 1, selectedToolIds, updatedAt: "2026-08-24T06:00:00Z" });
      if (command === "get_selective_activation_recovery") return Promise.resolve({
        version: 1,
        runId: "selective-1720000000000-42",
        selectedToolIds,
        overallStatus: "partial",
        updatedAt: "2026-08-24T06:00:00Z",
        rollbackStatus: null,
        rollbackAvailable: true,
      });
      if (command === "rollback_selective_activation") return Promise.resolve({ dashboard: {}, receipt: { runId: "selective-1720000000000-42", overallStatus: "succeeded", results: [] } });
      return Promise.resolve({});
    });

    render(<SelectiveActivationCard />);
    await waitFor(() => expect(screen.getByText("5/5")).toBeInTheDocument());
    expect(screen.getByText(/Automatic retry is disabled/)).toBeInTheDocument();
    expect(invoke.mock.calls.filter(([command]) => command === "activate_selected_tools")).toHaveLength(0);

    fireEvent.click(screen.getByRole("button", { name: "Undo last native tool activation" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("rollback_selective_activation", { runId: "selective-1720000000000-42" }));
  });

  it("keeps selection independent from a conflicting activation receipt", async () => {
    const nativeSelection = ["headroom", "rtk", "repo-intelligence", "token-xray", "ponytail"];
    const receiptSelection = ["caveman", "markitdown", "response-cache", "chonkify", "leanctx"];
    invoke.mockImplementation((command: string) => {
      if (command === "get_selective_activation_selection") return Promise.resolve({ version: 1, selectedToolIds: nativeSelection });
      if (command === "get_selective_activation_recovery") return Promise.resolve({
        version: 1,
        runId: "selective-1720000000000-43",
        selectedToolIds: receiptSelection,
        overallStatus: "succeeded",
        updatedAt: "2026-08-24T06:00:00Z",
        rollbackStatus: null,
        rollbackAvailable: true,
      });
      return Promise.resolve(null);
    });

    render(<SelectiveActivationCard />);
    await screen.findByText(/different five-tool selection/);
    expect(screen.getByRole("button", { name: /Headroom Engine/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: /Caveman/ })).toHaveAttribute("aria-pressed", "false");
  });

  it("does not overwrite native selection after a rejected read", async () => {
    const localSelection = ["headroom", "rtk", "repo-intelligence", "token-xray", "ponytail"];
    window.localStorage.setItem("ai-switchboard.selective-activation.v1", JSON.stringify({ version: 1, selectedToolIds: localSelection }));
    invoke.mockImplementation((command: string) => {
      if (command === "get_selective_activation_selection") return Promise.reject(new Error("corrupt native selection"));
      if (command === "get_selective_activation_recovery") return Promise.resolve(null);
      return Promise.resolve({});
    });

    render(<SelectiveActivationCard />);
    await screen.findByText(/will not be overwritten until you change it/);
    expect(invoke.mock.calls.filter(([command]) => command === "save_selective_activation_selection")).toHaveLength(0);

    fireEvent.click(screen.getByRole("button", { name: /Ponytail/ }));
    fireEvent.click(screen.getByRole("button", { name: /Caveman/ }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("save_selective_activation_selection", {
      selectedToolIds: ["headroom", "rtk", "repo-intelligence", "token-xray", "caveman"],
    }));
  });

  it("shows interrupted rollback as repair-required without resuming it", async () => {
    const selectedToolIds = ["headroom", "rtk", "repo-intelligence", "token-xray", "ponytail"];
    invoke.mockImplementation((command: string) => {
      if (command === "get_selective_activation_selection") return Promise.resolve(null);
      if (command === "get_selective_activation_recovery") return Promise.resolve({
        version: 1,
        runId: "selective-1720000000000-44",
        selectedToolIds,
        overallStatus: "partial",
        updatedAt: "2026-08-24T06:00:00Z",
        rollbackStatus: "partial",
        rollbackAvailable: false,
      });
      return Promise.resolve({});
    });

    render(<SelectiveActivationCard />);
    await screen.findByText(/requires repair/);
    expect(screen.queryByRole("button", { name: "Undo last native tool activation" })).not.toBeInTheDocument();
    expect(invoke.mock.calls.filter(([command]) => command === "rollback_selective_activation")).toHaveLength(0);
  });

  it("never activates or rolls back during StrictMode recovery", async () => {
    render(<StrictMode><SelectiveActivationCard /></StrictMode>);
    await screen.findByRole("button", { name: "Activate selected 5" });
    expect(invoke.mock.calls.filter(([command]) => command === "activate_selected_tools")).toHaveLength(0);
    expect(invoke.mock.calls.filter(([command]) => command === "rollback_selective_activation")).toHaveLength(0);
  });
});
