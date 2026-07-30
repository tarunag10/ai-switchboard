import { describe, expect, it } from "vitest";

import {
  canActivateLeanctxShadow,
  evaluateLeanctxPromotionGate,
  resolveMasterActivationLocalOptimizations,
  type LeanctxPromotionStatus,
} from "./leanctxPromotionGate";

const passingPromotion: LeanctxPromotionStatus = {
  status: "shadow_eligible",
  capabilityVersionOk: true,
  protectedContentOk: true,
  failOpenOk: true,
  shadowContractOk: true,
  livePromotionAllowed: false,
  reasons: ["shadow contract verified"],
};

describe("evaluateLeanctxPromotionGate", () => {
  it("blocks when promotion evidence is missing", () => {
    expect(evaluateLeanctxPromotionGate(null).verdict).toBe("blocked");
  });

  it("allows shadow when required evidence passes", () => {
    expect(evaluateLeanctxPromotionGate(passingPromotion).verdict).toBe(
      "shadow_eligible",
    );
    expect(canActivateLeanctxShadow(passingPromotion)).toBe(true);
  });

  it("never treats live promotion as master-activation safe without explicit allowlist", () => {
    const live: LeanctxPromotionStatus = {
      ...passingPromotion,
      livePromotionAllowed: true,
      status: "live_eligible",
    };
    expect(evaluateLeanctxPromotionGate(live).verdict).toBe("live_eligible");
    expect(canActivateLeanctxShadow(live)).toBe(true);
  });

  it("includes leanctx-shadow in master activation only when promotion passes", () => {
    expect(resolveMasterActivationLocalOptimizations(null)).toEqual(["semantic-cache"]);
    expect(resolveMasterActivationLocalOptimizations(passingPromotion)).toEqual([
      "semantic-cache",
      "leanctx-shadow",
    ]);
  });
});
