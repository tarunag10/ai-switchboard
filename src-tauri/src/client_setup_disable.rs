use anyhow::{anyhow, Result};

use crate::client_claude_settings::{
    remove_claude_settings_env, remove_legacy_vscode_base_url_keys, remove_vscode_connector_keys,
    strip_headroom_hook_from_settings,
};
use crate::client_codex_setup::{disable_codex_cli, disable_codex_gui};
use crate::client_connector_status::MANAGED_CLIENT_SPECS;
use crate::client_connectors::{planned_sidecar_spec, PLANNED_SIDECAR_SPECS};
use crate::client_paths::{
    claude_settings_candidates, headroom_rtk_hook_path, planned_sidecar_routing_path,
};
use crate::client_provider_configs::{
    remove_grok_provider_config, remove_opencode_provider_config, remove_windsurf_provider_config,
    remove_zed_provider_config, HEADROOM_ANTHROPIC_BASE_URL,
};
use crate::client_setup_state::{
    load_setup_state, normalized_setup_id, resolve_client_shell_targets_for_cleanup,
    write_setup_state,
};
use crate::managed_files::{remove_managed_block, remove_shell_block};

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
        "continue" => {
            let _ = crate::continue_provider_configs::remove_continue_provider_config()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "aider" => {
            let _ = crate::aider_provider_configs::remove_aider_provider_config()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
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
