import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SelectiveActivationCard } from "./SelectiveActivationCard";

const invoke = vi.fn();
const loadTokenXraySnapshot = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock("../lib/usageAnalytics", () => ({ loadTokenXraySnapshot: (...args: unknown[]) => loadTokenXraySnapshot(...args) }));

describe("SelectiveActivationCard", () => {
  beforeEach(() => {
    window.localStorage.clear();
    invoke.mockReset();
    invoke.mockResolvedValue({ configured: true });
    loadTokenXraySnapshot.mockReset();
    loadTokenXraySnapshot.mockResolvedValue({});
  });

  it("requires five tools and activates the selected tools in one click", async () => {
    render(<SelectiveActivationCard />);
    expect(screen.getByRole("button", { name: "Activate selected 5" })).toBeDisabled();

    const tools = screen.getAllByRole("button", { pressed: false });
    tools.slice(0, 5).forEach((tool) => fireEvent.click(tool));
    expect(screen.getByText("5/5")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Activate selected 5" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Activate selected 5" }));
    await waitFor(() => expect(screen.getByText("Activated all 5 selected tools.")).toBeInTheDocument());
    expect(invoke).toHaveBeenCalledWith("set_switchboard_mode", { mode: "full" });
    expect(invoke).toHaveBeenCalledWith("install_addon", { id: "rtk" });
    expect(loadTokenXraySnapshot).toHaveBeenCalled();
  });

  it("caps selection at five and persists the chosen ids", () => {
    render(<SelectiveActivationCard />);
    screen.getAllByRole("button", { pressed: false }).slice(0, 6).forEach((tool) => fireEvent.click(tool));
    expect(screen.getByText("5/5")).toBeInTheDocument();
    expect(JSON.parse(window.localStorage.getItem("ai-switchboard.selective-activation.v1") ?? "{}").selectedToolIds).toHaveLength(5);
  });
});
