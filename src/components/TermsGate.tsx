import { useState } from "react";

import { invoke } from "@tauri-apps/api/core";

import macAiSwitchboardLogo from "../assets/mac-ai-switchboard-logo.png";

export interface TermsGateProps {
  /// The terms version the user is accepting (DashboardState.requiredTermsVersion).
  requiredVersion: number;
  /// Called after acceptance is persisted so the host can drop the gate.
  onAccepted: () => void;
}

/// Full-window blocking gate shown until the user accepts the current Terms of
/// Service. Rendered in both the launcher and the main window, so new installs
/// and updating users alike must accept before reaching any other UI.
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
        <img
          className="terms-gate__logo"
          src={macAiSwitchboardLogo}
          alt=""
          aria-hidden="true"
        />
        <h1 id="terms-gate-title" className="terms-gate__title">
          Mac AI Switchboard Terms of Use
        </h1>
        <p className="terms-gate__copy">
          Please review and accept these Terms of Use to continue using Mac AI
          Switchboard.
        </p>
        <div className="terms-gate__terms" aria-label="Terms of Use summary">
          <p>
            Mac AI Switchboard is a local desktop utility for managing AI tool
            routing, helper runtimes, shell-output compression, and related
            workflow automation on this Mac.
          </p>
          <p>
            You are responsible for the tools, accounts, API keys, prompts,
            outputs, and local files you connect to the app. Review generated or
            transformed content before relying on it.
          </p>
          <p>
            The app is provided as-is, without warranties. It may configure
            local development tools and network endpoints at your request; you
            can disable those integrations or uninstall the app at any time.
          </p>
          <p>
            Do not use the app for unlawful activity or to bypass third-party
            service terms. Continued use means you accept these Terms of Use.
          </p>
        </div>
        <label className="terms-gate__consent">
          <input
            type="checkbox"
            checked={checked}
            onChange={(event) => setChecked(event.target.checked)}
          />
          <span>I have read and accept the Mac AI Switchboard Terms of Use.</span>
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
