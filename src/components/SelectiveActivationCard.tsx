import { invoke } from "@tauri-apps/api/core";
import { CheckCircle, Circle, Lightning, WarningCircle } from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import {
  SELECTIVE_ACTIVATION_LIMIT,
  SELECTIVE_ACTIVATION_TOOLS,
  normalizeActivationRecovery,
  normalizeActivationSelection,
  validateActivationSelection,
  type ActivationToolId,
} from "../lib/activationTools";
import type { DashboardState } from "../lib/types";

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
  dashboard: DashboardState;
  receipt: {
    runId: string;
    overallStatus: "succeeded" | "partial" | "failed";
    results: Array<{ toolId: ActivationToolId; state: string; detail: string }>;
  };
};
type NativeActivationSelection = { selectedToolIds?: unknown };

export function SelectiveActivationCard({ onComplete }: { onComplete?: (dashboard: DashboardState) => Promise<void> }) {
  const [selected, setSelected] = useState<ActivationToolId[]>(() => readSelection());
  const [results, setResults] = useState<Partial<Record<ActivationToolId, ToolResult>>>({});
  const [busy, setBusy] = useState(false);
  const [restoring, setRestoring] = useState(true);
  const [nativePersistenceEnabled, setNativePersistenceEnabled] = useState(false);
  const [runSummary, setRunSummary] = useState<string | null>(null);
  const [lastRunId, setLastRunId] = useState<string | null>(null);
  const validationError = useMemo(() => validateActivationSelection(selected), [selected]);

  useEffect(() => {
    let active = true;
    void (async () => {
      const [selectionResult, recoveryResult] = await Promise.allSettled([
        invoke<NativeActivationSelection | null>("get_selective_activation_selection"),
        invoke<unknown>("get_selective_activation_recovery"),
      ]);
      if (!active) return;

      const notices: string[] = [];
      let restoredSelection: ActivationToolId[] | null = null;
      if (selectionResult.status === "fulfilled") {
        if (selectionResult.value == null) {
          setNativePersistenceEnabled(true);
        } else {
          const normalized = normalizeActivationSelection(selectionResult.value.selectedToolIds);
          if (!validateActivationSelection(normalized)) {
            restoredSelection = normalized;
            setSelected(normalized);
            setNativePersistenceEnabled(true);
          } else {
            notices.push("The native tool selection could not be restored; the local selection was preserved and will not overwrite native state until you change it.");
          }
        }
      } else if (selectionResult.status === "rejected") {
        notices.push(`The native tool selection could not be restored and will not be overwritten until you change it: ${String(selectionResult.reason)}`);
      }

      if (recoveryResult.status === "fulfilled" && recoveryResult.value != null) {
        const recovery = normalizeActivationRecovery(recoveryResult.value);
        if (!recovery) {
          notices.push("The saved activation receipt failed recovery validation; no rollback was enabled.");
        } else {
          const visibleSelection = restoredSelection ?? selected;
          if (visibleSelection.length === SELECTIVE_ACTIVATION_LIMIT
            && (recovery.selectedToolIds.some((id) => !visibleSelection.includes(id))
              || visibleSelection.some((id) => !recovery.selectedToolIds.includes(id)))) {
            notices.push("The saved rollback belongs to a different five-tool selection; the visible selection was preserved and undo remains limited to that receipt's run-owned changes.");
          }
          if (recovery.rollbackAvailable) {
            setLastRunId(recovery.runId);
            notices.push(`A previous ${recovery.overallStatus} native tool activation can be undone. Automatic retry is disabled.`);
          } else if (recovery.rollbackStatus === "succeeded") {
            notices.push("The previous native tool activation has already been rolled back.");
          } else if (recovery.rollbackStatus === "partial" || recovery.rollbackStatus === "in_progress") {
            notices.push("The previous rollback was interrupted or partial and requires repair; automatic resume and retry are disabled.");
          } else {
            notices.push(`The previous ${recovery.overallStatus} native tool activation has no run-owned changes to undo. Automatic retry is disabled.`);
          }
        }
      } else if (recoveryResult.status === "rejected") {
        notices.push(`The saved activation receipt could not be restored: ${String(recoveryResult.reason)}`);
      }
      if (notices.length > 0) setRunSummary(notices.join(" "));
      setRestoring(false);
    })();
    return () => { active = false; };
  }, []);

  useEffect(() => writeSelection(selected), [selected]);

  useEffect(() => {
    if (restoring || !nativePersistenceEnabled || validationError) return;
    void Promise.resolve(invoke("save_selective_activation_selection", { selectedToolIds: selected })).catch(() => undefined);
  }, [restoring, nativePersistenceEnabled, selected, validationError]);

  const toggle = (id: ActivationToolId) => {
    setNativePersistenceEnabled(true);
    setRunSummary(null);
    setResults({});
    setSelected((current) => current.includes(id)
      ? current.filter((item) => item !== id)
      : current.length >= SELECTIVE_ACTIVATION_LIMIT ? current : [...current, id]);
  };

  const activateSelected = async () => {
    const error = validateActivationSelection(selected);
    if (restoring || error || busy) {
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
    setLastRunId(response.receipt.runId);
    try {
      if (onComplete) await onComplete(response.dashboard);
      setRunSummary(response.receipt.overallStatus === "succeeded"
        ? `Activated all ${selected.length} selected tools.`
        : `Selective activation finished with status ${response.receipt.overallStatus}. Failed tools are shown below; retry after correcting the reported prerequisite.`);
    } catch (reason) {
      setRunSummary(`Activation completed and remains undoable, but refreshing the dashboard failed: ${reason instanceof Error ? reason.message : String(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const rollbackLastActivation = async () => {
    if (!lastRunId || busy) return;
    setBusy(true);
    try {
      const response = await invoke<NativeActivationResult>("rollback_selective_activation", { runId: lastRunId });
      const nextResults: Partial<Record<ActivationToolId, ToolResult>> = {};
      for (const item of response.receipt.results) {
        nextResults[item.toolId] = {
          state: item.state === "failed" ? "failed" : "success",
          detail: item.detail,
        };
      }
      setResults(nextResults);
      setLastRunId(null);
      if (onComplete) await onComplete(response.dashboard);
      setRunSummary("Last native tool activation was rolled back. Pre-existing tools and refresh-only evidence were preserved.");
    } catch (reason) {
      setRunSummary(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
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
              disabled={restoring || busy || (!checked && selected.length >= SELECTIVE_ACTIVATION_LIMIT)}
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
      <button className="primary-button" type="button" onClick={() => void activateSelected()} disabled={restoring || busy || Boolean(validationError)}>
        {restoring ? "Restoring saved state…" : busy ? "Activating selected tools…" : "Activate selected 5"}
      </button>
      {lastRunId ? (
        <button className="addon-card__action" type="button" onClick={() => void rollbackLastActivation()} disabled={busy}>
          {busy ? "Rolling back…" : "Undo last native tool activation"}
        </button>
      ) : null}
      <p className="addon-card__hint">Each action reports its own result. Provider routing, experimental engines, and unsupported automatic model selection remain fail-closed.</p>
    </article>
  );
}
