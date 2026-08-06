import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import {
  describeInvokeError,
  tierRecommendationSourceLabel,
  upgradePlanIntentLabel,
  type BillingPeriod,
  type PricingAudience,
  type UpgradePlanId,
} from "./appHelpers";
import { isValidEmailAddress } from "./launcherHelpers";
import { localOnlyModeEnabled } from "./localMode";
import {
  cachePricingStatus,
  readCachedPricing,
  type CachedPricing,
  writeCachedPricing,
} from "./pricing";
import { SALES_CONTACT_URL } from "./supportUrls";
import { hasTauriEventRuntime } from "./tauriRuntime";
import { safeTrayViewForMode, type TrayView } from "./trayHelpers";
import { trackAnalyticsEvent } from "./analytics";
import { maybeFireTrialNotifications } from "./trialNotifications";
import { maybeFireUrgentPricingNotifications } from "./urgentNotifications";
import type {
  HeadroomAuthCodeRequest,
  HeadroomPricingStatus,
  HeadroomSubscriptionTier,
  RuntimeStatus,
} from "./types";

const authCodeExpiryFallbackSeconds = 900;

export interface UseTrayPricingControllerOptions {
  trayWindowFocused: boolean;
  runtimeStatus: RuntimeStatus | null;
  connectorPhase: "disabled" | "verifying" | "healthy";
  setActiveView: (view: TrayView) => void;
  refreshConnectors: () => Promise<void>;
  openExternalLink: (url: string) => Promise<void>;
}

export function useTrayPricingController({
  trayWindowFocused,
  runtimeStatus,
  connectorPhase,
  setActiveView,
  refreshConnectors,
  openExternalLink,
}: UseTrayPricingControllerOptions) {
  const localOnlyMode = localOnlyModeEnabled();
  const [pricingStatus, setPricingStatus] =
    useState<HeadroomPricingStatus | null>(null);
  const [cachedPricing] = useState<CachedPricing>(() => readCachedPricing());
  const [pricingBusy, setPricingBusy] = useState(false);
  const [pricingError, setPricingError] = useState<string | null>(null);
  const pricingRefreshInFlightRef = useRef(false);
  const desktopActivationSentRef = useRef(false);
  const [authEmail, setAuthEmail] = useState("");
  const [authCode, setAuthCode] = useState("");
  const [authCodeRequestedFor, setAuthCodeRequestedFor] = useState<
    string | null
  >(null);
  const [authCodeExpirySeconds, setAuthCodeExpirySeconds] = useState(
    authCodeExpiryFallbackSeconds,
  );
  const [authRequestBusy, setAuthRequestBusy] = useState(false);
  const [authVerifyBusy, setAuthVerifyBusy] = useState(false);
  const [authFlowError, setAuthFlowError] = useState<string | null>(null);
  const [authFlowSuccess, setAuthFlowSuccess] = useState<string | null>(null);
  const [pendingUpgradePlanId, setPendingUpgradePlanId] =
    useState<UpgradePlanId | null>(null);
  const [showAllUpgradePlans, setShowAllUpgradePlans] = useState(false);
  const [checkoutPollingDeadline, setCheckoutPollingDeadline] = useState<
    number | null
  >(null);
  const [pricingAudience, setPricingAudience] =
    useState<PricingAudience>("individual");
  const [billingPeriod, setBillingPeriod] = useState<BillingPeriod>("annual");
  const [upgradeActionBusy, setUpgradeActionBusy] =
    useState<UpgradePlanId | null>(null);
  const [upgradeActionError, setUpgradeActionError] = useState<string | null>(
    null,
  );
  const [pendingPlanChange, setPendingPlanChange] = useState<{
    fromTier: HeadroomSubscriptionTier;
    toTier: HeadroomSubscriptionTier;
    billingPeriod: BillingPeriod;
  } | null>(null);
  const [planChangeBusy, setPlanChangeBusy] = useState(false);
  const [planChangeError, setPlanChangeError] = useState<string | null>(null);
  const [reactivateBusy, setReactivateBusy] = useState(false);
  const [reactivateError, setReactivateError] = useState<string | null>(null);

  const authEmailValid = isValidEmailAddress(authEmail);

  async function refreshPricingStatus() {
    if (localOnlyMode) {
      setPricingBusy(false);
      setPricingError(null);
      return;
    }
    if (pricingRefreshInFlightRef.current) {
      return;
    }
    pricingRefreshInFlightRef.current = true;
    setPricingBusy(true);
    try {
      const status = await invoke<HeadroomPricingStatus>(
        "get_headroom_pricing_status",
      );
      setPricingStatus(status);
      void maybeFireTrialNotifications(status);
      void maybeFireUrgentPricingNotifications(status, { localOnlyMode });
      setPricingError(null);
    } catch (error) {
      setPricingError(
        error instanceof Error
          ? error.message
          : "Could not load pricing status.",
      );
    } finally {
      pricingRefreshInFlightRef.current = false;
      setPricingBusy(false);
    }
  }

  function openUpgradeAuthView(planId: UpgradePlanId | null = null) {
    setActiveView(safeTrayViewForMode("upgradeAuth", localOnlyMode));
    setPendingUpgradePlanId(planId);
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
  }

  function resetUpgradeAuthStep() {
    setAuthCode("");
    setAuthCodeRequestedFor(null);
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
  }

  async function handleRequestAuthCode() {
    if (!authEmailValid) {
      setAuthFlowError("Enter a valid email address.");
      return;
    }
    setAuthRequestBusy(true);
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
    try {
      const result = await invoke<HeadroomAuthCodeRequest>(
        "request_headroom_auth_code",
        {
          email: authEmail.trim(),
        },
      );
      setAuthCodeRequestedFor(result.email);
      setAuthCodeExpirySeconds(result.expiresInSeconds);
      setAuthFlowSuccess(`We sent a sign-in code to ${result.email}.`);
    } catch (error) {
      setAuthFlowError(
        describeInvokeError(error, "Could not send sign-in code."),
      );
    } finally {
      setAuthRequestBusy(false);
    }
  }

  async function handleVerifyAuthCode() {
    if (!authEmailValid) {
      setAuthFlowError("Enter a valid email address.");
      return;
    }
    if (!authCode.trim()) {
      setAuthFlowError("Enter the authentication code from your email.");
      return;
    }
    setAuthVerifyBusy(true);
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
    try {
      const status = await invoke<HeadroomPricingStatus>(
        "verify_headroom_auth_code",
        {
          email: authEmail.trim(),
          code: authCode.trim(),
          inviteCode: null,
        },
      );
      setPricingStatus(status);
      setAuthCode("");
      setAuthCodeRequestedFor(null);
      setAuthFlowSuccess("Switchboard account connected.");
      setPendingUpgradePlanId(null);
      setActiveView(safeTrayViewForMode("upgrade", localOnlyMode));
      await refreshConnectors();
    } catch (error) {
      setAuthFlowError(
        describeInvokeError(error, "Could not verify sign-in code."),
      );
    } finally {
      setAuthVerifyBusy(false);
    }
  }

  async function handleSignOutHeadroomAccount() {
    setAuthFlowError(null);
    setAuthFlowSuccess(null);
    try {
      await invoke("sign_out_headroom_account");
      setPricingStatus(
        await invoke<HeadroomPricingStatus>("get_headroom_pricing_status"),
      );
      setAuthCode("");
      setAuthCodeRequestedFor(null);
      setAuthFlowSuccess("Signed out of Headroom.");
      setPendingUpgradePlanId(null);
    } catch (error) {
      setAuthFlowError(
        error instanceof Error
          ? error.message
          : "Could not sign out of Headroom.",
      );
    }
  }

  async function handleUpgradeAction(planId: UpgradePlanId) {
    const activeHeadroomPlanId = pricingStatus?.account?.subscriptionActive
      ? (pricingStatus.account.subscriptionTier ?? null)
      : null;
    const action = (() => {
      switch (planId) {
        case "free":
          return {
            kind: activeHeadroomPlanId
              ? ("billing_portal" as const)
              : ("internal" as const),
          };
        case "pro":
        case "max5x":
        case "max20x": {
          if (activeHeadroomPlanId === planId)
            return { kind: "internal" as const };
          if (activeHeadroomPlanId) {
            return { kind: "change_plan" as const };
          }
          return { kind: "checkout" as const };
        }
        case "team":
          return {
            kind: "external" as const,
            url: SALES_CONTACT_URL,
            missing:
              "Set VITE_HEADROOM_SALES_CONTACT_URL to enable Team sales inquiries.",
          };
        case "enterprise":
          return {
            kind: "external" as const,
            url: SALES_CONTACT_URL,
            missing:
              "Set VITE_HEADROOM_SALES_CONTACT_URL to enable Enterprise contact.",
          };
        default:
          return null;
      }
    })();

    if (!action) {
      return;
    }

    trackAnalyticsEvent("upgrade_button_clicked", {
      plan_id: planId,
      action_kind: action.kind,
      email:
        pricingStatus?.account?.email ??
        pricingStatus?.claude?.email ??
        undefined,
    });

    if (action.kind === "internal") {
      setUpgradeActionError(null);
      setActiveView("home");
      return;
    }

    if (!pricingStatus?.authenticated) {
      openUpgradeAuthView(planId);
      return;
    }

    if (action.kind === "change_plan") {
      const fromTier = pricingStatus?.account?.subscriptionTier;
      if (!fromTier) return;
      setPlanChangeError(null);
      setPendingPlanChange({
        fromTier,
        toTier: planId as HeadroomSubscriptionTier,
        billingPeriod,
      });
      return;
    }

    if (action.kind === "checkout") {
      setUpgradeActionBusy(planId);
      setUpgradeActionError(null);

      try {
        const url = await invoke<string>("create_headroom_checkout_session", {
          subscriptionTier: planId,
          billingPeriod,
        });
        await openExternalLink(url);
        setCheckoutPollingDeadline(Date.now() + 5 * 60_000);
      } catch (error) {
        setUpgradeActionError(
          error instanceof Error
            ? error.message
            : typeof error === "string"
              ? error
              : "Could not start checkout.",
        );
      } finally {
        setUpgradeActionBusy(null);
      }
      return;
    }

    if (action.kind === "billing_portal") {
      setUpgradeActionBusy(planId);
      setUpgradeActionError(null);

      try {
        const url = await invoke<string>("get_headroom_billing_portal_url", {
          target: "subscription",
        });
        await openExternalLink(url);
      } catch (error) {
        setUpgradeActionError(
          error instanceof Error
            ? error.message
            : typeof error === "string"
              ? error
              : "Could not open billing portal.",
        );
      } finally {
        setUpgradeActionBusy(null);
      }
      return;
    }

    if (!action.url) {
      setUpgradeActionError(
        action.missing ?? "Could not open the selected plan link.",
      );
      return;
    }

    setUpgradeActionBusy(planId);
    setUpgradeActionError(null);

    try {
      await openExternalLink(action.url);
    } catch (error) {
      setUpgradeActionError(
        error instanceof Error
          ? error.message
          : "Could not open the selected plan link.",
      );
    } finally {
      setUpgradeActionBusy(null);
    }
  }

  async function confirmPlanChange() {
    if (!pendingPlanChange) return;
    setPlanChangeBusy(true);
    setPlanChangeError(null);
    try {
      await invoke("change_headroom_subscription_plan", {
        subscriptionTier: pendingPlanChange.toTier,
        billingPeriod: pendingPlanChange.billingPeriod,
      });
      await refreshPricingStatus();
      setPendingPlanChange(null);
      setActiveView("home");
    } catch (error) {
      setPlanChangeError(
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Could not change subscription plan.",
      );
    } finally {
      setPlanChangeBusy(false);
    }
  }

  function cancelPlanChange() {
    if (planChangeBusy) return;
    setPendingPlanChange(null);
    setPlanChangeError(null);
  }

  async function handleReactivateSubscription() {
    if (reactivateBusy) return;
    setReactivateBusy(true);
    setReactivateError(null);
    try {
      await invoke("reactivate_headroom_subscription");
      await refreshPricingStatus();
    } catch (error) {
      setReactivateError(
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Could not reactivate subscription.",
      );
    } finally {
      setReactivateBusy(false);
    }
  }

  useEffect(() => {
    if (!pricingStatus?.authenticated) {
      desktopActivationSentRef.current = false;
    }
  }, [pricingStatus?.authenticated]);

  useEffect(() => {
    if (!pricingStatus) return;
    writeCachedPricing(cachePricingStatus(pricingStatus));
  }, [pricingStatus]);

  useEffect(() => {
    const STORAGE_KEY = "headroom:lastNotifiedMismatchTier";
    if (localOnlyMode) {
      window.localStorage.removeItem(STORAGE_KEY);
      return;
    }
    const mismatch = pricingStatus?.tierMismatch;
    if (!mismatch) {
      window.localStorage.removeItem(STORAGE_KEY);
      return;
    }
    const rank: Record<string, number> = { pro: 1, max5x: 2, max20x: 3 };
    const previous = window.localStorage.getItem(STORAGE_KEY);
    if (
      previous !== null &&
      (rank[mismatch.recommendedTier] ?? 0) <= (rank[previous] ?? 0)
    ) {
      return;
    }
    const paidLabel = upgradePlanIntentLabel(mismatch.paidTier);
    const recommendedLabel = upgradePlanIntentLabel(mismatch.recommendedTier);
    const sourceLabel = tierRecommendationSourceLabel(
      mismatch.recommendedSource,
    );
    void invoke("show_notification", {
      title: "Upgrade your Headroom plan",
      body: `Your ${sourceLabel} usage needs the Switchboard ${recommendedLabel} plan, above your current ${paidLabel} plan. Upgrade to keep unlimited optimization.`,
    }).catch(() => {});
    window.localStorage.setItem(STORAGE_KEY, mismatch.recommendedTier);
  }, [
    localOnlyMode,
    pricingStatus?.tierMismatch?.recommendedTier,
    pricingStatus?.tierMismatch,
  ]);

  useEffect(() => {
    setShowAllUpgradePlans(false);
    if (pricingAudience !== "individual") setBillingPeriod("annual");
  }, [pricingAudience]);

  useEffect(() => {
    if (localOnlyMode || !hasTauriEventRuntime()) {
      return;
    }
    const intervalMs = trayWindowFocused ? 60_000 : 600_000;
    void refreshPricingStatus();
    const interval = window.setInterval(() => {
      void refreshPricingStatus();
    }, intervalMs);
    return () => {
      window.clearInterval(interval);
    };
  }, [localOnlyMode, trayWindowFocused]);

  useEffect(() => {
    if (localOnlyMode || !hasTauriEventRuntime()) {
      return;
    }
    let unlisten: (() => void) | undefined;
    void listen("pricing-refreshed", () => {
      void refreshPricingStatus();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [localOnlyMode]);

  useEffect(() => {
    if (localOnlyMode) {
      return;
    }
    if (checkoutPollingDeadline === null) return;
    if (Date.now() > checkoutPollingDeadline) {
      setCheckoutPollingDeadline(null);
      return;
    }
    const interval = window.setInterval(() => {
      if (Date.now() > checkoutPollingDeadline) {
        setCheckoutPollingDeadline(null);
        return;
      }
      void refreshPricingStatus();
    }, 5_000);
    return () => {
      window.clearInterval(interval);
    };
  }, [checkoutPollingDeadline, localOnlyMode]);

  useEffect(() => {
    if (localOnlyMode) {
      return;
    }
    if (
      checkoutPollingDeadline !== null &&
      pricingStatus?.account?.subscriptionActive
    ) {
      setCheckoutPollingDeadline(null);
    }
  }, [
    checkoutPollingDeadline,
    localOnlyMode,
    pricingStatus?.account?.subscriptionActive,
  ]);

  useEffect(() => {
    if (localOnlyMode) {
      return;
    }
    const runtimeHealthyNow =
      runtimeStatus?.running === true &&
      runtimeStatus?.proxyReachable === true &&
      connectorPhase === "healthy";
    if (
      !pricingStatus?.authenticated ||
      !runtimeHealthyNow ||
      desktopActivationSentRef.current
    ) {
      return;
    }
    desktopActivationSentRef.current = true;
    void invoke<HeadroomPricingStatus>("activate_headroom_account")
      .then((status) => setPricingStatus(status))
      .catch(() => {
        desktopActivationSentRef.current = false;
      });
  }, [
    connectorPhase,
    localOnlyMode,
    pricingStatus?.authenticated,
    runtimeStatus?.proxyReachable,
    runtimeStatus?.running,
  ]);

  return {
    pricingStatus,
    setPricingStatus,
    cachedPricing,
    pricingBusy,
    pricingError,
    authEmail,
    setAuthEmail,
    authCode,
    setAuthCode,
    authCodeRequestedFor,
    authCodeExpirySeconds,
    authRequestBusy,
    authVerifyBusy,
    authFlowError,
    authFlowSuccess,
    authEmailValid,
    pendingUpgradePlanId,
    showAllUpgradePlans,
    setShowAllUpgradePlans,
    checkoutPollingDeadline,
    pricingAudience,
    setPricingAudience,
    billingPeriod,
    setBillingPeriod,
    upgradeActionBusy,
    upgradeActionError,
    pendingPlanChange,
    planChangeBusy,
    planChangeError,
    reactivateBusy,
    reactivateError,
    setAuthFlowError,
    setUpgradeActionError,
    refreshPricingStatus,
    openUpgradeAuthView,
    resetUpgradeAuthStep,
    handleRequestAuthCode,
    handleVerifyAuthCode,
    handleSignOutHeadroomAccount,
    handleUpgradeAction,
    confirmPlanChange,
    cancelPlanChange,
    handleReactivateSubscription,
  };
}
