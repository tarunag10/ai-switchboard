//! Fail-closed provenance and SPDX license inventory for endpoint kinds.
//!
//! The repository and revision constants here are the single canonical pins;
//! `registry.rs` builds its `compatibility_source` strings from them so the
//! inventory cannot drift from enrollment. License values are verified SPDX
//! identifiers from each pinned upstream's own LICENSE file (SGLang and Envoy
//! AI Gateway: Apache-2.0; llama.cpp: MIT; vLLM, Dynamo, TensorRT-LLM:
//! Apache-2.0). LiteLLM stays `"Unknown"` because its repository mixes MIT
//! with a separately licensed `enterprise/` tree, so no single SPDX expression
//! is accurate for the whole pin.

/// Kinds mirror `registry::ManagedEndpoint` variants. Adding a variant
/// requires extending the exhaustive inventory match below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EndpointKind {
    OpenAiCompatible,
    Vllm,
    Sglang,
    LlamaCpp,
    LiteLlm,
    EnvoyAiGateway,
    Dynamo,
    TensorRtLlm,
}

pub(crate) const VLLM_UPSTREAM_REPO: &str = "vllm-project/vllm";
pub(crate) const VLLM_PINNED_REVISION: &str = "fe1c317157d4478fdc0e02096447e61305b871e9";
pub(crate) const SGLANG_UPSTREAM_REPO: &str = "sgl-project/sglang";
pub(crate) const SGLANG_PINNED_REVISION: &str = "d3589a7251e4df6710e14ac55071585e80ae62c7";
pub(crate) const LLAMA_CPP_UPSTREAM_REPO: &str = "ggml-org/llama.cpp";
pub(crate) const LLAMA_CPP_PINNED_REVISION: &str = "4df29be4f4c3673f428170fda944a5b19f743bb8";
pub(crate) const LITELLM_UPSTREAM_REPO: &str = "BerriAI/litellm";
pub(crate) const LITELLM_PINNED_REVISION: &str = "bc6e7df05b018eefe6c7293790ca3f4de38709ac";
pub(crate) const ENVOY_AI_GATEWAY_UPSTREAM_REPO: &str = "envoyproxy/ai-gateway";
pub(crate) const ENVOY_AI_GATEWAY_PINNED_REVISION: &str =
    "06381c5195178b349fa5b77648179775f0b1d839";
pub(crate) const DYNAMO_UPSTREAM_REPO: &str = "ai-dynamo/dynamo";
pub(crate) const DYNAMO_PINNED_REVISION: &str = "4ae1af02db404c6268c4560df1071c0225f88b36";
pub(crate) const TENSORRT_LLM_UPSTREAM_REPO: &str = "NVIDIA/TensorRT-LLM";
pub(crate) const TENSORRT_LLM_PINNED_REVISION: &str = "210397bedcbec4305722942b49ddcb17c1cce3c1";

/// License identifiers our own integration docs state. Docs currently name no
/// license for these runtimes, so they remain unknown pending R9 evidence.
pub(crate) const LICENSE_UNKNOWN: &str = "Unknown";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProvenanceEntry {
    pub(crate) upstream_repo: Option<&'static str>,
    pub(crate) pinned_revision: Option<&'static str>,
    pub(crate) license_spdx: &'static str,
}

impl EndpointKind {
    /// Stable wire/diagnostic label for this kind, as used by registry
    /// diagnostics.
    pub(crate) fn runtime_kind(&self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "open_ai_compatible",
            Self::Vllm => "vllm",
            Self::Sglang => "sglang",
            Self::LlamaCpp => "llama_cpp",
            Self::LiteLlm => "litellm",
            Self::EnvoyAiGateway => "envoy_ai_gateway",
            Self::Dynamo => "dynamo",
            Self::TensorRtLlm => "tensorrt_llm",
        }
    }

    /// Canonical `repo@revision` pin string used by endpoint profiles.
    /// Only meaningful for kinds with a pinned upstream.
    pub(crate) fn pinned_source(&self) -> String {
        let entry = self.provenance();
        format!(
            "{}@{}",
            entry
                .upstream_repo
                .expect("pinned kinds must declare an upstream repo"),
            entry
                .pinned_revision
                .expect("pinned kinds must declare a revision")
        )
    }

    pub(crate) fn provenance(&self) -> ProvenanceEntry {
        match self {
            Self::OpenAiCompatible => ProvenanceEntry {
                upstream_repo: None,
                pinned_revision: None,
                license_spdx: LICENSE_UNKNOWN,
            },
            Self::Vllm => ProvenanceEntry {
                upstream_repo: Some(VLLM_UPSTREAM_REPO),
                pinned_revision: Some(VLLM_PINNED_REVISION),
                license_spdx: "Apache-2.0",
            },
            Self::Sglang => ProvenanceEntry {
                upstream_repo: Some(SGLANG_UPSTREAM_REPO),
                pinned_revision: Some(SGLANG_PINNED_REVISION),
                license_spdx: "Apache-2.0",
            },
            Self::LlamaCpp => ProvenanceEntry {
                upstream_repo: Some(LLAMA_CPP_UPSTREAM_REPO),
                pinned_revision: Some(LLAMA_CPP_PINNED_REVISION),
                license_spdx: "MIT",
            },
            Self::LiteLlm => ProvenanceEntry {
                upstream_repo: Some(LITELLM_UPSTREAM_REPO),
                pinned_revision: Some(LITELLM_PINNED_REVISION),
                license_spdx: LICENSE_UNKNOWN,
            },
            Self::EnvoyAiGateway => ProvenanceEntry {
                upstream_repo: Some(ENVOY_AI_GATEWAY_UPSTREAM_REPO),
                pinned_revision: Some(ENVOY_AI_GATEWAY_PINNED_REVISION),
                license_spdx: "Apache-2.0",
            },
            Self::Dynamo => ProvenanceEntry {
                upstream_repo: Some(DYNAMO_UPSTREAM_REPO),
                pinned_revision: Some(DYNAMO_PINNED_REVISION),
                license_spdx: "Apache-2.0",
            },
            Self::TensorRtLlm => ProvenanceEntry {
                upstream_repo: Some(TENSORRT_LLM_UPSTREAM_REPO),
                pinned_revision: Some(TENSORRT_LLM_PINNED_REVISION),
                license_spdx: "Apache-2.0",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_KINDS: [EndpointKind; 8] = [
        EndpointKind::OpenAiCompatible,
        EndpointKind::Vllm,
        EndpointKind::Sglang,
        EndpointKind::LlamaCpp,
        EndpointKind::LiteLlm,
        EndpointKind::EnvoyAiGateway,
        EndpointKind::Dynamo,
        EndpointKind::TensorRtLlm,
    ];

    #[test]
    fn every_kind_has_a_complete_inventory_entry() {
        for kind in ALL_KINDS {
            let entry = kind.provenance();
            assert_eq!(
                entry.upstream_repo.is_some(),
                kind != EndpointKind::OpenAiCompatible
            );
            assert_eq!(entry.pinned_revision.is_some(), entry.upstream_repo.is_some());
            assert!(!entry.license_spdx.is_empty());
        }
    }

    #[test]
    fn pinned_sources_match_the_registry_compatibility_pins() {
        use crate::inference_endpoint::{
            DynamoEndpoint, EnterpriseGatewayEndpoint, LiteLlmEndpoint, LlamaCppEndpoint,
            SglangEndpoint, TensorRtLlmEndpoint, VllmEndpoint,
        };

        let vllm = VllmEndpoint::new(
            "id",
            "label",
            "http://127.0.0.1:8000/v1",
            "model",
            None,
            "bench",
        )
        .unwrap();
        assert_eq!(vllm.compatibility_source, EndpointKind::Vllm.pinned_source());

        let sglang = SglangEndpoint::new(
            "id",
            "label",
            "http://127.0.0.1:30000/v1",
            "model",
            None,
        )
        .unwrap();
        assert_eq!(
            sglang.compatibility_source,
            EndpointKind::Sglang.pinned_source()
        );

        let llama_cpp = LlamaCppEndpoint::new(
            "id",
            "label",
            "http://127.0.0.1:8080/v1",
            "model",
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            llama_cpp.compatibility_source,
            EndpointKind::LlamaCpp.pinned_source()
        );

        let litellm = LiteLlmEndpoint::new(
            "id",
            "label",
            "http://127.0.0.1:4000/v1",
            "model",
            None,
            false,
        )
        .unwrap();
        assert_eq!(
            litellm.compatibility_source,
            EndpointKind::LiteLlm.pinned_source()
        );

        let envoy = EnterpriseGatewayEndpoint::new(
            "id",
            "label",
            "http://127.0.0.1:8081/v1",
            "model",
            None,
            false,
        )
        .unwrap();
        assert_eq!(
            envoy.compatibility_source,
            EndpointKind::EnvoyAiGateway.pinned_source()
        );

        let dynamo = DynamoEndpoint::new(
            "id",
            "label",
            "http://127.0.0.1:8082/v1",
            "model",
            None,
            false,
        )
        .unwrap();
        assert_eq!(
            dynamo.compatibility_source,
            EndpointKind::Dynamo.pinned_source()
        );

        let tensorrt = TensorRtLlmEndpoint::new(
            "id",
            "label",
            "http://127.0.0.1:8083/v1",
            "model",
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            tensorrt.compatibility_source,
            EndpointKind::TensorRtLlm.pinned_source()
        );
    }
}
