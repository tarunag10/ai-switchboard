import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

import {
  describeProxySessionAuthStatus,
  type ProxySessionAuthStatus,
} from "../lib/proxySessionAuth";

export function ProxySessionAuthCard() {
  const [status, setStatus] = useState<ProxySessionAuthStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await invoke<ProxySessionAuthStatus>(
        "get_proxy_session_auth_status",
      );
      setStatus(next);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function setEnforce(enforce: boolean) {
    setBusy(true);
    setError(null);
    try {
      const next = await invoke<ProxySessionAuthStatus>(
        "set_proxy_session_auth_enforce",
        { enforce },
      );
      setStatus(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const copy = describeProxySessionAuthStatus(status);

  return (
    <article className="soft-card panel-card" id="proxy-session-auth">
      <div className="panel-card__header">
        <div>
          <h3>Proxy session auth</h3>
          <p className="panel-card__lede">
            Optional per-app-session token for loopback Headroom proxy traffic.
          </p>
        </div>
      </div>
      <p role="status" aria-live="polite">
        <strong>{copy.label}.</strong> {copy.detail}
      </p>
      {status ? (
        <p className="optimize-minimal__meta">
          Validated {status.validatedRequestCount} · Rejected{" "}
          {status.rejectedRequestCount}
        </p>
      ) : null}
      <div className="connector-item__controls">
        <button
          type="button"
          className="connector-item__action connector-item__action--primary"
          disabled={busy || !status?.available}
          onClick={() => void setEnforce(!status?.enforce)}
        >
          {busy
            ? "Saving…"
            : status?.enforce
              ? "Use advisory mode"
              : "Enforce session token"}
        </button>
        <button
          type="button"
          className="connector-item__action"
          disabled={busy}
          onClick={() => void refresh()}
        >
          Refresh
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
