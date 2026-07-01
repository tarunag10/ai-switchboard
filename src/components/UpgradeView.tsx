import { Sparkle } from "@phosphor-icons/react";
import {
  getFounderStepPricing,
  type BillingPeriod,
  type PricingAudience,
  type UpgradePlanId,
} from "../lib/appHelpers";
import { isValidEmailAddress } from "../lib/launcherHelpers";
import type { HeadroomPricingStatus } from "../lib/types";

export interface UpgradeViewProps {
  pricingAudience: PricingAudience;
  setPricingAudience: (audience: PricingAudience) => void;
  setUpgradeActionError: (error: string | null) => void;
  billingPeriod: BillingPeriod;
  setBillingPeriod: (period: BillingPeriod) => void;
  pricingStatus: HeadroomPricingStatus | null;
  upgradeTrialCallout: {
    tone: string;
    message: string;
    actionLabel?: string;
    onAction?: () => void;
  };
  authRequestBusy: boolean;
  authVerifyBusy: boolean;
  upgradeActionBusy: UpgradePlanId | null;
  upgradePlansState: {
    featuredPlanId: UpgradePlanId;
    plans: Array<{
      id: UpgradePlanId;
      name: string;
      tagline: string;
      price: string;
      originalPrice?: string;
      billingLines: [string, string];
      purchaseInfo?: {
        cancelAtPeriodEnd: boolean;
        endsOn?: string;
        renewsOn?: string;
        paidPerMonthLabel?: string;
        discountPct?: number;
      };
      ctaLabel: string;
      ctaTone?: string;
      ctaVariant?: string;
      disabled?: boolean;
      features: string[];
      centeredPriceLabel?: string;
    }>;
  };
  visibleUpgradePlans: Array<{
    id: UpgradePlanId;
    name: string;
    tagline: string;
    price: string;
    originalPrice?: string;
    billingLines: [string, string];
    purchaseInfo?: {
      cancelAtPeriodEnd: boolean;
      endsOn?: string;
      renewsOn?: string;
      paidPerMonthLabel?: string;
      discountPct?: number;
    };
    ctaLabel: string;
    ctaTone?: string;
    ctaVariant?: string;
    disabled?: boolean;
    features: string[];
    centeredPriceLabel?: string;
  }>;
  activeHeadroomPlanId: UpgradePlanId | null;
  handleContactSubmit: (event: React.FormEvent) => void;
  contactEmail: string;
  setContactEmail: (email: string) => void;
  contactSubmitError: string | null;
  setContactSubmitError: (error: string | null) => void;
  contactSubmitSuccess: string | null;
  setContactSubmitSuccess: (success: string | null) => void;
  contactMessage: string;
  setContactMessage: (message: string) => void;
  contactEmailValid: boolean;
  contactSubmitBusy: boolean;
  handleReactivateSubscription: () => void;
  reactivateBusy: boolean;
  handleUpgradeAction: (planId: UpgradePlanId) => void;
  hasHiddenUpgradePlans: boolean;
  showAllUpgradePlans: boolean;
  setShowAllUpgradePlans: (show: boolean | ((current: boolean) => boolean)) => void;
  upgradeActionError: string | null;
  reactivateError: string | null;
}

export function UpgradeView({
  pricingAudience,
  setPricingAudience,
  setUpgradeActionError,
  billingPeriod,
  setBillingPeriod,
  pricingStatus,
  upgradeTrialCallout,
  authRequestBusy,
  authVerifyBusy,
  upgradeActionBusy,
  upgradePlansState,
  visibleUpgradePlans,
  activeHeadroomPlanId,
  handleContactSubmit,
  contactEmail,
  setContactEmail,
  contactSubmitError,
  setContactSubmitError,
  contactSubmitSuccess,
  setContactSubmitSuccess,
  contactMessage,
  setContactMessage,
  contactEmailValid,
  contactSubmitBusy,
  handleReactivateSubscription,
  reactivateBusy,
  handleUpgradeAction,
  hasHiddenUpgradePlans,
  showAllUpgradePlans,
  setShowAllUpgradePlans,
  upgradeActionError,
  reactivateError,
}: UpgradeViewProps) {
  return (
    <div className="tray-content tray-content--upgrade">
      <section className="upgrade-hero">
        <h1>Plans based on your AI subscription</h1>
        <div
          className="upgrade-toggle"
          aria-label="Upgrade audiences"
          role="tablist"
        >
          {[
            { id: "individual" as const, label: "Individual" },
            { id: "teamEnterprise" as const, label: "Team & Enterprise" },
          ].map((audience) => (
            <button
              key={audience.id}
              aria-selected={pricingAudience === audience.id}
              className={`upgrade-toggle__item${pricingAudience === audience.id ? " is-active" : ""}`}
              onClick={() => {
                setPricingAudience(audience.id);
                setUpgradeActionError(null);
              }}
              role="tab"
              type="button"
            >
              {audience.label}
            </button>
          ))}
        </div>
        {pricingAudience === "individual" ? (
          <div
            className="upgrade-billing-toggle"
            role="group"
            aria-label="Billing period"
          >
            {(["annual", "monthly"] as const).map((period) => (
              <button
                key={period}
                className={`upgrade-billing-toggle__item${billingPeriod === period ? " is-active" : ""}`}
                onClick={() => setBillingPeriod(period)}
                type="button"
              >
                {period === "annual" ? (
                  <>
                    Annual{" "}
                    <span className="upgrade-billing-toggle__save">
                      Save 33%
                    </span>
                  </>
                ) : (
                  "Monthly"
                )}
              </button>
            ))}
          </div>
        ) : null}
      </section>

      {!pricingStatus?.account?.subscriptionActive ? (
        <>
          <section
            className={`upgrade-trial-callout upgrade-trial-callout--${upgradeTrialCallout.tone}`}
          >
            <div className="upgrade-trial-callout__content">
              <p className="upgrade-trial-callout__message">
                {upgradeTrialCallout.message}
              </p>
            </div>
            {upgradeTrialCallout.actionLabel &&
            upgradeTrialCallout.onAction ? (
              <button
                className="primary-button upgrade-trial-callout__button"
                disabled={
                  authRequestBusy ||
                  authVerifyBusy ||
                  upgradeActionBusy !== null
                }
                onClick={() => upgradeTrialCallout.onAction?.()}
                type="button"
              >
                {upgradeTrialCallout.actionLabel}
              </button>
            ) : null}
          </section>

          {pricingStatus?.launchDiscountActive
            ? (() => {
                const cohorts = pricingStatus.pricingCohorts ?? [];
                const active = cohorts.find((c) => c.status === "active");
                const activeLabel = active?.label ?? "Founder";
                const pct =
                  pricingStatus.activePercentOff ?? active?.percentOff ?? 0;
                const spotsLeft = active?.spotsLeft ?? null;
                const capacity = active?.capacity ?? null;
                const totalCapacity = cohorts.reduce(
                  (sum, c) => sum + (c.capacity ?? 0),
                  0,
                );
                const totalFilled = cohorts.reduce((sum, c) => {
                  const cap = c.capacity ?? 0;
                  if (c.status === "sold_out") return sum + cap;
                  if (c.status === "active")
                    return sum + Math.max(0, cap - (c.spotsLeft ?? 0));
                  return sum;
                }, 0);
                const filledPct =
                  totalCapacity > 0
                    ? Math.min(
                        100,
                        Math.round(50 + 50 * (totalFilled / totalCapacity)),
                      )
                    : null;
                const next =
                  cohorts.find((c) => c.status === "upcoming") ?? null;
                const stepPricing = getFounderStepPricing(
                  upgradePlansState.featuredPlanId,
                  billingPeriod,
                  pct,
                  next?.percentOff ?? 0,
                );
                return (
                  <section
                    className="founder-promo"
                    aria-label="Founder pricing"
                  >
                    <div className="founder-promo__main">
                      <p className="founder-promo__intro">
                        <span
                          className="founder-promo__live"
                          aria-hidden="true"
                        />
                        Launch promotion active. Prices rise as{" "}
                        {activeLabel.toLowerCase()} spots fill.
                      </p>
                      <div className="founder-promo__urgency">
                        <div className="founder-promo__count-row">
                          {spotsLeft != null ? (
                            <>
                              <span className="founder-promo__count">
                                {spotsLeft}
                              </span>
                              <span className="founder-promo__count-label">
                                {activeLabel} spots left
                              </span>
                            </>
                          ) : (
                            <span className="founder-promo__count-label">
                              {activeLabel} pricing
                            </span>
                          )}
                        </div>
                        {filledPct != null ? (
                          <div
                            className="founder-promo__bar"
                            role="presentation"
                          >
                            <span
                              className="founder-promo__bar-fill"
                              style={{ width: `${filledPct}%` }}
                            />
                          </div>
                        ) : null}
                      </div>
                    </div>
                    <div className="founder-promo__offer">
                      <div className="founder-promo__steps">
                        <div className="founder-promo__step founder-promo__step--now">
                          <span className="founder-promo__step-tag">
                            Now
                          </span>
                          <span className="founder-promo__step-pct">
                            {pct}% OFF
                          </span>
                          {stepPricing ? (
                            <span className="founder-promo__step-price">
                              {stepPricing.now} / month
                            </span>
                          ) : null}
                        </div>
                        {next ? (
                          <div className="founder-promo__step founder-promo__step--next">
                            <span className="founder-promo__step-tag">
                              Next
                            </span>
                            <span className="founder-promo__step-pct">
                              {next.percentOff > 0
                                ? `${next.percentOff}% OFF`
                                : "Full price"}
                            </span>
                            {stepPricing ? (
                              <span className="founder-promo__step-price">
                                {stepPricing.next} / month
                              </span>
                            ) : null}
                          </div>
                        ) : null}
                      </div>
                      <p className="founder-promo__lock">
                        Your price is locked in for good.
                      </p>
                    </div>
                  </section>
                );
              })()
            : null}
        </>
      ) : null}

      <section
        className={`upgrade-plan-grid${visibleUpgradePlans.length === 1 ? " upgrade-plan-grid--single" : ""}`}
      >
        {visibleUpgradePlans.map((plan) => {
          const isFeatured = plan.id === upgradePlansState.featuredPlanId;
          const downgradeButtonClassName =
            plan.ctaTone === "downgrade"
              ? " upgrade-plan-card__button--downgrade"
              : "";
          const buttonClassName =
            plan.id === "free"
              ? `primary-button upgrade-plan-card__button upgrade-plan-card__button--free${downgradeButtonClassName}`
              : plan.ctaVariant === "primary"
                ? `primary-button upgrade-plan-card__button${downgradeButtonClassName}`
                : `secondary-button upgrade-plan-card__button${downgradeButtonClassName}`;

          const isActivePlan = plan.id === activeHeadroomPlanId;
          return (
            <article
              className={`upgrade-plan-card${isFeatured ? " upgrade-plan-card--featured" : ""}${isActivePlan ? " upgrade-plan-card--active" : ""}`}
              key={plan.id}
            >
              <div className="upgrade-plan-card__top">
                <div className="upgrade-plan-card__title-block">
                  <span
                    className="upgrade-plan-card__icon"
                    aria-hidden="true"
                  >
                    <Sparkle weight={isFeatured ? "fill" : "duotone"} />
                  </span>
                  <div>
                    <h2>
                      {plan.name}
                      {isActivePlan ? (
                        <span className="upgrade-plan-card__active-badge">
                          Active
                        </span>
                      ) : null}
                    </h2>
                    <p>{plan.tagline}</p>
                  </div>
                </div>
                {plan.centeredPriceLabel ? (
                  <div className="upgrade-plan-card__price-note">
                    {plan.centeredPriceLabel}
                  </div>
                ) : (
                  <div className="upgrade-plan-card__price-block">
                    <div>
                      {plan.originalPrice && !activeHeadroomPlanId ? (
                        <div className="upgrade-plan-card__sale-row">
                          <s className="upgrade-plan-card__original-price">
                            {plan.originalPrice}
                          </s>
                          <span className="upgrade-plan-card__sale-badge">
                            {pricingStatus?.activePercentOff ?? 50}% off
                          </span>
                        </div>
                      ) : null}
                      <strong>{plan.price}</strong>
                    </div>
                    <span>
                      {plan.billingLines[0]}
                      <br />
                      {plan.billingLines[1]}
                    </span>
                  </div>
                )}
                {plan.purchaseInfo ? (
                  <p className="upgrade-plan-card__purchase-info">
                    {plan.purchaseInfo.cancelAtPeriodEnd &&
                    plan.purchaseInfo.endsOn
                      ? plan.id === "free"
                        ? `Activates on ${plan.purchaseInfo.endsOn}`
                        : `Downgrades to Free on ${plan.purchaseInfo.endsOn}`
                      : isActivePlan
                        ? (plan.purchaseInfo.discountPct ?? 0) > 0
                          ? `Renews ${plan.purchaseInfo.paidPerMonthLabel}/mo on ${plan.purchaseInfo.renewsOn} (${plan.purchaseInfo.discountPct}% off)`
                          : `Renews ${plan.price}/mo on ${plan.purchaseInfo.renewsOn}`
                        : null}
                  </p>
                ) : null}
              </div>
              <div className="upgrade-plan-card__action">
                {plan.id === "enterprise" ? (
                  <form
                    className="upgrade-plan-card__contact-form"
                    onSubmit={(event) => void handleContactSubmit(event)}
                  >
                    <input
                      className="upgrade-plan-card__contact-input"
                      onChange={(event) => {
                        setContactEmail(event.target.value);
                        if (contactSubmitError) {
                          setContactSubmitError(null);
                        }
                        if (contactSubmitSuccess) {
                          setContactSubmitSuccess(null);
                        }
                      }}
                      placeholder="you@company.com"
                      type="email"
                      value={contactEmail}
                    />
                    <textarea
                      className="upgrade-plan-card__contact-textarea"
                      maxLength={2000}
                      onChange={(event) => {
                        setContactMessage(event.target.value);
                        if (contactSubmitError) {
                          setContactSubmitError(null);
                        }
                        if (contactSubmitSuccess) {
                          setContactSubmitSuccess(null);
                        }
                      }}
                      placeholder="Tell us about your team and what you're looking for (optional)"
                      rows={4}
                      value={contactMessage}
                    />
                    <button
                      className={`secondary-button upgrade-plan-card__button upgrade-plan-card__contact-submit${contactEmailValid ? " is-ready" : ""}`}
                      disabled={!contactEmailValid || contactSubmitBusy}
                      type="submit"
                    >
                      {contactSubmitBusy ? "Sending..." : plan.ctaLabel}
                    </button>
                  </form>
                ) : isActivePlan && plan.purchaseInfo?.cancelAtPeriodEnd ? (
                  <button
                    className={buttonClassName}
                    disabled={reactivateBusy}
                    onClick={() => void handleReactivateSubscription()}
                    type="button"
                  >
                    {reactivateBusy
                      ? "Resuming..."
                      : `Resume ${plan.name} plan`}
                  </button>
                ) : plan.id === "free" &&
                  plan.purchaseInfo?.cancelAtPeriodEnd ? (
                  <button
                    className={buttonClassName}
                    disabled
                    type="button"
                  >
                    {plan.ctaLabel}
                  </button>
                ) : (
                  <button
                    className={buttonClassName}
                    disabled={
                      plan.disabled || upgradeActionBusy === plan.id
                    }
                    onClick={() => void handleUpgradeAction(plan.id)}
                    type="button"
                  >
                    {upgradeActionBusy === plan.id
                      ? "Opening..."
                      : plan.ctaLabel}
                  </button>
                )}
              </div>

              {plan.features.length > 0 ? (
                <div className="upgrade-plan-card__features">
                  <ul>
                    {plan.features.map((feature) => (
                      <li key={feature}>{feature}</li>
                    ))}
                  </ul>
                </div>
              ) : null}
              {plan.id === "enterprise" && contactSubmitError ? (
                <p className="upgrade-plan-card__contact-status upgrade-plan-card__contact-status--error">
                  {contactSubmitError}
                </p>
              ) : null}
              {plan.id === "enterprise" && contactSubmitSuccess ? (
                <p className="upgrade-plan-card__contact-status upgrade-plan-card__contact-status--success">
                  {contactSubmitSuccess}
                </p>
              ) : null}
            </article>
          );
        })}
      </section>
      {pricingAudience === "individual" &&
      (hasHiddenUpgradePlans || showAllUpgradePlans) ? (
        <button
          className="upgrade-plan-grid__toggle"
          onClick={() => setShowAllUpgradePlans((current) => !current)}
          type="button"
        >
          {showAllUpgradePlans ? "show fewer plans" : "show more plans"}
        </button>
      ) : null}

      {upgradeActionError ? (
        <p className="install-progress__error">{upgradeActionError}</p>
      ) : null}
      {reactivateError ? (
        <p className="install-progress__error">{reactivateError}</p>
      ) : null}
    </div>
  );
}
