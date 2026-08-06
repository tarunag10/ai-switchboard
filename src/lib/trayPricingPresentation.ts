import { subscriptionTierLabel } from "./pricing";
import type { HeadroomPricingStatus } from "./types";

export function trialDaysRemainingFromPricing(
  pricingStatus: HeadroomPricingStatus | null,
): number | null {
  const target = pricingStatus?.account?.trialEndsAt
    ? new Date(pricingStatus.account.trialEndsAt).getTime()
    : Number.NaN;
  if (Number.isNaN(target)) {
    return null;
  }
  return Math.max(0, Math.ceil((target - Date.now()) / 86_400_000));
}

export function localGraceHoursRemainingFromPricing(
  pricingStatus: HeadroomPricingStatus | null,
): number | null {
  const target = pricingStatus?.localGraceEndsAt
    ? new Date(pricingStatus.localGraceEndsAt).getTime()
    : Number.NaN;
  if (Number.isNaN(target)) {
    return null;
  }
  return Math.max(0, Math.ceil((target - Date.now()) / 3_600_000));
}

export function accountDisplayEmailFromPricing(
  pricingStatus: HeadroomPricingStatus | null,
  authEmail: string,
): string {
  const enteredEmail = authEmail.trim();
  return (
    pricingStatus?.account?.email ??
    (enteredEmail || pricingStatus?.claude.email || "unknown email")
  );
}

export function accountPlanNameFromPricing(
  pricingStatus: HeadroomPricingStatus | null,
  trialDaysRemaining: number | null,
): string | null {
  if (!pricingStatus?.authenticated) {
    return null;
  }
  if (!pricingStatus.account) {
    return pricingStatus.accountSyncError
      ? "Plan unavailable"
      : "Syncing plan...";
  }
  if (pricingStatus.account.subscriptionActive) {
    return subscriptionTierLabel(pricingStatus.account.subscriptionTier);
  }
  if (pricingStatus.account.trialActive) {
    if (trialDaysRemaining != null) {
      return `${trialDaysRemaining} day${trialDaysRemaining === 1 ? "" : "s"} left in trial`;
    }
    return "7-day trial";
  }
  return "Trial expired";
}

export interface UpgradeTrialCallout {
  tone: "neutral" | "warning" | "expired" | "active";
  message: string;
  actionLabel?: string;
  onAction?: () => void;
}

export function upgradeTrialCalloutFromPricing(
  pricingBusy: boolean,
  pricingStatus: HeadroomPricingStatus | null,
  localGraceHoursRemaining: number | null,
  openUpgradeAuthView: () => void,
): UpgradeTrialCallout {
  if (pricingBusy && !pricingStatus) {
    return {
      tone: "neutral",
      message: "Loading your Switchboard access...",
    };
  }
  if (!pricingStatus) {
    return {
      tone: "neutral",
      message: "Headroom pricing status is unavailable right now.",
    };
  }
  if (!pricingStatus.authenticated) {
    if (!pricingStatus.localGraceActive) {
      return {
        tone: "expired",
        message:
          "Your 72-hour Switchboard access expired. Create an account to extend to 7 days.",
        actionLabel: "Sign up",
        onAction: openUpgradeAuthView,
      };
    }
    const hoursLabel =
      localGraceHoursRemaining != null
        ? `${localGraceHoursRemaining} hour${localGraceHoursRemaining === 1 ? "" : "s"}`
        : "72 hours";
    return {
      tone: "warning",
      message: `You have ${hoursLabel} of local-only Switchboard access left. Sign up to extend to 7 days.`,
      actionLabel: "Sign up",
      onAction: openUpgradeAuthView,
    };
  }
  if (!pricingStatus.account) {
    return {
      tone: "neutral",
      message:
        pricingStatus.accountSyncError ??
        "Syncing your Headroom account details...",
    };
  }
  if (pricingStatus.account?.subscriptionActive) {
    return {
      tone: "active",
      message: `${subscriptionTierLabel(pricingStatus.account.subscriptionTier)} is active. Headroom can keep optimizing without limits.`,
    };
  }
  if (pricingStatus.account?.trialActive) {
    return {
      tone: "active",
      message:
        "Your 7-day trial is active. Headroom can optimize without limits.",
    };
  }
  return {
    tone: pricingStatus.optimizationAllowed ? "warning" : "expired",
    message: pricingStatus.gateMessage,
  };
}
