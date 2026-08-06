use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::client_connectors::{planned_sidecar_spec, PlannedSidecarSpec};
use crate::client_paths::planned_sidecar_routing_path;
use crate::client_paths::SWITCHBOARD_ROUTING_FILE;
use crate::client_provider_configs::HEADROOM_OPENAI_BASE_URL;
use crate::client_setup_state::{load_setup_state, normalized_setup_id, write_setup_state};
use crate::managed_files::{managed_block_updated_content, upsert_managed_block};
use crate::models::{ManagedConfigApplyPreview, ManagedConfigApplyResult, ManagedRollbackExecutionStatus};

pub(crate) const CURSOR_MARKER_PREFIX: &str = "headroom:cursor";
pub(crate) const CURSOR_SIDECAR_APPLY_RECORD_ID: &str = "cursor-sidecar-routing";
pub(crate) const CURSOR_SIDECAR_OWNER: &str = "Cursor routing sidecar";
pub(crate) const GOOSE_SIDECAR_APPLY_RECORD_ID: &str = "goose-sidecar-routing";
pub(crate) const GOOSE_SIDECAR_OWNER: &str = "Goose routing-intent sidecar";
pub(crate) const GROK_SIDECAR_APPLY_RECORD_ID: &str = "grok-sidecar-routing";
pub(crate) const GROK_SIDECAR_OWNER: &str = "Grok / xAI CLI routing-intent sidecar";

fn build_planned_switchboard_sidecar_body(spec: &PlannedSidecarSpec) -> String {
    if spec.id == "goose" {
        return format!(
            "Managed by AI Switchboard.\n\
             Purpose: reversible Goose Repo Memory MCP bridge marker alongside allowlisted native endpoint routing.\n\
             Reference proxy base: {HEADROOM_OPENAI_BASE_URL}\n\
             Boundary: native setup writes only documented non-secret OpenAI/Anthropic endpoint fields; account state, secrets, provider credentials, and model selection remain manual.\n\
             Additional Goose provider fields remain gated until their documented schema and reversible lifecycle are proven."
        );
    }

    format!(
        "Managed by AI Switchboard.\n\
         Purpose: reversible {} routing-intent sidecar while active provider config support remains gated.\n\
         Proxy base: {HEADROOM_OPENAI_BASE_URL}\n\
         Boundary: this file does not mutate account state, secrets, or undocumented provider config.\n\
         Next promotion gate: replace this sidecar with a documented {} config edit after dry-run, backup, verify, rollback, and Off cleanup pass.",
        spec.name, spec.name
    )
}

pub(crate) fn configure_planned_switchboard_sidecar(client_id: &str) -> Result<(bool, Option<PathBuf>)> {
    let spec = planned_sidecar_spec(client_id)
        .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
    let path = planned_sidecar_routing_path(client_id)?;
    upsert_managed_block(
        &path,
        spec.id,
        &build_planned_switchboard_sidecar_body(spec),
    )
}

fn cursor_sidecar_confirmation_phrase(current_state: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(current_state.as_bytes());
    let hash = hasher
        .finalize()
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "Apply {CURSOR_MARKER_PREFIX} sidecar to {} after reviewing {hash}",
        planned_sidecar_routing_path("cursor")?.display()
    ))
}

pub(crate) fn preview_cursor_sidecar_apply() -> Result<ManagedConfigApplyPreview> {
    let spec =
        planned_sidecar_spec("cursor").ok_or_else(|| anyhow!("Cursor sidecar is unavailable."))?;
    let path = planned_sidecar_routing_path("cursor")?;
    let current_state = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };
    let proposed_state = managed_block_updated_content(
        &current_state,
        spec.id,
        &build_planned_switchboard_sidecar_body(spec),
    );
    Ok(ManagedConfigApplyPreview {
        record_id: CURSOR_SIDECAR_APPLY_RECORD_ID.to_string(),
        owner: CURSOR_SIDECAR_OWNER.to_string(),
        target_path: path.display().to_string(),
        marker: CURSOR_MARKER_PREFIX.to_string(),
        backup_path: format!("{}.headroom-backup-*", SWITCHBOARD_ROUTING_FILE),
        status: ManagedRollbackExecutionStatus::Ready,
        confirmation_phrase: cursor_sidecar_confirmation_phrase(&current_state)?,
        current_state,
        proposed_state,
        rollback_preview: "Remove only the Switchboard-owned Cursor sidecar block through Rollback Center; Cursor settings, accounts, models, and extension storage remain untouched.".to_string(),
        blocked_reason: None,
        evidence: vec![
            "Cursor provider settings schema is not allowlisted; this preview targets only the Switchboard-owned sidecar.".to_string(),
            "Preview does not read Cursor settings.json, globalStorage, credentials, account state, or model selection.".to_string(),
            "Apply creates a sibling backup when a sidecar already exists, writes only the managed marker block, verifies it, and supports rollback and Off cleanup.".to_string(),
        ],
    })
}

fn sidecar_apply_confirmation_phrase(client_id: &str, current_state: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(current_state.as_bytes());
    let hash = hasher
        .finalize()
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "Apply headroom:{client_id} sidecar to {} after reviewing {hash}",
        planned_sidecar_routing_path(client_id)?.display()
    ))
}

pub(crate) fn preview_provider_sidecar_apply(
    record_id: &str,
    client_id: &str,
    owner: &str,
) -> Result<ManagedConfigApplyPreview> {
    let spec = planned_sidecar_spec(client_id)
        .ok_or_else(|| anyhow!("{owner} sidecar is unavailable."))?;
    let path = planned_sidecar_routing_path(client_id)?;
    let current_state = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };
    let proposed_state = managed_block_updated_content(
        &current_state,
        spec.id,
        &build_planned_switchboard_sidecar_body(spec),
    );
    Ok(ManagedConfigApplyPreview {
        record_id: record_id.to_string(),
        owner: owner.to_string(),
        target_path: path.display().to_string(),
        marker: format!("headroom:{client_id}"),
        backup_path: format!("{}.headroom-backup-*", SWITCHBOARD_ROUTING_FILE),
        status: ManagedRollbackExecutionStatus::Ready,
        confirmation_phrase: sidecar_apply_confirmation_phrase(client_id, &current_state)?,
        current_state,
        proposed_state,
        rollback_preview: format!("Remove only the Switchboard-owned {owner} block through Rollback Center; provider, model, credentials, and account state remain untouched."),
        blocked_reason: None,
        evidence: vec![
            format!("{owner} native provider schema is not allowlisted; this preview targets only the Switchboard-owned sidecar."),
            "Preview does not read credentials, account state, provider configuration, or model selection.".to_string(),
            "Apply is state-bound to this preview, creates a sibling backup when needed, re-reads the managed marker, and supports rollback and Off cleanup.".to_string(),
        ],
    })
}

pub(crate) fn execute_provider_sidecar_apply(
    record_id: &str,
    client_id: &str,
    owner: &str,
    confirmation_phrase: &str,
) -> Result<ManagedConfigApplyResult> {
    let preview = preview_provider_sidecar_apply(record_id, client_id, owner)?;
    if confirmation_phrase != preview.confirmation_phrase {
        return Err(anyhow!(
            "Managed config apply confirmation phrase does not match."
        ));
    }
    let path = planned_sidecar_routing_path(client_id)?;
    let (changed, backup) = configure_planned_switchboard_sidecar(client_id)?;
    if !planned_switchboard_sidecar_matches(client_id)? {
        return Err(anyhow!("{owner} verification failed after apply."));
    }
    let mut state = load_setup_state();
    state.configured_clients.insert(
        normalized_setup_id(client_id).to_string(),
        Utc::now().to_rfc3339(),
    );
    write_setup_state(&state)?;
    Ok(ManagedConfigApplyResult {
        record_id: record_id.to_string(), owner: owner.to_string(), target_path: path.display().to_string(),
        changed, backup_path: backup.map(|path| path.display().to_string()), marker: format!("headroom:{client_id}"),
        verification: vec![
            "Exact state-bound confirmation phrase matched the dry-run preview.".to_string(),
            format!("Only the Switchboard-owned {owner} sidecar was written; provider, model, credentials, and account state were not read or changed."),
            "Managed sidecar marker was re-read from disk after apply; Rollback Center and Off mode remove only the managed block.".to_string(),
        ],
    })
}

pub(crate) fn planned_switchboard_sidecar_matches(client_id: &str) -> Result<bool> {
    let spec = planned_sidecar_spec(client_id)
        .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
    let path = planned_sidecar_routing_path(client_id)?;
    if !path.exists() {
        return Ok(false);
    }

    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let expected_purpose = if spec.id == "goose" {
        "reversible Goose Repo Memory MCP bridge marker".to_string()
    } else {
        format!("reversible {} routing-intent sidecar", spec.name)
    };

    Ok(content.contains(&format!("# >>> headroom:{} >>>", spec.id))
        && content.contains(&format!("# <<< headroom:{} <<<", spec.id))
        && content.contains(HEADROOM_OPENAI_BASE_URL)
        && content.contains(&expected_purpose))
}
