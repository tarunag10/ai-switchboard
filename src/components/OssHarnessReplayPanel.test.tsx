import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { OssHarnessReplayPanel } from "./OssHarnessReplayPanel";

const open = vi.fn();
const replay = vi.fn();

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: (...args: unknown[]) => open(...args) }));
vi.mock("../lib/ossHarnessReplay", () => ({
  replayRedactedRouteEvents: (...args: unknown[]) => replay(...args),
}));

describe("OssHarnessReplayPanel", () => {
  it("loads and labels a local replay as observe-only", async () => {
    open.mockResolvedValue("/tmp/replay.json");
    replay.mockResolvedValue({
      result: {
        schemaVersion: 1,
        replayMode: "redacted_observe_only",
        automaticPromotion: "disabled",
        providerTraffic: "none",
        eventCount: 2,
        routeCounts: { headroom: 2 },
        outcomeCounts: { success: 2 },
        latency: { sampleCount: 2, p95Ms: 120 },
        replayDigest: `sha256:${"a".repeat(64)}`,
      },
      reference: {
        schemaVersion: 1,
        replayId: "replay-reference-1",
        validatedAt: "2026-08-23T00:00:00Z",
        replayMode: "redacted_observe_only",
        automaticPromotion: "disabled",
        providerTraffic: "none",
        eventCount: 2,
        replayDigest: `sha256:${"a".repeat(64)}`,
        receiptDigest: `sha256:${"b".repeat(64)}`,
      },
    });
    const user = userEvent.setup();
    render(<OssHarnessReplayPanel />);
    await user.click(screen.getByRole("button", { name: /choose replay json/i }));
    expect(replay).toHaveBeenCalledWith("/tmp/replay.json");
    expect(await screen.findByText(/provider traffic: none/i)).toBeInTheDocument();
    expect(screen.getByText(/replay receipt: replay-reference-1/i)).toBeInTheDocument();
    expect(screen.queryByText("/tmp/replay.json")).not.toBeInTheDocument();
  });

  it("surfaces rejected replay files without showing stale results", async () => {
    open.mockResolvedValue("/tmp/unsafe.json");
    replay.mockRejectedValue(new Error("sensitive field is not allowed"));
    const user = userEvent.setup();
    render(<OssHarnessReplayPanel />);
    await user.click(screen.getByRole("button", { name: /choose replay json/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/sensitive field/i);
  });
});
