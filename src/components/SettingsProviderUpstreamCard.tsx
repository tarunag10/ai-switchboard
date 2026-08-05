import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type {
  ProviderUpstreamProfilesState,
  ProviderUpstreamTestResult,
} from "../lib/types";

const emptyState = (): ProviderUpstreamProfilesState => ({
  version: 1,
  openai: { enabled: false, url: "" },
  anthropic: { enabled: false, url: "" },
});

export function SettingsProviderUpstreamCard() {
  const [state, setState] = useState<ProviderUpstreamProfilesState>(emptyState);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<
    Record<string, ProviderUpstreamTestResult>
  >({});

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      setState(await invoke<ProviderUpstreamProfilesState>("get_provider_upstream_profiles"));
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const save = async (next: ProviderUpstreamProfilesState, restartHeadroom: boolean) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const saved = await invoke<ProviderUpstreamProfilesState>(
        "set_provider_upstream_profiles",
        { state: next, restartHeadroom },
      );
      setState(saved);
      setNotice(
        restartHeadroom
          ? "Upstream overrides saved. Headroom restarted with the new target URLs."
          : "Upstream overrides saved. Restart Headroom before routing production traffic.",
      );
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const clearOverrides = async () => {
    if (
      !window.confirm(
        "Clear provider upstream overrides? Headroom will restart and route to default OpenAI and Anthropic endpoints.",
      )
    ) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      setState(
        await invoke<ProviderUpstreamProfilesState>(
          "clear_provider_upstream_profiles_command",
          { restartHeadroom: true },
        ),
      );
      setTestResults({});
      setNotice("Upstream overrides cleared.");
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const testProvider = async (provider: "openai" | "anthropic", url: string) => {
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<ProviderUpstreamTestResult>(
        "test_provider_upstream_profile",
        { provider, url },
      );
      setTestResults((current) => ({ ...current, [provider]: result }));
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const updateProvider = (
    provider: "openai" | "anthropic",
    patch: Partial<ProviderUpstreamProfilesState["openai"]>,
  ) => {
    setState((current) => ({
      ...current,
      [provider]: { ...current[provider], ...patch },
    }));
  };

  const renderProvider = (provider: "openai" | "anthropic", label: string) => {
    const override = state[provider];
    const test = testResults[provider];
    return (
      <fieldset className="gateway-profile" key={provider}>
        <legend>
          <strong>{label}</strong>
        </legend>
        <label className="settings-transfer__note">
          <input
            checked={override.enabled}
            disabled={busy || loading}
            onChange={(event) =>
              updateProvider(provider, { enabled: event.target.checked })
            }
            type="checkbox"
          />{" "}
          Route {label} traffic through a custom upstream URL
        </label>
        <input
          aria-label={`${label} upstream URL`}
          className="settings-transfer__textarea"
          disabled={busy || loading || !override.enabled}
          onChange={(event) => updateProvider(provider, { url: event.target.value })}
          placeholder={
            provider === "openai"
              ? "https://api.deepseek.com/v1"
              : "https://api.anthropic.com"
          }
          type="url"
          value={override.url}
        />
        <div className="gateway-profile__actions">
          <button
            className="addon-card__action"
            disabled={busy || loading || !override.enabled || !override.url.trim()}
            onClick={() => void testProvider(provider, override.url)}
            type="button"
          >
            Test connection
          </button>
        </div>
        {test ? (
          <p className="optimize-minimal__meta" role="status">
            <strong>{test.ok ? "Reachable" : "Failed"}:</strong> {test.detail}
            {test.statusCode ? ` (HTTP ${test.statusCode})` : ""}
          </p>
        ) : null}
      </fieldset>
    );
  };

  return (
    <article className="soft-card panel-card settings-provider-upstream-card">
      <div className="panel-card__header">
        <div>
          <h3>Provider upstream</h3>
          <p>
            Optional BYOK-compatible upstream URLs for Headroom spawn env. Switchboard stores
            only the URL override in local app storage — never API keys.
          </p>
        </div>
      </div>
      <p className="settings-transfer__note">
        Supported pattern: OpenAI-compatible endpoints (DeepSeek, Azure OpenAI, Together) via{" "}
        <code>OPENAI_TARGET_API_URL</code>, and Anthropic-compatible endpoints via{" "}
        <code>ANTHROPIC_TARGET_API_URL</code>. Only HTTPS URLs are accepted, except loopback HTTP
        for local testing. Doctor surfaces invalid URLs before production routing.
      </p>
      {loading ? <p role="status">Loading upstream profile…</p> : null}
      {error ? (
        <p className="gateway-profile__inline-feedback" role="alert">
          <strong>Upstream settings failed:</strong> {error}
        </p>
      ) : null}
      {notice ? (
        <p className="optimize-minimal__meta" role="status">
          {notice}
        </p>
      ) : null}
      {!loading ? (
        <>
          {renderProvider("openai", "OpenAI-compatible")}
          {renderProvider("anthropic", "Anthropic-compatible")}
          <div className="gateway-profile__actions">
            <button
              className="addon-card__action addon-card__action--primary"
              disabled={busy}
              onClick={() => {
                const restart = window.confirm(
                  "Save upstream overrides and restart Headroom now?",
                );
                void save(state, restart);
              }}
              type="button"
            >
              Save overrides
            </button>
            <button
              className="addon-card__action"
              disabled={busy}
              onClick={() => void clearOverrides()}
              type="button"
            >
              Clear overrides
            </button>
          </div>
        </>
      ) : null}
    </article>
  );
}
