import { ArrowClockwise, CheckCircle, Database, TerminalWindow, WarningCircle } from "@phosphor-icons/react";
import { useState } from "react";
import {
  type ModelRoutingValidationReceipt,
  type PromptCacheClientProof,
  validateModelRouting,
} from "../lib/optimization";

function statusIcon(status: string) {
  if (status === "blocked") {
    return <WarningCircle weight="duotone" aria-hidden="true" />;
  }
  return <CheckCircle weight="duotone" aria-hidden="true" />;
}

export function OptimizationStatusIcon({ status }: { status: string }) {
  return statusIcon(status);
}

export function PromptCacheClientProofList({
  clients,
}: {
  clients: PromptCacheClientProof[];
}) {
  return (
    <section className="optimize-minimal" aria-labelledby="cache-proof-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <Database weight="duotone" />
        </span>
        <h2 id="cache-proof-title">Cache Proof</h2>
      </div>
      <p className="optimize-minimal__meta">
        Provider cache reads by client. Rows appear only after provider usage telemetry is recorded.
      </p>
      {clients.length === 0 ? (
        <p className="optimize-minimal__meta">No provider cache telemetry yet.</p>
      ) : (
        <div className="optimize-projects">
          {clients.map((client) => (
            <div key={`${client.client}-${client.provider}`} className="optimize-project-row">
              <div className="optimize-project-row__main">
                <span className="optimize-project-row__name">{client.client}</span>
                <span className="optimize-project-row__training">
                  {client.provider} {client.efficiencyPercent}% efficient
                </span>
                <span className="optimize-minimal__meta">{client.proof}</span>
              </div>
              <span className="optimize-project-row__training">
                {client.cacheReadTokens.toLocaleString()} cache hits
              </span>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

export function RoutingValidationPanel() {
  const [receipt, setReceipt] = useState<ModelRoutingValidationReceipt | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function runValidation() {
    setBusy(true);
    setError(null);
    try {
      setReceipt(await validateModelRouting());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="optimize-minimal" aria-labelledby="routing-validation-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <TerminalWindow weight="duotone" />
        </span>
        <h2 id="routing-validation-title">Routing Validation</h2>
      </div>
      <p className="optimize-minimal__meta">
        One-click read-only proof that managed clients route trivial work to the cheaper model candidate.
      </p>
      <button
        className="secondary-button secondary-button--small"
        type="button"
        onClick={() => void runValidation()}
        disabled={busy}
      >
        <ArrowClockwise weight="bold" size={12} aria-hidden="true" />
        {busy ? "Validating" : "Validate routing"}
      </button>
      {error ? <p className="optimize-minimal__meta">{error}</p> : null}
      {receipt ? (
        <div className="optimize-projects">
          {receipt.checks.map((check) => (
            <div key={`${check.client}-${check.task}`} className="optimize-project-row">
              <div className="optimize-project-row__main">
                <span className="optimize-project-row__name">{check.client}</span>
                <span className="optimize-project-row__training">
                  {check.status}: {check.selectedModel}
                </span>
                <span className="optimize-minimal__meta">{check.reason}</span>
              </div>
              <span className="optimize-project-row__training">
                fallback {check.fallbackModel}
              </span>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
