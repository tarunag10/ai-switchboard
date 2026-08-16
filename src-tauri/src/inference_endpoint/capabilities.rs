//! Runtime-neutral capability evidence used by routing and diagnostics.
//!
//! `supported` describes a runtime's documented ability. `configured` and
//! `observed` describe progressively stronger evidence about this endpoint.
//! A supported feature must not be treated as enabled without configured or
//! observed evidence.

use serde::{Deserialize, Serialize};

use super::{EndpointCapabilities, InferenceEndpoint};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityState {
    Supported,
    Unsupported,
    Unknown,
    Configured,
    Observed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedCapability {
    pub state: CapabilityState,
    pub numeric_value: Option<u64>,
    pub evidence: Vec<String>,
}

impl NormalizedCapability {
    fn new(state: CapabilityState, evidence: impl Into<String>) -> Self {
        Self {
            state,
            numeric_value: None,
            evidence: vec![evidence.into()],
        }
    }

    fn configured_value(value: Option<u64>, evidence: impl Into<String>) -> Self {
        match value {
            Some(value) => Self {
                state: CapabilityState::Configured,
                numeric_value: Some(value),
                evidence: vec![evidence.into()],
            },
            None => Self::new(CapabilityState::Unknown, evidence),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedRuntimeCapabilities {
    pub prefix_cache: NormalizedCapability,
    pub speculative_decoding: NormalizedCapability,
    pub continuous_batching: NormalizedCapability,
    pub disaggregated_prefill_decode: NormalizedCapability,
    pub quantization: NormalizedCapability,
    pub parallelism: NormalizedCapability,
    pub max_context: NormalizedCapability,
    pub tool_calling: NormalizedCapability,
}

impl NormalizedRuntimeCapabilities {
    pub(crate) fn unknown(endpoint: &dyn InferenceEndpoint) -> Self {
        let capabilities = endpoint.capabilities();
        Self {
            prefix_cache: unknown("generic OpenAI-compatible endpoint"),
            speculative_decoding: unknown("generic OpenAI-compatible endpoint"),
            continuous_batching: unknown("generic OpenAI-compatible endpoint"),
            disaggregated_prefill_decode: unknown("generic OpenAI-compatible endpoint"),
            quantization: unknown("generic OpenAI-compatible endpoint"),
            parallelism: unknown("generic OpenAI-compatible endpoint"),
            max_context: NormalizedCapability::configured_value(
                capabilities.max_context,
                "user endpoint configuration",
            ),
            tool_calling: configured_bool(capabilities.tools, "generic endpoint declaration"),
        }
    }

    pub(crate) fn vllm(endpoint: &dyn InferenceEndpoint) -> Self {
        let source = "vllm-project/vllm@fe1c317157d4478fdc0e02096447e61305b871e9";
        runtime_profile(endpoint, source)
    }

    pub(crate) fn sglang(endpoint: &dyn InferenceEndpoint) -> Self {
        let source = "sgl-project/sglang@d3589a7251e4df6710e14ac55071585e80ae62c7";
        runtime_profile(endpoint, source)
    }

    pub(crate) fn llama_cpp(endpoint: &dyn InferenceEndpoint, quantization: Option<&str>) -> Self {
        let source = "ggml-org/llama.cpp@4df29be4f4c3673f428170fda944a5b19f743bb8";
        Self {
            prefix_cache: supported(source),
            speculative_decoding: supported(source),
            continuous_batching: unknown("not established by the endpoint profile"),
            disaggregated_prefill_decode: unknown("not established by the endpoint profile"),
            quantization: match quantization {
                Some(value) => configured(format!("user endpoint configuration: {value}")),
                None => supported(source),
            },
            parallelism: supported(source),
            max_context: NormalizedCapability::configured_value(
                endpoint.capabilities().max_context,
                "user endpoint configuration",
            ),
            tool_calling: unknown("model and chat-template dependent"),
        }
    }

    pub(crate) fn litellm(endpoint: &dyn InferenceEndpoint) -> Self {
        let mut profile = Self::unknown(endpoint);
        let evidence = "LiteLLM gateway capability is downstream-model dependent";
        profile.prefix_cache = unknown(evidence);
        profile.speculative_decoding = unknown(evidence);
        profile.continuous_batching = unknown(evidence);
        profile.disaggregated_prefill_decode = unknown(evidence);
        profile.quantization = unknown(evidence);
        profile.parallelism = unknown(evidence);
        profile.tool_calling = unknown(evidence);
        profile
    }
}

fn runtime_profile(
    endpoint: &dyn InferenceEndpoint,
    source: &'static str,
) -> NormalizedRuntimeCapabilities {
    NormalizedRuntimeCapabilities {
        // These are documented runtime abilities. No endpoint activation is
        // inferred from them.
        prefix_cache: supported(source),
        speculative_decoding: supported(source),
        continuous_batching: supported(source),
        disaggregated_prefill_decode: supported(source),
        quantization: supported(source),
        parallelism: supported(source),
        max_context: NormalizedCapability::configured_value(
            endpoint.capabilities().max_context,
            "user endpoint configuration",
        ),
        tool_calling: bool_support(endpoint.capabilities(), source),
    }
}

fn bool_support(
    capabilities: &EndpointCapabilities,
    evidence: impl Into<String>,
) -> NormalizedCapability {
    NormalizedCapability::new(
        if capabilities.tools {
            CapabilityState::Supported
        } else {
            CapabilityState::Unsupported
        },
        evidence,
    )
}

fn configured_bool(configured: bool, evidence: impl Into<String>) -> NormalizedCapability {
    NormalizedCapability::new(
        if configured {
            CapabilityState::Configured
        } else {
            CapabilityState::Unsupported
        },
        evidence,
    )
}

fn supported(evidence: impl Into<String>) -> NormalizedCapability {
    NormalizedCapability::new(CapabilityState::Supported, evidence)
}

fn configured(evidence: impl Into<String>) -> NormalizedCapability {
    NormalizedCapability::new(CapabilityState::Configured, evidence)
}

fn unknown(evidence: impl Into<String>) -> NormalizedCapability {
    NormalizedCapability::new(CapabilityState::Unknown, evidence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference_endpoint::{
        CredentialStrategy, HealthPolicy, InferenceProtocol, ModelDiscovery,
        OpenAiCompatibleEndpoint, PrefixCacheEvidence,
    };

    fn endpoint(max_context: Option<u64>) -> OpenAiCompatibleEndpoint {
        OpenAiCompatibleEndpoint::new(
            "test",
            "Test",
            "http://127.0.0.1:30000/v1",
            HealthPolicy::Passive,
            "model",
            EndpointCapabilities {
                protocol: InferenceProtocol::OpenAiCompatible,
                streaming: true,
                tools: true,
                structured_output: true,
                max_context,
                prefix_cache_evidence: PrefixCacheEvidence::Unknown,
                health_endpoint: None,
                model_discovery: ModelDiscovery::Static,
            },
            CredentialStrategy::None,
            false,
        )
        .unwrap()
    }

    #[test]
    fn runtime_support_does_not_claim_endpoint_activation() {
        let endpoint = endpoint(Some(65_536));
        let capabilities = NormalizedRuntimeCapabilities::sglang(&endpoint);

        assert_eq!(capabilities.prefix_cache.state, CapabilityState::Supported);
        assert_eq!(capabilities.max_context.state, CapabilityState::Configured);
        assert_eq!(capabilities.max_context.numeric_value, Some(65_536));
        assert_ne!(
            capabilities.speculative_decoding.state,
            CapabilityState::Observed
        );
    }

    #[test]
    fn absent_endpoint_evidence_remains_unknown() {
        let endpoint = endpoint(None);
        let capabilities = NormalizedRuntimeCapabilities::unknown(&endpoint);

        assert_eq!(capabilities.prefix_cache.state, CapabilityState::Unknown);
        assert_eq!(capabilities.max_context.state, CapabilityState::Unknown);
        assert_eq!(capabilities.tool_calling.state, CapabilityState::Configured);
    }

    #[test]
    fn llama_cpp_keeps_configured_quantization_distinct_from_runtime_support() {
        let endpoint = endpoint(Some(8_192));
        let capabilities = NormalizedRuntimeCapabilities::llama_cpp(&endpoint, Some("Q4_K_M"));

        assert_eq!(capabilities.quantization.state, CapabilityState::Configured);
        assert_eq!(
            capabilities.disaggregated_prefill_decode.state,
            CapabilityState::Unknown
        );
        assert_eq!(capabilities.tool_calling.state, CapabilityState::Unknown);
    }

    #[test]
    fn litellm_does_not_inherit_downstream_runtime_capabilities() {
        let endpoint = endpoint(Some(32_768));
        let capabilities = NormalizedRuntimeCapabilities::litellm(&endpoint);

        assert_eq!(capabilities.prefix_cache.state, CapabilityState::Unknown);
        assert_eq!(capabilities.quantization.state, CapabilityState::Unknown);
        assert_eq!(capabilities.max_context.state, CapabilityState::Configured);
    }
}
