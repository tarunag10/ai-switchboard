use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::client_cleanup;
use crate::client_detection::{
    detect_aider_client, detect_amazon_q_client, detect_claude_code_client, detect_codex_client,
    detect_continue_client, detect_cursor_client, detect_gemini_cli_client, detect_goose_client,
    detect_grok_cli_client, detect_opencode_client, detect_qwen_code_client, detect_windsurf_client,
    detect_zed_ai_client,
};
use crate::client_footprint::managed_backup_targets;
use crate::client_paths::{
    claude_settings_candidates, headroom_markitdown_hook_path, headroom_rtk_hook_path,
    resolve_default_shell_targets, rtk_codex_agents_path,
};
use crate::client_setup_apply::{
    claude_settings_hook_matches, ensure_rtk_integrations_for_targets,
};
use crate::client_setup_state::setup_state_path;
use crate::managed_files::remove_shell_block;
use crate::models::ClientStatus;

pub use crate::client_connector_list::list_client_connectors;
pub use crate::client_integrations::{
    caveman_integration_matches_level, caveman_integration_snapshot, disable_caveman_integration,
    disable_markitdown_integration, enable_caveman_integration, enable_markitdown_integration,
    restore_caveman_client_if_unchanged, CavemanIntegrationSnapshot,
};
pub use crate::client_setup_apply::{
    apply_client_setup, clear_client_setups, codex_provider_block_matches, disable_client_setup,
    restore_client_setups, verify_client_setup,
};
pub use crate::client_setup_state::{
    is_claude_code_enabled, is_codex_enabled, load_savings_mode, load_setup_state,
    load_switchboard_mode, write_savings_mode, write_setup_state, write_switchboard_mode,
};

pub(crate) use crate::client_integrations::{
    build_headroom_markitdown_hook, build_markitdown_codex_nudge, build_markitdown_office_nudge,
};
pub(crate) use crate::client_paths::{default_shell_targets_for_family, serialize_paths, SWITCHBOARD_ROUTING_FILE};
pub(crate) use crate::client_provider_configs::{
    grok_provider_config_matches, opencode_provider_config_matches, windsurf_provider_config_matches,
    zed_config_backup_pattern, zed_provider_config_matches, HEADROOM_ANTHROPIC_BASE_URL,
    HEADROOM_OPENAI_BASE_URL, GROK_HEADROOM_BASE_URL, WINDSURF_MARKER_PREFIX, ZED_MARKER_PREFIX,
};
pub(crate) use crate::client_setup_apply::{
    build_headroom_rtk_hook, claude_hook_present_in_value, configure_planned_switchboard_sidecar,
    entry_contains_hook, execute_provider_sidecar_apply, planned_switchboard_sidecar_matches,
    preview_cursor_sidecar_apply, preview_provider_sidecar_apply, remove_pre_tool_use_markers,
    shell_block_contains_in_files, shell_block_contains_text_in_files, shell_double_quote,
    strip_headroom_hook_from_settings, CODEX_ROOT_BLOCK_ID, CURSOR_MARKER_PREFIX,
    CURSOR_SIDECAR_APPLY_RECORD_ID, CURSOR_SIDECAR_OWNER, GEMINI_BASE_URL_ENV_KEY,
    GOOSE_SIDECAR_APPLY_RECORD_ID, GOOSE_SIDECAR_OWNER, GROK_SIDECAR_APPLY_RECORD_ID,
    GROK_SIDECAR_OWNER,
};
pub(crate) use crate::client_setup_state::{
    configured_timestamp, default_headroom_managed_python_path, default_headroom_rtk_path,
    is_configured, normalize_setup_state, normalized_setup_id, resolve_client_shell_targets,
    resolve_client_shell_targets_for_cleanup, ClientSetupState,
};
pub(crate) use crate::client_sidecar_rollbacks::sidecar_rollback_target;
pub(crate) use crate::managed_files::{
    parse_json_object, remove_managed_block, upsert_managed_block, write_file_if_changed,
};

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

pub fn rtk_integration_status() -> Result<(bool, bool)> {
    let path_configured = crate::client_setup_apply::shell_block_contains_text_in_files(
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

    if let Err(err) = clear_client_setups() {
        log::warn!("cleanup: clear_client_setups failed: {err}");
    }

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

    if let Err(err) = remove_managed_block(&rtk_codex_agents_path(), "rtk") {
        log::warn!("cleanup: removing rtk AGENTS.md block failed: {err}");
    }

    if let Err(err) = disable_caveman_integration() {
        log::warn!("cleanup: removing caveman managed blocks failed: {err}");
    }

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

    for target in managed_backup_targets() {
        removed.extend(client_cleanup::sweep_managed_backups(&target));
    }

    removed
}

pub(crate) use crate::client_detection::{
    append_gemini_manual_routing_note, claude_code_user_state_exists, codex_home, codex_logged_in,
    detect_codex_cli, discover_editor_settings_files, find_on_path_entries, nvm_binary_candidates,
    planned_cli_compatibility_evidence, PlannedCliCompatibilityReport,
};

pub(crate) use crate::client_managed_config::{
    execute_managed_config_apply, execute_managed_rollback, execute_managed_rollback_undo_all,
    preview_managed_config_apply, preview_managed_rollback, preview_managed_rollback_undo_all,
    GROK_ROLLBACK_RECORD_ID,
};

#[cfg(test)]
#[path = "client_adapters_tests.rs"]
mod tests;
