import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type EndpointKind =
  | "vllm"
  | "sglang"
  | "llama_cpp"
  | "litellm"
  | "enterprise_gateway"
  | "dynamo"
  | "tensorrt_llm"
  | "open_ai_compatible";

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
  baseUrl: string;
  host: string;
  modelId: string;
  maxContext?: number | null;
  quantization?: string | null;
  runtimeKind: string;
  externallyOwned: boolean;
  remoteConnectivityOptIn: boolean;
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
  quantization: "",
  remoteConnectivityOptIn: false,
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
            Add verified vLLM, SGLang, llama.cpp, LiteLLM, or generic
            OpenAI-compatible profiles. Selection stays manual and never
            rewrites coding-client config.
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
            <option value="sglang">SGLang verified profile</option>
            <option value="llama_cpp">llama.cpp local profile</option>
            <option value="litellm">LiteLLM externally owned profile</option>
            <option value="enterprise_gateway">Envoy AI Gateway enterprise profile</option>
            <option value="dynamo">NVIDIA Dynamo deployment profile</option>
            <option value="tensorrt_llm">TensorRT-LLM verified endpoint</option>
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
              value={String(form[field as "id" | "label" | "baseUrl" | "modelId" | "maxContext"])}
            />
          </label>
        ))}
        {form.kind === "llama_cpp" || form.kind === "tensorrt_llm" ? (
          <label>
            Quantization (optional)
            <input
              aria-label="Quantization (optional)"
              disabled={busy !== null}
              onChange={(event) =>
                setForm((current) => ({ ...current, quantization: event.target.value }))
              }
              placeholder="Q4_K_M"
              value={form.quantization}
            />
          </label>
        ) : null}
        {["litellm", "enterprise_gateway", "dynamo"].includes(form.kind) ? (
          <label className="optimize-project-row">
            <span className="optimize-project-row__main">
              <span className="optimize-project-row__name">External connectivity opt-in</span>
              <span className="optimize-project-row__meta">
                Required outside loopback. The service remains externally owned; token values are never stored.
              </span>
            </span>
            <input
              aria-label="External endpoint connectivity opt-in"
              checked={form.remoteConnectivityOptIn}
              disabled={busy !== null}
              onChange={(event) =>
                setForm((current) => ({ ...current, remoteConnectivityOptIn: event.target.checked }))
              }
              type="checkbox"
            />
          </label>
        ) : null}
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
            <p>
              <strong>Runtime:</strong> {endpoint.runtimeKind.replace(/_/g, " ")} ·{" "}
              <strong>Host:</strong> <code>{endpoint.host}</code> ·{" "}
              <strong>Model:</strong> <code>{endpoint.modelId}</code>
            </p>
            <p>
              <strong>Context:</strong> {endpoint.maxContext?.toLocaleString() ?? "unknown"} ·{" "}
              <strong>Quantization:</strong> {endpoint.quantization ?? "unknown"} ·{" "}
              <strong>Health:</strong> {endpoint.verification.status}
            </p>
            {endpoint.externallyOwned ? (
              <p>
                Externally owned profile · remote connectivity{" "}
                {endpoint.remoteConnectivityOptIn ? "explicitly allowed" : "loopback-only"} · secrets redacted.
              </p>
            ) : null}
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
