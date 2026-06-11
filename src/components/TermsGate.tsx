import { useState } from "react";

import { invoke } from "@tauri-apps/api/core";

import headroomLogo from "../assets/headroom-logo.svg";

export interface TermsGateProps {
  /// The terms version the user is accepting (DashboardState.requiredTermsVersion).
  requiredVersion: number;
  /// Canonical Terms URL to open in the browser.
  termsUrl: string;
  /// Called after acceptance is persisted so the host can drop the gate.
  onAccepted: () => void;
}

/// Full-window blocking gate shown until the user accepts the current Terms of
/// Service. Rendered in both the launcher and the main window, so new installs
/// and updating users alike must accept before reaching any other UI.
export function TermsGate({ requiredVersion, termsUrl, onAccepted }: TermsGateProps) {
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
      // Local acceptance failing is unexpected; re-enable the button so the
      // user can retry rather than getting stuck behind the gate.
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
        <img className="terms-gate__logo" src={headroomLogo} alt="" aria-hidden="true" />
        <h1 id="terms-gate-title" className="terms-gate__title">
          We&apos;ve updated our Terms
        </h1>
        <p className="terms-gate__copy">
          Please review and accept our Terms of Service to continue using
          Headroom.
        </p>
        <button
          type="button"
          className="terms-gate__link"
          onClick={() => void invoke("open_external_link", { url: termsUrl })}
        >
          Read the full Terms
        </button>
        <label className="terms-gate__consent">
          <input
            type="checkbox"
            checked={checked}
            onChange={(event) => setChecked(event.target.checked)}
          />
          <span>I have read and accept the Terms of Service.</span>
        </label>
        <button
          type="button"
          className="primary-button primary-button--large terms-gate__accept"
          disabled={!checked || accepting}
          onClick={() => void handleAccept()}
        >
          {accepting ? "Saving…" : "Accept & Continue"}
        </button>
      </div>
    </div>
  );
}
