import { ArrowClockwise } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import {
  type OptimizationActionPolicy,
  type PreemptiveCompactionReceipt,
  loadOptimizationActionPolicy,
  saveOptimizationActionPolicy,
  runPreemptiveCompaction,
} from "../lib/optimization";

export function OptimizationActionPanel() {
  const [policy, setPolicy] = useState<OptimizationActionPolicy | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void loadOptimizationActionPolicy().then((nextPolicy) => {
      if (!cancelled) setPolicy(nextPolicy);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  async function toggle(key: keyof Pick<
    OptimizationActionPolicy,
    | "promptCacheReorderEnabled"
    | "preemptiveCompactionEnabled"
    | "modelRoutingEnabled"
  >) {
    if (!policy) return;
    const nextPolicy = { ...policy, [key]: !policy[key] };
    setPolicy(nextPolicy);
    setSaving(true);
    try {
      setPolicy(await saveOptimizationActionPolicy(nextPolicy));
    } finally {
      setSaving(false);
    }
  }

  async function enableAll() {
    if (!policy) return;
    const nextPolicy = {
      ...policy,
      promptCacheReorderEnabled: true,
      preemptiveCompactionEnabled: true,
      modelRoutingEnabled: true,
    };
    setPolicy(nextPolicy);
    setSaving(true);
    try {
      setPolicy(await saveOptimizationActionPolicy(nextPolicy));
    } finally {
      setSaving(false);
    }
  }

  if (!policy) return null;

  return (
    <section className="optimize-minimal" aria-labelledby="optimization-action-title">
      <div className="optimize-card__head">
        <div>
          <h2 id="optimization-action-title">Action Policy</h2>
          <p className="optimize-minimal__meta">
            Controls that allow Switchboard to move from observe-only to guarded actions.
          </p>
        </div>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void enableAll()}
          disabled={saving}
        >
          Enable all
        </button>
      </div>
      <div className="optimize-projects">
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void toggle("promptCacheReorderEnabled")}
        >
          Prompt cache reorder: {policy.promptCacheReorderEnabled ? "on" : "off"}
        </button>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void toggle("preemptiveCompactionEnabled")}
        >
          Preemptive compaction: {policy.preemptiveCompactionEnabled ? "on" : "off"}
        </button>
        <button
          className="secondary-button secondary-button--small"
          type="button"
          onClick={() => void toggle("modelRoutingEnabled")}
        >
          Model routing: {policy.modelRoutingEnabled ? "on" : "off"}
        </button>
      </div>
    </section>
  );
}

export function PreemptiveCompactionButton() {
  const [receipt, setReceipt] = useState<PreemptiveCompactionReceipt | null>(null);
  const [busy, setBusy] = useState(false);

  async function run() {
    setBusy(true);
    try {
      setReceipt(await runPreemptiveCompaction());
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="optimize-minimal" aria-labelledby="preemptive-compaction-title">
      <div className="optimize-card__title-row">
        <span className="optimize-card__title-icon" aria-hidden="true">
          <ArrowClockwise weight="duotone" />
        </span>
        <h2 id="preemptive-compaction-title">Preemptive Compaction</h2>
      </div>
      <p className="optimize-minimal__meta">
        One click records the current threshold check and queues Switchboard's prevention path before
        clients hit an oversized-context failure.
      </p>
      <button
        className="secondary-button secondary-button--small"
        type="button"
        onClick={() => run()}
        disabled={busy}
      >
        <ArrowClockwise weight="bold" size={12} aria-hidden="true" />
        {busy ? "Running" : "Run compaction"}
      </button>
      {receipt ? (
        <p className="optimize-minimal__meta" role="status">
          {receipt.action} {receipt.contextUsedPercent}% used; trigger at {receipt.thresholdPercent}%.
        </p>
      ) : null}
    </section>
  );
}
