use anyhow::Result;

use crate::client_adapters::{
    configured_timestamp, is_configured, load_setup_state, normalized_setup_id, verify_client_setup,
};
use crate::client_connector_status::{
    managed_connector_config_locations, planned_connector_automation_path, MANAGED_CLIENT_SPECS,
};
use crate::client_connectors::{
    connector_manifest, manifest_config_locations, manifest_detection_sources,
    manifest_forbidden_reads, manifest_support_status, planned_config_creation_step_details,
    planned_connector_dry_run_preview, planned_connector_has_implemented_setup,
    PLANNED_CLIENT_SPECS, PLANNED_CONFIG_CREATION_STEPS,
};
use crate::models::{ClientConnectorStatus, ClientConnectorSupportStatus, ClientStatus};

pub fn list_client_connectors(
    detected_clients: &[ClientStatus],
) -> Result<Vec<ClientConnectorStatus>> {
    let setup_state = load_setup_state();

    let mut connectors = MANAGED_CLIENT_SPECS
        .iter()
        .map(|spec| {
            let manifest = connector_manifest(spec.id);
            let installed = detected_clients
                .iter()
                .find(|client| client.id == spec.id)
                .map(|client| client.installed)
                .unwrap_or(false);
            // Fall back to the remembered snapshot while restore_client_setups
            // is still re-applying on launch, so the connector doesn't flash
            // "disabled" during the async restore window after a restart.
            let enabled = is_configured(&setup_state, spec.id)
                || setup_state
                    .remembered_clients
                    .contains_key(normalized_setup_id(spec.id));
            let setup_verification = if enabled {
                verify_client_setup(spec.id).ok()
            } else {
                None
            };
            let verified = setup_verification
                .as_ref()
                .map(|result| result.verified)
                .unwrap_or(false);

            ClientConnectorStatus {
                client_id: spec.id.to_string(),
                name: manifest
                    .as_ref()
                    .map(|item| item.name.clone())
                    .unwrap_or_else(|| spec.name.to_string()),
                support_status: manifest_support_status(manifest.as_ref()),
                setup_phase: "managed".to_string(),
                setup_hint: "Automatic reversible setup, verification, repair, and off-mode cleanup are supported.".to_string(),
                category: manifest
                    .as_ref()
                    .map(|item| item.category.clone())
                    .unwrap_or_else(|| "managed".to_string()),
                detection_sources: manifest
                    .as_ref()
                    .map(manifest_detection_sources)
                    .unwrap_or_else(|| vec!["App state and local config".to_string()]),
                detection_evidence: detected_clients
                    .iter()
                    .find(|client| client.id == spec.id)
                    .map(|client| client.notes.clone())
                    .unwrap_or_default(),
                config_locations: {
                    let manifest_locations = manifest_config_locations(manifest.as_ref());
                    if manifest_locations.is_empty() {
                        managed_connector_config_locations(spec.id)
                    } else {
                        manifest_locations
                    }
                },
                automation_gates: manifest
                    .as_ref()
                    .map(|item| item.automation_gates.clone())
                    .unwrap_or_else(|| {
                        vec![
                            "Timestamped backups are created before managed config edits."
                                .to_string(),
                            "Verification confirms the connector routes through Headroom."
                                .to_string(),
                            "Off mode removes managed routing blocks and preserves user config."
                                .to_string(),
                        ]
                    }),
                manual_workflow: manifest
                    .as_ref()
                    .map(|item| item.manual_workflow.clone())
                    .unwrap_or_else(|| {
                        vec![
                            "Toggle the connector on from Settings.".to_string(),
                            "Use Doctor repair if verification reports a drifted config."
                                .to_string(),
                            "Switch to Off mode to remove managed routing.".to_string(),
                        ]
                    }),
                config_creation_steps: Vec::new(),
                config_creation_step_details: Vec::new(),
                config_dry_run_preview: None,
                automation_path: Vec::new(),
                installed,
                enabled,
                verified,
                setup_verification,
                last_configured_at: configured_timestamp(&setup_state, spec.id),
            }
        })
        .collect::<Vec<_>>();

    connectors.extend(PLANNED_CLIENT_SPECS.iter().map(|spec| {
        let manifest = connector_manifest(spec.id);
        let detected_client = detected_clients.iter().find(|client| client.id == spec.id);
        let installed = detected_client
            .map(|client| client.installed)
            .unwrap_or(false);
        let detection_evidence = detected_client
            .map(|client| client.notes.clone())
            .unwrap_or_else(|| vec!["Not checked yet.".to_string()]);
        let config_dry_run_preview = planned_connector_dry_run_preview(spec, &detection_evidence);
        let has_implemented_setup = planned_connector_has_implemented_setup(spec.id);
        let enabled = has_implemented_setup && is_configured(&setup_state, spec.id);
        let setup_verification = if enabled {
            verify_client_setup(spec.id).ok()
        } else {
            None
        };
        let verified = setup_verification
            .as_ref()
            .map(|result| result.verified)
            .unwrap_or(false);
        let automation_path = planned_connector_automation_path(
            spec,
            installed,
            config_dry_run_preview.as_ref(),
            enabled,
            verified,
        );
        let support_status = if has_implemented_setup {
            ClientConnectorSupportStatus::Managed
        } else {
            manifest_support_status(manifest.as_ref())
        };
        let setup_phase = if has_implemented_setup {
            "managed"
        } else {
            spec.setup_phase
        };
        let setup_hint = if has_implemented_setup {
            "Automatic reversible setup, verification, repair, restore, and off-mode cleanup are supported."
        } else {
            spec.setup_hint
        };
        let automation_gates = if has_implemented_setup {
            manifest
                .as_ref()
                .map(|item| item.automation_gates.clone())
                .unwrap_or_else(|| {
                    vec![
                        "Timestamped backups are created before managed config edits.".to_string(),
                        "Verification confirms managed routing config points to Headroom."
                            .to_string(),
                        "Off mode removes only Switchboard-managed routing and preserves user config."
                            .to_string(),
                    ]
                })
        } else {
            manifest
                .as_ref()
                .map(|item| item.automation_gates.clone())
                .unwrap_or_else(|| {
                    spec.automation_gates
                        .iter()
                        .map(|gate| gate.to_string())
                        .collect()
                })
        };
        let manual_workflow = if has_implemented_setup {
            manifest
                .as_ref()
                .map(|item| item.manual_workflow.clone())
                .unwrap_or_else(|| {
                    vec![
                        "Toggle the connector on from Settings.".to_string(),
                        "Use Doctor repair if verification reports a drifted config."
                            .to_string(),
                        "Switch to Off mode to remove managed routing.".to_string(),
                    ]
                })
        } else {
            manifest
                .as_ref()
                .map(|item| item.manual_workflow.clone())
                .unwrap_or_else(|| {
                    spec.manual_workflow
                        .iter()
                        .map(|step| step.to_string())
                        .collect()
                })
        };
        let config_creation_steps = if has_implemented_setup {
            Vec::new()
        } else {
            PLANNED_CONFIG_CREATION_STEPS
                .iter()
                .map(|step| step.to_string())
                .collect()
        };
        let forbidden_reads = manifest_forbidden_reads(manifest.as_ref());
        let config_creation_step_details = if has_implemented_setup {
            Vec::new()
        } else {
            planned_config_creation_step_details(spec, &forbidden_reads)
        };
        let config_dry_run_preview = if has_implemented_setup {
            None
        } else {
            config_dry_run_preview
        };

        ClientConnectorStatus {
            client_id: spec.id.to_string(),
            name: manifest
                .as_ref()
                .map(|item| item.name.clone())
                .unwrap_or_else(|| spec.name.to_string()),
            support_status,
            setup_phase: setup_phase.to_string(),
            setup_hint: setup_hint.to_string(),
            category: manifest
                .as_ref()
                .map(|item| item.category.clone())
                .unwrap_or_else(|| spec.category.to_string()),
            detection_sources: manifest
                .as_ref()
                .map(manifest_detection_sources)
                .unwrap_or_else(|| {
                    spec.detection_sources
                        .iter()
                        .map(|source| source.to_string())
                        .collect()
                }),
            detection_evidence,
            config_locations: {
                let manifest_locations = manifest_config_locations(manifest.as_ref());
                if manifest_locations.is_empty() {
                    spec.config_locations
                        .iter()
                        .map(|location| location.to_string())
                        .collect()
                } else {
                    manifest_locations
                }
            },
            automation_gates,
            manual_workflow,
            config_creation_steps,
            config_creation_step_details,
            config_dry_run_preview,
            automation_path: if has_implemented_setup {
                Vec::new()
            } else {
                automation_path
            },
            installed,
            enabled,
            verified,
            setup_verification,
            last_configured_at: configured_timestamp(&setup_state, spec.id),
        }
    }));

    Ok(connectors)
}
