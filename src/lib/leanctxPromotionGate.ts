import { filterActivatableOptimizationEngineIds } from "./optimizationEngines";

export interface LeanctxPromotionStatus {
  status: string;
  capabilityVersion?: string | null;
  capabilityVersionOk: boolean;
  protectedContentOk: boolean;
  failOpenOk: boolean;
  shadowContractOk: boolean;
  livePromotionAllowed: boolean;
  reasons: string[];
}

export type LeanctxPromotionGateVerdict = "blocked" | "shadow_eligible" | "live_eligible";

export function evaluateLeanctxPromotionGate(
  promotion: LeanctxPromotionStatus | null | undefined,
): {
  verdict: LeanctxPromotionGateVerdict;
  reasons: string[];
} {
  if (!promotion) {
    return {
      verdict: "blocked",
      reasons: ["leanctx promotion evidence is unavailable"],
    };
  }

  if (promotion.livePromotionAllowed) {
    return {
      verdict: "live_eligible",
      reasons: promotion.reasons,
    };
  }

  const shadowEligible =
    promotion.capabilityVersionOk &&
    promotion.protectedContentOk &&
    promotion.failOpenOk &&
    promotion.shadowContractOk;

  if (shadowEligible) {
    return {
      verdict: "shadow_eligible",
      reasons: promotion.reasons,
    };
  }

  return {
    verdict: "blocked",
    reasons:
      promotion.reasons.length > 0
        ? promotion.reasons
        : [
            "capability/version evidence missing",
            "protected-content coverage missing",
            "fail-open behavior missing",
            "shadow contract missing",
          ],
  };
}

export function canActivateLeanctxShadow(
  promotion: LeanctxPromotionStatus | null | undefined,
): boolean {
  const { verdict } = evaluateLeanctxPromotionGate(promotion);
  return verdict === "shadow_eligible" || verdict === "live_eligible";
}

export const MASTER_ACTIVATION_LEANCTX_SHADOW_ID = "leanctx-shadow";

/** Master activation allowlist: semantic-cache plus leanctx-shadow when promotion evidence passes. */
export function resolveMasterActivationLocalOptimizations(
  leanctxPromotion?: LeanctxPromotionStatus | null,
): string[] {
  const optimizers = [...filterActivatableOptimizationEngineIds(["semantic-cache"])];
  if (canActivateLeanctxShadow(leanctxPromotion)) {
    optimizers.push(MASTER_ACTIVATION_LEANCTX_SHADOW_ID);
  }
  return optimizers;
}
