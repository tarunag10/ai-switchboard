import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { UpgradeView, type UpgradeViewProps } from "./UpgradeView";

type UpgradePlanFixture = UpgradeViewProps["visibleUpgradePlans"][number];
const basePlan: UpgradePlanFixture = { id: "pro", name: "Pro", tagline: "For builders", price: "$10", billingLines: ["per month", "billed annually"], featureIntro: "Includes", ctaLabel: "Choose Pro", ctaVariant: "primary", ctaTone: "default", disabled: false, features: ["Fast routing"] };
function makeProps(overrides: Partial<UpgradeViewProps> = {}): UpgradeViewProps {
  return {
    pricingAudience: "individual", setPricingAudience: vi.fn(), setUpgradeActionError: vi.fn(), billingPeriod: "annual", setBillingPeriod: vi.fn(), pricingStatus: null,
    upgradeTrialCallout: { tone: "info", message: "Try Pro" }, authRequestBusy: false, authVerifyBusy: false, upgradeActionBusy: null,
    upgradePlansState: { featuredPlanId: "pro", plans: [basePlan] }, visibleUpgradePlans: [basePlan], activeHeadroomPlanId: null,
    handleContactSubmit: vi.fn(), contactEmail: "", setContactEmail: vi.fn(), contactSubmitError: null, setContactSubmitError: vi.fn(), contactSubmitSuccess: null, setContactSubmitSuccess: vi.fn(), contactMessage: "", setContactMessage: vi.fn(), contactEmailValid: false, contactSubmitBusy: false,
    handleReactivateSubscription: vi.fn(), reactivateBusy: false, handleUpgradeAction: vi.fn(), hasHiddenUpgradePlans: false, showAllUpgradePlans: false, setShowAllUpgradePlans: vi.fn(), upgradeActionError: null, reactivateError: null,
    ...overrides,
  } as UpgradeViewProps;
}

describe("UpgradeView alternate plan states", () => {
  it("renders founder cohort urgency and next pricing step", () => {
    render(<UpgradeView {...makeProps({ pricingStatus: {
      launchDiscountActive: true, activePercentOff: 50, account: null,
      pricingCohorts: [
        { label: "Founder", status: "active", percentOff: 50, capacity: 100, spotsLeft: 20 },
        { label: "Early", status: "upcoming", percentOff: 25, capacity: 100, spotsLeft: 100 },
      ],
    } as never })} />);
    expect(screen.getByLabelText("Founder pricing")).toHaveTextContent("20");
    expect(screen.getByLabelText("Founder pricing")).toHaveTextContent("Founder spots left");
    expect(screen.getByText("25% OFF")).toBeVisible();
    expect(screen.getByText("Your price is locked in for good.")).toBeVisible();
  });

  it("hides trial and billing controls for an active team account", () => {
    render(<UpgradeView {...makeProps({ pricingAudience: "teamEnterprise", pricingStatus: { account: { subscriptionActive: true } } as never })} />);
    expect(screen.queryByLabelText("Billing period")).not.toBeInTheDocument();
    expect(screen.queryByText("Try Pro")).not.toBeInTheDocument();
  });

  it("submits enterprise contact details and clears prior feedback while editing", async () => {
    const user = userEvent.setup();
    const enterprise: UpgradePlanFixture = { ...basePlan, id: "enterprise", name: "Enterprise", ctaLabel: "Contact sales", features: [] };
    const props = makeProps({ visibleUpgradePlans: [enterprise], contactEmail: "a@b.com", contactMessage: "Hello", contactEmailValid: true, contactSubmitError: "Old error", contactSubmitSuccess: "Old success" });
    render(<UpgradeView {...props} />);
    await user.type(screen.getByPlaceholderText("you@company.com"), "x");
    await user.type(screen.getByPlaceholderText(/Tell us about your team/), "x");
    fireEvent.submit(screen.getByRole("button", { name: "Contact sales" }).closest("form")!);
    expect(props.setContactEmail).toHaveBeenCalled();
    expect(props.setContactMessage).toHaveBeenCalled();
    expect(props.setContactSubmitError).toHaveBeenCalledWith(null);
    expect(props.setContactSubmitSuccess).toHaveBeenCalledWith(null);
    expect(props.handleContactSubmit).toHaveBeenCalledOnce();
  });

  it("resumes a cancelled active subscription and locks a scheduled free plan", async () => {
    const user = userEvent.setup();
    const purchaseInfo = { cancelAtPeriodEnd: true, endsOn: "Sep 1", renewsOn: "Sep 1", paidPerMonthLabel: "$10", discountPct: 0 };
    const active: UpgradePlanFixture = { ...basePlan, purchaseInfo };
    const free: UpgradePlanFixture = { ...basePlan, id: "free", name: "Free", ctaLabel: "Scheduled", purchaseInfo };
    const props = makeProps({ visibleUpgradePlans: [active, free], activeHeadroomPlanId: "pro" });
    render(<UpgradeView {...props} />);
    expect(screen.getByText("Downgrades to Free on Sep 1")).toBeVisible();
    expect(screen.getByRole("button", { name: "Scheduled" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Resume Pro plan" }));
    expect(props.handleReactivateSubscription).toHaveBeenCalledOnce();
  });

  it("renders centered price, sale, disabled, and empty-feature branches", () => {
    const plans: UpgradePlanFixture[] = [
      { ...basePlan, id: "free", name: "Free", centeredPriceLabel: "Always free", features: [], ctaLabel: "Current", disabled: true },
      { ...basePlan, id: "max5x", name: "Plus", originalPrice: "$20", ctaVariant: "secondary", ctaTone: "downgrade", ctaLabel: "Downgrade" },
    ];
    render(<UpgradeView {...makeProps({ visibleUpgradePlans: plans, pricingStatus: { activePercentOff: 40, account: null } as never })} />);
    expect(screen.getByText("Always free")).toBeVisible();
    expect(screen.getByText("40% off")).toBeVisible();
    expect(screen.getByRole("button", { name: "Current" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Downgrade" })).toHaveClass("upgrade-plan-card__button--downgrade");
  });
});
