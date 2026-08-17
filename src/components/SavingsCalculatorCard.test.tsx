import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { mockDashboard } from "../lib/mockData";
import { SavingsCalculatorCard } from "./SavingsCalculatorCard";

describe("SavingsCalculatorCard", () => {
  beforeEach(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  it("renders the honest empty state and forwards every scope", () => {
    const onScopeChange = vi.fn();
    render(<SavingsCalculatorCard dashboard={mockDashboard} scope="session" onScopeChange={onScopeChange} />);
    expect(screen.getByText("Waiting for usage")).toBeInTheDocument();
    for (const label of ["current repo", "today", "this week", "this month", "lifetime"]) {
      fireEvent.click(screen.getByRole("button", { name: label }));
    }
    expect(onScopeChange).toHaveBeenCalledTimes(5);
  });

  it("opens source details, filters and copies a populated measured ledger", async () => {
    const dashboard = {
      ...mockDashboard,
      lifetimeRequests: 3,
      sessionRequests: 2,
      sessionEstimatedSavingsUsd: 1.5,
      sessionEstimatedTokensSaved: 1_000,
      sessionSavingsPct: 25,
      lifetimeEstimatedSavingsUsd: 2,
      lifetimeEstimatedTokensSaved: 1_500,
    };
    render(
      <SavingsCalculatorCard
        dashboard={dashboard}
        scope="session"
        onScopeChange={vi.fn()}
        attributionEvents={[{
          schemaVersion: 1, id: "evt-1", observedAt: new Date().toISOString(), source: "headroom_engine", scope: "session",
          confidence: "measured", deltaTokensSaved: 500, deltaUsd: 0.5, totalTokensSent: 2_000, requestDelta: 1,
          evidence: ["test fixture"],
        }]}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Learn more" }));
    expect(screen.getByLabelText("Savings ledger")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "measured" }));
    fireEvent.click(screen.getByTitle("Copy savings summary"));
    await waitFor(() => expect(navigator.clipboard.writeText).toHaveBeenCalled());
    expect(screen.getByText("Copied measured.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Hide details" }));
  });

  it("reports an unavailable clipboard without throwing", () => {
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined });
    render(<SavingsCalculatorCard dashboard={{ ...mockDashboard, sessionRequests: 1 }} scope="session" onScopeChange={vi.fn()} />);
    fireEvent.click(screen.getByTitle("Copy savings summary"));
    expect(screen.getByText("Clipboard unavailable.")).toBeInTheDocument();
  });
});
