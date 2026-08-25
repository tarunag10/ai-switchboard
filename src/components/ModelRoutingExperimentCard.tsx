import { useEffect, useState } from "react";

import {
  defaultModelRoutingExperimentPolicy,
  getModelRoutingEffectiveStageReceipt,
  listModelRoutingPolicyPresets,
  loadModelRoutingExperimentPolicy,
  modelRoutingEffectiveStageReceipt,
  saveModelRoutingExperimentPolicy,
  type ModelRoutingEvidenceObservation,
  type ModelRoutingEffectiveStageReceipt,
  type ModelRoutingExperimentPolicy,
  type ModelRoutingPolicyPreset,
  type ModelRoutingStage,
} from "../lib/optimization";
import { ModelRoutingEvidenceCapture } from "./ModelRoutingEvidenceCapture";

const taskClasses = ["formatting", "commit_message", "rename", "diff_summary"] as const;

type ModelRoutingExperimentCardProps = {
  evidenceObservation?: ModelRoutingEvidenceObservation | null;
};

export function ModelRoutingExperimentCard({
  evidenceObservation,
}: ModelRoutingExperimentCardProps) {
  const [policy, setPolicy] = useState<ModelRoutingExperimentPolicy>(
    defaultModelRoutingExperimentPolicy,
  );
  const [presets, setPresets] = useState<ModelRoutingPolicyPreset[]>([]);
  const [effectiveStage, setEffectiveStage] = useState<ModelRoutingEffectiveStageReceipt>(
    () => modelRoutingEffectiveStageReceipt(defaultModelRoutingExperimentPolicy),
  );
  const [disabledClients, setDisabledClients] = useState("");
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    void Promise.all([loadModelRoutingExperimentPolicy(), listModelRoutingPolicyPresets()]).then(([loaded, availablePresets]) => {
      setPolicy(loaded);
      setDisabledClients(loaded.disabledClients.join(", "));
      setPresets(availablePresets);
    });
  }, []);

  useEffect(() => {
    let current = true;
    void getModelRoutingEffectiveStageReceipt(policy).then((receipt) => {
      if (current) setEffectiveStage(receipt);
    });
    return () => { current = false; };
  }, [policy]);

  const toggleTaskClass = (taskClass: string) => {
    setPolicy((current) => ({
      ...current,
      automaticTaskAllowlist: current.automaticTaskAllowlist.includes(taskClass)
        ? current.automaticTaskAllowlist.filter((value) => value !== taskClass)
        : [...current.automaticTaskAllowlist, taskClass],
    }));
  };

  const loadPreset = (preset: ModelRoutingPolicyPreset) => {
    setPolicy(preset.policy);
    setDisabledClients(preset.policy.disabledClients.join(", "));
    setNotice(`${preset.label} loaded as an unsaved draft. Save routing policy explicitly to persist it.`);
  };

  const save = async () => {
    setSaving(true);
    setNotice(null);
    try {
      const next = {
        ...policy,
        disabledClients: disabledClients
          .split(",")
          .map((value) => value.trim())
          .filter(Boolean),
      };
      const stored = await saveModelRoutingExperimentPolicy(next);
      setPolicy(stored);
      setDisabledClients(stored.disabledClients.join(", "));
      setNotice("Model-routing experiment policy saved locally.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <article className="soft-card panel-card">
      <div className="panel-card__header">
        <div>
          <h3>Model-routing experiment</h3>
          <p>
            Observe is the default. User-approved routes require approval for
            that request; automatic routes also require an allowlisted task
            class and passing success, quality, cost, latency, and rework evidence.
          </p>
        </div>
      </div>

      <label>
        Experiment stage
        <select
          aria-label="Model-routing experiment stage"
          disabled={saving}
          onChange={(event) =>
            setPolicy((current) => ({
              ...current,
              stage: event.target.value as ModelRoutingStage,
            }))
          }
          value={policy.stage}
        >
          <option value="observe">Observe only</option>
          <option value="userApproved">Ask for each route (configuration only)</option>
          <option value="automaticAllowlisted">Automatic after evidence gate (configuration only)</option>
        </select>
      </label>

      {presets.length > 0 ? (
        <fieldset>
          <legend>Observe-only policy drafts</legend>
          <p>These native templates update this form only. They do not validate routes, issue handles, save policy, or change provider traffic.</p>
          {presets.map((preset) => (
            <div key={preset.presetId}>
              <button
                className="addon-card__action"
                disabled={saving}
                onClick={() => loadPreset(preset)}
                type="button"
              >
                Load {preset.label}
              </button>
              <small>{preset.description}</small>
            </div>
          ))}
        </fieldset>
      ) : null}

      <label>
        <input
          checked={policy.globalEnabled}
          disabled={saving}
          onChange={(event) =>
            setPolicy((current) => ({ ...current, globalEnabled: event.target.checked }))
          }
          type="checkbox"
        />{" "}
        Enable model-routing experiments globally
      </label>

      <label>
        Disabled clients (comma-separated IDs)
        <input
          aria-label="Clients excluded from model routing"
          disabled={saving}
          onChange={(event) => setDisabledClients(event.target.value)}
          placeholder="codex, claude_code"
          type="text"
          value={disabledClients}
        />
      </label>

      <fieldset>
        <legend>Automatic task allowlist</legend>
        {taskClasses.map((taskClass) => (
          <label key={taskClass}>
            <input
              checked={policy.automaticTaskAllowlist.includes(taskClass)}
              disabled={saving}
              onChange={() => toggleTaskClass(taskClass)}
              type="checkbox"
            />{" "}
            {taskClass.replace(/_/g, " ")}
          </label>
        ))}
      </fieldset>

      <div aria-live="polite" aria-label="Model-routing effective stage">
        <strong>Operational routing status</strong>
        <p>
          Configured: <code>{effectiveStage.configuredStage}</code> · Effective: <code>{effectiveStage.effectiveStage}</code> · automatic routing: <code>{effectiveStage.automaticRouting}</code>
        </p>
        <p>{effectiveStage.reason}</p>
      </div>

      <p>
        Gate: at least {policy.thresholds.minimumSampleSize} samples, no more than{" "}
        {policy.thresholds.maximumSuccessRegressionBps / 100}% success regression, no more than{" "}
        {policy.thresholds.maximumQualityRegressionBps / 100}% quality regression, at least{" "}
        {policy.thresholds.minimumCostImprovementBps / 100}% successful-task cost improvement,
        no more than {policy.thresholds.maximumLatencyRegressionMs}ms p95 latency regression,
        and no more than {policy.thresholds.maximumReworkRateBps / 100}% follow-up rework.
      </p>
      <button
        className="addon-card__action addon-card__action--primary"
        disabled={saving}
        onClick={() => void save()}
        type="button"
      >
        {saving ? "Saving" : "Save routing policy"}
      </button>
      {notice ? <p role="status">{notice}</p> : null}
      <ModelRoutingEvidenceCapture observation={evidenceObservation} />
    </article>
  );
}
