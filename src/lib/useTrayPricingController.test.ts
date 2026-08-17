import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useTrayPricingController } from "./useTrayPricingController";
import type { HeadroomPricingStatus } from "./types";

const { invokeMock, listenMock, trackMock, runtimeFlags } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  trackMock: vi.fn(),
  runtimeFlags: { localOnly: false, eventRuntime: false },
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));
vi.mock("./tauriRuntime", () => ({
  hasTauriEventRuntime: () => runtimeFlags.eventRuntime,
}));
vi.mock("./localMode", () => ({
  localOnlyModeEnabled: () => runtimeFlags.localOnly,
}));
vi.mock("./analytics", () => ({
  trackAnalyticsEvent: (...args: unknown[]) => trackMock(...args),
}));
vi.mock("./trialNotifications", () => ({
  maybeFireTrialNotifications: vi.fn(async () => undefined),
}));
vi.mock("./urgentNotifications", () => ({
  maybeFireUrgentPricingNotifications: vi.fn(async () => undefined),
}));
vi.mock("./pricing", () => ({
  readCachedPricing: () => null,
  cachePricingStatus: (status: unknown) => status,
  writeCachedPricing: vi.fn(),
}));

const signedOut = { authenticated: false, account: null } as HeadroomPricingStatus;
const signedIn = {
  authenticated: true,
  account: {
    email: "person@example.com",
    subscriptionActive: false,
    subscriptionTier: null,
  },
} as HeadroomPricingStatus;

function setup() {
  const setActiveView = vi.fn();
  const refreshConnectors = vi.fn(async () => undefined);
  const openExternalLink = vi.fn(async () => undefined);
  const hook = renderHook(() =>
    useTrayPricingController({
      trayWindowFocused: true,
      runtimeStatus: null,
      connectorPhase: "disabled",
      setActiveView,
      refreshConnectors,
      openExternalLink,
    }),
  );
  return { ...hook, setActiveView, refreshConnectors, openExternalLink };
}

describe("useTrayPricingController", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    trackMock.mockReset();
    runtimeFlags.localOnly = false;
    runtimeFlags.eventRuntime = false;
    window.localStorage.clear();
  });

  it("guards invalid email and missing authentication codes", async () => {
    const { result } = setup();
    await act(() => result.current.handleRequestAuthCode());
    expect(result.current.authFlowError).toBe("Enter a valid email address.");
    expect(invokeMock).not.toHaveBeenCalled();

    act(() => result.current.setAuthEmail("person@example.com"));
    await act(() => result.current.handleVerifyAuthCode());
    expect(result.current.authFlowError).toBe(
      "Enter the authentication code from your email.",
    );
  });

  it("requests a sign-in code with a trimmed exact payload", async () => {
    invokeMock.mockResolvedValueOnce({
      email: "person@example.com",
      expiresInSeconds: 600,
    });
    const { result } = setup();
    act(() => result.current.setAuthEmail(" person@example.com "));
    await act(() => result.current.handleRequestAuthCode());

    expect(invokeMock).toHaveBeenCalledWith("request_headroom_auth_code", {
      email: "person@example.com",
    });
    expect(result.current.authCodeRequestedFor).toBe("person@example.com");
    expect(result.current.authCodeExpirySeconds).toBe(600);
    expect(result.current.authFlowSuccess).toContain("person@example.com");
  });

  it("surfaces request failures and always clears busy state", async () => {
    invokeMock.mockRejectedValueOnce("mail service unavailable");
    const { result } = setup();
    act(() => result.current.setAuthEmail("person@example.com"));
    await act(() => result.current.handleRequestAuthCode());
    expect(result.current.authRequestBusy).toBe(false);
    expect(result.current.authFlowError).toBe("mail service unavailable");
  });

  it("verifies the code, connects the account, and refreshes connectors", async () => {
    invokeMock.mockResolvedValueOnce(signedIn);
    const { result, setActiveView, refreshConnectors } = setup();
    act(() => {
      result.current.setAuthEmail(" person@example.com ");
      result.current.setAuthCode(" 123456 ");
    });
    await act(() => result.current.handleVerifyAuthCode());

    expect(invokeMock).toHaveBeenCalledWith("verify_headroom_auth_code", {
      email: "person@example.com",
      code: "123456",
      inviteCode: null,
    });
    expect(result.current.pricingStatus).toEqual(signedIn);
    expect(result.current.authCode).toBe("");
    expect(setActiveView).toHaveBeenCalledWith("upgrade");
    expect(refreshConnectors).toHaveBeenCalledOnce();
  });

  it("refreshes pricing and reports native failures", async () => {
    invokeMock.mockResolvedValueOnce(signedOut);
    const { result } = setup();
    await act(() => result.current.refreshPricingStatus());
    expect(invokeMock).toHaveBeenCalledWith("get_headroom_pricing_status");
    expect(result.current.pricingStatus).toEqual(signedOut);
    expect(result.current.pricingBusy).toBe(false);

    invokeMock.mockRejectedValueOnce(new Error("pricing offline"));
    await act(() => result.current.refreshPricingStatus());
    expect(result.current.pricingError).toBe("pricing offline");
  });

  it("routes unauthenticated upgrades to auth while internal free returns home", async () => {
    const { result, setActiveView } = setup();
    act(() => result.current.setPricingStatus(signedOut));
    await act(() => result.current.handleUpgradeAction("pro"));
    expect(setActiveView).toHaveBeenCalledWith("upgradeAuth");
    expect(result.current.pendingUpgradePlanId).toBe("pro");

    await act(() => result.current.handleUpgradeAction("free"));
    expect(setActiveView).toHaveBeenLastCalledWith("home");
    expect(trackMock).toHaveBeenCalledWith(
      "upgrade_button_clicked",
      expect.objectContaining({ plan_id: "free", action_kind: "internal" }),
    );
  });

  it("starts checkout with exact plan payload and opens the returned URL", async () => {
    invokeMock.mockResolvedValueOnce("https://checkout.example/session");
    const { result, openExternalLink } = setup();
    act(() => result.current.setPricingStatus(signedIn));
    await act(() => result.current.handleUpgradeAction("max5x"));

    expect(invokeMock).toHaveBeenCalledWith("create_headroom_checkout_session", {
      subscriptionTier: "max5x",
      billingPeriod: "annual",
    });
    expect(openExternalLink).toHaveBeenCalledWith(
      "https://checkout.example/session",
    );
    expect(result.current.checkoutPollingDeadline).not.toBeNull();
    expect(result.current.upgradeActionBusy).toBeNull();
  });

  it("signs out, reloads status, and resets auth state", async () => {
    invokeMock
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(signedOut);
    const { result } = setup();
    act(() => {
      result.current.setAuthEmail("person@example.com");
      result.current.setAuthCode("123456");
    });
    await act(() => result.current.handleSignOutHeadroomAccount());
    expect(invokeMock.mock.calls).toEqual([
      ["sign_out_headroom_account"],
      ["get_headroom_pricing_status"],
    ]);
    expect(result.current.pricingStatus).toEqual(signedOut);
    expect(result.current.authFlowSuccess).toBe("Signed out of Headroom.");
  });

  it("opens a plan-change confirmation and invokes the exact change payload", async () => {
    const active = {
      ...signedIn,
      account: {
        ...signedIn.account,
        subscriptionActive: true,
        subscriptionTier: "pro",
      },
    } as HeadroomPricingStatus;
    invokeMock
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(active);
    const { result, setActiveView } = setup();
    act(() => result.current.setPricingStatus(active));
    await act(() => result.current.handleUpgradeAction("max20x"));
    expect(result.current.pendingPlanChange).toEqual({
      fromTier: "pro",
      toTier: "max20x",
      billingPeriod: "annual",
    });
    await act(() => result.current.confirmPlanChange());
    expect(invokeMock).toHaveBeenNthCalledWith(
      1,
      "change_headroom_subscription_plan",
      { subscriptionTier: "max20x", billingPeriod: "annual" },
    );
    expect(setActiveView).toHaveBeenCalledWith("home");
    await waitFor(() => expect(result.current.pendingPlanChange).toBeNull());
  });

  it("resets authentication state and validates email before verification", async () => {
    const { result } = setup();
    act(() => {
      result.current.setAuthCode("999999");
      result.current.setAuthFlowError("old error");
      result.current.resetUpgradeAuthStep();
    });
    expect(result.current.authCode).toBe("");
    expect(result.current.authFlowError).toBeNull();

    await act(() => result.current.handleVerifyAuthCode());
    expect(result.current.authFlowError).toBe("Enter a valid email address.");
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("surfaces verification and sign-out failures", async () => {
    invokeMock.mockRejectedValueOnce(new Error("invalid code"));
    const { result } = setup();
    act(() => {
      result.current.setAuthEmail("person@example.com");
      result.current.setAuthCode("123456");
    });
    await act(() => result.current.handleVerifyAuthCode());
    expect(result.current.authFlowError).toBe("invalid code");
    expect(result.current.authVerifyBusy).toBe(false);

    invokeMock.mockRejectedValueOnce({ reason: "unknown" });
    await act(() => result.current.handleSignOutHeadroomAccount());
    expect(result.current.authFlowError).toBe("Could not sign out of Headroom.");
  });

  it("opens the billing portal for an active plan and surfaces failures", async () => {
    const active = {
      ...signedIn,
      account: {
        ...signedIn.account,
        subscriptionActive: true,
        subscriptionTier: "pro",
      },
    } as HeadroomPricingStatus;
    invokeMock.mockResolvedValueOnce("https://billing.example/portal");
    const { result, openExternalLink } = setup();
    act(() => result.current.setPricingStatus(active));
    await act(() => result.current.handleUpgradeAction("free"));
    expect(invokeMock).toHaveBeenCalledWith("get_headroom_billing_portal_url", {
      target: "subscription",
    });
    expect(openExternalLink).toHaveBeenCalledWith(
      "https://billing.example/portal",
    );

    invokeMock.mockRejectedValueOnce("portal offline");
    await act(() => result.current.handleUpgradeAction("free"));
    expect(result.current.upgradeActionError).toBe("portal offline");
    expect(result.current.upgradeActionBusy).toBeNull();
  });

  it("opens external sales links and reports browser failures", async () => {
    const { result, openExternalLink } = setup();
    act(() => result.current.setPricingStatus(signedIn));
    await act(() => result.current.handleUpgradeAction("team"));
    expect(openExternalLink).toHaveBeenCalledWith("mailto:hello@example.com");

    openExternalLink.mockRejectedValueOnce(new Error("no mail client"));
    await act(() => result.current.handleUpgradeAction("enterprise"));
    expect(result.current.upgradeActionError).toBe("no mail client");
  });

  it("reports checkout and plan-change failures and supports cancellation", async () => {
    invokeMock.mockRejectedValueOnce({ reason: "checkout unavailable" });
    const { result } = setup();
    act(() => result.current.setPricingStatus(signedIn));
    await act(() => result.current.handleUpgradeAction("pro"));
    expect(result.current.upgradeActionError).toBe("Could not start checkout.");

    const active = {
      ...signedIn,
      account: {
        ...signedIn.account,
        subscriptionActive: true,
        subscriptionTier: "pro",
      },
    } as HeadroomPricingStatus;
    act(() => result.current.setPricingStatus(active));
    await act(() => result.current.handleUpgradeAction("max5x"));
    act(() => result.current.cancelPlanChange());
    expect(result.current.pendingPlanChange).toBeNull();

    await act(() => result.current.handleUpgradeAction("max20x"));
    invokeMock.mockRejectedValueOnce(new Error("change rejected"));
    await act(() => result.current.confirmPlanChange());
    expect(result.current.planChangeError).toBe("change rejected");
    expect(result.current.pendingPlanChange).not.toBeNull();
    expect(result.current.planChangeBusy).toBe(false);
  });

  it("reactivates subscriptions and reports reactivation failures", async () => {
    invokeMock
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(signedIn);
    const { result } = setup();
    await act(() => result.current.handleReactivateSubscription());
    expect(invokeMock.mock.calls).toEqual([
      ["reactivate_headroom_subscription"],
      ["get_headroom_pricing_status"],
    ]);
    expect(result.current.reactivateBusy).toBe(false);

    invokeMock.mockRejectedValueOnce("reactivation denied");
    await act(() => result.current.handleReactivateSubscription());
    expect(result.current.reactivateError).toBe("reactivation denied");
  });

  it("stays offline in local-only mode without invoking pricing commands", async () => {
    runtimeFlags.localOnly = true;
    const { result } = setup();
    await act(() => result.current.refreshPricingStatus());
    expect(invokeMock).not.toHaveBeenCalled();
    expect(result.current.pricingBusy).toBe(false);
    expect(result.current.pricingError).toBeNull();
  });

  it("notifies once for a higher tier mismatch and suppresses repeats", async () => {
    invokeMock.mockResolvedValue(undefined);
    const mismatch = {
      ...signedIn,
      tierMismatch: {
        paidTier: "pro",
        recommendedTier: "max5x",
        recommendedSource: "both",
        graceEndsAt: "2026-01-01T00:00:00.000Z",
        clamped: false,
      },
    } as HeadroomPricingStatus;
    const { result } = setup();
    act(() => result.current.setPricingStatus(mismatch));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith(
        "show_notification",
        expect.objectContaining({ title: "Upgrade your Headroom plan" }),
      ),
    );
    expect(window.localStorage.getItem("headroom:lastNotifiedMismatchTier")).toBe(
      "max5x",
    );
    const notificationCount = invokeMock.mock.calls.filter(
      ([command]) => command === "show_notification",
    ).length;
    act(() => result.current.setPricingStatus({ ...mismatch }));
    expect(
      invokeMock.mock.calls.filter(([command]) => command === "show_notification")
        .length,
    ).toBe(notificationCount);
  });
});
