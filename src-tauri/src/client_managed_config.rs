use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_json;

use crate::client_adapters::{
    codex_provider_block_matches, configure_planned_switchboard_sidecar, disable_client_setup,
    execute_provider_sidecar_apply, load_setup_state, planned_switchboard_sidecar_matches,
    preview_cursor_sidecar_apply, preview_provider_sidecar_apply,
    resolve_client_shell_targets_for_cleanup, shell_block_contains_text_in_files,
    CURSOR_MARKER_PREFIX, CURSOR_SIDECAR_APPLY_RECORD_ID, CURSOR_SIDECAR_OWNER,
    GEMINI_BASE_URL_ENV_KEY, GOOSE_SIDECAR_APPLY_RECORD_ID, GOOSE_SIDECAR_OWNER,
    GROK_SIDECAR_APPLY_RECORD_ID, GROK_SIDECAR_OWNER,
};
use crate::aider_provider_configs::{
    aider_apply_confirmation_phrase, aider_config_backup_pattern, aider_next_provider_config,
    aider_provider_config_matches, configure_aider_provider_config, AIDER_NATIVE_APPLY_RECORD_ID,
    AIDER_NATIVE_MARKER, AIDER_NATIVE_OWNER,
};
use crate::client_paths::{
    aider_config_path, codex_config_toml_path, continue_config_path, grok_config_path, home_dir,
    opencode_config_path, planned_sidecar_routing_path, windsurf_config_path, zed_config_path,
    SWITCHBOARD_ROUTING_FILE,
};
use crate::continue_provider_configs::{
    configure_continue_provider_config, continue_apply_confirmation_phrase,
    continue_config_backup_pattern, continue_next_provider_config,
    continue_provider_config_matches, CONTINUE_NATIVE_APPLY_RECORD_ID, CONTINUE_NATIVE_MARKER,
    CONTINUE_NATIVE_OWNER,
};
use crate::client_provider_configs::{
    configure_grok_provider_config, configure_opencode_provider_config,
    configure_windsurf_provider_config, configure_zed_provider_config,
    grok_apply_confirmation_phrase, grok_config_backup_pattern, grok_next_provider_config,
    grok_provider_config_matches, opencode_apply_confirmation_phrase,
    opencode_config_backup_pattern, opencode_next_provider_config, opencode_provider_config_matches,
    windsurf_apply_confirmation_phrase, windsurf_config_backup_pattern,
    windsurf_next_provider_config, windsurf_provider_config_matches, zed_apply_confirmation_phrase,
    zed_config_backup_pattern, zed_next_provider_config, zed_provider_config_matches,
    GROK_MARKER_PREFIX,
};
use crate::client_sidecar_rollbacks::{
    execute_sidecar_rollback, preview_sidecar_rollback, sidecar_rollback_target,
};
use crate::goose_provider_configs::{
    configure_goose_provider_config, goose_apply_confirmation_phrase, goose_config_backup_pattern,
    goose_config_path, goose_provider_config_matches, preview_goose_provider_config,
    GOOSE_NATIVE_APPLY_RECORD_ID, GOOSE_NATIVE_MARKER, GOOSE_NATIVE_OWNER,
};
use crate::managed_files::backup_if_exists;
use crate::models::{
    ManagedConfigApplyPreview, ManagedConfigApplyResult, ManagedRollbackExecutionResult,
    ManagedRollbackExecutionStatus, ManagedRollbackPreview, ManagedRollbackUndoAllExecutionResult,
    ManagedRollbackUndoAllPreview,
};

pub(crate) const GROK_ROLLBACK_RECORD_ID: &str = "grok-routing";
const GROK_ROLLBACK_OWNER: &str = "Grok / xAI CLI routing";
const GROK_ROLLBACK_MARKER: &str = "ai-switchboard:grok";

const CODEX_ROLLBACK_RECORD_ID: &str = "codex-routing";
const CODEX_ROLLBACK_OWNER: &str = "Codex routing";
const CODEX_ROLLBACK_MARKER: &str = "ai-switchboard:codex_cli";
const OPENCODE_ROLLBACK_RECORD_ID: &str = "opencode-routing";
const OPENCODE_ROLLBACK_OWNER: &str = "OpenCode routing";
const OPENCODE_ROLLBACK_MARKER: &str = "ai-switchboard:opencode";
const GEMINI_ROLLBACK_RECORD_ID: &str = "gemini-routing";
const GEMINI_ROLLBACK_OWNER: &str = "Gemini CLI routing";
const GEMINI_ROLLBACK_MARKER: &str = "ai-switchboard:gemini_cli";
const ZED_ROLLBACK_RECORD_ID: &str = "zed-ai-routing";
const ZED_ROLLBACK_OWNER: &str = "Zed routing";
const ZED_ROLLBACK_MARKER: &str = "ai-switchboard:zed";
const ZED_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: zed-ai-routing.",
    "Backup must live next to ~/.config/zed/settings.json and use *.headroom-backup-*.",
    "Current config must still contain the managed Zed markers before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
];
const WINDSURF_ROLLBACK_RECORD_ID: &str = "windsurf-routing";
const WINDSURF_ROLLBACK_OWNER: &str = "Windsurf routing";
const WINDSURF_ROLLBACK_MARKER: &str = "ai-switchboard:windsurf";
const WINDSURF_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: windsurf-routing.",
    "Backup must live next to ~/Library/Application Support/Windsurf/User/settings.json and use *.headroom-backup-*.",
    "Current config must still contain the managed Windsurf markers before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
];
const CONTINUE_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: continue-provider-routing.",
    "Backup must live next to ~/.continue/config.yaml and use *.headroom-backup-*.",
    "Current config must still contain the managed Continue Headroom model before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
    "Provider credentials, apiKey values, account state, and unrelated model entries remain untouched.",
];
const AIDER_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: aider-provider-routing.",
    "Backup must live next to ~/.aider.conf.yml and use *.headroom-backup-*.",
    "Current config must still contain the managed openai-api-base field before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
    "API keys, api-key entries, set-env values, and unrelated Aider settings remain untouched.",
];
const MANAGED_ROLLBACK_UNDO_ALL_CONFIRMATION: &str =
    "Undo all ready Switchboard native rollback rows";
const NATIVE_MANAGED_ROLLBACK_RECORD_IDS: &[&str] = &[
    CODEX_ROLLBACK_RECORD_ID,
    GEMINI_ROLLBACK_RECORD_ID,
    OPENCODE_ROLLBACK_RECORD_ID,
    ZED_ROLLBACK_RECORD_ID,
    GOOSE_NATIVE_APPLY_RECORD_ID,
    "cursor-routing",
    "grok-routing",
    "aider-routing",
    CONTINUE_NATIVE_APPLY_RECORD_ID,
    AIDER_NATIVE_APPLY_RECORD_ID,
    "continue-routing",
    "goose-routing",
    "qwen-code-routing",
    "amazon-q-routing",
    "windsurf-routing",
];

struct ManagedRollbackTarget {
    record_id: &'static str,
    owner: &'static str,
    marker: &'static str,
    target_path: fn() -> PathBuf,
    marker_matches: fn() -> Result<bool>,
    backup_required: bool,
    proposed_action: &'static str,
    evidence: &'static [&'static str],
}

const CODEX_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: codex-routing.",
    "Backup must live next to ~/.codex/config.toml and use *.headroom-backup-*.",
    "Current config must still contain the managed Codex marker before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
];

const OPENCODE_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: opencode-routing.",
    "Backup must live next to ~/.config/opencode/opencode.json and use *.headroom-backup-*.",
    "Current config must still contain the managed OpenCode Headroom provider before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
];

const GROK_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: grok-routing.",
    "Backup must live next to ~/.grok/config.toml and use *.headroom-backup-*.",
    "Current config must still contain the managed Grok [endpoints].models_base_url marker before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
    "Switchboard never reads or writes Grok auth.json, API keys, account state, or model selection.",
];

const GEMINI_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: gemini-routing.",
    "Cleanup removes only Switchboard-owned Gemini shell and sidecar blocks.",
    "Current shell profile or sidecar must still contain the managed Gemini marker before cleanup.",
    "Relaunch-survival evidence requires re-reading managed files from disk after cleanup.",
];

const GOOSE_NATIVE_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: goose-provider-routing.",
    "Backup must live next to Goose config.yaml and use *.headroom-backup-*.",
    "Current config must still contain the managed Goose endpoint marker before restore.",
    "Credentials, secrets.yaml, keychain state, account state, and model values remain untouched.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
];

fn gemini_routing_marker_matches() -> Result<bool> {
    let state = load_setup_state();
    let shell_targets = resolve_client_shell_targets_for_cleanup(&state, "gemini_cli")?;
    let shell_matches =
        shell_block_contains_text_in_files(&shell_targets, "gemini_cli", GEMINI_BASE_URL_ENV_KEY)?;
    let sidecar_matches = planned_switchboard_sidecar_matches("gemini_cli").unwrap_or(false);
    Ok(shell_matches || sidecar_matches)
}

fn managed_rollback_target(record_id: &str) -> Result<ManagedRollbackTarget> {
    match record_id {
        CODEX_ROLLBACK_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: CODEX_ROLLBACK_RECORD_ID,
            owner: CODEX_ROLLBACK_OWNER,
            marker: CODEX_ROLLBACK_MARKER,
            target_path: codex_config_toml_path,
            marker_matches: codex_provider_block_matches,
            backup_required: true,
            proposed_action:
                "Restore the Codex config from the selected sibling backup after creating a fresh safety backup.",
            evidence: CODEX_ROLLBACK_EVIDENCE,
        }),
        OPENCODE_ROLLBACK_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: OPENCODE_ROLLBACK_RECORD_ID,
            owner: OPENCODE_ROLLBACK_OWNER,
            marker: OPENCODE_ROLLBACK_MARKER,
            target_path: opencode_config_path,
            marker_matches: opencode_provider_config_matches,
            backup_required: true,
            proposed_action:
                "Restore the OpenCode provider config from the selected sibling backup after creating a fresh safety backup.",
            evidence: OPENCODE_ROLLBACK_EVIDENCE,
        }),
        GROK_ROLLBACK_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: GROK_ROLLBACK_RECORD_ID,
            owner: GROK_ROLLBACK_OWNER,
            marker: GROK_ROLLBACK_MARKER,
            target_path: grok_config_path,
            marker_matches: grok_provider_config_matches,
            backup_required: true,
            proposed_action:
                "Restore the Grok config from the selected sibling backup after creating a fresh safety backup.",
            evidence: GROK_ROLLBACK_EVIDENCE,
        }),
        GEMINI_ROLLBACK_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: GEMINI_ROLLBACK_RECORD_ID,
            owner: GEMINI_ROLLBACK_OWNER,
            marker: GEMINI_ROLLBACK_MARKER,
            target_path: || {
                planned_sidecar_routing_path("gemini_cli")
                    .unwrap_or_else(|_| home_dir().join(".gemini").join(SWITCHBOARD_ROUTING_FILE))
            },
            marker_matches: gemini_routing_marker_matches,
            backup_required: false,
            proposed_action:
                "Remove only the Switchboard-owned Gemini shell routing and sidecar blocks after creating per-file safety backups.",
            evidence: GEMINI_ROLLBACK_EVIDENCE,
        }),
        WINDSURF_ROLLBACK_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: WINDSURF_ROLLBACK_RECORD_ID,
            owner: WINDSURF_ROLLBACK_OWNER,
            marker: WINDSURF_ROLLBACK_MARKER,
            target_path: windsurf_config_path,
            marker_matches: windsurf_provider_config_matches,
            backup_required: true,
            proposed_action:
                "Restore the Windsurf settings from the selected sibling backup after creating a fresh safety backup.",
            evidence: WINDSURF_ROLLBACK_EVIDENCE,
        }),
        ZED_ROLLBACK_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: ZED_ROLLBACK_RECORD_ID,
            owner: ZED_ROLLBACK_OWNER,
            marker: ZED_ROLLBACK_MARKER,
            target_path: zed_config_path,
            marker_matches: zed_provider_config_matches,
            backup_required: true,
            proposed_action:
                "Restore the Zed settings from the selected sibling backup after creating a fresh safety backup.",
            evidence: ZED_ROLLBACK_EVIDENCE,
        }),
        GOOSE_NATIVE_APPLY_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: GOOSE_NATIVE_APPLY_RECORD_ID,
            owner: GOOSE_NATIVE_OWNER,
            marker: GOOSE_NATIVE_MARKER,
            target_path: goose_config_path,
            marker_matches: goose_provider_config_matches,
            backup_required: true,
            proposed_action:
                "Restore the Goose config from the selected sibling backup after creating a fresh safety backup.",
            evidence: GOOSE_NATIVE_ROLLBACK_EVIDENCE,
        }),
        CONTINUE_NATIVE_APPLY_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: CONTINUE_NATIVE_APPLY_RECORD_ID,
            owner: CONTINUE_NATIVE_OWNER,
            marker: CONTINUE_NATIVE_MARKER,
            target_path: continue_config_path,
            marker_matches: continue_provider_config_matches,
            backup_required: true,
            proposed_action:
                "Restore the Continue config from the selected sibling backup after creating a fresh safety backup.",
            evidence: CONTINUE_ROLLBACK_EVIDENCE,
        }),
        AIDER_NATIVE_APPLY_RECORD_ID => Ok(ManagedRollbackTarget {
            record_id: AIDER_NATIVE_APPLY_RECORD_ID,
            owner: AIDER_NATIVE_OWNER,
            marker: AIDER_NATIVE_MARKER,
            target_path: aider_config_path,
            marker_matches: aider_provider_config_matches,
            backup_required: true,
            proposed_action:
                "Restore the Aider config from the selected sibling backup after creating a fresh safety backup.",
            evidence: AIDER_ROLLBACK_EVIDENCE,
        }),
        _ => Err(anyhow!(
            "Managed rollback execution is currently enabled only for {CODEX_ROLLBACK_RECORD_ID}, {OPENCODE_ROLLBACK_RECORD_ID}, {GROK_ROLLBACK_RECORD_ID}, {GOOSE_NATIVE_APPLY_RECORD_ID}, {CONTINUE_NATIVE_APPLY_RECORD_ID}, {AIDER_NATIVE_APPLY_RECORD_ID}, {GEMINI_ROLLBACK_RECORD_ID}, {WINDSURF_ROLLBACK_RECORD_ID}, and {ZED_ROLLBACK_RECORD_ID}."
        )),
    }
}

fn managed_rollback_confirmation_phrase(target: &ManagedRollbackTarget) -> String {
    format!("Restore {} for {}", target.marker, target.owner)
}

fn latest_headroom_backup_for(path: &Path) -> Option<PathBuf> {
    let dir = path.parent()?;
    let file_name = path.file_name()?.to_str()?;
    let prefix = format!("{file_name}.headroom-backup-");
    let mut backups = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|candidate| {
            candidate
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    backups.sort();
    backups.pop()
}

fn validate_managed_rollback_backup_path(target_path: &Path, backup_path: &Path) -> Result<()> {
    let target_dir = target_path
        .parent()
        .ok_or_else(|| anyhow!("Rollback target path has no parent directory."))?;
    let backup_parent = backup_path
        .parent()
        .ok_or_else(|| anyhow!("Rollback backup path has no parent directory."))?;
    if backup_parent != target_dir {
        return Err(anyhow!(
            "Rollback backup must live next to the managed config."
        ));
    }
    let target_file = target_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Rollback target path has no file name."))?;
    let expected_prefix = format!("{target_file}.headroom-backup-");
    let backup_name = backup_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Rollback backup path has no file name."))?;
    if !backup_name.starts_with(&expected_prefix) {
        return Err(anyhow!(
            "Rollback backup must use the Switchboard headroom-backup naming pattern."
        ));
    }
    if !backup_path.exists() {
        return Err(anyhow!("Rollback backup file does not exist."));
    }
    Ok(())
}

pub fn preview_managed_config_apply(record_id: &str) -> Result<ManagedConfigApplyPreview> {
    match record_id {
        CURSOR_SIDECAR_APPLY_RECORD_ID => preview_cursor_sidecar_apply(),
        GOOSE_NATIVE_APPLY_RECORD_ID => {
            let preview = preview_goose_provider_config()?;
            Ok(ManagedConfigApplyPreview {
                record_id: GOOSE_NATIVE_APPLY_RECORD_ID.to_string(),
                owner: GOOSE_NATIVE_OWNER.to_string(),
                target_path: preview.path.display().to_string(),
                marker: GOOSE_NATIVE_MARKER.to_string(),
                backup_path: goose_config_backup_pattern(),
                status: if preview.blocked_reason.is_some() {
                    ManagedRollbackExecutionStatus::Blocked
                } else {
                    ManagedRollbackExecutionStatus::Ready
                },
                confirmation_phrase: goose_apply_confirmation_phrase(&preview.current_state),
                current_state: preview.current_state,
                proposed_state: preview.proposed_state,
                rollback_preview:
                    "Restore the sibling *.headroom-backup-* Goose config through Rollback Center."
                        .to_string(),
                blocked_reason: preview.blocked_reason,
                evidence: preview.evidence,
            })
        }
        GOOSE_SIDECAR_APPLY_RECORD_ID => preview_provider_sidecar_apply(GOOSE_SIDECAR_APPLY_RECORD_ID, "goose", GOOSE_SIDECAR_OWNER),
        GROK_SIDECAR_APPLY_RECORD_ID => preview_provider_sidecar_apply(GROK_SIDECAR_APPLY_RECORD_ID, "grok_cli", GROK_SIDECAR_OWNER),
        GROK_ROLLBACK_RECORD_ID => {
            let path = grok_config_path();
            let current_state = if path.exists() {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?
            } else {
                String::new()
            };
            let (next_config, changed) = grok_next_provider_config()?;
            Ok(ManagedConfigApplyPreview {
                record_id: GROK_ROLLBACK_RECORD_ID.to_string(),
                owner: GROK_ROLLBACK_OWNER.to_string(),
                target_path: path.display().to_string(),
                marker: GROK_MARKER_PREFIX.to_string(),
                backup_path: grok_config_backup_pattern(),
                status: ManagedRollbackExecutionStatus::Ready,
                confirmation_phrase: grok_apply_confirmation_phrase(
                    GROK_MARKER_PREFIX,
                    &current_state,
                ),
                current_state,
                proposed_state: next_config,
                rollback_preview:
                    "Restore the sibling *.headroom-backup-* file through Rollback Center."
                        .to_string(),
                blocked_reason: None,
                evidence: vec![
                    "Installed Grok Build documentation explicitly allowlists [endpoints].models_base_url in ~/.grok/config.toml.".to_string(),
                    "Preview writes only the non-secret endpoint field and preserves all other TOML content.".to_string(),
                    format!("Preview changed: {changed}."),
                    "Apply creates a sibling backup, writes the endpoint, verifies the marker, and can roll back from the backup.".to_string(),
                    "XAI_API_KEY, auth.json, account state, and model selection remain untouched and manual.".to_string(),
                ],
            })
        }
        OPENCODE_ROLLBACK_RECORD_ID => {
            let path = opencode_config_path();
            let current_state = if path.exists() {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?
            } else {
                "{}".to_string()
            };
            let (next_config, changed) = opencode_next_provider_config()?;
            let proposed_state = serde_json::to_string_pretty(&next_config)
                .context("serializing OpenCode provider preview")?;
            Ok(ManagedConfigApplyPreview {
                record_id: OPENCODE_ROLLBACK_RECORD_ID.to_string(),
                owner: OPENCODE_ROLLBACK_OWNER.to_string(),
                target_path: path.display().to_string(),
                marker: OPENCODE_ROLLBACK_MARKER.to_string(),
                backup_path: opencode_config_backup_pattern(),
                status: ManagedRollbackExecutionStatus::Ready,
                confirmation_phrase: opencode_apply_confirmation_phrase(
                    OPENCODE_ROLLBACK_MARKER,
                    &current_state,
                ),
                current_state,
                proposed_state,
                rollback_preview:
                    "Restore the sibling *.headroom-backup-* file through Rollback Center."
                        .to_string(),
                blocked_reason: None,
                evidence: vec![
                    "OpenCode provider config is allowlisted for native safe apply.".to_string(),
                    "Preview preserves unmanaged JSON fields outside provider.headroom.".to_string(),
                    format!("Preview changed: {changed}."),
                    "Apply creates a sibling backup, writes the proposed JSON, verifies the provider, and can roll back from the backup.".to_string(),
                ],
            })
        }
        ZED_ROLLBACK_RECORD_ID => {
            let path = zed_config_path();
            let current_state = if path.exists() {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?
            } else {
                "{}".to_string()
            };
            let (next_config, changed) = zed_next_provider_config()?;
            let proposed_state = serde_json::to_string_pretty(&next_config)
                .context("serializing Zed provider preview")?;
            Ok(ManagedConfigApplyPreview {
                record_id: ZED_ROLLBACK_RECORD_ID.to_string(),
                owner: ZED_ROLLBACK_OWNER.to_string(),
                target_path: path.display().to_string(),
                marker: ZED_ROLLBACK_MARKER.to_string(),
                backup_path: zed_config_backup_pattern(),
                status: ManagedRollbackExecutionStatus::Ready,
                confirmation_phrase: zed_apply_confirmation_phrase(
                    ZED_ROLLBACK_MARKER,
                    &current_state,
                ),
                current_state,
                proposed_state,
                rollback_preview:
                    "Restore the sibling *.headroom-backup-* file through Rollback Center."
                        .to_string(),
                blocked_reason: None,
                evidence: vec![
                    "Zed provider config is allowlisted for native safe apply.".to_string(),
                    "Preview preserves unmanaged JSON fields outside provider routing.".to_string(),
                    format!("Preview changed: {changed}."),
                    "Apply creates a sibling backup, writes the proposed JSON, verifies the provider, and can roll back from the backup.".to_string(),
                ],
            })
        }
        WINDSURF_ROLLBACK_RECORD_ID => {
            let path = windsurf_config_path();
            let current_state = if path.exists() {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?
            } else {
                "{}".to_string()
            };
            let (next_config, changed) = windsurf_next_provider_config()?;
            let proposed_state = serde_json::to_string_pretty(&next_config)
                .context("serializing Windsurf provider preview")?;
            let confirmation =
                windsurf_apply_confirmation_phrase(WINDSURF_ROLLBACK_MARKER, &current_state);
            Ok(ManagedConfigApplyPreview {
                record_id: WINDSURF_ROLLBACK_RECORD_ID.to_string(),
                owner: WINDSURF_ROLLBACK_OWNER.to_string(),
                target_path: path.display().to_string(),
                marker: WINDSURF_ROLLBACK_MARKER.to_string(),
                backup_path: windsurf_config_backup_pattern(),
                status: ManagedRollbackExecutionStatus::Ready,
                confirmation_phrase: confirmation,
                current_state,
                proposed_state,
                rollback_preview:
                    "Restore the sibling *.headroom-backup-* file through Rollback Center."
                        .to_string(),
                blocked_reason: None,
                evidence: vec![
                    "Windsurf settings.json is allowlisted for native safe apply.".to_string(),
                    "Preview preserves unmanaged JSON fields outside managed markers.".to_string(),
                    format!("Preview changed: {changed}."),
                    "Apply creates a sibling backup, writes the proposed JSON, verifies the markers, and can roll back from the backup.".to_string(),
                ],
            })
        }
        CONTINUE_NATIVE_APPLY_RECORD_ID => {
            let path = continue_config_path();
            let current_state = if path.exists() {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?
            } else {
                String::new()
            };
            let (proposed_state, changed) = continue_next_provider_config()?;
            Ok(ManagedConfigApplyPreview {
                record_id: CONTINUE_NATIVE_APPLY_RECORD_ID.to_string(),
                owner: CONTINUE_NATIVE_OWNER.to_string(),
                target_path: path.display().to_string(),
                marker: CONTINUE_NATIVE_MARKER.to_string(),
                backup_path: continue_config_backup_pattern(),
                status: ManagedRollbackExecutionStatus::Ready,
                confirmation_phrase: continue_apply_confirmation_phrase(
                    CONTINUE_NATIVE_MARKER,
                    &current_state,
                ),
                current_state,
                proposed_state,
                rollback_preview:
                    "Restore the sibling *.headroom-backup-* file through Rollback Center."
                        .to_string(),
                blocked_reason: None,
                evidence: vec![
                    "Continue config.yaml models[] is allowlisted for native safe apply.".to_string(),
                    "Preview preserves unrelated models and never reads or writes apiKey values.".to_string(),
                    format!("Preview changed: {changed}."),
                    "Apply creates a sibling backup, writes the proposed YAML, verifies the Headroom model, and can roll back from the backup.".to_string(),
                ],
            })
        }
        AIDER_NATIVE_APPLY_RECORD_ID => {
            let path = aider_config_path();
            let current_state = if path.exists() {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?
            } else {
                String::new()
            };
            let (proposed_state, changed) = aider_next_provider_config()?;
            Ok(ManagedConfigApplyPreview {
                record_id: AIDER_NATIVE_APPLY_RECORD_ID.to_string(),
                owner: AIDER_NATIVE_OWNER.to_string(),
                target_path: path.display().to_string(),
                marker: AIDER_NATIVE_MARKER.to_string(),
                backup_path: aider_config_backup_pattern(),
                status: ManagedRollbackExecutionStatus::Ready,
                confirmation_phrase: aider_apply_confirmation_phrase(
                    AIDER_NATIVE_MARKER,
                    &current_state,
                ),
                current_state,
                proposed_state,
                rollback_preview:
                    "Restore the sibling *.headroom-backup-* file through Rollback Center."
                        .to_string(),
                blocked_reason: None,
                evidence: vec![
                    "Aider .aider.conf.yml openai-api-base is allowlisted for native safe apply."
                        .to_string(),
                    "Preview preserves API keys, api-key entries, set-env values, and unrelated settings."
                        .to_string(),
                    format!("Preview changed: {changed}."),
                    "Apply creates a sibling backup, writes the proposed YAML, verifies openai-api-base, and can roll back from the backup.".to_string(),
                ],
            })
        }
        _ => Err(anyhow!(
            "Managed config apply is currently promoted only for {CURSOR_SIDECAR_APPLY_RECORD_ID}, {GOOSE_NATIVE_APPLY_RECORD_ID}, {GOOSE_SIDECAR_APPLY_RECORD_ID}, {GROK_SIDECAR_APPLY_RECORD_ID}, {GROK_ROLLBACK_RECORD_ID}, {OPENCODE_ROLLBACK_RECORD_ID}, {ZED_ROLLBACK_RECORD_ID}, {WINDSURF_ROLLBACK_RECORD_ID}, {CONTINUE_NATIVE_APPLY_RECORD_ID}, and {AIDER_NATIVE_APPLY_RECORD_ID}."
        )),
    }
}

pub fn execute_managed_config_apply(
    record_id: &str,
    confirmation_phrase: &str,
) -> Result<ManagedConfigApplyResult> {
    let preview = preview_managed_config_apply(record_id)?;
    if confirmation_phrase != preview.confirmation_phrase {
        return Err(anyhow!(
            "Managed config apply confirmation phrase does not match."
        ));
    }
    match record_id {
        CURSOR_SIDECAR_APPLY_RECORD_ID => {
            let path = planned_sidecar_routing_path("cursor")?;
            let (changed, backup) = configure_planned_switchboard_sidecar("cursor")?;
            if !planned_switchboard_sidecar_matches("cursor")? {
                return Err(anyhow!(
                    "Cursor Switchboard sidecar verification failed after apply."
                ));
            }
            Ok(ManagedConfigApplyResult {
                record_id: CURSOR_SIDECAR_APPLY_RECORD_ID.to_string(),
                owner: CURSOR_SIDECAR_OWNER.to_string(),
                target_path: path.display().to_string(),
                changed,
                backup_path: backup.map(|path| path.display().to_string()),
                marker: CURSOR_MARKER_PREFIX.to_string(),
                verification: vec![
                    "Exact confirmation phrase matched the dry-run preview.".to_string(),
                    "Only the Switchboard-owned Cursor sidecar was written; Cursor settings, accounts, models, and extension storage were not read or changed.".to_string(),
                    "Managed sidecar marker was re-read from disk after apply.".to_string(),
                    "Rollback Center and Off mode remove only the managed sidecar block."
                        .to_string(),
                ],
            })
        }
        GOOSE_NATIVE_APPLY_RECORD_ID => {
            let preview = preview_goose_provider_config()?;
            if let Some(reason) = preview.blocked_reason {
                return Err(anyhow!("Goose native provider routing is blocked: {reason}"));
            }
            let path = preview.path;
            let (changed_files, backup_files) = configure_goose_provider_config()?;
            if !goose_provider_config_matches()? {
                return Err(anyhow!(
                    "Goose native endpoint config verification failed after apply."
                ));
            }
            Ok(ManagedConfigApplyResult {
                record_id: GOOSE_NATIVE_APPLY_RECORD_ID.to_string(),
                owner: GOOSE_NATIVE_OWNER.to_string(),
                target_path: path.display().to_string(),
                changed: changed_files
                    .iter()
                    .any(|changed| changed == &path.display().to_string()),
                backup_path: backup_files.first().cloned(),
                marker: GOOSE_NATIVE_MARKER.to_string(),
                verification: vec![
                    "Exact confirmation phrase matched the dry-run preview.".to_string(),
                    "Sibling backup was created before writing when a prior Goose config existed.".to_string(),
                    "Only allowlisted OpenAI/Anthropic endpoint fields were changed; provider, model, credentials, and account state remained untouched.".to_string(),
                    "Goose native endpoint values were re-read from disk after apply.".to_string(),
                    "Rollback Center can restore the selected sibling backup.".to_string(),
                ],
            })
        }
        GOOSE_SIDECAR_APPLY_RECORD_ID => execute_provider_sidecar_apply(record_id, "goose", GOOSE_SIDECAR_OWNER, confirmation_phrase),
        GROK_SIDECAR_APPLY_RECORD_ID => execute_provider_sidecar_apply(record_id, "grok_cli", GROK_SIDECAR_OWNER, confirmation_phrase),
        GROK_ROLLBACK_RECORD_ID => {
            let path = grok_config_path();
            let (changed_files, backup_files) = configure_grok_provider_config()?;
            if !grok_provider_config_matches()? {
                return Err(anyhow!(
                    "Grok native endpoint config verification failed after apply."
                ));
            }
            Ok(ManagedConfigApplyResult {
                record_id: GROK_ROLLBACK_RECORD_ID.to_string(),
                owner: GROK_ROLLBACK_OWNER.to_string(),
                target_path: path.display().to_string(),
                changed: changed_files
                    .iter()
                    .any(|changed| changed == &path.display().to_string()),
                backup_path: backup_files.first().cloned(),
                marker: GROK_MARKER_PREFIX.to_string(),
                verification: vec![
                    "Exact confirmation phrase matched the dry-run preview.".to_string(),
                    "Sibling backup was created before writing when a prior config existed.".to_string(),
                    "Grok [endpoints].models_base_url matches the Switchboard-managed proxy endpoint.".to_string(),
                    "Provider, model, account, auth.json, and API-key values were not read or changed.".to_string(),
                    "Rollback Center can restore the selected sibling backup.".to_string(),
                ],
            })
        }
        OPENCODE_ROLLBACK_RECORD_ID => {
            let path = opencode_config_path();
            let (changed_files, backup_files) = configure_opencode_provider_config()?;
            if !opencode_provider_config_matches()? {
                return Err(anyhow!(
                    "OpenCode provider config verification failed after apply."
                ));
            }
            Ok(ManagedConfigApplyResult {
                record_id: OPENCODE_ROLLBACK_RECORD_ID.to_string(),
                owner: OPENCODE_ROLLBACK_OWNER.to_string(),
                target_path: path.display().to_string(),
                changed: changed_files
                    .iter()
                    .any(|changed| changed == &path.display().to_string()),
                backup_path: backup_files.first().cloned(),
                marker: OPENCODE_ROLLBACK_MARKER.to_string(),
                verification: vec![
                    "Exact confirmation phrase matched the dry-run preview.".to_string(),
                    "Sibling backup was created before writing when a prior config existed."
                        .to_string(),
                    "OpenCode provider.headroom matches the Switchboard-managed provider."
                        .to_string(),
                    "Rollback Center can restore the selected sibling backup.".to_string(),
                ],
            })
        }
        ZED_ROLLBACK_RECORD_ID => {
            let path = zed_config_path();
            let (changed_files, backup_files) = configure_zed_provider_config()?;
            if !zed_provider_config_matches()? {
                return Err(anyhow!(
                    "Zed provider config verification failed after apply."
                ));
            }
            Ok(ManagedConfigApplyResult {
                record_id: ZED_ROLLBACK_RECORD_ID.to_string(),
                owner: ZED_ROLLBACK_OWNER.to_string(),
                target_path: path.display().to_string(),
                changed: changed_files
                    .iter()
                    .any(|changed| changed == &path.display().to_string()),
                backup_path: backup_files.first().cloned(),
                marker: ZED_ROLLBACK_MARKER.to_string(),
                verification: vec![
                    "Exact confirmation phrase matched the dry-run preview.".to_string(),
                    "Sibling backup was created before writing when a prior config existed."
                        .to_string(),
                    "Zed managed routing block matches the Switchboard-managed config."
                        .to_string(),
                    "Rollback Center can restore the selected sibling backup.".to_string(),
                ],
            })
        }
        WINDSURF_ROLLBACK_RECORD_ID => {
            let path = windsurf_config_path();
            let (changed_files, backup_files) = configure_windsurf_provider_config()?;
            if !windsurf_provider_config_matches()? {
                return Err(anyhow!(
                    "Windsurf provider config verification failed after apply."
                ));
            }
            Ok(ManagedConfigApplyResult {
                record_id: WINDSURF_ROLLBACK_RECORD_ID.to_string(),
                owner: WINDSURF_ROLLBACK_OWNER.to_string(),
                target_path: path.display().to_string(),
                changed: changed_files
                    .iter()
                    .any(|changed| changed == &path.display().to_string()),
                backup_path: backup_files.first().cloned(),
                marker: WINDSURF_ROLLBACK_MARKER.to_string(),
                verification: vec![
                    "Exact confirmation phrase matched the dry-run preview.".to_string(),
                    "Sibling backup was created before writing when a prior config existed."
                        .to_string(),
                    "Windsurf managed markers and anthropic.baseUrl match the Switchboard-managed values."
                        .to_string(),
                    "Rollback Center can restore the selected sibling backup.".to_string(),
                ],
            })
        }
        CONTINUE_NATIVE_APPLY_RECORD_ID => {
            let path = continue_config_path();
            let (changed_files, backup_files) = configure_continue_provider_config()?;
            if !continue_provider_config_matches()? {
                return Err(anyhow!(
                    "Continue provider config verification failed after apply."
                ));
            }
            Ok(ManagedConfigApplyResult {
                record_id: CONTINUE_NATIVE_APPLY_RECORD_ID.to_string(),
                owner: CONTINUE_NATIVE_OWNER.to_string(),
                target_path: path.display().to_string(),
                changed: changed_files
                    .iter()
                    .any(|changed| changed == &path.display().to_string()),
                backup_path: backup_files.first().cloned(),
                marker: CONTINUE_NATIVE_MARKER.to_string(),
                verification: vec![
                    "Exact confirmation phrase matched the dry-run preview.".to_string(),
                    "Sibling backup was created before writing when a prior config existed."
                        .to_string(),
                    "Continue models[] contains the Switchboard-managed Headroom model.".to_string(),
                    "Provider credentials, apiKey values, and unrelated model entries were not read or changed.".to_string(),
                    "Rollback Center can restore the selected sibling backup.".to_string(),
                ],
            })
        }
        AIDER_NATIVE_APPLY_RECORD_ID => {
            let path = aider_config_path();
            let (changed_files, backup_files) = configure_aider_provider_config()?;
            if !aider_provider_config_matches()? {
                return Err(anyhow!(
                    "Aider provider config verification failed after apply."
                ));
            }
            Ok(ManagedConfigApplyResult {
                record_id: AIDER_NATIVE_APPLY_RECORD_ID.to_string(),
                owner: AIDER_NATIVE_OWNER.to_string(),
                target_path: path.display().to_string(),
                changed: changed_files
                    .iter()
                    .any(|changed| changed == &path.display().to_string()),
                backup_path: backup_files.first().cloned(),
                marker: AIDER_NATIVE_MARKER.to_string(),
                verification: vec![
                    "Exact confirmation phrase matched the dry-run preview.".to_string(),
                    "Sibling backup was created before writing when a prior config existed."
                        .to_string(),
                    "Aider openai-api-base matches the Switchboard-managed Headroom proxy URL."
                        .to_string(),
                    "API keys, api-key entries, set-env values, and unrelated settings were not read or changed.".to_string(),
                    "Rollback Center can restore the selected sibling backup.".to_string(),
                ],
            })
        }
        _ => Err(anyhow!(
            "Managed config apply is currently promoted only for {CURSOR_SIDECAR_APPLY_RECORD_ID}, {GOOSE_NATIVE_APPLY_RECORD_ID}, {GOOSE_SIDECAR_APPLY_RECORD_ID}, {GROK_SIDECAR_APPLY_RECORD_ID}, {OPENCODE_ROLLBACK_RECORD_ID}, {ZED_ROLLBACK_RECORD_ID}, {WINDSURF_ROLLBACK_RECORD_ID}, {CONTINUE_NATIVE_APPLY_RECORD_ID}, and {AIDER_NATIVE_APPLY_RECORD_ID}."
        )),
    }
}

pub fn preview_managed_rollback(record_id: &str) -> Result<ManagedRollbackPreview> {
    if matches!(
        record_id,
        CODEX_ROLLBACK_RECORD_ID
            | OPENCODE_ROLLBACK_RECORD_ID
            | GROK_ROLLBACK_RECORD_ID
            | GOOSE_NATIVE_APPLY_RECORD_ID
            | GEMINI_ROLLBACK_RECORD_ID
            | WINDSURF_ROLLBACK_RECORD_ID
            | ZED_ROLLBACK_RECORD_ID
    ) {
        return preview_native_managed_rollback(record_id);
    }

    if let Some(target) = sidecar_rollback_target(record_id) {
        return preview_sidecar_rollback(target);
    }

    preview_native_managed_rollback(record_id)
}

fn preview_native_managed_rollback(record_id: &str) -> Result<ManagedRollbackPreview> {
    let target = managed_rollback_target(record_id)?;
    let target_path = (target.target_path)();
    let marker_present = (!target.backup_required || target_path.exists())
        && (target.marker_matches)().unwrap_or(false);
    let backup_path = target
        .backup_required
        .then(|| latest_headroom_backup_for(&target_path))
        .flatten();
    let backup_exists =
        !target.backup_required || backup_path.as_ref().is_some_and(|path| path.exists());
    let blocked_reason = if !marker_present {
        Some(format!(
            "Managed {} marker is not present in the target config.",
            target.owner
        ))
    } else if target.backup_required && !backup_exists {
        Some(format!(
            "No sibling Switchboard backup was found for the {} config.",
            target.owner
        ))
    } else {
        None
    };

    Ok(ManagedRollbackPreview {
        record_id: target.record_id.to_string(),
        owner: target.owner.to_string(),
        target_path: target_path.display().to_string(),
        marker: target.marker.to_string(),
        backup_path: backup_path.map(|path| path.display().to_string()),
        marker_present,
        backup_exists,
        status: if blocked_reason.is_none() {
            ManagedRollbackExecutionStatus::Ready
        } else {
            ManagedRollbackExecutionStatus::Blocked
        },
        confirmation_phrase: managed_rollback_confirmation_phrase(&target),
        proposed_action: target.proposed_action.to_string(),
        blocked_reason,
        evidence: target
            .evidence
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
    })
}

pub fn execute_managed_rollback(
    record_id: &str,
    backup_path: &str,
    confirmation_phrase: &str,
) -> Result<ManagedRollbackExecutionResult> {
    if matches!(
        record_id,
        CODEX_ROLLBACK_RECORD_ID
            | OPENCODE_ROLLBACK_RECORD_ID
            | GROK_ROLLBACK_RECORD_ID
            | GOOSE_NATIVE_APPLY_RECORD_ID
            | CONTINUE_NATIVE_APPLY_RECORD_ID
            | AIDER_NATIVE_APPLY_RECORD_ID
            | GEMINI_ROLLBACK_RECORD_ID
            | WINDSURF_ROLLBACK_RECORD_ID
            | ZED_ROLLBACK_RECORD_ID
    ) {
        return execute_native_managed_rollback(record_id, backup_path, confirmation_phrase);
    }

    if let Some(target) = sidecar_rollback_target(record_id) {
        return execute_sidecar_rollback(target, confirmation_phrase);
    }

    execute_native_managed_rollback(record_id, backup_path, confirmation_phrase)
}

fn execute_native_managed_rollback(
    record_id: &str,
    backup_path: &str,
    confirmation_phrase: &str,
) -> Result<ManagedRollbackExecutionResult> {
    let target = managed_rollback_target(record_id)?;
    let expected_confirmation = managed_rollback_confirmation_phrase(&target);
    if confirmation_phrase != expected_confirmation {
        return Err(anyhow!("Rollback confirmation phrase does not match."));
    }

    let target_path = (target.target_path)();
    if target.backup_required && !target_path.exists() {
        return Err(anyhow!("Rollback config target does not exist."));
    }
    if !(target.marker_matches)()? {
        return Err(anyhow!(
            "Managed {} marker is missing or has drifted; refusing rollback.",
            target.owner
        ));
    }
    let (restored_from, safety_backup, verification) = if target.backup_required {
        let backup_path = PathBuf::from(backup_path);
        validate_managed_rollback_backup_path(&target_path, &backup_path)?;

        let safety_backup = backup_if_exists(&target_path)?;
        std::fs::copy(&backup_path, &target_path).with_context(|| {
            format!(
                "restoring {} from {}",
                target_path.display(),
                backup_path.display()
            )
        })?;
        let _ = std::fs::read_to_string(&target_path)
            .with_context(|| format!("re-reading {}", target_path.display()))?;
        (
            backup_path.display().to_string(),
            safety_backup.map(|path| path.display().to_string()),
            vec![
                "Exact confirmation phrase matched.".to_string(),
                "Backup path was validated as a sibling Switchboard backup.".to_string(),
                "A fresh safety backup was created before restore.".to_string(),
                "Relaunch-survival evidence: restored config was re-read from disk after write."
                    .to_string(),
            ],
        )
    } else {
        disable_client_setup("gemini_cli")?;
        if target_path.exists() {
            let _ = std::fs::read_to_string(&target_path)
                .with_context(|| format!("re-reading {}", target_path.display()))?;
        }
        (
            "Switchboard-owned Gemini shell and sidecar blocks removed.".to_string(),
            None,
            vec![
                "Exact confirmation phrase matched.".to_string(),
                "Managed Gemini marker was present before cleanup.".to_string(),
                "Cleanup used disable_client_setup for Gemini Off-mode parity.".to_string(),
                "Relaunch-survival evidence: Gemini shell and sidecar files were re-read from disk after cleanup."
                    .to_string(),
            ],
        )
    };

    Ok(ManagedRollbackExecutionResult {
        record_id: target.record_id.to_string(),
        owner: target.owner.to_string(),
        target_path: target_path.display().to_string(),
        restored_from,
        safety_backup_path: safety_backup,
        marker: target.marker.to_string(),
        verification,
    })
}

pub fn preview_managed_rollback_undo_all() -> ManagedRollbackUndoAllPreview {
    let mut ready = Vec::new();
    let mut blocked = Vec::new();

    for record_id in NATIVE_MANAGED_ROLLBACK_RECORD_IDS {
        match preview_managed_rollback(record_id) {
            Ok(preview) if preview.status == ManagedRollbackExecutionStatus::Ready => {
                ready.push(preview)
            }
            Ok(preview) => blocked.push(preview),
            Err(err) => blocked.push(ManagedRollbackPreview {
                record_id: (*record_id).to_string(),
                owner: (*record_id).to_string(),
                target_path: String::new(),
                marker: String::new(),
                backup_path: None,
                marker_present: false,
                backup_exists: false,
                status: ManagedRollbackExecutionStatus::Blocked,
                confirmation_phrase: String::new(),
                proposed_action: "No native rollback preview could be prepared.".to_string(),
                blocked_reason: Some(err.to_string()),
                evidence: vec![format!(
                    "Undo-all preview failed while checking {record_id}; no files were modified."
                )],
            }),
        }
    }

    ManagedRollbackUndoAllPreview {
        status: if ready.is_empty() {
            ManagedRollbackExecutionStatus::Blocked
        } else {
            ManagedRollbackExecutionStatus::Ready
        },
        confirmation_phrase: MANAGED_ROLLBACK_UNDO_ALL_CONFIRMATION.to_string(),
        evidence: vec![
            "Undo-all preview is limited to allowlisted native rollback rows.".to_string(),
            "Each ready row already passed its per-row marker and backup readiness checks."
                .to_string(),
            "Execution re-previews rows immediately before modifying files.".to_string(),
            "Blocked rows are reported and left untouched.".to_string(),
        ],
        ready,
        blocked,
    }
}

pub fn execute_managed_rollback_undo_all(
    confirmation_phrase: &str,
) -> Result<ManagedRollbackUndoAllExecutionResult> {
    if confirmation_phrase != MANAGED_ROLLBACK_UNDO_ALL_CONFIRMATION {
        return Err(anyhow!("Undo-all confirmation phrase does not match."));
    }

    let preview = preview_managed_rollback_undo_all();
    if preview.ready.is_empty() {
        return Err(anyhow!("No native rollback rows are ready to execute."));
    }

    let mut executed = Vec::new();
    let mut verification = vec![
        "Undo-all confirmation phrase matched.".to_string(),
        "Rows were re-previewed before execution.".to_string(),
        "Only rows with ready native previews were executed.".to_string(),
    ];

    for row in &preview.ready {
        let result = execute_managed_rollback(
            &row.record_id,
            row.backup_path.as_deref().unwrap_or(""),
            &row.confirmation_phrase,
        )
        .with_context(|| format!("executing native rollback row {}", row.record_id))?;
        verification.push(format!("Executed {} ({})", row.owner, row.record_id));
        executed.push(result);
    }

    Ok(ManagedRollbackUndoAllExecutionResult {
        confirmation_phrase: MANAGED_ROLLBACK_UNDO_ALL_CONFIRMATION.to_string(),
        executed,
        blocked: preview.blocked,
        verification,
    })
}
