import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DailyUsageBriefingView } from "./DailyUsageBriefingView";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

const briefing = { dayKey: "2026-07-11", timezone: "Asia/Kolkata", generatedAt: "2026-07-11T08:00:00Z", completeness: "partial", totals: { requests: 2, inputTokens: { value: 90, confidence: "measured" }, savedTokens: { value: 20, confidence: "estimated" }, avoidedTokens: { value: 5, confidence: "inferred" }, outputTokens: { value: 0, confidence: "unavailable" }, estimatedCostUsd: { value: null, confidence: "unavailable" }, estimatedSavingsUsd: { value: null, confidence: "unavailable" } }, agents: [], attentionItems: [], recommendations: [], evidenceCoverage: {} };

describe("DailyUsageBriefingView history and retention", () => {
  beforeEach(() => { invokeMock.mockReset(); invokeMock.mockImplementation((command: string) => { if (command === "get_daily_usage_briefing") return Promise.resolve(briefing); return Promise.reject(new Error("unavailable")); }); });
  it("explains unavailable persisted history without blocking the current briefing", async () => {
    render(<DailyUsageBriefingView hidden={false} onNavigate={vi.fn()} />);
    await screen.findByText(/Local usage for 2026-07-11/i);
    fireEvent.click(screen.getByRole("button", { name: /load history/i }));
    await screen.findByText(/history is not available/i);
    expect(screen.getByText(/Analytics retention/i)).toBeInTheDocument();
  });
  it("requires a deletion preview before showing the destructive action", async () => {
    invokeMock.mockImplementation((command: string) => { if (command === "get_daily_usage_briefing") return Promise.resolve(briefing); if (command === "preview_clear_usage_analytics") return Promise.resolve({ briefingCount: 2, eventCount: 8, detail: "Local only." }); return Promise.reject(new Error("unavailable")); });
    render(<DailyUsageBriefingView hidden={false} onNavigate={vi.fn()} />);
    await screen.findByText(/Local usage for 2026-07-11/i);
    fireEvent.click(screen.getByRole("button", { name: /preview local analytics deletion/i }));
    await waitFor(() => expect(screen.getByRole("button", { name: /delete local analytics/i })).toBeInTheDocument());
    expect(screen.getByText(/2 saved briefings and 8 local analytics events/i)).toBeInTheDocument();
  });
});
