use serde::Deserialize;

use crate::models::ClientConnectorSupportStatus;

pub(crate) const CONNECTOR_MANIFEST_JSON: &str = include_str!("../../connectors/manifest.json");

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ConnectorManifest {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) category: String,
    pub(crate) support_status: String,
    pub(crate) detection: ConnectorManifestDetection,
    pub(crate) config: Option<ConnectorManifestConfig>,
    pub(crate) automation_gates: Vec<String>,
    pub(crate) manual_workflow: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ConnectorManifestDetection {
    #[serde(default)]
    pub(crate) binaries: Vec<String>,
    #[serde(default)]
    pub(crate) paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ConnectorManifestConfig {
    #[serde(default)]
    pub(crate) locations: Vec<String>,
    #[serde(default)]
    pub(crate) forbidden_reads: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlannedSidecarSpec {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) config_dir: &'static [&'static str],
}

pub(crate) const PLANNED_SIDECAR_SPECS: [PlannedSidecarSpec; 11] = [
    PlannedSidecarSpec {
        id: "gemini_cli",
        name: "Gemini CLI",
        config_dir: &[".gemini"],
    },
    PlannedSidecarSpec {
        id: "opencode",
        name: "OpenCode",
        config_dir: &[".config", "opencode"],
    },
    PlannedSidecarSpec {
        id: "cursor",
        name: "Cursor",
        config_dir: &["Library", "Application Support", "Cursor"],
    },
    PlannedSidecarSpec {
        id: "grok_cli",
        name: "Grok / xAI CLI",
        config_dir: &[".config", "xai"],
    },
    PlannedSidecarSpec {
        id: "aider",
        name: "Aider",
        config_dir: &[".config", "aider"],
    },
    PlannedSidecarSpec {
        id: "continue",
        name: "Continue",
        config_dir: &[".continue"],
    },
    PlannedSidecarSpec {
        id: "goose",
        name: "Goose",
        config_dir: &[".config", "goose"],
    },
    PlannedSidecarSpec {
        id: "qwen_code",
        name: "Qwen Code",
        config_dir: &[".qwen"],
    },
    PlannedSidecarSpec {
        id: "amazon_q",
        name: "Amazon Q Developer CLI",
        config_dir: &[".aws", "amazonq"],
    },
    PlannedSidecarSpec {
        id: "windsurf",
        name: "Windsurf",
        config_dir: &["Library", "Application Support", "Windsurf"],
    },
    PlannedSidecarSpec {
        id: "zed_ai",
        name: "Zed AI",
        config_dir: &[".config", "zed"],
    },
];

pub(crate) fn planned_sidecar_spec(client_id: &str) -> Option<&'static PlannedSidecarSpec> {
    let client_id = normalized_connector_id(client_id);
    PLANNED_SIDECAR_SPECS
        .iter()
        .find(|spec| spec.id == client_id)
}

pub(crate) fn connector_manifests() -> Vec<ConnectorManifest> {
    serde_json::from_str(CONNECTOR_MANIFEST_JSON).unwrap_or_default()
}

pub(crate) fn connector_manifest(client_id: &str) -> Option<ConnectorManifest> {
    connector_manifests()
        .into_iter()
        .find(|manifest| manifest.id == client_id)
}

pub(crate) fn manifest_support_status(
    manifest: Option<&ConnectorManifest>,
) -> ClientConnectorSupportStatus {
    match manifest.map(|item| item.support_status.as_str()) {
        Some("managed") => ClientConnectorSupportStatus::Managed,
        _ => ClientConnectorSupportStatus::Planned,
    }
}

pub(crate) fn manifest_detection_sources(manifest: &ConnectorManifest) -> Vec<String> {
    manifest
        .detection
        .binaries
        .iter()
        .map(|binary| format!("PATH: {binary}"))
        .chain(manifest.detection.paths.iter().cloned())
        .collect()
}

pub(crate) fn manifest_config_locations(manifest: Option<&ConnectorManifest>) -> Vec<String> {
    manifest
        .and_then(|item| item.config.as_ref())
        .map(|config| config.locations.clone())
        .unwrap_or_default()
}

pub(crate) fn manifest_forbidden_reads(manifest: Option<&ConnectorManifest>) -> Vec<String> {
    manifest
        .and_then(|item| item.config.as_ref())
        .map(|config| config.forbidden_reads.clone())
        .unwrap_or_default()
}

fn normalized_connector_id(client_id: &str) -> &str {
    match client_id {
        "codex" | "codex_gui" => "codex_cli",
        "vscode" => "claude_code",
        other => other,
    }
}
