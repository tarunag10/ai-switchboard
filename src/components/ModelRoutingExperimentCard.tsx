import { useEffect, useState } from "react";

import {
  defaultModelRoutingExperimentPolicy,
  loadModelRoutingExperimentPolicy,
  saveModelRoutingExperimentPolicy,
  type ModelRoutingExperimentPolicy,
  type ModelRoutingStage,
} from "../lib/optimization";

const taskClasses = ["formatting", "commit_message", "rename", "diff_summary"] as const;

export function ModelRoutingExperimentCard() {
  const [policy, setPolicy] = useState<ModelRoutingExperimentPolicy>(
    defaultModelRoutingExperimentPolicy,
  );
  const [disabledClients, setDisabledClients] = useState("");
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    void loadModelRoutingExperimentPolicy().then((loaded) => {
      setPolicy(loaded);
      setDisabledClients(loaded.disabledClients.join(", "));
    });
  }, []);

  const toggleTaskClass = (taskClass: string) => {
    setPolicy((current) => ({
      ...current,
      automaticTaskAllowlist: current.automaticTaskAllowlist.includes(taskClass)
        ? current.automaticTaskAllowlist.filter((value) => value !== taskClass)
        : [...current.automaticTaskAllowlist, taskClass],
    }));
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
            class and passing success, cost, and rework evidence.
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
          <option value="userApproved">Ask for each route</option>
          <option value="automaticAllowlisted">Automatic after evidence gate</option>
        </select>
      </label>

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

      <p>
        Gate: at least {policy.thresholds.minimumSampleSize} samples, no more than{" "}
        {policy.thresholds.maximumSuccessRegressionBps / 100}% success regression, at least{" "}
        {policy.thresholds.minimumCostImprovementBps / 100}% successful-task cost improvement,
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
    </article>
  );
}
