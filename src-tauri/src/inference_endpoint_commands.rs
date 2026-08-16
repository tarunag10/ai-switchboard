use std::io::Read;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, State};
use url::Url;

use crate::inference_endpoint::{
    CredentialStrategy, EndpointCapabilities, EndpointDiagnostic, EndpointProbe,
    EndpointRegistryService, EndpointVerification, HealthPolicy, InferenceProtocol, ModelDiscovery,
    OpenAiCompatibleEndpoint, PrefixCacheEvidence, ProbeObservation, ProbePurpose, ProbeRequest,
    UserEndpointApproval, VllmEndpoint,
};
use crate::state::AppState;

const MAX_PROBE_BODY_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddEndpointInput {
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub model_id: String,
    pub max_context: Option<u64>,
    pub kind: EndpointKind,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EndpointKind {
    OpenAiCompatible,
    Vllm,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointMutationResult {
    diagnostics: Vec<EndpointDiagnostic>,
    selected_endpoint_id: Option<String>,
}

fn registry_dir() -> std::path::PathBuf {
    crate::storage::app_data_dir().join("config")
}

fn load_registry() -> Result<EndpointRegistryService, String> {
    EndpointRegistryService::load(&registry_dir())
}

fn mutation_result(service: &EndpointRegistryService) -> EndpointMutationResult {
    EndpointMutationResult {
        diagnostics: service.list_diagnostics(),
        selected_endpoint_id: service
            .selected_route_target()
            .map(|target| target.endpoint_id),
    }
}

#[tauri::command]
pub(crate) fn list_inference_endpoints() -> Result<EndpointMutationResult, String> {
    let service = load_registry()?;
    Ok(mutation_result(&service))
}

#[tauri::command]
pub(crate) fn add_inference_endpoint(
    input: AddEndpointInput,
    confirmation: String,
) -> Result<EndpointMutationResult, String> {
    let expected = format!("ADD ENDPOINT {}", input.id.trim());
    if confirmation != expected {
        return Err(format!("Confirmation must exactly match: {expected}"));
    }
    let approval = UserEndpointApproval::explicit(&input.base_url)?;
    let mut service = load_registry()?;
    match input.kind {
        EndpointKind::Vllm => service.add_vllm(
            VllmEndpoint::new(
                input.id,
                input.label,
                input.base_url,
                input.model_id,
                input.max_context,
                "vllm-aiperf-v1",
            )?,
            approval,
        )?,
        EndpointKind::OpenAiCompatible => service.add_openai(
            OpenAiCompatibleEndpoint::new(
                input.id,
                input.label,
                input.base_url,
                HealthPolicy::Passive,
                input.model_id,
                EndpointCapabilities {
                    protocol: InferenceProtocol::OpenAiCompatible,
                    streaming: true,
                    tools: true,
                    structured_output: false,
                    max_context: input.max_context,
                    prefix_cache_evidence: PrefixCacheEvidence::Unknown,
                    health_endpoint: None,
                    model_discovery: ModelDiscovery::Endpoint {
                        path: "/v1/models".to_string(),
                    },
                },
                CredentialStrategy::None,
                false,
            )?,
            approval,
        )?,
    }
    Ok(mutation_result(&service))
}

#[tauri::command]
pub(crate) async fn verify_inference_endpoint(
    endpoint_id: String,
) -> Result<EndpointVerification, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut service = load_registry()?;
        service.verify(&endpoint_id, &HttpEndpointProbe::new()?)
    })
    .await
    .map_err(|err| format!("Endpoint verification worker failed: {err}"))?
}

#[tauri::command]
pub(crate) fn select_inference_endpoint(
    app: AppHandle,
    endpoint_id: String,
    restart_optimizer: bool,
) -> Result<EndpointMutationResult, String> {
    let mut service = load_registry()?;
    service.select(&endpoint_id)?;
    if restart_optimizer {
        let state: State<'_, AppState> = app.state();
        crate::switchboard_commands::repair_runtime(&state)?;
    }
    Ok(mutation_result(&service))
}

#[tauri::command]
pub(crate) fn disable_inference_endpoint(
    app: AppHandle,
    endpoint_id: String,
    restart_optimizer: bool,
) -> Result<EndpointMutationResult, String> {
    let mut service = load_registry()?;
    service.disable(&endpoint_id)?;
    if restart_optimizer {
        let state: State<'_, AppState> = app.state();
        crate::switchboard_commands::repair_runtime(&state)?;
    }
    Ok(mutation_result(&service))
}

struct HttpEndpointProbe {
    client: reqwest::blocking::Client,
}

impl HttpEndpointProbe {
    fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Could not initialize the endpoint verifier.".to_string())?;
        Ok(Self { client })
    }

    fn probe_url(request: &ProbeRequest) -> Result<Url, String> {
        let mut url = Url::parse(&request.base_url)
            .map_err(|_| "Configured endpoint URL is invalid.".to_string())?;
        url.set_path(&request.path);
        url.set_query(None);
        url.set_fragment(None);
        Ok(url)
    }
}

impl EndpointProbe for HttpEndpointProbe {
    fn probe(&self, request: &ProbeRequest) -> Result<ProbeObservation, String> {
        let response = self
            .client
            .get(Self::probe_url(request)?)
            .send()
            .map_err(|_| "Endpoint probe failed.".to_string())?;
        let status = response.status().as_u16();
        if request.purpose == ProbePurpose::Health {
            return Ok(ProbeObservation {
                status,
                runtime_implementation: None,
                runtime_version: None,
                model_ids: Vec::new(),
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_PROBE_BODY_BYTES as u64)
        {
            return Err("Endpoint probe response exceeded the safety limit.".to_string());
        }
        let mut body = Vec::with_capacity(MAX_PROBE_BODY_BYTES.min(16 * 1024));
        response
            .take((MAX_PROBE_BODY_BYTES + 1) as u64)
            .read_to_end(&mut body)
            .map_err(|_| "Endpoint probe response could not be read.".to_string())?;
        if body.len() > MAX_PROBE_BODY_BYTES {
            return Err("Endpoint probe response exceeded the safety limit.".to_string());
        }
        let value: Value = serde_json::from_slice(&body)
            .map_err(|_| "Endpoint probe returned malformed JSON.".to_string())?;
        let (runtime_implementation, runtime_version, model_ids) = match request.purpose {
            ProbePurpose::RuntimeIdentity => (
                Some("vllm".to_string()),
                value
                    .get("version")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                Vec::new(),
            ),
            ProbePurpose::Models => (
                None,
                None,
                value
                    .get("data")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|model| model.get("id").and_then(Value::as_str))
                    .map(ToOwned::to_owned)
                    .collect(),
            ),
            ProbePurpose::Health => unreachable!("handled before body parsing"),
        };
        Ok(ProbeObservation {
            status,
            runtime_implementation,
            runtime_version,
            model_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_paths_replace_the_configured_api_prefix_without_credentials() {
        let request = ProbeRequest {
            endpoint_id: "local".to_string(),
            base_url: "http://192.168.1.5:8000/v1".to_string(),
            path: "/health".to_string(),
            purpose: ProbePurpose::Health,
        };
        assert_eq!(
            HttpEndpointProbe::probe_url(&request).unwrap().as_str(),
            "http://192.168.1.5:8000/health"
        );
    }
}
