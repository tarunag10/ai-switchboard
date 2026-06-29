import type {
  BillingPeriod,
  HeadroomPricingStatus,
  HeadroomSubscriptionTier,
  TierRecommendationSource,
} from "./types";

export type PricingAudience = "individual" | "teamEnterprise";
export type { BillingPeriod };

const PLAN_PRICES: Record<
  "pro" | "max5x" | "max20x",
  Record<BillingPeriod, { full: string; fullCents: number }>
> = {
  pro:   { annual: { full: "$5",  fullCents: 500  }, monthly: { full: "$7.50", fullCents: 750  } },
  max5x: { annual: { full: "$20", fullCents: 2000 }, monthly: { full: "$30",   fullCents: 3000 } },
  max20x:{ annual: { full: "$40", fullCents: 4000 }, monthly: { full: "$60",   fullCents: 6000 } },
};

// Discounted price for a tier, mirroring the web `sale_price_cents` rounding
// (half-up to the cent) so desktop and marketing prices never disagree.
function discountedPriceLabel(fullCents: number, percentOff: number): string {
  return formatCents(Math.round((fullCents * (100 - percentOff)) / 100));
}
const TIER_RANK: Record<HeadroomSubscriptionTier, number> = { pro: 1, max5x: 2, max20x: 3 };

export function isTierDowngrade(
  fromTier: HeadroomSubscriptionTier,
  toTier: HeadroomSubscriptionTier
): boolean {
  return TIER_RANK[toTier] < TIER_RANK[fromTier];
}

function projectPerMonthCents(
  toTier: HeadroomSubscriptionTier,
  billingPeriod: BillingPeriod,
  options?: { fromTier?: HeadroomSubscriptionTier; currentPaidCents?: number | null }
): number {
  // PLAN_PRICES.fullCents is per-month even on annual cycles.
  const toFullPerMonth = PLAN_PRICES[toTier][billingPeriod].fullCents;
  const fromTier = options?.fromTier;
  const currentPaidCents = options?.currentPaidCents ?? null;
  if (!fromTier || currentPaidCents === null) return toFullPerMonth;
  const fromFullPerMonth = PLAN_PRICES[fromTier][billingPeriod].fullCents;
  if (fromFullPerMonth <= 0) return toFullPerMonth;
  // Polar reports subscription_amount_cents per full billing cycle (12x
  // per-month for annual), so normalize to per-month before the ratio math.
  const cycleMonths = billingPeriod === "annual" ? 12 : 1;
  const currentPaidPerMonth = currentPaidCents / cycleMonths;
  return Math.round(toFullPerMonth * (currentPaidPerMonth / fromFullPerMonth));
}

function formatCents(cents: number): string {
  const dollars = cents / 100;
  return cents % 100 === 0 ? `$${dollars}` : `$${dollars.toFixed(2)}`;
}

/// Per-month price label for the target tier (e.g. `$20 / month`), with the
/// user's current discount ratio carried forward. Matches the upgrade view
/// convention where annual prices are shown per-month for tier comparison.
export function getPlanRenewalPriceLabel(
  toTier: HeadroomSubscriptionTier,
  billingPeriod: BillingPeriod,
  options?: { fromTier?: HeadroomSubscriptionTier; currentPaidCents?: number | null }
): string {
  return `${formatCents(projectPerMonthCents(toTier, billingPeriod, options))} / month`;
}

/// Founder-promo step prices for the plan matched to the user's Claude/Codex
/// tier: the current discounted price (`now`) and the price at the next cohort's
/// percent (`next`). Returns null for plans without a fixed price (free / team /
/// enterprise) so the promo can fall back to percent-only chips.
export function getFounderStepPricing(
  planId: UpgradePlanId,
  billingPeriod: BillingPeriod,
  nowPercentOff: number,
  nextPercentOff: number
): { now: string; next: string } | null {
  if (planId !== "pro" && planId !== "max5x" && planId !== "max20x") return null;
  const fullCents = PLAN_PRICES[planId][billingPeriod].fullCents;
  return {
    now: discountedPriceLabel(fullCents, nowPercentOff),
    next: discountedPriceLabel(fullCents, nextPercentOff),
  };
}

export type UpgradePlanId = "free" | "pro" | "max5x" | "max20x" | "team" | "enterprise";
type IndividualUpgradePlanId = "free" | "pro" | "max5x" | "max20x";
type PaidUpgradePlanId = HeadroomSubscriptionTier;

const INDIVIDUAL_PLAN_ORDER: IndividualUpgradePlanId[] = ["free", "pro", "max5x", "max20x"];

export interface UpgradePlanPurchaseInfo {
  renewsOn: string;
  paidPerMonthLabel: string;
  discountPct: number;
  cancelAtPeriodEnd?: boolean;
  endsOn?: string;
}

export interface UpgradePlan {
  id: UpgradePlanId;
  name: string;
  tagline: string;
  price: string;
  originalPrice?: string;
  billingLines: [string, string];
  centeredPriceLabel?: string;
  featureIntro: string;
  features: string[];
  ctaLabel: string;
  ctaVariant: "primary" | "secondary";
  ctaTone?: "default" | "downgrade";
  disabled?: boolean;
  purchaseInfo?: UpgradePlanPurchaseInfo;
}

export function upgradePlanIntentLabel(planId: UpgradePlanId | null) {
  switch (planId) {
    case "pro":
      return "Pro";
    case "max5x":
      return "Max x5";
    case "max20x":
      return "Max x20";
    default:
      return null;
  }
}

// Connector(s) whose detected plan drives a tier-mismatch recommendation, for
// the upgrade banner copy.
export function tierRecommendationSourceLabel(source: TierRecommendationSource) {
  switch (source) {
    case "codex":
      return "Codex";
    case "both":
      return "Claude and Codex";
    default:
      return "Claude";
  }
}

export function describeInvokeError(error: unknown, fallback: string) {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string" &&
    error.message.trim()
  ) {
    return error.message;
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "error" in error &&
    typeof error.error === "string" &&
    error.error.trim()
  ) {
    return error.error;
  }
  return fallback;
}

export type RuntimeCalloutTone =
  | "auto-paused"
  | "degraded"
  | "disabled"
  | "disconnected"
  | "healthy"
  | "paused"
  | "starting";

export function shouldOfferRuntimeRestartAction(
  tone: RuntimeCalloutTone,
  options: {
    runtimeHealthy: boolean;
    runtimeStarting?: boolean;
    connectorPhase?: "disabled" | "healthy" | "verifying";
  },
): boolean {
  if (options.runtimeStarting || options.connectorPhase === "verifying") {
    return false;
  }
  if (tone === "auto-paused" || tone === "paused") {
    return true;
  }
  return !options.runtimeHealthy && (tone === "disconnected" || tone === "degraded");
}

export function getNextLowerUpgradePlanId(
  planId?: PaidUpgradePlanId | null
): IndividualUpgradePlanId | null {
  switch (planId) {
    case "pro":
      return "free";
    case "max5x":
      return "pro";
    case "max20x":
      return "max5x";
    default:
      return null;
  }
}

export function getUpgradePlans(
  audience: PricingAudience,
  claudePlanTier?: HeadroomPricingStatus["claude"]["planTier"],
  recommendedSubscriptionTier?: HeadroomPricingStatus["recommendedSubscriptionTier"],
  headroomSubscriptionTier?: HeadroomSubscriptionTier | null,
  hasActiveHeadroomSubscription = false,
  launchDiscountActive = false,
  billingPeriod: BillingPeriod = "annual",
  subscriptionAmountCents?: number | null,
  subscriptionBillingPeriod?: string | null,
  subscriptionRenewsAt?: string | null,
  subscriptionStartedAt?: string | null,
  subscriptionDiscountDuration?: string | null,
  subscriptionDiscountDurationInMonths?: number | null,
  subscriptionCancelAtPeriodEnd: boolean = false,
  subscriptionEndsAt?: string | null,
  activePercentOff: number = 0
): {
  plans: UpgradePlan[];
  featuredPlanId: UpgradePlanId;
} {
  if (audience === "individual") {
    const downgradeEndsOn = subscriptionCancelAtPeriodEnd && subscriptionEndsAt
      ? new Date(subscriptionEndsAt).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })
      : undefined;

    const freePlan: UpgradePlan = {
      id: "free",
      name: "Free",
      tagline: "Limited usage with Claude or Codex",
      price: "$0",
      billingLines: ["/ month", "free"],
      featureIntro: "Includes:",
      features: [
        "Unlocks cost savings and stats",
        "Use with 50% of your Claude or Codex plan",
        "Optimize Claude Code or Codex"
      ],
      ctaLabel: downgradeEndsOn ? "Downgrade scheduled" : "Stay on Free plan",
      ctaVariant: "secondary",
      ctaTone: "default",
      ...(downgradeEndsOn
        ? {
            purchaseInfo: {
              renewsOn: downgradeEndsOn,
              paidPerMonthLabel: "$0",
              discountPct: 0,
              cancelAtPeriodEnd: true,
              endsOn: downgradeEndsOn
            }
          }
        : {})
    };

    const billingLabel = billingPeriod === "annual" ? "billed annually" : "billed monthly";

    const activeHeadroomPlanId =
      hasActiveHeadroomSubscription && headroomSubscriptionTier
        ? headroomSubscriptionTier
        : null;

    // Compute purchase info for the active plan card when data is available.
    const activePurchaseInfo = ((): UpgradePlanPurchaseInfo | undefined => {
      if (!activeHeadroomPlanId || subscriptionAmountCents == null) {
        return undefined;
      }
      const purchasePeriod = (subscriptionBillingPeriod === "annual" || subscriptionBillingPeriod === "monthly")
        ? subscriptionBillingPeriod
        : billingPeriod;
      const fullCents = PLAN_PRICES[activeHeadroomPlanId][purchasePeriod].fullCents;

      // Determine if the discount will still apply at renewal time.
      const discountAppliesAtRenewal = ((): boolean => {
        if (!subscriptionDiscountDuration) return false;
        if (subscriptionDiscountDuration === "forever") return true;
        if (subscriptionDiscountDuration === "once") return false;
        // "repeating": check if renewal falls within the discount window
        if (
          subscriptionDiscountDuration === "repeating" &&
          subscriptionDiscountDurationInMonths != null &&
          subscriptionStartedAt &&
          subscriptionRenewsAt
        ) {
          const discountExpiresAt = new Date(subscriptionStartedAt);
          discountExpiresAt.setMonth(discountExpiresAt.getMonth() + subscriptionDiscountDurationInMonths);
          return new Date(subscriptionRenewsAt) < discountExpiresAt;
        }
        return false;
      })();

      // Amount is stored as per-billing-cycle cents; convert to per-month.
      const paidCentsPerMonth = purchasePeriod === "annual"
        ? subscriptionAmountCents / 12
        : subscriptionAmountCents;

      // If the discount won't apply at renewal, show full price for the renewal.
      const renewalCentsPerMonth = discountAppliesAtRenewal ? paidCentsPerMonth : fullCents;
      const discountPct = discountAppliesAtRenewal && fullCents > 0
        ? Math.round((1 - paidCentsPerMonth / fullCents) * 100)
        : 0;
      const paidPerMonthLabel = `$${(renewalCentsPerMonth / 100).toFixed(2).replace(/\.00$/, "")}`;
      const renewsOn = subscriptionRenewsAt
        ? new Date(subscriptionRenewsAt).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })
        : null;
      if (!renewsOn) return undefined;
      const endsOn = subscriptionCancelAtPeriodEnd && subscriptionEndsAt
        ? new Date(subscriptionEndsAt).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })
        : undefined;
      return {
        renewsOn,
        paidPerMonthLabel,
        discountPct,
        cancelAtPeriodEnd: subscriptionCancelAtPeriodEnd,
        endsOn
      };
    })();

    function paidPlan(
      id: "pro" | "max5x" | "max20x",
      name: string,
      tagline: string,
      featureIntro: string,
      features: string[],
      ctaLabel: string
    ): UpgradePlan {
      const prices = PLAN_PRICES[id][billingPeriod];
      // Upgrade-target cards show the discounted price because checkout always
      // attaches the active cohort discount; the active plan card uses
      // purchaseInfo (actual paid amount) instead of a generic discount badge.
      // Percent is driven live by the cohort ladder; fall back to 50% for legacy
      // callers that only signal launchDiscountActive without a percent.
      const effectivePercentOff = activePercentOff > 0 ? activePercentOff : 50;
      const showDiscount = launchDiscountActive && id !== activeHeadroomPlanId;
      const price = showDiscount
        ? discountedPriceLabel(prices.fullCents, effectivePercentOff)
        : prices.full;
      return {
        id,
        name,
        tagline,
        price,
        ...(showDiscount ? { originalPrice: prices.full } : {}),
        ...(id === activeHeadroomPlanId && activePurchaseInfo ? { purchaseInfo: activePurchaseInfo } : {}),
        billingLines: ["USD / month", billingLabel],
        featureIntro,
        features,
        ctaLabel,
        ctaVariant: "primary",
        ctaTone: "default"
      };
    }

    const paidPlans: Record<"pro" | "max5x" | "max20x", UpgradePlan> = {
      pro: paidPlan("pro", "Pro", "Unlock unlimited savings", "Everything in Free, plus:", [
        "Unlimited use with Claude Pro or ChatGPT Plus",
        "Use on all your devices with one account",
        "Email-based support"
      ], "Get Pro"),
      max5x: paidPlan("max5x", "Max x5", "For Claude Max x5 or ChatGPT Pro x5 accounts", "Includes:", [
        "Unlimited use with Claude Max x5 or ChatGPT Pro x5",
        "Use on all your devices with one account",
        "Email-based support"
      ], "Get Max x5"),
      max20x: paidPlan("max20x", "Max x20", "For Claude Max x20 or ChatGPT Pro x20 accounts", "Includes:", [
        "Unlimited use with Claude Max x20 or ChatGPT Pro x20",
        "Use on all your devices with one account",
        "Priority support"
      ], "Get Max x20"),
    };

    const withRelativeCta = (plan: UpgradePlan): UpgradePlan => {
      if (!activeHeadroomPlanId) {
        return plan;
      }

      // Free card during a scheduled downgrade is the pending target — its
      // CTA was set to "Downgrade scheduled" above and must not be overridden.
      if (plan.purchaseInfo?.cancelAtPeriodEnd && plan.id !== activeHeadroomPlanId) {
        return plan;
      }

      const planRank = INDIVIDUAL_PLAN_ORDER.indexOf(plan.id as IndividualUpgradePlanId);
      const activeRank = INDIVIDUAL_PLAN_ORDER.indexOf(activeHeadroomPlanId);
      if (planRank === -1 || activeRank === -1) {
        return plan;
      }

      if (plan.id === activeHeadroomPlanId) {
        return {
          ...plan,
          ctaLabel: `Stay on ${plan.name} plan`,
          ctaVariant: "secondary",
          ctaTone: "default"
        };
      }

      if (planRank < activeRank) {
        return {
          ...plan,
          ctaLabel: `Downgrade to ${plan.name} plan`,
          ctaVariant: "secondary",
          ctaTone: "downgrade"
        };
      }

      return {
        ...plan,
        ctaLabel: `Upgrade to ${plan.name}`,
        ctaVariant: "primary",
        ctaTone: "default"
      };
    };

    if (activeHeadroomPlanId) {
      const orderedPaidPlans = [
        paidPlans[activeHeadroomPlanId],
        ...(["pro", "max5x", "max20x"] as const)
          .filter((planId) => planId !== activeHeadroomPlanId)
          .map((planId) => paidPlans[planId])
      ].map(withRelativeCta);
      return {
        plans: [withRelativeCta(freePlan), ...orderedPaidPlans],
        featuredPlanId: activeHeadroomPlanId
      };
    }

    const activePaidPlanId = (() => {
      switch (claudePlanTier) {
        case "pro":
          return "pro" as const;
        case "max5x":
          return "max5x" as const;
        case "max20x":
          return "max20x" as const;
        default:
          return headroomSubscriptionTier ?? null;
      }
    })();

    if (activePaidPlanId) {
      const orderedPaidPlans = [
        paidPlans[activePaidPlanId],
        ...(["pro", "max5x", "max20x"] as const)
          .filter((planId) => planId !== activePaidPlanId)
          .map((planId) => paidPlans[planId])
      ];
      return {
        plans: [freePlan, ...orderedPaidPlans],
        featuredPlanId: activePaidPlanId
      };
    }

    if (recommendedSubscriptionTier) {
      const orderedPaidPlans = [
        paidPlans[recommendedSubscriptionTier],
        ...(["pro", "max5x", "max20x"] as const)
          .filter((planId) => planId !== recommendedSubscriptionTier)
          .map((planId) => paidPlans[planId])
      ];
      return {
        plans: [freePlan, ...orderedPaidPlans],
        featuredPlanId: recommendedSubscriptionTier
      };
    }

    if (claudePlanTier === "unknown") {
      return {
        plans: [
          freePlan,
          paidPlans.max5x,
          paidPlans.pro,
          paidPlans.max20x
        ],
        featuredPlanId: "max5x"
      };
    }

    return {
      plans: [
        freePlan,
        paidPlans.pro,
        paidPlans.max5x,
        paidPlans.max20x
      ],
      featuredPlanId: "pro"
    };
  }

  return {
    plans: [
      {
        id: "enterprise",
        name: "Team & Enterprise",
        tagline: "Shared controls, governance, and private deployment options",
        price: "",
        billingLines: ["", ""],
        centeredPriceLabel: "custom pricing • contact us",
        featureIntro: "",
        features: [],
        ctaLabel: "Submit",
        ctaVariant: "primary",
        ctaTone: "default"
      }
    ],
    featuredPlanId: "enterprise"
  };
}
