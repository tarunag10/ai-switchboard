import { afterEach, describe, expect, it, vi } from "vitest";

import {
  accountDisplayEmailFromPricing,
  accountPlanNameFromPricing,
  localGraceHoursRemainingFromPricing,
  trialDaysRemainingFromPricing,
  upgradeTrialCalloutFromPricing,
} from "./trayPricingPresentation";
import type { HeadroomPricingStatus } from "./types";

function status(overrides: Record<string, unknown> = {}) {
  return {
    authenticated: false,
    localGraceStartedAt: "2026-01-01T00:00:00.000Z",
    localGraceEndsAt: "2026-01-04T00:00:00.000Z",
    localGraceActive: true,
    needsAuthentication: true,
    optimizationAllowed: true,
    shouldNudge: false,
    nudgeLevel: 0,
    gateMessage: "Upgrade required.",
    claude: { email: "claude@example.com" },
    launchDiscountActive: false,
    ...overrides,
  } as HeadroomPricingStatus;
}

describe("trayPricingPresentation", () => {
  afterEach(() => vi.useRealTimers());

  it("calculates trial days and local grace hours with expiration floors", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-01T00:00:00.000Z"));
    expect(
      trialDaysRemainingFromPricing(
        status({ account: { trialEndsAt: "2026-01-02T12:00:00.000Z" } }),
      ),
    ).toBe(2);
    expect(
      localGraceHoursRemainingFromPricing(
        status({ localGraceEndsAt: "2026-01-01T01:01:00.000Z" }),
      ),
    ).toBe(2);
    expect(
      trialDaysRemainingFromPricing(
        status({ account: { trialEndsAt: "2025-12-01T00:00:00.000Z" } }),
      ),
    ).toBe(0);
    expect(
      localGraceHoursRemainingFromPricing(
        status({ localGraceEndsAt: "2025-12-01T00:00:00.000Z" }),
      ),
    ).toBe(0);
    expect(trialDaysRemainingFromPricing(null)).toBeNull();
    expect(localGraceHoursRemainingFromPricing(null)).toBeNull();
  });

  it("selects account, entered, Claude, and unknown email fallbacks", () => {
    expect(
      accountDisplayEmailFromPricing(
        status({ account: { email: "account@example.com" } }),
        "entered@example.com",
      ),
    ).toBe("account@example.com");
    expect(accountDisplayEmailFromPricing(status(), " entered@example.com ")).toBe(
      "entered@example.com",
    );
    expect(accountDisplayEmailFromPricing(status(), "")).toBe(
      "claude@example.com",
    );
    expect(
      accountDisplayEmailFromPricing(status({ claude: { email: "" } }), ""),
    ).toBe("unknown email");
  });

  it("describes signed-out, syncing, unavailable, paid, trial, and expired plans", () => {
    expect(accountPlanNameFromPricing(status(), null)).toBeNull();
    expect(
      accountPlanNameFromPricing(status({ authenticated: true, account: null }), null),
    ).toBe("Syncing plan...");
    expect(
      accountPlanNameFromPricing(
        status({ authenticated: true, account: null, accountSyncError: "offline" }),
        null,
      ),
    ).toBe("Plan unavailable");
    expect(
      accountPlanNameFromPricing(
        status({
          authenticated: true,
          account: { subscriptionActive: true, subscriptionTier: "max5x" },
        }),
        null,
      ),
    ).toContain("Max");
    expect(
      accountPlanNameFromPricing(
        status({
          authenticated: true,
          account: { subscriptionActive: false, trialActive: true },
        }),
        1,
      ),
    ).toBe("1 day left in trial");
    expect(
      accountPlanNameFromPricing(
        status({
          authenticated: true,
          account: { subscriptionActive: false, trialActive: true },
        }),
        3,
      ),
    ).toBe("3 days left in trial");
    expect(
      accountPlanNameFromPricing(
        status({
          authenticated: true,
          account: { subscriptionActive: false, trialActive: true },
        }),
        null,
      ),
    ).toBe("7-day trial");
    expect(
      accountPlanNameFromPricing(
        status({
          authenticated: true,
          account: { subscriptionActive: false, trialActive: false },
        }),
        null,
      ),
    ).toBe("Trial expired");
  });

  it("builds loading and unavailable callouts", () => {
    expect(upgradeTrialCalloutFromPricing(true, null, null, vi.fn())).toEqual({
      tone: "neutral",
      message: "Loading your Switchboard access...",
    });
    expect(upgradeTrialCalloutFromPricing(false, null, null, vi.fn())).toEqual({
      tone: "neutral",
      message: "Headroom pricing status is unavailable right now.",
    });
  });

  it("wires signed-out grace and expired callouts", () => {
    const open = vi.fn();
    const grace = upgradeTrialCalloutFromPricing(false, status(), 1, open);
    expect(grace).toMatchObject({ tone: "warning", actionLabel: "Sign up" });
    expect(grace.message).toContain("1 hour");
    grace.onAction?.();
    expect(open).toHaveBeenCalledOnce();
    expect(
      upgradeTrialCalloutFromPricing(false, status(), null, open).message,
    ).toContain("72 hours");
    expect(
      upgradeTrialCalloutFromPricing(
        false,
        status({ localGraceActive: false }),
        0,
        open,
      ).tone,
    ).toBe("expired");
  });

  it("builds authenticated sync, paid, trial, warning, and expired callouts", () => {
    expect(
      upgradeTrialCalloutFromPricing(
        false,
        status({ authenticated: true, account: null, accountSyncError: "sync failed" }),
        null,
        vi.fn(),
      ).message,
    ).toBe("sync failed");
    expect(
      upgradeTrialCalloutFromPricing(
        false,
        status({ authenticated: true, account: null }),
        null,
        vi.fn(),
      ).message,
    ).toContain("Syncing");
    expect(
      upgradeTrialCalloutFromPricing(
        false,
        status({
          authenticated: true,
          account: { subscriptionActive: true, subscriptionTier: "pro" },
        }),
        null,
        vi.fn(),
      ).tone,
    ).toBe("active");
    expect(
      upgradeTrialCalloutFromPricing(
        false,
        status({
          authenticated: true,
          account: { subscriptionActive: false, trialActive: true },
        }),
        null,
        vi.fn(),
      ).tone,
    ).toBe("active");
    expect(
      upgradeTrialCalloutFromPricing(
        false,
        status({
          authenticated: true,
          optimizationAllowed: true,
          account: { subscriptionActive: false, trialActive: false },
        }),
        null,
        vi.fn(),
      ).tone,
    ).toBe("warning");
    expect(
      upgradeTrialCalloutFromPricing(
        false,
        status({
          authenticated: true,
          optimizationAllowed: false,
          account: { subscriptionActive: false, trialActive: false },
        }),
        null,
        vi.fn(),
      ).tone,
    ).toBe("expired");
  });
});
