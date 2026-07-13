use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::cli_discovery;
use crate::client_cleanup;
use crate::client_connector_status::MANAGED_CLIENT_SPECS;
use crate::client_connectors::{
    planned_connector_has_implemented_setup, planned_sidecar_spec, PlannedSidecarSpec,
    PLANNED_SIDECAR_SPECS,
};
use crate::client_footprint::managed_backup_targets;
use crate::client_paths::{
    all_shell_paths, claude_settings_candidates, claude_settings_path, codex_config_toml_path,
    dedupe_paths, dedupe_strings, default_shell_targets_for_family, detect_shell_family,
    discover_managed_shell_targets, grok_config_path, headroom_markitdown_hook_path,
    headroom_rtk_hook_path, home_dir, is_profile_file, opencode_config_path,
    planned_sidecar_routing_path, resolve_default_shell_targets, rtk_codex_agents_path,
    serialize_paths, shell_targets_from_state, windsurf_config_path, zed_config_path,
    SWITCHBOARD_ROUTING_FILE,
};
use crate::client_provider_configs::{
    configure_grok_provider_config, configure_opencode_provider_config,
    configure_windsurf_provider_config, configure_zed_provider_config,
    grok_apply_confirmation_phrase, grok_config_backup_pattern, grok_next_provider_config,
    grok_provider_config_matches, opencode_apply_confirmation_phrase,
    opencode_config_backup_pattern, opencode_next_provider_config,
    opencode_provider_config_matches, remove_grok_provider_config, remove_opencode_provider_config,
    remove_windsurf_provider_config, remove_zed_provider_config,
    windsurf_apply_confirmation_phrase, windsurf_config_backup_pattern,
    windsurf_next_provider_config, windsurf_provider_config_matches, zed_apply_confirmation_phrase,
    zed_config_backup_pattern, zed_next_provider_config, zed_provider_config_matches,
    GROK_MARKER_PREFIX, HEADROOM_ANTHROPIC_BASE_URL, HEADROOM_OPENAI_BASE_URL,
    OPENCODE_HEADROOM_PROVIDER_ID,
};
#[cfg(test)]
use crate::client_provider_configs::{
    GROK_HEADROOM_BASE_URL, WINDSURF_MARKER_PREFIX, ZED_MARKER_PREFIX,
};
use crate::client_sidecar_rollbacks::{
    execute_sidecar_rollback, preview_sidecar_rollback, sidecar_rollback_target,
};
use crate::cursor_native::{assess_native_schema, evidence_lines as cursor_native_evidence};
use crate::goose_provider_configs::{
    configure_goose_provider_config, goose_apply_confirmation_phrase, goose_config_backup_pattern,
    goose_config_path, goose_provider_config_matches, preview_goose_provider_config,
    GOOSE_NATIVE_APPLY_RECORD_ID, GOOSE_NATIVE_MARKER, GOOSE_NATIVE_OWNER,
};
use crate::managed_files::{
    backup_if_exists, managed_block_updated_content, managed_marker_end, managed_marker_start,
    marker_block_contains, parse_json_object, remove_managed_block, remove_shell_block,
    strip_marker_block, upsert_managed_block, write_file_if_changed,
};
use crate::models::{
    ClientHealth, ClientSetupResult, ClientSetupVerification, ClientStatus,
    ManagedConfigApplyPreview, ManagedConfigApplyResult, ManagedRollbackExecutionResult,
    ManagedRollbackExecutionStatus, ManagedRollbackPreview, ManagedRollbackUndoAllExecutionResult,
    ManagedRollbackUndoAllPreview, SavingsMode, SwitchboardMode,
};
use crate::storage::{app_data_dir, config_file};

// Raw proxy base — use provider-specific constants below when configuring client endpoints.
const HEADROOM_PROXY_URL: &str = "http://127.0.0.1:6767";
const GEMINI_BASE_URL_ENV_KEY: &str = "GOOGLE_GEMINI_BASE_URL";
const GEMINI_COMPAT_BASE_URL_ENV_KEY: &str = "GEMINI_BASE_URL";
const GEMINI_API_KEY_ENV_KEY: &str = "GEMINI_API_KEY";
const GEMINI_HEADROOM_API_KEY_VALUE: &str = "headroom-local";
const CURSOR_MARKER_PREFIX: &str = "headroom:cursor";
const CURSOR_SIDECAR_APPLY_RECORD_ID: &str = "cursor-sidecar-routing";
const CURSOR_SIDECAR_OWNER: &str = "Cursor routing sidecar";
const GOOSE_SIDECAR_APPLY_RECORD_ID: &str = "goose-sidecar-routing";
const GOOSE_SIDECAR_OWNER: &str = "Goose routing-intent sidecar";
const GROK_SIDECAR_APPLY_RECORD_ID: &str = "grok-sidecar-routing";
const GROK_SIDECAR_OWNER: &str = "Grok / xAI CLI routing-intent sidecar";
const GROK_ROLLBACK_RECORD_ID: &str = "grok-routing";
const GROK_ROLLBACK_OWNER: &str = "Grok / xAI CLI routing";
const GROK_ROLLBACK_MARKER: &str = "headroom:grok";
const MARKER_PREFIX: &str = "headroom";
pub fn detect_clients() -> Vec<ClientStatus> {
    let setup_state = load_setup_state();

    vec![
        detect_claude_code_client(is_configured(&setup_state, "claude_code")),
        detect_codex_client(is_configured(&setup_state, "codex")),
        detect_gemini_cli_client(),
        detect_opencode_client(),
        detect_cursor_client(),
        detect_grok_cli_client(),
        detect_aider_client(),
        detect_continue_client(),
        detect_goose_client(),
        detect_qwen_code_client(),
        detect_amazon_q_client(),
        detect_windsurf_client(),
        detect_zed_ai_client(),
    ]
}

pub fn ensure_rtk_integrations(
    managed_rtk_path: &Path,
    managed_python_path: &Path,
) -> Result<(Vec<String>, Vec<String>)> {
    ensure_rtk_integrations_for_targets(
        managed_rtk_path,
        managed_python_path,
        &resolve_default_shell_targets(),
    )
}

fn ensure_rtk_integrations_for_targets(
    managed_rtk_path: &Path,
    managed_python_path: &Path,
    shell_targets: &[PathBuf],
) -> Result<(Vec<String>, Vec<String>)> {
    // Respect the user's opt-out so bootstrap, restore, and client setup don't
    // silently re-add the PATH export and Claude Code hook after they've been
    // turned off via the tool status toggle. Also skip when the binary is absent
    // (not installed / uninstalled) so we never write integrations pointing at a
    // missing rtk.
    if is_rtk_disabled() || !managed_rtk_path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();

    let mut path_updates = ensure_managed_rtk_on_path(managed_rtk_path, shell_targets)?;
    let mut hook_updates = ensure_claude_code_rtk_hook(managed_rtk_path, managed_python_path)?;
    changed_files.append(&mut path_updates.0);
    backup_files.append(&mut path_updates.1);
    changed_files.append(&mut hook_updates.0);
    backup_files.append(&mut hook_updates.1);

    // Codex has no PreToolUse-style hook, so the auto-rewrite can't be wired the
    // way it is for Claude Code. Mirror the MarkItDown approach: drop a managed
    // `~/.codex/AGENTS.md` nudge telling Codex to route shell commands through
    // the managed `rtk` binary (which is already on PATH via the block above).
    if is_codex_enabled() {
        let agents = rtk_codex_agents_path();
        let (codex_changed, codex_backup) =
            upsert_managed_block(&agents, "rtk", &build_rtk_codex_nudge(managed_rtk_path))?;
        if codex_changed {
            changed_files.push(agents.display().to_string());
        }
        if let Some(path) = codex_backup {
            backup_files.push(path.display().to_string());
        }
    }

    Ok((changed_files, backup_files))
}

/// Codex nudge: Codex has no command-rewrite hook, so it routes shell commands
/// through the managed `rtk` binary by being told to prefix them with it.
fn build_rtk_codex_nudge(managed_rtk_path: &Path) -> String {
    let bin = managed_rtk_path.display();
    format!(
        "## Token-saving shell commands (Headroom RTK)\n\
         Run shell commands through RTK to get compact, token-optimized output:\n\
         prefix the command with `{bin} ` (for example `{bin} git status`,\n\
         `{bin} ls -la`, `{bin} cargo build`). RTK passes through anything it\n\
         does not optimize, so it is safe to use as a prefix for any command."
    )
}

pub fn rtk_integration_status() -> Result<(bool, bool)> {
    let path_configured = shell_block_contains_text_in_files(
        &resolve_default_shell_targets(),
        "managed_rtk",
        "export PATH=",
    )?;
    let hook_configured = claude_settings_hook_matches("headroom-rtk-rewrite.sh")?
        && headroom_rtk_hook_path().exists();
    Ok((path_configured, hook_configured))
}

/// True when the user turned RTK off via the tool status toggle.
pub fn is_rtk_disabled() -> bool {
    load_setup_state().rtk_disabled
}

/// Enable or disable RTK from the tool status toggle. Disabling tears down the
/// RTK PATH export, the Claude Code hook, and the Codex AGENTS.md nudge (without
/// touching `ANTHROPIC_BASE_URL` routing) and persists the opt-out so bootstrap
/// won't re-add them. Enabling clears the flag and re-applies the integrations.
pub fn set_rtk_enabled(
    enabled: bool,
    managed_rtk_path: &Path,
    managed_python_path: &Path,
) -> Result<()> {
    let mut state = load_setup_state();
    state.rtk_disabled = !enabled;
    write_setup_state(&state)?;

    if enabled {
        ensure_rtk_integrations(managed_rtk_path, managed_python_path)?;
    } else {
        let shell_targets = resolve_client_shell_targets_for_cleanup(&state, "claude_code")?;
        remove_shell_block(&shell_targets, "managed_rtk")?;
        for settings_path in claude_settings_candidates() {
            let _ = strip_headroom_hook_from_settings(&settings_path);
        }
        let hook_path = headroom_rtk_hook_path();
        if hook_path.exists() {
            let _ = std::fs::remove_file(&hook_path);
        }
        let _ = remove_managed_block(&rtk_codex_agents_path(), "rtk");
    }

    Ok(())
}

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

pub fn verify_client_setup(client_id: &str) -> Result<ClientSetupVerification> {
    let mut checks = Vec::new();
    let mut failures = Vec::new();

    match client_id {
        "claude_code" => {
            let state = load_setup_state();
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let shell_ok = shell_block_contains_in_files(
                &shell_targets,
                "claude_code",
                "ANTHROPIC_BASE_URL",
                HEADROOM_ANTHROPIC_BASE_URL,
            )?;
            let rtk_path_ok =
                shell_block_contains_text_in_files(&shell_targets, "managed_rtk", "export PATH=")?;
            let claude_settings_ok =
                claude_settings_env_matches("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
            let rtk_hook_ok = claude_settings_hook_matches("headroom-rtk-rewrite.sh")?
                && headroom_rtk_hook_path().exists();

            if shell_ok {
                checks.push(
                    "Found Claude Code ANTHROPIC_BASE_URL export in managed shell block.".into(),
                );
            }
            if rtk_path_ok {
                checks.push("Found Headroom-managed RTK PATH export in shell profiles.".into());
            }
            if claude_settings_ok {
                checks.push(
                    "Found ~/.claude/settings.json env.ANTHROPIC_BASE_URL pointing to Headroom."
                        .into(),
                );
            }
            if rtk_hook_ok {
                checks.push(
                    "Found Headroom-managed RTK Claude hook in ~/.claude/settings.json.".into(),
                );
            }
            if !shell_ok && !claude_settings_ok {
                failures.push(
                    "Claude Code ANTHROPIC_BASE_URL was not found in shell blocks or ~/.claude/settings.json."
                        .into(),
                );
            }
            // RTK is a separate, opt-in integration (`set_rtk_enabled` tears it
            // down without touching ANTHROPIC_BASE_URL routing). Its wiring is
            // only ever added when the managed binary exists on disk (see
            // `ensure_rtk_integrations_for_targets`), so its absence must not
            // fail Claude Code verification when RTK isn't installed or the user
            // disabled it — routing is what "connected" means here.
            let rtk_required = !state.rtk_disabled && default_headroom_rtk_path().exists();
            if rtk_required && !rtk_path_ok {
                failures.push(
                    "Headroom-managed RTK PATH export was not found in shell profiles.".into(),
                );
            }
            if rtk_required && !rtk_hook_ok {
                failures.push(
                    "Headroom-managed RTK Claude hook was not found in ~/.claude/settings.json."
                        .into(),
                );
            }
        }
        "vscode" => {
            let mut delegated = verify_client_setup("claude_code")?;
            delegated.client_id = "vscode".to_string();
            return Ok(delegated);
        }
        "codex" | "codex_cli" => {
            let state = load_setup_state();
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let shell_ok = shell_block_contains_in_files(
                &shell_targets,
                "codex_cli",
                "OPENAI_BASE_URL",
                HEADROOM_OPENAI_BASE_URL,
            )?;
            let toml_ok = codex_provider_block_matches()?;

            if shell_ok {
                checks.push("Found Codex OPENAI_BASE_URL export in managed shell block.".into());
            }
            if toml_ok {
                checks
                    .push("Found Headroom-managed provider block in ~/.codex/config.toml.".into());
            }
            if toml_ok && !shell_ok {
                checks.push(
                    "Codex shell OPENAI_BASE_URL export was not found; config.toml provider routing is active."
                        .into(),
                );
            }
            if !toml_ok {
                failures.push(
                    "Headroom-managed provider block was not found in ~/.codex/config.toml.".into(),
                );
            }
            if !shell_ok && !toml_ok {
                failures
                    .push("Codex OPENAI_BASE_URL export was not found in shell profiles.".into());
            }
        }
        "goose" => {
            if crate::goose_provider_configs::goose_provider_config_matches()? {
                checks.push(
                    "Found Switchboard-managed Goose provider endpoint configuration.".into(),
                );
            } else {
                failures.push("Switchboard-managed Goose provider endpoint configuration was not found or does not match.".into());
            }
            if planned_switchboard_sidecar_matches(client_id)? {
                checks.push(
                    "Found Switchboard-managed Goose Repo Memory MCP bridge metadata.".into(),
                );
            }
        }
        "gemini_cli" => {
            let state = load_setup_state();
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {client_id}"))?;
            let sidecar_path = planned_sidecar_routing_path(client_id)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(client_id)?;
            let google_base_ok = shell_block_contains_in_files(
                &shell_targets,
                "gemini_cli",
                GEMINI_BASE_URL_ENV_KEY,
                HEADROOM_PROXY_URL,
            )?;
            let compat_base_ok = shell_block_contains_in_files(
                &shell_targets,
                "gemini_cli",
                GEMINI_COMPAT_BASE_URL_ENV_KEY,
                HEADROOM_PROXY_URL,
            )?;
            let api_key_ok = shell_block_contains_in_files(
                &shell_targets,
                "gemini_cli",
                GEMINI_API_KEY_ENV_KEY,
                GEMINI_HEADROOM_API_KEY_VALUE,
            )?;

            if sidecar_ok {
                checks.push(format!(
                    "Found Switchboard-managed {} sidecar at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            } else {
                failures.push(format!(
                    "Switchboard-managed {} sidecar was not found at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            }
            if google_base_ok {
                checks.push(format!(
                    "Found Gemini {} export pointing to Headroom.",
                    GEMINI_BASE_URL_ENV_KEY
                ));
            } else {
                failures.push(format!(
                    "Gemini {} export was not found in shell profiles.",
                    GEMINI_BASE_URL_ENV_KEY
                ));
            }
            if compat_base_ok {
                checks.push(format!(
                    "Found Gemini compatibility {} export pointing to Headroom.",
                    GEMINI_COMPAT_BASE_URL_ENV_KEY
                ));
            } else {
                failures.push(format!(
                    "Gemini compatibility {} export was not found in shell profiles.",
                    GEMINI_COMPAT_BASE_URL_ENV_KEY
                ));
            }
            if api_key_ok {
                checks.push(format!(
                    "Found Gemini {} export for local Headroom proxy auth.",
                    GEMINI_API_KEY_ENV_KEY
                ));
            } else {
                failures.push(format!(
                    "Gemini {} export was not found in shell profiles.",
                    GEMINI_API_KEY_ENV_KEY
                ));
            }
        }
        "opencode" => {
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {client_id}"))?;
            let sidecar_path = planned_sidecar_routing_path(client_id)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(client_id)?;
            let provider_ok = opencode_provider_config_matches()?;

            if sidecar_ok {
                checks.push(format!(
                    "Found Switchboard-managed {} sidecar at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            } else {
                failures.push(format!(
                    "Switchboard-managed {} sidecar was not found at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            }
            if provider_ok {
                checks.push(format!(
                    "Found OpenCode provider {} pointing to Headroom.",
                    OPENCODE_HEADROOM_PROVIDER_ID
                ));
            } else {
                failures.push(format!(
                    "OpenCode provider {} was not found in {}.",
                    OPENCODE_HEADROOM_PROVIDER_ID,
                    opencode_config_path().display()
                ));
            }
        }
        "grok_cli" => {
            let provider_ok = grok_provider_config_matches()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {client_id}"))?;
            let sidecar_path = planned_sidecar_routing_path(client_id)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(client_id)?;
            if provider_ok {
                checks.push(format!(
                    "Found Grok / xAI native endpoint routing in {} pointing to Headroom.",
                    grok_config_path().display()
                ));
                checks.push(
                    "Grok provider, model, account, and credential values remain user-controlled; Switchboard manages only [endpoints].models_base_url.".into(),
                );
            } else {
                failures.push(format!(
                    "Switchboard-managed Grok endpoint routing was not found in {}.",
                    grok_config_path().display()
                ));
            }
            if sidecar_ok {
                checks.push(format!(
                    "Found Switchboard-managed {} sidecar at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            } else {
                failures.push(format!(
                    "Switchboard-managed {} sidecar was not found at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            }
        }
        "windsurf" => {
            let provider_ok = windsurf_provider_config_matches()?;
            if provider_ok {
                checks.push(format!(
                    "Found Windsurf managed routing config in {}.",
                    windsurf_config_path().display()
                ));
            } else {
                failures.push(format!(
                    "Windsurf managed routing config was not found in {}.",
                    windsurf_config_path().display()
                ));
            }
        }
        "zed_ai" => {
            let provider_ok = zed_provider_config_matches()?;
            if provider_ok {
                checks.push(format!(
                    "Found Zed managed routing config in {}.",
                    zed_config_path().display()
                ));
            } else {
                failures.push(format!(
                    "Zed managed routing config was not found in {}.",
                    zed_config_path().display()
                ));
            }
        }
        other if planned_sidecar_spec(other).is_some() => {
            let sidecar = planned_sidecar_spec(other)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {other}"))?;
            let sidecar_path = planned_sidecar_routing_path(other)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(other)?;

            if sidecar_ok {
                checks.push(format!(
                    "Found Switchboard-managed {} sidecar at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            } else {
                failures.push(format!(
                    "Switchboard-managed {} sidecar was not found at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            }
        }
        other => return Err(anyhow!("Verification is not supported yet for {other}.",)),
    }

    // Proxy reachability is transient runtime state — the runtime warm-up
    // can finish after this verification runs. Surface it via the
    // `proxy_reachable` field, but don't fail `verified` on it. `verified`
    // attests only to "we wrote everything we needed to write".
    let proxy_reachable = is_headroom_proxy_reachable();
    if proxy_reachable {
        checks.push("Headroom proxy is reachable on 127.0.0.1:6767.".into());
    }

    Ok(ClientSetupVerification {
        client_id: client_id.to_string(),
        verified: failures.is_empty(),
        proxy_reachable,
        checks,
        failures,
    })
}

pub fn is_claude_code_enabled() -> bool {
    is_configured(&load_setup_state(), "claude_code")
}

pub fn is_codex_enabled() -> bool {
    is_configured(&load_setup_state(), "codex_cli")
}

pub use crate::client_connector_list::list_client_connectors;

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

fn configure_planned_switchboard_sidecar(client_id: &str) -> Result<(bool, Option<PathBuf>)> {
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

fn preview_cursor_sidecar_apply() -> Result<ManagedConfigApplyPreview> {
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

fn preview_provider_sidecar_apply(
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

fn execute_provider_sidecar_apply(
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

pub fn disable_client_setup(client_id: &str) -> Result<()> {
    let mut state = load_setup_state();

    match client_id {
        "codex" | "codex_cli" => {
            disable_codex_cli()?;
            disable_codex_gui()?;
            // Hand the threads back to the native-provider menu so the full
            // history stays visible once Codex no longer routes through Headroom.
            let _ = crate::codex_threads::retag_codex_thread_providers("headroom", "openai");
        }
        "codex_gui" => {
            disable_codex_gui()?;
        }
        "claude_code" => {
            let shell_targets = resolve_client_shell_targets_for_cleanup(&state, client_id)?;
            remove_shell_block(&shell_targets, "claude_code")?;
            // Also drop the managed_rtk PATH block so `rtk` isn't exported from
            // shell profiles after quit — otherwise the user's next shell still
            // has Headroom binaries shadowing whatever's on PATH.
            remove_shell_block(&shell_targets, "managed_rtk")?;
            remove_claude_settings_env("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
            let _ = remove_legacy_vscode_base_url_keys()?;
            // Strip the PreToolUse hook entry and delete the hook script so CC
            // behaves exactly as it did before Headroom was launched.
            for settings_path in claude_settings_candidates() {
                let _ = strip_headroom_hook_from_settings(&settings_path);
            }
            let hook_path = headroom_rtk_hook_path();
            if hook_path.exists() {
                let _ = std::fs::remove_file(&hook_path);
            }
        }
        "vscode" => remove_vscode_connector_keys()?,
        "gemini_cli" => {
            let shell_targets = resolve_client_shell_targets_for_cleanup(&state, client_id)?;
            remove_shell_block(&shell_targets, "gemini_cli")?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "opencode" => {
            remove_opencode_provider_config()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "goose" => {
            let _ = crate::goose_provider_configs::remove_goose_provider_config()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "grok_cli" => {
            let _ = remove_grok_provider_config()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "windsurf" => {
            remove_windsurf_provider_config()?;
        }
        "zed_ai" => {
            remove_zed_provider_config()?;
        }
        other if planned_sidecar_spec(other).is_some() => {
            let sidecar = planned_sidecar_spec(other)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {other}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(other)?, sidecar.id)?;
        }
        other => {
            return Err(anyhow!(
                "Automatic setup disable is not supported yet for {other}.",
            ))
        }
    }

    match client_id {
        "codex" | "codex_cli" => {
            state.configured_clients.remove("codex");
            state.configured_clients.remove("codex_cli");
            state.configured_clients.remove("codex_gui");
            state.remembered_clients.remove("codex");
            state.remembered_clients.remove("codex_cli");
            state.remembered_clients.remove("codex_gui");
            state.managed_shell_files.remove("codex");
            state.managed_shell_files.remove("codex_cli");
            state.managed_shell_files.remove("codex_gui");
            state.remembered_shell_files.remove("codex");
            state.remembered_shell_files.remove("codex_cli");
            state.remembered_shell_files.remove("codex_gui");
        }
        _ => {
            let state_id = normalized_setup_id(client_id);
            state.configured_clients.remove(state_id);
            state.remembered_clients.remove(state_id);
            state.managed_shell_files.remove(state_id);
            state.remembered_shell_files.remove(state_id);
        }
    }
    write_setup_state(&state)?;
    Ok(())
}

pub fn clear_client_setups() -> Result<()> {
    // Capture snapshot before disabling. We re-apply it afterwards because
    // disable_client_setup also clears remembered_clients as a side effect,
    // which would otherwise erase the snapshot we need for restore_client_setups.
    let pre = load_setup_state();
    let snapshot_clients = pre.configured_clients.clone();
    let snapshot_shell_files = pre.managed_shell_files.clone();

    for spec in MANAGED_CLIENT_SPECS {
        let _ = disable_client_setup(spec.id);
    }
    let _ = disable_client_setup("codex_gui");
    for spec in PLANNED_SIDECAR_SPECS {
        if pre.configured_clients.contains_key(spec.id) {
            let _ = disable_client_setup(spec.id);
        }
    }

    // Re-save the remembered snapshot so restore_client_setups works on next launch.
    if !snapshot_clients.is_empty() {
        let mut state = load_setup_state();
        state.remembered_clients = snapshot_clients;
        state.remembered_shell_files = snapshot_shell_files;
        write_setup_state(&state)?;
    }

    Ok(())
}

/// Fully uninstalls Headroom's on-disk footprint on a best-effort basis:
/// reverses every client setup, strips Headroom's hook entry from Claude Code
/// settings (both `settings.json` and `settings.local.json`), deletes the
/// managed hook script, the Headroom application-support directory, the
/// `~/.headroom` Python runtime, the macOS LaunchAgent plist, Preferences,
/// Caches, and keychain entries.
///
/// Returns the list of paths that were successfully removed (useful for
/// surfacing to the user). Per-step failures are logged and skipped.
pub fn perform_full_cleanup() -> Vec<String> {
    let mut removed: Vec<String> = Vec::new();

    // Reverse settings.json mutations and shell blocks for every known client.
    if let Err(err) = clear_client_setups() {
        log::warn!("cleanup: clear_client_setups failed: {err}");
    }

    // Strip the Headroom hook entry from both ~/.claude/settings.json and
    // ~/.claude/settings.local.json. `clear_client_setups` doesn't do this —
    // it only removes env keys — so without this step the hook entry remains,
    // points to a deleted script, and Claude Code logs errors on every call.
    for settings_path in claude_settings_candidates() {
        match strip_headroom_hook_from_settings(&settings_path) {
            Ok(true) => removed.push(settings_path.display().to_string()),
            Ok(false) => {}
            Err(err) => log::warn!(
                "cleanup: stripping hook from {} failed: {err}",
                settings_path.display()
            ),
        }
    }

    for hook_path in [headroom_rtk_hook_path(), headroom_markitdown_hook_path()] {
        if hook_path.exists() {
            match std::fs::remove_file(&hook_path) {
                Ok(_) => removed.push(hook_path.display().to_string()),
                Err(err) => log::warn!("cleanup: removing {} failed: {err}", hook_path.display()),
            }
        }
    }

    // Drop the managed RTK nudge from ~/.codex/AGENTS.md (clear_client_setups
    // handles env/shell blocks but not these managed Markdown blocks).
    if let Err(err) = remove_managed_block(&rtk_codex_agents_path(), "rtk") {
        log::warn!("cleanup: removing rtk AGENTS.md block failed: {err}");
    }

    // Drop the managed Caveman guidance blocks from both client instruction files.
    if let Err(err) = disable_caveman_integration() {
        log::warn!("cleanup: removing caveman managed blocks failed: {err}");
    }

    // Also wipe the per-client setup-state file so a reinstall starts clean.
    let setup_state = setup_state_path();
    if setup_state.exists() {
        let _ = std::fs::remove_file(&setup_state);
    }

    removed.extend(client_cleanup::remove_managed_runtime_storage());

    #[cfg(target_os = "macos")]
    {
        removed.extend(client_cleanup::remove_macos_launch_agents());
        removed.extend(client_cleanup::remove_macos_app_state());
    }

    #[cfg(not(target_os = "macos"))]
    client_cleanup::remove_known_keychain_entries();

    // Sweep `<basename>.headroom-backup-*` and `<basename>.nommer-backup-*`
    // siblings created by `backup_if_exists` for every file we ever mutated.
    // Without this, stale backups remain in ~/.claude, ~/.claude/hooks,
    // ~/.codex, ~/Library/Application Support/Code/User, and the user's
    // shell rc directory after uninstall.
    for target in managed_backup_targets() {
        removed.extend(client_cleanup::sweep_managed_backups(&target));
    }

    removed
}

/// Remove the PreToolUse entry pointing at `headroom-rtk-rewrite.sh`. Drops
/// the `PreToolUse` array if it becomes empty, and the `hooks` object if it
/// has no remaining event arrays. Returns true if the file was modified.
fn strip_headroom_hook_from_settings(settings_path: &Path) -> Result<bool> {
    remove_pre_tool_use_markers(
        settings_path,
        &["headroom-rtk-rewrite.sh", "headroom-markitdown-read.sh"],
    )
}

/// Removes every PreToolUse hook entry whose command contains one of `markers`,
/// pruning empty `PreToolUse`/`hooks` containers. Returns whether the file changed.
fn remove_pre_tool_use_markers(settings_path: &Path, markers: &[&str]) -> Result<bool> {
    if !settings_path.exists() {
        return Ok(false);
    }

    let raw = std::fs::read_to_string(settings_path)
        .with_context(|| format!("reading {}", settings_path.display()))?;
    if raw.trim().is_empty() {
        return Ok(false);
    }
    let mut root = parse_json_object(&raw, settings_path)?;

    let Some(hooks_val) = root.get_mut("hooks") else {
        return Ok(false);
    };
    let Some(hooks_obj) = hooks_val.as_object_mut() else {
        return Ok(false);
    };

    let mut changed = false;

    if let Some(pre_tool_use) = hooks_obj
        .get_mut("PreToolUse")
        .and_then(|value| value.as_array_mut())
    {
        let before = pre_tool_use.len();
        pre_tool_use.retain(|entry| {
            !markers
                .iter()
                .any(|marker| entry_contains_hook(entry, marker))
        });
        if pre_tool_use.len() != before {
            changed = true;
        }
        if pre_tool_use.is_empty() {
            hooks_obj.remove("PreToolUse");
        }
    }

    if hooks_obj.is_empty() {
        root.remove("hooks");
    }

    if !changed {
        return Ok(false);
    }

    let _ = backup_if_exists(settings_path)?;
    std::fs::write(
        settings_path,
        serde_json::to_vec_pretty(&Value::Object(root))
            .context("serializing Claude settings for hook cleanup")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok(true)
}

/// Re-applies setup for all clients that were active at the last pause or quit.
pub fn restore_client_setups() {
    let state = load_setup_state();
    let to_restore: Vec<String> = state.remembered_clients.keys().cloned().collect();
    for client_id in to_restore {
        let _ = apply_client_setup(&client_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientSetupState {
    pub(crate) configured_clients: BTreeMap<String, String>,
    /// Snapshot of configured_clients taken at last pause/quit, used to restore on next startup.
    #[serde(default)]
    pub(crate) remembered_clients: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) managed_shell_files: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub(crate) remembered_shell_files: BTreeMap<String, Vec<String>>,
    /// User opted RTK out via the tool status toggle. When true, bootstrap and
    /// client setup skip re-adding the RTK PATH export and Claude Code hook.
    #[serde(default)]
    rtk_disabled: bool,
    #[serde(default)]
    switchboard_mode: Option<SwitchboardMode>,
    #[serde(default)]
    savings_mode: Option<SavingsMode>,
}

pub fn load_switchboard_mode() -> Option<SwitchboardMode> {
    load_setup_state().switchboard_mode
}

pub fn write_switchboard_mode(mode: SwitchboardMode) -> Result<()> {
    let mut state = load_setup_state();
    state.switchboard_mode = Some(mode);
    write_setup_state(&state)
}

pub fn load_savings_mode() -> SavingsMode {
    load_setup_state()
        .savings_mode
        .unwrap_or(SavingsMode::Balanced)
}

pub fn write_savings_mode(mode: SavingsMode) -> Result<()> {
    let mut state = load_setup_state();
    state.savings_mode = Some(mode);
    write_setup_state(&state)
}

pub(crate) fn is_configured(state: &ClientSetupState, client_id: &str) -> bool {
    configured_timestamp(state, client_id).is_some()
}

pub(crate) fn configured_timestamp(state: &ClientSetupState, client_id: &str) -> Option<String> {
    let primary = normalized_setup_id(client_id);
    state.configured_clients.get(primary).cloned()
}

pub(crate) fn load_setup_state() -> ClientSetupState {
    let path = setup_state_path();
    if !path.exists() {
        return ClientSetupState::default();
    }

    // The on-disk file is rewritten by other code paths in this module
    // (apply_client_setup, disable_client_setup, clear_client_setups). Even
    // though `write_setup_state` now publishes via tmp+rename, retry once
    // before giving up: a parse failure on an existing file is almost always
    // a transient race or a partially-written file from an older build, and
    // returning the empty default flips `is_claude_code_enabled` to false,
    // which the tray reads as "Claude Code disconnected" and notifies on.
    match try_load_setup_state(&path) {
        Ok(state) => normalize_setup_state(state),
        Err(first_err) => {
            std::thread::sleep(std::time::Duration::from_millis(15));
            match try_load_setup_state(&path) {
                Ok(state) => normalize_setup_state(state),
                Err(second_err) => {
                    log::warn!(
                        "load_setup_state: failed to read/parse {} twice ({first_err:#}; {second_err:#}); returning default",
                        path.display()
                    );
                    ClientSetupState::default()
                }
            }
        }
    }
}

fn try_load_setup_state(path: &Path) -> Result<ClientSetupState> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice::<ClientSetupState>(&bytes)
        .with_context(|| format!("parsing {}", path.display()))
}

fn normalize_setup_state(mut state: ClientSetupState) -> ClientSetupState {
    state.configured_clients = normalize_setup_entries(state.configured_clients);
    state.remembered_clients = normalize_setup_entries(state.remembered_clients);
    state.managed_shell_files = normalize_shell_file_entries(state.managed_shell_files);
    state.remembered_shell_files = normalize_shell_file_entries(state.remembered_shell_files);
    state
}

fn normalize_setup_entries(mut entries: BTreeMap<String, String>) -> BTreeMap<String, String> {
    // codex_gui is a removed id; codex/codex_cli are live again, keep them.
    entries.remove("codex_gui");

    entries
}

fn normalize_shell_file_entries(
    mut entries: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    entries.remove("codex_gui");

    for files in entries.values_mut() {
        dedupe_strings(files);
    }

    entries
}

pub(crate) fn write_setup_state(state: &ClientSetupState) -> Result<()> {
    let path = setup_state_path();
    let payload = serde_json::to_vec_pretty(state).context("serializing client setup state")?;

    // Publish atomically: write to a sibling tmp file then rename. POSIX
    // rename is atomic, so concurrent readers (e.g. the tray-icon thread
    // calling `is_claude_code_enabled` every 2s) see either the old file or
    // the new one — never a half-written truncate. The previous direct
    // `fs::write` opened a microsecond window where readers parsed an empty
    // file, concluded no clients were configured, and flipped the tray to
    // "Disconnected" with a spurious notification.
    let tmp_path = {
        let mut s = path.clone().into_os_string();
        s.push(".tmp");
        PathBuf::from(s)
    };
    std::fs::write(&tmp_path, &payload)
        .with_context(|| format!("writing {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path)
        .with_context(|| format!("renaming {} -> {}", tmp_path.display(), path.display()))
}

fn setup_state_path() -> PathBuf {
    config_file(&app_data_dir(), "client-setup.json")
}

fn default_headroom_root_dir() -> PathBuf {
    app_data_dir().join("headroom")
}

fn default_headroom_rtk_path() -> PathBuf {
    default_headroom_root_dir().join("bin").join("rtk")
}

fn default_headroom_managed_python_path() -> PathBuf {
    default_headroom_root_dir()
        .join("runtime")
        .join("venv")
        .join("bin")
        .join("python3")
}

fn resolve_client_shell_targets(state: &ClientSetupState, client_id: &str) -> Result<Vec<PathBuf>> {
    let state_id = normalized_setup_id(client_id);
    let mut targets = shell_targets_from_state(state.managed_shell_files.get(state_id));
    if targets.is_empty() {
        targets = shell_targets_from_state(state.remembered_shell_files.get(state_id));
    }
    targets.extend(discover_managed_shell_targets(&[
        "claude_code",
        "managed_rtk",
        "codex_cli",
    ])?);

    let default_targets = default_shell_targets_for_family(detect_shell_family());
    if targets.is_empty() {
        targets = default_targets;
    } else {
        for file in default_targets {
            if is_profile_file(&file) {
                targets.push(file);
            }
        }
    }

    Ok(dedupe_paths(targets))
}

fn resolve_client_shell_targets_for_cleanup(
    state: &ClientSetupState,
    client_id: &str,
) -> Result<Vec<PathBuf>> {
    let mut targets = resolve_client_shell_targets(state, client_id)?;
    targets.extend(all_shell_paths());
    Ok(dedupe_paths(targets))
}

fn configure_shell_block(
    shell_targets: &[PathBuf],
    block_id: &str,
    block_body: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut changed = Vec::new();
    let mut backups = Vec::new();

    for file in shell_targets {
        let (did_change, backup) = upsert_managed_block(&file, block_id, block_body)?;
        if did_change {
            changed.push(file.display().to_string());
            if let Some(path) = backup {
                backups.push(path.display().to_string());
            }
        }
    }

    Ok((changed, backups))
}

fn ensure_managed_rtk_on_path(
    rtk_path: &Path,
    shell_targets: &[PathBuf],
) -> Result<(Vec<String>, Vec<String>)> {
    let managed_bin_dir = rtk_path.parent().ok_or_else(|| {
        anyhow!(
            "managed RTK path {} is missing a parent directory",
            rtk_path.display()
        )
    })?;
    let path_value = shell_double_quote(&managed_bin_dir.to_string_lossy());
    configure_shell_block(
        shell_targets,
        "managed_rtk",
        &format!("export PATH=\"{path_value}:$PATH\""),
    )
}

fn ensure_claude_code_rtk_hook(
    managed_rtk_path: &Path,
    managed_python_path: &Path,
) -> Result<(Vec<String>, Vec<String>)> {
    let hook_path = headroom_rtk_hook_path();
    let hook_body = build_headroom_rtk_hook(managed_rtk_path, managed_python_path);
    let (hook_changed, hook_backup) = write_file_if_changed(&hook_path, &hook_body, true)?;
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();

    if hook_changed {
        changed_files.push(hook_path.display().to_string());
    }
    if let Some(path) = hook_backup {
        backup_files.push(path.display().to_string());
    }

    let (settings_changed, settings_backups) =
        ensure_claude_settings_hook(&hook_path, "Bash", "headroom-rtk-rewrite.sh")?;
    changed_files.extend(settings_changed);
    backup_files.extend(settings_backups);

    Ok((changed_files, backup_files))
}

fn markitdown_claude_md_path() -> PathBuf {
    home_dir().join(".claude").join("CLAUDE.md")
}

fn markitdown_codex_agents_path() -> PathBuf {
    codex_home().join("AGENTS.md")
}

/// Office-only nudge for Claude Code, where PDFs are already handled by the
/// PreToolUse(Read) hook.
fn build_markitdown_office_nudge(shim_path: &Path) -> String {
    let bin = shim_path.display();
    format!(
        "## Reading Office documents (Headroom MarkItDown)\n\
         The Read tool cannot open .docx, .doc, .pptx, .ppt, .xlsx, or .xls files.\n\
         To read one, run `{bin} <path>` via Bash and use the Markdown it prints.\n\
         (PDFs are handled automatically and need no special step.)"
    )
}

/// Codex nudge: Codex has no PreToolUse-style hook, so it covers PDF *and*
/// Office formats through the `markitdown` CLI.
fn build_markitdown_codex_nudge(shim_path: &Path) -> String {
    let bin = shim_path.display();
    format!(
        "## Reading documents (Headroom MarkItDown)\n\
         To read a .pdf, .docx, .doc, .pptx, .ppt, .xlsx, or .xls file, run\n\
         `{bin} <path>` in the shell and use the Markdown it prints, rather than\n\
         opening the raw file. This keeps large documents cheap to read."
    )
}

/// Enables the MarkItDown addon integration for whichever coding clients are
/// configured through Headroom: Claude Code gets the PDF Read hook plus an
/// Office nudge (managed `~/.claude/CLAUDE.md` block + scoped Bash permission);
/// Codex gets a managed `~/.codex/AGENTS.md` nudge covering PDF and Office (it
/// has no hook mechanism). Idempotent and safe to re-run.
pub fn enable_markitdown_integration(
    markitdown_entrypoint: &Path,
    markitdown_shim: &Path,
    python_path: &Path,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();

    if is_claude_code_enabled() {
        let hook_path = headroom_markitdown_hook_path();
        let hook_body = build_headroom_markitdown_hook(markitdown_entrypoint, python_path);
        let (hook_changed, hook_backup) = write_file_if_changed(&hook_path, &hook_body, true)?;
        if hook_changed {
            changed_files.push(hook_path.display().to_string());
        }
        if let Some(path) = hook_backup {
            backup_files.push(path.display().to_string());
        }

        let (settings_changed, settings_backups) =
            ensure_claude_settings_hook(&hook_path, "Read", "headroom-markitdown-read.sh")?;
        changed_files.extend(settings_changed);
        backup_files.extend(settings_backups);

        let claude_md = markitdown_claude_md_path();
        let (md_changed, md_backup) = upsert_managed_block(
            &claude_md,
            "markitdown_office",
            &build_markitdown_office_nudge(markitdown_shim),
        )?;
        if md_changed {
            changed_files.push(claude_md.display().to_string());
        }
        if let Some(path) = md_backup {
            backup_files.push(path.display().to_string());
        }

        if set_markitdown_bash_permission(markitdown_shim, true)? {
            changed_files.push(claude_settings_path().display().to_string());
        }
    }

    if is_codex_enabled() {
        let agents = markitdown_codex_agents_path();
        let (codex_changed, codex_backup) = upsert_managed_block(
            &agents,
            "markitdown",
            &build_markitdown_codex_nudge(markitdown_shim),
        )?;
        if codex_changed {
            changed_files.push(agents.display().to_string());
        }
        if let Some(path) = codex_backup {
            backup_files.push(path.display().to_string());
        }
    }

    Ok((changed_files, backup_files))
}

/// Removes every MarkItDown integration artifact for all clients (Claude Read
/// hook + script + Office nudge + Bash permission, and the Codex AGENTS.md
/// nudge), leaving any RTK hook untouched. Cleanup runs unconditionally so a
/// client that was later disconnected is still scrubbed.
pub fn disable_markitdown_integration(markitdown_shim: &Path) -> Result<bool> {
    let mut changed =
        remove_pre_tool_use_markers(&claude_settings_path(), &["headroom-markitdown-read.sh"])?;
    let hook_path = headroom_markitdown_hook_path();
    if hook_path.exists() {
        let _ = std::fs::remove_file(&hook_path);
    }
    changed |= remove_managed_block(&markitdown_claude_md_path(), "markitdown_office")?;
    changed |= set_markitdown_bash_permission(markitdown_shim, false)?;
    changed |= remove_managed_block(&markitdown_codex_agents_path(), "markitdown")?;
    Ok(changed)
}

fn caveman_claude_md_path() -> PathBuf {
    home_dir().join(".claude").join("CLAUDE.md")
}

fn caveman_codex_agents_path() -> PathBuf {
    codex_home().join("AGENTS.md")
}

/// Terse-output guidance body keyed by level. Scoped is the conservative
/// default: terse only where short output is safe, never hiding required
/// legal, safety, or debugging detail. Aggressive asks for terseness broadly.
/// Compact Chinese is experimental and only for internal working notes.
fn build_caveman_nudge(level: &str) -> String {
    match level {
        "aggressive" => "## Terse output (Switchboard Caveman, aggressive)\n\
             Default to terse output everywhere. Lead with the answer or result; cut\n\
             preamble, restated questions, and summaries of what you just did. Prefer\n\
             fragments and short synonyms. Still include any legal, safety, or\n\
             debugging detail the task actually requires -- brevity never overrides\n\
             correctness or required disclosure."
            .to_string(),
        "compact_chinese" => {
            "## Terse output (Switchboard Caveman, compact Chinese experimental)\n\
             Use compact Chinese only for private internal planning notes, scratch\n\
             handoffs, and hidden working prompts when that reduces tokens. Keep all\n\
             user-visible replies, commit messages, PR notes, legal, safety,\n\
             debugging, and release-readiness content in the user's requested\n\
             language with complete required detail. Never translate code, commands,\n\
             file paths, identifiers, error text, secrets, citations, or quoted\n\
             source material. If compact Chinese could make verification ambiguous,\n\
             use terse English instead."
                .to_string()
        }
        _ => "## Terse output (Switchboard Caveman, scoped)\n\
             For command summaries, PR notes, and handoffs, keep output terse: lead\n\
             with the result and drop preamble and self-summaries. Do NOT shorten\n\
             legal, safety, or debugging content -- keep those complete even when the\n\
             surrounding prose is terse."
            .to_string(),
    }
}

/// Enables the Caveman addon: writes a Switchboard-owned managed guidance block
/// into the instruction file of each configured coding client (Claude Code's
/// `~/.claude/CLAUDE.md`, Codex's `~/.codex/AGENTS.md`). Pure guidance -- no
/// hook, runtime, or permission. Idempotent and safe to re-run.
pub fn enable_caveman_integration(level: &str) -> Result<(Vec<String>, Vec<String>)> {
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();
    let body = build_caveman_nudge(level);

    if is_claude_code_enabled() {
        let claude_md = caveman_claude_md_path();
        let (md_changed, md_backup) = upsert_managed_block(&claude_md, "caveman", &body)?;
        if md_changed {
            changed_files.push(claude_md.display().to_string());
        }
        if let Some(path) = md_backup {
            backup_files.push(path.display().to_string());
        }
    }

    if is_codex_enabled() {
        let agents = caveman_codex_agents_path();
        let (codex_changed, codex_backup) = upsert_managed_block(&agents, "caveman", &body)?;
        if codex_changed {
            changed_files.push(agents.display().to_string());
        }
        if let Some(path) = codex_backup {
            backup_files.push(path.display().to_string());
        }
    }

    Ok((changed_files, backup_files))
}

pub fn caveman_integration_matches_level(level: &str) -> Result<bool> {
    let expected = build_caveman_nudge(level);
    if is_claude_code_enabled()
        && !managed_block_contains_text(&caveman_claude_md_path(), "caveman", &expected)?
    {
        return Ok(false);
    }
    if is_codex_enabled()
        && !managed_block_contains_text(&caveman_codex_agents_path(), "caveman", &expected)?
    {
        return Ok(false);
    }
    Ok(true)
}

/// Removes the managed Caveman block from every client instruction file. Runs
/// unconditionally so a later-disconnected client is still scrubbed.
pub fn disable_caveman_integration() -> Result<bool> {
    let mut changed = remove_managed_block(&caveman_claude_md_path(), "caveman")?;
    changed |= remove_managed_block(&caveman_codex_agents_path(), "caveman")?;
    Ok(changed)
}

/// Adds or removes a `Bash(<shim> *)` entry in `permissions.allow` so the Office
/// nudge can run `markitdown` without prompting. Returns whether settings changed.
fn set_markitdown_bash_permission(shim_path: &Path, present: bool) -> Result<bool> {
    let settings_path = claude_settings_path();
    let entry = format!("Bash({} *)", shim_path.display());

    let mut content = if settings_path.exists() {
        let raw = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("reading {}", settings_path.display()))?;
        if raw.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            Value::Object(parse_json_object(&raw, &settings_path)?)
        }
    } else if present {
        Value::Object(Default::default())
    } else {
        return Ok(false);
    };

    let root = content
        .as_object_mut()
        .ok_or_else(|| anyhow!("unable to write Claude permissions settings"))?;
    let allow = root
        .entry("permissions")
        .or_insert_with(|| Value::Object(Default::default()))
        .as_object_mut()
        .ok_or_else(|| anyhow!("permissions is not an object"))?
        .entry("allow")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| anyhow!("permissions.allow is not an array"))?;

    let already = allow.iter().any(|v| v.as_str() == Some(entry.as_str()));
    if present == already {
        return Ok(false);
    }
    if present {
        allow.push(Value::String(entry));
    } else {
        allow.retain(|v| v.as_str() != Some(entry.as_str()));
    }

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let _ = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&content).context("serializing Claude permissions settings")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;
    Ok(true)
}

fn disable_codex_cli() -> Result<()> {
    remove_codex_provider_block()?;
    let _ = remove_codex_toml_key("openai_base_url", HEADROOM_OPENAI_BASE_URL);
    let shell_targets = all_shell_paths();
    let _ = remove_shell_block(&shell_targets, "codex_cli");
    let _ = remove_shell_block(&shell_targets, "codex");
    Ok(())
}

fn disable_codex_gui() -> Result<()> {
    clear_legacy_codex_gui_launch_env()?;
    Ok(())
}

fn clear_legacy_codex_gui_launch_env() -> Result<()> {
    remove_launchctl_env(&["OPENAI_BASE_URL", "OPENAI_API_BASE"])?;
    Ok(())
}

fn configure_vscode_settings() -> Result<(Vec<String>, Vec<String>)> {
    let (mut changed_files, mut backup_files) =
        configure_claude_settings_env("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
    let (legacy_changed, legacy_backups) = remove_legacy_vscode_base_url_keys()?;
    changed_files.extend(legacy_changed);
    backup_files.extend(legacy_backups);
    Ok((changed_files, backup_files))
}

fn remove_vscode_connector_keys() -> Result<()> {
    remove_claude_settings_env("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
    let _ = remove_legacy_vscode_base_url_keys()?;
    Ok(())
}

fn set_json_string(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    expected_value: &str,
) -> bool {
    let next = Value::String(expected_value.to_string());
    match obj.get(key) {
        Some(existing) if existing == &next => false,
        _ => {
            obj.insert(key.to_string(), next);
            true
        }
    }
}

fn remove_json_key_if_matches(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    expected_value: &str,
) -> bool {
    match obj.get(key) {
        Some(Value::String(value)) if value == expected_value => obj.remove(key).is_some(),
        _ => false,
    }
}

fn configure_claude_settings_env(
    env_key: &str,
    env_value: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let settings_path = claude_settings_path();
    let mut content = if settings_path.exists() {
        let raw = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("reading {}", settings_path.display()))?;
        Value::Object(parse_json_object(&raw, &settings_path)?)
    } else {
        Value::Object(Default::default())
    };

    if !content.is_object() {
        content = Value::Object(Default::default());
    }

    let Some(root) = content.as_object_mut() else {
        return Err(anyhow!("unable to write Claude settings"));
    };

    if !root
        .get("env")
        .map(|value| value.is_object())
        .unwrap_or(false)
    {
        root.insert("env".into(), Value::Object(Default::default()));
    }

    let Some(env_obj) = root.get_mut("env").and_then(|value| value.as_object_mut()) else {
        return Err(anyhow!("unable to write Claude env settings"));
    };

    let changed = set_json_string(env_obj, env_key, env_value);
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let backup = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&content).context("serializing Claude settings")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok((
        vec![settings_path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

fn ensure_claude_settings_hook(
    hook_path: &Path,
    matcher: &str,
    marker: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let settings_path = claude_settings_path();
    let mut content = if settings_path.exists() {
        let raw = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("reading {}", settings_path.display()))?;
        Value::Object(parse_json_object(&raw, &settings_path)?)
    } else {
        Value::Object(Default::default())
    };

    if !content.is_object() {
        content = Value::Object(Default::default());
    }

    let hook_command = hook_path
        .to_str()
        .ok_or_else(|| anyhow!("hook path contains invalid UTF-8: {}", hook_path.display()))?;
    let already_present = claude_hook_present_in_value(&content, hook_command);
    if already_present {
        return Ok((Vec::new(), Vec::new()));
    }

    let Some(root) = content.as_object_mut() else {
        return Err(anyhow!("unable to write Claude hook settings"));
    };

    if !root
        .get("hooks")
        .map(|value| value.is_object())
        .unwrap_or(false)
    {
        root.insert("hooks".into(), Value::Object(Default::default()));
    }

    let Some(hooks_obj) = root
        .get_mut("hooks")
        .and_then(|value| value.as_object_mut())
    else {
        return Err(anyhow!("unable to write Claude hooks settings"));
    };
    if !hooks_obj
        .get("PreToolUse")
        .map(|value| value.is_array())
        .unwrap_or(false)
    {
        hooks_obj.insert("PreToolUse".into(), Value::Array(Vec::new()));
    }

    let Some(pre_tool_use) = hooks_obj
        .get_mut("PreToolUse")
        .and_then(|value| value.as_array_mut())
    else {
        return Err(anyhow!("unable to write Claude PreToolUse hooks"));
    };

    pre_tool_use.retain(|entry| !entry_contains_hook(entry, marker));
    pre_tool_use.push(serde_json::json!({
        "matcher": matcher,
        "hooks": [{
            "type": "command",
            "command": hook_command
        }]
    }));

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let backup = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&content).context("serializing Claude hook settings")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok((
        vec![settings_path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

fn remove_claude_settings_env(env_key: &str, expected_value: &str) -> Result<()> {
    let settings_path = claude_settings_path();
    if !settings_path.exists() {
        return Ok(());
    }

    let raw = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("reading {}", settings_path.display()))?;
    let mut root = parse_json_object(&raw, &settings_path)?;
    let mut changed = false;

    if let Some(Value::Object(env_obj)) = root.get_mut("env") {
        changed |= remove_json_key_if_matches(env_obj, env_key, expected_value);
        if env_obj.is_empty() {
            root.remove("env");
            changed = true;
        }
    }

    if !changed {
        return Ok(());
    }

    let _ = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&Value::Object(root))
            .context("serializing Claude settings for connector removal")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok(())
}

fn claude_hook_present_in_value(content: &Value, hook_path: &str) -> bool {
    content
        .get("hooks")
        .and_then(|value| value.get("PreToolUse"))
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|hooks| hooks.as_array())
                    .map(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("command")
                                .and_then(|command| command.as_str())
                                .map(|command| command == hook_path)
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn entry_contains_hook(entry: &Value, hook_fragment: &str) -> bool {
    entry
        .get("hooks")
        .and_then(|hooks| hooks.as_array())
        .map(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("command")
                    .and_then(|command| command.as_str())
                    .map(|command| command.contains(hook_fragment))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn remove_legacy_vscode_base_url_keys() -> Result<(Vec<String>, Vec<String>)> {
    let settings_path = home_dir()
        .join("Library")
        .join("Application Support")
        .join("Code")
        .join("User")
        .join("settings.json");
    if !settings_path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let raw = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("reading {}", settings_path.display()))?;
    let mut obj = parse_json_object(&raw, &settings_path)?;

    let mut changed = false;
    changed |= remove_json_key_if_matches(&mut obj, "openai.baseUrl", HEADROOM_PROXY_URL);
    changed |= remove_json_key_if_matches(&mut obj, "anthropic.baseUrl", HEADROOM_PROXY_URL);
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    let backup = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&Value::Object(obj))
            .context("serializing VS Code settings for legacy key cleanup")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok((
        vec![settings_path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

// The managed Codex config is split across two marker blocks so each lands in
// the correct TOML scope. `model_provider`/`openai_base_url` are root keys: a
// bare key belongs to the most recently opened `[table]` above it, so appending
// them at end-of-file (as a naive text upsert does) silently absorbs them into
// whatever table the user's config happens to end in (e.g. `[features]`, whose
// values must be booleans), producing
// `invalid type: string "headroom", expected a boolean in features`. The root
// keys therefore go in a block at the *top* of the file (nothing above ⇒ root
// scope), and the `[model_providers.headroom]` table goes in a block at the
// *end*. `requires_openai_auth` is emitted only for ChatGPT-OAuth users: the
// flag is what makes Codex render the account menu (profile/email/plan/usage),
// but it also forces Codex to demand an OpenAI OAuth login (issue #406), which
// would break users authenticated with an OpenAI API key. See
// `codex_uses_chatgpt_auth`.
const CODEX_ROOT_BLOCK_ID: &str = "codex_cli";
const CODEX_TABLE_BLOCK_ID: &str = "codex_cli_provider";

fn codex_root_keys_body() -> String {
    format!(
        "model_provider = \"headroom\"\n\
         openai_base_url = \"{base}\"",
        base = HEADROOM_OPENAI_BASE_URL,
    )
}

/// Whether Codex is authenticated via ChatGPT OAuth (rather than an OpenAI API
/// key), read from `~/.codex/auth.json`. Drives whether the managed provider
/// block carries `requires_openai_auth = true` (see [`codex_provider_table_body`]).
fn codex_uses_chatgpt_auth() -> bool {
    let path = codex_home().join("auth.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    let Some(obj) = value.as_object() else {
        return false;
    };
    // Codex records the active method explicitly; trust it when present.
    if let Some(mode) = obj.get("auth_mode").and_then(Value::as_str) {
        return mode.eq_ignore_ascii_case("chatgpt");
    }
    // Older auth.json files predate `auth_mode`: infer ChatGPT mode from the
    // presence of an OAuth account id.
    obj.get("tokens")
        .and_then(Value::as_object)
        .and_then(|tokens| tokens.get("account_id"))
        .and_then(Value::as_str)
        .is_some_and(|id| !id.is_empty())
}

fn codex_provider_table_body(requires_openai_auth: bool) -> String {
    let mut body = format!(
        "[model_providers.headroom]\n\
         name = \"Headroom persistent proxy\"\n\
         base_url = \"{base}\"\n\
         supports_websockets = true",
        base = HEADROOM_OPENAI_BASE_URL,
    );
    if requires_openai_auth {
        body.push_str("\nrequires_openai_auth = true");
    }
    body
}

fn codex_marker_block(block_id: &str, body: &str) -> String {
    format!(
        "{}\n{body}\n{}\n",
        managed_marker_start(MARKER_PREFIX, block_id),
        managed_marker_end(MARKER_PREFIX, block_id)
    )
}

/// Remove every Headroom-managed artifact from Codex `config.toml` text: both
/// managed marker blocks, plus any orphan root keys an older (buggy) build may
/// have left absorbed into a preceding table. Leaves all other content intact.
fn strip_codex_managed_toml(content: &str) -> String {
    let without_blocks = strip_marker_block(
        &strip_marker_block(content, CODEX_ROOT_BLOCK_ID),
        CODEX_TABLE_BLOCK_ID,
    );
    let openai_orphan_prefix = "openai_base_url = \"http://127.0.0.1:";
    strip_legacy_codex_headroom_provider_table(&without_blocks)
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !(trimmed == "model_provider = \"headroom\""
                || (trimmed.starts_with(openai_orphan_prefix) && trimmed.ends_with("/v1\"")))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_legacy_codex_headroom_provider_table(content: &str) -> String {
    let mut out = Vec::new();
    let mut dropping = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[model_providers.headroom]" {
            dropping = true;
            continue;
        }
        if dropping && trimmed.starts_with('[') && trimmed.ends_with(']') {
            dropping = false;
        }
        if !dropping {
            out.push(line);
        }
    }
    out.join("\n")
}

/// Reconstruct `config.toml` with the managed root keys pinned to the top and
/// the provider table appended at the end, around the user's other content.
fn render_codex_config(existing: &str) -> String {
    let mid = strip_codex_managed_toml(existing);
    let mid = mid.trim();

    let mut out = codex_marker_block(CODEX_ROOT_BLOCK_ID, &codex_root_keys_body());
    if !mid.is_empty() {
        out.push('\n');
        out.push_str(mid);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&codex_marker_block(
        CODEX_TABLE_BLOCK_ID,
        &codex_provider_table_body(codex_uses_chatgpt_auth()),
    ));
    out
}

fn configure_codex_provider_block() -> Result<(Vec<String>, Vec<String>)> {
    let path = codex_config_toml_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let existing = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };

    let updated = render_codex_config(&existing);
    if updated == existing {
        return Ok((Vec::new(), Vec::new()));
    }

    let backup = backup_if_exists(&path)?;
    std::fs::write(&path, &updated).with_context(|| format!("writing {}", path.display()))?;

    let mut backup_files = Vec::new();
    if let Some(backup_path) = backup {
        backup_files.push(backup_path.display().to_string());
    }
    Ok((vec![path.display().to_string()], backup_files))
}

pub fn codex_provider_block_matches() -> Result<bool> {
    let path = codex_config_toml_path();
    if !path.exists() {
        return Ok(false);
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let base_url = format!("base_url = \"{}\"", HEADROOM_OPENAI_BASE_URL);
    let openai_base = format!("openai_base_url = \"{}\"", HEADROOM_OPENAI_BASE_URL);
    let root_ok = marker_block_contains(
        &content,
        CODEX_ROOT_BLOCK_ID,
        "model_provider = \"headroom\"",
    ) && marker_block_contains(&content, CODEX_ROOT_BLOCK_ID, &openai_base);
    let table_ok = marker_block_contains(&content, CODEX_TABLE_BLOCK_ID, &base_url);
    Ok(root_ok && table_ok)
}

const CODEX_ROLLBACK_RECORD_ID: &str = "codex-routing";
const CODEX_ROLLBACK_OWNER: &str = "Codex routing";
const CODEX_ROLLBACK_MARKER: &str = "headroom:codex_cli";
const OPENCODE_ROLLBACK_RECORD_ID: &str = "opencode-routing";
const OPENCODE_ROLLBACK_OWNER: &str = "OpenCode routing";
const OPENCODE_ROLLBACK_MARKER: &str = "headroom:opencode";
const GEMINI_ROLLBACK_RECORD_ID: &str = "gemini-routing";
const GEMINI_ROLLBACK_OWNER: &str = "Gemini CLI routing";
const GEMINI_ROLLBACK_MARKER: &str = "headroom:gemini_cli";
const ZED_ROLLBACK_RECORD_ID: &str = "zed-ai-routing";
const ZED_ROLLBACK_OWNER: &str = "Zed routing";
const ZED_ROLLBACK_MARKER: &str = "headroom:zed";
const ZED_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: zed-ai-routing.",
    "Backup must live next to ~/.config/zed/settings.json and use *.headroom-backup-*.",
    "Current config must still contain the managed Zed markers before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
];
const WINDSURF_ROLLBACK_RECORD_ID: &str = "windsurf-routing";
const WINDSURF_ROLLBACK_OWNER: &str = "Windsurf routing";
const WINDSURF_ROLLBACK_MARKER: &str = "headroom:windsurf";
const WINDSURF_ROLLBACK_EVIDENCE: &[&str] = &[
    "Allowlisted rollback execution row: windsurf-routing.",
    "Backup must live next to ~/Library/Application Support/Windsurf/User/settings.json and use *.headroom-backup-*.",
    "Current config must still contain the managed Windsurf markers before restore.",
    "Relaunch-survival evidence requires re-reading restored config from disk after write.",
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
        _ => Err(anyhow!(
            "Managed rollback execution is currently enabled only for {CODEX_ROLLBACK_RECORD_ID}, {OPENCODE_ROLLBACK_RECORD_ID}, {GROK_ROLLBACK_RECORD_ID}, {GOOSE_NATIVE_APPLY_RECORD_ID}, {GEMINI_ROLLBACK_RECORD_ID}, {WINDSURF_ROLLBACK_RECORD_ID}, and {ZED_ROLLBACK_RECORD_ID}."
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
        _ => Err(anyhow!(
            "Managed config apply is currently promoted only for {CURSOR_SIDECAR_APPLY_RECORD_ID}, {GOOSE_NATIVE_APPLY_RECORD_ID}, {GOOSE_SIDECAR_APPLY_RECORD_ID}, {GROK_SIDECAR_APPLY_RECORD_ID}, {GROK_ROLLBACK_RECORD_ID}, {OPENCODE_ROLLBACK_RECORD_ID}, {ZED_ROLLBACK_RECORD_ID}, and {WINDSURF_ROLLBACK_RECORD_ID}."
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
        _ => Err(anyhow!(
            "Managed config apply is currently promoted only for {CURSOR_SIDECAR_APPLY_RECORD_ID}, {GOOSE_NATIVE_APPLY_RECORD_ID}, {GOOSE_SIDECAR_APPLY_RECORD_ID}, {GROK_SIDECAR_APPLY_RECORD_ID}, {OPENCODE_ROLLBACK_RECORD_ID}, {ZED_ROLLBACK_RECORD_ID}, and {WINDSURF_ROLLBACK_RECORD_ID}."
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

fn remove_codex_provider_block() -> Result<()> {
    let path = codex_config_toml_path();
    if !path.exists() {
        return Ok(());
    }
    let existing =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let stripped = strip_codex_managed_toml(&existing);
    let normalized = {
        let trimmed = stripped.trim();
        if trimmed.is_empty() {
            String::new()
        } else {
            format!("{trimmed}\n")
        }
    };
    if normalized == existing {
        return Ok(());
    }
    let _ = backup_if_exists(&path)?;
    std::fs::write(&path, &normalized).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn remove_codex_toml_key(key: &str, expected_value: &str) -> Result<()> {
    let path = codex_config_toml_path();
    if !path.exists() {
        return Ok(());
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let target_line = format!("{key} = \"{expected_value}\"");
    let filtered: Vec<&str> = content
        .lines()
        .filter(|l| l.trim() != target_line)
        .collect();
    if filtered.len() == content.lines().count() {
        return Ok(());
    }
    let _ = backup_if_exists(&path)?;
    let mut result = filtered.join("\n");
    if !result.ends_with('\n') && !result.is_empty() {
        result.push('\n');
    }
    std::fs::write(&path, result).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn remove_launchctl_env(keys: &[&str]) -> Result<()> {
    for key in keys {
        let _ = run_launchctl(&["unsetenv", key]);
    }
    Ok(())
}

fn run_launchctl(args: &[&str]) -> Result<std::process::Output> {
    let output = Command::new("launchctl")
        .args(args)
        .output()
        .with_context(|| format!("running launchctl {}", args.join(" ")))?;
    if output.status.success() {
        return Ok(output);
    }

    Err(anyhow!(
        "launchctl {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

pub(crate) fn normalized_setup_id(client_id: &str) -> &str {
    match client_id {
        "codex" | "codex_gui" => "codex_cli",
        "vscode" => "claude_code",
        other => other,
    }
}

fn shell_block_contains_in_files(
    shell_targets: &[PathBuf],
    block_id: &str,
    var_name: &str,
    expected_value: &str,
) -> Result<bool> {
    for file in shell_targets {
        if !file.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let start = format!("# >>> headroom:{block_id} >>>");
        let end = format!("# <<< headroom:{block_id} <<<");

        if let (Some(start_idx), Some(end_idx)) = (content.find(&start), content.find(&end)) {
            let block = &content[start_idx..end_idx];
            let expected_line = format!("export {var_name}={expected_value}");
            if block.contains(&expected_line) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn shell_block_contains_text_in_files(
    shell_targets: &[PathBuf],
    block_id: &str,
    expected_text: &str,
) -> Result<bool> {
    for file in shell_targets {
        if !file.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let start = format!("# >>> headroom:{block_id} >>>");
        let end = format!("# <<< headroom:{block_id} <<<");

        if let (Some(start_idx), Some(end_idx)) = (content.find(&start), content.find(&end)) {
            if content[start_idx..end_idx].contains(expected_text) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn claude_settings_env_matches(env_key: &str, expected_value: &str) -> Result<bool> {
    let path = claude_settings_path();
    if !path.exists() {
        return Ok(false);
    }

    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let content: Value = Value::Object(parse_json_object(&raw, &path)?);
    Ok(matches!(
        content.get("env").and_then(|env| env.get(env_key)),
        Some(Value::String(value)) if value == expected_value
    ))
}

fn claude_settings_hook_matches(hook_fragment: &str) -> Result<bool> {
    let path = claude_settings_path();
    if !path.exists() {
        return Ok(false);
    }

    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let content: Value = Value::Object(parse_json_object(&raw, &path)?);

    Ok(content
        .get("hooks")
        .and_then(|hooks| hooks.get("PreToolUse"))
        .and_then(|hooks| hooks.as_array())
        .map(|entries| {
            entries
                .iter()
                .any(|entry| entry_contains_hook(entry, hook_fragment))
        })
        .unwrap_or(false))
}

fn is_headroom_proxy_reachable() -> bool {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    ["127.0.0.1", "localhost"].iter().any(|host| {
        client
            .get(format!("http://{host}:6767/readyz"))
            .send()
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    })
}

fn managed_block_contains_text(
    file_path: &Path,
    block_id: &str,
    expected_text: &str,
) -> Result<bool> {
    if !file_path.exists() {
        return Ok(false);
    }
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;
    let start = format!("# >>> headroom:{block_id} >>>");
    let end = format!("# <<< headroom:{block_id} <<<");
    let (Some(start_idx), Some(end_idx)) = (content.find(&start), content.find(&end)) else {
        return Ok(false);
    };
    Ok(content[start_idx..end_idx].contains(expected_text))
}

/// PreToolUse(Read) hook: when Claude reads a PDF, convert it to Markdown via
/// the managed `markitdown` and redirect the read at the converted file through
/// `updatedInput.file_path`. Fails open at every step so a missing binary,
/// oversized file, or conversion error falls through to a native Read.
///
/// Scoped to PDF deliberately: Claude Code's Read tool rejects unsupported
/// binary types (docx/pptx/xlsx) at input validation *before* PreToolUse hooks
/// run, so a hook can never intercept them. Office formats are handled instead
/// by the managed CLAUDE.md nudge that points Claude at the `markitdown` CLI.
fn build_headroom_markitdown_hook(markitdown_path: &Path, python_path: &Path) -> String {
    let markitdown = shell_double_quote(&markitdown_path.to_string_lossy());
    let python = shell_double_quote(&python_path.to_string_lossy());

    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

HEADROOM_MARKITDOWN="{markitdown}"
HEADROOM_PYTHON="{python}"

if [ ! -x "$HEADROOM_MARKITDOWN" ] || [ ! -x "$HEADROOM_PYTHON" ]; then
  exit 0
fi

INPUT="$(cat)"
if [ -z "$INPUT" ]; then
  exit 0
fi

HEADROOM_MD_CACHE="${{TMPDIR:-/tmp}}/headroom-markitdown"
mkdir -p "$HEADROOM_MD_CACHE" 2>/dev/null || exit 0

HEADROOM_MARKITDOWN_BIN="$HEADROOM_MARKITDOWN" HEADROOM_MD_CACHE="$HEADROOM_MD_CACHE" "$HEADROOM_PYTHON" -c 'import json, os, sys, subprocess, hashlib
ALLOWED = {{".pdf"}}
MAX_BYTES = 25 * 1024 * 1024
try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
tool_input = data.get("tool_input")
if not isinstance(tool_input, dict):
    sys.exit(0)
fp = tool_input.get("file_path")
if not isinstance(fp, str) or not fp:
    sys.exit(0)
if os.path.splitext(fp)[1].lower() not in ALLOWED:
    sys.exit(0)
try:
    st = os.stat(fp)
except OSError:
    sys.exit(0)
if st.st_size > MAX_BYTES:
    sys.exit(0)
binpath = os.environ["HEADROOM_MARKITDOWN_BIN"]
cache = os.environ["HEADROOM_MD_CACHE"]
key = hashlib.sha256((os.path.abspath(fp) + ":" + str(st.st_mtime_ns)).encode()).hexdigest()[:16]
out = os.path.join(cache, key + ".md")
if not (os.path.exists(out) and os.path.getsize(out) > 0):
    try:
        subprocess.run([binpath, fp, "-o", out], check=True, capture_output=True, timeout=120)
    except Exception:
        sys.exit(0)
if not (os.path.exists(out) and os.path.getsize(out) > 0):
    sys.exit(0)
updated = dict(tool_input)
updated["file_path"] = out
json.dump({{"hookSpecificOutput": {{"hookEventName": "PreToolUse", "permissionDecision": "allow", "permissionDecisionReason": "Headroom MarkItDown conversion", "updatedInput": updated}}}}, sys.stdout)' <<<"$INPUT" 2>/dev/null || exit 0
"#
    )
}

fn shell_double_quote(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

fn build_headroom_rtk_hook(managed_rtk_path: &Path, managed_python_path: &Path) -> String {
    let rtk = shell_double_quote(&managed_rtk_path.to_string_lossy());
    let python = shell_double_quote(&managed_python_path.to_string_lossy());

    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

HEADROOM_RTK="{rtk}"
HEADROOM_PYTHON="{python}"

if [ ! -x "$HEADROOM_RTK" ] || [ ! -x "$HEADROOM_PYTHON" ]; then
  exit 0
fi

INPUT="$(cat)"
if [ -z "$INPUT" ]; then
  exit 0
fi

CMD="$("$HEADROOM_PYTHON" -c 'import json, sys; data = json.load(sys.stdin); cmd = data.get("tool_input", {{}}).get("command", ""); print(cmd if isinstance(cmd, str) else "")' <<<"$INPUT" 2>/dev/null || true)"
if [ -z "$CMD" ]; then
  exit 0
fi

REWRITTEN="$("$HEADROOM_RTK" rewrite "$CMD" 2>/dev/null || true)"
if [ -z "$REWRITTEN" ] || [ "$CMD" = "$REWRITTEN" ]; then
  exit 0
fi

# `rtk rewrite` emits a bare `rtk` leading token, which only resolves if the
# managed PATH export has propagated into this session's environment. GUI apps
# (VSCode, terminals) launched before rtk was enabled inherit a stale PATH, so
# `rtk` is missing and the rewrite would fail with "command not found". Pin the
# leading token to the managed binary's absolute path so it works regardless.
if [ "${{REWRITTEN%% *}}" = "rtk" ]; then
  REWRITTEN="$HEADROOM_RTK${{REWRITTEN#rtk}}"
fi

# Defense-in-depth: if the rewritten command's first token isn't resolvable
# (e.g. a partial uninstall left `rtk` missing from PATH), fall through to the
# original command instead of handing Claude Code a command that will fail with
# "command not found".
FIRST_TOKEN="${{REWRITTEN%% *}}"
case "$FIRST_TOKEN" in
  /*)
    [ -x "$FIRST_TOKEN" ] || exit 0
    ;;
  *)
    command -v "$FIRST_TOKEN" >/dev/null 2>&1 || exit 0
    ;;
esac

HEADROOM_RTK_REWRITTEN="$REWRITTEN" "$HEADROOM_PYTHON" -c 'import json, os, sys; data = json.load(sys.stdin); tool_input = data.get("tool_input"); 
if not isinstance(tool_input, dict):
    sys.exit(0)
updated = dict(tool_input)
updated["command"] = os.environ["HEADROOM_RTK_REWRITTEN"]
json.dump({{"hookSpecificOutput": {{"hookEventName": "PreToolUse", "permissionDecision": "allow", "permissionDecisionReason": "Headroom RTK auto-rewrite", "updatedInput": updated}}}}, sys.stdout)' <<<"$INPUT" 2>/dev/null || exit 0
"#
    )
}

/// Codex's home directory. Mirrors the Codex CLI and the upstream Headroom
/// proxy: honor `$CODEX_HOME` when set, else `~/.codex`. Staying in sync with
/// the proxy matters — if the two layers disagree on where Codex lives, the
/// provider retag rewrites a different store than the config it edited.
pub(crate) fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".codex"))
}

fn detect_claude_code_client(configured: bool) -> ClientStatus {
    let executable = claude_code_candidate_paths()
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["claude", "claude-code"]));

    if let Some(path) = executable {
        return ClientStatus {
            id: "claude_code".into(),
            name: "Claude Code".into(),
            installed: true,
            configured,
            health: if configured {
                ClientHealth::Healthy
            } else {
                ClientHealth::Attention
            },
            notes: if configured {
                vec![
                    format!("Detected at {}", path.display()),
                    "Configured by Headroom.".into(),
                ]
            } else {
                vec![
                    format!("Detected at {}", path.display()),
                    "Route Claude Code through Headroom's localhost proxy so prompts stay lean."
                        .into(),
                ]
            },
        };
    }

    if claude_code_user_state_exists(&home_dir()) {
        return ClientStatus {
            id: "claude_code".into(),
            name: "Claude Code".into(),
            installed: true,
            configured,
            health: if configured {
                ClientHealth::Healthy
            } else {
                ClientHealth::Attention
            },
            notes: if configured {
                vec![
                    "Detected Claude Code data in ~/.claude.".into(),
                    "Configured by Headroom.".into(),
                ]
            } else {
                vec![
                    "Detected Claude Code data in ~/.claude.".into(),
                    "Claude Code appears to be installed, but Headroom could not resolve the CLI from its current launch PATH. This is common when Headroom starts outside your shell and Claude was installed via nvm or another user-local toolchain.".into(),
                ]
            },
        };
    }

    ClientStatus {
        id: "claude_code".into(),
        name: "Claude Code".into(),
        installed: false,
        configured: false,
        health: ClientHealth::NotDetected,
        notes: vec!["Not detected on this machine yet.".into()],
    }
}

fn claude_code_candidate_paths() -> Vec<PathBuf> {
    let home = home_dir();
    let binary_names = ["claude", "claude-code"];
    let mut candidates = vec![
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude-code"),
        PathBuf::from("/opt/homebrew/bin/claude-code"),
    ];

    let user_bin_dirs = vec![
        home.join(".local").join("bin"),
        home.join("bin"),
        home.join(".npm-global").join("bin"),
        home.join(".yarn").join("bin"),
        home.join(".config")
            .join("yarn")
            .join("global")
            .join("node_modules")
            .join(".bin"),
        home.join(".volta").join("bin"),
        home.join(".bun").join("bin"),
        home.join(".asdf").join("shims"),
        home.join(".mise").join("shims"),
        home.join(".nodenv").join("shims"),
    ];

    candidates.extend(binary_candidates_in_dirs(&user_bin_dirs, &binary_names));
    candidates.extend(nvm_binary_candidates(&home, &binary_names));
    dedupe_paths(candidates)
}

fn binary_candidates_in_dirs(directories: &[PathBuf], binary_names: &[&str]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for directory in directories {
        for binary_name in binary_names {
            candidates.push(directory.join(binary_name));
            if cfg!(windows) {
                for ext in windows_path_extensions() {
                    candidates.push(directory.join(format!("{binary_name}{ext}")));
                }
            }
        }
    }
    candidates
}

fn nvm_binary_candidates(home: &Path, binary_names: &[&str]) -> Vec<PathBuf> {
    let mut candidates = binary_candidates_in_dirs(
        &[home.join(".nvm").join("current").join("bin")],
        binary_names,
    );
    let versions_dir = home.join(".nvm").join("versions").join("node");
    let Ok(entries) = std::fs::read_dir(versions_dir) else {
        return candidates;
    };

    let mut version_bins = entries
        .flatten()
        .map(|entry| entry.path().join("bin"))
        .collect::<Vec<_>>();
    version_bins.sort();
    version_bins.reverse();
    candidates.extend(binary_candidates_in_dirs(&version_bins, binary_names));
    candidates
}

fn claude_code_user_state_exists(home: &Path) -> bool {
    let claude_root = home.join(".claude");
    claude_root.join("settings.json").exists()
        || claude_root.join("projects").exists()
        || claude_root.join("sessions").exists()
        || claude_root.join("statsig").exists()
}

fn detect_codex_client(configured: bool) -> ClientStatus {
    let executable = cli_discovery::detect_codex_cli();

    let detected = executable
        .as_ref()
        .map(|path| format!("Detected at {}", path.display()))
        .or_else(|| {
            codex_user_state_exists()
                .then(|| format!("Detected Codex data in {}.", codex_home().display()))
        });

    if let Some(detected_note) = detected {
        return ClientStatus {
            id: "codex".into(),
            name: "Codex".into(),
            installed: true,
            configured,
            health: if configured {
                ClientHealth::Healthy
            } else {
                ClientHealth::Attention
            },
            notes: if configured {
                vec![detected_note, "Configured by Headroom.".into()]
            } else {
                vec![
                    detected_note,
                    "Route Codex through Headroom's localhost proxy so prompts stay lean.".into(),
                ]
            },
        };
    }

    ClientStatus {
        id: "codex".into(),
        name: "Codex".into(),
        installed: false,
        configured: false,
        health: ClientHealth::NotDetected,
        notes: vec!["Not detected on this machine yet.".into()],
    }
}

fn codex_user_state_exists() -> bool {
    let codex_root = codex_home();
    codex_root.join("config.toml").exists()
        || codex_root.join("auth.json").exists()
        || codex_root.join("sessions").exists()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedCliCompatibilityReport {
    label: &'static str,
    binary_path: Option<PathBuf>,
    version: Option<String>,
    config_surfaces: Vec<PathBuf>,
    routing_blocker: &'static str,
}

fn read_cli_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

fn planned_cli_compatibility_report(
    label: &'static str,
    binary_path: Option<PathBuf>,
    config_candidates: &[PathBuf],
    routing_blocker: &'static str,
) -> PlannedCliCompatibilityReport {
    let config_surfaces = config_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let version = binary_path.as_deref().and_then(read_cli_version);

    PlannedCliCompatibilityReport {
        label,
        binary_path,
        version,
        config_surfaces,
        routing_blocker,
    }
}

fn planned_cli_compatibility_evidence(report: &PlannedCliCompatibilityReport) -> Vec<String> {
    let mut evidence = Vec::new();
    if let Some(path) = &report.binary_path {
        evidence.push(format!("{} binary: {}", report.label, path.display()));
    }
    evidence.push(match &report.version {
        Some(version) => format!("{} version: {version}", report.label),
        None => format!("{} version: unavailable from --version.", report.label),
    });
    if report.config_surfaces.is_empty() {
        evidence.push(format!(
            "{} config surface: none detected yet.",
            report.label
        ));
    } else {
        evidence.push(format!(
            "{} config surface: {}",
            report.label,
            report
                .config_surfaces
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    evidence.push(report.routing_blocker.to_string());
    evidence
}

/// Detect Gemini CLI without mutating config. The compatibility report is
/// surfaced as planned-connector evidence while routing remains manual.
fn detect_gemini_cli_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["gemini"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["gemini"]));
    let config_candidates = [
        home_dir().join(".gemini"),
        home_dir().join(".config").join("gemini"),
    ];
    let report = planned_cli_compatibility_report(
        "Gemini",
        executable.clone(),
        &config_candidates,
        "Managed shell/base-url routing uses Switchboard-owned shell blocks, sibling rollback backups, Doctor verification, rollback, and Off mode cleanup.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage Gemini CLI shell/base-url routing while keeping account and model choices user-owned."
                .into(),
        );
    }

    let mut status = ClientStatus {
        id: "gemini_cli".into(),
        name: "Gemini CLI".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    };
    append_gemini_manual_routing_note(&mut status);
    status
}

fn append_gemini_manual_routing_note(status: &mut ClientStatus) {
    if status.installed {
        status.notes.push(
            "Gemini routing is managed through reversible shell/base-url exports with backup, Doctor verification, rollback evidence, and Off mode cleanup."
                .into(),
        );
    }
}

fn detect_opencode_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["opencode", "open-code"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["opencode", "open-code"]));
    let config_candidates = [
        home_dir().join(".opencode"),
        home_dir().join(".config").join("opencode"),
    ];
    let report = planned_cli_compatibility_report(
        "OpenCode",
        executable.clone(),
        &config_candidates,
        "Managed provider routing uses the active OpenCode config path with backup, Doctor verification, rollback, and Off mode cleanup.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage OpenCode provider routing with backup, verification, rollback, and Off mode cleanup."
                .into(),
        );
    }

    ClientStatus {
        id: "opencode".into(),
        name: "OpenCode".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_cursor_client() -> ClientStatus {
    let app_path = PathBuf::from("/Applications/Cursor.app");
    let command_path = find_on_path(&["cursor"]);
    let profile_candidates = [home_dir()
        .join("Library")
        .join("Application Support")
        .join("Cursor")];
    let profile_surfaces = profile_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let settings_files = discover_editor_settings_files(&profile_surfaces);
    let native_schema = assess_native_schema(&home_dir());
    let installed = app_path.exists() || command_path.is_some() || !profile_surfaces.is_empty();
    let mut notes = if installed {
        let mut evidence = Vec::new();
        if app_path.exists() {
            evidence.push(format!("Cursor app: {}", app_path.display()));
        } else if let Some(path) = command_path {
            evidence.push(format!("Cursor app: command {}", path.display()));
        }
        if profile_surfaces.is_empty() {
            evidence.push("Cursor profile settings: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Cursor profile settings: {}",
                profile_surfaces
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if settings_files.is_empty() {
            evidence.push("Cursor settings files: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Cursor settings files: {}",
                settings_files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        evidence.push(
            "Cursor settings routing remains blocked because its provider schema is not allowlisted; only the isolated Switchboard sidecar can be applied with preview, exact consent, backup, verification, rollback, and Off cleanup.".into(),
        );
        evidence.extend(cursor_native_evidence(&native_schema));
        evidence
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Cursor provider/account/model settings remain manual; Switchboard can safely manage only its isolated routing-intent sidecar."
                .into(),
        );
    }

    ClientStatus {
        id: "cursor".into(),
        name: "Cursor".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_grok_cli_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["grok", "xai"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["grok", "xai"]));
    let config_candidates = [home_dir().join(".config").join("xai")];
    let report = planned_cli_compatibility_report(
        "Grok / xAI",
        executable.clone(),
        &config_candidates,
        "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can safely manage only its isolated routing-intent sidecar; xAI provider, model, credentials, and account settings remain manual."
                .into(),
        );
    }

    ClientStatus {
        id: "grok_cli".into(),
        name: "Grok / xAI CLI".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_aider_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["aider"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["aider"]));
    let config_candidates = [
        home_dir().join(".aider.conf.yml"),
        home_dir().join(".config").join("aider"),
    ];
    let report = planned_cli_compatibility_report(
        "Aider",
        executable.clone(),
        &config_candidates,
        "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while provider config remains manual.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage the Aider routing-intent sidecar while keeping provider config manual."
                .into(),
        );
    }

    ClientStatus {
        id: "aider".into(),
        name: "Aider".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_continue_client() -> ClientStatus {
    let command_path = find_on_path(&["continue"]);
    let config_candidates = [
        home_dir().join(".continue"),
        home_dir().join(".config").join("continue"),
    ];
    let config_surfaces = config_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let installed = command_path.is_some() || !config_surfaces.is_empty();
    let mut notes = if installed {
        let mut evidence = Vec::new();
        if let Some(path) = command_path {
            evidence.push(format!("Continue command: {}", path.display()));
        }
        if config_surfaces.is_empty() {
            evidence.push("Continue config folder: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Continue config folder: {}",
                config_surfaces
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        evidence.push(
            "Settings routing blocked until multi-provider parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist."
                .into(),
        );
        evidence
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
                "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while provider choices remain manual."
                    .into(),
            );
    }

    ClientStatus {
        id: "continue".into(),
        name: "Continue".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_goose_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["goose"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["goose"]));
    let config_candidates = [home_dir().join(".config").join("goose")];
    let report = planned_cli_compatibility_report(
        "Goose",
        executable.clone(),
        &config_candidates,
        "Managed Switchboard-owned routing-intent sidecar and read-only Repo Memory MCP bridge use exact confirmation, Doctor verification, rollback, and Off mode cleanup while provider routing remains manual.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage its isolated Goose routing-intent sidecar and read-only Repo Memory MCP bridge; provider, model, credentials, and account settings remain manual."
                .into(),
        );
    }

    ClientStatus {
        id: "goose".into(),
        name: "Goose".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_qwen_code_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["qwen", "qwen-code"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["qwen", "qwen-code"]));
    let config_candidates = [
        home_dir().join(".qwen"),
        home_dir().join(".config").join("qwen"),
    ];
    let report = planned_cli_compatibility_report(
        "Qwen Code",
        executable.clone(),
        &config_candidates,
        "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while model/account choices remain manual.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage the Qwen Code routing-intent sidecar while keeping model and account setup manual."
                .into(),
        );
    }

    ClientStatus {
        id: "qwen_code".into(),
        name: "Qwen Code".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_amazon_q_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["q"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["q"]));
    let config_candidates = [
        home_dir().join(".aws").join("amazonq"),
        home_dir().join(".config").join("amazon-q"),
    ];
    let report = planned_cli_compatibility_report(
        "Amazon Q",
        executable.clone(),
        &config_candidates,
        "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while AWS auth, provider, and workspace choices remain manual.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage the Amazon Q routing-intent sidecar while keeping AWS and Amazon Q account state manual."
                .into(),
        );
    }

    ClientStatus {
        id: "amazon_q".into(),
        name: "Amazon Q Developer CLI".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_windsurf_client() -> ClientStatus {
    let app_path = PathBuf::from("/Applications/Windsurf.app");
    let command_path = find_on_path(&["windsurf"]);
    let settings_candidates = [home_dir()
        .join("Library")
        .join("Application Support")
        .join("Windsurf")];
    let settings_surfaces = settings_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let settings_files = discover_editor_settings_files(&settings_surfaces);
    let installed = app_path.exists() || command_path.is_some() || !settings_surfaces.is_empty();
    let mut notes = if installed {
        let mut evidence = Vec::new();
        if app_path.exists() {
            evidence.push(format!("Windsurf app: {}", app_path.display()));
        } else if let Some(path) = command_path {
            evidence.push(format!("Windsurf app: command {}", path.display()));
        }
        if settings_surfaces.is_empty() {
            evidence.push("Windsurf settings: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Windsurf settings: {}",
                settings_surfaces
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if settings_files.is_empty() {
            evidence.push("Windsurf settings files: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Windsurf settings files: {}",
                settings_files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        evidence.push(
            "Managed Windsurf settings routing uses settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup."
                .into(),
        );
        evidence
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage Windsurf editor settings routing with backup, verification, rollback, and Off mode cleanup."
                .into(),
        );
    }

    ClientStatus {
        id: "windsurf".into(),
        name: "Windsurf".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_zed_ai_client() -> ClientStatus {
    let app_path = PathBuf::from("/Applications/Zed.app");
    let command_path = find_on_path(&["zed"]);
    let settings_candidates = [
        home_dir().join(".config").join("zed"),
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("Zed"),
    ];
    let settings_surfaces = settings_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let installed = app_path.exists() || command_path.is_some() || !settings_surfaces.is_empty();
    let mut notes = if installed {
        let mut evidence = Vec::new();
        if app_path.exists() {
            evidence.push(format!("Zed app: {}", app_path.display()));
        } else if let Some(path) = command_path {
            evidence.push(format!("Zed app: command {}", path.display()));
        }
        if settings_surfaces.is_empty() {
            evidence.push("Zed assistant settings: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Zed assistant settings: {}",
                settings_surfaces
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        evidence.push(
            "Managed Zed settings routing uses lossless settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup."
                .into(),
        );
        evidence
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage Zed assistant settings routing with backup, verification, rollback, and Off mode cleanup."
                .into(),
        );
    }

    ClientStatus {
        id: "zed_ai".into(),
        name: "Zed AI".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_planned_client(
    id: &str,
    name: &str,
    binary_names: &[&str],
    state_paths: &[PathBuf],
    planned_note: &str,
) -> ClientStatus {
    let executable = common_cli_candidate_paths(binary_names)
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(binary_names));
    let detected = executable
        .as_ref()
        .map(|path| format!("Detected at {}", path.display()))
        .or_else(|| {
            state_paths
                .iter()
                .find(|path| path.exists())
                .map(|path| format!("Detected data at {}.", path.display()))
        });

    if let Some(detected_note) = detected {
        return ClientStatus {
            id: id.into(),
            name: name.into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![detected_note, planned_note.into()],
        };
    }

    ClientStatus {
        id: id.into(),
        name: name.into(),
        installed: false,
        configured: false,
        health: ClientHealth::NotDetected,
        notes: vec!["Not detected on machine yet.".into()],
    }
}

fn common_cli_candidate_paths(binary_names: &[&str]) -> Vec<PathBuf> {
    let home = home_dir();
    let mut directories = vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
    ];
    directories.extend([
        home.join(".local").join("bin"),
        home.join(".cargo").join("bin"),
        home.join("bin"),
        home.join(".npm-global").join("bin"),
        home.join(".yarn").join("bin"),
        home.join(".volta").join("bin"),
        home.join(".bun").join("bin"),
        home.join(".asdf").join("shims"),
        home.join(".mise").join("shims"),
        home.join(".nodenv").join("shims"),
    ]);

    let mut paths = binary_candidates_in_dirs(&directories, binary_names);
    paths.extend(nvm_binary_candidates(&home, binary_names));
    dedupe_paths(paths)
}

pub(crate) fn detect_codex_cli() -> Option<PathBuf> {
    cli_discovery::detect_codex_cli()
}

/// True once the user has signed in to Codex with their ChatGPT account — the
/// OAuth token lands in `~/.codex/auth.json`. Required for the keyless
/// `codex exec` analysis backend.
pub(crate) fn codex_logged_in() -> bool {
    codex_home().join("auth.json").is_file()
}

fn discover_editor_settings_files(profile_roots: &[PathBuf]) -> Vec<PathBuf> {
    let relative_candidates = [
        PathBuf::from("User").join("settings.json"),
        PathBuf::from("User").join("settings.jsonc"),
        PathBuf::from("settings.json"),
        PathBuf::from("settings.jsonc"),
        PathBuf::from("profiles").join("User").join("settings.json"),
        PathBuf::from("profiles")
            .join("User")
            .join("settings.jsonc"),
    ];
    let mut candidates = Vec::new();
    for root in profile_roots {
        for relative in &relative_candidates {
            let path = root.join(relative);
            if path.is_file() {
                candidates.push(path);
            }
        }
        // Cursor profiles are stored beneath User/profiles/<profile-id>.  Only
        // inspect the well-known settings filenames: globalStorage and state
        // databases are intentionally never traversed or read.
        let profiles_dir = root.join("User").join("profiles");
        if let Ok(entries) = std::fs::read_dir(profiles_dir) {
            for entry in entries.flatten() {
                let profile = entry.path();
                if !profile.is_dir() {
                    continue;
                }
                for name in ["settings.json", "settings.jsonc"] {
                    let path = profile.join(name);
                    if path.is_file() {
                        candidates.push(path);
                    }
                }
            }
        }
    }
    dedupe_paths(candidates)
}

fn find_on_path(binary_names: &[&str]) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    find_on_path_entries(std::env::split_paths(&path_var), binary_names)
}

fn find_on_path_entries<I>(path_entries: I, binary_names: &[&str]) -> Option<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    for entry in path_entries {
        for binary_name in binary_names {
            let candidate = entry.join(binary_name);
            if candidate.exists() {
                return Some(candidate);
            }

            if cfg!(windows) {
                for ext in windows_path_extensions() {
                    let with_ext = entry.join(format!("{binary_name}{ext}"));
                    if with_ext.exists() {
                        return Some(with_ext);
                    }
                }
            }
        }
    }

    None
}

fn windows_path_extensions() -> Vec<String> {
    std::env::var_os("PATHEXT")
        .unwrap_or_else(|| OsStr::new(".COM;.EXE;.BAT;.CMD").to_os_string())
        .to_string_lossy()
        .split(';')
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value.starts_with('.') {
                value.to_string()
            } else {
                format!(".{value}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::client_connectors::{
        planned_connector_has_implemented_setup, CONNECTOR_MANIFEST_JSON, PLANNED_CLIENT_SPECS,
        PLANNED_CONFIG_CREATION_STEPS, PLANNED_CONFIG_CREATION_STEP_IDS,
    };
    use crate::client_paths::{
        grok_config_path, planned_sidecar_routing_path, SWITCHBOARD_ROUTING_FILE,
    };
    use crate::client_provider_configs::HEADROOM_OPENAI_BASE_URL;
    use crate::models::{
        ClientConnectorSupportStatus, ClientHealth, ClientStatus, CodexThreadRetaggingMode,
        CodexThreadRetaggingSettings, ManagedRollbackExecutionStatus, SwitchboardMode,
    };

    use super::{
        build_headroom_markitdown_hook, build_headroom_rtk_hook, build_markitdown_codex_nudge,
        build_markitdown_office_nudge, claude_code_user_state_exists, claude_hook_present_in_value,
        codex_home, default_shell_targets_for_family, entry_contains_hook, find_on_path_entries,
        list_client_connectors, normalize_setup_state, normalized_setup_id, nvm_binary_candidates,
        parse_json_object, planned_switchboard_sidecar_matches, remove_managed_block,
        remove_pre_tool_use_markers, serialize_paths, shell_block_contains_in_files,
        shell_block_contains_text_in_files, shell_double_quote, strip_headroom_hook_from_settings,
        upsert_managed_block, write_file_if_changed, ClientSetupState,
    };
    use crate::client_connector_status::MANAGED_CLIENT_SPECS;
    use crate::client_footprint;
    use crate::client_paths::{zed_config_path, OPENCODE_CONFIG_FILE};
    use rusqlite::Connection;

    #[test]
    fn normalize_setup_state_keeps_codex_but_drops_legacy_codex_gui() {
        let state = ClientSetupState {
            configured_clients: BTreeMap::from([
                ("claude_code".into(), "2026-03-27T10:00:00Z".into()),
                ("codex_cli".into(), "2026-03-27T10:01:00Z".into()),
                ("codex_gui".into(), "2026-03-27T10:02:00Z".into()),
            ]),
            remembered_clients: BTreeMap::from([
                ("codex".into(), "2026-03-27T10:03:00Z".into()),
                ("claude_code".into(), "2026-03-27T10:04:00Z".into()),
            ]),
            managed_shell_files: BTreeMap::from([
                ("claude_code".into(), vec!["/Users/test/.zprofile".into()]),
                ("codex_cli".into(), vec!["/Users/test/.zshrc".into()]),
                ("codex_gui".into(), vec!["/Users/test/.zshrc".into()]),
            ]),
            remembered_shell_files: BTreeMap::from([
                ("codex".into(), vec!["/Users/test/.bash_profile".into()]),
                ("claude_code".into(), vec!["/Users/test/.bashrc".into()]),
            ]),
            rtk_disabled: false,
            switchboard_mode: Some(SwitchboardMode::Full),
            savings_mode: None,
        };

        let normalized = normalize_setup_state(state);

        // codex_cli stays configured; only the removed codex_gui id is stripped.
        assert!(normalized.configured_clients.contains_key("claude_code"));
        assert!(normalized.configured_clients.contains_key("codex_cli"));
        assert!(!normalized.configured_clients.contains_key("codex_gui"));

        assert!(normalized.remembered_clients.contains_key("claude_code"));
        assert!(normalized.remembered_clients.contains_key("codex"));
        assert_eq!(normalized.switchboard_mode, Some(SwitchboardMode::Full));

        assert!(normalized.managed_shell_files.contains_key("claude_code"));
        assert!(normalized.managed_shell_files.contains_key("codex_cli"));
        assert!(!normalized.managed_shell_files.contains_key("codex_gui"));

        assert!(normalized
            .remembered_shell_files
            .contains_key("claude_code"));
        assert!(normalized.remembered_shell_files.contains_key("codex"));
    }

    #[test]
    fn planned_connector_registry_tracks_popular_agent_tools() {
        let ids = PLANNED_CLIENT_SPECS
            .iter()
            .map(|spec| spec.id)
            .collect::<BTreeSet<_>>();

        assert_eq!(
            ids,
            BTreeSet::from([
                "aider",
                "amazon_q",
                "continue",
                "cursor",
                "gemini_cli",
                "goose",
                "grok_cli",
                "opencode",
                "qwen_code",
                "windsurf",
                "zed_ai",
            ])
        );
    }

    #[test]
    fn connector_registry_uses_manifest_owned_identity_and_status() {
        let detected_clients = vec![
            ClientStatus {
                id: "claude_code".into(),
                name: "Claude Code".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Healthy,
                notes: vec!["Claude config present".into()],
            },
            ClientStatus {
                id: "gemini_cli".into(),
                name: "Gemini CLI".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec!["Gemini binary present".into()],
            },
        ];
        let connectors = list_client_connectors(&detected_clients).expect("list connectors");
        let manifests = serde_json::from_str::<Vec<serde_json::Value>>(CONNECTOR_MANIFEST_JSON)
            .expect("valid connector manifest");
        let rust_ids = MANAGED_CLIENT_SPECS
            .iter()
            .map(|spec| spec.id)
            .chain(PLANNED_CLIENT_SPECS.iter().map(|spec| spec.id))
            .collect::<BTreeSet<_>>();

        for manifest in manifests {
            let id = manifest["id"].as_str().expect("manifest id");
            assert!(rust_ids.contains(id), "{id} missing from Rust registry");
            let connector = connectors
                .iter()
                .find(|connector| connector.client_id == id)
                .unwrap_or_else(|| panic!("{id} missing from connector status"));
            assert_eq!(connector.name, manifest["name"].as_str().unwrap());
            assert_eq!(connector.category, manifest["category"].as_str().unwrap());
            let expected_status = match manifest["support_status"].as_str().unwrap() {
                "managed" => ClientConnectorSupportStatus::Managed,
                _ => ClientConnectorSupportStatus::Planned,
            };
            assert_eq!(connector.support_status, expected_status);
        }
    }

    #[test]
    fn manifest_managed_connectors_have_implemented_setup_paths() {
        let manifests = serde_json::from_str::<Vec<serde_json::Value>>(CONNECTOR_MANIFEST_JSON)
            .expect("valid connector manifest");
        let managed_ids = manifests
            .iter()
            .filter(|manifest| manifest["support_status"].as_str() == Some("managed"))
            .map(|manifest| manifest["id"].as_str().expect("manifest id"))
            .collect::<BTreeSet<_>>();

        assert_eq!(
            managed_ids,
            BTreeSet::from([
                "aider",
                "continue",
                "claude_code",
                "codex",
                "gemini_cli",
                "goose",
                "grok_cli",
                "opencode",
                "qwen_code",
                "amazon_q",
                "windsurf",
                "zed_ai",
            ])
        );

        for id in managed_ids {
            if id == "goose" {
                continue;
            }
            let native_managed = MANAGED_CLIENT_SPECS.iter().any(|spec| spec.id == id);
            let promoted_planned = planned_connector_has_implemented_setup(id);
            assert!(
                native_managed || promoted_planned,
                "{id} is manifest-managed but has no apply/verify/repair setup path"
            );
        }
    }

    #[test]
    fn planned_connector_registry_includes_backend_detection_metadata() {
        for spec in PLANNED_CLIENT_SPECS {
            assert!(matches!(spec.category, "cli" | "editor" | "agent"));
            assert!(matches!(
                spec.setup_phase,
                "detect" | "guide" | "adapt" | "managed" | "managed mcp"
            ));
            assert!(
                !spec.detection_sources.is_empty(),
                "{} should have detection sources",
                spec.id
            );
            assert!(
                !spec.config_locations.is_empty(),
                "{} should have config locations",
                spec.id
            );
            if planned_connector_has_implemented_setup(spec.id) {
                assert!(
                    spec.setup_hint.contains("Managed")
                        || (spec.id == "goose" && spec.setup_hint.contains("Managed MCP")),
                    "{} should describe its managed setup lifecycle",
                    spec.id
                );
            } else {
                assert!(
                    spec.setup_hint.contains("Manual guide")
                        || spec.setup_hint.contains("Detection only"),
                    "{} should stay manual until reversible adapters exist",
                    spec.id
                );
            }
        }
        let gemini = PLANNED_CLIENT_SPECS
            .iter()
            .find(|spec| spec.id == "gemini_cli")
            .expect("Gemini spec");
        let gemini_copy = format!(
            "{} {}",
            gemini.setup_hint,
            gemini.automation_gates.join(" ")
        );
        assert!(gemini_copy.contains("sibling rollback backups"));
        assert!(!gemini_copy.contains("sidecar evidence"));
    }

    #[test]
    fn editor_settings_discovery_finds_user_settings_without_writing() {
        let root = unique_temp_dir("editor-settings-discovery");
        let cursor_root = root.join("Cursor");
        let windsurf_root = root.join("Windsurf");
        fs::create_dir_all(cursor_root.join("User")).expect("create cursor user");
        fs::create_dir_all(windsurf_root.join("profiles").join("User"))
            .expect("create windsurf profile");
        let cursor_settings = cursor_root.join("User").join("settings.json");
        let windsurf_settings = windsurf_root
            .join("profiles")
            .join("User")
            .join("settings.jsonc");
        fs::write(&cursor_settings, "{}").expect("write cursor settings");
        fs::write(&windsurf_settings, "{}").expect("write windsurf settings");

        let discovered =
            super::discover_editor_settings_files(&[cursor_root.clone(), windsurf_root.clone()]);

        assert!(discovered.contains(&cursor_settings));
        assert!(discovered.contains(&windsurf_settings));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[serial_test::serial]
    fn planned_connectors_are_detected_but_not_enabled_or_verified() {
        let _home = TestHome::new();
        let detected_clients = vec![
        ClientStatus {
            id: "gemini_cli".into(),
            name: "Gemini CLI".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![
                "Gemini binary: /opt/homebrew/bin/gemini".into(),
                "Gemini version: gemini 0.2.1".into(),
                "Gemini config surface: /Users/test/.gemini".into(),
                "Provider routing blocked until stable config surface, backup, verify, rollback, and Off mode cleanup exist.".into(),
            ],
        },
            ClientStatus {
                id: "opencode".into(),
                name: "OpenCode".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "OpenCode binary: /opt/homebrew/bin/opencode".into(),
                    "OpenCode version: opencode 1.0.0".into(),
                    "OpenCode config surface: /Users/test/.config/opencode".into(),
                    "Provider routing blocked until active config path, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "grok_cli".into(),
                name: "Grok / xAI CLI".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Grok / xAI binary: /opt/homebrew/bin/xai".into(),
                    "Grok / xAI version: xai 0.4.0".into(),
                    "Grok / xAI config surface: /Users/test/.config/xai".into(),
                    "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "cursor".into(),
                name: "Cursor".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Cursor app: /Applications/Cursor.app".into(),
                    "Cursor profile settings: /Users/test/Library/Application Support/Cursor".into(),
                    "Settings routing blocked until profile settings parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "aider".into(),
                name: "Aider".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Aider binary: /opt/homebrew/bin/aider".into(),
                    "Aider version: aider 0.84.0".into(),
                    "Aider config surface: /Users/test/.aider.conf.yml".into(),
                    "Provider routing blocked until reversible environment wrapper, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "continue".into(),
                name: "Continue".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Continue command: /opt/homebrew/bin/continue".into(),
                    "Continue config folder: /Users/test/.continue".into(),
                    "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while provider choices remain manual.".into(),
                ],
            },
            ClientStatus {
                id: "goose".into(),
                name: "Goose".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Goose binary: /opt/homebrew/bin/goose".into(),
                    "Goose version: goose 1.2.0".into(),
                    "Goose config surface: /Users/test/.config/goose".into(),
                    "Provider routing blocked until MCP handoff shape, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "qwen_code".into(),
                name: "Qwen Code".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Qwen Code binary: /opt/homebrew/bin/qwen-code".into(),
                    "Qwen Code version: qwen-code 0.9.0".into(),
                    "Qwen Code config surface: /Users/test/.qwen".into(),
                    "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while model/account choices remain manual.".into(),
                ],
            },
            ClientStatus {
                id: "amazon_q".into(),
                name: "Amazon Q Developer CLI".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Amazon Q binary: /opt/homebrew/bin/q".into(),
                    "Amazon Q version: q 1.11.0".into(),
                    "Amazon Q config surface: /Users/test/.aws/amazonq".into(),
                    "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while AWS auth, provider, and workspace choices remain manual.".into(),
                ],
            },
            ClientStatus {
                id: "windsurf".into(),
                name: "Windsurf".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Windsurf app: /Applications/Windsurf.app".into(),
                    "Windsurf settings: /Users/test/Library/Application Support/Windsurf"
                        .into(),
                    "Managed Windsurf settings routing uses settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup.".into(),
                ],
            },
            ClientStatus {
                id: "zed_ai".into(),
                name: "Zed AI".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Zed app: /Applications/Zed.app".into(),
                    "Zed assistant settings: /Users/test/.config/zed".into(),
                    "Managed Zed settings routing uses lossless settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup.".into(),
                ],
            },
        ];

        let connectors = list_client_connectors(&detected_clients).expect("list connectors");
        let planned = connectors
            .iter()
            .filter(|connector| connector.support_status == ClientConnectorSupportStatus::Planned)
            .collect::<Vec<_>>();

        assert_eq!(
            planned
                .iter()
                .map(|connector| connector.client_id.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["cursor"])
        );

        for connector in planned {
            assert!(!connector.enabled);
            assert!(!connector.verified);
            assert_eq!(connector.last_configured_at, None);
            assert!(!connector.category.is_empty());
            assert!(!connector.detection_sources.is_empty());
            assert!(!connector.detection_evidence.is_empty());
            assert!(!connector.config_locations.is_empty());
            assert_eq!(
                connector.config_creation_steps,
                PLANNED_CONFIG_CREATION_STEPS
                    .iter()
                    .map(|step| step.to_string())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                connector
                    .config_creation_step_details
                    .iter()
                    .map(|step| step.id.as_str())
                    .collect::<Vec<_>>(),
                PLANNED_CONFIG_CREATION_STEP_IDS
            );
            assert!(connector
                .config_creation_step_details
                .iter()
                .all(|step| !step.label.is_empty()
                    && step.detail.len() > 30
                    && step.required_evidence.len() >= 2
                    && step
                        .required_evidence
                        .iter()
                        .all(|evidence| evidence.len() > 30)));
            let dry_run = connector
                .config_creation_step_details
                .iter()
                .find(|step| step.id == "dryRunDiff")
                .expect("gated connector dry-run step");
            let dry_run_copy =
                format!("{} {}", dry_run.detail, dry_run.required_evidence.join(" "));
            for snippet in [
                "target path",
                "before/after",
                "managed marker boundary",
                "rollback preview",
                "confirmation phrase",
            ] {
                assert!(dry_run_copy.contains(snippet));
            }
            let preview = connector
                .config_dry_run_preview
                .as_ref()
                .expect("gated connector dry-run preview");
            assert_eq!(
                preview.marker,
                format!("mac-ai-switchboard:{}", connector.client_id)
            );
            assert!(preview.backup_path.ends_with(".mac-ai-switchboard.bak"));
            assert!(preview.current_state.contains(&connector.name));
            assert!(preview.proposed_state.contains("Preview only"));
            assert!(preview.proposed_state.contains("no files are written"));
            assert!(preview.apply_blocked_reason.contains(&connector.name));
            if connector.client_id == "cursor" {
                assert!(preview
                    .apply_blocked_reason
                    .contains("does not document a stable on-disk"));
            } else {
                assert!(preview
                    .apply_blocked_reason
                    .contains("backup, verify, rollback, and Off cleanup"));
            }
            if connector.client_id == "cursor" {
                assert!(preview.rollback_preview.contains("No Cursor native write"));
                assert_eq!(preview.confirmation_phrase, "CURSOR NATIVE SCHEMA GATE");
            } else {
                assert!(preview.rollback_preview.contains("remove only"));
                assert_eq!(
                    preview.confirmation_phrase,
                    format!("APPLY {} CONFIG", connector.name.to_uppercase())
                );
            }
            assert!(preview.writes.is_empty());
            assert_eq!(connector.automation_path.len(), 7);
            assert_eq!(
                connector
                    .automation_path
                    .iter()
                    .map(|stage| stage.id.as_str())
                    .collect::<Vec<_>>(),
                PLANNED_CONFIG_CREATION_STEP_IDS
            );
            assert_eq!(connector.automation_path[0].status, "ready");
            assert_eq!(connector.automation_path[1].status, "ready");
            assert!(connector
                .automation_path
                .iter()
                .skip(2)
                .all(|stage| stage.status == "blocked"));
            assert!(connector.automation_path[1]
                .evidence
                .contains(&preview.confirmation_phrase));
        }

        let gemini = connectors
            .iter()
            .find(|connector| connector.client_id == "gemini_cli")
            .expect("gemini connector");
        assert_eq!(gemini.support_status, ClientConnectorSupportStatus::Managed);
        assert_eq!(gemini.setup_phase, "managed");
        assert!(gemini.config_creation_steps.is_empty());
        assert!(gemini.config_creation_step_details.is_empty());
        assert!(gemini.config_dry_run_preview.is_none());
        assert!(gemini.automation_path.is_empty());
        assert!(!gemini.enabled);
        assert!(!gemini.verified);

        let opencode = connectors
            .iter()
            .find(|connector| connector.client_id == "opencode")
            .expect("opencode connector");
        assert_eq!(
            opencode.support_status,
            ClientConnectorSupportStatus::Managed
        );
        assert_eq!(opencode.setup_phase, "managed");
        assert!(opencode.config_creation_steps.is_empty());
        assert!(opencode.config_creation_step_details.is_empty());
        assert!(opencode.config_dry_run_preview.is_none());
        assert!(opencode.automation_path.is_empty());
        assert!(!opencode.enabled);
        assert!(!opencode.verified);

        let managed = connectors
            .iter()
            .filter(|connector| connector.support_status == ClientConnectorSupportStatus::Managed)
            .collect::<Vec<_>>();
        assert!(managed
            .iter()
            .all(|connector| connector.config_creation_steps.is_empty()
                && connector.config_creation_step_details.is_empty()));

        assert!(connectors.iter().any(|connector| {
            connector.client_id == "gemini_cli"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Gemini binary: /opt/homebrew/bin/gemini".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Gemini version: gemini 0.2.1".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "opencode"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"OpenCode binary: /opt/homebrew/bin/opencode".to_string())
                && connector
                    .detection_evidence
                    .contains(&"OpenCode version: opencode 1.0.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "grok_cli"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Grok / xAI binary: /opt/homebrew/bin/xai".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Grok / xAI version: xai 0.4.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "cursor"
                && connector.support_status == ClientConnectorSupportStatus::Planned
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Cursor app: /Applications/Cursor.app".to_string())
                && connector.detection_evidence.contains(
                    &"Settings routing blocked until profile settings parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist."
                        .to_string()
                )
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "aider"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Aider binary: /opt/homebrew/bin/aider".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Aider version: aider 0.84.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "continue"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Continue command: /opt/homebrew/bin/continue".to_string())
                && connector.detection_evidence.contains(
                    &"Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while provider choices remain manual."
                        .to_string()
                )
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "goose"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Goose binary: /opt/homebrew/bin/goose".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Goose version: goose 1.2.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "qwen_code"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Qwen Code binary: /opt/homebrew/bin/qwen-code".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Qwen Code version: qwen-code 0.9.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "amazon_q"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Amazon Q binary: /opt/homebrew/bin/q".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Amazon Q version: q 1.11.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "windsurf"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Windsurf app: /Applications/Windsurf.app".to_string())
                && connector.detection_evidence.contains(
                    &"Managed Windsurf settings routing uses settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup."
                        .to_string()
                )
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "zed_ai"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Zed app: /Applications/Zed.app".to_string())
                && connector.detection_evidence.contains(
                    &"Managed Zed settings routing uses lossless settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup."
                        .to_string()
                )
        }));
    }

    #[test]
    fn gemini_compatibility_evidence_reports_version_config_and_managed_routing() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Gemini",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/gemini")),
            version: Some("gemini 0.2.1".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.gemini")],
            routing_blocker:
                "Managed shell/base-url routing uses Switchboard-owned shell blocks, sibling rollback backups, Doctor verification, rollback, and Off mode cleanup.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Gemini binary: /opt/homebrew/bin/gemini"));
        assert!(evidence.contains("Gemini version: gemini 0.2.1"));
        assert!(evidence.contains("Gemini config surface: /Users/test/.gemini"));
        assert!(evidence.contains("Managed shell/base-url routing"));
        assert!(evidence.contains("sibling rollback backups"));
        assert!(!evidence.contains("sidecar evidence"));
        assert!(evidence.contains("Doctor verification"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    fn opencode_compatibility_evidence_reports_version_config_and_managed_routing() {
        let report = super::PlannedCliCompatibilityReport {
            label: "OpenCode",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/opencode")),
            version: Some("opencode 1.0.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.config/opencode")],
            routing_blocker:
                "Managed provider routing uses the active OpenCode config path with backup, Doctor verification, rollback, and Off mode cleanup.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("OpenCode binary: /opt/homebrew/bin/opencode"));
        assert!(evidence.contains("OpenCode version: opencode 1.0.0"));
        assert!(evidence.contains("OpenCode config surface: /Users/test/.config/opencode"));
        assert!(evidence.contains("Managed provider routing"));
        assert!(evidence.contains("active OpenCode config path"));
        assert!(evidence.contains("Doctor verification"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    fn grok_compatibility_evidence_reports_model_account_blocker() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Grok / xAI",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/xai")),
            version: Some("xai 0.4.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.config/xai")],
            routing_blocker:
                "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Grok / xAI binary: /opt/homebrew/bin/xai"));
        assert!(evidence.contains("Grok / xAI version: xai 0.4.0"));
        assert!(evidence.contains("Grok / xAI config surface: /Users/test/.config/xai"));
        assert!(evidence.contains("model/account guardrails"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("verify"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    fn aider_compatibility_evidence_reports_environment_wrapper_blocker() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Aider",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/aider")),
            version: Some("aider 0.84.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.aider.conf.yml")],
            routing_blocker:
                "Provider routing blocked until reversible environment wrapper, backup, verify, rollback, and Off mode cleanup exist.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Aider binary: /opt/homebrew/bin/aider"));
        assert!(evidence.contains("Aider version: aider 0.84.0"));
        assert!(evidence.contains("Aider config surface: /Users/test/.aider.conf.yml"));
        assert!(evidence.contains("reversible environment wrapper"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("verify"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    fn goose_compatibility_evidence_reports_mcp_handoff_blocker() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Goose",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/goose")),
            version: Some("goose 1.2.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.config/goose")],
            routing_blocker:
                "Provider routing blocked until MCP handoff shape, backup, verify, rollback, and Off mode cleanup exist.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Goose binary: /opt/homebrew/bin/goose"));
        assert!(evidence.contains("Goose version: goose 1.2.0"));
        assert!(evidence.contains("Goose config surface: /Users/test/.config/goose"));
        assert!(evidence.contains("MCP handoff shape"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("verify"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    fn qwen_compatibility_evidence_reports_managed_sidecar_lifecycle() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Qwen Code",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/qwen-code")),
            version: Some("qwen-code 0.9.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.qwen")],
            routing_blocker:
                "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while model/account choices remain manual.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Qwen Code binary: /opt/homebrew/bin/qwen-code"));
        assert!(evidence.contains("Qwen Code version: qwen-code 0.9.0"));
        assert!(evidence.contains("Qwen Code config surface: /Users/test/.qwen"));
        assert!(evidence.contains("Managed sidecar routing-intent setup"));
        assert!(evidence.contains("Switchboard-owned config marker"));
        assert!(evidence.contains("Doctor verification"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    fn amazon_q_compatibility_evidence_reports_managed_sidecar_lifecycle() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Amazon Q",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/q")),
            version: Some("q 1.11.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.aws/amazonq")],
            routing_blocker:
                "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while AWS auth, provider, and workspace choices remain manual.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Amazon Q binary: /opt/homebrew/bin/q"));
        assert!(evidence.contains("Amazon Q version: q 1.11.0"));
        assert!(evidence.contains("Amazon Q config surface: /Users/test/.aws/amazonq"));
        assert!(evidence.contains("Managed sidecar routing-intent setup"));
        assert!(evidence.contains("Switchboard-owned config marker"));
        assert!(evidence.contains("Doctor verification"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    fn gemini_detection_reports_managed_routing_lifecycle() {
        let mut status = ClientStatus {
            id: "gemini_cli".into(),
            name: "Gemini CLI".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec!["Detected at /opt/homebrew/bin/gemini".into()],
        };

        super::append_gemini_manual_routing_note(&mut status);

        let notes = status.notes.join(" ");
        assert!(notes.contains("Gemini routing is managed"));
        assert!(notes.contains("reversible shell/base-url exports"));
        assert!(notes.contains("Doctor verification"));
        assert!(notes.contains("backup"));
        assert!(notes.contains("rollback"));
        assert!(notes.contains("Off mode cleanup"));
    }

    #[test]
    fn parse_json_object_accepts_json5_but_rejects_non_objects() {
        let parsed = parse_json_object(
            "{ unquoted: 'value', trailing: true, }",
            Path::new("settings.json"),
        )
        .expect("json5 object should parse");
        assert_eq!(
            parsed.get("unquoted").and_then(|value| value.as_str()),
            Some("value")
        );
        assert_eq!(
            parsed.get("trailing").and_then(|value| value.as_bool()),
            Some(true)
        );

        let err =
            parse_json_object("[]", Path::new("settings.json")).expect_err("arrays are rejected");
        assert!(err
            .to_string()
            .contains("must contain a top-level JSON object"));
    }

    #[test]
    fn setup_aliases_map_to_current_primary_ids() {
        assert_eq!(normalized_setup_id("codex"), "codex_cli");
        assert_eq!(normalized_setup_id("codex_gui"), "codex_cli");
        assert_eq!(normalized_setup_id("vscode"), "claude_code");
        assert_eq!(normalized_setup_id("claude_code"), "claude_code");
    }

    #[test]
    fn shell_double_quote_escapes_shell_sensitive_characters() {
        let escaped = shell_double_quote("path with spaces/$HOME/\"quoted\"`cmd`\\tail");
        assert_eq!(
            escaped,
            "path with spaces/\\$HOME/\\\"quoted\\\"\\`cmd\\`\\\\tail"
        );
    }

    #[test]
    fn shell_targets_include_profile_and_rc_for_supported_shells() {
        let zsh_targets = default_shell_targets_for_family(crate::client_paths::ShellFamily::Zsh);
        let bash_targets = default_shell_targets_for_family(crate::client_paths::ShellFamily::Bash);

        assert!(zsh_targets.iter().any(|path| path.ends_with(".zprofile")));
        assert!(zsh_targets.iter().any(|path| path.ends_with(".zshrc")));
        assert!(bash_targets.iter().any(|path| {
            path.ends_with(".bash_profile")
                || path.ends_with(".bash_login")
                || path.ends_with(".profile")
        }));
        assert!(bash_targets.iter().any(|path| path.ends_with(".bashrc")));
    }

    #[test]
    fn serialize_paths_dedupes_repeated_entries() {
        let serialized = serialize_paths(&[
            PathBuf::from("/Users/test/.zprofile"),
            PathBuf::from("/Users/test/.zprofile"),
            PathBuf::from("/Users/test/.zshrc"),
        ]);

        assert_eq!(
            serialized,
            vec![
                "/Users/test/.zprofile".to_string(),
                "/Users/test/.zshrc".to_string()
            ]
        );
    }

    #[test]
    fn generated_rtk_hook_uses_escaped_paths_and_rewrite_reason() {
        let hook = build_headroom_rtk_hook(
            Path::new("/tmp/head room/bin/rtk"),
            Path::new("/tmp/head room/runtime/$python"),
        );

        assert!(hook.contains("HEADROOM_RTK=\"/tmp/head room/bin/rtk\""));
        assert!(hook.contains("HEADROOM_PYTHON=\"/tmp/head room/runtime/\\$python\""));
        assert!(hook.contains("Headroom RTK auto-rewrite"));
        assert!(hook.contains("\"updatedInput\": updated"));
    }

    #[test]
    fn generated_markitdown_hook_escapes_paths_and_redirects_read() {
        let hook = build_headroom_markitdown_hook(
            Path::new("/tmp/head room/venv/bin/markitdown"),
            Path::new("/tmp/head room/venv/bin/$python"),
        );

        assert!(hook.contains("HEADROOM_MARKITDOWN=\"/tmp/head room/venv/bin/markitdown\""));
        assert!(hook.contains("HEADROOM_PYTHON=\"/tmp/head room/venv/bin/\\$python\""));
        // Scoped to PDF only (Office is handled by the nudge, not the hook),
        // redirects via updatedInput, and fails open.
        assert!(hook.contains("ALLOWED = {\".pdf\"}"));
        assert!(!hook.contains(".docx"));
        assert!(hook.contains("updated[\"file_path\"] = out"));
        assert!(hook.contains("\"updatedInput\": updated"));
        assert!(hook.contains("Headroom MarkItDown conversion"));
        assert!(hook.contains("sys.exit(0)"));
    }

    #[test]
    fn disabling_markitdown_marker_leaves_rtk_hook_intact() {
        let root = unique_temp_dir("headroom-strip-markitdown");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        fs::write(
            &settings,
            serde_json::to_string_pretty(&json!({
                "hooks": {
                    "PreToolUse": [
                        { "matcher": "Bash", "hooks": [{ "type": "command", "command": "/h/headroom-rtk-rewrite.sh" }] },
                        { "matcher": "Read", "hooks": [{ "type": "command", "command": "/h/headroom-markitdown-read.sh" }] }
                    ]
                }
            }))
            .unwrap(),
        )
        .expect("write settings");

        let changed = remove_pre_tool_use_markers(&settings, &["headroom-markitdown-read.sh"])
            .expect("strip");
        assert!(changed);

        let after: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
        let entries = after["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entry_contains_hook(&entries[0], "headroom-rtk-rewrite.sh"));
    }

    #[test]
    fn markitdown_office_nudge_points_at_the_shim_and_skips_pdf() {
        let nudge = build_markitdown_office_nudge(Path::new("/h/bin/markitdown"));
        assert!(nudge.contains("/h/bin/markitdown <path>"));
        assert!(nudge.contains(".docx"));
        assert!(nudge.contains("PDFs are handled automatically"));
    }

    #[test]
    fn markitdown_codex_nudge_covers_pdf_and_office() {
        let nudge = build_markitdown_codex_nudge(Path::new("/h/bin/markitdown"));
        assert!(nudge.contains("/h/bin/markitdown <path>"));
        // Codex has no hook, so PDF is covered by the CLI nudge too.
        assert!(nudge.contains(".pdf"));
        assert!(nudge.contains(".docx"));
    }

    #[test]
    fn hook_detection_finds_nested_hook_commands() {
        let hook_path = "/Users/test/.claude/hooks/headroom-rtk-rewrite.sh";
        let content = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "bash",
                        "hooks": [
                            { "type": "command", "command": hook_path }
                        ]
                    }
                ]
            }
        });

        assert!(claude_hook_present_in_value(&content, hook_path));
        assert!(entry_contains_hook(
            &content["hooks"]["PreToolUse"][0],
            "headroom-rtk-rewrite.sh"
        ));
        assert!(!entry_contains_hook(
            &json!({ "hooks": [] }),
            "headroom-rtk-rewrite.sh"
        ));
    }

    #[test]
    fn nvm_binary_candidates_include_installed_versions() {
        let home = unique_temp_dir("headroom-nvm-detect");
        let version_bin = home
            .join(".nvm")
            .join("versions")
            .join("node")
            .join("v22.17.1")
            .join("bin");
        fs::create_dir_all(&version_bin).expect("create nvm bin");
        fs::write(version_bin.join("claude"), "").expect("write fake claude binary");

        let candidates = nvm_binary_candidates(&home, &["claude"]);

        assert!(candidates
            .iter()
            .any(|candidate| candidate == &version_bin.join("claude")));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn path_lookup_scans_supplied_entries() {
        let home = unique_temp_dir("headroom-path-detect");
        let bin_dir = home.join("custom-bin");
        fs::create_dir_all(&bin_dir).expect("create custom bin");
        fs::write(bin_dir.join("claude"), "").expect("write fake claude binary");

        let detected = find_on_path_entries(vec![bin_dir.clone()], &["claude"]);

        assert_eq!(detected, Some(bin_dir.join("claude")));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn claude_user_state_detection_accepts_settings_or_projects() {
        let home = unique_temp_dir("headroom-claude-home");
        let claude_root = home.join(".claude");
        fs::create_dir_all(&claude_root).expect("create claude root");
        assert!(!claude_code_user_state_exists(&home));

        fs::write(claude_root.join("settings.json"), "{}").expect("write settings");
        assert!(claude_code_user_state_exists(&home));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn managed_block_upsert_replaces_existing_block_without_duplication() {
        let root = unique_temp_dir("headroom-managed-block");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zshrc");
        fs::write(&path, "export PATH=/usr/bin\n").expect("write shell file");

        let first = upsert_managed_block(
            &path,
            "claude_code",
            "export ANTHROPIC_BASE_URL=http://127.0.0.1:6767",
        )
        .expect("insert managed block");
        assert!(first.0);
        assert!(first.1.is_some());

        upsert_managed_block(
            &path,
            "claude_code",
            "export ANTHROPIC_BASE_URL=http://127.0.0.1:6767\nexport HEADROOM=1",
        )
        .expect("replace managed block");

        let content = fs::read_to_string(&path).expect("read updated shell file");
        assert_eq!(content.matches("# >>> headroom:claude_code >>>").count(), 1);
        assert!(content.contains("export PATH=/usr/bin"));
        assert!(content.contains("export HEADROOM=1"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn remove_managed_block_keeps_surrounding_shell_content_intact() {
        let root = unique_temp_dir("headroom-remove-block");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zprofile");
        fs::write(
            &path,
            "export PATH=/usr/bin\n# >>> headroom:claude_code >>>\nexport ANTHROPIC_BASE_URL=http://127.0.0.1:6767\n# <<< headroom:claude_code <<<\nexport EDITOR=vim\n",
        )
        .expect("write shell file");

        let removed = remove_managed_block(&path, "claude_code").expect("remove managed block");

        assert!(removed);
        assert_eq!(
            fs::read_to_string(&path).expect("read cleaned shell file"),
            "export PATH=/usr/bin\nexport EDITOR=vim\n"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn shell_block_helpers_only_match_content_inside_the_named_block() {
        let root = unique_temp_dir("headroom-shell-match");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".bashrc");
        fs::write(
            &path,
            "export ANTHROPIC_BASE_URL=https://example.com\n# >>> headroom:claude_code >>>\nexport ANTHROPIC_BASE_URL=http://127.0.0.1:6767\nexport PATH=/tmp/headroom:$PATH\n# <<< headroom:claude_code <<<\n",
        )
        .expect("write shell file");

        assert!(shell_block_contains_in_files(
            &[path.clone()],
            "claude_code",
            "ANTHROPIC_BASE_URL",
            "http://127.0.0.1:6767",
        )
        .expect("detect managed export"));
        assert!(
            shell_block_contains_text_in_files(&[path.clone()], "claude_code", "export PATH=",)
                .expect("detect managed text")
        );
        assert!(!shell_block_contains_in_files(
            &[path],
            "managed_rtk",
            "ANTHROPIC_BASE_URL",
            "http://127.0.0.1:6767",
        )
        .expect("ignore other block ids"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn write_file_if_changed_skips_backups_when_content_is_unchanged() {
        let root = unique_temp_dir("headroom-write-file");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join("headroom-rtk-rewrite.sh");
        fs::write(&path, "#!/bin/sh\necho headroom\n").expect("write hook file");

        let changed = write_file_if_changed(&path, "#!/bin/sh\necho headroom\n", false)
            .expect("skip unchanged write");

        assert_eq!(changed, (false, None));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn managed_block_round_trip_preserves_realistic_zshrc_content() {
        let root = unique_temp_dir("headroom-zshrc-roundtrip");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zshrc");
        let original = r#"export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# pnpm
export PNPM_HOME="/Users/test/Library/pnpm"
case ":$PATH:" in
  *":$PNPM_HOME:"*) ;;
  *) export PATH="$PNPM_HOME:$PATH" ;;
esac

export BUN_INSTALL="$HOME/.bun"
export PATH="$BUN_INSTALL/bin:$PATH"
"#;
        fs::write(&path, original).expect("write zshrc");

        upsert_managed_block(
            &path,
            "managed_rtk",
            "export PATH=\"/tmp/headroom/bin:$PATH\"",
        )
        .expect("add managed rtk block");
        upsert_managed_block(
            &path,
            "claude_code",
            "export ANTHROPIC_BASE_URL=http://127.0.0.1:6767",
        )
        .expect("add claude block");

        remove_managed_block(&path, "claude_code").expect("remove claude block");
        remove_managed_block(&path, "managed_rtk").expect("remove managed rtk block");

        let final_content = fs::read_to_string(&path).expect("read round-tripped zshrc");
        assert_eq!(final_content, original);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn updating_one_managed_block_does_not_touch_other_blocks_or_user_content() {
        let root = unique_temp_dir("headroom-multi-block-update");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zprofile");
        let original = r#"eval "$(/opt/homebrew/bin/brew shellenv)"

# >>> headroom:managed_rtk >>>
export PATH="/old/headroom/bin:$PATH"
# <<< headroom:managed_rtk <<<

# >>> headroom:claude_code >>>
export ANTHROPIC_BASE_URL=http://127.0.0.1:6767
# <<< headroom:claude_code <<<

eval "$(/opt/homebrew/bin/rbenv init - zsh)"
"#;
        fs::write(&path, original).expect("write zprofile");

        upsert_managed_block(
            &path,
            "managed_rtk",
            "export PATH=\"/new/headroom/bin:$PATH\"",
        )
        .expect("update managed rtk block");

        let updated = fs::read_to_string(&path).expect("read updated zprofile");
        assert!(updated.contains("eval \"$(/opt/homebrew/bin/brew shellenv)\""));
        assert!(updated.contains("eval \"$(/opt/homebrew/bin/rbenv init - zsh)\""));
        assert!(updated.contains("export PATH=\"/new/headroom/bin:$PATH\""));
        assert!(updated.contains("export ANTHROPIC_BASE_URL=http://127.0.0.1:6767"));
        assert_eq!(updated.matches("# >>> headroom:managed_rtk >>>").count(), 1);
        assert_eq!(updated.matches("# >>> headroom:claude_code >>>").count(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn removing_one_managed_block_leaves_other_managed_blocks_and_user_content() {
        let root = unique_temp_dir("headroom-remove-single-block");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zshrc");
        fs::write(
            &path,
            r#"export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# >>> headroom:managed_rtk >>>
export PATH="/tmp/headroom/bin:$PATH"
# <<< headroom:managed_rtk <<<

# >>> headroom:claude_code >>>
export ANTHROPIC_BASE_URL=http://127.0.0.1:6767
# <<< headroom:claude_code <<<
"#,
        )
        .expect("write zshrc");

        remove_managed_block(&path, "claude_code").expect("remove claude block");

        let updated = fs::read_to_string(&path).expect("read cleaned zshrc");
        assert!(updated.contains("export NVM_DIR=\"$HOME/.nvm\""));
        assert!(updated.contains("[ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\""));
        assert!(updated.contains("# >>> headroom:managed_rtk >>>"));
        assert!(updated.contains("export PATH=\"/tmp/headroom/bin:$PATH\""));
        assert!(!updated.contains("# >>> headroom:claude_code >>>"));

        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn strip_hook_returns_false_when_file_missing() {
        let root = unique_temp_dir("headroom-strip-missing");
        let settings = root.join("does-not-exist.json");
        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(!changed, "missing file should report no change");
        assert!(!settings.exists(), "should not create the file");
    }

    #[test]
    fn strip_hook_removes_headroom_entry_and_leaves_other_entries() {
        let root = unique_temp_dir("headroom-strip-mixed");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        let content = json!({
            "env": { "SOME_KEY": "keep-me" },
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "/other/tool/script.sh" }
                        ]
                    },
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/Users/test/.claude/hooks/headroom-rtk-rewrite.sh"
                            }
                        ]
                    }
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&content).unwrap())
            .expect("write settings");

        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(changed, "should report change");

        let raw = fs::read_to_string(&settings).expect("read settings");
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse settings");
        let entries = parsed
            .get("hooks")
            .and_then(|v| v.get("PreToolUse"))
            .and_then(|v| v.as_array())
            .expect("PreToolUse preserved");
        assert_eq!(entries.len(), 1, "only the non-headroom entry remains");
        assert!(
            entry_contains_hook(&entries[0], "other/tool/script.sh"),
            "unrelated entry preserved"
        );
        assert_eq!(
            parsed.get("env").and_then(|v| v.get("SOME_KEY")),
            Some(&json!("keep-me")),
            "unrelated top-level keys untouched"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strip_hook_drops_empty_pre_tool_use_and_hooks_keys() {
        let root = unique_temp_dir("headroom-strip-empty");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        let content = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/path/to/headroom-rtk-rewrite.sh"
                            }
                        ]
                    }
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&content).unwrap())
            .expect("write settings");

        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(changed);

        let raw = fs::read_to_string(&settings).expect("read settings");
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse settings");
        assert!(
            parsed.get("hooks").is_none(),
            "empty hooks object should be removed, got {parsed}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strip_hook_leaves_file_untouched_when_no_headroom_entry_present() {
        let root = unique_temp_dir("headroom-strip-noop");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        let original = serde_json::to_string_pretty(&json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "/unrelated.sh" }
                        ]
                    }
                ]
            }
        }))
        .unwrap();
        fs::write(&settings, &original).expect("write settings");

        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(!changed, "should report no change");

        let after = fs::read_to_string(&settings).expect("read settings");
        assert_eq!(after, original, "file should be byte-identical");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strip_hook_tolerates_empty_file() {
        let root = unique_temp_dir("headroom-strip-empty-file");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        fs::write(&settings, "").expect("write empty file");

        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(!changed, "empty file should report no change");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_script_falls_through_when_rewritten_first_token_missing_from_path() {
        // The hook has an OR guard that exits 0 when the binaries are missing,
        // so we give it real paths and verify the PATH-resolution check kicks in
        // when `rtk rewrite` produces a command whose first token can't be
        // resolved. That's the regression-prone slice added this session.
        let root = unique_temp_dir("headroom-hook-bash");
        fs::create_dir_all(&root).expect("create root");

        // Fake rtk that always prepends a made-up binary name that won't be on PATH.
        let fake_rtk = root.join("fake-rtk");
        fs::write(
            &fake_rtk,
            "#!/usr/bin/env bash\nshift  # drop the 'rewrite' arg\necho \"__headroom_nonexistent_binary_xyzzy__ $*\"\n",
        )
        .expect("write fake rtk");
        fs::set_permissions(
            &fake_rtk,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod rtk");

        // Use the real system python3 so the embedded Python snippets run.
        let system_python = PathBuf::from("/usr/bin/python3");
        assert!(system_python.exists(), "this test assumes /usr/bin/python3");

        let hook_body = build_headroom_rtk_hook(&fake_rtk, &system_python);
        let hook_path = root.join("hook.sh");
        fs::write(&hook_path, &hook_body).expect("write hook");
        fs::set_permissions(
            &hook_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod hook");

        // Hook expects a JSON object on stdin with tool_input.command.
        let stdin = r#"{"tool_input":{"command":"git status"}}"#;
        let output = std::process::Command::new("bash")
            .arg(&hook_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(stdin.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .expect("run hook");

        assert!(output.status.success(), "hook should exit 0");
        assert!(
            output.stdout.is_empty(),
            "hook should emit no rewrite when first token isn't resolvable, got: {:?}",
            String::from_utf8_lossy(&output.stdout)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_script_emits_rewrite_when_first_token_is_valid_absolute_path() {
        let root = unique_temp_dir("headroom-hook-bash-ok");
        fs::create_dir_all(&root).expect("create root");

        // Pick a binary that definitely exists on macOS/Linux test hosts.
        let real_binary = "/bin/echo";
        assert!(Path::new(real_binary).exists());

        // Fake rtk rewrites to use an absolute path that *does* exist.
        let fake_rtk = root.join("fake-rtk");
        fs::write(
            &fake_rtk,
            format!("#!/usr/bin/env bash\nshift\necho \"{real_binary} $*\"\n"),
        )
        .expect("write fake rtk");
        fs::set_permissions(
            &fake_rtk,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod rtk");

        let system_python = PathBuf::from("/usr/bin/python3");
        let hook_body = build_headroom_rtk_hook(&fake_rtk, &system_python);
        let hook_path = root.join("hook.sh");
        fs::write(&hook_path, &hook_body).expect("write hook");
        fs::set_permissions(
            &hook_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod hook");

        let stdin = r#"{"tool_input":{"command":"git status"}}"#;
        let output = std::process::Command::new("bash")
            .arg(&hook_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(stdin.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .expect("run hook");

        assert!(output.status.success(), "hook should exit 0");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(real_binary),
            "rewrite should be emitted when first token is a valid absolute path, got stdout: {stdout:?}, stderr: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("Headroom RTK auto-rewrite"),
            "should be a rewrite hookSpecificOutput payload"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_script_pins_bare_rtk_token_to_managed_absolute_path() {
        let root = unique_temp_dir("headroom-hook-pin-rtk");
        fs::create_dir_all(&root).expect("create root");

        // Fake rtk emits a bare `rtk` leading token, like the real binary.
        // `rtk` is NOT on PATH here, so without pinning the rewrite would be a
        // "command not found" landmine and the defense-in-depth guard would
        // drop it. Pinning to the managed absolute path must keep the rewrite.
        let fake_rtk = root.join("rtk");
        fs::write(&fake_rtk, "#!/usr/bin/env bash\nshift\necho \"rtk $*\"\n")
            .expect("write fake rtk");
        fs::set_permissions(
            &fake_rtk,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod rtk");

        let system_python = PathBuf::from("/usr/bin/python3");
        let hook_body = build_headroom_rtk_hook(&fake_rtk, &system_python);
        let hook_path = root.join("hook.sh");
        fs::write(&hook_path, &hook_body).expect("write hook");
        fs::set_permissions(
            &hook_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod hook");

        let stdin = r#"{"tool_input":{"command":"git status"}}"#;
        let output = std::process::Command::new("bash")
            .arg(&hook_path)
            .env("PATH", "/usr/bin:/bin") // ensure bare `rtk` is unresolvable
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(stdin.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .expect("run hook");

        assert!(output.status.success(), "hook should exit 0");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Headroom RTK auto-rewrite"),
            "rewrite should survive when bare `rtk` is pinned to absolute path, got stdout: {stdout:?}, stderr: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains(&fake_rtk.to_string_lossy().replace('"', "\\\"")),
            "rewritten command should invoke the managed rtk by absolute path, got: {stdout:?}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_script_emits_rewrite_even_when_rtk_rewrite_exits_nonzero() {
        let root = unique_temp_dir("headroom-hook-bash-nonzero");
        fs::create_dir_all(&root).expect("create root");

        let real_binary = "/bin/echo";
        assert!(Path::new(real_binary).exists());

        // Match the real rtk behavior we observed during smoke testing:
        // emit a rewrite, then exit non-zero. The hook's `|| true` should
        // still preserve the rewritten command.
        let fake_rtk = root.join("fake-rtk");
        fs::write(
            &fake_rtk,
            format!("#!/usr/bin/env bash\nshift\necho \"{real_binary} $*\"\nexit 3\n"),
        )
        .expect("write fake rtk");
        fs::set_permissions(
            &fake_rtk,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod rtk");

        let system_python = PathBuf::from("/usr/bin/python3");
        let hook_body = build_headroom_rtk_hook(&fake_rtk, &system_python);
        let hook_path = root.join("hook.sh");
        fs::write(&hook_path, &hook_body).expect("write hook");
        fs::set_permissions(
            &hook_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod hook");

        let stdin = r#"{"tool_input":{"command":"git status"}}"#;
        let output = std::process::Command::new("bash")
            .arg(&hook_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(stdin.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .expect("run hook");

        assert!(output.status.success(), "hook should exit 0");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(real_binary),
            "rewrite output should survive non-zero RTK exit, got stdout: {stdout:?}, stderr: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("Headroom RTK auto-rewrite"),
            "should still emit a rewrite hookSpecificOutput payload"
        );

        let _ = fs::remove_dir_all(root);
    }

    // ── Lifecycle integration tests ──────────────────────────────────────────
    //
    // These tests drive `apply_client_setup` / `verify_client_setup` /
    // `disable_client_setup` / `clear_client_setups` against a temp $HOME so we
    // catch regressions in the user-visible setup-then-teardown flow. Tests are
    // serialized via `serial_test` because they mutate process-wide env vars
    // (HOME, XDG_DATA_HOME, SHELL).

    /// RAII-style guard that snapshots HOME / XDG_DATA_HOME / SHELL, points
    /// them at a fresh tempdir, and restores them on drop. Used to keep
    /// lifecycle tests from touching the developer's real profile.
    static TEST_HOME_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    struct TestHome {
        _lock: std::sync::MutexGuard<'static, ()>,
        _tmp: tempfile::TempDir,
        home: PathBuf,
        prev_home: Option<std::ffi::OsString>,
        prev_xdg: Option<std::ffi::OsString>,
        prev_shell: Option<std::ffi::OsString>,
        prev_codex: Option<std::ffi::OsString>,
        prev_goose_env: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl TestHome {
        fn new() -> Self {
            let lock = TEST_HOME_LOCK
                .get_or_init(|| std::sync::Mutex::new(()))
                .lock()
                .expect("lock test home env");
            let tmp = tempfile::tempdir().expect("create temp home");
            let home = tmp.path().to_path_buf();
            let prev_home = std::env::var_os("HOME");
            let prev_xdg = std::env::var_os("XDG_DATA_HOME");
            let prev_shell = std::env::var_os("SHELL");
            let prev_codex = std::env::var_os("CODEX_HOME");
            let goose_env_keys = [
                "GOOSE_PROVIDER",
                "OPENAI_HOST",
                "OPENAI_BASE_URL",
                "OPENAI_BASE_PATH",
                "ANTHROPIC_HOST",
            ];
            let prev_goose_env = goose_env_keys
                .iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect::<Vec<_>>();
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_DATA_HOME", home.join(".local").join("share"));
            // Force a deterministic shell family so tests don't depend on the
            // dev's login shell.
            std::env::set_var("SHELL", "/bin/zsh");
            // Clear any real CODEX_HOME so codex_home() falls back to the temp
            // $HOME/.codex and the Codex tests stay hermetic on dev machines.
            std::env::remove_var("CODEX_HOME");
            // Goose treats endpoint environment variables as higher priority
            // than config.yaml. Clear them for fixture-home lifecycle tests so
            // the native adapter exercises its documented persisted schema;
            // the original values are restored by Drop.
            for (key, _) in &prev_goose_env {
                std::env::remove_var(key);
            }
            // Mirror what the app does at startup so write_setup_state has a
            // config dir to land in.
            crate::storage::ensure_data_dirs(&crate::storage::app_data_dir())
                .expect("ensure_data_dirs in test home");
            TestHome {
                _lock: lock,
                _tmp: tmp,
                home,
                prev_home,
                prev_xdg,
                prev_shell,
                prev_codex,
                prev_goose_env,
            }
        }

        fn path(&self) -> &Path {
            &self.home
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            match self.prev_home.take() {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match self.prev_xdg.take() {
                Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
            match self.prev_shell.take() {
                Some(v) => std::env::set_var("SHELL", v),
                None => std::env::remove_var("SHELL"),
            }
            match self.prev_codex.take() {
                Some(v) => std::env::set_var("CODEX_HOME", v),
                None => std::env::remove_var("CODEX_HOME"),
            }
            for (key, value) in self.prev_goose_env.drain(..) {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    /// RTK is opt-in: its PATH block and Claude Code hook are only wired when the
    /// managed binary exists on disk. Drop a fake one at the default location so
    /// tests covering a fully-configured environment exercise the RTK wiring.
    fn seed_installed_rtk() {
        let rtk = super::default_headroom_rtk_path();
        fs::create_dir_all(rtk.parent().unwrap()).unwrap();
        fs::write(&rtk, "#!/bin/sh\n").unwrap();
    }

    #[test]
    #[serial_test::serial]
    fn gemini_setup_writes_verifies_and_cleans_sidecar_only() {
        let home = TestHome::new();
        let sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create gemini dir");
        fs::write(&sidecar, "# user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("gemini_cli").expect("apply gemini setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result
            .changed_files
            .contains(&home.path().join(".zprofile").display().to_string()));
        assert!(result
            .changed_files
            .contains(&sidecar.display().to_string()));
        assert_eq!(result.backup_files.len(), 1);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Switchboard sidecar written"));

        let content = fs::read_to_string(&sidecar).expect("read sidecar");
        assert!(content.contains("# user note\nkeep this"));
        assert!(content.contains("# >>> headroom:gemini_cli >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        let shell_content = fs::read_to_string(home.path().join(".zprofile")).expect("read shell");
        assert!(shell_content.contains("GOOGLE_GEMINI_BASE_URL=http://127.0.0.1:6767"));
        assert!(shell_content.contains("GEMINI_BASE_URL=http://127.0.0.1:6767"));

        let detected_clients = vec![ClientStatus {
            id: "gemini_cli".into(),
            name: "Gemini CLI".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![
                "Gemini binary: /opt/homebrew/bin/gemini".into(),
                format!(
                    "Gemini config surface: {}",
                    home.path().join(".gemini").display()
                ),
            ],
        }];
        let connectors = list_client_connectors(&detected_clients).expect("list connectors");
        let gemini = connectors
            .iter()
            .find(|connector| connector.client_id == "gemini_cli")
            .expect("gemini connector");
        assert!(gemini.enabled);
        assert!(gemini.verified);
        assert!(gemini.last_configured_at.is_some());
        assert!(gemini
            .automation_path
            .iter()
            .all(|stage| stage.status == "ready"));

        super::disable_client_setup("gemini_cli").expect("disable gemini setup");
        let content = fs::read_to_string(&sidecar).expect("read cleaned sidecar");
        assert_eq!(content, "# user note\nkeep this\n");
        let shell_content = fs::read_to_string(home.path().join(".zprofile")).expect("read shell");
        assert!(!shell_content.contains("GOOGLE_GEMINI_BASE_URL"));
        let verification =
            super::verify_client_setup("gemini_cli").expect("verify cleaned gemini setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn gemini_managed_rollback_removes_shell_and_sidecar_blocks() {
        let home = TestHome::new();
        let sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create gemini dir");
        fs::write(&sidecar, "# user note\nkeep this\n").expect("seed sidecar");

        super::apply_client_setup("gemini_cli").expect("apply gemini setup");

        let preview =
            super::preview_managed_rollback("gemini-routing").expect("preview gemini rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.backup_path.is_none());
        assert!(preview.backup_exists);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore headroom:gemini_cli for Gemini CLI routing"
        );

        let result = super::execute_managed_rollback(
            "gemini-routing",
            "",
            "Restore headroom:gemini_cli for Gemini CLI routing",
        )
        .expect("execute gemini rollback");
        assert_eq!(
            result.restored_from,
            "Switchboard-owned Gemini shell and sidecar blocks removed."
        );

        let content = fs::read_to_string(&sidecar).expect("read cleaned sidecar");
        assert_eq!(content, "# user note\nkeep this\n");
        let shell_content = fs::read_to_string(home.path().join(".zprofile")).expect("read shell");
        assert!(!shell_content.contains("GOOGLE_GEMINI_BASE_URL"));
    }

    #[test]
    #[serial_test::serial]
    fn sidecar_managed_rollback_removes_existing_cursor_sidecar_block_only() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create cursor dir");
        fs::write(&sidecar, "# cursor user note\nkeep this\n").expect("seed sidecar");

        super::configure_planned_switchboard_sidecar("cursor").expect("seed cursor sidecar");

        let preview =
            super::preview_managed_rollback("cursor-routing").expect("preview cursor rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.backup_path.is_none());
        assert!(preview.backup_exists);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore headroom:cursor for Cursor routing"
        );
        assert!(preview.proposed_action.contains("Cursor sidecar block"));
        assert!(preview
            .evidence
            .join(" ")
            .contains("Current sidecar must still contain"));

        let result = super::execute_managed_rollback(
            "cursor-routing",
            "",
            "Restore headroom:cursor for Cursor routing",
        )
        .expect("execute cursor rollback");
        assert_eq!(
            result.restored_from,
            "Switchboard-owned cursor sidecar block removed."
        );
        let safety_backup = result
            .safety_backup_path
            .as_ref()
            .expect("sidecar rollback reports safety backup");
        assert!(
            safety_backup.contains(".headroom-backup-"),
            "unexpected safety backup path: {safety_backup}"
        );
        assert!(std::path::Path::new(safety_backup).exists());
        assert!(result
            .verification
            .join(" ")
            .contains("fresh sidecar safety backup"));
        assert!(result
            .verification
            .join(" ")
            .contains("Relaunch-survival evidence"));

        let content = fs::read_to_string(&sidecar).expect("read cleaned sidecar");
        assert_eq!(content, "# cursor user note\nkeep this\n");
        assert!(!super::planned_switchboard_sidecar_matches("cursor")
            .expect("check cleaned cursor sidecar"));
    }

    #[test]
    #[serial_test::serial]
    fn amazon_q_sidecar_lifecycle_applies_repairs_rolls_back_and_disables() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join(".aws")
            .join("amazonq")
            .join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create amazon q dir");
        fs::write(&sidecar, "# amazon q user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("amazon_q").expect("apply amazon q setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert_eq!(result.changed_files, vec![sidecar.display().to_string()]);
        assert_eq!(result.backup_files.len(), 1);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Amazon Q Developer CLI"));

        let content = fs::read_to_string(&sidecar).expect("read amazon q sidecar");
        assert!(content.contains("# amazon q user note\nkeep this"));
        assert!(content.contains("# >>> headroom:amazon_q >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(content.contains("Amazon Q Developer CLI routing-intent sidecar"));

        let connectors = list_client_connectors(&[ClientStatus {
            id: "amazon_q".into(),
            name: "Amazon Q Developer CLI".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![format!(
                "Amazon Q config surface: {}",
                sidecar.parent().unwrap().display()
            )],
        }])
        .expect("list connectors");
        let amazon_q = connectors
            .iter()
            .find(|connector| connector.client_id == "amazon_q")
            .expect("amazon q connector");
        assert_eq!(
            amazon_q.support_status,
            ClientConnectorSupportStatus::Managed
        );
        assert!(amazon_q.enabled);
        assert!(amazon_q.verified);
        assert!(amazon_q.config_creation_steps.is_empty());
        assert!(amazon_q.automation_path.is_empty());

        let drifted = content.replace(super::HEADROOM_OPENAI_BASE_URL, "http://127.0.0.1:1");
        fs::write(&sidecar, drifted).expect("drift amazon q sidecar");
        let verification =
            super::verify_client_setup("amazon_q").expect("verify drifted amazon q setup");
        assert!(!verification.verified);
        assert!(verification
            .failures
            .join(" ")
            .contains("Switchboard-managed Amazon Q Developer CLI sidecar was not found"));

        let repaired = super::apply_client_setup("amazon_q").expect("repair amazon q setup");
        assert!(repaired.verification.verified);
        assert!(fs::read_to_string(&sidecar)
            .expect("read repaired amazon q sidecar")
            .contains(super::HEADROOM_OPENAI_BASE_URL));

        let preview =
            super::preview_managed_rollback("amazon-q-routing").expect("preview amazon q rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore headroom:amazon_q for Amazon Q Developer CLI routing"
        );

        let rollback = super::execute_managed_rollback(
            "amazon-q-routing",
            "",
            "Restore headroom:amazon_q for Amazon Q Developer CLI routing",
        )
        .expect("execute amazon q rollback");
        assert_eq!(
            rollback.restored_from,
            "Switchboard-owned amazon_q sidecar block removed."
        );
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read rolled back amazon q sidecar"),
            "# amazon q user note\nkeep this\n"
        );

        super::apply_client_setup("amazon_q").expect("reapply amazon q setup");
        super::disable_client_setup("amazon_q").expect("disable amazon q setup");
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read disabled amazon q sidecar"),
            "# amazon q user note\nkeep this\n"
        );
        let verification =
            super::verify_client_setup("amazon_q").expect("verify disabled amazon q setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn aider_sidecar_lifecycle_applies_repairs_rolls_back_and_disables() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join(".config")
            .join("aider")
            .join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create aider dir");
        fs::write(&sidecar, "# aider user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("aider").expect("apply aider setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert_eq!(result.changed_files, vec![sidecar.display().to_string()]);
        assert_eq!(result.backup_files.len(), 1);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Aider"));

        let content = fs::read_to_string(&sidecar).expect("read aider sidecar");
        assert!(content.contains("# aider user note\nkeep this"));
        assert!(content.contains("# >>> headroom:aider >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(content.contains("Aider routing-intent sidecar"));

        let connectors = list_client_connectors(&[ClientStatus {
            id: "aider".into(),
            name: "Aider".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![format!(
                "Aider config surface: {}",
                sidecar.parent().unwrap().display()
            )],
        }])
        .expect("list connectors");
        let aider = connectors
            .iter()
            .find(|connector| connector.client_id == "aider")
            .expect("aider connector");
        assert_eq!(aider.support_status, ClientConnectorSupportStatus::Managed);
        assert!(aider.enabled);
        assert!(aider.verified);
        assert!(aider.config_creation_steps.is_empty());
        assert!(aider.automation_path.is_empty());

        let drifted = content.replace(super::HEADROOM_OPENAI_BASE_URL, "http://127.0.0.1:1");
        fs::write(&sidecar, drifted).expect("drift aider sidecar");
        let verification = super::verify_client_setup("aider").expect("verify drifted aider setup");
        assert!(!verification.verified);
        assert!(verification
            .failures
            .join(" ")
            .contains("Switchboard-managed Aider sidecar was not found"));

        let repaired = super::apply_client_setup("aider").expect("repair aider setup");
        assert!(repaired.verification.verified);
        assert!(fs::read_to_string(&sidecar)
            .expect("read repaired aider sidecar")
            .contains(super::HEADROOM_OPENAI_BASE_URL));

        let preview =
            super::preview_managed_rollback("aider-routing").expect("preview aider rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore headroom:aider for Aider routing"
        );

        let rollback = super::execute_managed_rollback(
            "aider-routing",
            "",
            "Restore headroom:aider for Aider routing",
        )
        .expect("execute aider rollback");
        assert_eq!(
            rollback.restored_from,
            "Switchboard-owned aider sidecar block removed."
        );
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read rolled back aider sidecar"),
            "# aider user note\nkeep this\n"
        );

        super::apply_client_setup("aider").expect("reapply aider setup");
        super::disable_client_setup("aider").expect("disable aider setup");
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read disabled aider sidecar"),
            "# aider user note\nkeep this\n"
        );
        let verification =
            super::verify_client_setup("aider").expect("verify disabled aider setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn continue_sidecar_lifecycle_applies_repairs_rolls_back_and_disables() {
        let home = TestHome::new();
        let sidecar = home.path().join(".continue").join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create continue dir");
        fs::write(&sidecar, "# continue user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("continue").expect("apply continue setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert_eq!(result.changed_files, vec![sidecar.display().to_string()]);
        assert_eq!(result.backup_files.len(), 1);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Continue"));

        let content = fs::read_to_string(&sidecar).expect("read continue sidecar");
        assert!(content.contains("# continue user note\nkeep this"));
        assert!(content.contains("# >>> headroom:continue >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(content.contains("Continue routing-intent sidecar"));

        let connectors = list_client_connectors(&[ClientStatus {
            id: "continue".into(),
            name: "Continue".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![format!(
                "Continue config folder: {}",
                sidecar.parent().unwrap().display()
            )],
        }])
        .expect("list connectors");
        let continue_connector = connectors
            .iter()
            .find(|connector| connector.client_id == "continue")
            .expect("continue connector");
        assert_eq!(
            continue_connector.support_status,
            ClientConnectorSupportStatus::Managed
        );
        assert!(continue_connector.enabled);
        assert!(continue_connector.verified);
        assert!(continue_connector.config_creation_steps.is_empty());
        assert!(continue_connector.automation_path.is_empty());

        let drifted = content.replace(super::HEADROOM_OPENAI_BASE_URL, "http://127.0.0.1:1");
        fs::write(&sidecar, drifted).expect("drift continue sidecar");
        let verification =
            super::verify_client_setup("continue").expect("verify drifted continue setup");
        assert!(!verification.verified);
        assert!(verification
            .failures
            .join(" ")
            .contains("Switchboard-managed Continue sidecar was not found"));

        let repaired = super::apply_client_setup("continue").expect("repair continue setup");
        assert!(repaired.verification.verified);
        assert!(fs::read_to_string(&sidecar)
            .expect("read repaired continue sidecar")
            .contains(super::HEADROOM_OPENAI_BASE_URL));

        let preview =
            super::preview_managed_rollback("continue-routing").expect("preview continue rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore headroom:continue for Continue routing"
        );

        let rollback = super::execute_managed_rollback(
            "continue-routing",
            "",
            "Restore headroom:continue for Continue routing",
        )
        .expect("execute continue rollback");
        assert_eq!(
            rollback.restored_from,
            "Switchboard-owned continue sidecar block removed."
        );
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read rolled back continue sidecar"),
            "# continue user note\nkeep this\n"
        );

        super::apply_client_setup("continue").expect("reapply continue setup");
        super::disable_client_setup("continue").expect("disable continue setup");
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read disabled continue sidecar"),
            "# continue user note\nkeep this\n"
        );
        let verification =
            super::verify_client_setup("continue").expect("verify disabled continue setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn goose_native_provider_and_mcp_bridge_lifecycle_preserves_user_state() {
        let home = TestHome::new();
        let config = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Block")
            .join("goose")
            .join("config.yaml");
        fs::create_dir_all(config.parent().unwrap()).expect("create goose config dir");
        fs::write(
            &config,
            "active_provider: openai\nproviders:\n  openai:\n    enabled: true\n    model: gpt-4o\n    configured: true\nkeep: true\n",
        )
        .expect("seed goose config");

        let sidecar = planned_sidecar_routing_path("goose").expect("goose sidecar path");
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create goose dir");
        fs::write(&sidecar, "# goose user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("goose").expect("apply goose setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result.changed_files.contains(&config.display().to_string()));
        assert!(result
            .changed_files
            .contains(&sidecar.display().to_string()));
        assert_eq!(result.backup_files.len(), 2);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Repo Memory MCP bridge"));
        assert!(result
            .summary
            .contains("credentials and account state remain manual"));
        assert!(result
            .next_steps
            .iter()
            .any(|step| step.contains("allowlisted provider endpoint fields")));

        let content = fs::read_to_string(&sidecar).expect("read goose sidecar");
        assert!(content.contains("# goose user note\nkeep this"));
        assert!(content.contains("# >>> headroom:goose >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(content.contains("Repo Memory MCP bridge marker"));
        assert!(content.contains("allowlisted native endpoint routing"));
        assert!(content.contains(
            "account state, secrets, provider credentials, and model selection remain manual"
        ));

        let config_content = fs::read_to_string(&config).expect("read configured goose config");
        assert!(config_content.contains("active_provider: openai"));
        assert!(config_content.contains("model: gpt-4o"));
        assert!(config_content.contains("keep: true"));
        assert!(config_content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(config_content.contains("OPENAI_BASE_PATH: v1/chat/completions"));

        let preview =
            super::preview_managed_rollback("goose-routing").expect("preview goose rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore headroom:goose for Goose MCP bridge"
        );

        let rollback = super::execute_managed_rollback(
            "goose-routing",
            "",
            "Restore headroom:goose for Goose MCP bridge",
        )
        .expect("execute goose rollback");
        assert_eq!(
            rollback.restored_from,
            "Switchboard-owned goose sidecar block removed."
        );
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read rolled back goose sidecar"),
            "# goose user note\nkeep this\n"
        );

        super::apply_client_setup("goose").expect("reapply goose setup");
        super::disable_client_setup("goose").expect("disable goose setup");
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read disabled goose sidecar"),
            "# goose user note\nkeep this\n"
        );
        let disabled_config = fs::read_to_string(&config).expect("read disabled goose config");
        assert!(disabled_config.contains("active_provider: openai"));
        assert!(disabled_config.contains("model: gpt-4o"));
        assert!(disabled_config.contains("keep: true"));
        assert!(!disabled_config.contains(super::HEADROOM_OPENAI_BASE_URL));
        let verification =
            super::verify_client_setup("goose").expect("verify disabled goose setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn goose_and_grok_sidecar_apply_requires_exact_current_confirmation_and_preserves_user_content()
    {
        let home = TestHome::new();
        for (record_id, client_id, owner) in [
            (
                super::GOOSE_SIDECAR_APPLY_RECORD_ID,
                "goose",
                super::GOOSE_SIDECAR_OWNER,
            ),
            (
                super::GROK_SIDECAR_APPLY_RECORD_ID,
                "grok_cli",
                super::GROK_SIDECAR_OWNER,
            ),
        ] {
            let sidecar = planned_sidecar_routing_path(client_id).expect("sidecar path");
            fs::create_dir_all(sidecar.parent().unwrap()).expect("create sidecar parent");
            fs::write(&sidecar, format!("# {client_id} user note\nkeep this\n"))
                .expect("seed user content");

            let preview = super::preview_managed_config_apply(record_id).expect("preview");
            assert!(preview.current_state.contains("keep this"));
            assert!(preview
                .proposed_state
                .contains(&format!("headroom:{client_id}")));
            assert!(preview
                .evidence
                .iter()
                .any(|item| item.contains("not allowlisted")));

            fs::write(&sidecar, "changed outside Switchboard\n").expect("create stale preview");
            assert!(
                super::execute_managed_config_apply(record_id, &preview.confirmation_phrase)
                    .is_err()
            );
            assert_eq!(
                fs::read_to_string(&sidecar).unwrap(),
                "changed outside Switchboard\n"
            );

            let preview = super::preview_managed_config_apply(record_id).expect("fresh preview");
            let applied =
                super::execute_managed_config_apply(record_id, &preview.confirmation_phrase)
                    .expect("apply sidecar");
            assert_eq!(applied.owner, owner);
            assert!(super::planned_switchboard_sidecar_matches(client_id).unwrap());
            assert!(applied
                .verification
                .iter()
                .any(|item| item.contains("credentials")));

            super::disable_client_setup(client_id).expect("off cleanup");
            let cleaned = fs::read_to_string(&sidecar).expect("read cleanup");
            assert!(cleaned.contains("changed outside Switchboard"));
            assert!(!super::planned_switchboard_sidecar_matches(client_id).unwrap());
        }
        assert!(!home
            .path()
            .join(".config")
            .join("xai")
            .join("auth.json")
            .exists());
    }

    #[test]
    #[serial_test::serial]
    fn promoted_editor_rollback_records_use_native_targets_not_sidecars() {
        let _home = TestHome::new();

        assert!(super::sidecar_rollback_target("windsurf-routing").is_none());
        assert!(super::sidecar_rollback_target("zed-ai-routing").is_none());

        let windsurf =
            super::preview_managed_rollback("windsurf-routing").expect("preview windsurf rollback");
        assert_eq!(windsurf.record_id, "windsurf-routing");
        assert_eq!(windsurf.marker, "headroom:windsurf");
        assert!(windsurf
            .target_path
            .ends_with("Library/Application Support/Windsurf/User/settings.json"));
        assert!(windsurf
            .proposed_action
            .contains("Restore the Windsurf settings"));

        let zed = super::preview_managed_rollback("zed-ai-routing").expect("preview zed rollback");
        assert_eq!(zed.record_id, "zed-ai-routing");
        assert_eq!(zed.marker, "headroom:zed");
        assert!(zed.target_path.ends_with(".config/zed/settings.json"));
        assert!(zed.proposed_action.contains("Restore the Zed settings"));
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_undo_all_executes_ready_native_rows_only() {
        let home = TestHome::new();
        let gemini_sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(gemini_sidecar.parent().unwrap()).expect("create gemini dir");
        fs::write(&gemini_sidecar, "# gemini user note\nkeep this\n").expect("seed gemini");
        let cursor_sidecar = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(cursor_sidecar.parent().unwrap()).expect("create cursor dir");
        fs::write(&cursor_sidecar, "# cursor user note\nkeep this\n").expect("seed cursor");

        super::apply_client_setup("gemini_cli").expect("apply gemini setup");
        super::configure_planned_switchboard_sidecar("cursor").expect("seed cursor sidecar");

        let preview = super::preview_managed_rollback_undo_all();
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        let ready_ids = preview
            .ready
            .iter()
            .map(|row| row.record_id.as_str())
            .collect::<Vec<_>>();
        assert!(ready_ids.contains(&"gemini-routing"));
        assert!(ready_ids.contains(&"cursor-routing"));
        assert!(
            !preview.blocked.is_empty(),
            "unused native rows should remain blocked"
        );

        let result = super::execute_managed_rollback_undo_all(
            "Undo all ready Switchboard native rollback rows",
        )
        .expect("execute undo-all");
        let executed_ids = result
            .executed
            .iter()
            .map(|row| row.record_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(executed_ids, vec!["gemini-routing", "cursor-routing"]);
        let cursor_result = result
            .executed
            .iter()
            .find(|row| row.record_id == "cursor-routing")
            .expect("cursor rollback result");
        assert!(cursor_result.safety_backup_path.is_some());
        assert_eq!(
            fs::read_to_string(&gemini_sidecar).expect("read cleaned gemini"),
            "# gemini user note\nkeep this\n"
        );
        assert_eq!(
            fs::read_to_string(&cursor_sidecar).expect("read cleaned cursor"),
            "# cursor user note\nkeep this\n"
        );
    }

    #[test]
    #[serial_test::serial]
    fn opencode_setup_writes_verifies_and_cleans_native_routing_only() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join(".config")
            .join("opencode")
            .join(SWITCHBOARD_ROUTING_FILE);
        let config = home
            .path()
            .join(".config")
            .join("opencode")
            .join(OPENCODE_CONFIG_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create opencode dir");
        fs::write(&sidecar, "# opencode user note\nkeep this\n").expect("seed sidecar");
        fs::write(
            &config,
            r#"{"provider":{"custom":{"name":"Custom"}},"theme":"system"}"#,
        )
        .expect("seed opencode config");

        let result = super::apply_client_setup("opencode").expect("apply opencode setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result.changed_files.contains(&config.display().to_string()));
        assert!(result
            .changed_files
            .contains(&sidecar.display().to_string()));
        assert_eq!(result.backup_files.len(), 2);
        assert!(result.verification.verified);
        assert!(result
            .summary
            .contains("OpenCode Switchboard sidecar written"));

        let content = fs::read_to_string(&sidecar).expect("read sidecar");
        assert!(content.contains("# opencode user note\nkeep this"));
        assert!(content.contains("# >>> headroom:opencode >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        let config_value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config).expect("read config"))
                .expect("parse config");
        assert_eq!(config_value["theme"], "system");
        assert_eq!(config_value["provider"]["custom"]["name"], "Custom");
        assert_eq!(
            config_value["provider"]["headroom"]["options"]["baseURL"],
            super::HEADROOM_OPENAI_BASE_URL
        );

        let detected_clients = vec![ClientStatus {
            id: "opencode".into(),
            name: "OpenCode".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![
                "OpenCode binary: /opt/homebrew/bin/opencode".into(),
                format!(
                    "OpenCode config surface: {}",
                    home.path().join(".config").join("opencode").display()
                ),
            ],
        }];
        let connectors = list_client_connectors(&detected_clients).expect("list connectors");
        let opencode = connectors
            .iter()
            .find(|connector| connector.client_id == "opencode")
            .expect("opencode connector");
        assert!(opencode.enabled);
        assert!(opencode.verified);
        assert!(opencode.last_configured_at.is_some());
        assert!(opencode
            .automation_path
            .iter()
            .all(|stage| stage.status == "ready"));

        super::disable_client_setup("opencode").expect("disable opencode setup");
        let content = fs::read_to_string(&sidecar).expect("read cleaned sidecar");
        assert_eq!(content, "# opencode user note\nkeep this\n");
        let config_value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config).expect("read config"))
                .expect("parse config");
        assert!(config_value["provider"]["headroom"].is_null());
        assert_eq!(config_value["provider"]["custom"]["name"], "Custom");
        let verification =
            super::verify_client_setup("opencode").expect("verify cleaned opencode setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn windsurf_setup_writes_verifies_and_off_cleanup_removes_native_routing_only() {
        let home = TestHome::new();
        let windsurf_dir = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Windsurf")
            .join("User");
        fs::create_dir_all(&windsurf_dir).unwrap();
        let settings_json = windsurf_dir.join("settings.json");
        fs::write(
            &settings_json,
            r#"{"workbench.colorTheme":"Quiet Light","assistant":{"defaultModel":"claude-3-5-sonnet"}}"#,
        )
        .unwrap();

        let result = super::apply_client_setup("windsurf").expect("apply windsurf setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result
            .changed_files
            .contains(&settings_json.display().to_string()));
        assert_eq!(result.backup_files.len(), 1);
        assert!(result.verification.verified);

        let configured: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_json).expect("read settings"))
                .expect("parse settings");
        assert_eq!(configured["workbench.colorTheme"], "Quiet Light");
        assert_eq!(configured["assistant"]["defaultModel"], "claude-3-5-sonnet");
        assert_eq!(
            configured["anthropic.baseUrl"],
            super::HEADROOM_ANTHROPIC_BASE_URL
        );
        assert!(configured
            .get(format!("// >>> {} >>>", super::WINDSURF_MARKER_PREFIX))
            .is_some());

        super::disable_client_setup("windsurf").expect("disable windsurf setup");
        let cleaned: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_json).expect("read cleaned"))
                .expect("parse cleaned settings");
        assert_eq!(cleaned["workbench.colorTheme"], "Quiet Light");
        assert_eq!(cleaned["assistant"]["defaultModel"], "claude-3-5-sonnet");
        assert!(cleaned.get("anthropic.baseUrl").is_none());
        assert!(cleaned
            .get(format!("// >>> {} >>>", super::WINDSURF_MARKER_PREFIX))
            .is_none());
        assert!(cleaned
            .get(format!("// <<< {} <<<", super::WINDSURF_MARKER_PREFIX))
            .is_none());

        let verification =
            super::verify_client_setup("windsurf").expect("verify cleaned windsurf setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn zed_setup_writes_verifies_and_off_cleanup_removes_native_routing_only() {
        let home = TestHome::new();
        let zed_dir = home.path().join(".config").join("zed");
        fs::create_dir_all(&zed_dir).unwrap();
        let settings_json = zed_dir.join("settings.json");
        fs::write(
            &settings_json,
            r#"{"theme":"One Dark","assistant":{"default_model":"claude-3-5-sonnet"}}"#,
        )
        .unwrap();

        let result = super::apply_client_setup("zed_ai").expect("apply zed setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result
            .changed_files
            .contains(&settings_json.display().to_string()));
        assert!(result.backup_files.len() == 1);
        assert!(result.verification.verified);

        let configured: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_json).expect("read settings"))
                .expect("parse settings");
        assert_eq!(configured["theme"], "One Dark");
        assert_eq!(
            configured["assistant"]["default_model"],
            "claude-3-5-sonnet"
        );
        assert_eq!(
            configured["anthropic.baseUrl"],
            super::HEADROOM_ANTHROPIC_BASE_URL
        );
        assert!(configured
            .get(format!("// >>> {} >>>", super::ZED_MARKER_PREFIX))
            .is_some());

        super::disable_client_setup("zed_ai").expect("disable zed setup");
        let cleaned: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_json).expect("read cleaned"))
                .expect("parse cleaned settings");
        assert_eq!(cleaned["theme"], "One Dark");
        assert_eq!(cleaned["assistant"]["default_model"], "claude-3-5-sonnet");
        assert!(cleaned.get("anthropic.baseUrl").is_none());
        assert!(cleaned
            .get(format!("// >>> {} >>>", super::ZED_MARKER_PREFIX))
            .is_none());
        assert!(cleaned
            .get(format!("// <<< {} <<<", super::ZED_MARKER_PREFIX))
            .is_none());

        let verification = super::verify_client_setup("zed_ai").expect("verify cleaned zed setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn grok_sidecar_setup_is_managed_and_never_reads_or_writes_xai_credentials() {
        let home = TestHome::new();
        let connectors = [("grok_cli", "Grok / xAI CLI")];

        for (client_id, name) in connectors {
            let sidecar = planned_sidecar_routing_path(client_id).expect("sidecar path available");
            fs::create_dir_all(sidecar.parent().unwrap()).expect("create sidecar parent");
            fs::write(&sidecar, format!("# {client_id} user note\nkeep this\n"))
                .expect("seed sidecar");

            let result = super::apply_client_setup(client_id).expect("apply managed sidecar setup");
            assert!(result.verification.verified);
            let content = fs::read_to_string(&sidecar).expect("read sidecar");
            assert!(content.contains(&format!("# {client_id} user note\nkeep this")));
            assert!(content.contains("routing-intent sidecar"));

            let detected_clients = vec![ClientStatus {
                id: client_id.into(),
                name: name.into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![format!("{name} config surface: {}", sidecar.display())],
            }];
            let listed = list_client_connectors(&detected_clients).expect("list connectors");
            let connector = listed
                .iter()
                .find(|connector| connector.client_id == client_id)
                .unwrap_or_else(|| panic!("{client_id} connector listed"));
            assert_eq!(
                connector.support_status,
                ClientConnectorSupportStatus::Managed,
                "{client_id} sidecar lifecycle is safely managed"
            );
            assert!(connector.enabled, "{client_id} should be enabled");
            assert!(connector.verified, "{client_id} should be verified");
            assert!(connector.config_creation_steps.is_empty());
            assert!(connector.config_dry_run_preview.is_none());
            assert!(connector.automation_path.is_empty());
            super::disable_client_setup(client_id).expect("off cleanup");
        }

        assert!(
            !home.path().join(".aws").join("credentials").exists(),
            "Amazon Q sidecar setup must not create AWS credentials"
        );
    }

    fn read_settings_json(path: &Path) -> serde_json::Value {
        let raw = fs::read_to_string(path).expect("read settings.json");
        serde_json::from_str(&raw).expect("parse settings.json")
    }

    fn seed_caveman_clients_configured() {
        super::write_setup_state(&ClientSetupState {
            configured_clients: BTreeMap::from([
                ("claude_code".into(), "2026-06-27T00:00:00Z".into()),
                ("codex_cli".into(), "2026-06-27T00:00:01Z".into()),
            ]),
            remembered_clients: BTreeMap::new(),
            managed_shell_files: BTreeMap::new(),
            remembered_shell_files: BTreeMap::new(),
            rtk_disabled: false,
            switchboard_mode: None,
            savings_mode: None,
        })
        .expect("write setup state");
    }

    #[test]
    #[serial_test::serial]
    fn caveman_block_round_trips_for_configured_clients() {
        let home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("scoped").expect("enable caveman");

        let claude =
            fs::read_to_string(home.path().join(".claude").join("CLAUDE.md")).expect("read claude");
        let codex =
            fs::read_to_string(home.path().join(".codex").join("AGENTS.md")).expect("read codex");
        assert!(claude.contains("headroom:caveman"));
        assert!(claude.contains("Switchboard Caveman, scoped"));
        assert!(codex.contains("headroom:caveman"));
        assert!(codex.contains("Switchboard Caveman, scoped"));
    }

    #[test]
    #[serial_test::serial]
    fn caveman_level_switch_rewrites_managed_body() {
        let home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("scoped").expect("enable scoped");
        super::enable_caveman_integration("aggressive").expect("enable aggressive");

        let agents =
            fs::read_to_string(home.path().join(".codex").join("AGENTS.md")).expect("read codex");
        assert!(agents.contains("Switchboard Caveman, aggressive"));
        assert!(!agents.contains("Switchboard Caveman, scoped"));
    }

    #[test]
    #[serial_test::serial]
    fn caveman_integration_match_detects_stale_level_body() {
        let _home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("scoped").expect("enable scoped");

        assert!(
            super::caveman_integration_matches_level("scoped").expect("check scoped"),
            "scoped body should match"
        );
        assert!(
            !super::caveman_integration_matches_level("compact_chinese")
                .expect("check compact chinese"),
            "compact Chinese should not match stale scoped body"
        );
    }

    #[test]
    #[serial_test::serial]
    fn caveman_compact_chinese_profile_is_internal_only() {
        let home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("compact_chinese").expect("enable compact chinese");

        let agents =
            fs::read_to_string(home.path().join(".codex").join("AGENTS.md")).expect("read codex");
        assert!(agents.contains("Switchboard Caveman, compact Chinese experimental"));
        assert!(agents.contains("private internal planning notes"));
        assert!(agents.contains("user-visible replies"));
        assert!(agents.contains("legal, safety"));
        assert!(agents.contains("debugging"));
        assert!(agents.contains("release-readiness"));
        assert!(agents.contains("Never translate code"));
    }

    #[test]
    #[serial_test::serial]
    fn caveman_disable_and_full_cleanup_remove_managed_blocks() {
        let home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("scoped").expect("enable caveman");
        assert!(super::disable_caveman_integration().expect("disable caveman"));
        let claude_path = home.path().join(".claude").join("CLAUDE.md");
        assert!(!fs::read_to_string(&claude_path)
            .expect("read claude")
            .contains("headroom:caveman"));

        super::enable_caveman_integration("scoped").expect("enable again");
        super::perform_full_cleanup();
        let codex_path = home.path().join(".codex").join("AGENTS.md");
        assert!(!fs::read_to_string(codex_path)
            .expect("read codex")
            .contains("headroom:caveman"));
    }

    #[test]
    #[serial_test::serial]
    fn apply_then_verify_claude_code_writes_expected_files() {
        let home = TestHome::new();
        // Seed an empty zshrc/zshenv so the shell-block writers have files to
        // edit and don't depend on the dev's real shell config layout.
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(
            home.path().join(".claude").join("settings.json"),
            r#"{"hooks": {}}"#,
        )
        .unwrap();
        seed_installed_rtk();

        let result = super::apply_client_setup("claude_code").expect("apply_client_setup succeeds");
        assert!(result.applied);
        assert_eq!(result.client_id, "claude_code");

        // Hook script and settings.json hook entry must be present.
        let hook_path = home
            .path()
            .join(".claude")
            .join("hooks")
            .join("headroom-rtk-rewrite.sh");
        assert!(hook_path.exists(), "hook script written to disk");
        let hook_contents = fs::read_to_string(&hook_path).unwrap();
        assert!(
            hook_contents.starts_with("#!/usr/bin/env bash"),
            "hook has expected shebang"
        );

        let settings = read_settings_json(&home.path().join(".claude").join("settings.json"));
        assert_eq!(
            settings["env"]["ANTHROPIC_BASE_URL"].as_str(),
            Some("http://127.0.0.1:6767"),
            "claude settings.json points env at headroom proxy"
        );
        let pre_tool_use = &settings["hooks"]["PreToolUse"];
        assert!(
            pre_tool_use.is_array() && !pre_tool_use.as_array().unwrap().is_empty(),
            "PreToolUse hook entry exists, got: {settings}"
        );

        // Shell block in zshenv (or whichever profile the writer chose) should
        // export ANTHROPIC_BASE_URL pointing at the loopback proxy.
        let zshrc = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let combined = format!("{zshrc}\n{zshenv}");
        assert!(
            combined.contains("ANTHROPIC_BASE_URL=http://127.0.0.1:6767"),
            "ANTHROPIC_BASE_URL exported from a managed shell block, got:\n{combined}"
        );

        // verify_client_setup should report all the configured checks.
        // Proxy reachability is reported via `proxy_reachable` only, so a
        // missing proxy in the test environment no longer flips `verified`.
        let verification =
            super::verify_client_setup("claude_code").expect("verify_client_setup succeeds");
        assert_eq!(verification.client_id, "claude_code");
        assert!(
            verification
                .checks
                .iter()
                .any(|c| c.contains("ANTHROPIC_BASE_URL")),
            "verification reports the env check, got: {:?}",
            verification.checks
        );
        assert!(
            verification
                .checks
                .iter()
                .any(|c| c.contains("RTK Claude hook")),
            "verification reports the hook check, got: {:?}",
            verification.checks
        );
    }

    #[test]
    #[serial_test::serial]
    fn verify_claude_code_passes_when_rtk_deliberately_disabled() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(
            home.path().join(".claude").join("settings.json"),
            r#"{"hooks": {}}"#,
        )
        .unwrap();

        super::apply_client_setup("claude_code").expect("apply_client_setup succeeds");

        // User turns RTK off: this strips the RTK PATH block + hook but leaves
        // ANTHROPIC_BASE_URL routing intact, and persists the opt-out.
        super::set_rtk_enabled(false, home.path(), home.path()).expect("disable RTK");

        let hook_path = home
            .path()
            .join(".claude")
            .join("hooks")
            .join("headroom-rtk-rewrite.sh");
        assert!(!hook_path.exists(), "RTK hook removed when RTK disabled");

        // Routing config is still present, so Claude Code must verify green
        // even though the RTK pieces are gone.
        let verification =
            super::verify_client_setup("claude_code").expect("verify_client_setup succeeds");
        assert!(
            verification.verified,
            "claude_code verifies on routing alone when RTK is disabled, failures: {:?}",
            verification.failures
        );
        assert!(
            verification.failures.iter().all(|f| !f.contains("RTK")),
            "no RTK failures reported when RTK is disabled, got: {:?}",
            verification.failures
        );
    }

    #[test]
    #[serial_test::serial]
    fn verify_claude_code_passes_when_rtk_not_installed() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(
            home.path().join(".claude").join("settings.json"),
            r#"{"hooks": {}}"#,
        )
        .unwrap();

        // Clean install with RTK auto-install removed: routing is configured but
        // the managed RTK binary was never dropped on disk and the user never
        // toggled RTK off (rtk_disabled stays false). Claude Code must still
        // verify green on routing alone.
        super::apply_client_setup("claude_code").expect("apply_client_setup succeeds");

        assert!(
            !super::default_headroom_rtk_path().exists(),
            "RTK binary must be absent for this test"
        );
        let state = super::load_setup_state();
        assert!(
            !state.rtk_disabled,
            "rtk_disabled stays false when untoggled"
        );

        let verification =
            super::verify_client_setup("claude_code").expect("verify_client_setup succeeds");
        assert!(
            verification.verified,
            "claude_code verifies on routing alone when RTK isn't installed, failures: {:?}",
            verification.failures
        );
        assert!(
            verification.failures.iter().all(|f| !f.contains("RTK")),
            "no RTK failures reported when RTK isn't installed, got: {:?}",
            verification.failures
        );
    }

    #[test]
    #[serial_test::serial]
    fn ensure_rtk_integrations_writes_codex_nudge_and_disable_removes_it() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(home.path().join(".claude").join("settings.json"), "{}").unwrap();

        // Mark Codex as a configured client so the AGENTS.md nudge path runs.
        let mut state = super::load_setup_state();
        state
            .configured_clients
            .insert("codex_cli".into(), "now".into());
        super::write_setup_state(&state).unwrap();

        // Fake managed rtk + python binaries so the binary-present guard passes.
        let bin_dir = home.path().join("managed-bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let rtk = bin_dir.join("rtk");
        fs::write(&rtk, "#!/bin/sh\n").unwrap();
        let python = bin_dir.join("python3");
        fs::write(&python, "#!/bin/sh\n").unwrap();

        super::ensure_rtk_integrations(&rtk, &python).expect("ensure_rtk_integrations");

        let agents = home.path().join(".codex").join("AGENTS.md");
        let body = fs::read_to_string(&agents).expect("AGENTS.md written");
        assert!(
            body.contains("Headroom RTK"),
            "nudge heading present: {body}"
        );
        assert!(
            body.contains(&rtk.display().to_string()),
            "nudge references the managed rtk path: {body}"
        );

        // Disabling RTK must remove the managed block.
        super::set_rtk_enabled(false, &rtk, &python).expect("disable rtk");
        let after = fs::read_to_string(&agents).unwrap_or_default();
        assert!(
            !after.contains("Headroom RTK"),
            "nudge removed on disable: {after}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_claude_code_is_byte_idempotent() {
        // Regression: a second apply used to add blank-line padding between
        // managed blocks, so byte-exact idempotency now holds and is
        // asserted here.
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        seed_installed_rtk();

        super::apply_client_setup("claude_code").expect("first apply");
        let zshrc_after_first = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv_after_first = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let settings_after_first =
            fs::read_to_string(home.path().join(".claude").join("settings.json")).unwrap();
        let hook_after_first = fs::read_to_string(
            home.path()
                .join(".claude")
                .join("hooks")
                .join("headroom-rtk-rewrite.sh"),
        )
        .unwrap();

        super::apply_client_setup("claude_code").expect("second apply");
        let zshrc_after_second = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv_after_second = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let settings_after_second =
            fs::read_to_string(home.path().join(".claude").join("settings.json")).unwrap();
        let hook_after_second = fs::read_to_string(
            home.path()
                .join(".claude")
                .join("hooks")
                .join("headroom-rtk-rewrite.sh"),
        )
        .unwrap();

        assert_eq!(zshrc_after_first, zshrc_after_second, "zshrc byte-stable");
        assert_eq!(
            zshenv_after_first, zshenv_after_second,
            "zshenv byte-stable"
        );
        assert_eq!(
            settings_after_first, settings_after_second,
            "settings.json byte-stable"
        );
        assert_eq!(
            hook_after_first, hook_after_second,
            "hook script byte-stable"
        );

        // Sanity: each managed block still appears exactly once.
        let combined = format!("{zshrc_after_second}\n{zshenv_after_second}");
        assert_eq!(
            combined.matches("# >>> headroom:claude_code >>>").count(),
            1
        );
        assert_eq!(
            combined.matches("# >>> headroom:managed_rtk >>>").count(),
            1
        );
    }

    #[test]
    #[serial_test::serial]
    fn disable_then_clear_claude_code_removes_traces() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        seed_installed_rtk();

        super::apply_client_setup("claude_code").expect("apply");
        let hook_path = home
            .path()
            .join(".claude")
            .join("hooks")
            .join("headroom-rtk-rewrite.sh");
        assert!(hook_path.exists(), "hook present after apply");

        super::disable_client_setup("claude_code").expect("disable");

        // Hook script removed.
        assert!(!hook_path.exists(), "hook removed after disable");

        // Shell blocks removed.
        let zshrc = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let combined = format!("{zshrc}\n{zshenv}");
        assert!(
            !combined.contains("ANTHROPIC_BASE_URL=http://127.0.0.1:6767"),
            "ANTHROPIC_BASE_URL export removed, got:\n{combined}"
        );

        // settings.json no longer points env at the proxy and no longer carries
        // the Headroom hook entry.
        let settings = read_settings_json(&home.path().join(".claude").join("settings.json"));
        assert!(
            settings["env"]["ANTHROPIC_BASE_URL"].is_null(),
            "ANTHROPIC_BASE_URL stripped from settings.json env, got: {settings}"
        );
        let still_has_headroom_hook =
            claude_hook_present_in_value(&settings, "headroom-rtk-rewrite.sh");
        assert!(
            !still_has_headroom_hook,
            "Headroom hook entry stripped from settings.json, got: {settings}"
        );

        // clear_client_setups runs disable across all clients without error,
        // and the setup state file is left without a `claude_code` entry.
        super::clear_client_setups().expect("clear");
        let post = super::load_setup_state();
        assert!(
            post.configured_clients.get("claude_code").is_none(),
            "claude_code dropped from configured_clients, got: {:?}",
            post.configured_clients
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_then_verify_then_disable_codex_round_trip() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();

        let result = super::apply_client_setup("codex").expect("apply_client_setup succeeds");
        assert!(result.applied);
        assert_eq!(result.client_id, "codex");

        // Managed provider block lands in ~/.codex/config.toml.
        let config_toml = home.path().join(".codex").join("config.toml");
        let toml = fs::read_to_string(&config_toml).expect("codex config.toml written");
        assert!(
            toml.contains("# >>> headroom:codex_cli >>>"),
            "managed marker present, got:\n{toml}"
        );
        assert!(
            toml.contains("model_provider = \"headroom\""),
            "model_provider set, got:\n{toml}"
        );
        assert!(
            toml.contains("base_url = \"http://127.0.0.1:6767/v1\""),
            "provider base_url points at proxy, got:\n{toml}"
        );
        // No ~/.codex/auth.json in this test ⇒ not ChatGPT-OAuth ⇒ the flag is
        // omitted (it would force an OpenAI OAuth login for API-key users, #406).
        assert!(
            !toml.contains("requires_openai_auth"),
            "requires_openai_auth must NOT be written without ChatGPT auth, got:\n{toml}"
        );

        // OPENAI_BASE_URL exported from a managed shell block.
        let zshrc = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let combined = format!("{zshrc}\n{zshenv}");
        assert!(
            combined.contains("OPENAI_BASE_URL=http://127.0.0.1:6767/v1"),
            "OPENAI_BASE_URL exported from a managed shell block, got:\n{combined}"
        );

        // verify_client_setup reports the configured checks and passes.
        let verification =
            super::verify_client_setup("codex").expect("verify_client_setup succeeds");
        assert_eq!(verification.client_id, "codex");
        assert!(
            verification.failures.is_empty(),
            "no verification failures, got: {:?}",
            verification.failures
        );
        assert!(
            verification
                .checks
                .iter()
                .any(|c| c.contains("config.toml")),
            "verification reports the toml check, got: {:?}",
            verification.checks
        );

        // Disable strips both the toml block and the shell export.
        super::disable_client_setup("codex").expect("disable_client_setup succeeds");
        let toml_after = fs::read_to_string(&config_toml).unwrap_or_default();
        assert!(
            !toml_after.contains("# >>> headroom:codex_cli >>>"),
            "managed block removed on disable, got:\n{toml_after}"
        );
        let combined_after = format!(
            "{}\n{}",
            fs::read_to_string(home.path().join(".zshrc")).unwrap(),
            fs::read_to_string(home.path().join(".zshenv")).unwrap(),
        );
        assert!(
            !combined_after.contains("OPENAI_BASE_URL=http://127.0.0.1:6767/v1"),
            "shell export removed on disable, got:\n{combined_after}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn verify_codex_accepts_config_provider_without_shell_export() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        super::apply_client_setup("codex").expect("apply_client_setup succeeds");
        for shell_profile in [
            ".zshrc",
            ".zshenv",
            ".zprofile",
            ".bashrc",
            ".bash_profile",
            ".profile",
        ] {
            fs::write(home.path().join(shell_profile), "# user shell profile\n").unwrap();
        }

        let verification =
            super::verify_client_setup("codex").expect("verify_client_setup succeeds");
        assert!(
            verification.failures.is_empty(),
            "config-provider-only routing should pass, got: {:?}",
            verification.failures
        );
        assert!(
            verification
                .checks
                .iter()
                .any(|check| check.contains("config.toml provider routing is active")),
            "verification should report config-only routing evidence, got: {:?}",
            verification.checks
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_is_byte_idempotent() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();

        super::apply_client_setup("codex").expect("first apply");
        let config_toml = home.path().join(".codex").join("config.toml");
        let toml_first = fs::read_to_string(&config_toml).unwrap();
        let zshenv_first = fs::read_to_string(home.path().join(".zshenv")).unwrap();

        super::apply_client_setup("codex").expect("second apply");
        let toml_second = fs::read_to_string(&config_toml).unwrap();
        let zshenv_second = fs::read_to_string(home.path().join(".zshenv")).unwrap();

        assert_eq!(toml_first, toml_second, "config.toml byte-stable");
        assert_eq!(zshenv_first, zshenv_second, "zshenv byte-stable");
        assert_eq!(
            toml_second.matches("# >>> headroom:codex_cli >>>").count(),
            1,
            "managed block appears exactly once"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_preview_and_execute_restores_codex_backup() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let config_toml = codex_dir.join("config.toml");
        let original = "model = \"gpt-5\"\n[profiles.default]\napproval_policy = \"never\"\n";
        fs::write(&config_toml, original).unwrap();

        super::apply_client_setup("codex").expect("apply codex");
        let preview = super::preview_managed_rollback("codex-routing").expect("preview rollback");

        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert!(preview.backup_exists);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore headroom:codex_cli for Codex routing"
        );
        let backup_path = preview.backup_path.expect("backup path");

        let result = super::execute_managed_rollback(
            "codex-routing",
            &backup_path,
            "Restore headroom:codex_cli for Codex routing",
        )
        .expect("execute rollback");

        assert_eq!(result.record_id, "codex-routing");
        assert_eq!(result.restored_from, backup_path);
        assert!(
            result.safety_backup_path.is_some(),
            "fresh safety backup is created before restore"
        );
        assert_eq!(fs::read_to_string(&config_toml).unwrap(), original);
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_backup_outside_codex_config_directory() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(codex_dir.join("config.toml"), "model = \"gpt-5\"\n").unwrap();
        super::apply_client_setup("codex").expect("apply codex");
        let wrong_backup = home.path().join("config.toml.headroom-backup-wrong");
        fs::write(&wrong_backup, "model = \"gpt-4\"\n").unwrap();

        let err = super::execute_managed_rollback(
            "codex-routing",
            wrong_backup.to_str().unwrap(),
            "Restore headroom:codex_cli for Codex routing",
        )
        .expect_err("wrong backup must be rejected");

        assert!(
            err.to_string().contains("must live next to"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_missing_codex_marker() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let config_toml = codex_dir.join("config.toml");
        fs::write(&config_toml, "model = \"gpt-5\"\n").unwrap();
        super::apply_client_setup("codex").expect("apply codex");
        let preview = super::preview_managed_rollback("codex-routing").expect("preview");
        let backup_path = preview.backup_path.expect("backup");
        fs::write(&config_toml, "model = \"gpt-5\"\n").unwrap();

        let err = super::execute_managed_rollback(
            "codex-routing",
            &backup_path,
            "Restore headroom:codex_cli for Codex routing",
        )
        .expect_err("missing marker must be rejected");

        assert!(
            err.to_string().contains("marker is missing"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_preview_and_execute_restores_opencode_backup() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let config_json = opencode_dir.join("opencode.json");
        let original = serde_json::json!({
            "provider": {
                "openai": {
                    "npm": "@ai-sdk/openai",
                    "name": "OpenAI",
                    "options": {
                        "baseURL": "https://api.openai.com/v1"
                    }
                }
            },
            "theme": "system"
        });
        fs::write(
            &config_json,
            serde_json::to_vec_pretty(&original).expect("serialize original opencode"),
        )
        .unwrap();

        super::apply_client_setup("opencode").expect("apply opencode");
        let preview =
            super::preview_managed_rollback("opencode-routing").expect("preview rollback");

        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert!(preview.backup_exists);
        assert!(preview
            .evidence
            .join(" ")
            .contains("Relaunch-survival evidence"));
        assert_eq!(
            preview.confirmation_phrase,
            "Restore headroom:opencode for OpenCode routing"
        );
        let backup_path = preview.backup_path.expect("backup path");

        let result = super::execute_managed_rollback(
            "opencode-routing",
            &backup_path,
            "Restore headroom:opencode for OpenCode routing",
        )
        .expect("execute rollback");

        assert_eq!(result.record_id, "opencode-routing");
        assert_eq!(result.restored_from, backup_path);
        assert!(
            result.safety_backup_path.is_some(),
            "fresh safety backup is created before restore"
        );
        assert!(result
            .verification
            .join(" ")
            .contains("Relaunch-survival evidence"));
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_json).unwrap()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    #[serial_test::serial]
    fn managed_config_apply_preview_and_execute_promotes_opencode_safely() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let config_json = opencode_dir.join("opencode.json");
        let original = serde_json::json!({
            "provider": {
                "openai": {
                    "name": "OpenAI",
                    "options": {
                        "baseURL": "https://api.openai.com/v1"
                    }
                }
            },
            "theme": "system"
        });
        fs::write(
            &config_json,
            serde_json::to_vec_pretty(&original).expect("serialize original opencode"),
        )
        .unwrap();

        let preview =
            super::preview_managed_config_apply("opencode-routing").expect("preview apply");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.confirmation_phrase.starts_with(&format!(
            "Apply headroom:opencode to {} after reviewing ",
            config_json.display()
        )));
        assert!(preview.current_state.contains("OpenAI"));
        assert!(preview.proposed_state.contains("AI Switchboard"));
        assert!(preview.proposed_state.contains("\"theme\": \"system\""));
        assert!(preview.rollback_preview.contains("Rollback Center"));

        let result =
            super::execute_managed_config_apply("opencode-routing", &preview.confirmation_phrase)
                .expect("execute apply");
        assert!(result.changed);
        assert!(result.backup_path.is_some());
        assert!(result
            .verification
            .join(" ")
            .contains("provider.headroom matches"));
        assert!(super::opencode_provider_config_matches().expect("verify opencode"));

        let applied: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_json).unwrap()).unwrap();
        assert_eq!(applied["theme"], "system");
        assert_eq!(
            applied["provider"]["openai"],
            original["provider"]["openai"]
        );
        assert_eq!(
            applied["provider"]["headroom"]["options"]["baseURL"],
            super::HEADROOM_OPENAI_BASE_URL
        );

        let rollback = super::execute_managed_rollback(
            "opencode-routing",
            result.backup_path.as_deref().expect("backup"),
            "Restore headroom:opencode for OpenCode routing",
        )
        .expect("rollback applied config");
        assert_eq!(rollback.record_id, "opencode-routing");
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_json).unwrap()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    #[serial_test::serial]
    fn cursor_sidecar_apply_is_profile_aware_and_preserves_user_owned_files() {
        let home = TestHome::new();
        let cursor_root = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Cursor");
        let profile_settings = cursor_root
            .join("User")
            .join("profiles")
            .join("work")
            .join("settings.json");
        fs::create_dir_all(profile_settings.parent().unwrap()).unwrap();
        let profile_contents = r#"{"cursor.model":"user-selected","token":"must-not-read"}"#;
        fs::write(&profile_settings, profile_contents).unwrap();

        let status = super::detect_cursor_client();
        assert!(status.installed);
        assert!(status
            .notes
            .iter()
            .any(|note| note.contains("User/profiles/work/settings.json")));
        assert_eq!(
            fs::read_to_string(&profile_settings).unwrap(),
            profile_contents
        );

        let sidecar = cursor_root.join(super::SWITCHBOARD_ROUTING_FILE);
        fs::write(&sidecar, "# user-owned cursor note\nkeep this\n").unwrap();
        let preview = super::preview_managed_config_apply(super::CURSOR_SIDECAR_APPLY_RECORD_ID)
            .expect("preview Cursor sidecar apply");
        assert_eq!(preview.record_id, super::CURSOR_SIDECAR_APPLY_RECORD_ID);
        assert_eq!(preview.target_path, sidecar.display().to_string());
        assert!(preview.current_state.contains("user-owned cursor note"));
        assert!(preview.proposed_state.contains("headroom:cursor"));
        assert!(!preview.proposed_state.contains("user-selected"));
        assert!(preview.confirmation_phrase.contains("after reviewing"));
        assert_eq!(
            fs::read_to_string(&sidecar).unwrap(),
            "# user-owned cursor note\nkeep this\n"
        );

        let result = super::execute_managed_config_apply(
            super::CURSOR_SIDECAR_APPLY_RECORD_ID,
            &preview.confirmation_phrase,
        )
        .expect("apply Cursor sidecar");
        assert!(result.changed);
        assert!(result.backup_path.is_some());
        assert!(super::planned_switchboard_sidecar_matches("cursor").unwrap());
        assert_eq!(
            fs::read_to_string(&profile_settings).unwrap(),
            profile_contents
        );

        let rollback = super::execute_managed_rollback(
            "cursor-routing",
            "",
            "Restore headroom:cursor for Cursor routing",
        )
        .expect("remove only Cursor sidecar block");
        assert!(rollback.safety_backup_path.is_some());
        assert_eq!(
            fs::read_to_string(&sidecar).unwrap(),
            "# user-owned cursor note\nkeep this\n"
        );
        assert_eq!(
            fs::read_to_string(&profile_settings).unwrap(),
            profile_contents
        );
    }

    #[test]
    #[serial_test::serial]
    fn cursor_sidecar_apply_rejects_stale_or_wrong_confirmation() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join(super::SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        fs::write(&sidecar, "first\n").unwrap();
        let preview = super::preview_managed_config_apply(super::CURSOR_SIDECAR_APPLY_RECORD_ID)
            .expect("preview Cursor sidecar");
        fs::write(&sidecar, "changed outside Switchboard\n").unwrap();

        let err = super::execute_managed_config_apply(
            super::CURSOR_SIDECAR_APPLY_RECORD_ID,
            &preview.confirmation_phrase,
        )
        .expect_err("stale confirmation must be rejected");
        assert!(err.to_string().contains("confirmation phrase"));
        assert_eq!(
            fs::read_to_string(&sidecar).unwrap(),
            "changed outside Switchboard\n"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_config_apply_preview_and_execute_promotes_zed_rollback_safely() {
        let home = TestHome::new();
        let zed_dir = home.path().join(".config").join("zed");
        fs::create_dir_all(&zed_dir).unwrap();
        let settings_json = zed_dir.join("settings.json");
        let original = serde_json::json!({
            "theme": "One Dark",
            "assistant": { "default_model": "claude-3-5-sonnet" }
        });
        fs::write(
            &settings_json,
            serde_json::to_vec_pretty(&original).unwrap(),
        )
        .unwrap();

        let preview =
            super::preview_managed_config_apply("zed-ai-routing").expect("preview zed apply");
        assert_eq!(preview.record_id, "zed-ai-routing");
        assert!(preview.target_path.ends_with(".config/zed/settings.json"));
        assert!(preview.current_state.contains("One Dark"));
        assert!(preview.proposed_state.contains("anthropic.baseUrl"));
        assert!(preview.rollback_preview.contains("Rollback Center"));

        let result =
            super::execute_managed_config_apply("zed-ai-routing", &preview.confirmation_phrase)
                .expect("execute zed apply");
        assert!(result.changed);
        assert!(result.backup_path.is_some());
        assert!(super::zed_provider_config_matches().expect("verify zed"));

        let applied: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings_json).unwrap()).unwrap();
        assert_eq!(applied["theme"], "One Dark");
        assert_eq!(applied["assistant"]["default_model"], "claude-3-5-sonnet");
        assert_eq!(
            applied["anthropic.baseUrl"],
            super::HEADROOM_ANTHROPIC_BASE_URL
        );

        let rollback_preview =
            super::preview_managed_rollback("zed-ai-routing").expect("preview zed rollback");
        assert_eq!(rollback_preview.record_id, "zed-ai-routing");
        assert_eq!(rollback_preview.marker, "headroom:zed");
        assert!(rollback_preview.backup_path.is_some());
        assert!(rollback_preview
            .proposed_action
            .contains("Restore the Zed settings"));

        let rollback = super::execute_managed_rollback(
            "zed-ai-routing",
            result.backup_path.as_deref().expect("backup"),
            "Restore headroom:zed for Zed routing",
        )
        .expect("rollback applied zed config");
        assert_eq!(rollback.record_id, "zed-ai-routing");
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings_json).unwrap()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    #[serial_test::serial]
    fn managed_config_apply_preview_and_execute_promotes_windsurf_rollback_safely() {
        let home = TestHome::new();
        let windsurf_dir = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Windsurf")
            .join("User");
        fs::create_dir_all(&windsurf_dir).unwrap();
        let settings_json = windsurf_dir.join("settings.json");
        let original = serde_json::json!({
            "workbench.colorTheme": "Quiet Light",
            "assistant": { "defaultModel": "claude-3-5-sonnet" }
        });
        fs::write(
            &settings_json,
            serde_json::to_vec_pretty(&original).unwrap(),
        )
        .unwrap();

        let preview = super::preview_managed_config_apply("windsurf-routing")
            .expect("preview windsurf apply");
        assert_eq!(preview.record_id, "windsurf-routing");
        assert!(preview
            .target_path
            .ends_with("Application Support/Windsurf/User/settings.json"));
        assert!(preview.current_state.contains("Quiet Light"));
        assert!(preview.proposed_state.contains("anthropic.baseUrl"));
        assert!(preview.rollback_preview.contains("Rollback Center"));

        let result =
            super::execute_managed_config_apply("windsurf-routing", &preview.confirmation_phrase)
                .expect("execute windsurf apply");
        assert!(result.changed);
        assert!(result.backup_path.is_some());
        assert!(super::windsurf_provider_config_matches().expect("verify windsurf"));

        let applied: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings_json).unwrap()).unwrap();
        assert_eq!(applied["workbench.colorTheme"], "Quiet Light");
        assert_eq!(applied["assistant"]["defaultModel"], "claude-3-5-sonnet");
        assert_eq!(
            applied["anthropic.baseUrl"],
            super::HEADROOM_ANTHROPIC_BASE_URL
        );

        let rollback_preview =
            super::preview_managed_rollback("windsurf-routing").expect("preview windsurf rollback");
        assert_eq!(rollback_preview.record_id, "windsurf-routing");
        assert_eq!(rollback_preview.marker, "headroom:windsurf");
        assert!(rollback_preview.backup_path.is_some());
        assert!(rollback_preview
            .proposed_action
            .contains("Restore the Windsurf settings"));

        let rollback = super::execute_managed_rollback(
            "windsurf-routing",
            result.backup_path.as_deref().expect("backup"),
            "Restore headroom:windsurf for Windsurf routing",
        )
        .expect("rollback applied windsurf config");
        assert_eq!(rollback.record_id, "windsurf-routing");
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings_json).unwrap()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    #[serial_test::serial]
    fn managed_config_apply_rejects_wrong_confirmation_for_opencode() {
        let _home = TestHome::new();
        let err = super::execute_managed_config_apply("opencode-routing", "Apply OpenCode")
            .expect_err("wrong confirmation must be rejected");
        assert!(
            err.to_string().contains("confirmation phrase"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_config_apply_rejects_opencode_drift_after_preview() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let config_json = opencode_dir.join("opencode.json");
        fs::write(&config_json, r#"{"provider":{},"theme":"system"}"#).unwrap();

        let preview =
            super::preview_managed_config_apply("opencode-routing").expect("preview apply");
        fs::write(&config_json, r#"{"provider":{},"theme":"midnight"}"#).unwrap();

        let err =
            super::execute_managed_config_apply("opencode-routing", &preview.confirmation_phrase)
                .expect_err("post-preview drift must be rejected");

        assert!(
            err.to_string().contains("confirmation phrase"),
            "unexpected error: {err:#}"
        );
        let current: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_json).unwrap()).unwrap();
        assert_eq!(current["theme"], "midnight");
        assert!(current["provider"].as_object().unwrap().is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_backup_outside_opencode_config_directory() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        fs::write(opencode_dir.join("opencode.json"), "{}").unwrap();
        super::apply_client_setup("opencode").expect("apply opencode");
        let wrong_backup = home.path().join("opencode.json.headroom-backup-wrong");
        fs::write(&wrong_backup, "{}").unwrap();

        let err = super::execute_managed_rollback(
            "opencode-routing",
            wrong_backup.to_str().unwrap(),
            "Restore headroom:opencode for OpenCode routing",
        )
        .expect_err("wrong backup must be rejected");

        assert!(
            err.to_string().contains("must live next to"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_backup_outside_promoted_editor_config_directories() {
        let home = TestHome::new();

        let windsurf_dir = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Windsurf")
            .join("User");
        fs::create_dir_all(&windsurf_dir).unwrap();
        fs::write(windsurf_dir.join("settings.json"), "{}").unwrap();
        super::apply_client_setup("windsurf").expect("apply windsurf");
        let wrong_windsurf_backup = home.path().join("settings.json.headroom-backup-wrong");
        fs::write(&wrong_windsurf_backup, "{}").unwrap();

        let windsurf_err = super::execute_managed_rollback(
            "windsurf-routing",
            wrong_windsurf_backup.to_str().unwrap(),
            "Restore headroom:windsurf for Windsurf routing",
        )
        .expect_err("wrong Windsurf backup must be rejected");

        assert!(
            windsurf_err.to_string().contains("must live next to"),
            "unexpected error: {windsurf_err:#}"
        );

        let zed_dir = home.path().join(".config").join("zed");
        fs::create_dir_all(&zed_dir).unwrap();
        fs::write(zed_dir.join("settings.json"), "{}").unwrap();
        super::apply_client_setup("zed_ai").expect("apply zed");
        let wrong_zed_backup = home.path().join("settings.json.headroom-backup-wrong");
        fs::write(&wrong_zed_backup, "{}").unwrap();

        let zed_preview =
            super::preview_managed_rollback("zed-ai-routing").expect("preview zed rollback");
        assert!(zed_preview
            .evidence
            .contains(&"Allowlisted rollback execution row: zed-ai-routing.".to_string()));

        let zed_err = super::execute_managed_rollback(
            "zed-ai-routing",
            wrong_zed_backup.to_str().unwrap(),
            "Restore headroom:zed for Zed routing",
        )
        .expect_err("wrong Zed backup must be rejected");

        assert!(
            zed_err.to_string().contains("must live next to"),
            "unexpected error: {zed_err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_missing_opencode_provider() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let config_json = opencode_dir.join("opencode.json");
        fs::write(&config_json, "{}").unwrap();
        super::apply_client_setup("opencode").expect("apply opencode");
        let preview = super::preview_managed_rollback("opencode-routing").expect("preview");
        let backup_path = preview.backup_path.expect("backup");
        fs::write(&config_json, "{}").unwrap();

        let err = super::execute_managed_rollback(
            "opencode-routing",
            &backup_path,
            "Restore headroom:opencode for OpenCode routing",
        )
        .expect_err("missing provider must be rejected");

        assert!(
            err.to_string().contains("marker is missing"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_emits_requires_openai_auth_for_chatgpt_users() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("auth.json"),
            "{\"auth_mode\":\"chatgpt\",\"tokens\":{\"account_id\":\"acct_123\"}}",
        )
        .unwrap();

        super::apply_client_setup("codex").expect("apply_client_setup succeeds");
        let toml = fs::read_to_string(codex_dir.join("config.toml")).unwrap();
        assert!(
            toml.contains("requires_openai_auth = true"),
            "ChatGPT-OAuth users need the flag for the account menu, got:\n{toml}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_omits_requires_openai_auth_for_api_key_users() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("auth.json"),
            "{\"auth_mode\":\"apikey\",\"OPENAI_API_KEY\":\"sk-test\"}",
        )
        .unwrap();

        super::apply_client_setup("codex").expect("apply_client_setup succeeds");
        let toml = fs::read_to_string(codex_dir.join("config.toml")).unwrap();
        assert!(
            !toml.contains("requires_openai_auth"),
            "API-key users must not be forced into an OpenAI OAuth login (#406), got:\n{toml}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_replaces_unmarked_legacy_headroom_provider_table() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("config.toml"),
            "model_provider = \"headroom\"\n\
model = \"gpt-5.5\"\n\n\
[model_providers.headroom]\n\
name = \"OpenAI via old Headroom proxy\"\n\
base_url = \"http://127.0.0.1:8787/v1\"\n\
supports_websockets = true\n\n\
[features]\n\
js_repl = false\n",
        )
        .unwrap();

        super::apply_client_setup("codex").expect("apply_client_setup repairs stale provider");
        let toml = fs::read_to_string(codex_dir.join("config.toml")).unwrap();
        let parsed: toml::Value = toml.parse().expect("repaired config parses");

        assert_eq!(
            toml.matches("[model_providers.headroom]").count(),
            1,
            "stale provider table should be replaced, got:\n{toml}"
        );
        assert!(
            !toml.contains("127.0.0.1:8787"),
            "stale proxy port should be removed, got:\n{toml}"
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|providers| providers.get("headroom"))
                .and_then(|headroom| headroom.get("base_url"))
                .and_then(|value| value.as_str()),
            Some(super::HEADROOM_OPENAI_BASE_URL),
            "managed provider should point at current Headroom proxy, got:\n{toml}"
        );
        assert_eq!(
            parsed
                .get("features")
                .and_then(|features| features.get("js_repl"))
                .and_then(|value| value.as_bool()),
            Some(false),
            "unrelated user tables should be preserved, got:\n{toml}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_keeps_root_keys_at_root_scope_when_config_ends_in_a_table() {
        // Regression for the `invalid type: string "headroom", expected a
        // boolean in features` error: a config whose last table is `[features]`
        // (boolean-only values) used to absorb the appended root keys.
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let config_toml = codex_dir.join("config.toml");
        fs::write(
            &config_toml,
            "model = \"gpt-5.4\"\n\n[features]\njs_repl = false\n",
        )
        .unwrap();

        super::apply_client_setup("codex").expect("apply succeeds");

        let raw = fs::read_to_string(&config_toml).unwrap();
        let parsed: toml::Value = raw
            .parse()
            .unwrap_or_else(|e| panic!("valid toml: {e}\n{raw}"));

        assert_eq!(
            parsed.get("model_provider").and_then(|v| v.as_str()),
            Some("headroom"),
            "model_provider must resolve at root scope, got:\n{raw}"
        );
        assert!(
            parsed
                .get("features")
                .and_then(|f| f.get("model_provider"))
                .is_none(),
            "model_provider must not leak into [features], got:\n{raw}"
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|m| m.get("headroom"))
                .and_then(|h| h.get("base_url"))
                .and_then(|v| v.as_str()),
            Some(super::HEADROOM_OPENAI_BASE_URL),
            "provider table base_url points at the proxy, got:\n{raw}"
        );
        // The user's own content survives untouched.
        assert_eq!(
            parsed.get("model").and_then(|v| v.as_str()),
            Some("gpt-5.4"),
            "existing root key preserved, got:\n{raw}"
        );
        assert_eq!(
            parsed
                .get("features")
                .and_then(|f| f.get("js_repl"))
                .and_then(|v| v.as_bool()),
            Some(false),
            "existing [features] table preserved, got:\n{raw}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_repairs_a_previously_corrupted_features_block() {
        // A machine upgraded mid-bug: the old single block sits at end-of-file,
        // its root keys absorbed into [features]. Re-applying must repair it so
        // the file parses and the keys resolve at root scope.
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let config_toml = codex_dir.join("config.toml");
        fs::write(
            &config_toml,
            "[features]\njs_repl = false\n\
             # >>> headroom:codex_cli >>>\n\
             model_provider = \"headroom\"\n\
             openai_base_url = \"http://127.0.0.1:6767/v1\"\n\n\
             [model_providers.headroom]\n\
             name = \"Headroom persistent proxy\"\n\
             base_url = \"http://127.0.0.1:6767/v1\"\n\
             supports_websockets = true\n\
             # <<< headroom:codex_cli <<<\n",
        )
        .unwrap();

        // The corrupted file is invalid against Codex's schema, but still parses
        // as TOML with the key wrongly nested under [features].
        let before: toml::Value = fs::read_to_string(&config_toml).unwrap().parse().unwrap();
        assert_eq!(
            before
                .get("features")
                .and_then(|f| f.get("model_provider"))
                .and_then(|v| v.as_str()),
            Some("headroom"),
            "precondition: corruption present"
        );

        super::apply_client_setup("codex").expect("re-apply repairs config");

        let after: toml::Value = fs::read_to_string(&config_toml).unwrap().parse().unwrap();
        assert_eq!(
            after.get("model_provider").and_then(|v| v.as_str()),
            Some("headroom")
        );
        assert!(after
            .get("features")
            .and_then(|f| f.get("model_provider"))
            .is_none());
    }

    #[test]
    #[serial_test::serial]
    fn write_setup_state_publishes_atomically() {
        let _home = TestHome::new();
        let mut state = super::ClientSetupState::default();
        state
            .configured_clients
            .insert("claude_code".into(), "2026-01-01T00:00:00+00:00".into());
        super::write_setup_state(&state).expect("write");

        let path = super::setup_state_path();
        assert!(path.exists(), "setup state file written");

        // The sibling .tmp file must not be left behind after a successful
        // publish — its presence would mean the rename step never happened.
        let tmp = {
            let mut s = path.clone().into_os_string();
            s.push(".tmp");
            std::path::PathBuf::from(s)
        };
        assert!(!tmp.exists(), "tmp file cleaned up by rename, got: {tmp:?}");

        // Round-trip survives.
        let reloaded = super::load_setup_state();
        assert!(reloaded.configured_clients.contains_key("claude_code"));
    }

    #[test]
    #[serial_test::serial]
    fn load_setup_state_falls_back_to_default_on_corrupt_file() {
        let _home = TestHome::new();
        let path = super::setup_state_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Simulate a torn / partial write that would have happened with the
        // pre-fix non-atomic writer. The retry path inside load_setup_state
        // re-reads after a short backoff and, when the file is still bad,
        // logs a warning and returns the default rather than panicking.
        std::fs::write(&path, b"{ not json").unwrap();

        let state = super::load_setup_state();
        assert!(state.configured_clients.is_empty());
        assert!(state.remembered_clients.is_empty());
    }

    fn seed_codex_threads_db(path: &Path, rows: &[(&str, &str)]) {
        let conn = Connection::open(path).unwrap();
        conn.execute(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, model_provider TEXT NOT NULL)",
            [],
        )
        .unwrap();
        for (id, provider) in rows {
            conn.execute(
                "INSERT INTO threads (id, model_provider) VALUES (?, ?)",
                [id, provider],
            )
            .unwrap();
        }
    }

    fn provider_count(path: &Path, provider: &str) -> i64 {
        let conn = Connection::open(path).unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM threads WHERE model_provider = ?1",
            [provider],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn enable_codex_retagging() {
        crate::codex_threads::set_codex_thread_retagging_settings(CodexThreadRetaggingSettings {
            codex_thread_retagging: CodexThreadRetaggingMode::Enabled,
        })
        .unwrap();
    }

    #[test]
    fn retag_one_codex_db_moves_only_matching_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state_5.sqlite");
        seed_codex_threads_db(
            &db,
            &[
                ("a", "openai"),
                ("b", "openai"),
                ("c", "headroom"),
                ("d", "anthropic"),
            ],
        );

        let moved = crate::codex_threads::retag_one_codex_db(&db, "openai", "headroom").unwrap();
        assert_eq!(moved, 2);
        assert_eq!(provider_count(&db, "openai"), 0);
        assert_eq!(provider_count(&db, "headroom"), 3);
        // Third-party providers are untouched.
        assert_eq!(provider_count(&db, "anthropic"), 1);

        // Reverse direction round-trips only the headroom rows.
        let back = crate::codex_threads::retag_one_codex_db(&db, "headroom", "openai").unwrap();
        assert_eq!(back, 3);
        assert_eq!(provider_count(&db, "headroom"), 0);
        assert_eq!(provider_count(&db, "openai"), 3);
        assert_eq!(provider_count(&db, "anthropic"), 1);
    }

    #[test]
    fn retag_one_codex_db_noop_without_threads_table() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state_5.sqlite");
        // Open creates an empty DB with no `threads` table.
        Connection::open(&db).unwrap();
        assert_eq!(
            crate::codex_threads::retag_one_codex_db(&db, "openai", "headroom").unwrap(),
            0
        );
    }

    #[test]
    #[serial_test::serial]
    fn retag_codex_thread_providers_silent_when_no_store() {
        let _home = TestHome::new();
        // No ~/.codex stores exist under the temp home: must not panic.
        crate::codex_threads::retag_codex_thread_providers("openai", "headroom");
    }

    #[test]
    #[serial_test::serial]
    fn codex_sqlite_store_expected_gates_on_sqlite_dir_not_config() {
        let home = TestHome::new();
        let codex = home.path().join(".codex");
        // CLI-only / pre-sqlite Codex: config + sessions but no sqlite/ store.
        std::fs::create_dir_all(codex.join("sessions")).unwrap();
        std::fs::write(codex.join("config.toml"), "").unwrap();
        assert!(
            !crate::codex_threads::codex_sqlite_store_expected(),
            "config/sessions alone must not trigger the moved-store warning"
        );
        // CLI store renamed loose in codex_home (version no longer parses) ->
        // expected, so the relocation gets flagged.
        std::fs::write(codex.join("state_5x.sqlite"), "").unwrap();
        assert!(crate::codex_threads::codex_sqlite_store_expected());
        std::fs::remove_file(codex.join("state_5x.sqlite")).unwrap();
        // GUI store dir present -> a missing state_<N>.sqlite is worth flagging.
        std::fs::create_dir_all(codex.join("sqlite")).unwrap();
        assert!(crate::codex_threads::codex_sqlite_store_expected());
    }

    #[test]
    #[serial_test::serial]
    fn retag_codex_threads_to_headroom_pulls_native_threads_back() {
        // Reproduces the app-update restart path: the quit handler left threads
        // tagged `openai`; launch must retag them back to `headroom`.
        let home = TestHome::new();
        let db = home.path().join(".codex").join("state_5.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        seed_codex_threads_db(&db, &[("a", "openai"), ("b", "openai"), ("c", "anthropic")]);
        enable_codex_retagging();

        crate::codex_threads::retag_codex_threads_to_headroom();

        assert_eq!(provider_count(&db, "headroom"), 2);
        assert_eq!(provider_count(&db, "openai"), 0);
        // Third-party threads are untouched.
        assert_eq!(provider_count(&db, "anthropic"), 1);
    }

    #[test]
    fn codex_store_version_parses_state_filename() {
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_5.sqlite")),
            Some(5)
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_42.sqlite")),
            Some(42)
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/config.toml")),
            None
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_.sqlite")),
            None
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_x.sqlite")),
            None
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_5.db")),
            None
        );
    }

    #[test]
    #[serial_test::serial]
    fn codex_home_honors_env_else_default() {
        let home = TestHome::new();
        // TestHome clears CODEX_HOME, so we fall back to $HOME/.codex.
        assert_eq!(codex_home(), home.path().join(".codex"));

        let custom = home.path().join("custom-codex");
        std::env::set_var("CODEX_HOME", &custom);
        assert_eq!(codex_home(), custom);

        // An empty value is ignored (treated as unset).
        std::env::set_var("CODEX_HOME", "");
        assert_eq!(codex_home(), home.path().join(".codex"));
    }

    #[test]
    #[serial_test::serial]
    fn discover_codex_state_dbs_finds_versioned_stores() {
        let home = TestHome::new();
        let codex = home.path().join(".codex");
        std::fs::create_dir_all(codex.join("sqlite")).unwrap();
        // GUI store under sqlite/, CLI store at the root, on different versions.
        std::fs::File::create(codex.join("sqlite").join("state_6.sqlite")).unwrap();
        std::fs::File::create(codex.join("state_5.sqlite")).unwrap();
        // A non-store file in the same dir must be ignored.
        std::fs::File::create(codex.join("config.toml")).unwrap();

        let versions: BTreeSet<u32> = crate::codex_threads::discover_codex_state_dbs()
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        assert_eq!(versions, BTreeSet::from([5, 6]));
    }

    #[test]
    #[serial_test::serial]
    fn retag_handles_unknown_store_version() {
        // Future-proofing: a Codex store-version bump (here state_99) must not
        // write until the schema version is verified.
        let home = TestHome::new();
        let db = home.path().join(".codex").join("state_99.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        seed_codex_threads_db(&db, &[("a", "openai"), ("b", "openai"), ("c", "anthropic")]);
        enable_codex_retagging();

        let report = crate::codex_threads::retag_codex_thread_providers("openai", "headroom");

        assert_eq!(report.reports.len(), 1);
        assert_eq!(report.reports[0].rows_changed, 0);
        assert!(report.reports[0]
            .skipped_reason
            .as_deref()
            .unwrap_or_default()
            .contains("unknown Codex store version"));
        assert_eq!(provider_count(&db, "headroom"), 0);
        assert_eq!(provider_count(&db, "openai"), 2);
        assert_eq!(provider_count(&db, "anthropic"), 1);
    }

    #[test]
    #[serial_test::serial]
    fn codex_retagging_defaults_to_ask_and_does_not_write() {
        let home = TestHome::new();
        let db = home.path().join(".codex").join("state_5.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        seed_codex_threads_db(&db, &[("a", "openai"), ("b", "openai")]);

        let settings = crate::codex_threads::get_codex_thread_retagging_settings();
        assert_eq!(
            settings.codex_thread_retagging,
            CodexThreadRetaggingMode::Ask
        );
        let report = crate::codex_threads::retag_codex_thread_providers("openai", "headroom");

        assert_eq!(report.mode, CodexThreadRetaggingMode::Ask);
        assert!(report.reports.is_empty());
        assert_eq!(provider_count(&db, "openai"), 2);
        assert!(!crate::codex_threads::codex_retagging_settings_path().exists());
    }

    #[test]
    #[serial_test::serial]
    fn enabled_codex_retagging_creates_backup_and_can_restore() {
        let home = TestHome::new();
        let db = home.path().join(".codex").join("state_5.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        seed_codex_threads_db(&db, &[("a", "openai"), ("b", "openai"), ("c", "anthropic")]);
        enable_codex_retagging();

        let report = crate::codex_threads::retag_codex_thread_providers("openai", "headroom");

        assert_eq!(report.mode, CodexThreadRetaggingMode::Enabled);
        assert_eq!(report.reports.len(), 1);
        let backup = report.reports[0].backup_path.as_ref().expect("backup path");
        assert!(Path::new(backup).exists());
        assert_eq!(report.reports[0].rows_changed, 2);
        assert_eq!(provider_count(&db, "headroom"), 2);

        let restored = crate::codex_threads::restore_codex_thread_db_backup(backup).unwrap();
        assert_eq!(restored.restored_path, db.display().to_string());
        assert_eq!(provider_count(&db, "openai"), 2);
        assert_eq!(provider_count(&db, "headroom"), 0);
        assert_eq!(provider_count(&db, "anthropic"), 1);
    }

    #[test]
    #[serial_test::serial]
    fn managed_footprint_report_is_redacted_and_lists_core_surfaces() {
        let home = TestHome::new();
        std::fs::create_dir_all(home.path().join(".codex")).unwrap();
        std::fs::write(
            home.path().join(".codex").join("config.toml"),
            "secret = true",
        )
        .unwrap();

        let report = client_footprint::get_managed_footprint();
        let ids = report
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<BTreeSet<_>>();

        assert!(ids.contains("app-storage"));
        assert!(ids.contains("legacy-storage"));
        assert!(ids.contains("codex-config"));
        assert!(ids.contains("claude-settings"));
        assert!(ids.contains("launch-agent"));
        assert!(ids.contains("keychain-mac-ai-switchboard"));
        assert!(ids.contains("gemini_cli-sidecar"));

        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("secret = true"));
        assert!(!serialized.contains("sk-"));
        assert!(serialized.contains("Keychain service: mac-ai-switchboard"));
        assert!(serialized.contains("*.headroom-backup-*"));
        assert!(!serialized.contains("*.headroom.bak"));
    }

    #[test]
    #[serial_test::serial]
    fn managed_footprint_marks_existing_paths_without_reading_values() {
        let home = TestHome::new();
        let sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        std::fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        std::fs::write(&sidecar, "token = sk-test").unwrap();

        let report = client_footprint::get_managed_footprint();
        let gemini = report
            .items
            .iter()
            .find(|item| item.id == "gemini_cli-sidecar")
            .expect("gemini footprint");

        assert!(gemini.exists);
        assert!(gemini.managed);
        assert!(gemini.reversible);
        assert!(!serde_json::to_string(&report).unwrap().contains("sk-test"));
    }

    #[test]
    #[serial_test::serial]
    fn uninstall_dry_run_lists_current_cleanup_targets() {
        let home = TestHome::new();
        let current_storage = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Mac AI Switchboard");
        std::fs::create_dir_all(&current_storage).unwrap();

        let report = client_footprint::uninstall_dry_run_report();
        let serialized = serde_json::to_string(&report).unwrap();

        assert!(serialized.contains("Mac AI Switchboard"));
        assert!(serialized.contains(client_footprint::APP_BUNDLE_ID));
        assert!(serialized.contains("keychain://com.tarunagarwal.mac-ai-switchboard.account"));
        assert!(serialized.contains("User repositories and source files are never deleted."));
        assert!(!serialized.contains("session-token="));

        let app_storage = report
            .targets
            .iter()
            .find(|target| target.id == "app-support-current")
            .expect("current app storage target");
        assert!(app_storage.exists);
        assert!(app_storage.managed);
        assert!(app_storage.requires_confirmation);
    }

    #[test]
    fn zed_config_path_returns_user_home_config_json() {
        let path = zed_config_path();
        assert!(path.to_string_lossy().contains(".config"));
        assert!(path.to_string_lossy().contains("zed"));
        assert!(path.to_string_lossy().ends_with("settings.json"));
    }

    #[test]
    fn zed_config_backup_pattern_matches_timestamped_backups() {
        let pattern = super::zed_config_backup_pattern();
        assert!(pattern.contains("settings.json"));
        assert!(pattern.contains("headroom-backup-"));
    }

    #[test]
    #[serial_test::serial]
    fn grok_native_endpoint_setup_preserves_config_and_supports_rollback_and_off_cleanup() {
        let home = TestHome::new();
        let config = grok_config_path();
        fs::create_dir_all(config.parent().unwrap()).expect("create Grok config parent");
        let original = "[cli]\nauto_update = false\n\n[models]\ndefault = \"grok-build\"\n\n[model.grok-build]\ncontext_window = 128000\n";
        fs::write(&config, original).expect("seed Grok config");

        let preview =
            super::preview_managed_config_apply("grok-routing").expect("native Grok preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.target_path.ends_with(".grok/config.toml"));
        assert!(preview.proposed_state.contains("[endpoints]"));
        assert!(preview
            .proposed_state
            .contains(super::GROK_HEADROOM_BASE_URL));
        assert!(preview
            .evidence
            .iter()
            .any(|item| item.contains("models_base_url")));

        let applied = super::execute_managed_config_apply(
            super::GROK_ROLLBACK_RECORD_ID,
            &preview.confirmation_phrase,
        )
        .expect("apply native Grok endpoint");
        assert!(applied.changed);
        assert!(super::grok_provider_config_matches().expect("verify native Grok"));
        let configured = fs::read_to_string(&config).expect("read configured Grok");
        assert!(configured.contains("auto_update = false"));
        assert!(configured.contains("default = \"grok-build\""));
        assert!(configured.contains("context_window = 128000"));
        assert!(configured.contains(super::GROK_HEADROOM_BASE_URL));
        assert!(!configured.contains("auth.json"));

        let rollback = super::preview_managed_rollback(super::GROK_ROLLBACK_RECORD_ID)
            .expect("native Grok rollback preview");
        assert_eq!(rollback.status, ManagedRollbackExecutionStatus::Ready);
        let backup = rollback.backup_path.clone().expect("Grok backup path");
        let rollback_result = super::execute_managed_rollback(
            super::GROK_ROLLBACK_RECORD_ID,
            &backup,
            &rollback.confirmation_phrase,
        )
        .expect("rollback native Grok");
        assert_eq!(rollback_result.record_id, super::GROK_ROLLBACK_RECORD_ID);
        assert_eq!(fs::read_to_string(&config).unwrap(), original);

        // Re-apply through the normal connector path to cover sidecar parity,
        // then Off cleanup must remove only Switchboard-owned artifacts.
        super::apply_client_setup("grok_cli").expect("apply Grok connector");
        assert!(
            super::verify_client_setup("grok_cli")
                .expect("verify Grok connector")
                .verified
        );
        super::disable_client_setup("grok_cli").expect("disable Grok connector");
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        assert!(!super::grok_provider_config_matches().unwrap());
        assert!(!planned_switchboard_sidecar_matches("grok_cli").unwrap());

        drop(home);
    }

    #[test]
    #[serial_test::serial]
    fn goose_native_endpoint_apply_exposes_rollback_center_restore() {
        let home = TestHome::new();
        let config = crate::goose_provider_configs::goose_config_path();
        fs::create_dir_all(config.parent().unwrap()).expect("create Goose config dir");
        let original = "active_provider: openai\nproviders:\n  openai:\n    enabled: true\n    model: gpt-4o\n    configured: true\nkeep: true\n";
        fs::write(&config, original).expect("seed Goose config");

        let preview = super::preview_managed_config_apply(
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID,
        )
        .expect("native Goose preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(preview.target_path, config.display().to_string());
        assert!(preview.proposed_state.contains(HEADROOM_OPENAI_BASE_URL));
        assert!(preview.evidence.iter().any(|item| {
            let item = item.to_ascii_lowercase();
            item.contains("allowlisted") && item.contains("endpoint")
        }));

        let applied = super::execute_managed_config_apply(
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID,
            &preview.confirmation_phrase,
        )
        .expect("apply native Goose endpoint");
        assert!(applied.changed);
        let backup = applied.backup_path.clone().expect("Goose backup path");
        assert!(crate::goose_provider_configs::goose_provider_config_matches().unwrap());
        let configured = fs::read_to_string(&config).expect("read configured Goose");
        assert!(configured.contains("active_provider: openai"));
        assert!(configured.contains("model: gpt-4o"));
        assert!(configured.contains("keep: true"));
        assert!(configured.contains(HEADROOM_OPENAI_BASE_URL));
        assert!(!configured.contains("secrets.yaml"));

        let rollback_preview = super::preview_managed_rollback(
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID,
        )
        .expect("native Goose rollback preview");
        assert_eq!(
            rollback_preview.status,
            ManagedRollbackExecutionStatus::Ready
        );
        assert!(rollback_preview.marker_present);
        assert!(rollback_preview.backup_exists);
        let rollback = super::execute_managed_rollback(
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID,
            &backup,
            &rollback_preview.confirmation_phrase,
        )
        .expect("rollback native Goose endpoint");
        assert_eq!(
            rollback.record_id,
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID
        );
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        assert!(!crate::goose_provider_configs::goose_provider_config_matches().unwrap());

        drop(home);
    }

    #[test]
    fn cursor_connector_has_fixture_home_dry_run_preview() {
        let detected_clients = Vec::new();
        let connectors = super::list_client_connectors(&detected_clients).expect("list connectors");
        let cursor = connectors
            .iter()
            .find(|connector| connector.client_id == "cursor")
            .expect("cursor connector");

        assert_eq!(cursor.support_status, ClientConnectorSupportStatus::Planned);
        assert_eq!(cursor.automation_gates.len(), 7);
        assert!(cursor
            .config_creation_step_details
            .iter()
            .any(|step| step.detail.contains("state.vscdb")));
        assert!(cursor
            .config_locations
            .iter()
            .any(|location| location.contains("Cursor/User/settings.json")));
        assert!(cursor
            .config_locations
            .iter()
            .any(|location| location.contains("Cursor/User/profiles/*/settings.json")));
        assert!(!cursor
            .config_locations
            .iter()
            .any(|location| location.contains("globalStorage")));

        let preview = cursor
            .config_dry_run_preview
            .as_ref()
            .expect("cursor dry-run preview");
        assert!(preview.target.contains("Cursor/User/settings.json"));
        assert!(preview.marker.contains("cursor"));
        assert!(preview.rollback_preview.contains("Switchboard-owned"));
        assert!(preview
            .apply_blocked_reason
            .contains("does not document a stable on-disk"));
        assert_eq!(preview.confirmation_phrase, "CURSOR NATIVE SCHEMA GATE");
    }

    #[test]
    fn grok_connector_exposes_documented_native_endpoint_and_credential_boundary() {
        let detected_clients = Vec::new();
        let connectors = super::list_client_connectors(&detected_clients).expect("list connectors");
        let grok = connectors
            .iter()
            .find(|connector| connector.client_id == "grok_cli")
            .expect("grok connector");

        assert_eq!(grok.support_status, ClientConnectorSupportStatus::Managed);
        assert!(!grok.enabled);
        assert!(grok
            .config_locations
            .iter()
            .any(|location| location.contains(".config/xai")));
        assert!(grok
            .config_locations
            .iter()
            .any(|location| location.contains(".grok/config.toml")));
        assert!(grok
            .automation_gates
            .iter()
            .any(|gate| gate.contains("models_base_url")));
        assert!(grok
            .automation_gates
            .iter()
            .any(|gate| gate.contains("XAI_API_KEY")));
        assert!(grok.config_dry_run_preview.is_none());
        assert!(grok.config_creation_step_details.is_empty());
    }

    #[test]
    fn continue_connector_exposes_managed_sidecar_without_native_writes() {
        let detected_clients = Vec::new();
        let connectors = super::list_client_connectors(&detected_clients).expect("list connectors");
        let continue_connector = connectors
            .iter()
            .find(|connector| connector.client_id == "continue")
            .expect("continue connector");

        assert_eq!(
            continue_connector.support_status,
            ClientConnectorSupportStatus::Managed
        );
        assert!(!continue_connector.enabled);
        assert!(!continue_connector.verified);
        assert!(continue_connector
            .config_locations
            .iter()
            .any(|location| location.contains(".continue")));
        assert!(continue_connector.config_creation_step_details.is_empty());
        assert!(continue_connector.config_dry_run_preview.is_none());
        assert!(continue_connector.automation_path.is_empty());
    }

    #[test]
    fn qwen_connector_exposes_managed_sidecar_without_provider_writes() {
        let _home = TestHome::new();
        let detected_clients = Vec::new();
        let connectors = super::list_client_connectors(&detected_clients).expect("list connectors");
        let qwen = connectors
            .iter()
            .find(|connector| connector.client_id == "qwen_code")
            .expect("qwen connector");

        assert_eq!(qwen.support_status, ClientConnectorSupportStatus::Managed);
        assert!(!qwen.enabled);
        assert!(!qwen.verified);
        assert!(qwen
            .config_locations
            .iter()
            .any(|location| location.contains(".qwen")));
        assert!(qwen
            .config_locations
            .iter()
            .any(|location| location.contains(".config/qwen")));
        assert!(qwen.config_creation_step_details.is_empty());

        assert!(qwen.config_dry_run_preview.is_none());
    }

    #[test]
    fn qwen_connector_applies_and_disables_switchboard_owned_sidecar_only() {
        let _home = TestHome::new();

        let result = super::apply_client_setup("qwen_code").expect("apply qwen sidecar");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert_eq!(result.client_id, "qwen_code");
        assert!(result
            .changed_files
            .iter()
            .any(|path| path.contains("mac-ai-switchboard-routing.md")));

        let routing_path = planned_sidecar_routing_path("qwen_code").expect("qwen sidecar path");
        let body = std::fs::read_to_string(&routing_path).expect("read qwen sidecar");
        assert!(body.contains("headroom:qwen_code"));
        assert!(body.contains("reversible Qwen Code routing-intent sidecar"));
        assert!(
            super::verify_client_setup("qwen_code")
                .expect("verify qwen sidecar")
                .verified
        );

        super::disable_client_setup("qwen_code").expect("disable qwen sidecar");
        assert!(
            !super::verify_client_setup("qwen_code")
                .expect("verify removed qwen sidecar")
                .verified
        );
        assert!(routing_path.exists());
        let cleaned = std::fs::read_to_string(&routing_path).expect("read cleaned qwen sidecar");
        assert!(!cleaned.contains("headroom:qwen_code"));
    }
}
