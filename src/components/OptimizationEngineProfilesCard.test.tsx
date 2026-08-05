import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { OptimizationEngineProfilesCard } from "./OptimizationEngineProfilesCard";
import type { RuntimeStatus } from "../lib/types";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

describe("OptimizationEngineProfilesCard", () => {
  beforeEach(() => {
    invoke.mockReset();
    window.localStorage.clear();
    invoke.mockImplementation((command: string) => {
      if (command === "get_semantic_cache_status") {
        return Promise.resolve({
          enabled: false,
          semanticV2Enabled: false,
          entries: 0,
          hits: 0,
          misses: 0,
          databasePath: "/local/semantic-cache.sqlite3",
          policy: "exact-v1",
          disclosure: "Local exact replay only.",
          storageAvailable: true,
          readFailures: 0,
          writeFailures: 0,
          evidence: "local-observed-exact-replay",
        });
      }
      if (command === "get_semantic_cache_stats") {
        return Promise.resolve({ namespaces: [] });
      }
      if (command === "get_compression_profile") {
        return Promise.resolve({
          version: 1,
          presetId: "balanced",
          advanced: {},
          effectiveSavingsMode: "balanced",
          historyCompressionSupported: false,
          presets: [{ id: "balanced", label: "Balanced", description: "", savingsMode: "balanced" }],
          storagePath: "/tmp/profile.json",
        });
      }
      return Promise.resolve({
        profileId: "leanctx",
        configuration: [{ label: "Base URL", environmentVariable: "LEANCTX_BASE_URL", present: true }],
        executablePresent: false,
        pathPresent: false,
        connectivity: { attempted: false, status: "not-run", detail: "No connectivity preflight was run." },
        live: false,
        guidance: "Advisory presence only.",
      });
    });
  });

  it("renders all engine gates and keeps blocked engines disabled", () => {
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    expect(screen.getByText("Token optimization engines")).toBeInTheDocument();
    expect(screen.getByText("Lean Context")).toBeInTheDocument();
    expect(screen.getAllByText("Chonkify").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("PXPipe Text/Image")).toBeInTheDocument();
    expect(screen.getByLabelText("Headroom Native state: checking")).toBeInTheDocument();
    expect(screen.getByLabelText("Chonkify state: ready")).toBeInTheDocument();
    expect(screen.getByText(/Headroom Native is the only live provider compressor/)).toBeInTheDocument();
    expect(screen.getByText(/no upstream Headroom text_image capability/)).toBeInTheDocument();
    expect(screen.getAllByText(/Individual and master activation are safe no-ops/)).toHaveLength(3);
    expect(screen.getByRole("button", { name: "Activation unavailable for PXPipe Text/Image" })).toBeDisabled();
    expect(screen.getByRole("region", { name: "Token optimization evidence and promotion matrix" })).toBeInTheDocument();
    expect(screen.getAllByText("Design-only").length).toBe(2);
  });

  it("offers an accessible promotion-gate recheck without enabling blocked engines", async () => {
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Recheck token optimization promotion gates" }));
    expect(await screen.findByText(/Promotion gates rechecked at/)).toBeInTheDocument();
    expect(screen.getByLabelText("PXPipe Text/Image state: blocked")).toBeInTheDocument();
  });

  it("does not restore persisted experimental or blocked enablement", () => {
    window.localStorage.setItem("ai-switchboard.optimization-engines.v1", JSON.stringify({
      version: 1,
      enabled: { leanctx: true, chonkify: true, "pxpipe-text-image": true, "semantic-cache": true },
      receipts: [],
    }));
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    expect(screen.getByRole("button", { name: "Activation unavailable for Lean Context" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Enable local profile for Exact Replay Cache" })).toBeInTheDocument();
  });

  it("runs only the explicit leanctx readiness command", async () => {
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Check Lean Context readiness" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("get_optimization_addon_readiness", {
      profileId: "leanctx",
      runLocalConnectivity: false,
    }));
  });

  it("surfaces safety scope and blockers without implying live compression", () => {
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    expect(screen.getByText(/Headroom remains the sole provider proxy/)).toBeInTheDocument();
    expect(screen.getByText(/cache only eligible repeated text requests/)).toBeInTheDocument();
    expect(screen.getByText(/no upstream Headroom text_image capability/)).toBeInTheDocument();
    expect(screen.getByLabelText("PXPipe Text/Image state: blocked")).toBeInTheDocument();
  });

  it("reports readiness failures in an alert and keeps the card usable", async () => {
    invoke.mockImplementation(() => Promise.reject(new Error("readiness unavailable")));
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Check Lean Context readiness" }));
    expect((await screen.findAllByRole("alert")).some((element) => element.textContent?.includes("readiness unavailable"))).toBe(true);
    expect(screen.getAllByRole("button", { name: "View evidence" })[2]).toBeInTheDocument();
  });

  it("routes semantic-cache enablement to the backend command", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "get_semantic_cache_status") {
        return Promise.resolve({
          enabled: false,
          semanticV2Enabled: false,
          entries: 0,
          hits: 0,
          misses: 0,
          databasePath: "/local/semantic-cache.sqlite3",
          policy: "exact-v1",
          disclosure: "Local exact replay only.",
          storageAvailable: true,
          readFailures: 0,
          writeFailures: 0,
          evidence: "local-observed-exact-replay",
        });
      }
      if (command === "get_semantic_cache_stats") {
        return Promise.resolve({ namespaces: [] });
      }
      if (command === "set_addon_enabled") return Promise.resolve({});
      return Promise.resolve({
        profileId: "leanctx",
        configuration: [],
        executablePresent: false,
        pathPresent: false,
        connectivity: { attempted: false, status: "not-run", detail: "Not run." },
        live: false,
        guidance: "Advisory presence only.",
      });
    });
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    fireEvent.click(await screen.findByRole("button", { name: "Enable local profile for Exact Replay Cache" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_addon_enabled", { id: "semantic-cache", enabled: true }));
  });

  it("does not claim native Headroom is live when runtime health is down", () => {
    render(
      <OptimizationEngineProfilesCard
        onCopyGuidance={vi.fn()}
        runtimeStatus={{
          installed: true,
          running: false,
          proxyReachable: false,
          autoPaused: true,
          kompressEnabled: false,
        } as RuntimeStatus}
      />,
    );
    expect(screen.getByLabelText("Headroom Native state: paused")).toBeInTheDocument();
  });

  it("reports reachable Headroom without native compression as degraded", () => {
    render(
      <OptimizationEngineProfilesCard
        onCopyGuidance={vi.fn()}
        runtimeStatus={{
          installed: true,
          running: true,
          proxyReachable: true,
          autoPaused: false,
          kompressEnabled: false,
          rtk: { installed: false, enabled: false },
        } as RuntimeStatus}
      />,
    );
    expect(screen.getByLabelText("Headroom Native state: degraded")).toBeInTheDocument();
    fireEvent.click(screen.getAllByRole("button", { name: "View evidence" })[0]);
  });

  it("shows and copies lifecycle receipts without calling them savings", async () => {
    const copy = vi.fn();
    render(<OptimizationEngineProfilesCard onCopyGuidance={copy} />);
    const toggle = screen.getByRole("button", { name: "Enable local profile for Exact Replay Cache" });
    await waitFor(() => expect(toggle).not.toBeDisabled());
    fireEvent.click(toggle);
    await waitFor(() => expect(screen.queryByText(/No optimization lifecycle actions recorded yet\./)).not.toBeInTheDocument());
    expect(screen.getByText(/enabled · cache-hit · none evidence/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Copy receipts" }));
    expect(copy).toHaveBeenCalledWith(expect.stringContaining("configuration events"), "optimization lifecycle receipts");
  });
});
