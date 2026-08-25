use crate::client_connectors::{
    planned_config_creation_step_details, planned_sidecar_spec, PlannedClientSpec,
};
use crate::client_paths::planned_sidecar_routing_path;
use crate::models::{ClientConnectorAutomationStage, ClientConnectorConfigDryRunPreview};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONNECTOR_LIFECYCLE_FIXTURES_JSON: &str =
    include_str!("../../connectors/lifecycle-fixtures.json");
const CONNECTOR_LIFECYCLE_FIXTURE_VERSION: u32 = 1;
const CONNECTOR_LIFECYCLE_REQUIRED_STAGES: [&str; 7] = [
    "detect", "preview", "backup", "apply", "verify", "rollback", "off",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectorLifecycleFixtureCatalog {
    version: u32,
    required_stages: Vec<String>,
    connectors: Vec<ConnectorLifecycleFixture>,
}

#[derive(Debug, Deserialize)]
struct ConnectorLifecycleFixture {
    id: String,
    stages: BTreeMap<String, Option<String>>,
}

fn parse_connector_lifecycle_fixture_catalog(
    fixture_json: &str,
) -> Option<ConnectorLifecycleFixtureCatalog> {
    let catalog = serde_json::from_str::<ConnectorLifecycleFixtureCatalog>(fixture_json).ok()?;
    if catalog.version != CONNECTOR_LIFECYCLE_FIXTURE_VERSION
        || catalog.required_stages.len() != CONNECTOR_LIFECYCLE_REQUIRED_STAGES.len()
        || !catalog
            .required_stages
            .iter()
            .zip(CONNECTOR_LIFECYCLE_REQUIRED_STAGES)
            .all(|(actual, expected)| actual == expected)
    {
        return None;
    }

    let mut connector_ids = BTreeSet::new();
    for fixture in &catalog.connectors {
        if fixture.id.trim().is_empty()
            || crate::client_connectors::connector_manifest(&fixture.id).is_none()
            || !connector_ids.insert(fixture.id.as_str())
        {
            return None;
        }
        if fixture.stages.len() != CONNECTOR_LIFECYCLE_REQUIRED_STAGES.len()
            || CONNECTOR_LIFECYCLE_REQUIRED_STAGES
                .iter()
                .any(|stage| !fixture.stages.contains_key(*stage))
        {
            return None;
        }
    }

    Some(catalog)
}

fn connector_has_complete_lifecycle_fixture_in(fixture_json: &str, client_id: &str) -> bool {
    let Some(catalog) = parse_connector_lifecycle_fixture_catalog(fixture_json) else {
        return false;
    };
    let normalized = if client_id == "codex_cli" {
        "codex"
    } else {
        client_id
    };
    catalog
        .connectors
        .iter()
        .find(|fixture| fixture.id == normalized)
        .is_some_and(|fixture| {
            catalog.required_stages.iter().all(|stage| {
                fixture
                    .stages
                    .get(stage)
                    .and_then(Option::as_deref)
                    .is_some_and(|proof| !proof.trim().is_empty())
            })
        })
}

pub(crate) fn connector_has_complete_lifecycle_fixture(client_id: &str) -> bool {
    connector_has_complete_lifecycle_fixture_in(CONNECTOR_LIFECYCLE_FIXTURES_JSON, client_id)
}

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
    use serde_json::{json, Value};

    fn complete_test_catalog() -> Value {
        json!({
            "version": CONNECTOR_LIFECYCLE_FIXTURE_VERSION,
            "requiredStages": CONNECTOR_LIFECYCLE_REQUIRED_STAGES,
            "connectors": [{
                "id": "claude_code",
                "stages": {
                    "detect": "detect_proof",
                    "preview": "preview_proof",
                    "backup": "backup_proof",
                    "apply": "apply_proof",
                    "verify": "verify_proof",
                    "rollback": "rollback_proof",
                    "off": "off_proof"
                }
            }]
        })
    }

    fn catalog_string(catalog: &Value) -> String {
        serde_json::to_string(catalog).expect("catalog JSON")
    }

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
    fn managed_connector_labels_require_complete_lifecycle_fixture_proof() {
        let catalog = parse_connector_lifecycle_fixture_catalog(CONNECTOR_LIFECYCLE_FIXTURES_JSON)
            .expect("valid fixture catalog");
        let adapter_tests = include_str!("client_adapters_tests.rs");

        for fixture in &catalog.connectors {
            for proof in fixture.stages.values().flatten() {
                assert!(
                    adapter_tests.contains(&format!("fn {proof}(")),
                    "{} references missing lifecycle test {proof}",
                    fixture.id
                );
            }
        }
        for spec in MANAGED_CLIENT_SPECS {
            assert!(connector_has_complete_lifecycle_fixture(spec.id));
        }
        for spec in PLANNED_CLIENT_SPECS {
            let manifest =
                crate::client_connectors::connector_manifest(spec.id).expect("connector manifest");
            assert_eq!(
                manifest.support_status == "managed",
                connector_has_complete_lifecycle_fixture(spec.id),
                "{} manifest status must match complete lifecycle fixture proof",
                spec.id
            );
        }
    }

    #[test]
    fn lifecycle_fixture_catalog_requires_exact_supported_version() {
        let canonical = complete_test_catalog();
        assert!(connector_has_complete_lifecycle_fixture_in(
            &catalog_string(&canonical),
            "claude_code"
        ));

        let mut missing = canonical.clone();
        missing
            .as_object_mut()
            .expect("catalog object")
            .remove("version");
        assert!(!connector_has_complete_lifecycle_fixture_in(
            &catalog_string(&missing),
            "claude_code"
        ));

        for version in [json!(0), json!(2), json!("1")] {
            let mut invalid = canonical.clone();
            invalid["version"] = version;
            assert!(!connector_has_complete_lifecycle_fixture_in(
                &catalog_string(&invalid),
                "claude_code"
            ));
        }
    }

    #[test]
    fn lifecycle_fixture_catalog_rejects_noncanonical_required_stages() {
        let canonical = complete_test_catalog();
        let invalid_stage_lists = [
            json!([]),
            json!(["off", "rollback", "verify", "apply", "backup", "preview", "detect"]),
            json!(["detect", "preview", "backup", "apply", "verify", "rollback", "unknown"]),
            json!(["detect", "detect", "backup", "apply", "verify", "rollback", "off"]),
        ];

        for required_stages in invalid_stage_lists {
            let mut invalid = canonical.clone();
            invalid["requiredStages"] = required_stages;
            assert!(!connector_has_complete_lifecycle_fixture_in(
                &catalog_string(&invalid),
                "claude_code"
            ));
        }
    }

    #[test]
    fn lifecycle_fixture_catalog_rejects_duplicate_or_invalid_connector_ids() {
        let canonical = complete_test_catalog();
        let original = canonical["connectors"][0].clone();

        let mut duplicate = canonical.clone();
        duplicate["connectors"] = json!([original.clone(), original.clone()]);
        assert!(!connector_has_complete_lifecycle_fixture_in(
            &catalog_string(&duplicate),
            "claude_code"
        ));

        let mut empty = canonical.clone();
        empty["connectors"][0]["id"] = json!("   ");
        assert!(!connector_has_complete_lifecycle_fixture_in(
            &catalog_string(&empty),
            "claude_code"
        ));

        let mut missing = canonical.clone();
        missing["connectors"][0]
            .as_object_mut()
            .expect("connector object")
            .remove("id");
        assert!(!connector_has_complete_lifecycle_fixture_in(
            &catalog_string(&missing),
            "claude_code"
        ));

        let mut unknown = canonical.clone();
        unknown["connectors"][0]["id"] = json!("unknown");
        assert!(!connector_has_complete_lifecycle_fixture_in(
            &catalog_string(&unknown),
            "claude_code"
        ));
    }

    #[test]
    fn lifecycle_fixture_catalog_rejects_missing_or_unknown_stage_keys() {
        let canonical = complete_test_catalog();

        let mut missing = canonical.clone();
        missing["connectors"][0]["stages"]
            .as_object_mut()
            .expect("stage object")
            .remove("off");
        assert!(!connector_has_complete_lifecycle_fixture_in(
            &catalog_string(&missing),
            "claude_code"
        ));

        let mut unknown = canonical.clone();
        unknown["connectors"][0]["stages"]
            .as_object_mut()
            .expect("stage object")
            .insert("unknown".to_string(), json!("unknown_proof"));
        assert!(!connector_has_complete_lifecycle_fixture_in(
            &catalog_string(&unknown),
            "claude_code"
        ));
    }

    #[test]
    fn lifecycle_fixture_partial_null_evidence_remains_incomplete() {
        let mut partial = complete_test_catalog();
        partial["connectors"][0]["stages"]["off"] = Value::Null;

        let partial_json = catalog_string(&partial);
        assert!(parse_connector_lifecycle_fixture_catalog(&partial_json).is_some());
        assert!(!connector_has_complete_lifecycle_fixture_in(
            &partial_json,
            "claude_code"
        ));
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
