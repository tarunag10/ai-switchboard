import { beforeEach, describe, expect, it, vi } from "vitest";
import { loadClaudeUsage, normalizeClaudeUsage } from "./claudeUsage";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("Claude usage normalization", () => {
  beforeEach(() => invoke.mockReset());

  it("normalizes camelCase and snake_case usage windows without inventing values", () => {
    expect(normalizeClaudeUsage({ five_hour: { utilization: 42.5, resets_at: "2026-08-23T12:00:00Z" }, extra_usage: { is_enabled: false } })).toEqual({
      fiveHour: { utilization: 42.5, resetsAt: "2026-08-23T12:00:00Z" },
      sevenDay: null,
      extraUsage: { isEnabled: false, monthlyLimit: null, usedCredits: null, utilization: null },
    });
  });

  it("loads from the native command", async () => {
    invoke.mockResolvedValue({ fiveHour: null, sevenDay: null, extraUsage: null });
    await expect(loadClaudeUsage()).resolves.toEqual({ fiveHour: null, sevenDay: null, extraUsage: null });
    expect(invoke).toHaveBeenCalledWith("get_claude_usage");
  });
});
