import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export interface HeadroomAdvancedSettings {
  version: number;
  ccSwitchReconcile: boolean;
}

const defaultSettings = (): HeadroomAdvancedSettings => ({
  version: 1,
  ccSwitchReconcile: false,
});

export function SettingsHeadroomAdvancedCard() {
  const [settings, setSettings] = useState<HeadroomAdvancedSettings>(defaultSettings);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      setSettings(await invoke<HeadroomAdvancedSettings>("get_headroom_advanced_settings"));
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const save = async (next: HeadroomAdvancedSettings, restartHeadroom: boolean) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const saved = await invoke<HeadroomAdvancedSettings>("set_headroom_advanced_settings", {
        settings: next,
        restartHeadroom,
      });
      setSettings(saved);
      setNotice(
        restartHeadroom
          ? "Advanced Headroom settings saved and runtime restarted."
          : "Advanced Headroom settings saved. Restart Headroom before relying on cc-switch reconciliation.",
      );
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <article className="soft-card panel-card">
      <div className="panel-card__header">
        <div>
          <h3>Advanced Headroom settings</h3>
          <p>
            Optional runtime flags injected into the Headroom spawn environment. Defaults stay off
            until you opt in.
          </p>
        </div>
      </div>
      {loading ? <p>Loading advanced settings…</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {notice ? <p role="status">{notice}</p> : null}
      <label className="optimize-project-row">
        <span className="optimize-project-row__main">
          <span className="optimize-project-row__name">cc-switch reconciler</span>
          <span className="optimize-project-row__meta">
            Sets <code>HEADROOM_CC_SWITCH_RECONCILE=1</code> when enabled. Use only if you run the
            Headroom cc-switch reconciler and understand its routing side effects.
          </span>
        </span>
        <input
          type="checkbox"
          checked={settings.ccSwitchReconcile}
          disabled={busy || loading}
          onChange={(event) => {
            const next = { ...settings, ccSwitchReconcile: event.target.checked };
            setSettings(next);
            void save(next, false);
          }}
        />
      </label>
      <div className="settings-transfer__actions">
        <button
          type="button"
          className="secondary-button secondary-button--small"
          disabled={busy || loading}
          onClick={() => void save(settings, true)}
        >
          Save and restart Headroom
        </button>
      </div>
    </article>
  );
}
