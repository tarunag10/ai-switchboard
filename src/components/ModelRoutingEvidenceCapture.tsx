import { useState } from "react";

import {
  completeModelRoutingCompletion,
  exportModelRoutingEvidenceForHandle,
  issueModelRoutingCompletionHandle,
  type ModelRoutingEvidenceArtifact,
  type ModelRoutingCompletionHandle,
} from "../lib/optimization";

type CaptureState = {
  runId: string;
  taskClass: string;
  succeeded: boolean;
  costMicrounits: string;
  qualityScoreBps: string;
  latencyMs: string;
  followUpRework: boolean;
  client: string;
  requestedModel: string;
  cheapModel: string;
  capableModel: string;
};

const initialState: CaptureState = {
  runId: "",
  taskClass: "formatting",
  succeeded: true,
  costMicrounits: "",
  qualityScoreBps: "10000",
  latencyMs: "",
  followUpRework: false,
  client: "codex",
  requestedModel: "",
  cheapModel: "",
  capableModel: "",
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
  const [completionHandle, setCompletionHandle] = useState<ModelRoutingCompletionHandle | null>(null);

  const update = <K extends keyof CaptureState>(key: K, value: CaptureState[K]) => {
    setState((current) => ({ ...current, [key]: value }));
    setArtifact(null);
  };

  const exportRun = async () => {
    if (!completionHandle) {
      setNotice("Issue and complete a native completion handle before exporting evidence.");
      return;
    }
    setWorking(true);
    setNotice(null);
    try {
      const next = await exportModelRoutingEvidenceForHandle(
        completionHandle.handleId,
        state.taskClass.trim(),
      );
      setArtifact(next);
      setCompletionHandle(null);
      setNotice("Completion-bound observe-only evidence exported once; automatic routing remains disabled.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setWorking(false);
    }
  };

  const issueHandle = async () => {
    setWorking(true);
    setNotice(null);
    setArtifact(null);
    try {
      const issued = await issueModelRoutingCompletionHandle({
        client: state.client.trim(),
        task: state.taskClass.trim(),
        requestedModel: state.requestedModel.trim(),
        cheapModel: state.cheapModel.trim(),
        capableModel: state.capableModel.trim(),
        enabled: true,
      });
      setCompletionHandle(issued);
      setState((current) => ({ ...current, runId: issued.runId }));
      setNotice("Native completion handle issued; record the provider outcome, then complete it.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setWorking(false);
    }
  };

  const completeHandle = async () => {
    if (!completionHandle) return;
    setWorking(true);
    setNotice(null);
    try {
      await completeModelRoutingCompletion(completionHandle.handleId, {
        succeeded: state.succeeded,
        successfulTaskCostMicrounits: state.succeeded
          ? numericValue(state.costMicrounits, "Successful task cost")
          : null,
        qualityScoreBps: numericValue(state.qualityScoreBps, "Quality score"),
        latencyMs: numericValue(state.latencyMs, "Latency"),
        followUpRework: state.followUpRework,
      });
      setNotice("Provider outcome completed; completion-bound export is now available.");
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
          Client
          <input aria-label="Routing evidence client" value={state.client} onChange={(event) => update("client", event.target.value)} />
        </label>
        <label>
          Native run ID (issued)
          <input aria-label="Native routing run ID" readOnly value={state.runId} />
        </label>
        <label>
          Task class
          <input aria-label="Routing evidence task class" value={state.taskClass} onChange={(event) => update("taskClass", event.target.value)} />
        </label>
        <label>
          Requested model
          <input aria-label="Routing requested model" value={state.requestedModel} onChange={(event) => update("requestedModel", event.target.value)} />
        </label>
        <label>
          Cheap model
          <input aria-label="Routing cheap model" value={state.cheapModel} onChange={(event) => update("cheapModel", event.target.value)} />
        </label>
        <label>
          Capable model
          <input aria-label="Routing capable model" value={state.capableModel} onChange={(event) => update("capableModel", event.target.value)} />
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
        <button className="addon-card__action" disabled={working} onClick={() => void issueHandle()} type="button">Issue completion handle</button>
        <button className="addon-card__action" disabled={working || !completionHandle} onClick={() => void completeHandle()} type="button">Complete provider outcome</button>
        <button className="addon-card__action" disabled={working || completionHandle === null} onClick={() => void exportRun()} type="button">Export completion evidence</button>
      </div>
      {completionHandle ? <p role="status">Active handle: {completionHandle.handleId} ({completionHandle.decision.stage}; {completionHandle.decision.selectedModel})</p> : null}
      {notice ? <p role="status">{notice}</p> : null}
      {artifact ? <pre aria-label="Exported routing evidence">{JSON.stringify(artifact, null, 2)}</pre> : null}
    </section>
  );
}
