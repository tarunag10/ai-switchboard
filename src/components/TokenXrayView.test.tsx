import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TokenXrayView } from "./TokenXrayView";

const { loadTokenXraySnapshot, loadTokenXrayLiveUpdate } = vi.hoisted(() => ({ loadTokenXraySnapshot: vi.fn(), loadTokenXrayLiveUpdate: vi.fn() }));

vi.mock("../lib/usageAnalytics", () => ({
  loadTokenXraySnapshot,
  loadTokenXrayLiveUpdate,
  formatMetric: (metric: { value: number | null }) => metric.value === null ? "Unavailable" : String(metric.value),
}));

describe("TokenXrayView", () => {
  beforeEach(() => {
    loadTokenXraySnapshot.mockReset();
    loadTokenXrayLiveUpdate.mockReset();
    loadTokenXrayLiveUpdate.mockResolvedValue(null);
    loadTokenXraySnapshot.mockResolvedValue({
      schemaVersion: 1, generatedAt: Date.now(), sessionId: "session-1", agent: "Codex", provider: "Provider A", model: "Model A", freshness: "live",
      metrics: Object.fromEntries(["inputTokens", "outputTokens", "cacheReadTokens", "cacheWriteTokens", "savedTokens", "avoidedTokens", "estimatedCostUsd", "estimatedSavingsUsd"].map((key) => [key, { value: key === "savedTokens" ? null : 10, confidence: key === "savedTokens" ? "unavailable" : "measured", source: "local runtime", observedAt: null, caveat: null }])),
      contextPressure: { usedTokens: 10, limitTokens: 100, percent: 10, projectedPercent: null, band: "low", limitSource: "local runtime", caveat: null }, sources: [], timeline: [], anomalies: [],
    });
  });

  it("shows provider attribution and honest provider-specific metric availability", async () => {
    render(<TokenXrayView hidden={false} />);
    expect(await screen.findByText(/Current session provider: Provider A/)).toBeInTheDocument();
    expect(screen.getByText(/Provider-specific breakdown: unavailable/)).toBeInTheDocument();
    expect(screen.getAllByText("Unavailable").length).toBeGreaterThan(1);
  });

  it("provides an accessible evidence refresh control", async () => {
    render(<TokenXrayView hidden={false} />);
    await screen.findByText(/Current session provider/);
    fireEvent.click(screen.getByRole("button", { name: "Refresh Token X-Ray evidence" }));
    await waitFor(() => expect(loadTokenXraySnapshot).toHaveBeenCalledTimes(2));
  });

  it("merges revisioned live updates without replacing source evidence", async () => {
    loadTokenXrayLiveUpdate.mockResolvedValueOnce({
      schemaVersion: 1, revision: 4, generatedAt: Date.now(), sessionId: "session-1", agent: "Codex", provider: "Provider A", model: "Model A", freshness: "live",
      metrics: { inputTokens: { value: 42, confidence: "measured", source: "live", observedAt: null, caveat: null } },
      contextPressure: { usedTokens: 42, limitTokens: 100, percent: 42, projectedPercent: null, band: "low", limitSource: "live", caveat: null }, timeline: [],
    });
    render(<TokenXrayView hidden={false} />);
    expect((await screen.findAllByText("42")).length).toBeGreaterThan(0);
    expect(loadTokenXrayLiveUpdate).toHaveBeenCalledWith(null);
  });
});
