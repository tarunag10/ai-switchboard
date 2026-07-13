use crate::models::{ClientConnectorStatus, DoctorIssue, DoctorSeverity, SwitchboardMode};

pub(crate) fn codex_routing_doctor_issue(
    connectors: &[ClientConnectorStatus],
    desired_mode: &SwitchboardMode,
    provider_block_matches: bool,
) -> Option<DoctorIssue> {
    if !matches!(
        desired_mode,
        SwitchboardMode::Full | SwitchboardMode::Headroom
    ) {
        return None;
    }

    let codex = connectors
        .iter()
        .find(|client| client.client_id == "codex")?;
    if !codex.installed {
        return None;
    }
    if codex.verified && provider_block_matches {
        return None;
    }

    let body = if codex.enabled {
        "Codex routing is repair ready: its model provider, shell export, or proxy URL no longer matches the managed Headroom setup. This can cause direct routing, empty model-provider errors, or unsupported-model errors. Repair will re-apply the reversible Codex setup and verify the managed provider evidence."
    } else {
        "Codex routing is repair ready: Codex is detected on this Mac, but Switchboard has not applied its reversible managed setup. Repair will add the reversible OPENAI_BASE_URL shell export and Headroom-managed provider block, then verify the setup."
    };

    Some(DoctorIssue {
        id: "codex_provider_mismatch".to_string(),
        title: "Codex routing config needs repair".to_string(),
        body: body.to_string(),
        severity: DoctorSeverity::Warning,
        repair_action: Some("repair_codex_setup".to_string()),
    })
}

pub(crate) fn unrouted_managed_connector_issues(
    connectors: &[ClientConnectorStatus],
    desired_mode: &SwitchboardMode,
) -> Vec<DoctorIssue> {
    if !matches!(
        desired_mode,
        SwitchboardMode::Full | SwitchboardMode::Headroom
    ) {
        return Vec::new();
    }

    connectors
        .iter()
        .filter(|client| {
            client.client_id != "codex"
                && client.installed
                && !client.enabled
                && matches!(
                    client.support_status,
                    crate::models::ClientConnectorSupportStatus::Managed
                )
        })
        .map(|client| DoctorIssue {
            id: format!("{}_routing_not_configured", client.client_id),
            title: format!("{} routing is repair ready", client.name),
            body: format!(
                "{} is installed on this Mac, but Switchboard has not applied its reversible managed routing setup. Repair will re-apply this managed client setup, preserve user-owned config outside Switchboard markers, and verify routing evidence.",
                client.name
            ),
            severity: DoctorSeverity::Warning,
            repair_action: Some(format!("repair_client_setup:{}", client.client_id)),
        })
        .collect()
}

pub(crate) fn unverified_managed_connector_issues(
    connectors: &[ClientConnectorStatus],
) -> Vec<DoctorIssue> {
    connectors
        .iter()
        .filter(|client| {
            client.enabled
                && !client.verified
                && client.client_id != "codex"
                && matches!(
                    client.support_status,
                    crate::models::ClientConnectorSupportStatus::Managed
                )
        })
        .map(|connector| DoctorIssue {
            id: format!("{}_routing_config_mismatch", connector.client_id),
            title: format!("{} routing config needs repair", connector.name),
            body: format!(
                "{} is marked as connected, but its managed routing config no longer verifies. Repair will re-apply the reversible managed setup and preserve user-owned config outside Switchboard markers.",
                connector.name
            ),
            severity: DoctorSeverity::Warning,
            repair_action: Some(format!("repair_client_setup:{}", connector.client_id)),
        })
        .collect()
}

pub(crate) fn planned_connector_doctor_body(connectors: &[ClientConnectorStatus]) -> String {
    let names = connectors
        .iter()
        .map(|client| client.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let sources = connectors
        .iter()
        .flat_map(|client| client.detection_sources.iter().map(String::as_str))
        .take(4)
        .collect::<Vec<_>>();
    let locations = connectors
        .iter()
        .flat_map(|client| client.config_locations.iter().map(String::as_str))
        .take(3)
        .collect::<Vec<_>>();
    let evidence = connectors
        .iter()
        .flat_map(|client| client.detection_evidence.iter().map(String::as_str))
        .take(3)
        .collect::<Vec<_>>();
    let gates = connectors
        .iter()
        .flat_map(|client| client.automation_gates.iter().map(String::as_str))
        .take(3)
        .collect::<Vec<_>>();
    let manual_workflow = connectors
        .iter()
        .flat_map(|client| client.manual_workflow.iter().map(String::as_str))
        .take(3)
        .collect::<Vec<_>>();
    let config_steps = connectors
        .iter()
        .flat_map(|client| {
            if client.config_creation_step_details.is_empty() {
                client.config_creation_steps.clone()
            } else {
                client
                    .config_creation_step_details
                    .iter()
                    .map(|step| step.label.clone())
                    .collect()
            }
        })
        .take(7)
        .collect::<Vec<_>>();
    let mut parts = vec![format!(
        "{names} detected, but automatic provider routing is not enabled for these tools yet. AI Switchboard can identify them and show setup evidence, while provider/model settings remain manual until backup, verify, rollback, and Off mode cleanup coverage is promoted."
    )];

    if !sources.is_empty() {
        parts.push(format!("Backend checks: {}.", sources.join(", ")));
    }

    if !locations.is_empty() {
        parts.push(format!(
            "Config locations watched: {}.",
            locations.join(", ")
        ));
    }
    if !evidence.is_empty() {
        parts.push(format!("Detection evidence: {}.", evidence.join(" | ")));
    }
    if !manual_workflow.is_empty() {
        parts.push(format!("Manual workflow: {}.", manual_workflow.join(" | ")));
    }
    if !gates.is_empty() || !config_steps.is_empty() {
        parts.push(
            "Why setup is gated: Switchboard must first prove exact backups, consented apply, Doctor verification, safe rollback, and Off mode cleanup for these native settings."
                .to_string(),
        );
    }
    parts.push(
        "Safe today: use RTK-only mode or Repo Intelligence packs; review provider and model settings manually."
            .to_string(),
    );

    parts.join(" ")
}
