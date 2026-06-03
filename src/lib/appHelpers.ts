import type {
  BillingPeriod,
  HeadroomPricingStatus,
  HeadroomSubscriptionTier,
} from "./types";

export type PricingAudience = "individual" | "teamEnterprise";
export type { BillingPeriod };

const PLAN_PRICES: Record<
  "pro" | "max5x" | "max20x",
  Record<BillingPeriod, { full: string; fullCents: number; discounted: string }>
> = {
  pro:   { annual: { full: "$5",  fullCents: 500,  discounted: "$2.50" }, monthly: { full: "$7.50", fullCents: 750,  discounted: "$3.75" } },
  max5x: { annual: { full: "$20", fullCents: 2000, discounted: "$10"   }, monthly: { full: "$30",   fullCents: 3000, discounted: "$15"   } },
  max20x:{ annual: { full: "$40", fullCents: 4000, discounted: "$20"   }, monthly: { full: "$60",   fullCents: 6000, discounted: "$30"   } },
};
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

/// Total amount charged on a single billing cycle (e.g. `$120` for a 50%-off
/// Max x5 annual subscriber paying $10/month for 12 months upfront). Used in
/// the upgrade confirmation modal when telling a checkout-bound user the
/// dollar figure they'll see on Polar's checkout.
export function getPlanCycleTotalLabel(
  toTier: HeadroomSubscriptionTier,
  billingPeriod: BillingPeriod,
  options?: { fromTier?: HeadroomSubscriptionTier; currentPaidCents?: number | null }
): string {
  const perMonth = projectPerMonthCents(toTier, billingPeriod, options);
  const cycleMonths = billingPeriod === "annual" ? 12 : 1;
  return formatCents(perMonth * cycleMonths);
}

/// Full-price cents for one complete billing cycle (annual = 12x the per-
/// month figure, monthly = 1x).
export function standardListPriceCents(
  tier: HeadroomSubscriptionTier,
  billingPeriod: BillingPeriod
): number {
  const perMonth = PLAN_PRICES[tier][billingPeriod].fullCents;
  const cycleMonths = billingPeriod === "annual" ? 12 : 1;
  return perMonth * cycleMonths;
}

/// Returns true when the user is effectively running with an active launch
/// discount. The primary signal is the synced `subscription_discount_duration`,
/// but that goes null after a `once`-duration discount is consumed by its
/// first invoice — leaving the user looking "undiscounted" even though Polar's
/// launch discount is still globally active and the user is paying below list.
/// The secondary signal catches that case so the desktop doesn't silently
/// route them into a PATCH that bills the full prorated diff.
export function detectSubscriberHasDiscount(args: {
  subscriptionDiscountDuration?: string | null;
  launchDiscountActive?: boolean;
  currentTier?: HeadroomSubscriptionTier | null;
  currentBillingPeriod?: BillingPeriod | null;
  subscriptionAmountCents?: number | null;
}): boolean {
  if (args.subscriptionDiscountDuration) return true;
  if (
    args.launchDiscountActive &&
    args.currentTier &&
    args.currentBillingPeriod &&
    args.subscriptionAmountCents !== null &&
    args.subscriptionAmountCents !== undefined
  ) {
    const listCents = standardListPriceCents(args.currentTier, args.currentBillingPeriod);
    return args.subscriptionAmountCents < listCents;
  }
  return false;
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
  subscriptionEndsAt?: string | null
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
      tagline: "Limited usage with Claude",
      price: "$0",
      billingLines: ["/ month", "free"],
      featureIntro: "Includes:",
      features: [
        "Unlocks cost savings and stats",
        "Use with 25% of your Claude plan",
        "Optimize Claude Code practices"
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
      // attaches the launch discount; the active plan card uses purchaseInfo
      // (actual paid amount) instead of a generic discount badge.
      const showDiscount = launchDiscountActive && id !== activeHeadroomPlanId;
      const price = showDiscount ? prices.discounted : prices.full;
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
        "Unlimited use with Claude Pro",
        "Track sessions across devices",
        "Email-based support"
      ], "Get Pro"),
      max5x: paidPlan("max5x", "Max x5", "For Claude Max x5 accounts", "Includes:", [
        "Unlimited use with Claude Max x5",
        "Track sessions across devices",
        "Email-based support"
      ], "Get Max x5"),
      max20x: paidPlan("max20x", "Max x20", "For Claude Max x20 accounts", "Includes:", [
        "Unlimited use with Claude Max x20",
        "Track sessions across devices",
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
