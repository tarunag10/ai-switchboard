use anyhow::Result;

use crate::client_adapter_contract::adapter_status_for_listing;
use crate::client_adapters::{
    configured_timestamp, is_configured, load_setup_state, normalized_setup_id, verify_client_setup,
};
use crate::client_connector_status::{
    connector_has_complete_lifecycle_fixture, managed_connector_config_locations,
    planned_connector_automation_path, MANAGED_CLIENT_SPECS,
};
use crate::client_connectors::{
    connector_manifest, managed_connector_dry_run_preview, manifest_config_locations,
    manifest_detection_sources, manifest_forbidden_reads, manifest_support_status,
    planned_config_creation_step_details, planned_connector_dry_run_preview,
    planned_connector_has_implemented_setup, planned_connector_has_implemented_sidecar_setup,
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
            let lifecycle_managed = connector_has_complete_lifecycle_fixture(spec.id);
            let detected_client = detected_clients
                .iter()
                .find(|client| client.id == spec.id);
            // Fall back to the remembered snapshot while restore_client_setups
            // is still re-applying on launch, so the connector doesn't flash
            // "disabled" during the async restore window after a restart.
            let enabled = is_configured(&setup_state, spec.id)
                || setup_state
                    .remembered_clients
                    .contains_key(normalized_setup_id(spec.id));
            let adapter_contract = detected_client
                .and_then(|detected| {
                    adapter_status_for_listing(spec.id, detected, enabled)
                        .ok()
                        .flatten()
                });
            let installed = adapter_contract
                .as_ref()
                .map(|status| status.detection.installed)
                .or_else(|| detected_client.map(|client| client.installed))
                .unwrap_or(false);
            let setup_verification = adapter_contract
                .as_ref()
                .and_then(|status| status.verification.as_ref())
                .map(|report| crate::models::ClientSetupVerification {
                    client_id: report.client_id.clone(),
                    verified: report.verified,
                    proxy_reachable: report.proxy_reachable,
                    checks: report.checks.clone(),
                    failures: report.failures.clone(),
                })
                .or_else(|| enabled.then(|| verify_client_setup(spec.id).ok()).flatten());
            let verified = adapter_contract
                .as_ref()
                .and_then(|status| status.verification.as_ref())
                .map(|report| report.verified)
                .or_else(|| setup_verification.as_ref().map(|result| result.verified))
                .unwrap_or(false);

            ClientConnectorStatus {
                client_id: spec.id.to_string(),
                name: manifest
                    .as_ref()
                    .map(|item| item.name.clone())
                    .unwrap_or_else(|| spec.name.to_string()),
                support_status: if lifecycle_managed {
                    manifest_support_status(manifest.as_ref())
                } else {
                    ClientConnectorSupportStatus::Planned
                },
                setup_phase: if lifecycle_managed { "managed" } else { "fixture-incomplete" }.to_string(),
                setup_hint: if lifecycle_managed {
                    "Automatic reversible setup, verification, repair, and off-mode cleanup are supported."
                } else {
                    "Lifecycle fixture proof is incomplete; this connector cannot be labelled Managed."
                }.to_string(),
                category: manifest
                    .as_ref()
                    .map(|item| item.category.clone())
                    .unwrap_or_else(|| "managed".to_string()),
                detection_sources: manifest
                    .as_ref()
                    .map(manifest_detection_sources)
                    .unwrap_or_else(|| vec!["App state and local config".to_string()]),
                detection_evidence: adapter_contract
                    .as_ref()
                    .map(|status| status.detection.evidence.clone())
                    .or_else(|| detected_client.map(|client| client.notes.clone()))
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
                adapter_contract,
                last_configured_at: configured_timestamp(&setup_state, spec.id),
            }
        })
        .collect::<Vec<_>>();

    connectors.extend(PLANNED_CLIENT_SPECS.iter().map(|spec| {
        let manifest = connector_manifest(spec.id);
        let detected_client = detected_clients.iter().find(|client| client.id == spec.id);
        let detection_evidence = detected_client
            .map(|client| client.notes.clone())
            .unwrap_or_else(|| vec!["Not checked yet.".to_string()]);
        let config_dry_run_preview = planned_connector_dry_run_preview(spec, &detection_evidence);
        let has_implemented_setup = planned_connector_has_implemented_setup(spec.id);
        let has_implemented_sidecar_setup =
            planned_connector_has_implemented_sidecar_setup(spec.id);
        let lifecycle_managed =
            has_implemented_setup && connector_has_complete_lifecycle_fixture(spec.id);
        let enabled = (has_implemented_setup || has_implemented_sidecar_setup)
            && is_configured(&setup_state, spec.id);
        let adapter_contract = detected_client
            .and_then(|detected| {
                adapter_status_for_listing(spec.id, detected, enabled)
                    .ok()
                    .flatten()
            });
        let installed = adapter_contract
            .as_ref()
            .map(|status| status.detection.installed)
            .or_else(|| detected_client.map(|client| client.installed))
            .unwrap_or(false);
        let setup_verification = adapter_contract
            .as_ref()
            .and_then(|status| status.verification.as_ref())
            .map(|report| crate::models::ClientSetupVerification {
                client_id: report.client_id.clone(),
                verified: report.verified,
                proxy_reachable: report.proxy_reachable,
                checks: report.checks.clone(),
                failures: report.failures.clone(),
            })
            .or_else(|| enabled.then(|| verify_client_setup(spec.id).ok()).flatten());
        let verified = adapter_contract
            .as_ref()
            .and_then(|status| status.verification.as_ref())
            .map(|report| report.verified)
            .or_else(|| setup_verification.as_ref().map(|result| result.verified))
            .unwrap_or(false);
        let automation_path = planned_connector_automation_path(
            spec,
            installed,
            config_dry_run_preview.as_ref(),
            enabled,
            verified,
        );
        let support_status = if lifecycle_managed {
            ClientConnectorSupportStatus::Managed
        } else {
            manifest_support_status(manifest.as_ref())
        };
        let setup_phase = if lifecycle_managed {
            "managed"
        } else {
            spec.setup_phase
        };
        let setup_hint = if lifecycle_managed {
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
            managed_connector_dry_run_preview(spec, &detection_evidence)
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
            adapter_contract,
            last_configured_at: configured_timestamp(&setup_state, spec.id),
        }
    }));

    Ok(connectors)
}
