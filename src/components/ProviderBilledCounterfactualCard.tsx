import { useEffect, useState } from "react";

import {
  loadProviderBilledUsageSnapshot,
  recordProviderBilledCounterfactual,
  type ProviderBilledProvider,
} from "../lib/providerBilledCounterfactual";
import {
  DEFAULT_PROVIDER_BILLED_SAMPLING,
  loadProviderBilledSamplingSettings,
  providerBilledSamplingDisclosure,
  saveProviderBilledSamplingSettings,
  shouldRunProviderBilledSample,
} from "../lib/providerBilledScheduledSampling";

interface ProviderBilledCounterfactualCardProps {
  onRecorded: () => Promise<void>;
}

const providers: Array<{ id: ProviderBilledProvider; label: string }> = [
  { id: "headroom_stats", label: "Headroom /stats" },
  { id: "codex", label: "Codex usage" },
  { id: "claude", label: "Claude OAuth usage" },
];

export function ProviderBilledCounterfactualCard({
  onRecorded,
}: ProviderBilledCounterfactualCardProps) {
  const [provider, setProvider] = useState<ProviderBilledProvider>("headroom_stats");
  const [baselineTokens, setBaselineTokens] = useState("");
  const [optimizedTokens, setOptimizedTokens] = useState("");
  const [baselineEvidence, setBaselineEvidence] = useState("");
  const [optimizedEvidence, setOptimizedEvidence] = useState("");
  const [requestDelta, setRequestDelta] = useState("1");
  const [sampling, setSampling] = useState(DEFAULT_PROVIDER_BILLED_SAMPLING);

  useEffect(() => {
    setSampling(loadProviderBilledSamplingSettings());
  }, []);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function captureHeadroomReading(side: "baseline" | "optimized") {
    setStatus(null);
    const reading = await loadProviderBilledUsageSnapshot();
    if (!reading) {
      setStatus("Headroom /stats provider-billed counters are unavailable right now.");
      return;
    }
    const evidence = `${reading.sourceEndpoint}: ${reading.billedInputTokens.toLocaleString()} billed input tokens · observed ${new Date(reading.observedAt).toISOString()}`;
    if (side === "baseline") {
      setBaselineTokens(String(reading.billedInputTokens));
      setBaselineEvidence(evidence);
    } else {
      setOptimizedTokens(String(reading.billedInputTokens));
      setOptimizedEvidence(evidence);
    }
    setProvider("headroom_stats");
  }

  async function submit() {
    setBusy(true);
    setStatus(null);
    try {
      const result = await recordProviderBilledCounterfactual({
        provider,
        baselineTokens: Number(baselineTokens),
        optimizedTokens: Number(optimizedTokens),
        baselineEvidence,
        optimizedEvidence,
        requestDelta: Number(requestDelta),
      });
      if (!result.recorded) {
        setStatus(
          result.reason
            ? `Could not record measured savings (${result.reason}).`
            : "Could not record measured savings.",
        );
        return;
      }
      setStatus(
        `Recorded ${result.tokensSaved.toLocaleString()} measured provider-billed tokens saved across ${result.requestDelta} request(s).`,
      );
      await onRecorded();
    } catch (error: unknown) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <article className="soft-card panel-card provider-billed-card">
      <div className="panel-card__header">
        <div>
          <h3>Provider-billed counterfactual</h3>
          <p>
            Record measured savings only when independent before/after provider-billed
            token readings exist. Incomplete pairs stay estimated.
          </p>
        </div>
      </div>
      <label className="optimize-project-row">
        <span className="optimize-project-row__main">
          <span className="optimize-project-row__name">Provider source</span>
          <select
            aria-label="Provider billed source"
            value={provider}
            onChange={(event) =>
              setProvider(event.target.value as ProviderBilledProvider)
            }
          >
            {providers.map((item) => (
              <option key={item.id} value={item.id}>
                {item.label}
              </option>
            ))}
          </select>
        </span>
      </label>
      <div className="settings-transfer__actions">
        <button
          type="button"
          className="secondary-button secondary-button--small"
          disabled={busy}
          onClick={() => void captureHeadroomReading("baseline")}
        >
          Capture Headroom baseline
        </button>
        <button
          type="button"
          className="secondary-button secondary-button--small"
          disabled={busy}
          onClick={() => void captureHeadroomReading("optimized")}
        >
          Capture Headroom optimized
        </button>
      </div>
      <div className="provider-billed-card__grid">
        <label>
          Baseline billed tokens
          <input
            aria-label="Baseline billed tokens"
            inputMode="numeric"
            value={baselineTokens}
            onChange={(event) => setBaselineTokens(event.target.value)}
          />
        </label>
        <label>
          Optimized billed tokens
          <input
            aria-label="Optimized billed tokens"
            inputMode="numeric"
            value={optimizedTokens}
            onChange={(event) => setOptimizedTokens(event.target.value)}
          />
        </label>
      </div>
      <label className="settings-transfer__textarea-wrap">
        Baseline evidence
        <textarea
          className="settings-transfer__textarea"
          aria-label="Baseline evidence"
          value={baselineEvidence}
          onChange={(event) => setBaselineEvidence(event.target.value)}
        />
      </label>
      <label className="settings-transfer__textarea-wrap">
        Optimized evidence
        <textarea
          className="settings-transfer__textarea"
          aria-label="Optimized evidence"
          value={optimizedEvidence}
          onChange={(event) => setOptimizedEvidence(event.target.value)}
        />
      </label>
      <label className="optimize-project-row">
        <span className="optimize-project-row__main">
          <span className="optimize-project-row__name">Matched requests</span>
          <input
            aria-label="Matched request count"
            inputMode="numeric"
            value={requestDelta}
            onChange={(event) => setRequestDelta(event.target.value)}
          />
        </span>
      </label>
      <div className="settings-transfer__actions">
        <button
          type="button"
          className="secondary-button secondary-button--small"
          disabled={busy}
          onClick={() => void submit()}
        >
          {busy ? "Recording…" : "Record measured savings"}
        </button>
      </div>
      {status ? (
        <p className="optimize-minimal__meta optimize-minimal__meta--notice" role="status">
          {status}
        </p>
      ) : null}
      <section className="provider-billed-card__sampling" aria-label="Scheduled sampling">
        <h4>Weekly counterfactual sampling</h4>
        <p>{providerBilledSamplingDisclosure()}</p>
        <label className="optimize-project-row">
          <span className="optimize-project-row__main">
            <span className="optimize-project-row__name">Opt in to weekly sampling</span>
            <span className="optimize-project-row__meta">
              {sampling.lastSampleAt
                ? `Last sample: ${new Date(sampling.lastSampleAt).toLocaleString()}`
                : "No samples recorded yet."}
              {shouldRunProviderBilledSample(sampling) && sampling.enabled
                ? " A sample is due on the next manual capture."
                : ""}
            </span>
          </span>
          <input
            type="checkbox"
            checked={sampling.enabled}
            onChange={(event) => {
              const next = saveProviderBilledSamplingSettings({
                ...sampling,
                enabled: event.target.checked,
              });
              setSampling(next);
            }}
          />
        </label>
      </section>
    </article>
  );
}
