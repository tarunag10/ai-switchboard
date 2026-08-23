import { describe, expect, it } from "vitest";
import {
  SELECTIVE_ACTIVATION_LIMIT,
  SELECTIVE_ACTIVATION_TOOLS,
  normalizeActivationRecovery,
  normalizeActivationSelection,
  validateActivationSelection,
} from "./activationTools";

describe("selective activation catalog", () => {
  it("defines ten unique selectable tools", () => {
    expect(SELECTIVE_ACTIVATION_TOOLS).toHaveLength(10);
    expect(new Set(SELECTIVE_ACTIVATION_TOOLS.map((tool) => tool.id)).size).toBe(10);
  });

  it("requires exactly five known unique tools", () => {
    const five = SELECTIVE_ACTIVATION_TOOLS.slice(0, SELECTIVE_ACTIVATION_LIMIT).map((tool) => tool.id);
    expect(validateActivationSelection(five)).toBeNull();
    expect(validateActivationSelection(five.slice(0, 4))).toMatch(/exactly five/);
    expect(validateActivationSelection([...five, "leanctx"])).toMatch(/exactly five/);
    expect(validateActivationSelection([five[0], five[0], ...five.slice(1, 4)])).toMatch(/duplicate/);
  });

  it("drops malformed persisted values without inventing tools", () => {
    expect(normalizeActivationSelection(["rtk", "rtk", "unknown", 4])).toEqual(["rtk"]);
  });

  it("accepts only a bounded native recovery view with exactly five tools", () => {
    const selectedToolIds = SELECTIVE_ACTIVATION_TOOLS.slice(0, 5).map((tool) => tool.id);
    const valid = {
      version: 1,
      runId: "selective-1720000000000-42",
      selectedToolIds,
      overallStatus: "partial",
      updatedAt: "2026-08-24T06:00:00Z",
      rollbackStatus: null,
      rollbackAvailable: true,
    };
    expect(normalizeActivationRecovery(valid)).toEqual(valid);
    expect(normalizeActivationRecovery({ ...valid, selectedToolIds: selectedToolIds.slice(0, 4) })).toBeNull();
    expect(normalizeActivationRecovery({ ...valid, selectedToolIds: [...selectedToolIds, selectedToolIds[0]] })).toBeNull();
    expect(normalizeActivationRecovery({ ...valid, selectedToolIds: [...selectedToolIds.slice(0, 4), "unknown"] })).toBeNull();
    expect(normalizeActivationRecovery({ ...valid, rollbackStatus: "automatic_retry" })).toBeNull();
    expect(normalizeActivationRecovery({ ...valid, rollbackStatus: "succeeded", rollbackAvailable: true })).toBeNull();
    expect(normalizeActivationRecovery({ ...valid, rollbackStatus: "partial", rollbackAvailable: true })).toBeNull();
    expect(normalizeActivationRecovery({ ...valid, rollbackStatus: "in_progress", rollbackAvailable: true })).toBeNull();
    expect(normalizeActivationRecovery({ ...valid, rollbackStatus: "partial", rollbackAvailable: false })).not.toBeNull();
    expect(normalizeActivationRecovery({ ...valid, ownedChanges: ["private"] })).toBeNull();
  });
});
