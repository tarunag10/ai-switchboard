import { useState } from "react";

import {
  modelRoutingCompletionPort,
  type ModelRoutingDecisionReference,
  type ModelRoutingEvidenceArtifact,
  type ModelRoutingCompletionHandle,
  type ModelRoutingCompletionPort,
  type ModelRoutingEvidenceObservation,
} from "../lib/optimization";

type CaptureState = {
  runId: string;
  taskClass: string;
  succeeded: boolean | null;
  costMicrounits: string;
  qualityScoreBps: string;
  latencyMs: string;
  followUpRework: boolean;
  client: string;
  requestedModel: string;
  cheapModel: string;
  capableModel: string;
};

type ModelRoutingEvidenceCaptureProps = {
  observation?: ModelRoutingEvidenceObservation | null;
  completionPort?: ModelRoutingCompletionPort | null;
};

const initialState: CaptureState = {
  runId: "",
  taskClass: "formatting",
  succeeded: null,
  costMicrounits: "",
  qualityScoreBps: "",
  latencyMs: "",
  followUpRework: false,
  client: "codex",
  requestedModel: "",
  cheapModel: "",
  capableModel: "",
};

function completionBlockReason(state: CaptureState): string | null {
  if (state.succeeded === null) {
    return "Select whether the provider outcome succeeded before completing it.";
  }
  if (!state.qualityScoreBps.trim()) {
    return "Supply a quality score before completing the provider outcome.";
  }
  if (!state.latencyMs.trim()) {
    return "Supply latency before completing the provider outcome.";
  }
  if (state.succeeded && !state.costMicrounits.trim()) {
    return "Supply successful-task cost before completing a successful outcome.";
  }
  return null;
}

function numericValue(value: string, label: string): number {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new Error(`${label} must be a non-negative integer.`);
  }
  return parsed;
}

export function ModelRoutingEvidenceCapture({
  observation,
  completionPort,
}: ModelRoutingEvidenceCaptureProps) {
  const port = completionPort ?? modelRoutingCompletionPort;
  const [state, setState] = useState<CaptureState>(initialState);
  const [working, setWorking] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [artifact, setArtifact] = useState<ModelRoutingEvidenceArtifact | null>(null);
  const [completionHandle, setCompletionHandle] = useState<ModelRoutingCompletionHandle | null>(null);
  const [decisionReference, setDecisionReference] = useState<ModelRoutingDecisionReference | null>(null);
  const completionReady = completionBlockReason(state) === null;

  const update = <K extends keyof CaptureState>(key: K, value: CaptureState[K]) => {
    setState((current) => ({ ...current, [key]: value }));
    setArtifact(null);
    setDecisionReference(null);
  };

  const exportRun = async () => {
    if (!completionHandle) {
      setNotice("Issue and complete a native completion handle before exporting evidence.");
      return;
    }
    setWorking(true);
    setNotice(null);
    try {
      const next = await port.exportModelRoutingEvidenceForHandle(
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
    setDecisionReference(null);
    try {
      const issued = await port.issueModelRoutingCompletionHandle({
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
    const blockReason = completionBlockReason(state);
    if (blockReason) {
      setNotice(blockReason);
      return;
    }
    setWorking(true);
    setNotice(null);
    try {
      const succeeded = state.succeeded;
      if (succeeded === null) {
        throw new Error("Select whether the provider outcome succeeded before completing it.");
      }
      const reference = await port.completeModelRoutingCompletion(completionHandle.handleId, {
        succeeded,
        successfulTaskCostMicrounits: succeeded
          ? numericValue(state.costMicrounits, "Successful task cost")
          : null,
        qualityScoreBps: numericValue(state.qualityScoreBps, "Quality score"),
        latencyMs: numericValue(state.latencyMs, "Latency"),
        followUpRework: state.followUpRework,
      });
      setDecisionReference(reference);
      setNotice("Provider outcome completed. Its native observe-only Router receipt is now available in Workbench.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setWorking(false);
    }
  };

  const recordObservation = async () => {
    if (!observation) {
      setNotice("Supply an explicit routing observation to record evidence.");
      return;
    }
    setWorking(true);
    setNotice(null);
    try {
      await port.recordModelRoutingEvidence(observation);
      setNotice("Supplied routing observation recorded exactly as provided.");
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
          Successful task succeeded?
          <select aria-label="Successful task outcome" value={state.succeeded === null ? "" : state.succeeded ? "succeeded" : "failed"} onChange={(event) => update("succeeded", event.target.value === "" ? null : event.target.value === "succeeded")}>
            <option value="">Unset</option>
            <option value="succeeded">Succeeded</option>
            <option value="failed">Failed</option>
          </select>
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
        <input type="checkbox" checked={state.followUpRework} onChange={(event) => update("followUpRework", event.target.checked)} /> Follow-up rework required
      </label>
      <div className="optimization-evidence-capture__actions">
        <button className="addon-card__action" disabled={working || !observation} onClick={() => void recordObservation()} type="button">Record supplied observation</button>
        <button className="addon-card__action" disabled={working} onClick={() => void issueHandle()} type="button">Issue completion handle</button>
        <button className="addon-card__action" disabled={working || !completionHandle || !completionReady} onClick={() => void completeHandle()} type="button">Complete provider outcome</button>
        <button className="addon-card__action" disabled={working || completionHandle === null} onClick={() => void exportRun()} type="button">Export completion evidence</button>
      </div>
      {completionHandle && !completionReady ? <p role="status">Complete the explicit outcome fields before finishing the provider result.</p> : null}
      {!observation ? <p role="status">No explicit routing observation supplied yet.</p> : null}
      {observation ? (
        <p role="status">
          Supplied observation ready: {observation.runId} · {observation.taskClass} · {observation.arm}
        </p>
      ) : null}
      {completionHandle ? <p role="status">Active handle: {completionHandle.handleId} ({completionHandle.decision.stage}; {completionHandle.decision.selectedModel})</p> : null}
      {decisionReference ? <p role="status"><strong>Workbench Router receipt:</strong> {decisionReference.decisionId} · {decisionReference.routingMode} · {decisionReference.evidenceDigest}</p> : null}
      {notice ? <p role="status">{notice}</p> : null}
      {artifact ? <pre aria-label="Exported routing evidence">{JSON.stringify(artifact, null, 2)}</pre> : null}
    </section>
  );
}
