import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ClaudeUsageCard } from "./ClaudeUsageCard";

const load = vi.hoisted(() => vi.fn());
vi.mock("../lib/claudeUsage", () => ({ loadClaudeUsage: load }));

describe("ClaudeUsageCard", () => {
  beforeEach(() => load.mockReset());

  it("renders reset windows and refreshes", async () => {
    load.mockResolvedValue({ fiveHour: { utilization: 42.5, resetsAt: "2026-08-23T12:00:00Z" }, sevenDay: { utilization: 12, resetsAt: "2026-08-24T12:00:00Z" }, extraUsage: { isEnabled: true, monthlyLimit: 20, usedCredits: 2, utilization: 10 } });
    render(<ClaudeUsageCard hidden={false} />);
    expect(await screen.findByText("Five-hour window")).toBeInTheDocument();
    expect(screen.getByText("42.5% used")).toBeInTheDocument();
    expect(screen.getByText(/Extra usage: enabled.*2 credits used of 20 monthly/)).toBeInTheDocument();
    load.mockResolvedValueOnce({ fiveHour: null, sevenDay: null, extraUsage: null });
    fireEvent.click(screen.getByRole("button", { name: "Refresh usage" }));
    await waitFor(() => expect(load).toHaveBeenCalledTimes(2));
  });

  it("shows auth/API failures without hiding the scope explanation", async () => {
    load.mockImplementationOnce(() => { throw new Error("Claude token unavailable"); });
    render(<ClaudeUsageCard hidden={false} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Claude token unavailable");
    expect(screen.getByText(/using the locally captured Claude OAuth session/)).toBeInTheDocument();
  });
});
