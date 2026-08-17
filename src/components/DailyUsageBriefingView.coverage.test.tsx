import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DailyUsageBriefingView } from "./DailyUsageBriefingView";

const mocks = vi.hoisted(() => ({ load: vi.fn(), history: vi.fn(), export: vi.fn(), preview: vi.fn(), clear: vi.fn() }));
vi.mock("../lib/usageAnalytics", () => ({
  loadDailyUsageBriefing: mocks.load,
  loadDailyUsageBriefingHistory: mocks.history,
  exportDailyUsageBriefing: mocks.export,
  previewClearUsageAnalytics: mocks.preview,
  clearUsageAnalytics: mocks.clear,
  formatMetric: (metric: { value?: number | null } | number) => typeof metric === "number" ? String(metric) : String(metric.value ?? "Unavailable"),
}));

const metric = (value: number) => ({ value, confidence: "measured", source: "local", caveat: null });
const briefing = {
  dayKey: "2026-08-17", timezone: "Asia/Kolkata", generatedAt: null, completeness: "partial", headline: null,
  totals: { requests: metric(2), spentTokens: metric(100), savedTokens: metric(40), cachedTokens: metric(10), avoidedTokens: metric(5), estimatedCostUsd: metric(1), estimatedSavingsUsd: metric(2) },
  agents: [{ id: "codex", label: "Codex", requests: 2, spentTokens: metric(100), savedTokens: metric(40), highestContextPercent: 80, detail: "active" }],
  attentionItems: [{ id: "doctor", title: "Repair", detail: "Runtime warning", severity: "warning", destination: "/doctor" }],
  recommendations: [{ id: "usage", title: "Review", evidence: "Measured", destination: null }],
  evidenceCoverage: { measured: 2, estimated: 1, inferred: 0, unavailable: 0 },
};

describe("DailyUsageBriefingView scenarios", () => {
  beforeEach(() => {
    mocks.load.mockReset().mockResolvedValue(briefing);
    mocks.history.mockReset().mockResolvedValue([briefing]);
    mocks.export.mockReset().mockImplementation((format: string) => Promise.resolve(`${format} export`));
    mocks.preview.mockReset().mockResolvedValue({ briefingCount: 1, eventCount: 2, detail: "Savings retained." });
    mocks.clear.mockReset().mockResolvedValue({ detail: "Analytics cleared." });
  });

  it("renders evidence, navigates destinations, and loads saved history", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    const onNavigate = vi.fn();
    render(<DailyUsageBriefingView hidden={false} onNavigate={onNavigate} />);
    expect(await screen.findByText("Local usage for 2026-08-17")).toBeVisible();
    expect(screen.getByText(/peak context 80%/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Open" }));
    expect(onNavigate).toHaveBeenCalledWith("doctor");
    await user.click(screen.getByRole("button", { name: "Load history" }));
    expect(await screen.findByText("2026-08-17")).toBeVisible();
  });

  it("copies both secret-free export formats", async () => {
    const user = userEvent.setup({ writeToClipboard: false });
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    render(<DailyUsageBriefingView hidden={false} onNavigate={vi.fn()} />);
    await screen.findByText("Local usage for 2026-08-17");
    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(writeText).toHaveBeenCalledWith("markdown export");
    await user.click(screen.getByRole("button", { name: "JSON" }));
    expect(writeText).toHaveBeenCalledWith("json export");
    expect(await screen.findByText("Secret-free JSON export copied.")).toBeVisible();
  });

  it("previews, cancels, then clears analytics and refreshes", async () => {
    const user = userEvent.setup();
    render(<DailyUsageBriefingView hidden={false} onNavigate={vi.fn()} />);
    await screen.findByText("Local usage for 2026-08-17");
    await user.click(screen.getByRole("button", { name: "Preview local analytics deletion" }));
    expect(await screen.findByText(/Ready to delete 1 saved briefings/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("button", { name: "Delete local analytics" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Preview local analytics deletion" }));
    await user.click(await screen.findByRole("button", { name: "Delete local analytics" }));
    expect(await screen.findByText("Analytics cleared.")).toBeVisible();
    expect(mocks.load).toHaveBeenCalledTimes(2);
  });

  it("shows insufficient data and safe load fallback errors", async () => {
    mocks.load.mockResolvedValueOnce({ ...briefing, completeness: "insufficient-data" });
    const { rerender } = render(<DailyUsageBriefingView hidden={false} onNavigate={vi.fn()} />);
    expect(await screen.findByText("Not enough local evidence yet")).toBeVisible();
    mocks.load.mockRejectedValueOnce({ reason: "unknown" });
    rerender(<DailyUsageBriefingView hidden onNavigate={vi.fn()} />);
    rerender(<DailyUsageBriefingView hidden={false} onNavigate={vi.fn()} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Daily briefing is unavailable.");
  });

  it("reports preview and deletion failures without pretending data changed", async () => {
    const user = userEvent.setup();
    mocks.preview.mockRejectedValueOnce(new Error("unsupported"));
    render(<DailyUsageBriefingView hidden={false} onNavigate={vi.fn()} />);
    await screen.findByText("Local usage for 2026-08-17");
    await user.click(screen.getByRole("button", { name: "Preview local analytics deletion" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("retention controls are not available");
    mocks.preview.mockResolvedValueOnce({ briefingCount: 1, eventCount: 2, detail: "Local" });
    mocks.clear.mockRejectedValueOnce(new Error("denied"));
    await user.click(screen.getByRole("button", { name: "Preview local analytics deletion" }));
    await user.click(await screen.findByRole("button", { name: "Delete local analytics" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("data was left unchanged");
  });
});
