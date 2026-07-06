import { useState } from "react";

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
  const [baselineTokens, setBaselineTokens] = useState("");
  const [optimizedTokens, setOptimizedTokens] = useState("");
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
      <label>
        <span>Before</span>
        <input
          type="number"
          min="0"
          inputMode="numeric"
          value={baselineTokens}
          disabled={disabled || busy}
          onChange={(event) => setBaselineTokens(event.currentTarget.value)}
        />
      </label>
      <label>
        <span>After</span>
        <input
          type="number"
          min="0"
          inputMode="numeric"
          value={optimizedTokens}
          disabled={disabled || busy}
          onChange={(event) => setOptimizedTokens(event.currentTarget.value)}
        />
      </label>
      <button
        type="button"
        className="addon-card__sample-button"
        disabled={disabled || busy || !baselineTokens || !optimizedTokens}
        onClick={() => void submitMeasuredSample()}
      >
        {busy ? "Recording..." : "Record measured sample"}
      </button>
      {status ? <p>{status}</p> : null}
    </div>
  );
}
