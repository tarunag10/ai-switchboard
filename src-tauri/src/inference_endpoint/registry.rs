//! Persisted, endpoint-agnostic selection state for user-managed inference.
//!
//! Verification never scans the network. It asks an injected probe for only
//! the paths declared by the selected profile. The vLLM paths are pinned to
//! the official vLLM OpenAI-compatible server contract observed at upstream
//! commit `fe1c317157d4478fdc0e02096447e61305b871e9` (2026-08-16): `/health`,
//! `/version`, and `/v1/models`. A runtime response is evidence, not proof of
//! that source revision, so its reported version is retained separately.

use super::{
    validate_profile, CredentialStrategy, EndpointCapabilities, EndpointProfile, HealthPolicy,
    InferenceEndpoint, InferenceProtocol, ModelDiscovery, OpenAiCompatibleEndpoint,
    PrefixCacheEvidence, SecurityClassification,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const REGISTRY_FILE: &str = "inference-endpoints.json";
const REGISTRY_SCHEMA_VERSION: u32 = 1;
const VLLM_HEALTH_PATH: &str = "/health";
const VLLM_VERSION_PATH: &str = "/version";
const VLLM_MODELS_PATH: &str = "/v1/models";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct EndpointAllowlist(BTreeSet<String>);

impl EndpointAllowlist {
    pub(crate) fn contains(&self, base_url: &str) -> bool {
        self.0.contains(base_url)
    }

    fn insert(&mut self, base_url: String) {
        self.0.insert(base_url);
    }
}

/// Proof that enrollment came from an explicit user action for this exact URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UserEndpointApproval {
    approved_base_url: String,
}

impl UserEndpointApproval {
    pub(crate) fn explicit(base_url: impl Into<String>) -> Result<Self, String> {
        let raw = base_url.into();
        let normalized = super::validate_base_url(&raw, true)?;
        Ok(Self {
            approved_base_url: normalized,
        })
    }

    fn authorizes(&self, base_url: &str) -> bool {
        self.approved_base_url == base_url
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VllmEndpoint {
    profile: EndpointProfile,
    pub benchmark_profile_id: String,
    pub compatibility_source: String,
}

impl VllmEndpoint {
    pub(crate) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        base_url: impl Into<String>,
        model_id: impl Into<String>,
        max_context: Option<u64>,
        benchmark_profile_id: impl Into<String>,
    ) -> Result<Self, String> {
        let endpoint = OpenAiCompatibleEndpoint::new(
            id,
            label,
            base_url,
            HealthPolicy::Active,
            model_id,
            EndpointCapabilities {
                protocol: InferenceProtocol::OpenAiCompatible,
                streaming: true,
                tools: true,
                structured_output: true,
                max_context,
                prefix_cache_evidence: PrefixCacheEvidence::Unknown,
                health_endpoint: Some(VLLM_HEALTH_PATH.to_string()),
                model_discovery: ModelDiscovery::Endpoint {
                    path: VLLM_MODELS_PATH.to_string(),
                },
            },
            CredentialStrategy::None,
            false,
        )?;
        let benchmark_profile_id = benchmark_profile_id.into();
        if benchmark_profile_id.trim().is_empty() {
            return Err("vLLM benchmark profile id is required".to_string());
        }
        Ok(Self {
            profile: endpoint.profile,
            benchmark_profile_id,
            compatibility_source: "vllm-project/vllm@fe1c317157d4478fdc0e02096447e61305b871e9"
                .to_string(),
        })
    }
}

impl InferenceEndpoint for VllmEndpoint {
    fn id(&self) -> &str {
        &self.profile.id
    }
    fn label(&self) -> &str {
        &self.profile.label
    }
    fn base_url(&self) -> &str {
        &self.profile.base_url
    }
    fn protocol(&self) -> InferenceProtocol {
        self.profile.protocol
    }
    fn health_policy(&self) -> HealthPolicy {
        self.profile.health_policy
    }
    fn model_id(&self) -> &str {
        &self.profile.model_id
    }
    fn capabilities(&self) -> &EndpointCapabilities {
        &self.profile.capabilities
    }
    fn credential_strategy(&self) -> &CredentialStrategy {
        &self.profile.credential_strategy
    }
    fn security_classification(&self) -> SecurityClassification {
        self.profile.security_classification
    }
    fn enabled(&self) -> bool {
        self.profile.enabled
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbePurpose {
    Health,
    RuntimeIdentity,
    Models,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProbeRequest {
    pub endpoint_id: String,
    pub base_url: String,
    pub path: String,
    pub purpose: ProbePurpose,
}

/// Sanitized observation only: implementations must not return raw bodies,
/// headers, credentials, or request URLs in errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProbeObservation {
    pub status: u16,
    pub runtime_implementation: Option<String>,
    pub runtime_version: Option<String>,
    pub model_ids: Vec<String>,
}

pub(crate) trait EndpointProbe {
    fn probe(&self, request: &ProbeRequest) -> Result<ProbeObservation, String>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub(crate) enum EndpointVerification {
    Unverified,
    Verified {
        runtime_id: Option<String>,
        runtime_version: Option<String>,
        model_ids: Vec<String>,
        benchmark_profile_id: Option<String>,
    },
    Failed {
        reason: String,
    },
}

impl EndpointVerification {
    fn is_verified(&self) -> bool {
        matches!(self, Self::Verified { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "profile")]
enum ManagedEndpoint {
    OpenAi(OpenAiCompatibleEndpoint),
    Vllm(VllmEndpoint),
}

impl ManagedEndpoint {
    fn endpoint(&self) -> &dyn InferenceEndpoint {
        match self {
            Self::OpenAi(value) => value,
            Self::Vllm(value) => value,
        }
    }

    fn profile_mut(&mut self) -> &mut EndpointProfile {
        match self {
            Self::OpenAi(value) => &mut value.profile,
            Self::Vllm(value) => &mut value.profile,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EndpointRecord {
    endpoint: ManagedEndpoint,
    verification: EndpointVerification,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointRegistry {
    schema_version: u32,
    allowlist: EndpointAllowlist,
    records: BTreeMap<String, EndpointRecord>,
    selected_endpoint_id: Option<String>,
}

impl Default for EndpointRegistry {
    fn default() -> Self {
        Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            allowlist: EndpointAllowlist::default(),
            records: BTreeMap::new(),
            selected_endpoint_id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointDiagnostic {
    pub id: String,
    pub label: String,
    pub location_class: SecurityClassification,
    pub enabled: bool,
    pub selected: bool,
    pub verification: EndpointVerification,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteTarget {
    pub endpoint_id: String,
    pub base_url: String,
    pub protocol: InferenceProtocol,
    pub model_id: String,
    pub security_classification: SecurityClassification,
}

impl EndpointRegistry {
    fn add(
        &mut self,
        endpoint: ManagedEndpoint,
        approval: UserEndpointApproval,
    ) -> Result<(), String> {
        let view = endpoint.endpoint();
        if !approval.authorizes(view.base_url()) {
            return Err("approval must match the exact normalized endpoint URL".to_string());
        }
        if self.records.contains_key(view.id()) {
            return Err("endpoint id already exists".to_string());
        }
        let id = view.id().to_string();
        let base_url = view.base_url().to_string();
        self.allowlist.insert(base_url);
        self.records.insert(
            id,
            EndpointRecord {
                endpoint,
                verification: EndpointVerification::Unverified,
            },
        );
        Ok(())
    }

    fn verify(
        &mut self,
        id: &str,
        probe: &dyn EndpointProbe,
    ) -> Result<EndpointVerification, String> {
        let verification = {
            let record = self
                .records
                .get_mut(id)
                .ok_or_else(|| "endpoint not found".to_string())?;
            if !self
                .allowlist
                .contains(record.endpoint.endpoint().base_url())
            {
                return Err("endpoint URL is not allowlisted".to_string());
            }
            let result = verify_record(record, probe);
            record.verification = match result {
                Ok(value) => value,
                Err(reason) => EndpointVerification::Failed { reason },
            };
            if !record.verification.is_verified() {
                record.endpoint.profile_mut().enabled = false;
            }
            record.verification.clone()
        };
        if !verification.is_verified() && self.selected_endpoint_id.as_deref() == Some(id) {
            self.selected_endpoint_id = None;
        }
        Ok(verification)
    }

    fn select(&mut self, id: &str) -> Result<(), String> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| "endpoint not found".to_string())?;
        if !record.verification.is_verified() {
            return Err("endpoint must pass verification before selection".to_string());
        }
        record.endpoint.profile_mut().enabled = true;
        self.selected_endpoint_id = Some(id.to_string());
        Ok(())
    }

    fn disable(&mut self, id: &str) -> Result<(), String> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| "endpoint not found".to_string())?;
        record.endpoint.profile_mut().enabled = false;
        if self.selected_endpoint_id.as_deref() == Some(id) {
            self.selected_endpoint_id = None;
        }
        Ok(())
    }

    pub(crate) fn diagnostics(&self) -> Vec<EndpointDiagnostic> {
        self.records
            .iter()
            .map(|(id, record)| {
                let endpoint = record.endpoint.endpoint();
                EndpointDiagnostic {
                    id: id.clone(),
                    label: endpoint.label().to_string(),
                    location_class: endpoint.security_classification(),
                    enabled: endpoint.enabled(),
                    selected: self.selected_endpoint_id.as_deref() == Some(id),
                    verification: record.verification.clone(),
                }
            })
            .collect()
    }

    pub(crate) fn selected_route_target(&self) -> Option<RouteTarget> {
        let id = self.selected_endpoint_id.as_deref()?;
        let record = self.records.get(id)?;
        let endpoint = record.endpoint.endpoint();
        (endpoint.enabled() && record.verification.is_verified()).then(|| RouteTarget {
            endpoint_id: endpoint.id().to_string(),
            base_url: endpoint.base_url().to_string(),
            protocol: endpoint.protocol(),
            model_id: endpoint.model_id().to_string(),
            security_classification: endpoint.security_classification(),
        })
    }

    fn validate_loaded(&self) -> Result<(), String> {
        if self.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err("unsupported endpoint registry schema version".to_string());
        }
        for (id, record) in &self.records {
            let endpoint = record.endpoint.endpoint();
            if id != endpoint.id() || !self.allowlist.contains(endpoint.base_url()) {
                return Err("endpoint registry integrity check failed".to_string());
            }
            match &record.endpoint {
                ManagedEndpoint::OpenAi(value) => validate_profile(&value.profile)?,
                ManagedEndpoint::Vllm(value) => validate_profile(&value.profile)?,
            }
        }
        if let Some(selected) = &self.selected_endpoint_id {
            let record = self
                .records
                .get(selected)
                .ok_or_else(|| "selected endpoint is missing".to_string())?;
            if !record.endpoint.endpoint().enabled() || !record.verification.is_verified() {
                return Err("selected endpoint is not enabled and verified".to_string());
            }
        }
        Ok(())
    }
}

fn verify_record(
    record: &EndpointRecord,
    probe: &dyn EndpointProbe,
) -> Result<EndpointVerification, String> {
    let endpoint = record.endpoint.endpoint();
    let request = |path: &str, purpose| ProbeRequest {
        endpoint_id: endpoint.id().to_string(),
        base_url: endpoint.base_url().to_string(),
        path: path.to_string(),
        purpose,
    };
    let successful = |observation: &ProbeObservation| (200..300).contains(&observation.status);

    if let Some(path) = &endpoint.capabilities().health_endpoint {
        let observation = probe
            .probe(&request(path, ProbePurpose::Health))
            .map_err(|_| "health probe failed".to_string())?;
        if !successful(&observation) {
            return Err("health probe returned a non-success status".to_string());
        }
    }

    let model_path = match &endpoint.capabilities().model_discovery {
        ModelDiscovery::Endpoint { path } => Some(path.as_str()),
        _ => None,
    };
    let models = if let Some(path) = model_path {
        let observation = probe
            .probe(&request(path, ProbePurpose::Models))
            .map_err(|_| "model probe failed".to_string())?;
        if !successful(&observation) {
            return Err("model probe returned a non-success status".to_string());
        }
        if !observation
            .model_ids
            .iter()
            .any(|value| value == endpoint.model_id())
        {
            return Err("configured model was not reported by the endpoint".to_string());
        }
        observation.model_ids
    } else {
        return Err("endpoint has no explicit verification probe".to_string());
    };

    match &record.endpoint {
        ManagedEndpoint::Vllm(vllm) => {
            let identity = probe
                .probe(&request(VLLM_VERSION_PATH, ProbePurpose::RuntimeIdentity))
                .map_err(|_| "runtime identity probe failed".to_string())?;
            if !successful(&identity)
                || identity
                    .runtime_implementation
                    .as_deref()
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                    != Some("vllm")
                || identity
                    .runtime_version
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err("runtime identity did not provide verified vLLM evidence".to_string());
            }
            Ok(EndpointVerification::Verified {
                runtime_id: Some("vllm".to_string()),
                runtime_version: identity.runtime_version,
                model_ids: models,
                benchmark_profile_id: Some(vllm.benchmark_profile_id.clone()),
            })
        }
        ManagedEndpoint::OpenAi(_) => Ok(EndpointVerification::Verified {
            runtime_id: None,
            runtime_version: None,
            model_ids: models,
            benchmark_profile_id: None,
        }),
    }
}

pub(crate) struct EndpointRegistryService {
    path: PathBuf,
    registry: EndpointRegistry,
}

impl EndpointRegistryService {
    pub(crate) fn load(base_dir: &Path) -> Result<Self, String> {
        let path = base_dir.join(REGISTRY_FILE);
        let registry = if path.exists() {
            let raw = fs::read_to_string(&path)
                .map_err(|err| format!("read endpoint registry: {err}"))?;
            let value: EndpointRegistry = serde_json::from_str(&raw)
                .map_err(|err| format!("parse endpoint registry: {err}"))?;
            value.validate_loaded()?;
            value
        } else {
            EndpointRegistry::default()
        };
        Ok(Self { path, registry })
    }

    pub(crate) fn registry_path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn list_diagnostics(&self) -> Vec<EndpointDiagnostic> {
        self.registry.diagnostics()
    }
    pub(crate) fn selected_route_target(&self) -> Option<RouteTarget> {
        self.registry.selected_route_target()
    }

    pub(crate) fn add_openai(
        &mut self,
        endpoint: OpenAiCompatibleEndpoint,
        approval: UserEndpointApproval,
    ) -> Result<(), String> {
        self.mutate(|registry| registry.add(ManagedEndpoint::OpenAi(endpoint), approval))
    }

    pub(crate) fn add_vllm(
        &mut self,
        endpoint: VllmEndpoint,
        approval: UserEndpointApproval,
    ) -> Result<(), String> {
        self.mutate(|registry| registry.add(ManagedEndpoint::Vllm(endpoint), approval))
    }

    pub(crate) fn verify(
        &mut self,
        id: &str,
        probe: &dyn EndpointProbe,
    ) -> Result<EndpointVerification, String> {
        self.mutate_with_result(|registry| registry.verify(id, probe))
    }

    pub(crate) fn select(&mut self, id: &str) -> Result<(), String> {
        self.mutate(|registry| registry.select(id))
    }

    pub(crate) fn disable(&mut self, id: &str) -> Result<(), String> {
        self.mutate(|registry| registry.disable(id))
    }

    fn mutate(
        &mut self,
        action: impl FnOnce(&mut EndpointRegistry) -> Result<(), String>,
    ) -> Result<(), String> {
        self.mutate_with_result(|registry| {
            action(registry)?;
            Ok(())
        })
    }

    fn mutate_with_result<T>(
        &mut self,
        action: impl FnOnce(&mut EndpointRegistry) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut next = self.registry.clone();
        let result = action(&mut next)?;
        persist(&self.path, &next)?;
        self.registry = next;
        Ok(result)
    }
}

fn persist(path: &Path, registry: &EndpointRegistry) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create endpoint registry directory: {err}"))?;
    }
    let bytes = serde_json::to_vec_pretty(registry)
        .map_err(|err| format!("serialize endpoint registry: {err}"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|err| format!("write endpoint registry: {err}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|err| format!("secure endpoint registry: {err}"))?;
    }
    fs::rename(&temporary, path).map_err(|err| format!("replace endpoint registry: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockProbe {
        requests: Mutex<Vec<ProbeRequest>>,
        fail_identity: bool,
    }
    impl EndpointProbe for MockProbe {
        fn probe(&self, request: &ProbeRequest) -> Result<ProbeObservation, String> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(match request.purpose {
                ProbePurpose::Health => ProbeObservation {
                    status: 200,
                    runtime_implementation: None,
                    runtime_version: None,
                    model_ids: vec![],
                },
                ProbePurpose::Models => ProbeObservation {
                    status: 200,
                    runtime_implementation: None,
                    runtime_version: None,
                    model_ids: vec!["test-model".into()],
                },
                ProbePurpose::RuntimeIdentity => ProbeObservation {
                    status: 200,
                    runtime_implementation: Some(
                        if self.fail_identity {
                            "unknown"
                        } else {
                            "vllm"
                        }
                        .into(),
                    ),
                    runtime_version: Some("0.10.test".into()),
                    model_ids: vec![],
                },
            })
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "switchboard-endpoint-{name}-{}",
            uuid::Uuid::new_v4()
        ))
    }

    fn vllm() -> VllmEndpoint {
        VllmEndpoint::new(
            "local-vllm",
            "Local vLLM",
            "http://192.168.1.20:8000/v1",
            "test-model",
            Some(32_768),
            "vllm-default-v1",
        )
        .unwrap()
    }

    fn generic_openai() -> OpenAiCompatibleEndpoint {
        OpenAiCompatibleEndpoint::new(
            "remote-openai",
            "Remote OpenAI-compatible",
            "https://inference.example.test/v1",
            HealthPolicy::Active,
            "test-model",
            EndpointCapabilities {
                protocol: InferenceProtocol::OpenAiCompatible,
                streaming: true,
                tools: false,
                structured_output: false,
                max_context: None,
                prefix_cache_evidence: PrefixCacheEvidence::Unknown,
                health_endpoint: Some("/health".to_string()),
                model_discovery: ModelDiscovery::Endpoint {
                    path: "/v1/models".to_string(),
                },
            },
            CredentialStrategy::EnvironmentVariable {
                variable: "PRIVATE_ENDPOINT_KEY".to_string(),
            },
            false,
        )
        .unwrap()
    }

    #[test]
    fn enrollment_requires_exact_explicit_approval_and_classifies_lan() {
        let dir = temp_dir("approval");
        let mut service = EndpointRegistryService::load(&dir).unwrap();
        let endpoint = vllm();
        let wrong = UserEndpointApproval::explicit("http://127.0.0.1:8000/v1").unwrap();
        assert!(service.add_vllm(endpoint.clone(), wrong).is_err());
        let approval = UserEndpointApproval::explicit(endpoint.base_url()).unwrap();
        service.add_vllm(endpoint, approval).unwrap();
        assert_eq!(
            service.list_diagnostics()[0].location_class,
            SecurityClassification::LocalNetwork
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn generic_openai_can_be_added_verified_and_selected_without_persisting_credentials() {
        let dir = temp_dir("generic");
        let mut service = EndpointRegistryService::load(&dir).unwrap();
        let endpoint = generic_openai();
        service
            .add_openai(
                endpoint.clone(),
                UserEndpointApproval::explicit(endpoint.base_url()).unwrap(),
            )
            .unwrap();
        let probe = MockProbe {
            requests: Mutex::new(vec![]),
            fail_identity: false,
        };
        assert!(matches!(
            service.verify(endpoint.id(), &probe).unwrap(),
            EndpointVerification::Verified {
                runtime_id: None,
                ..
            }
        ));
        service.select(endpoint.id()).unwrap();
        assert_eq!(
            service
                .selected_route_target()
                .unwrap()
                .security_classification,
            SecurityClassification::UserConfiguredRemote
        );
        let persisted = fs::read_to_string(service.registry_path()).unwrap();
        assert!(!persisted.contains("secret"));
        assert!(!persisted.contains("PRIVATE_ENDPOINT_KEY="));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn vllm_verification_uses_only_pinned_paths_then_selection_projects_route() {
        let dir = temp_dir("verify");
        let mut service = EndpointRegistryService::load(&dir).unwrap();
        let endpoint = vllm();
        service
            .add_vllm(
                endpoint.clone(),
                UserEndpointApproval::explicit(endpoint.base_url()).unwrap(),
            )
            .unwrap();
        assert!(service.select(endpoint.id()).is_err());
        let probe = MockProbe {
            requests: Mutex::new(vec![]),
            fail_identity: false,
        };
        let verified = service.verify(endpoint.id(), &probe).unwrap();
        assert!(
            matches!(verified, EndpointVerification::Verified { runtime_id: Some(ref value), .. } if value == "vllm")
        );
        let paths: BTreeSet<String> = probe
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|request| request.path.clone())
            .collect();
        assert_eq!(
            paths,
            BTreeSet::from([
                VLLM_HEALTH_PATH.to_string(),
                VLLM_MODELS_PATH.to_string(),
                VLLM_VERSION_PATH.to_string()
            ])
        );
        service.select(endpoint.id()).unwrap();
        let route = service.selected_route_target().unwrap();
        assert_eq!(route.endpoint_id, endpoint.id());
        assert_eq!(route.base_url, endpoint.base_url());
        service.disable(endpoint.id()).unwrap();
        assert!(service.selected_route_target().is_none());
        assert!(!service.registry_path().with_extension("json.tmp").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(service.registry_path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn failed_runtime_identity_is_redacted_and_persisted_fail_closed() {
        let dir = temp_dir("failure");
        let mut service = EndpointRegistryService::load(&dir).unwrap();
        let endpoint = vllm();
        service
            .add_vllm(
                endpoint.clone(),
                UserEndpointApproval::explicit(endpoint.base_url()).unwrap(),
            )
            .unwrap();
        let probe = MockProbe {
            requests: Mutex::new(vec![]),
            fail_identity: true,
        };
        let result = service.verify(endpoint.id(), &probe).unwrap();
        assert!(matches!(result, EndpointVerification::Failed { .. }));
        assert!(service.select(endpoint.id()).is_err());
        let loaded = EndpointRegistryService::load(&dir).unwrap();
        let diagnostic = &loaded.list_diagnostics()[0];
        assert!(!serde_json::to_string(diagnostic)
            .unwrap()
            .contains(endpoint.base_url()));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn failed_reverification_clears_selection_and_keeps_registry_loadable() {
        let dir = temp_dir("reverify");
        let mut service = EndpointRegistryService::load(&dir).unwrap();
        let endpoint = vllm();
        service
            .add_vllm(
                endpoint.clone(),
                UserEndpointApproval::explicit(endpoint.base_url()).unwrap(),
            )
            .unwrap();
        let passing = MockProbe {
            requests: Mutex::new(vec![]),
            fail_identity: false,
        };
        service.verify(endpoint.id(), &passing).unwrap();
        service.select(endpoint.id()).unwrap();

        let failing = MockProbe {
            requests: Mutex::new(vec![]),
            fail_identity: true,
        };
        assert!(matches!(
            service.verify(endpoint.id(), &failing).unwrap(),
            EndpointVerification::Failed { .. }
        ));
        assert!(service.selected_route_target().is_none());
        let loaded = EndpointRegistryService::load(&dir).unwrap();
        assert!(loaded.selected_route_target().is_none());
        let diagnostic = &loaded.list_diagnostics()[0];
        assert!(!diagnostic.enabled);
        assert!(!diagnostic.selected);
        let _ = fs::remove_dir_all(dir);
    }
}
