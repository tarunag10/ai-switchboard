import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { OptimizationEngineProfilesCard } from "./OptimizationEngineProfilesCard";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

const cacheStatus = {
  enabled: true, semanticV2Enabled: false, entries: 3, hits: 4, misses: 2,
  databasePath: "/tmp/cache.sqlite", policy: "exact-v1", disclosure: "Local only",
  storageAvailable: true, readFailures: 0, writeFailures: 0, evidence: "local-observed-exact-replay",
};
const cacheStats = { namespaces: [{ provider: "openai", model: "gpt-test", account: "local", workspace: "repo", policy: "exact-v1", hits: 4, misses: 2, entries: 3, lastHitAt: null }] };
const diagnostics = { exactMatchOnly: true, storagePath: "/tmp/cache.sqlite", bypassRules: ["streaming"], safetyRules: ["sha256_request_key"], clearAction: { confirmationPhrase: "clear exact response cache" } };
const compressionProfile = {
  version: 1, presetId: "balanced", advanced: { compressUserMessages: true, compressToolResults: true, compressHistory: false, outputShaper: false },
  effectiveSavingsMode: "balanced", historyCompressionSupported: false,
  presets: [
    { id: "balanced", label: "Balanced", description: "Balanced defaults", savingsMode: "balanced" },
    { id: "maximum", label: "Maximum", description: "Maximum savings", savingsMode: "maximum" },
  ], storagePath: "/tmp/profile.json",
};

function setupInvoke(overrides: Record<string, unknown> = {}) {
  invokeMock.mockImplementation((command: string) => {
    if (command in overrides) {
      const value = overrides[command];
      return value instanceof Error ? Promise.reject(value) : Promise.resolve(value);
    }
    if (command === "get_semantic_cache_status") return Promise.resolve(cacheStatus);
    if (command === "get_semantic_cache_stats") return Promise.resolve(cacheStats);
    if (command === "get_response_cache_diagnostics") return Promise.resolve(diagnostics);
    if (command === "get_compression_profile") return Promise.resolve(compressionProfile);
    if (command === "set_compression_profile" || command === "clear_compression_profile_command") return Promise.resolve(compressionProfile);
    if (command === "get_leanctx_sidecar_status") return Promise.resolve({ configured: false, enabled: false, running: false, mode: "off", health: "missing", executablePresent: false, loopbackOnly: true, ownership: "User supplied", error: null });
    return Promise.resolve(undefined);
  });
}

function engine(label: string) {
  const title = screen.getByText(label, { selector: "strong" });
  const section = title.closest("section.gateway-profile");
  if (!section) throw new Error(`Missing engine ${label}`);
  return within(section as HTMLElement);
}

describe("OptimizationEngineProfilesCard guarded actions", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.restoreAllMocks();
    window.localStorage.clear();
  });

  it("clears one exact-cache namespace and all responses only with exact phrases", async () => {
    const user = userEvent.setup();
    setupInvoke();
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    const cache = engine("Exact Replay Cache");
    await waitFor(() => expect(cache.getByRole("button", { name: "Refresh Exact Replay Cache status" })).toBeEnabled());
    await user.click(cache.getByRole("button", { name: "View evidence" }));

    const namespaceButton = cache.getByRole("button", { name: "Clear first namespace" });
    expect(namespaceButton).toBeDisabled();
    await user.type(cache.getByPlaceholderText("clear cache namespace"), "clear cache namespace");
    await user.click(namespaceButton);
    expect(invokeMock).toHaveBeenCalledWith("clear_semantic_cache_namespace", { request: {
      provider: "openai", model: "gpt-test", account: "local", workspace: "repo", policy: "exact-v1", confirmationPhrase: "clear cache namespace",
    } });

    const clearAll = cache.getByRole("button", { name: "Clear cached responses" });
    await user.type(cache.getByPlaceholderText("clear exact response cache"), "clear exact response cache");
    await user.click(clearAll);
    expect(invokeMock).toHaveBeenCalledWith("clear_response_cache", { confirmationPhrase: "clear exact response cache" });
  });

  it("surfaces cache-clear command failures and retains evidence controls", async () => {
    const user = userEvent.setup();
    setupInvoke({ clear_response_cache: new Error("cache locked") });
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} />);
    const cache = engine("Exact Replay Cache");
    await user.click(cache.getByRole("button", { name: "View evidence" }));
    await user.type(await cache.findByPlaceholderText("clear exact response cache"), "clear exact response cache");
    await user.click(cache.getByRole("button", { name: "Clear cached responses" }));
    expect(await cache.findByRole("alert")).toHaveTextContent("cache locked");
  });

  it("applies and resets Headroom compression profiles with restart confirmation", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    setupInvoke();
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} runtimeStatus={{ running: true, proxyReachable: true, kompressEnabled: true, rtk: { installed: true, enabled: true } } as any} />);
    const headroom = engine("Headroom Native");
    await user.click(headroom.getByRole("button", { name: "View evidence" }));
    await user.click(await headroom.findByRole("button", { name: "Maximum" }));
    expect(confirm).toHaveBeenCalledWith(expect.stringContaining("proxy restarts"));
    expect(invokeMock).toHaveBeenCalledWith("set_compression_profile", {
      presetId: "maximum", advanced: compressionProfile.advanced, restartHeadroom: true,
    });

    await user.click(headroom.getByRole("button", { name: "Show advanced toggles" }));
    await user.click(headroom.getByRole("button", { name: "Reset to defaults" }));
    expect(invokeMock).toHaveBeenCalledWith("clear_compression_profile_command", { restartHeadroom: true });
    expect(headroom.getByText(/History compression is unavailable/)).toBeInTheDocument();
  });

  it("cancels profile application without invoking mutation and reports load failure", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "confirm").mockReturnValue(false);
    setupInvoke();
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} runtimeStatus={{ running: true, proxyReachable: true, kompressEnabled: true, rtk: { installed: false, enabled: false } } as any} />);
    const headroom = engine("Headroom Native");
    await user.click(headroom.getByRole("button", { name: "View evidence" }));
    await user.click(await headroom.findByRole("button", { name: "Maximum" }));
    expect(invokeMock).not.toHaveBeenCalledWith("set_compression_profile", expect.anything());
  });

  it("runs loopback readiness and copies bounded engine previews", async () => {
    const user = userEvent.setup();
    const copy = vi.fn();
    setupInvoke({ get_optimization_addon_readiness: {
      guidance: "Loopback safe", live: false, executablePresent: true,
      connectivity: { status: "reachable", attempted: true, detail: "Loopback responded" },
      configuration: [{ label: "Base URL", environmentVariable: "LEANCTX_BASE_URL", present: true }],
    } });
    render(<OptimizationEngineProfilesCard onCopyGuidance={copy} />);
    const leanctx = engine("Lean Context");
    await user.click(leanctx.getByRole("button", { name: "Run Lean Context loopback preflight" }));
    expect(invokeMock).toHaveBeenCalledWith("get_optimization_addon_readiness", { profileId: "leanctx", runLocalConnectivity: true });
    expect((await leanctx.findAllByText(/Loopback safe/)).length).toBeGreaterThan(0);
    await user.click(leanctx.getByRole("button", { name: "Copy preview" }));
    expect(copy).toHaveBeenCalledWith(expect.stringContaining("Switchboard preview only"), "Lean Context preview");
  });

  it("applies advanced compression toggles with the current preset", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "confirm").mockReturnValue(true);
    setupInvoke();
    render(<OptimizationEngineProfilesCard onCopyGuidance={vi.fn()} runtimeStatus={{ running: true, proxyReachable: true, kompressEnabled: true, rtk: { installed: true, enabled: true } } as any} />);
    const headroom = engine("Headroom Native");
    await user.click(headroom.getByRole("button", { name: "View evidence" }));
    await user.click(await headroom.findByRole("button", { name: "Show advanced toggles" }));
    await user.click(headroom.getByRole("checkbox", { name: "Output shaper" }));
    expect(invokeMock).toHaveBeenCalledWith("set_compression_profile", {
      presetId: "balanced", advanced: { ...compressionProfile.advanced, outputShaper: true }, restartHeadroom: true,
    });
  });
});
