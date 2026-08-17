import { describe, expect, it } from "vitest";

import { canPromotePxpipeExperimental, evaluatePxpipePromotionGate } from "./pxpipePromotionGate";

describe("pxpipePromotionGate", () => {
  it("keeps the pinned complete fixture experimental-eligible", () => {
    expect(canPromotePxpipeExperimental()).toBe(false);
    expect(evaluatePxpipePromotionGate().verdict).toBe("blocked");
    expect(evaluatePxpipePromotionGate({
      schemaVersion: 1,
      headroomCapability: "text_image",
      minHeadroomVersion: "1.0.0",
      visualQualityChecklistSigned: true,
      requiredSignals: ["visual-quality"],
    }).verdict).toBe("experimental_eligible");
  });

  it("fails closed with all missing provenance reasons", () => {
    const result = evaluatePxpipePromotionGate({
      schemaVersion: 1,
      headroomCapability: "",
      minHeadroomVersion: "",
      visualQualityChecklistSigned: false,
      requiredSignals: [],
    });
    expect(result.verdict).toBe("blocked");
    expect(result.reasons).toHaveLength(3);
  });
});
