use crate::client_connectors::{
    planned_config_creation_step_details, planned_sidecar_spec, PlannedClientSpec,
};
use crate::client_paths::planned_sidecar_routing_path;
use crate::models::{ClientConnectorAutomationStage, ClientConnectorConfigDryRunPreview};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ManagedClientSpec {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
}

pub(crate) const MANAGED_CLIENT_SPECS: [ManagedClientSpec; 2] = [
    ManagedClientSpec {
        id: "claude_code",
        name: "Claude Code",
    },
    ManagedClientSpec {
        id: "codex",
        name: "Codex",
    },
];

pub(crate) fn planned_connector_automation_path(
    spec: &PlannedClientSpec,
    installed: bool,
    preview: Option<&ClientConnectorConfigDryRunPreview>,
    enabled: bool,
    verified: bool,
) -> Vec<ClientConnectorAutomationStage> {
    let step_details = planned_config_creation_step_details(spec, &[]);
    let sidecar_spec = planned_sidecar_spec(spec.id);
    step_details
        .into_iter()
        .map(|step| {
            let status = match step.id.as_str() {
                "detect" if installed => "ready",
                "detect" => "blocked",
                "dryRunDiff" if preview.is_some() => "ready",
                "backup" | "apply" | "rollback" | "offCleanup"
                    if sidecar_spec.is_some() && enabled =>
                {
                    "ready"
                }
                "verify" if sidecar_spec.is_some() && verified => "ready",
                _ => "blocked",
            };
            let evidence = match step.id.as_str() {
                "detect" if installed => {
                    format!("{} has local detection evidence; no config writes performed.", spec.name)
                }
                "detect" => {
                    format!("{} is not detected locally yet; install or expose it on PATH first.", spec.name)
                }
                "dryRunDiff" if let Some(preview) = preview => format!(
                    "Blocked preview ready for {} with target {}, marker {}, backup {}, and confirmation phrase {}.",
                    spec.name, preview.target, preview.marker, preview.backup_path, preview.confirmation_phrase
                ),
                "dryRunDiff" => {
                    "Dry-run preview is blocked until a connector config surface is detected.".to_string()
                }
                "backup" if sidecar_spec.is_some() && enabled => format!(
                    "{} sidecar writes use Headroom timestamped backups when {} already exists.",
                    spec.name,
                    planned_sidecar_routing_path(spec.id)
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|_| "the connector sidecar".to_string())
                ),
                "apply" if sidecar_spec.is_some() && enabled => format!(
                    "{} sidecar is present at {} with the Switchboard-managed marker.",
                    spec.name,
                    planned_sidecar_routing_path(spec.id)
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|_| "the connector sidecar".to_string())
                ),
                "verify" if sidecar_spec.is_some() && verified => {
                    format!(
                        "Doctor verified the {} sidecar marker and local proxy endpoint reference.",
                        spec.name
                    )
                }
                "rollback" if sidecar_spec.is_some() && enabled => {
                    format!(
                        "Rollback removes only the Switchboard-managed {} sidecar block.",
                        spec.name
                    )
                }
                "offCleanup" if sidecar_spec.is_some() && enabled => {
                    format!(
                        "Off mode cleanup is wired through disable_client_setup for the {} sidecar.",
                        spec.name
                    )
                }
                _ => step.required_evidence.join(" "),
            };
            ClientConnectorAutomationStage {
                id: step.id,
                label: step.label,
                status: status.to_string(),
                evidence,
            }
        })
        .collect()
}

pub(crate) fn managed_connector_config_locations(client_id: &str) -> Vec<String> {
    match client_id {
        "claude_code" => vec![
            "~/.claude/settings.json".to_string(),
            "~/.claude/settings.local.json".to_string(),
        ],
        "codex" => vec![
            "~/.codex/config.toml".to_string(),
            "~/.codex/AGENTS.md".to_string(),
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_connectors::PLANNED_CLIENT_SPECS;

    fn preview() -> ClientConnectorConfigDryRunPreview {
        ClientConnectorConfigDryRunPreview {
            target: "~/.config/opencode/opencode.json".to_string(),
            marker: "mac-ai-switchboard:opencode".to_string(),
            backup_path: "~/.config/opencode/opencode.json.headroom-backup-*".to_string(),
            current_state: "manual".to_string(),
            proposed_state: "managed".to_string(),
            apply_blocked_reason: "Requires confirmation.".to_string(),
            rollback_preview: "Restore prior provider config.".to_string(),
            confirmation_phrase: "APPLY OPENCODE CONFIG".to_string(),
            writes: vec!["provider.headroom".to_string()],
        }
    }

    #[test]
    fn managed_connector_config_locations_cover_native_managed_clients() {
        assert_eq!(
            managed_connector_config_locations("claude_code"),
            vec!["~/.claude/settings.json", "~/.claude/settings.local.json"]
        );
        assert_eq!(
            managed_connector_config_locations("codex"),
            vec!["~/.codex/config.toml", "~/.codex/AGENTS.md"]
        );
        assert!(managed_connector_config_locations("cursor").is_empty());
    }

    #[test]
    fn planned_connector_automation_path_tracks_ready_and_blocked_stages() {
        let spec = PLANNED_CLIENT_SPECS
            .iter()
            .find(|spec| spec.id == "opencode")
            .expect("opencode planned spec");
        let stages = planned_connector_automation_path(spec, true, Some(&preview()), false, false);

        let detect = stages
            .iter()
            .find(|stage| stage.id == "detect")
            .expect("detect stage");
        let dry_run = stages
            .iter()
            .find(|stage| stage.id == "dryRunDiff")
            .expect("dry-run stage");
        let apply = stages
            .iter()
            .find(|stage| stage.id == "apply")
            .expect("apply stage");

        assert_eq!(detect.status, "ready");
        assert_eq!(dry_run.status, "ready");
        assert_eq!(apply.status, "blocked");
        assert!(dry_run.evidence.contains("APPLY OPENCODE CONFIG"));
    }
}
