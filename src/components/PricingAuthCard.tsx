import { EnvelopeSimple, Key } from "@phosphor-icons/react";

interface PricingAuthCardProps {
  authCode: string;
  authCodeRequestedFor: string | null;
  authCodeValid: boolean;
  authEmail: string;
  authEmailValid: boolean;
  authFlowError: string | null;
  authFlowSuccess: string | null;
  authRequestBusy: boolean;
  authVerifyBusy: boolean;
  onAuthCodeChange: (value: string) => void;
  onAuthEmailChange: (value: string) => void;
  onRequestAuthCode: () => void;
  onResetAuthStep: () => void;
  onVerifyAuthCode: () => void;
  pricingError: string | null;
  upgradeAuthMessage: string;
}

export function PricingAuthCard({
  authCode,
  authCodeRequestedFor,
  authCodeValid,
  authEmail,
  authEmailValid,
  authFlowError,
  authFlowSuccess,
  authRequestBusy,
  authVerifyBusy,
  onAuthCodeChange,
  onAuthEmailChange,
  onRequestAuthCode,
  onResetAuthStep,
  onVerifyAuthCode,
  pricingError,
  upgradeAuthMessage,
}: PricingAuthCardProps) {
  return (
    <section className="pricing-auth-card pricing-auth-card--standalone">
      <div className="pricing-auth-card__header">
        <div>
          <h2>{upgradeAuthMessage}.</h2>
        </div>
      </div>
      {!authCodeRequestedFor ? (
        <>
          <div className="pricing-auth-card__grid pricing-auth-card__grid--single">
            <label className="pricing-auth-field">
              <span>Email</span>
              <div className="pricing-auth-field__input">
                <EnvelopeSimple size={16} weight="bold" />
                <input
                  onChange={(event) => onAuthEmailChange(event.target.value)}
                  placeholder="you@example.com"
                  type="email"
                  value={authEmail}
                />
              </div>
            </label>
          </div>
          <div className="pricing-auth-card__actions">
            <button
              className="primary-button"
              disabled={!authEmailValid || authRequestBusy}
              onClick={onRequestAuthCode}
              type="button"
            >
              {authRequestBusy ? "Sending..." : "Sign in"}
            </button>
          </div>
          <p className="pricing-auth-card__legal">
            By signing in, you agree to the AI Switchboard Terms of Use shown at
            launch.
          </p>
        </>
      ) : (
        <>
          <div className="pricing-auth-card__code-step">
            <p className="pricing-auth-card__step-copy">
              Enter the authentication code we sent to{" "}
              <strong>{authCodeRequestedFor}</strong>.
            </p>
            <button
              className="link-button pricing-auth-card__change-email"
              onClick={onResetAuthStep}
              type="button"
            >
              Use a different email
            </button>
          </div>
          <div className="pricing-auth-card__grid pricing-auth-card__grid--single">
            <label className="pricing-auth-field">
              <span>Authentication code</span>
              <div className="pricing-auth-field__input">
                <Key size={16} weight="bold" />
                <input
                  onChange={(event) => onAuthCodeChange(event.target.value)}
                  placeholder={`Enter the code sent to ${authCodeRequestedFor}`}
                  type="text"
                  value={authCode}
                />
              </div>
            </label>
          </div>
          <div className="pricing-auth-card__actions">
            <button
              className="primary-button"
              disabled={!authCodeValid || authVerifyBusy}
              onClick={onVerifyAuthCode}
              type="button"
            >
              {authVerifyBusy ? "Verifying..." : "Verify and continue"}
            </button>
            <p className="pricing-auth-card__resend">
              Didn't receive a code?{" "}
              <button
                className="link-button"
                disabled={authRequestBusy}
                onClick={onRequestAuthCode}
                type="button"
              >
                {authRequestBusy ? "Sending..." : "Resend code"}
              </button>
            </p>
          </div>
        </>
      )}
      {authFlowError ? (
        <p className="install-progress__error">{authFlowError}</p>
      ) : null}
      {authFlowSuccess ? (
        <p className="upgrade-plan-card__contact-status upgrade-plan-card__contact-status--success">
          {authFlowSuccess}
        </p>
      ) : null}
      {pricingError ? (
        <p className="install-progress__error">{pricingError}</p>
      ) : null}
    </section>
  );
}
