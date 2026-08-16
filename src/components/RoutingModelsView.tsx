import { SettingsProviderUpstreamCard } from "./SettingsProviderUpstreamCard";
import { InferenceEndpointProfilesCard } from "./InferenceEndpointProfilesCard";
import { ModelRoutingExperimentCard } from "./ModelRoutingExperimentCard";

interface RoutingModelsViewProps {
  hidden: boolean;
}

export function RoutingModelsView({ hidden }: RoutingModelsViewProps) {
  return (
    <div className="tray-content" hidden={hidden}>
      <section className="repo-intelligence-view" aria-labelledby="routing-models-title">
        <header className="repo-intelligence-view__header">
          <div>
            <h1 id="routing-models-title">Routing &amp; Models</h1>
            <p className="repo-intelligence-view__subtitle">
              Add and verify explicit inference endpoints, then select the
              upstream used by the local Switchboard intercept.
            </p>
          </div>
          <span className="repo-intelligence-view__badge">Evidence gated</span>
        </header>

        <article className="soft-card panel-card">
          <div className="panel-card__header">
            <div>
              <h3>External runtime safety</h3>
              <p>
                AI Switchboard never scans your network or installs vLLM or SGLang. An
                endpoint is used only after you enter it, verify it, and
                explicitly enable it. Credential values stay outside
                diagnostics.
              </p>
            </div>
          </div>
        </article>

        <ModelRoutingExperimentCard />

        <InferenceEndpointProfilesCard />

        <details>
          <summary>Legacy provider URL overrides</summary>
          <SettingsProviderUpstreamCard />
        </details>
      </section>
    </div>
  );
}
