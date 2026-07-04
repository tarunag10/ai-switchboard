import React from "react";
import ReactDOM from "react-dom/client";
import * as Sentry from "@sentry/react";
import Clarity from "@microsoft/clarity";
import App from "./App";
import { remoteTelemetryEnabled } from "./lib/localMode";
import "./styles.css";
import "./switchboard-theme.css";

const telemetryEnabled = remoteTelemetryEnabled();
const isEEA = Intl.DateTimeFormat().resolvedOptions().timeZone.startsWith("Europe/");

// Clarity is only enabled outside the EEA/UK to avoid GDPR consent requirements.
// Microsoft's own FAQ confirms explicit consent is required for EEA users.
if (telemetryEnabled && !isEEA && import.meta.env.VITE_CLARITY_PROJECT_ID) {
  Clarity.init(import.meta.env.VITE_CLARITY_PROJECT_ID);
  Clarity.consent(!document.hidden);
  document.addEventListener("visibilitychange", () => {
    Clarity.consent(!document.hidden);
  });
}

if (telemetryEnabled && import.meta.env.VITE_SENTRY_DSN) {
  Sentry.init({
    dsn: import.meta.env.VITE_SENTRY_DSN,
    integrations: [Sentry.browserTracingIntegration()],
    tracesSampleRate: 0.1,
  });
}

function hideBootLoading() {
  const bootLoading = document.getElementById("boot-loading");
  if (!bootLoading) {
    return;
  }
  bootLoading.classList.add("boot-loading--done");
  window.setTimeout(() => {
    bootLoading.remove();
  }, 280);
}

window.addEventListener("headroom:boot-complete", () => {
  window.requestAnimationFrame(() => {
    hideBootLoading();
  });
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {telemetryEnabled ? (
      <Sentry.ErrorBoundary fallback={<p>Something went wrong.</p>}>
        <App />
      </Sentry.ErrorBoundary>
    ) : (
      <App />
    )}
  </React.StrictMode>
);
