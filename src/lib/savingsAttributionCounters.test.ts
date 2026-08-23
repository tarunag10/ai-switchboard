import { beforeEach, describe, expect, it, vi } from "vitest";
import { loadSavingsAttributionCounters } from "./savingsAttributionCounters";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("savings attribution counters", () => {
  beforeEach(() => invoke.mockReset());

  it("loads the compact native source counters", async () => {
    invoke.mockResolvedValue([]);
    await expect(loadSavingsAttributionCounters()).resolves.toEqual([]);
    expect(invoke).toHaveBeenCalledWith("get_savings_attribution_counters");
  });
});
