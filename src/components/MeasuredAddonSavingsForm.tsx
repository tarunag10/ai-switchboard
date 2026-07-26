import { useEffect, useId, useState } from "react";

import {
  recordMeasuredAddonSavings,
  type MeasuredAddonSavingsSource,
} from "../lib/measuredSavingsAttribution";
import { loadTokenXraySnapshot, type TokenXraySnapshot } from "../lib/usageAnalytics";

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
  const [requestDelta, setRequestDelta] = useState("1");
  const [xraySnapshot, setXraySnapshot] = useState<TokenXraySnapshot | null>(null);
  const [xrayError, setXrayError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    void loadTokenXraySnapshot()
      .then((snapshot) => {
        if (active) setXraySnapshot(snapshot);
      })
      .catch(() => {
        if (active) setXrayError("Token X-Ray input evidence is unavailable.");
      });
    return () => {
      active = false;
    };
  }, []);

  const xrayInput = xraySnapshot?.metrics.inputTokens;
  const canCaptureXray =
    xrayInput?.value !== null &&
    xrayInput?.value !== undefined &&
    xrayInput.confidence !== "unavailable";

  function xrayEvidence(snapshot: TokenXraySnapshot, capturedAt: number) {
    const input = snapshot.metrics.inputTokens;
    const observedAt = input.observedAt ?? snapshot.generatedAt;
    const timestamp = observedAt > 0 ? new Date(observedAt).toISOString() : "unavailable";
    return `Token X-Ray input metric: ${input.value!.toLocaleString()} tokens · session ${snapshot.sessionId ?? "unavailable"} · provider ${snapshot.provider ?? "unavailable"} · model ${snapshot.model ?? "unavailable"} · observed ${timestamp} · captured ${new Date(capturedAt).toISOString()}`;
  }

  async function captureXray(side: "baseline" | "optimized") {
    setStatus(null);
    try {
      const snapshot = await loadTokenXraySnapshot();
      const input = snapshot.metrics.inputTokens;
      if (input.value === null || input.confidence === "unavailable") {
        setXraySnapshot(snapshot);
        setStatus("Token X-Ray input tokens are unavailable; enter a credible counter manually.");
        return;
      }
      setXraySnapshot(snapshot);
      const capturedAt = Date.now();
      const evidence = xrayEvidence(snapshot, capturedAt);
      if (side === "baseline") {
        setBaselineTokens(String(Math.floor(input.value)));
        setBaselineEvidence(evidence);
      } else {
        setOptimizedTokens(String(Math.floor(input.value)));
        setOptimizedEvidence(evidence);
      }
    } catch {
      setXrayError("Token X-Ray input evidence is unavailable.");
      setStatus("Token X-Ray input tokens are unavailable; enter a credible counter manually.");
    }
  }

  async function submitMeasuredSample() {
    setBusy(true);
    setStatus(null);
    try {
      const result = await recordMeasuredAddonSavings({
        source,
        label,
        baselineTokens: Number(baselineTokens),
        optimizedTokens: Number(optimizedTokens),
        requestDelta: Number(requestDelta),
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
      setRequestDelta("1");
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
      <div className="addon-card__measurement-capture">
        <button
          type="button"
          className="secondary-button secondary-button--small"
          disabled={disabled || busy || !canCaptureXray}
          onClick={() => void captureXray("baseline")}
        >
          Capture X-Ray into Before
        </button>
        <button
          type="button"
          className="secondary-button secondary-button--small"
          disabled={disabled || busy || !canCaptureXray}
          onClick={() => void captureXray("optimized")}
        >
          Capture X-Ray into After
        </button>
      </div>
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
      <label htmlFor={`${evidenceId}-request-delta`}>
        <span>Request count / delta</span>
        <input
          id={`${evidenceId}-request-delta`}
          type="number"
          min="1"
          step="1"
          inputMode="numeric"
          value={requestDelta}
          disabled={disabled || busy}
          onChange={(event) => setRequestDelta(event.currentTarget.value)}
        />
      </label>
      <p className="addon-card__measurement-note" role="note">
        Capture uses only the current local Token X-Ray input metric. It is disabled when that metric is unavailable; credible local or external counters may be entered manually.
      </p>
      {xrayError ? <p role="status">{xrayError}</p> : null}
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
