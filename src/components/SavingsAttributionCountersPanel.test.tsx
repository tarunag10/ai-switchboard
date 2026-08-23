import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SavingsAttributionCountersPanel } from "./SavingsAttributionCountersPanel";

const load = vi.hoisted(() => vi.fn());
vi.mock("../lib/savingsAttributionCounters", () => ({ loadSavingsAttributionCounters: load }));

describe("SavingsAttributionCountersPanel", () => {
  beforeEach(() => load.mockReset());

  it("shows compact source totals and refreshes", async () => {
    load.mockResolvedValue([{ source: "headroom_engine", scope: "session", eventCount: 3, runtimeEventCount: 3, measuredEventCount: 2, estimatedEventCount: 1, inferredEventCount: 0, deltaTokensSaved: 1200, totalTokensSent: 4000, lastSeenAt: null }]);
    render(<SavingsAttributionCountersPanel hidden={false} />);
    expect(await screen.findByText("Headroom")).toBeInTheDocument();
    expect(screen.getByText(/1.2K tokens saved/)).toBeInTheDocument();
    load.mockResolvedValueOnce([]);
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(load).toHaveBeenCalledTimes(2));
  });

  it("keeps backend failures visible", async () => {
    load.mockImplementationOnce(() => {
      throw new Error("counter backend unavailable");
    });
    render(<SavingsAttributionCountersPanel hidden={false} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("counter backend unavailable");
  });
});
