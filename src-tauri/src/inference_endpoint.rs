//! Identity and capability boundary for inference runtimes.
//!
//! Phase 0 intentionally keeps this interface read-only. It does not select an
//! endpoint, send a request, discover a network service, or promote routing.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InferenceProtocol {
    OpenAiCompatible,
    AnthropicCompatible,
    LocalMock,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InferenceCapabilities {
    pub streaming: bool,
    pub tool_use: bool,
    pub runtime_optimization: bool,
}

/// Object-safe identity/capability boundary shared by live benchmark targets.
///
/// Request execution deliberately remains outside this Phase 0 trait. Runtime
/// adapters can extend the boundary after request-path and failure semantics
/// are defined, without making the benchmark harness a routing mechanism.
pub(crate) trait InferenceEndpoint: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn protocol(&self) -> InferenceProtocol;
    fn capabilities(&self) -> InferenceCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LocalEndpoint;

    impl InferenceEndpoint for LocalEndpoint {
        fn id(&self) -> &str {
            "local"
        }

        fn display_name(&self) -> &str {
            "Local test endpoint"
        }

        fn protocol(&self) -> InferenceProtocol {
            InferenceProtocol::LocalMock
        }

        fn capabilities(&self) -> InferenceCapabilities {
            InferenceCapabilities {
                streaming: true,
                tool_use: false,
                runtime_optimization: true,
            }
        }
    }

    #[test]
    fn endpoint_boundary_is_object_safe_and_read_only() {
        let endpoint: &dyn InferenceEndpoint = &LocalEndpoint;
        assert_eq!(endpoint.id(), "local");
        assert_eq!(endpoint.display_name(), "Local test endpoint");
        assert_eq!(endpoint.protocol(), InferenceProtocol::LocalMock);
        assert!(endpoint.capabilities().streaming);
        assert!(endpoint.capabilities().runtime_optimization);
    }
}
