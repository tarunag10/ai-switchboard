import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TokenXrayView } from "./TokenXrayView";

const mocks = vi.hoisted(() => ({ load: vi.fn() }));
vi.mock("../lib/usageAnalytics", () => ({
  loadTokenXraySnapshot: mocks.load,
  formatMetric: (metric: { value: number | null }) => metric.value === null ? "Unavailable" : String(metric.value),
}));

const snapshot = {
  schemaVersion: 1, generatedAt: 0, sessionId: null, agent: null, provider: null, model: null, freshness: "saved",
  metrics: {},
  contextPressure: { usedTokens: null, limitTokens: null, percent: null, projectedPercent: null, band: "critical", limitSource: "unknown", caveat: "No limit" },
  sources: [], timeline: [], anomalies: [],
};

describe("TokenXrayView alternate states", () => {
  beforeEach(() => mocks.load.mockReset());

  it("shows unavailable evidence without manufacturing metrics", async () => {
    mocks.load.mockResolvedValue({ ...snapshot, freshness: "unavailable" });
    render(<TokenXrayView hidden={false} />);
    expect(await screen.findByText("No live session telemetry yet")).toBeVisible();
  });

  it("reports load errors and retries", async () => {
    const user = userEvent.setup();
    mocks.load.mockRejectedValueOnce(new Error("telemetry unreadable")).mockResolvedValueOnce(snapshot);
    render(<TokenXrayView hidden={false} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("telemetry unreadable");
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByText("Agent unavailable")).toBeVisible();
    expect(mocks.load).toHaveBeenCalledTimes(2);
  });

  it("renders critical unknown pressure, empty impact/timeline, and provenance", async () => {
    const user = userEvent.setup();
    mocks.load.mockResolvedValue(snapshot);
    const { container } = render(<TokenXrayView hidden={false} />);
    expect(await screen.findByText("Absolute evidence only")).toBeVisible();
    expect(screen.getByText("No source-level impact has been attributed to this session.")).toBeVisible();
    expect(screen.getByText(/No material usage/)).toBeVisible();
    expect(container.querySelector(".repo-map-hero--warning")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Show provenance and metric caveats" }));
    expect(screen.getByText(/Measured values come from local runtime evidence/)).toBeVisible();
    expect(screen.getByRole("button", { name: "Hide provenance and metric caveats" })).toHaveAttribute("aria-expanded", "true");
  });
});
