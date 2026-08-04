import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

import { deriveRepoMemoryMcpSupervisionSummary } from "../lib/repoMemoryMcpSupervision";
import type {
  RepoMemoryMcpRelaunchSurvivalStatus,
  RepoMemoryMcpSupervisionScope,
} from "../lib/repoMemoryMcpSupervision";
import type { RuntimeStatus } from "../lib/types";

export function RepoMemoryMcpSupervisionCard() {
  const [runtime, setRuntime] = useState<RuntimeStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await invoke<RuntimeStatus>("get_runtime_status");
      setRuntime(next);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const summary = deriveRepoMemoryMcpSupervisionSummary({
    supervisionStatus: runtime?.repoMemoryMcpSupervisionStatus ?? "unknown",
    relaunchSurvivalStatus:
      (runtime?.repoMemoryMcpRelaunchSurvivalStatus as
        | RepoMemoryMcpRelaunchSurvivalStatus
        | null
        | undefined) ?? "not_applicable",
    supervisionScope:
      (runtime?.repoMemoryMcpSupervisionScope as
        | RepoMemoryMcpSupervisionScope
        | null
        | undefined) ?? "app_session",
    active: runtime?.repoMemoryMcpActive === true,
  });

  return (
    <article className="soft-card panel-card repo-memory-supervision-card">
      <div className="panel-card__header">
        <div>
          <h3>Repo Memory MCP supervision</h3>
          <p>
            App-session supervision with relaunch smoke recheck. OS daemon and reboot survival are
            not claimed.
          </p>
        </div>
      </div>
      <p
        className={`repo-memory-supervision-card__status repo-memory-supervision-card__status--${summary.tone}`}
        role="status"
      >
        <strong>{summary.tone === "success" ? "Healthy" : summary.tone === "warning" ? "Attention" : "Blocked"}.</strong>{" "}
        {summary.summary}
      </p>
      {runtime ? (
        <div className="repo-memory-supervision-card__metrics" aria-label="Repo Memory MCP counters">
          <span>
            Configured <strong>{runtime.repoMemoryMcpConfigured ? "yes" : "no"}</strong>
          </span>
          <span>
            Active <strong>{runtime.repoMemoryMcpActive ? "yes" : "no"}</strong>
          </span>
          <span>
            Relaunch <strong>{runtime.repoMemoryMcpRelaunchSurvivalStatus ?? "n/a"}</strong>
          </span>
        </div>
      ) : null}
      <div className="settings-transfer__actions">
        <button
          type="button"
          className="secondary-button secondary-button--small"
          onClick={() => void refresh()}
        >
          Refresh supervision
        </button>
      </div>
      {error ? (
        <p className="install-progress__error" role="alert">
          {error}
        </p>
      ) : null}
    </article>
  );
}
