import { describe, expect, it } from "vitest";
import {
  optimizationEngineIds,
  optimizationEngines,
  previewOptimizationEngineConfig,
  summarizeOptimizationEngineStatus,
  validateOptimizationEngineGovernance,
} from "./optimizationEngines";

describe("optimization engine registry", () => {
  it("contains the complete disjoint engine set", () => {
    expect(optimizationEngines.map((engine) => engine.id)).toEqual([...optimizationEngineIds]);
    expect(new Set(optimizationEngineIds).size).toBe(optimizationEngineIds.length);
  });

  it("passes governance and lifecycle validation", () => {
    expect(validateOptimizationEngineGovernance()).toEqual([]);
    for (const engine of optimizationEngines) {
      expect(engine.setup).toBeTruthy();
      expect(engine.rollback).toBeTruthy();
      expect(engine.off).toBeTruthy();
    }
  });

  it("summarizes statuses without extra registry detail", () => {
    const summary = summarizeOptimizationEngineStatus();
    expect(summary).toContain("headroom-native: enabled");
    expect(summary).toContain("pxpipe-text-image: blocked");
    expect(summary).not.toContain("config");
  });

  it("redacts secrets and omits complex config values", () => {
    const preview = previewOptimizationEngineConfig(optimizationEngines[0], {
      apiKey: "do-not-leak",
      endpoint: "http://127.0.0.1:6767",
      nested: { secret: "hidden" },
    });
    expect(preview).toEqual({ apiKey: "[redacted]", endpoint: "http://127.0.0.1:6767", nested: "[omitted]" });
    expect(JSON.stringify(preview)).not.toContain("do-not-leak");
  });
});
