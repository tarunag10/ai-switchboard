use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::client_connector_status::MANAGED_CLIENT_SPECS;
use crate::client_connectors::{
    planned_connector_has_implemented_setup, planned_sidecar_spec, PlannedSidecarSpec,
    PLANNED_SIDECAR_SPECS,
};
use crate::client_detection::codex_home;
use crate::client_paths::{
    all_shell_paths, claude_settings_candidates, claude_settings_path, codex_config_toml_path,
    grok_config_path, headroom_rtk_hook_path, home_dir, opencode_config_path,
    planned_sidecar_routing_path, rtk_codex_agents_path, serialize_paths, windsurf_config_path,
    zed_config_path, SWITCHBOARD_ROUTING_FILE,
};
use crate::client_provider_configs::{
    configure_grok_provider_config, configure_opencode_provider_config,
    configure_windsurf_provider_config, configure_zed_provider_config,
    grok_provider_config_matches, opencode_provider_config_matches,
    remove_grok_provider_config, remove_opencode_provider_config,
    remove_windsurf_provider_config, remove_zed_provider_config,
    windsurf_provider_config_matches, zed_provider_config_matches,
    GROK_MARKER_PREFIX, HEADROOM_ANTHROPIC_BASE_URL, HEADROOM_OPENAI_BASE_URL,
    OPENCODE_HEADROOM_PROVIDER_ID,
};
pub(crate) use crate::client_codex_setup::{
    configure_codex_provider_block, disable_codex_cli, disable_codex_gui,
};
pub use crate::client_codex_setup::{codex_provider_block_matches, CODEX_ROOT_BLOCK_ID};
pub(crate) use crate::client_claude_settings::{
    configure_claude_settings_env, configure_vscode_settings, ensure_claude_settings_hook,
    remove_claude_settings_env, remove_legacy_vscode_base_url_keys, remove_vscode_connector_keys,
    claude_hook_present_in_value, entry_contains_hook,
};
pub(crate) use crate::client_setup_sidecar::{
    configure_planned_switchboard_sidecar, execute_provider_sidecar_apply,
    planned_switchboard_sidecar_matches, preview_cursor_sidecar_apply,
    preview_provider_sidecar_apply, CURSOR_MARKER_PREFIX, CURSOR_SIDECAR_APPLY_RECORD_ID,
    CURSOR_SIDECAR_OWNER, GOOSE_SIDECAR_APPLY_RECORD_ID, GOOSE_SIDECAR_OWNER,
    GROK_SIDECAR_APPLY_RECORD_ID, GROK_SIDECAR_OWNER,
};
pub(crate) use crate::client_shell_setup::{
    build_headroom_rtk_hook, claude_settings_env_matches, claude_settings_hook_matches,
    configure_shell_block, ensure_rtk_integrations_for_targets, is_headroom_proxy_reachable,
    managed_block_contains_text, shell_block_contains_in_files, shell_block_contains_text_in_files,
    shell_double_quote,
};
use crate::client_setup_state::{
    default_headroom_managed_python_path, default_headroom_rtk_path, is_codex_enabled,
    load_setup_state, normalized_setup_id, resolve_client_shell_targets,
    resolve_client_shell_targets_for_cleanup, write_setup_state, ClientSetupState,
};
use crate::managed_files::{
    backup_if_exists, managed_block_updated_content, managed_marker_end, managed_marker_start,
    marker_block_contains, parse_json_object, remove_managed_block, remove_shell_block,
    strip_marker_block, upsert_managed_block, write_file_if_changed,
};
use crate::models::{
    ClientSetupResult, ClientSetupVerification, ManagedConfigApplyPreview,
    ManagedConfigApplyResult, ManagedRollbackExecutionStatus,
};
use crate::client_setup_verify::verify_client_setup;

// Raw proxy base — use provider-specific constants below when configuring client endpoints.
const HEADROOM_PROXY_URL: &str = "http://127.0.0.1:6767";
pub(crate) const GEMINI_BASE_URL_ENV_KEY: &str = "GOOGLE_GEMINI_BASE_URL";
const GEMINI_COMPAT_BASE_URL_ENV_KEY: &str = "GEMINI_BASE_URL";
const GEMINI_API_KEY_ENV_KEY: &str = "GEMINI_API_KEY";
const GEMINI_HEADROOM_API_KEY_VALUE: &str = "headroom-local";
const MARKER_PREFIX: &str = "headroom";

pub fn apply_client_setup(client_id: &str) -> Result<ClientSetupResult> {
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();
    let mut state = load_setup_state();
    let state_id = normalized_setup_id(client_id).to_string();

    match client_id {
        "claude_code" => {
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let mut rtk_updates = ensure_rtk_integrations_for_targets(
                &default_headroom_rtk_path(),
                &default_headroom_managed_python_path(),
                &shell_targets,
            )?;
            let env_block = format!("export ANTHROPIC_BASE_URL={}", HEADROOM_ANTHROPIC_BASE_URL);
            let mut updates = configure_shell_block(&shell_targets, "claude_code", &env_block)?;
            let mut claude_updates =
                configure_claude_settings_env("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
            let mut legacy_updates = remove_legacy_vscode_base_url_keys()?;
            updates.0.append(&mut rtk_updates.0);
            updates.1.append(&mut rtk_updates.1);
            updates.0.append(&mut claude_updates.0);
            updates.1.append(&mut claude_updates.1);
            updates.0.append(&mut legacy_updates.0);
            updates.1.append(&mut legacy_updates.1);
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
            state
                .managed_shell_files
                .insert(state_id.clone(), serialize_paths(&shell_targets));
        }
        "vscode" => {
            let updates = configure_vscode_settings()?;
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        "codex" | "codex_cli" => {
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let env_block = format!("export OPENAI_BASE_URL={}", HEADROOM_OPENAI_BASE_URL);
            let mut updates = configure_shell_block(&shell_targets, "codex_cli", &env_block)?;
            let mut toml_updates = configure_codex_provider_block()?;
            updates.0.append(&mut toml_updates.0);
            updates.1.append(&mut toml_updates.1);
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
            state
                .managed_shell_files
                .insert(state_id.clone(), serialize_paths(&shell_targets));
            // Pull existing native threads into the headroom-provider menu so the
            // Codex history list stays whole once it routes through Headroom.
            let _ = crate::codex_threads::retag_codex_thread_providers("openai", "headroom");
        }
        "gemini_cli" => {
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let env_block = format!(
                "export {GEMINI_BASE_URL_ENV_KEY}={HEADROOM_PROXY_URL}\nexport {GEMINI_COMPAT_BASE_URL_ENV_KEY}={HEADROOM_PROXY_URL}\nexport {GEMINI_API_KEY_ENV_KEY}={GEMINI_HEADROOM_API_KEY_VALUE}"
            );
            let mut updates = configure_shell_block(&shell_targets, "gemini_cli", &env_block)?;
            let (changed, backup) = configure_planned_switchboard_sidecar(client_id)?;
            if changed {
                updates.0.push(
                    planned_sidecar_routing_path(client_id)?
                        .display()
                        .to_string(),
                );
            }
            if let Some(backup) = backup {
                updates.1.push(backup.display().to_string());
            }
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
            state
                .managed_shell_files
                .insert(state_id.clone(), serialize_paths(&shell_targets));
        }
        "opencode" => {
            let mut updates = configure_opencode_provider_config()?;
            let (changed, backup) = configure_planned_switchboard_sidecar(client_id)?;
            if changed {
                updates.0.push(
                    planned_sidecar_routing_path(client_id)?
                        .display()
                        .to_string(),
                );
            }
            if let Some(backup) = backup {
                updates.1.push(backup.display().to_string());
            }
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        "grok_cli" => {
            let mut updates = configure_grok_provider_config()?;
            let (sidecar_changed, sidecar_backup) =
                configure_planned_switchboard_sidecar(client_id)?;
            if sidecar_changed {
                updates.0.push(
                    planned_sidecar_routing_path(client_id)?
                        .display()
                        .to_string(),
                );
            }
            if let Some(backup) = sidecar_backup {
                updates.1.push(backup.display().to_string());
            }
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        "goose" => {
            let (changed, backups) =
                crate::goose_provider_configs::configure_goose_provider_config()?;
            changed_files.extend(changed);
            backup_files.extend(backups);
            let (sidecar_changed, sidecar_backup) =
                configure_planned_switchboard_sidecar(client_id)?;
            if sidecar_changed {
                changed_files.push(
                    planned_sidecar_routing_path(client_id)?
                        .display()
                        .to_string(),
                );
            }
            if let Some(backup) = sidecar_backup {
                backup_files.push(backup.display().to_string());
            }
        }
        "windsurf" => {
            let updates = configure_windsurf_provider_config()?;
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        "zed_ai" => {
            let updates = configure_zed_provider_config()?;
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        other if planned_sidecar_spec(other).is_some() => {
            if !planned_connector_has_implemented_setup(other) {
                return Err(anyhow!(
                    "Automatic setup is not supported yet for {other}. Use the guided workflow until backup, verify, rollback, and Off mode coverage are promoted."
                ));
            }
            let (changed, backup) = configure_planned_switchboard_sidecar(other)?;
            if changed {
                changed_files.push(planned_sidecar_routing_path(other)?.display().to_string());
            }
            if let Some(backup) = backup {
                backup_files.push(backup.display().to_string());
            }
        }
        other => return Err(anyhow!("Automatic setup is not supported yet for {other}.",)),
    }

    let configured_at = Utc::now().to_rfc3339();
    state.configured_clients.insert(state_id, configured_at);
    write_setup_state(&state)?;

    let already_configured = changed_files.is_empty();
    let summary = if let Some(sidecar) = planned_sidecar_spec(client_id) {
        if sidecar.id == "goose" && already_configured {
            "Goose provider routing and Repo Memory MCP bridge were already configured.".to_string()
        } else if sidecar.id == "goose" {
            "Goose provider routing and Repo Memory MCP bridge were configured; credentials and account state remain manual."
                .to_string()
        } else if sidecar.id == "grok_cli" && already_configured {
            "Grok / xAI native endpoint routing and Switchboard sidecar were already present."
                .to_string()
        } else if sidecar.id == "grok_cli" {
            "Grok / xAI native endpoint routing and Switchboard sidecar were written; credentials, account, and model selection remain manual.".to_string()
        } else if already_configured {
            format!("{} Switchboard sidecar was already present.", sidecar.name)
        } else {
            format!(
                "{} Switchboard sidecar written for reversible routing intent.",
                sidecar.name
            )
        }
    } else if already_configured {
        "Client was already configured for Headroom.".to_string()
    } else {
        "Client configuration updated to route through Headroom.".to_string()
    };

    let verification = verify_client_setup(client_id)?;

    Ok(ClientSetupResult {
        client_id: client_id.to_string(),
        applied: true,
        already_configured,
        summary,
        changed_files,
        backup_files,
        next_steps: client_setup_next_steps(client_id),
        verification,
    })
}
fn client_setup_next_steps(client_id: &str) -> Vec<String> {
    if normalized_setup_id(client_id) == "goose" {
        return vec![
            "Prepare the Repo Memory MCP handoff from Mode Inspector.".into(),
            "Keep Goose credentials, account state, and model selection configured manually; only the allowlisted provider endpoint fields are managed.".into(),
        ];
    }

    if normalized_setup_id(client_id) == "grok_cli" {
        return vec![
            "Keep XAI_API_KEY or Grok login authentication configured manually; Switchboard never stores credentials.".into(),
            "Run one Grok prompt and verify activity appears in Headroom.".into(),
        ];
    }

    vec![
        "Restart your terminal/editor session to pick up environment changes.".into(),
        format!(
            "Run one {} prompt and verify activity appears in Headroom.",
            match normalized_setup_id(client_id) {
                "codex_cli" => "Codex",
                "gemini_cli" => "Gemini CLI",
                "opencode" => "OpenCode",
                "cursor" => "Cursor",
                "grok_cli" => "Grok / xAI CLI",
                "aider" => "Aider",
                "continue" => "Continue",
                "qwen_code" => "Qwen Code",
                "amazon_q" => "Amazon Q Developer CLI",
                "windsurf" => "Windsurf",
                "zed_ai" => "Zed AI",
                _ => "Claude Code",
            }
        ),
    ]
}

pub use crate::client_setup_verify::verify_client_setup;
pub use crate::client_setup_disable::{
    clear_client_setups, disable_client_setup, restore_client_setups,
};
pub(crate) use crate::client_claude_settings::{
    remove_pre_tool_use_markers, strip_headroom_hook_from_settings,
};
