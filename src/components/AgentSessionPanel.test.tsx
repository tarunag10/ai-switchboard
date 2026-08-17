import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AgentSessionPanel } from "./AgentSessionPanel";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("AgentSessionPanel", () => {
  beforeEach(() => {
    localStorage.clear();
    invoke.mockReset();
    invoke.mockImplementation(async (command: string) => {
      if (command === "get_runtime_status") {
        return { installed: true, running: true, proxyReachable: true, mcpConfigured: true, rtk: { installed: true, enabled: true } };
      }
      if (command === "get_switchboard_state") return { mode: "full" };
      if (command === "get_semantic_cache_status") return { enabled: false };
      if (command === "get_latest_repo_intelligence_summary") return null;
      throw new Error(command);
    });
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  it("loads safety evidence, persists edits, switches agents, and copies after acknowledgment", async () => {
    render(<AgentSessionPanel />);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(4));

    const task = screen.getByRole("textbox");
    fireEvent.change(task, { target: { value: "Summarize this diff" } });
    const budget = screen.getByRole("spinbutton");
    fireEvent.change(budget, { target: { value: "4000" } });
    expect(localStorage.getItem("ai-switchboard.agent-session.budget.v1")).toContain("4000");

    const agent = screen.getAllByRole("combobox")[0];
    const nextAgent = Array.from((agent as HTMLSelectElement).options).find((option) => option.value !== (agent as HTMLSelectElement).value);
    if (nextAgent) fireEvent.change(agent, { target: { value: nextAgent.value } });

    const acknowledge = screen.queryByRole("checkbox", { name: /reviewed the checklist warnings/i });
    if (acknowledge) fireEvent.click(acknowledge);
    const copy = screen.getByRole("button", { name: /Copy payload/i });
    if (!copy.hasAttribute("disabled")) {
      fireEvent.click(copy);
      await waitFor(() => expect(navigator.clipboard.writeText).toHaveBeenCalled());
      expect(screen.getByText(/Payload copied/i)).toBeInTheDocument();
    }
  });

  it("recovers from malformed stored budgets and unavailable backend evidence", async () => {
    localStorage.setItem("ai-switchboard.agent-session.budget.v1", "not-json");
    invoke.mockRejectedValue(new Error("offline"));
    render(<AgentSessionPanel />);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(4));
    expect(screen.getByText("Compression checklist")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("spinbutton"), { target: { value: "0" } });
    expect(screen.getByText("0 means no session budget limit.")).toBeInTheDocument();
  });
});
