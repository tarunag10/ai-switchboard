import { useId, useState } from "react";

import {
  recordMeasuredAddonSavings,
  type MeasuredAddonSavingsSource,
} from "../lib/measuredSavingsAttribution";

interface MeasuredAddonSavingsFormProps {
  source: MeasuredAddonSavingsSource;
  label: string;
  onRecorded: () => Promise<void>;
  disabled?: boolean;
}

export function MeasuredAddonSavingsForm({
  source,
  label,
  onRecorded,
  disabled = false,
}: MeasuredAddonSavingsFormProps) {
  const evidenceId = useId();
  const [baselineTokens, setBaselineTokens] = useState("");
  const [optimizedTokens, setOptimizedTokens] = useState("");
  const [baselineEvidence, setBaselineEvidence] = useState("");
  const [optimizedEvidence, setOptimizedEvidence] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submitMeasuredSample() {
    setBusy(true);
    setStatus(null);
    try {
      const result = await recordMeasuredAddonSavings({
        source,
        label,
        baselineTokens: Number(baselineTokens),
        optimizedTokens: Number(optimizedTokens),
        measurementEvidence: {
          baseline: baselineEvidence,
          optimized: optimizedEvidence,
        },
        detail: `${label} before/after token sample recorded from the Addons panel.`,
      });
      if (!result.recorded) {
        setStatus("Sample was not recorded because the optimized count must be lower.");
        return;
      }
      await onRecorded();
      setStatus(`${result.tokensSaved.toLocaleString()} tokens recorded.`);
      setBaselineTokens("");
      setOptimizedTokens("");
      setBaselineEvidence("");
      setOptimizedEvidence("");
    } catch (error) {
      setStatus(
        error instanceof Error
          ? error.message
          : "Measured savings sample could not be recorded.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="addon-card__measured-sample">
      <p className="addon-card__measurement-note">
        Measured savings require evidence for both token counts. Until a
        before/after pair is recorded, this add-on’s savings remain estimated.
      </p>
      <label htmlFor={`${evidenceId}-baseline-tokens`}>
        <span>Before</span>
        <input
          id={`${evidenceId}-baseline-tokens`}
          type="number"
          min="0"
          inputMode="numeric"
          value={baselineTokens}
          disabled={disabled || busy}
          onChange={(event) => setBaselineTokens(event.currentTarget.value)}
        />
      </label>
      <label htmlFor={`${evidenceId}-optimized-tokens`}>
        <span>After</span>
        <input
          id={`${evidenceId}-optimized-tokens`}
          type="number"
          min="0"
          inputMode="numeric"
          value={optimizedTokens}
          disabled={disabled || busy}
          onChange={(event) => setOptimizedTokens(event.currentTarget.value)}
        />
      </label>
      <label htmlFor={`${evidenceId}-baseline-evidence`}>
        <span>Baseline evidence</span>
        <input
          id={`${evidenceId}-baseline-evidence`}
          type="text"
          value={baselineEvidence}
          disabled={disabled || busy}
          placeholder="Where the before count came from"
          onChange={(event) => setBaselineEvidence(event.currentTarget.value)}
        />
      </label>
      <label htmlFor={`${evidenceId}-optimized-evidence`}>
        <span>Optimized evidence</span>
        <input
          id={`${evidenceId}-optimized-evidence`}
          type="text"
          value={optimizedEvidence}
          disabled={disabled || busy}
          placeholder="Where the after count came from"
          onChange={(event) => setOptimizedEvidence(event.currentTarget.value)}
        />
      </label>
      <button
        type="button"
        className="addon-card__sample-button"
        disabled={
          disabled ||
          busy ||
          !baselineTokens ||
          !optimizedTokens ||
          !baselineEvidence.trim() ||
          !optimizedEvidence.trim()
        }
        onClick={() => void submitMeasuredSample()}
      >
        {busy ? "Recording..." : "Record measured sample"}
      </button>
      {status ? <p role="status">{status}</p> : null}
    </div>
  );
}
