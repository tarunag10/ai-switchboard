import { describe, expect, it } from "vitest";

import {
  assertMaxCompressionAllowlistMatchesRegistry,
  createMaxCompressionActivationPlan,
  createMaxCompressionLifecycleReceipts,
  MAX_COMPRESSION_EXCLUDED_ENGINE_IDS,
  resolveMaxCompressionActivatableEngines,
} from "./maxCompressionActivation";
import { filterActivatableOptimizationEngineIds } from "./optimizationEngines";

describe("maxCompressionActivation", () => {
  it("allowlist matches filterActivatableOptimizationEngineIds", () => {
    const engines = resolveMaxCompressionActivatableEngines();
    expect(engines).toEqual(
      filterActivatableOptimizationEngineIds([
        "headroom-native",
        "semantic-cache",
        "rtk",
      ]),
    );
    expect(engines).toEqual(["headroom-native", "semantic-cache", "rtk"]);
  });

  it("never includes experimental or blocked engines", () => {
    for (const id of MAX_COMPRESSION_EXCLUDED_ENGINE_IDS) {
      expect(resolveMaxCompressionActivatableEngines()).not.toContain(id);
    }
    expect(assertMaxCompressionAllowlistMatchesRegistry()).toEqual([]);
  });

  it("plans full mode, cache when recommended, RTK, and repo index prompt", () => {
    const plan = createMaxCompressionActivationPlan({
      mode: "full",
      proxyReachable: true,
      semanticCacheEnabled: false,
    });

    expect(plan.steps.map((step) => step.id)).toEqual([
      "full-mode",
      "headroom-native",
      "semantic-cache",
      "rtk",
      "repo-index",
    ]);
    expect(plan.excludedCopy).toContain("leanctx");
    expect(plan.excludedCopy).toContain("chonkify");
    expect(plan.suggestDoctorRerun).toBe(true);
  });

  it("skips exact cache when already enabled or not recommended", () => {
    const enabled = createMaxCompressionActivationPlan({
      mode: "full",
      proxyReachable: true,
      semanticCacheEnabled: true,
    });
    expect(enabled.steps.some((step) => step.id === "semantic-cache")).toBe(
      false,
    );

    const rtkOnly = createMaxCompressionActivationPlan({
      mode: "rtk",
      proxyReachable: false,
      semanticCacheEnabled: false,
    });
    expect(rtkOnly.steps.some((step) => step.id === "semantic-cache")).toBe(
      false,
    );
  });

  it("records lifecycle receipts for each enabled engine", () => {
    const plan = createMaxCompressionActivationPlan({
      mode: "full",
      proxyReachable: true,
    });
    const receipts = createMaxCompressionLifecycleReceipts(plan);
    expect(receipts.map((receipt) => receipt.engine)).toEqual(plan.engines);
    expect(receipts.every((receipt) => receipt.createdAt)).toBe(true);
  });
});
