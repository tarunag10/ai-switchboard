import { describe, expect, it } from "vitest";

import {
  describeInvokeError,
  detectSubscriberHasDiscount,
  getNextLowerUpgradePlanId,
  getPlanCycleTotalLabel,
  getPlanRenewalPriceLabel,
  getUpgradePlans,
  isTierDowngrade,
  standardListPriceCents,
  upgradePlanIntentLabel,
} from "./appHelpers";

describe("app helpers", () => {
  it("formats upgrade intent labels for paid plans only", () => {
    expect(upgradePlanIntentLabel("pro")).toBe("Pro");
    expect(upgradePlanIntentLabel("max5x")).toBe("Max x5");
    expect(upgradePlanIntentLabel("max20x")).toBe("Max x20");
    expect(upgradePlanIntentLabel("free")).toBeNull();
    expect(upgradePlanIntentLabel(null)).toBeNull();
  });

  it("extracts invoke errors from common shapes before falling back", () => {
    expect(describeInvokeError(new Error("network down"), "fallback")).toBe("network down");
    expect(describeInvokeError("permission denied", "fallback")).toBe("permission denied");
    expect(describeInvokeError({ message: "typed message" }, "fallback")).toBe("typed message");
    expect(describeInvokeError({ error: "nested error" }, "fallback")).toBe("nested error");
    expect(describeInvokeError({ message: "   " }, "fallback")).toBe("fallback");
  });

  it("returns the next lower visible plan for paid subscriptions", () => {
    expect(getNextLowerUpgradePlanId("pro")).toBe("free");
    expect(getNextLowerUpgradePlanId("max5x")).toBe("pro");
    expect(getNextLowerUpgradePlanId("max20x")).toBe("max5x");
    expect(getNextLowerUpgradePlanId(null)).toBeNull();
  });

  it("prioritizes the active individual subscription plan", () => {
    const result = getUpgradePlans("individual", "max20x");

    expect(result.featuredPlanId).toBe("max20x");
    expect(result.plans.map((plan) => plan.id)).toEqual([
      "free",
      "max20x",
      "pro",
      "max5x",
    ]);
  });

  it("uses recommended subscription order when no active plan exists", () => {
    const result = getUpgradePlans("individual", "free", "max5x");

    expect(result.featuredPlanId).toBe("max5x");
    expect(result.plans.map((plan) => plan.id)).toEqual([
      "free",
      "max5x",
      "pro",
      "max20x",
    ]);
  });

  it("defaults unknown individual plans toward max x5 guidance", () => {
    const result = getUpgradePlans("individual", "unknown");

    expect(result.featuredPlanId).toBe("max5x");
    expect(result.plans.map((plan) => plan.id)).toEqual([
      "free",
      "max5x",
      "pro",
      "max20x",
    ]);
  });

  it("returns the enterprise contact card for team audiences", () => {
    const result = getUpgradePlans("teamEnterprise");

    expect(result.featuredPlanId).toBe("enterprise");
    expect(result.plans).toHaveLength(1);
    expect(result.plans[0]).toMatchObject({
      id: "enterprise",
      ctaLabel: "Submit",
    });
  });

  it("makes individual plan buttons relative to the active paid Headroom plan", () => {
    const result = getUpgradePlans("individual", "max20x", undefined, "pro", true);

    expect(result.featuredPlanId).toBe("pro");
    expect(result.plans.map((plan) => [plan.id, plan.ctaLabel])).toEqual([
      ["free", "Downgrade to Free plan"],
      ["pro", "Stay on Pro plan"],
      ["max5x", "Upgrade to Max x5"],
      ["max20x", "Upgrade to Max x20"],
    ]);
  });

  it("shows full annual prices when launch discount is inactive", () => {
    const result = getUpgradePlans("individual");

    expect(result.plans.map((plan) => [plan.id, plan.price])).toEqual([
      ["free", "$0"],
      ["pro", "$5"],
      ["max5x", "$20"],
      ["max20x", "$40"],
    ]);
  });

  it("shows discounted annual prices when launch discount is active", () => {
    const result = getUpgradePlans("individual", "free", undefined, undefined, undefined, true);

    expect(result.plans.map((plan) => [plan.id, plan.price])).toEqual([
      ["free", "$0"],
      ["pro", "$2.50"],
      ["max5x", "$10"],
      ["max20x", "$20"],
    ]);
  });

  it("shows full monthly prices when launch discount is inactive", () => {
    const result = getUpgradePlans("individual", "free", undefined, undefined, undefined, false, "monthly");

    expect(result.plans.map((plan) => [plan.id, plan.price])).toEqual([
      ["free", "$0"],
      ["pro", "$7.50"],
      ["max5x", "$30"],
      ["max20x", "$60"],
    ]);
  });

  it("shows discounted monthly prices when launch discount is active", () => {
    const result = getUpgradePlans("individual", "free", undefined, undefined, undefined, true, "monthly");

    expect(result.plans.map((plan) => [plan.id, plan.price])).toEqual([
      ["free", "$0"],
      ["pro", "$3.75"],
      ["max5x", "$15"],
      ["max20x", "$30"],
    ]);
  });

  it("shows discounted prices on upgrade-target cards for an active subscriber with the launch discount", () => {
    const result = getUpgradePlans("individual", "max20x", undefined, "pro", true, true);

    const byId = (id: string) => result.plans.find((plan) => plan.id === id);
    // Active plan card keeps its full list price (purchaseInfo conveys the real amount).
    expect(byId("pro")?.price).toBe("$5");
    expect(byId("pro")?.originalPrice).toBeUndefined();
    // Upgrade targets show the discounted price with the full price struck through.
    expect([byId("max5x")?.price, byId("max5x")?.originalPrice]).toEqual(["$10", "$20"]);
    expect([byId("max20x")?.price, byId("max20x")?.originalPrice]).toEqual(["$20", "$40"]);
  });

  it("classifies tier direction for plan changes", () => {
    expect(isTierDowngrade("pro", "max20x")).toBe(false);
    expect(isTierDowngrade("max20x", "pro")).toBe(true);
    expect(isTierDowngrade("max5x", "max20x")).toBe(false);
    expect(isTierDowngrade("max20x", "max5x")).toBe(true);
  });

  describe("getPlanRenewalPriceLabel", () => {
    it("returns the standard per-month price when no current paid amount is given", () => {
      // Max x5 annual is $20 / month (billed annually).
      expect(getPlanRenewalPriceLabel("max5x", "annual")).toBe("$20 / month");
      expect(getPlanRenewalPriceLabel("max5x", "monthly")).toBe("$30 / month");
    });

    it("carries the user's current discount ratio forward to the target plan", () => {
      // 100% off Pro annual (paid $0 vs $60/year list) -> 100% off Max x20.
      expect(
        getPlanRenewalPriceLabel("max20x", "annual", { fromTier: "pro", currentPaidCents: 0 })
      ).toBe("$0 / month");
      // 50% off Pro annual (paid $30/year = 3000 cents per cycle vs $60 list)
      // -> 50% off Max x5 annual: $20 / month list -> $10 / month.
      expect(
        getPlanRenewalPriceLabel("max5x", "annual", { fromTier: "pro", currentPaidCents: 3000 })
      ).toBe("$10 / month");
      // 50% off monthly cycle (paid $3.75 vs $7.50 list per month) -> 50% off Max x5
      // monthly: $30 / month list -> $15 / month.
      expect(
        getPlanRenewalPriceLabel("max5x", "monthly", { fromTier: "pro", currentPaidCents: 375 })
      ).toBe("$15 / month");
    });
  });

  describe("getPlanCycleTotalLabel", () => {
    it("returns the full-cycle total for the target plan", () => {
      // Max x5 annual is $20 / month -> $240 charged once a year.
      expect(getPlanCycleTotalLabel("max5x", "annual")).toBe("$240");
      // Max x5 monthly is $30 / month -> $30 per monthly cycle.
      expect(getPlanCycleTotalLabel("max5x", "monthly")).toBe("$30");
    });

    it("carries the user's current discount ratio into the cycle total", () => {
      // 100%-off Pro annual ($0 paid) -> $0 for a full year of Max x20.
      expect(
        getPlanCycleTotalLabel("max20x", "annual", { fromTier: "pro", currentPaidCents: 0 })
      ).toBe("$0");
      // 50%-off Pro annual (paid $30/year of $60 list) -> half of Max x5
      // annual: $240 list -> $120 total today.
      expect(
        getPlanCycleTotalLabel("max5x", "annual", { fromTier: "pro", currentPaidCents: 3000 })
      ).toBe("$120");
    });
  });

  describe("standardListPriceCents", () => {
    it("returns the full-cycle list price in cents", () => {
      // Pro annual: $5 / month list -> $60 / year.
      expect(standardListPriceCents("pro", "annual")).toBe(6000);
      // Max x5 annual: $20 / month list -> $240 / year.
      expect(standardListPriceCents("max5x", "annual")).toBe(24000);
      // Max x20 monthly: $60 / month list -> $60 / month.
      expect(standardListPriceCents("max20x", "monthly")).toBe(6000);
    });
  });

  describe("detectSubscriberHasDiscount", () => {
    it("returns true when a discount duration is present", () => {
      expect(
        detectSubscriberHasDiscount({ subscriptionDiscountDuration: "forever" })
      ).toBe(true);
      // 'once' counts too; we route to checkout to avoid the proration trap.
      expect(
        detectSubscriberHasDiscount({ subscriptionDiscountDuration: "once" })
      ).toBe(true);
    });

    it("falls back to launch-discount + below-list when sync shows no discount", () => {
      // The exhausted-'once' state: discount duration is null but the user
      // pays below list and the launch promo is still globally active.
      expect(
        detectSubscriberHasDiscount({
          subscriptionDiscountDuration: null,
          launchDiscountActive: true,
          currentTier: "max5x",
          currentBillingPeriod: "annual",
          // List for Max x5 annual is $240 / 24000. They're at $0 -> discounted.
          subscriptionAmountCents: 0
        })
      ).toBe(true);
      // Same user but paying full list price -> no discount.
      expect(
        detectSubscriberHasDiscount({
          subscriptionDiscountDuration: null,
          launchDiscountActive: true,
          currentTier: "max5x",
          currentBillingPeriod: "annual",
          subscriptionAmountCents: 24000
        })
      ).toBe(false);
    });

    it("returns false when launch discount is inactive even if user pays below list", () => {
      expect(
        detectSubscriberHasDiscount({
          subscriptionDiscountDuration: null,
          launchDiscountActive: false,
          currentTier: "pro",
          currentBillingPeriod: "annual",
          subscriptionAmountCents: 0
        })
      ).toBe(false);
    });

    it("returns false when the amount or tier is unknown", () => {
      expect(
        detectSubscriberHasDiscount({
          launchDiscountActive: true,
          currentTier: null,
          currentBillingPeriod: "annual",
          subscriptionAmountCents: 0
        })
      ).toBe(false);
      expect(
        detectSubscriberHasDiscount({
          launchDiscountActive: true,
          currentTier: "pro",
          currentBillingPeriod: "annual",
          subscriptionAmountCents: null
        })
      ).toBe(false);
    });
  });

  describe("active plan purchase info", () => {
    const baseArgs = [
      "individual" as const,
      undefined,
      undefined,
      "pro" as const,
      true,
      false,
      "annual" as const,
    ] as const;

    function activePlan(result: ReturnType<typeof getUpgradePlans>) {
      return result.plans.find((p) => p.id === "pro");
    }

    it("omits purchase info when subscription amount is missing", () => {
      const result = getUpgradePlans(...baseArgs, null, "annual", "2026-12-01");
      expect(activePlan(result)?.purchaseInfo).toBeUndefined();
    });

    it("omits purchase info when renewal date is missing", () => {
      // 6000 cents = $5/mo * 12 months
      const result = getUpgradePlans(...baseArgs, 6000, "annual", null);
      expect(activePlan(result)?.purchaseInfo).toBeUndefined();
    });

    it("shows full renewal price when no discount is present", () => {
      const result = getUpgradePlans(...baseArgs, 6000, "annual", "2026-12-01");
      expect(activePlan(result)?.purchaseInfo).toMatchObject({
        paidPerMonthLabel: "$5",
        discountPct: 0,
      });
    });

    it("shows full renewal price for a once-off discount", () => {
      // 100% discount this period (0 cents), but "once" so renewal is full price
      const result = getUpgradePlans(...baseArgs, 0, "annual", "2026-04-16", "2025-04-16", "once");
      expect(activePlan(result)?.purchaseInfo).toMatchObject({
        paidPerMonthLabel: "$5",
        discountPct: 0,
      });
    });

    it("shows discounted renewal price for a forever discount", () => {
      // 3000 cents = $2.50/mo * 12 months (50% off)
      const result = getUpgradePlans(...baseArgs, 3000, "annual", "2026-12-01", "2025-12-01", "forever");
      expect(activePlan(result)?.purchaseInfo).toMatchObject({
        paidPerMonthLabel: "$2.50",
        discountPct: 50,
      });
    });

    it("shows discounted renewal price when repeating discount window has not expired", () => {
      // Started 2025-04-16, 12-month discount window → expires 2026-04-16
      // Renewal at 2026-01-01 is within window → discount applies
      const result = getUpgradePlans(...baseArgs, 3000, "annual", "2026-01-01", "2025-04-16", "repeating", 12);
      expect(activePlan(result)?.purchaseInfo).toMatchObject({
        paidPerMonthLabel: "$2.50",
        discountPct: 50,
      });
    });

    it("shows full renewal price when repeating discount window has expired", () => {
      // Started 2024-01-01, 12-month window → expired 2025-01-01
      // Renewal at 2026-04-01 is outside window → full price
      const result = getUpgradePlans(...baseArgs, 3000, "annual", "2026-04-01", "2024-01-01", "repeating", 12);
      expect(activePlan(result)?.purchaseInfo).toMatchObject({
        paidPerMonthLabel: "$5",
        discountPct: 0,
      });
    });

    it("shows full renewal price for repeating discount with missing window data", () => {
      // "repeating" but duration_in_months is null → treat as no discount at renewal
      const result = getUpgradePlans(...baseArgs, 3000, "annual", "2026-12-01", "2025-12-01", "repeating", null);
      expect(activePlan(result)?.purchaseInfo).toMatchObject({
        paidPerMonthLabel: "$5",
        discountPct: 0,
      });
    });
  });

  describe("scheduled downgrade", () => {
    const baseArgs = [
      "individual" as const,
      undefined,
      undefined,
      "max20x" as const,
      true,
      false,
      "annual" as const,
    ] as const;

    function planById(result: ReturnType<typeof getUpgradePlans>, id: string) {
      return result.plans.find((p) => p.id === id);
    }

    it("stamps cancel info on the active plan card when downgrade is scheduled", () => {
      const result = getUpgradePlans(
        ...baseArgs,
        12000, // $10/mo annual
        "annual",
        "2027-03-31",
        "2026-03-31",
        null,
        null,
        true,
        "2027-03-31T20:31:45Z"
      );
      const active = planById(result, "max20x");
      expect(active?.purchaseInfo?.cancelAtPeriodEnd).toBe(true);
      expect(active?.purchaseInfo?.endsOn).toBeDefined();
    });

    it("stamps the free plan with the activation date and downgrade-scheduled CTA", () => {
      const result = getUpgradePlans(
        ...baseArgs,
        12000,
        "annual",
        "2027-03-31",
        "2026-03-31",
        null,
        null,
        true,
        "2027-03-31T20:31:45Z"
      );
      const free = planById(result, "free");
      expect(free?.ctaLabel).toBe("Downgrade scheduled");
      expect(free?.purchaseInfo?.cancelAtPeriodEnd).toBe(true);
      expect(free?.purchaseInfo?.endsOn).toBeDefined();
    });

    it("leaves both cards untouched when no downgrade is scheduled", () => {
      const result = getUpgradePlans(
        ...baseArgs,
        12000,
        "annual",
        "2027-03-31",
        "2026-03-31",
        null,
        null,
        false,
        null
      );
      const free = planById(result, "free");
      const active = planById(result, "max20x");
      // On Max x20, the Free card's natural relative-CTA is "Downgrade to Free plan".
      expect(free?.ctaLabel).toBe("Downgrade to Free plan");
      expect(free?.purchaseInfo).toBeUndefined();
      expect(active?.purchaseInfo?.cancelAtPeriodEnd).toBe(false);
      expect(active?.purchaseInfo?.endsOn).toBeUndefined();
    });
  });
});
