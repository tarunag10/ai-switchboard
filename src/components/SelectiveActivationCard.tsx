import { invoke } from "@tauri-apps/api/core";
import { CheckCircle, Circle, Lightning, WarningCircle } from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import {
  SELECTIVE_ACTIVATION_LIMIT,
  SELECTIVE_ACTIVATION_TOOLS,
  normalizeActivationSelection,
  validateActivationSelection,
  type ActivationToolId,
} from "../lib/activationTools";
import { saveRepoPackCompressionPreference } from "../lib/repoIntelligence";

const STORAGE_KEY = "ai-switchboard.selective-activation.v1";
type ToolResult = { state: "success" | "failed"; detail: string };

function readSelection(): ActivationToolId[] {
  try {
    const raw = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "null");
    return normalizeActivationSelection(raw?.selectedToolIds);
  } catch {
    return [];
  }
}

function writeSelection(selectedToolIds: ActivationToolId[]) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, selectedToolIds }));
  } catch {
    // Storage is a convenience; activation remains usable when it is unavailable.
  }
}

type NativeActivationResult = {
  receipt: {
    overallStatus: "succeeded" | "partial" | "failed";
    results: Array<{ toolId: ActivationToolId; state: string; detail: string }>;
  };
};

export function SelectiveActivationCard({ onComplete }: { onComplete?: () => Promise<void> }) {
  const [selected, setSelected] = useState<ActivationToolId[]>(() => readSelection());
  const [results, setResults] = useState<Partial<Record<ActivationToolId, ToolResult>>>({});
  const [busy, setBusy] = useState(false);
  const [runSummary, setRunSummary] = useState<string | null>(null);
  const validationError = useMemo(() => validateActivationSelection(selected), [selected]);

  useEffect(() => writeSelection(selected), [selected]);

  useEffect(() => {
    if (validationError) return;
    void Promise.resolve(invoke("save_selective_activation_selection", { selectedToolIds: selected })).catch(() => undefined);
  }, [selected, validationError]);

  const toggle = (id: ActivationToolId) => {
    setRunSummary(null);
    setResults({});
    setSelected((current) => current.includes(id)
      ? current.filter((item) => item !== id)
      : current.length >= SELECTIVE_ACTIVATION_LIMIT ? current : [...current, id]);
  };

  const activateSelected = async () => {
    const error = validateActivationSelection(selected);
    if (error || busy) {
      setRunSummary(error);
      return;
    }
    setBusy(true);
    setResults({});
    try {
      await invoke("validate_selective_activation_selection", { selectedToolIds: selected });
    } catch (reason) {
      setBusy(false);
      setRunSummary(reason instanceof Error ? reason.message : String(reason));
      return;
    }
    let response: NativeActivationResult;
    try {
      response = await invoke<NativeActivationResult>("activate_selected_tools", { selectedToolIds: selected });
    } catch (reason) {
      setBusy(false);
      setRunSummary(reason instanceof Error ? reason.message : String(reason));
      return;
    }
    const nextResults: Partial<Record<ActivationToolId, ToolResult>> = {};
    for (const item of response.receipt.results) {
      nextResults[item.toolId] = {
        state: item.state === "failed" ? "failed" : "success",
        detail: item.detail,
      };
    }
    setResults(nextResults);
    if (selected.includes("chonkify") && response.receipt.overallStatus === "succeeded") {
      saveRepoPackCompressionPreference("chonkify");
    }
    if (response.receipt.overallStatus === "succeeded" && onComplete) {
      await onComplete();
    }
    setBusy(false);
    setRunSummary(response.receipt.overallStatus === "succeeded"
      ? `Activated all ${selected.length} selected tools.`
      : `Selective activation finished with status ${response.receipt.overallStatus}. Failed tools are shown below; retry after correcting the reported prerequisite.`);
  };

  return (
    <article className="soft-card panel-card selective-activation-card" aria-labelledby="selective-activation-title">
      <div className="panel-card__header">
        <div>
          <h2 id="selective-activation-title"><Lightning weight="duotone" /> Activate 5 tools</h2>
          <p>Choose exactly five of ten available local tools. Existing workspace activation remains separate.</p>
        </div>
        <strong aria-live="polite">{selected.length}/{SELECTIVE_ACTIVATION_LIMIT}</strong>
      </div>
      <div className="selective-activation-card__grid">
        {SELECTIVE_ACTIVATION_TOOLS.map((tool) => {
          const checked = selected.includes(tool.id);
          const result = results[tool.id];
          return (
            <button
              className={`selective-activation-card__tool${checked ? " is-selected" : ""}`}
              key={tool.id}
              type="button"
              aria-pressed={checked}
              onClick={() => toggle(tool.id)}
              disabled={busy || (!checked && selected.length >= SELECTIVE_ACTIVATION_LIMIT)}
            >
              <span className="selective-activation-card__tool-icon" aria-hidden="true">
                {result?.state === "success" ? <CheckCircle weight="fill" /> : result?.state === "failed" ? <WarningCircle weight="fill" /> : checked ? <CheckCircle /> : <Circle />}
              </span>
              <span><strong>{tool.label}</strong><small>{tool.description}</small><em>{tool.scope}</em>{result ? <small role="status">{result.detail}</small> : null}</span>
            </button>
          );
        })}
      </div>
      {validationError ? <p className="addon-card__hint" aria-live="polite">{validationError}</p> : null}
      {runSummary ? <p className="addon-card__hint" role="status">{runSummary}</p> : null}
      <button className="primary-button" type="button" onClick={() => void activateSelected()} disabled={busy || Boolean(validationError)}>
        {busy ? "Activating selected tools…" : "Activate selected 5"}
      </button>
      <p className="addon-card__hint">Each action reports its own result. Provider routing, experimental engines, and unsupported automatic model selection remain fail-closed.</p>
    </article>
  );
}
