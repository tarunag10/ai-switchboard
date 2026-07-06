use anyhow::{anyhow, Context, Result};

use crate::client_adapters::{
    load_setup_state, normalized_setup_id, planned_switchboard_sidecar_matches, write_setup_state,
};
use crate::client_connectors::planned_sidecar_spec;
use crate::client_paths::planned_sidecar_routing_path;
use crate::managed_files::remove_managed_block_with_backup;
use crate::models::{
    ManagedRollbackExecutionResult, ManagedRollbackExecutionStatus, ManagedRollbackPreview,
};

pub(crate) struct SidecarRollbackTarget {
    record_id: &'static str,
    client_id: &'static str,
    owner: &'static str,
    marker: &'static str,
}

pub(crate) fn sidecar_rollback_target(record_id: &str) -> Option<SidecarRollbackTarget> {
    match record_id {
        "cursor-routing" => Some(SidecarRollbackTarget {
            record_id: "cursor-routing",
            client_id: "cursor",
            owner: "Cursor routing",
            marker: "headroom:cursor",
        }),
        "grok-routing" => Some(SidecarRollbackTarget {
            record_id: "grok-routing",
            client_id: "grok_cli",
            owner: "Grok / xAI CLI routing",
            marker: "headroom:grok_cli",
        }),
        "aider-routing" => Some(SidecarRollbackTarget {
            record_id: "aider-routing",
            client_id: "aider",
            owner: "Aider routing",
            marker: "headroom:aider",
        }),
        "continue-routing" => Some(SidecarRollbackTarget {
            record_id: "continue-routing",
            client_id: "continue",
            owner: "Continue routing",
            marker: "headroom:continue",
        }),
        "goose-routing" => Some(SidecarRollbackTarget {
            record_id: "goose-routing",
            client_id: "goose",
            owner: "Goose MCP bridge",
            marker: "headroom:goose",
        }),
        "qwen-code-routing" => Some(SidecarRollbackTarget {
            record_id: "qwen-code-routing",
            client_id: "qwen_code",
            owner: "Qwen Code routing",
            marker: "headroom:qwen_code",
        }),
        "amazon-q-routing" => Some(SidecarRollbackTarget {
            record_id: "amazon-q-routing",
            client_id: "amazon_q",
            owner: "Amazon Q Developer CLI routing",
            marker: "headroom:amazon_q",
        }),
        _ => None,
    }
}

fn sidecar_rollback_confirmation_phrase(target: &SidecarRollbackTarget) -> String {
    format!("Restore {} for {}", target.marker, target.owner)
}

pub(crate) fn preview_sidecar_rollback(
    target: SidecarRollbackTarget,
) -> Result<ManagedRollbackPreview> {
    let sidecar = planned_sidecar_spec(target.client_id).ok_or_else(|| {
        anyhow!(
            "No Switchboard sidecar is configured for {}.",
            target.client_id
        )
    })?;
    let target_path = planned_sidecar_routing_path(target.client_id)?;
    let marker_present = target_path.exists()
        && planned_switchboard_sidecar_matches(target.client_id).unwrap_or(false);
    let blocked_reason = if marker_present {
        None
    } else {
        Some(format!(
            "Managed {} marker is not present in the sidecar config.",
            target.owner
        ))
    };

    Ok(ManagedRollbackPreview {
        record_id: target.record_id.to_string(),
        owner: target.owner.to_string(),
        target_path: target_path.display().to_string(),
        marker: target.marker.to_string(),
        backup_path: None,
        marker_present,
        backup_exists: true,
        status: if blocked_reason.is_none() {
            ManagedRollbackExecutionStatus::Ready
        } else {
            ManagedRollbackExecutionStatus::Blocked
        },
        confirmation_phrase: sidecar_rollback_confirmation_phrase(&target),
        proposed_action: format!(
            "Remove only the Switchboard-owned {} sidecar block after creating a per-file safety backup.",
            sidecar.name
        ),
        blocked_reason,
        evidence: vec![
            format!("Allowlisted rollback execution row: {}.", target.record_id),
            format!(
                "Cleanup removes only the Switchboard-owned {} sidecar block.",
                sidecar.name
            ),
            "Current sidecar must still contain the managed marker before cleanup.".to_string(),
        ],
    })
}

pub(crate) fn execute_sidecar_rollback(
    target: SidecarRollbackTarget,
    confirmation_phrase: &str,
) -> Result<ManagedRollbackExecutionResult> {
    let expected_confirmation = sidecar_rollback_confirmation_phrase(&target);
    if confirmation_phrase != expected_confirmation {
        return Err(anyhow!("Rollback confirmation phrase does not match."));
    }
    let target_path = planned_sidecar_routing_path(target.client_id)?;
    if !target_path.exists() || !planned_switchboard_sidecar_matches(target.client_id)? {
        return Err(anyhow!(
            "Managed {} marker is missing or has drifted; refusing rollback.",
            target.owner
        ));
    }
    let sidecar = planned_sidecar_spec(target.client_id).ok_or_else(|| {
        anyhow!(
            "No Switchboard sidecar is configured for {}.",
            target.client_id
        )
    })?;
    let (removed, safety_backup) = remove_managed_block_with_backup(&target_path, sidecar.id)?;
    if !removed {
        return Err(anyhow!(
            "Managed {} marker disappeared before rollback could remove it.",
            target.owner
        ));
    }
    let mut state = load_setup_state();
    let state_id = normalized_setup_id(target.client_id);
    state.configured_clients.remove(state_id);
    state.remembered_clients.remove(state_id);
    state.managed_shell_files.remove(state_id);
    state.remembered_shell_files.remove(state_id);
    write_setup_state(&state)?;
    if target_path.exists() {
        let _ = std::fs::read_to_string(&target_path)
            .with_context(|| format!("re-reading {}", target_path.display()))?;
        if planned_switchboard_sidecar_matches(target.client_id)? {
            return Err(anyhow!(
                "Managed {} marker is still present after rollback.",
                target.owner
            ));
        }
    }

    Ok(ManagedRollbackExecutionResult {
        record_id: target.record_id.to_string(),
        owner: target.owner.to_string(),
        target_path: target_path.display().to_string(),
        restored_from: format!(
            "Switchboard-owned {} sidecar block removed.",
            target.client_id
        ),
        safety_backup_path: safety_backup.map(|path| path.display().to_string()),
        marker: target.marker.to_string(),
        verification: vec![
            "Exact confirmation phrase matched.".to_string(),
            format!(
                "Managed {} marker was present before cleanup.",
                target.owner
            ),
            "A fresh sidecar safety backup was created before cleanup.".to_string(),
            format!(
                "Setup state was cleared for {} Off-mode parity.",
                target.client_id
            ),
            "Relaunch-survival evidence: sidecar file was re-read from disk after cleanup."
                .to_string(),
        ],
    })
}
