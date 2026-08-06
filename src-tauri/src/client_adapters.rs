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
use crate::client_detection::{
    detect_aider_client, detect_amazon_q_client, detect_claude_code_client, detect_codex_client,
    detect_continue_client, detect_cursor_client, detect_gemini_cli_client, detect_goose_client,
    detect_grok_cli_client, detect_opencode_client, detect_qwen_code_client, detect_windsurf_client,
    detect_zed_ai_client,
};
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

pub(crate) use crate::client_detection::{
    append_gemini_manual_routing_note, claude_code_user_state_exists, codex_home, codex_logged_in,
    detect_codex_cli, discover_editor_settings_files, find_on_path_entries, nvm_binary_candidates,
    planned_cli_compatibility_evidence, PlannedCliCompatibilityReport,
};

#[cfg(test)]
#[path = "client_adapters_tests.rs"]
mod tests;
