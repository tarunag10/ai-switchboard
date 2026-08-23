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
import { loadTokenXraySnapshot } from "../lib/usageAnalytics";

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

async function activateTool(id: ActivationToolId): Promise<string> {
  switch (id) {
    case "headroom":
      await invoke("set_switchboard_mode", { mode: "full" });
      return "Full local optimization mode enabled.";
    case "rtk":
      await invoke("install_addon", { id: "rtk" });
      await invoke("set_rtk_enabled", { enabled: true });
      return "RTK installed and enabled.";
    case "repo-intelligence":
      await invoke("get_latest_repo_intelligence_summary");
      return "Local repository intelligence summary refreshed.";
    case "token-xray":
      await loadTokenXraySnapshot();
      return "Token X-Ray evidence refreshed.";
    case "ponytail":
    case "caveman":
    case "markitdown":
      await invoke("install_addon", { id });
      await invoke("set_addon_enabled", { id, enabled: true });
      return `${id === "markitdown" ? "MarkItDown" : id[0].toUpperCase() + id.slice(1)} installed and enabled.`;
    case "response-cache":
      await invoke("set_addon_enabled", { id: "response-cache", enabled: true });
      return "Exact Response Cache enabled.";
    case "chonkify":
      saveRepoPackCompressionPreference("chonkify");
      return "Repo-pack compression enabled for local packs.";
    case "leanctx": {
      const status = await invoke<{ configured?: boolean }>("get_leanctx_sidecar_status");
      if (!status?.configured) await invoke("install_addon", { id: "leanctx" });
      await invoke("set_addon_enabled", { id: "leanctx", enabled: true });
      return status?.configured ? "Leanctx shadow enabled." : "Leanctx installed and shadow enabled.";
    }
  }
}

export function SelectiveActivationCard() {
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
    let succeeded = 0;
    const nextResults: Partial<Record<ActivationToolId, ToolResult>> = {};
    for (const id of selected) {
      try {
        nextResults[id] = { state: "success", detail: await activateTool(id) };
        succeeded += 1;
      } catch (reason) {
        nextResults[id] = { state: "failed", detail: reason instanceof Error ? reason.message : String(reason) };
      }
      setResults({ ...nextResults });
    }
    setBusy(false);
    setRunSummary(succeeded === selected.length
      ? `Activated all ${succeeded} selected tools.`
      : `Activated ${succeeded} of ${selected.length}. Failed tools are shown below; no destructive automatic rollback was attempted.`);
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
