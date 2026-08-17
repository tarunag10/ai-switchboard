import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { UsageSavingsView } from "./UsageSavingsView";
import type { DashboardState } from "../lib/types";

vi.mock("./SavingsCalculatorCard", () => ({ SavingsCalculatorCard: ({ onScopeChange }: { onScopeChange: (v: string) => void }) => <button onClick={() => onScopeChange("week")}>scope child</button> }));
vi.mock("./ClientSavingsTrendsCard", () => ({ ClientSavingsTrendsCard: () => <div>trends child</div> }));
vi.mock("./DailySavingsChart", () => ({ DailySavingsChart: ({ chartMode }: { chartMode: string }) => <div>chart {chartMode}</div> }));
vi.mock("./OutputReductionChip", () => ({ OutputReductionChip: () => <span>reduction child</span> }));

const dashboard = { lifetimeEstimatedSavingsUsd: 12, lifetimeEstimatedTokensSaved: 5000, outputReduction: { reductionPercent: 20 }, savingsHistoryLoaded: true, dailySavings: [], hourlySavings: [] } as unknown as DashboardState;
const base = {
  hidden: false, chartMode: "usd", setChartMode: vi.fn(), setShowSavingsInfo: vi.fn(), savingsDashboard: dashboard, dashboard,
  savingsCalculatorRepoEstimate: {}, runtimeStatus: null, activityFeed: { tiles: { rtkToday: null } }, savingsAttributionEvents: [], cavemanSavingsEstimate: null, ponytailSavingsEstimate: null, markitdownSavingsEstimate: null,
  savingsCalculatorScope: "session", setSavingsCalculatorScope: vi.fn(), historyLoadTimedOut: false, chartResetSignal: 0,
} as unknown as React.ComponentProps<typeof UsageSavingsView>;

describe("UsageSavingsView", () => {
  it("wires card clicks, keyboard selection, info isolation, and child scope", async () => {
    const user = userEvent.setup();
    const props = { ...base, setChartMode: vi.fn(), setShowSavingsInfo: vi.fn(), setSavingsCalculatorScope: vi.fn() };
    render(<UsageSavingsView {...props} />);
    await user.click(screen.getByRole("button", { name: "How savings are calculated" }));
    expect(props.setShowSavingsInfo).toHaveBeenCalledWith(true);
    expect(props.setChartMode).not.toHaveBeenCalled();
    const tokenCard = screen.getByText("All-time input tokens saved").closest('[role="button"]')!;
    await user.click(tokenCard);
    expect(props.setChartMode).toHaveBeenCalledWith("tokens");
    tokenCard.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    expect(props.setChartMode).toHaveBeenCalledTimes(2);
    await user.click(screen.getByRole("button", { name: "scope child" }));
    expect(props.setSavingsCalculatorScope).toHaveBeenCalledWith("week");
    expect(screen.getByText("reduction child")).toBeVisible();
  });

  it("shows loading until history arrives and then mounts the chart", () => {
    const { rerender } = render(<UsageSavingsView {...base} dashboard={{ ...dashboard, savingsHistoryLoaded: false } as never} />);
    expect(screen.getByRole("status")).toHaveTextContent("Loading savings history");
    rerender(<UsageSavingsView {...base} dashboard={{ ...dashboard, savingsHistoryLoaded: false } as never} historyLoadTimedOut />);
    expect(screen.getByText("chart usd")).toBeVisible();
  });
});
