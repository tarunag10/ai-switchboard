use serde::Deserialize;

use crate::cursor_native::{CURSOR_API_KEYS_DOCS_URL, CURSOR_NATIVE_GATE_REASON};
use crate::models::{
    ClientConnectorConfigCreationStep, ClientConnectorConfigDryRunPreview,
    ClientConnectorSupportStatus,
};

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlannedClientSpec {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) category: &'static str,
    pub(crate) setup_phase: &'static str,
    pub(crate) setup_hint: &'static str,
    pub(crate) detection_sources: &'static [&'static str],
    pub(crate) config_locations: &'static [&'static str],
    pub(crate) automation_gates: &'static [&'static str],
    pub(crate) manual_workflow: &'static [&'static str],
}

pub(crate) const PLANNED_CONFIG_CREATION_STEPS: [&str; 7] = [
    "Detect config surface",
    "Show dry-run diff",
    "Create backup",
    "Apply with consent",
    "Verify in Doctor",
    "Rollback safely",
    "Clean up in Off mode",
];

pub(crate) const PLANNED_CONFIG_CREATION_STEP_IDS: [&str; 7] = [
    "detect",
    "dryRunDiff",
    "backup",
    "apply",
    "verify",
    "rollback",
    "offCleanup",
];

pub(crate) const PLANNED_CLIENT_SPECS: [PlannedClientSpec; 11] = [
    PlannedClientSpec {
        id: "gemini_cli",
        name: "Gemini CLI",
        category: "cli",
        setup_phase: "adapt",
        setup_hint: "Managed shell/base-url routing with sibling rollback backups, Doctor verification, rollback, and Off mode cleanup.",
        detection_sources: &["PATH: gemini", "~/.gemini", "~/.config/gemini"],
        config_locations: &["~/.gemini", "~/.config/gemini"],
        automation_gates: &[
            "Detect Gemini CLI and Gemini provider config surfaces before applying routing.",
            "Write only Switchboard-managed shell/base-url routing and sibling rollback backups.",
            "Verify Doctor repair, model/account compatibility visibility, and Off mode cleanup preserve account state.",
        ],
        manual_workflow: &[
            "Confirm Gemini CLI is installed.",
            "Toggle the connector on from Settings.",
            "Use Doctor repair if managed Gemini routing drifts.",
        ],
    },
    PlannedClientSpec {
        id: "opencode",
        name: "OpenCode",
        category: "cli",
        setup_phase: "adapt",
        setup_hint: "Managed provider routing with backup, Doctor verification, rollback, and Off mode cleanup.",
        detection_sources: &["PATH: opencode", "PATH: open-code", "~/.opencode", "~/.config/opencode"],
        config_locations: &["~/.opencode", "~/.config/opencode"],
        automation_gates: &[
            "Identify active OpenCode provider config path without guessing.",
            "Create timestamped backups before provider edits.",
            "Prove Off mode restores the exact previous provider config.",
        ],
        manual_workflow: &[
            "Confirm OpenCode is installed.",
            "Toggle the connector on from Settings.",
            "Use Doctor repair if managed OpenCode routing drifts.",
        ],
    },
    PlannedClientSpec {
        id: "cursor",
        name: "Cursor",
        category: "editor",
        setup_phase: "guide",
        setup_hint: "Manual guide only. Cursor routing stays opt-in until account-specific settings are safely detected.",
        detection_sources: &["PATH: cursor", "/Applications/Cursor.app", "~/Library/Application Support/Cursor"],
        config_locations: &[
            "~/Library/Application Support/Cursor/User/settings.json",
            "~/Library/Application Support/Cursor/User/globalStorage",
        ],
        automation_gates: &[
            "Detect the active Cursor user profile and settings surface before proposing provider changes.",
            "Show a dry-run diff and keep account-specific model choices visible before routing.",
            "Back up Cursor settings without reading extension-managed secrets or global state databases.",
            "Require exact user consent before any native/provider write is enabled.",
            "Verify Cursor routing through Doctor evidence after a managed write.",
            "Rollback restores the exact profile backup without touching unrelated editor or extension config.",
            "Off mode removes only Switchboard-owned Cursor routing markers.",
        ],
        manual_workflow: &[
            "Open Cursor settings.",
            "Review provider and model settings manually.",
            "Use Repo Intelligence packs as copyable context until editor handoff is stable.",
        ],
    },
    PlannedClientSpec {
        id: "grok_cli",
        name: "Grok / xAI CLI",
        category: "cli",
        setup_phase: "adapt",
        setup_hint: "Managed native endpoint routing in the documented Grok Build config.toml surface with sibling backup, Doctor verification, rollback, and Off mode cleanup; credentials, account, and model config remain manual.",
        detection_sources: &["PATH: grok", "PATH: xai", "~/.grok/config.toml", "~/.config/xai"],
        config_locations: &[
            "~/.grok/config.toml",
            "endpoints.models_base_url",
            "~/.config/xai",
        ],
        automation_gates: &[
            "Detect the documented Grok Build config.toml surface without reading API keys, account state, or model configuration.",
            "Write only endpoints.models_base_url to the Switchboard loopback proxy after exact state-bound confirmation.",
            "Verify Doctor repair, rollback, and Off mode cleanup leave xAI provider, model, credential, and account state untouched.",
            "Keep XAI_API_KEY, auth.json, account state, and model selection outside Switchboard-managed storage.",
        ],
        manual_workflow: &[
            "Confirm whether grok or xai exists locally and review the native config preview.",
            "Keep XAI_API_KEY or Grok login authentication configured manually.",
            "Run a Grok prompt and verify activity appears in Headroom.",
        ],
    },
    PlannedClientSpec {
        id: "aider",
        name: "Aider",
        category: "agent",
        setup_phase: "managed",
        setup_hint: "Managed Switchboard-owned sidecar setup with Doctor verify, rollback, and Off mode cleanup; provider config stays manual.",
        detection_sources: &["PATH: aider", "~/.aider.conf.yml", "~/.config/aider"],
        config_locations: &["~/.aider.conf.yml", "~/.config/aider"],
        automation_gates: &[
            "Detect provider configuration without exposing secrets.",
            "Write only the Switchboard-owned Aider routing-intent sidecar.",
            "Verify Doctor repair, rollback, and Off mode cleanup leave provider config untouched.",
        ],
        manual_workflow: &[
            "Confirm Aider is installed.",
            "Toggle the connector on from Settings to create the Switchboard-owned sidecar.",
            "Keep saved provider config manual until a documented provider file adapter is proven.",
        ],
    },
    PlannedClientSpec {
        id: "continue",
        name: "Continue",
        category: "editor",
        setup_phase: "managed",
        setup_hint: "Managed Switchboard-owned sidecar setup with Doctor verify, rollback, and Off mode cleanup; provider config stays manual.",
        detection_sources: &["~/.continue", "~/Library/Application Support/Continue"],
        config_locations: &["~/.continue", "~/Library/Application Support/Continue"],
        automation_gates: &[
            "Detect active Continue config without reading secrets.",
            "Write only the Switchboard-owned Continue routing-intent sidecar.",
            "Verify Doctor repair, rollback, and Off mode cleanup leave provider config untouched.",
        ],
        manual_workflow: &[
            "Confirm Continue config storage is available.",
            "Toggle the connector on from Settings to create the Switchboard-owned sidecar.",
            "Keep saved provider config manual until a documented provider file adapter is proven.",
        ],
    },
    PlannedClientSpec {
        id: "goose",
        name: "Goose",
        category: "agent",
        setup_phase: "managed",
        setup_hint: "Managed Goose allowlisted provider endpoint fields plus Switchboard-owned routing-intent sidecar and read-only Repo Memory MCP bridge with dry-run, exact confirmation, Doctor verification, rollback, and Off mode cleanup; credentials, account state, and model selection remain manual.",
        detection_sources: &["PATH: goose", "~/.config/goose"],
        config_locations: &[
            "~/Library/Application Support/Block/goose/config.yaml",
            "~/.config/goose/config.yaml",
            "OpenAI/Anthropic endpoint fields",
            "~/Library/Application Support/Headroom/config/repo-memory-mcp.json",
            "~/.config/goose",
        ],
        automation_gates: &[
            "Detect the documented Goose config.yaml provider surface without reading secrets.",
            "Write only allowlisted OpenAI/Anthropic endpoint fields and create a sibling backup before apply.",
            "Verify native endpoint fields, the read-only MCP smoke contract, and sidecar marker before advertising Goose readiness.",
            "Rollback and Off mode clean up only Switchboard-owned endpoint, MCP bridge, and sidecar metadata; credentials, account state, and model selection remain untouched.",
        ],
        manual_workflow: &[
            "Confirm Goose is installed.",
            "Review the config.yaml dry-run, then toggle the connector on to prepare native endpoint routing and the Repo Memory MCP context handoff.",
            "Keep Goose credentials, account state, and model selection configured manually.",
        ],
    },
    PlannedClientSpec {
        id: "qwen_code",
        name: "Qwen Code",
        category: "cli",
        setup_phase: "managed",
        setup_hint: "Managed Switchboard-owned sidecar setup with Doctor verify, rollback, and Off mode cleanup; native provider/account config remains manual.",
        detection_sources: &["PATH: qwen", "PATH: qwen-code", "~/.qwen", "~/.config/qwen"],
        config_locations: &["~/.qwen", "~/.config/qwen"],
        automation_gates: &[
            "Detect a stable Qwen Code CLI surface.",
            "Document provider/account compatibility before routing.",
            "Verify Off mode leaves credentials and account state untouched.",
        ],
        manual_workflow: &[
            "Confirm Qwen Code is installed locally.",
            "Paste Repo Intelligence implementation packs into long sessions.",
            "Use RTK-only mode for noisy shell output until adapter support is built.",
        ],
    },
    PlannedClientSpec {
        id: "amazon_q",
        name: "Amazon Q Developer CLI",
        category: "cli",
        setup_phase: "managed",
        setup_hint: "Managed Switchboard-owned sidecar setup with Doctor verify, rollback, and Off mode cleanup; AWS account and workspace state stay manual.",
        detection_sources: &["PATH: q", "~/.aws/amazonq", "~/.config/amazon-q"],
        config_locations: &["~/.aws/amazonq", "~/.config/amazon-q"],
        automation_gates: &[
            "Detect Amazon Q CLI without reading account credentials, AWS profiles, or SSO cache.",
            "Write only the Switchboard-owned routing-intent sidecar inside the Amazon Q config directory.",
            "Verify Doctor repair, rollback, and Off mode cleanup leave AWS and Amazon Q account state untouched.",
        ],
        manual_workflow: &[
            "Confirm Amazon Q Developer CLI is installed.",
            "Toggle the connector on from Settings to create the Switchboard-owned sidecar.",
            "Keep AWS authentication, provider, and workspace selection manual.",
        ],
    },
    PlannedClientSpec {
        id: "windsurf",
        name: "Windsurf",
        category: "editor",
        setup_phase: "adapt",
        setup_hint: "Managed editor settings routing with backup, Doctor verification, rollback, and Off mode cleanup.",
        detection_sources: &[
            "PATH: windsurf",
            "~/Library/Application Support/Windsurf",
            "/Applications/Windsurf.app",
        ],
        config_locations: &["~/Library/Application Support/Windsurf/User/settings.json"],
        automation_gates: &[
            "Back up Windsurf settings before managed routing edits.",
            "Verify managed Windsurf routing points at Headroom.",
            "Verify Off mode removes only Switchboard-owned managed markers.",
        ],
        manual_workflow: &[
            "Confirm Windsurf is installed.",
            "Toggle the connector on from Settings.",
            "Use Doctor repair if managed Windsurf routing drifts.",
        ],
    },
    PlannedClientSpec {
        id: "zed_ai",
        name: "Zed AI",
        category: "editor",
        setup_phase: "adapt",
        setup_hint: "Managed editor settings routing with backup, Doctor verification, rollback, and Off mode cleanup.",
        detection_sources: &[
            "PATH: zed",
            "~/.config/zed",
            "~/Library/Application Support/Zed",
            "/Applications/Zed.app",
        ],
        config_locations: &["~/.config/zed", "~/Library/Application Support/Zed"],
        automation_gates: &[
            "Detect Zed settings before injecting managed routing.",
            "Preserve unknown settings losslessly.",
            "Verify Off mode removes only Switchboard-owned managed routing.",
        ],
        manual_workflow: &[
            "Confirm Zed is installed.",
            "Toggle the connector on from Settings.",
            "Use Doctor repair if managed Zed routing drifts.",
        ],
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

pub(crate) fn planned_config_creation_step_details(
    spec: &PlannedClientSpec,
    forbidden_reads: &[String],
) -> Vec<ClientConnectorConfigCreationStep> {
    let detect_detail = format!(
        "Read-only probe only: inspect {} and watch {} without creating or modifying config.",
        spec.detection_sources.join(", "),
        spec.config_locations.join(", ")
    );
    let forbidden_boundary = if forbidden_reads.is_empty() {
        String::new()
    } else {
        format!(
            " Forbidden reads excluded from dry-run and backup probes: {}.",
            forbidden_reads.join(", ")
        )
    };
    let dry_run_detail = format!(
        "Preview a copyable dry-run artifact with target path, before/after provider intent, managed marker boundary, rollback preview, and confirmation phrase before any file, profile, or environment edit.{forbidden_boundary}"
    );
    let backup_detail = spec
        .automation_gates
        .iter()
        .find(|gate| gate.to_lowercase().contains("back up"))
        .copied()
        .unwrap_or("Create a timestamped backup before any managed setup.")
        .to_string();
    let apply_detail = format!(
        "Apply stays disabled for {} until the dry-run diff, backup, verify, rollback, and Off cleanup gates all pass.",
        spec.name
    );
    let verify_detail = spec
        .automation_gates
        .iter()
        .find(|gate| {
            let gate = gate.to_lowercase();
            gate.contains("doctor")
                || gate.contains("verify")
                || gate.contains("guardrails")
                || gate.contains("compatibility")
        })
        .copied()
        .unwrap_or("Doctor verification must prove the connector state after setup.")
        .to_string();
    let rollback_detail = spec
        .automation_gates
        .iter()
        .find(|gate| {
            let gate = gate.to_lowercase();
            gate.contains("restore") || gate.contains("off mode") || gate.contains("unchanged")
        })
        .copied()
        .unwrap_or("Rollback must restore previous config without touching unrelated settings.")
        .to_string();
    let off_cleanup_detail = format!(
        "Off cleanup removes only Switchboard-managed routing; manual workflow remains: {}",
        spec.manual_workflow.join(" ")
    );
    let details = [
        detect_detail,
        dry_run_detail,
        backup_detail,
        apply_detail,
        verify_detail,
        rollback_detail,
        off_cleanup_detail,
    ];
    let required_evidence = [
        vec![
            "Read-only binary or app detection result.".to_string(),
            "Detected config, settings, profile, or environment surface documented without writes."
                .to_string(),
        ],
        vec![
            "User-visible dry-run diff artifact showing target, before/after local proxy/provider change, managed marker boundary, rollback preview, and confirmation phrase."
                .to_string(),
            "No files, profiles, credentials, or account state changed by the preview.".to_string(),
        ],
        vec![
            "Timestamped backup path or environment-wrapper restore point.".to_string(),
            "Fixture-home restore test proving unknown fields and unrelated provider entries are preserved."
                .to_string(),
        ],
        vec![
            format!("Explicit user consent captured for {}.", spec.name),
            "Managed marker or wrapper boundary proving only Switchboard-owned routing was applied."
                .to_string(),
        ],
        vec![
            "Doctor check confirming account/model guardrails without storing secrets.".to_string(),
            "Compatibility or caveat message visible before routing is considered supported."
                .to_string(),
        ],
        vec![
            "Fixture-home rollback test restoring the exact backup or removing only managed wrapper state."
                .to_string(),
            "Post-rollback diff proving unrelated user settings are unchanged.".to_string(),
        ],
        vec![
            "Fixture-home Off-mode cleanup showing managed routing removed.".to_string(),
            "Doctor verification that the connector returns to manual or RTK-only mode.".to_string(),
        ],
    ];

    PLANNED_CONFIG_CREATION_STEP_IDS
        .iter()
        .zip(PLANNED_CONFIG_CREATION_STEPS.iter())
        .zip(details)
        .zip(required_evidence)
        .map(
            |(((id, label), detail), required_evidence)| ClientConnectorConfigCreationStep {
                id: (*id).to_string(),
                label: (*label).to_string(),
                detail,
                required_evidence,
            },
        )
        .collect()
}

fn detected_config_surface(
    spec: &PlannedClientSpec,
    detection_evidence: &[String],
) -> Option<String> {
    if detection_evidence
        .iter()
        .any(|item| item == "Not detected on machine yet.")
    {
        return None;
    }

    for item in detection_evidence {
        for label in [
            "config surface:",
            "config folder:",
            "profile settings:",
            "assistant settings:",
            "settings:",
        ] {
            if let Some((_, value)) = item.split_once(label) {
                let value = value.trim();
                if !value.is_empty() && value != "none detected yet." {
                    return Some(value.to_string());
                }
            }
        }
    }

    spec.config_locations
        .first()
        .map(|location| location.to_string())
}

pub(crate) fn planned_connector_dry_run_preview(
    spec: &PlannedClientSpec,
    detection_evidence: &[String],
) -> Option<ClientConnectorConfigDryRunPreview> {
    let target = detected_config_surface(spec, detection_evidence)?;
    let cursor_gate = spec.id == "cursor";

    Some(ClientConnectorConfigDryRunPreview {
        target: target.clone(),
        marker: format!("mac-ai-switchboard:{}", spec.id),
        backup_path: format!("{target}.mac-ai-switchboard.bak"),
        current_state: if cursor_gate {
            "Path-only Cursor settings discovery; settings contents are not read.".to_string()
        } else {
            format!(
                "No Switchboard-managed {} provider routing detected.",
                spec.name
            )
        },
        proposed_state: if cursor_gate {
            format!(
                "Preview only: no files are written. Cursor native provider/model/base-url routing is blocked until a documented file schema exists. See {CURSOR_API_KEYS_DOCS_URL}."
            )
        } else {
            format!(
                "Preview only: no files are written. after explicit consent, add Mac AI Switchboard local provider routing for {}.",
                spec.name
            )
        },
        apply_blocked_reason: if cursor_gate {
            format!("{CURSOR_NATIVE_GATE_REASON} Native writes are disabled; the isolated Switchboard sidecar remains available. See {CURSOR_API_KEYS_DOCS_URL}.")
        } else {
            format!(
                "{} automation is disabled until backup, verify, rollback, and Off cleanup gates pass.",
                spec.name
            )
        },
        rollback_preview: if cursor_gate {
            "No Cursor native write is available to roll back; only the isolated Switchboard-owned sidecar can be removed.".to_string()
        } else {
            format!(
                "Restore the {} config backup or remove only the Switchboard-managed provider block.",
                spec.name
            )
        },
        confirmation_phrase: if cursor_gate {
            "CURSOR NATIVE SCHEMA GATE".to_string()
        } else {
            format!("APPLY {} CONFIG", spec.name.to_uppercase())
        },
        writes: Vec::new(),
    })
}

pub(crate) fn planned_connector_has_implemented_setup(client_id: &str) -> bool {
    matches!(normalized_connector_id(client_id), "goose" | "grok_cli")
        || planned_sidecar_spec(client_id).is_some()
            && matches!(
                connector_manifest(client_id)
                    .as_ref()
                    .map(|manifest| manifest.support_status.as_str()),
                Some("managed")
            )
}

fn normalized_connector_id(client_id: &str) -> &str {
    match client_id {
        "codex" | "codex_gui" => "codex_cli",
        "vscode" => "claude_code",
        other => other,
    }
}
