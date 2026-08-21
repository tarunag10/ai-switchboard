import { useState } from "react";

import {
  exportModelRoutingEvidence,
  recordModelRoutingEvidence,
  type ModelRoutingEvidenceArm,
  type ModelRoutingEvidenceArtifact,
} from "../lib/optimization";

type CaptureState = {
  runId: string;
  taskClass: string;
  arm: ModelRoutingEvidenceArm;
  baselineModel: string;
  candidateModel: string;
  succeeded: boolean;
  costMicrounits: string;
  qualityScoreBps: string;
  latencyMs: string;
  followUpRework: boolean;
};

const initialState: CaptureState = {
  runId: "",
  taskClass: "formatting",
  arm: "baseline",
  baselineModel: "",
  candidateModel: "",
  succeeded: true,
  costMicrounits: "",
  qualityScoreBps: "10000",
  latencyMs: "",
  followUpRework: false,
};

function numericValue(value: string, label: string): number {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new Error(`${label} must be a non-negative integer.`);
  }
  return parsed;
}

export function ModelRoutingEvidenceCapture() {
  const [state, setState] = useState<CaptureState>(initialState);
  const [working, setWorking] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [artifact, setArtifact] = useState<ModelRoutingEvidenceArtifact | null>(null);

  const update = <K extends keyof CaptureState>(key: K, value: CaptureState[K]) => {
    setState((current) => ({ ...current, [key]: value }));
    setArtifact(null);
  };

  const record = async () => {
    setWorking(true);
    setNotice(null);
    setArtifact(null);
    try {
      await recordModelRoutingEvidence({
        runId: state.runId.trim(),
        capturedAt: new Date().toISOString(),
        taskClass: state.taskClass.trim(),
        arm: state.arm,
        baselineModel: state.baselineModel.trim(),
        candidateModel: state.candidateModel.trim(),
        succeeded: state.succeeded,
        successfulTaskCostMicrounits: state.succeeded
          ? numericValue(state.costMicrounits, "Successful task cost")
          : null,
        qualityScoreBps: numericValue(state.qualityScoreBps, "Quality score"),
        latencyMs: numericValue(state.latencyMs, "Latency"),
        followUpRework: state.followUpRework,
      });
      setNotice("Redacted routing observation recorded locally.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setWorking(false);
    }
  };

  const exportRun = async () => {
    setWorking(true);
    setNotice(null);
    try {
      const next = await exportModelRoutingEvidence(state.runId.trim(), state.taskClass.trim());
      setArtifact(next);
      setNotice("Observe-only routing evidence exported; automatic routing remains disabled.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setWorking(false);
    }
  };

  return (
    <section aria-labelledby="model-routing-evidence-title" className="optimization-evidence-capture">
      <h4 id="model-routing-evidence-title">Redacted evidence capture</h4>
      <p>
        Record outcome metrics only. Prompts, responses, credentials, and request content are never captured.
        Exported local evidence is permanently observe-only.
      </p>
      <div className="optimization-evidence-capture__grid">
        <label>
          Run ID
          <input aria-label="Routing evidence run ID" value={state.runId} onChange={(event) => update("runId", event.target.value)} />
        </label>
        <label>
          Task class
          <input aria-label="Routing evidence task class" value={state.taskClass} onChange={(event) => update("taskClass", event.target.value)} />
        </label>
        <label>
          Arm
          <select aria-label="Routing evidence arm" value={state.arm} onChange={(event) => update("arm", event.target.value as ModelRoutingEvidenceArm)}>
            <option value="baseline">Baseline</option>
            <option value="candidate">Candidate</option>
          </select>
        </label>
        <label>
          Baseline model ID
          <input aria-label="Baseline model ID" value={state.baselineModel} onChange={(event) => update("baselineModel", event.target.value)} />
        </label>
        <label>
          Candidate model ID
          <input aria-label="Candidate model ID" value={state.candidateModel} onChange={(event) => update("candidateModel", event.target.value)} />
        </label>
        <label>
          Successful-task cost (microunits)
          <input aria-label="Successful task cost" inputMode="numeric" value={state.costMicrounits} onChange={(event) => update("costMicrounits", event.target.value)} />
        </label>
        <label>
          Quality score (basis points)
          <input aria-label="Quality score" inputMode="numeric" value={state.qualityScoreBps} onChange={(event) => update("qualityScoreBps", event.target.value)} />
        </label>
        <label>
          Latency (milliseconds)
          <input aria-label="Latency" inputMode="numeric" value={state.latencyMs} onChange={(event) => update("latencyMs", event.target.value)} />
        </label>
      </div>
      <label>
        <input type="checkbox" checked={state.succeeded} onChange={(event) => update("succeeded", event.target.checked)} /> Successful task
      </label>
      <label>
        <input type="checkbox" checked={state.followUpRework} onChange={(event) => update("followUpRework", event.target.checked)} /> Follow-up rework required
      </label>
      <div className="optimization-evidence-capture__actions">
        <button className="addon-card__action addon-card__action--primary" disabled={working} onClick={() => void record()} type="button">Record observation</button>
        <button className="addon-card__action" disabled={working} onClick={() => void exportRun()} type="button">Export run evidence</button>
      </div>
      {notice ? <p role="status">{notice}</p> : null}
      {artifact ? <pre aria-label="Exported routing evidence">{JSON.stringify(artifact, null, 2)}</pre> : null}
    </section>
  );
}
