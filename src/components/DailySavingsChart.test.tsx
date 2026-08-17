import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DailySavingsChart } from "./DailySavingsChart";

const eventMocks = vi.hoisted(() => ({ listener: undefined as undefined | ((event: { payload: number }) => void) }));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_name: string, listener: (event: { payload: number }) => void) => {
    eventMocks.listener = listener;
    return vi.fn();
  }),
}));
vi.mock("../lib/tauriRuntime", () => ({ hasTauriEventRuntime: () => true }));
vi.mock("recharts", () => {
  const Box = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  return {
    ResponsiveContainer: Box,
    BarChart: Box,
    CartesianGrid: Box,
    Tooltip: Box,
    XAxis: Box,
    YAxis: Box,
    Bar: ({ shape }: { shape?: (props: Record<string, unknown>) => React.ReactNode }) => (
      <div>{shape ? <>{shape({ x: 1, y: 2, width: 10, height: 12, fill: "red" })}{shape({ x: 0, y: 0, width: 0, height: 0, fill: "red" })}</> : null}</div>
    ),
  };
});

describe("DailySavingsChart", () => {
  beforeEach(() => {
    eventMocks.listener = undefined;
  });

  it("switches metric and history windows while enforcing navigation bounds", async () => {
    const setChartMode = vi.fn();
    const { rerender } = render(
      <DailySavingsChart
        chartMode="usd"
        data={[{ date: "2026-07-01", estimatedSavingsUsd: 2, estimatedTokensSaved: 200, actualCostUsd: 1, totalTokensSent: 500 }]}
        hourlyData={[{ hour: "2026-08-15T10:00", estimatedSavingsUsd: 1, estimatedTokensSaved: 100, actualCostUsd: 0.5, totalTokensSent: 300, byProvider: [] }]}
        resetSignal={0}
        setChartMode={setChartMode}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "tokens" }));
    expect(setChartMode).toHaveBeenCalledWith("tokens");
    fireEvent.click(screen.getByRole("button", { name: "Prev" }));
    expect(screen.getByRole("button", { name: "Next" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "month" }));
    fireEvent.click(screen.getByRole("button", { name: "Prev" }));
    expect(screen.getByRole("button", { name: "Next" })).toBeEnabled();
    rerender(
      <DailySavingsChart chartMode="tokens" data={[]} hourlyData={[]} resetSignal={1} setChartMode={setChartMode} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "day" }));
    expect(screen.getByText("saved today")).toBeInTheDocument();
  });

  it("uses the live savings event for today's USD overlay", async () => {
    render(<DailySavingsChart chartMode="usd" data={[]} hourlyData={[]} resetSignal={0} setChartMode={vi.fn()} />);
    await waitFor(() => expect(eventMocks.listener).toBeTypeOf("function"));
    act(() => eventMocks.listener?.({ payload: 12.5 }));
    expect(await screen.findByText("$13")).toBeInTheDocument();
  });
});
