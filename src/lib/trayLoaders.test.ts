import { beforeEach, describe, expect, it, vi } from "vitest";

import { mockDashboard } from "./mockData";
import { delay, loadDashboard, loadSavingsAttributionEvents } from "./trayLoaders";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("trayLoaders", () => {
  beforeEach(() => invokeMock.mockReset());

  it("loads the dashboard from the exact Tauri command", async () => {
    const dashboard = { ...mockDashboard, requestsToday: 42 };
    invokeMock.mockResolvedValueOnce(dashboard);

    await expect(loadDashboard()).resolves.toEqual(dashboard);
    expect(invokeMock).toHaveBeenCalledWith("get_dashboard_state");
  });

  it("falls back to mock dashboard data when native loading fails", async () => {
    invokeMock.mockRejectedValueOnce(new Error("offline"));
    await expect(loadDashboard()).resolves.toBe(mockDashboard);
  });

  it("loads savings events and returns an empty list on failure", async () => {
    const events = [{ requestId: "request-1" }];
    invokeMock.mockResolvedValueOnce(events);
    await expect(loadSavingsAttributionEvents()).resolves.toEqual(events);
    expect(invokeMock).toHaveBeenCalledWith("get_savings_attribution_events");

    invokeMock.mockRejectedValueOnce("unavailable");
    await expect(loadSavingsAttributionEvents()).resolves.toEqual([]);
  });

  it("resolves delay only after the requested timeout", async () => {
    vi.useFakeTimers();
    const completion = vi.fn();
    void delay(125).then(completion);
    expect(completion).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(125);
    expect(completion).toHaveBeenCalledOnce();
    vi.useRealTimers();
  });
});
