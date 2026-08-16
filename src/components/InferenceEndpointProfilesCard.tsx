import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type EndpointKind = "vllm" | "open_ai_compatible";

interface EndpointVerification {
  status: "unverified" | "verified" | "failed";
  runtimeId?: string | null;
  runtimeVersion?: string | null;
  reason?: string;
}

interface EndpointDiagnostic {
  id: string;
  label: string;
  locationClass:
    | "local_loopback"
    | "local_network"
    | "user_configured_remote"
    | "remote_provider";
  enabled: boolean;
  selected: boolean;
  verification: EndpointVerification;
}

interface EndpointMutationResult {
  diagnostics: EndpointDiagnostic[];
  selectedEndpointId: string | null;
}

const emptyForm = {
  id: "",
  label: "",
  baseUrl: "",
  modelId: "",
  maxContext: "",
  kind: "vllm" as EndpointKind,
};

export function InferenceEndpointProfilesCard() {
  const [snapshot, setSnapshot] = useState<EndpointMutationResult>({
    diagnostics: [],
    selectedEndpointId: null,
  });
  const [form, setForm] = useState(emptyForm);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const refresh = async () => {
    setSnapshot(await invoke<EndpointMutationResult>("list_inference_endpoints"));
  };

  useEffect(() => {
    void refresh().catch((reason: unknown) =>
      setError(reason instanceof Error ? reason.message : String(reason)),
    );
  }, []);

  const perform = async (id: string, action: () => Promise<void>) => {
    setBusy(id);
    setError(null);
    setNotice(null);
    try {
      await action();
      await refresh();
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(null);
    }
  };

  const addEndpoint = async () => {
    const id = form.id.trim();
    const confirmation = window.prompt(
      `Adding this endpoint allowlists only the URL you entered. Type ADD ENDPOINT ${id} to continue.`,
    );
    if (confirmation === null) return;
    await perform("add", async () => {
      const result = await invoke<EndpointMutationResult>(
        "add_inference_endpoint",
        {
          input: {
            ...form,
            maxContext: form.maxContext ? Number(form.maxContext) : null,
          },
          confirmation,
        },
      );
      setSnapshot(result);
      setForm(emptyForm);
      setNotice("Endpoint added to the local allowlist. Verify it before selection.");
    });
  };

  return (
    <article className="soft-card panel-card settings-provider-upstream-card">
      <div className="panel-card__header">
        <div>
          <h3>Inference endpoint profiles</h3>
          <p>
            Add a generic OpenAI-compatible server or a verified vLLM profile.
            Selection stays manual and never rewrites coding-client config.
          </p>
        </div>
      </div>

      {error ? <p role="alert"><strong>Endpoint action failed:</strong> {error}</p> : null}
      {notice ? <p role="status">{notice}</p> : null}

      <fieldset className="gateway-profile">
        <legend><strong>Add endpoint</strong></legend>
        <label>
          Runtime type
          <select
            aria-label="Endpoint runtime type"
            disabled={busy !== null}
            onChange={(event) =>
              setForm((current) => ({
                ...current,
                kind: event.target.value as EndpointKind,
              }))
            }
            value={form.kind}
          >
            <option value="vllm">vLLM verified profile</option>
            <option value="open_ai_compatible">Generic OpenAI-compatible</option>
          </select>
        </label>
        {[
          ["id", "Endpoint ID", "local-gpu"],
          ["label", "Display label", "Studio vLLM"],
          ["baseUrl", "Base URL", "http://192.168.1.50:8000/v1"],
          ["modelId", "Model ID", "Qwen/Qwen3-Coder"],
          ["maxContext", "Max context (optional)", "32768"],
        ].map(([field, label, placeholder]) => (
          <label key={field}>
            {label}
            <input
              aria-label={label}
              disabled={busy !== null}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  [field]: event.target.value,
                }))
              }
              placeholder={placeholder}
              type={field === "maxContext" ? "number" : "text"}
              value={form[field as keyof typeof form]}
            />
          </label>
        ))}
        <button
          className="addon-card__action addon-card__action--primary"
          disabled={
            busy !== null ||
            !form.id.trim() ||
            !form.label.trim() ||
            !form.baseUrl.trim() ||
            !form.modelId.trim()
          }
          onClick={() => void addEndpoint()}
          type="button"
        >
          Add to allowlist
        </button>
      </fieldset>

      <div className="settings-transfer__list" aria-label="Configured inference endpoints">
        {snapshot.diagnostics.length === 0 ? (
          <p>No user-managed endpoints configured.</p>
        ) : snapshot.diagnostics.map((endpoint) => (
          <article className="gateway-profile" key={endpoint.id}>
            <h4>{endpoint.label}</h4>
            <p>
              <code>{endpoint.id}</code> · {endpoint.locationClass.replace(/_/g, " ")} ·{" "}
              {endpoint.verification.status}
              {endpoint.selected ? " · selected" : ""}
            </p>
            {endpoint.verification.reason ? <p>{endpoint.verification.reason}</p> : null}
            <div className="gateway-profile__actions">
              <button
                className="addon-card__action"
                disabled={busy !== null}
                onClick={() => void perform(endpoint.id, async () => {
                  await invoke("verify_inference_endpoint", { endpointId: endpoint.id });
                  setNotice("Endpoint verification evidence refreshed.");
                })}
                type="button"
              >
                Verify
              </button>
              <button
                className="addon-card__action addon-card__action--primary"
                disabled={busy !== null || endpoint.verification.status !== "verified" || endpoint.selected}
                onClick={() => void perform(endpoint.id, async () => {
                  await invoke("select_inference_endpoint", {
                    endpointId: endpoint.id,
                    restartOptimizer: true,
                  });
                  setNotice("Endpoint selected and optimizer restarted.");
                })}
                type="button"
              >
                Select &amp; restart
              </button>
              <button
                className="addon-card__action"
                disabled={busy !== null || !endpoint.enabled}
                onClick={() => void perform(endpoint.id, async () => {
                  await invoke("disable_inference_endpoint", {
                    endpointId: endpoint.id,
                    restartOptimizer: true,
                  });
                  setNotice("Endpoint disabled; coding-client config was unchanged.");
                })}
                type="button"
              >
                Disable
              </button>
            </div>
          </article>
        ))}
      </div>
    </article>
  );
}
