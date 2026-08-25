//! Built-in OSS interoperability capability metadata.
//!
//! This registry is deliberately not a provider router. It exposes bounded
//! labels and capability declarations only; credentials, URLs, request data,
//! and write operations never enter this surface.

use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OssProviderCapability {
    pub id: &'static str,
    pub label: &'static str,
    pub model_families: &'static [&'static str],
    pub context_limit: u64,
    pub auth_source: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OssToolCapability {
    pub id: &'static str,
    pub label: &'static str,
    pub provider_id: &'static str,
    pub capabilities: &'static [&'static str],
    pub requires_approval: bool,
    pub writes_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OssCapabilityRegistry {
    pub schema_version: u32,
    pub registry_mode: &'static str,
    pub writes_enabled: bool,
    pub approval_mode: &'static str,
    pub providers: Vec<OssProviderCapability>,
    pub tools: Vec<OssToolCapability>,
}

pub fn registry() -> OssCapabilityRegistry {
    OssCapabilityRegistry {
        schema_version: 1,
        registry_mode: "metadata_only",
        writes_enabled: false,
        approval_mode: "fail_closed",
        providers: vec![
            OssProviderCapability {
                id: "anthropic",
                label: "Anthropic",
                model_families: &["frontier"],
                context_limit: 200_000,
                auth_source: "keychain",
            },
            OssProviderCapability {
                id: "openai",
                label: "OpenAI",
                model_families: &["frontier"],
                context_limit: 200_000,
                auth_source: "environment",
            },
        ],
        tools: vec![
            OssToolCapability {
                id: "repo_context",
                label: "Repo context",
                provider_id: "anthropic",
                capabilities: &["context", "read_only"],
                requires_approval: true,
                writes_enabled: false,
            },
            OssToolCapability {
                id: "redacted_replay",
                label: "Redacted replay",
                provider_id: "openai",
                capabilities: &["replay", "observe_only"],
                requires_approval: true,
                writes_enabled: false,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::registry;

    #[test]
    fn registry_is_metadata_only_and_fail_closed() {
        let registry = registry();
        assert_eq!(registry.schema_version, 1);
        assert_eq!(registry.registry_mode, "metadata_only");
        assert!(!registry.writes_enabled);
        assert_eq!(registry.approval_mode, "fail_closed");
        assert!(registry
            .providers
            .iter()
            .all(|provider| !provider.auth_source.is_empty()));
        assert!(registry
            .tools
            .iter()
            .all(|tool| tool.requires_approval && !tool.writes_enabled));
    }

    #[test]
    fn registry_tool_providers_are_known() {
        let registry = registry();
        for tool in &registry.tools {
            assert!(registry
                .providers
                .iter()
                .any(|provider| provider.id == tool.provider_id));
        }
    }
}
