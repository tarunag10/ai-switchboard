import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { UpgradeView, type UpgradeViewProps } from "./UpgradeView";

const individualPlan = {
  id: "pro", name: "Individual", tagline: "For one person", price: "$9", billingLines: ["per month", "billed annually"],
  ctaLabel: "Choose Individual", ctaVariant: "primary", ctaTone: "upgrade", disabled: false, features: ["Unlimited optimization"],
} as any;

function props(overrides: Partial<UpgradeViewProps> = {}): UpgradeViewProps {
  return {
    pricingAudience: "individual", setPricingAudience: vi.fn(), setUpgradeActionError: vi.fn(), billingPeriod: "annual",
    setBillingPeriod: vi.fn(), pricingStatus: null, upgradeTrialCallout: { tone: "info", message: "Start trial", actionLabel: "Sign in", onAction: vi.fn() },
    authRequestBusy: false, authVerifyBusy: false, upgradeActionBusy: null,
    upgradePlansState: { featuredPlanId: "pro", plans: [individualPlan] }, visibleUpgradePlans: [individualPlan], activeHeadroomPlanId: null,
    handleContactSubmit: vi.fn(), contactEmail: "", setContactEmail: vi.fn(), contactSubmitError: null, setContactSubmitError: vi.fn(),
    contactSubmitSuccess: null, setContactSubmitSuccess: vi.fn(), contactMessage: "", setContactMessage: vi.fn(), contactEmailValid: false,
    contactSubmitBusy: false, handleReactivateSubscription: vi.fn(), reactivateBusy: false, handleUpgradeAction: vi.fn(),
    hasHiddenUpgradePlans: true, showAllUpgradePlans: false, setShowAllUpgradePlans: vi.fn(), upgradeActionError: null, reactivateError: null,
    ...overrides,
  } as UpgradeViewProps;
}

describe("UpgradeView integration", () => {
  it("wires audience, billing, trial, plan, and expansion actions", async () => {
    const user = userEvent.setup();
    const p = props();
    render(<UpgradeView {...p} />);
    await user.click(screen.getByRole("tab", { name: "Team & Enterprise" }));
    await user.click(screen.getByRole("button", { name: "Monthly" }));
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    await user.click(screen.getByRole("button", { name: "Choose Individual" }));
    await user.click(screen.getByRole("button", { name: "show more plans" }));
    expect(p.setPricingAudience).toHaveBeenCalledWith("teamEnterprise");
    expect(p.setUpgradeActionError).toHaveBeenCalledWith(null);
    expect(p.setBillingPeriod).toHaveBeenCalledWith("monthly");
    expect(p.upgradeTrialCallout.onAction).toHaveBeenCalledOnce();
    expect(p.handleUpgradeAction).toHaveBeenCalledWith("pro");
    expect(p.setShowAllUpgradePlans).toHaveBeenCalledWith(expect.any(Function));
  });

  it("disables in-flight actions and presents operation errors", async () => {
    const user = userEvent.setup();
    const p = props({ upgradeActionBusy: "pro", authRequestBusy: true, upgradeActionError: "Checkout failed", reactivateError: "Resume failed" });
    render(<UpgradeView {...p} />);
    const trial = screen.getByRole("button", { name: "Sign in" });
    const plan = screen.getByRole("button", { name: "Opening..." });
    expect(trial).toBeDisabled();
    expect(plan).toBeDisabled();
    expect(screen.getByText("Checkout failed")).toBeInTheDocument();
    expect(screen.getByText("Resume failed")).toBeInTheDocument();
    await user.click(plan);
    expect(p.handleUpgradeAction).not.toHaveBeenCalled();
  });
});
