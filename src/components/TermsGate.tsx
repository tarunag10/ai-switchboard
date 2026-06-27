import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import macAiSwitchboardLogo from "../assets/mac-ai-switchboard-logo.png";
import {
  privacyNoticeParagraphs,
  privacyNoticeTitle,
  termsOfUseParagraphs,
  termsOfUseTitle,
} from "../lib/legalText";

export interface TermsGateProps {
  /// terms version user is accepting (DashboardState.requiredTermsVersion).
  requiredVersion: number;
  /// Called after acceptance is persisted so the host can drop the gate.
  onAccepted: () => void;
}

/// Full-window blocking gate shown until the user accepts the current legal
/// notices. Rendered in both launcher and main windows so new installs and
/// updating users must accept before any other UI.
export function TermsGate({ requiredVersion, onAccepted }: TermsGateProps) {
  const [checked, setChecked] = useState(false);
  const [accepting, setAccepting] = useState(false);

  async function handleAccept() {
    if (!checked || accepting) {
      return;
    }
    setAccepting(true);
    try {
      await invoke("accept_terms", { version: requiredVersion });
      onAccepted();
    } catch {
      // Local acceptance failing is unexpected; re-enable retry rather than trapping the user.
      setAccepting(false);
    }
  }

  return (
    <div
      className="terms-gate"
      role="dialog"
      aria-modal="true"
      aria-labelledby="terms-gate-title"
    >
      <div className="terms-gate__panel">
        <img
          className="terms-gate__logo"
          src={macAiSwitchboardLogo}
          alt=""
          aria-hidden="true"
        />
        <h1 id="terms-gate-title" className="terms-gate__title">
          {termsOfUseTitle}
        </h1>
        <p className="terms-gate__copy">
          Please review and accept the bundled legal notices to continue using
          Mac AI Switchboard.
        </p>
        <div className="terms-gate__terms" aria-label="Legal notices">
          <section aria-labelledby="terms-gate-terms-title">
            <h2 id="terms-gate-terms-title">{termsOfUseTitle}</h2>
            {termsOfUseParagraphs.map((paragraph) => (
              <p key={paragraph}>{paragraph}</p>
            ))}
          </section>
          <section aria-labelledby="terms-gate-privacy-title">
            <h2 id="terms-gate-privacy-title">{privacyNoticeTitle}</h2>
            {privacyNoticeParagraphs.map((paragraph) => (
              <p key={paragraph}>{paragraph}</p>
            ))}
          </section>
        </div>
        <label className="terms-gate__consent">
          <input
            type="checkbox"
            checked={checked}
            onChange={(event) => setChecked(event.currentTarget.checked)}
          />
          <span>
            I have read and accept the Mac AI Switchboard Terms of Use and
            Privacy Notice.
          </span>
        </label>
        <button
          type="button"
          className="primary-button primary-button--large terms-gate__accept"
          disabled={!checked || accepting}
          onClick={() => void handleAccept()}
        >
          {accepting ? "Saving..." : "Accept & Continue"}
        </button>
      </div>
    </div>
  );
}
