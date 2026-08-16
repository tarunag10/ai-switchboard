//! Identity, configuration, and capability boundary for inference endpoints.
//!
//! Phase 1 supports only the current remote-provider shape and a generic
//! OpenAI-compatible endpoint. It does not select an endpoint, send a request,
//! discover a network service, or contain runtime-specific behavior.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use url::Url;

mod capabilities;
mod registry;

pub(crate) use capabilities::{CapabilityState, NormalizedRuntimeCapabilities};

pub(crate) use registry::{
    DynamoEndpoint, EndpointAllowlist, EndpointDiagnostic, EndpointProbe, EndpointRegistry,
    EndpointRegistryService, EndpointVerification, EnterpriseGatewayEndpoint, LiteLlmEndpoint,
    LlamaCppEndpoint, ProbeObservation, ProbePurpose, ProbeRequest, RouteTarget, SglangEndpoint,
    TensorRtLlmEndpoint, UserEndpointApproval, VllmEndpoint,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InferenceProtocol {
    OpenAiCompatible,
    AnthropicCompatible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrefixCacheEvidence {
    Unknown,
    ProviderDeclared,
    Measured,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub(crate) enum ModelDiscovery {
    Unsupported,
    Static,
    Endpoint { path: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointCapabilities {
    pub protocol: InferenceProtocol,
    pub streaming: bool,
    pub tools: bool,
    pub structured_output: bool,
    pub max_context: Option<u64>,
    pub prefix_cache_evidence: PrefixCacheEvidence,
    pub health_endpoint: Option<String>,
    pub model_discovery: ModelDiscovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HealthPolicy {
    Disabled,
    Passive,
    Active,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub(crate) enum CredentialStrategy {
    ClientProvided,
    EnvironmentVariable { variable: String },
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SecurityClassification {
    RemoteProvider,
    UserConfiguredRemote,
    LocalLoopback,
    LocalNetwork,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EndpointProfile {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) base_url: String,
    pub(super) protocol: InferenceProtocol,
    pub(super) health_policy: HealthPolicy,
    pub(super) model_id: String,
    pub(super) capabilities: EndpointCapabilities,
    pub(super) credential_strategy: CredentialStrategy,
    pub(super) security_classification: SecurityClassification,
    pub(super) enabled: bool,
}

/// Object-safe, read-only endpoint boundary shared by policy and benchmarks.
pub(crate) trait InferenceEndpoint: Send + Sync {
    fn id(&self) -> &str;
    fn label(&self) -> &str;
    fn base_url(&self) -> &str;
    fn protocol(&self) -> InferenceProtocol;
    fn health_policy(&self) -> HealthPolicy;
    fn model_id(&self) -> &str;
    fn capabilities(&self) -> &EndpointCapabilities;
    fn credential_strategy(&self) -> &CredentialStrategy;
    fn security_classification(&self) -> SecurityClassification;
    fn enabled(&self) -> bool;
}

/// Compatibility representation for the provider-hosted route used today.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct CurrentRemoteProviderEndpoint {
    profile: EndpointProfile,
}

impl CurrentRemoteProviderEndpoint {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        base_url: impl Into<String>,
        protocol: InferenceProtocol,
        health_policy: HealthPolicy,
        model_id: impl Into<String>,
        capabilities: EndpointCapabilities,
        credential_strategy: CredentialStrategy,
        enabled: bool,
    ) -> Result<Self, String> {
        let profile = EndpointProfile {
            id: id.into(),
            label: label.into(),
            base_url: validate_base_url(&base_url.into(), false)?,
            protocol,
            health_policy,
            model_id: model_id.into(),
            capabilities,
            credential_strategy,
            security_classification: SecurityClassification::RemoteProvider,
            enabled,
        };
        validate_profile(&profile)?;
        Ok(Self { profile })
    }
}

/// User-configured endpoint speaking the generic OpenAI-compatible protocol.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct OpenAiCompatibleEndpoint {
    profile: EndpointProfile,
}

impl OpenAiCompatibleEndpoint {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        base_url: impl Into<String>,
        health_policy: HealthPolicy,
        model_id: impl Into<String>,
        capabilities: EndpointCapabilities,
        credential_strategy: CredentialStrategy,
        enabled: bool,
    ) -> Result<Self, String> {
        let base_url = validate_base_url(&base_url.into(), true)?;
        let parsed =
            Url::parse(&base_url).map_err(|err| format!("invalid endpoint base URL: {err}"))?;
        let security_classification = classify_user_endpoint(&parsed);
        let profile = EndpointProfile {
            id: id.into(),
            label: label.into(),
            base_url,
            protocol: InferenceProtocol::OpenAiCompatible,
            health_policy,
            model_id: model_id.into(),
            capabilities,
            credential_strategy,
            security_classification,
            enabled,
        };
        validate_profile(&profile)?;
        Ok(Self { profile })
    }
}

macro_rules! impl_endpoint {
    ($endpoint:ty) => {
        impl InferenceEndpoint for $endpoint {
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
    };
}

impl_endpoint!(CurrentRemoteProviderEndpoint);
impl_endpoint!(OpenAiCompatibleEndpoint);

fn classify_user_endpoint(parsed: &Url) -> SecurityClassification {
    let Some(host) = parsed.host_str() else {
        return SecurityClassification::UserConfiguredRemote;
    };
    if matches!(host, "localhost" | "127.0.0.1" | "::1") {
        return SecurityClassification::LocalLoopback;
    }
    if host.ends_with(".local") {
        return SecurityClassification::LocalNetwork;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        let private = match ip {
            IpAddr::V4(ip) => ip.is_private() || ip.is_link_local(),
            IpAddr::V6(ip) => ip.is_unique_local() || ip.is_unicast_link_local(),
        };
        if private {
            return SecurityClassification::LocalNetwork;
        }
    }
    SecurityClassification::UserConfiguredRemote
}

fn validate_profile(profile: &EndpointProfile) -> Result<(), String> {
    if profile.id.trim().is_empty() {
        return Err("endpoint id is required".to_string());
    }
    if profile.label.trim().is_empty() {
        return Err("endpoint label is required".to_string());
    }
    if profile.model_id.trim().is_empty() {
        return Err("endpoint model id is required".to_string());
    }
    if profile.protocol != profile.capabilities.protocol {
        return Err("endpoint protocol must match its capability protocol".to_string());
    }
    if profile.health_policy == HealthPolicy::Active
        && profile.capabilities.health_endpoint.is_none()
    {
        return Err("active health policy requires a health endpoint".to_string());
    }
    Ok(())
}

fn validate_base_url(raw: &str, allow_private_http: bool) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let parsed = Url::parse(trimmed).map_err(|err| format!("invalid endpoint base URL: {err}"))?;
    let private = matches!(
        classify_user_endpoint(&parsed),
        SecurityClassification::LocalLoopback | SecurityClassification::LocalNetwork
    );
    if parsed.host_str().is_none() || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(
            "endpoint base URL requires a host and must not contain credentials".to_string(),
        );
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("endpoint base URL must not contain a query or fragment".to_string());
    }
    if parsed.scheme() != "https" && !(allow_private_http && parsed.scheme() == "http" && private) {
        return Err(
            "endpoint base URL must use HTTPS; local and LAN HTTP are allowed for user-managed endpoints"
                .to_string(),
        );
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities(protocol: InferenceProtocol) -> EndpointCapabilities {
        EndpointCapabilities {
            protocol,
            streaming: true,
            tools: true,
            structured_output: false,
            max_context: Some(200_000),
            prefix_cache_evidence: PrefixCacheEvidence::ProviderDeclared,
            health_endpoint: None,
            model_discovery: ModelDiscovery::Static,
        }
    }

    #[test]
    fn current_remote_provider_exposes_the_complete_endpoint_contract() {
        let endpoint = CurrentRemoteProviderEndpoint::new(
            "current-anthropic",
            "Current remote provider",
            "https://api.anthropic.com/",
            InferenceProtocol::AnthropicCompatible,
            HealthPolicy::Passive,
            "claude-current",
            capabilities(InferenceProtocol::AnthropicCompatible),
            CredentialStrategy::ClientProvided,
            true,
        )
        .unwrap();
        let endpoint: &dyn InferenceEndpoint = &endpoint;

        assert_eq!(endpoint.id(), "current-anthropic");
        assert_eq!(endpoint.label(), "Current remote provider");
        assert_eq!(endpoint.base_url(), "https://api.anthropic.com");
        assert_eq!(endpoint.protocol(), InferenceProtocol::AnthropicCompatible);
        assert_eq!(endpoint.health_policy(), HealthPolicy::Passive);
        assert_eq!(endpoint.model_id(), "claude-current");
        assert!(endpoint.capabilities().streaming);
        assert_eq!(
            endpoint.credential_strategy(),
            &CredentialStrategy::ClientProvided
        );
        assert_eq!(
            endpoint.security_classification(),
            SecurityClassification::RemoteProvider
        );
        assert!(endpoint.enabled());
    }

    #[test]
    fn generic_openai_endpoint_allows_local_loopback_and_keeps_capabilities_as_data() {
        let endpoint = OpenAiCompatibleEndpoint::new(
            "local-openai",
            "Local OpenAI-compatible",
            "http://127.0.0.1:8000/v1",
            HealthPolicy::Active,
            "local-model",
            EndpointCapabilities {
                protocol: InferenceProtocol::OpenAiCompatible,
                streaming: true,
                tools: false,
                structured_output: true,
                max_context: Some(32_768),
                prefix_cache_evidence: PrefixCacheEvidence::Measured,
                health_endpoint: Some("/health".to_string()),
                model_discovery: ModelDiscovery::Endpoint {
                    path: "/models".to_string(),
                },
            },
            CredentialStrategy::None,
            false,
        )
        .unwrap();

        assert_eq!(endpoint.protocol(), InferenceProtocol::OpenAiCompatible);
        assert_eq!(
            endpoint.security_classification(),
            SecurityClassification::LocalLoopback
        );
        assert_eq!(endpoint.capabilities().max_context, Some(32_768));
        assert!(!endpoint.enabled());
    }

    #[test]
    fn endpoint_validation_rejects_inconsistent_or_unsafe_profiles() {
        let mismatch = OpenAiCompatibleEndpoint::new(
            "mismatch",
            "Mismatch",
            "https://example.com/v1",
            HealthPolicy::Passive,
            "model",
            capabilities(InferenceProtocol::AnthropicCompatible),
            CredentialStrategy::EnvironmentVariable {
                variable: "EXAMPLE_API_KEY".to_string(),
            },
            true,
        );
        assert!(mismatch.is_err());

        let unsafe_remote = OpenAiCompatibleEndpoint::new(
            "unsafe",
            "Unsafe",
            "http://example.com/v1",
            HealthPolicy::Disabled,
            "model",
            capabilities(InferenceProtocol::OpenAiCompatible),
            CredentialStrategy::None,
            true,
        );
        assert!(unsafe_remote.is_err());
    }
}
