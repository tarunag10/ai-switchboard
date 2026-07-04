use tauri::State;

use crate::client_adapters;
use crate::models::DashboardState;
use crate::state::AppState;

#[tauri::command]
pub async fn install_addon(
    state: State<'_, AppState>,
    id: String,
) -> Result<DashboardState, String> {
    match id.as_str() {
        "markitdown" => {
            state
                .tool_manager
                .install_markitdown()
                .map_err(|err| err.to_string())?;
            let (changed_files, backup_files) = client_adapters::enable_markitdown_integration(
                &state.tool_manager.markitdown_entrypoint(),
                &state.tool_manager.markitdown_shim_path(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| {
                format!("markitdown installed but enabling integration failed: {err:#}")
            })?;
            let _ = state.record_markitdown_attribution(&changed_files, &backup_files);
            Ok(state.dashboard())
        }
        "rtk" => {
            state
                .tool_manager
                .install_rtk()
                .map_err(|err| err.to_string())?;
            client_adapters::set_rtk_enabled(
                true,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| format!("rtk installed but enabling integration failed: {err:#}"))?;
            Ok(state.dashboard())
        }
        "ponytail" => {
            state
                .tool_manager
                .install_ponytail()
                .map_err(|err| err.to_string())?;
            let hosts = state.tool_manager.ponytail_registered_hosts();
            let _ = state.record_ponytail_attribution(&hosts);
            Ok(state.dashboard())
        }
        "caveman" => {
            state
                .tool_manager
                .install_caveman()
                .map_err(|err| err.to_string())?;
            let level = state.tool_manager.caveman_level();
            let (changed_files, backup_files) = client_adapters::enable_caveman_integration(&level)
                .map_err(|err| {
                    format!("caveman installed but enabling guidance failed: {err:#}")
                })?;
            let _ = state.record_caveman_attribution(&level, &changed_files, &backup_files);
            Ok(state.dashboard())
        }
        other => Err(format!("unknown addon: {other}")),
    }
}

#[tauri::command]
pub async fn set_addon_enabled(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<DashboardState, String> {
    match id.as_str() {
        "markitdown" => {
            state
                .tool_manager
                .set_markitdown_enabled(enabled)
                .map_err(|err| err.to_string())?;
            if enabled {
                let (changed_files, backup_files) = client_adapters::enable_markitdown_integration(
                    &state.tool_manager.markitdown_entrypoint(),
                    &state.tool_manager.markitdown_shim_path(),
                    &state.tool_manager.managed_python(),
                )
                .map_err(|err| err.to_string())?;
                let _ = state.record_markitdown_attribution(&changed_files, &backup_files);
            } else {
                client_adapters::disable_markitdown_integration(
                    &state.tool_manager.markitdown_shim_path(),
                )
                .map_err(|err| err.to_string())?;
            }
            Ok(state.dashboard())
        }
        "ponytail" => {
            state
                .tool_manager
                .set_ponytail_enabled(enabled)
                .map_err(|err| err.to_string())?;
            if enabled {
                let hosts = state.tool_manager.ponytail_registered_hosts();
                let _ = state.record_ponytail_attribution(&hosts);
            }
            Ok(state.dashboard())
        }
        "caveman" => {
            state
                .tool_manager
                .set_caveman_enabled(enabled)
                .map_err(|err| err.to_string())?;
            if enabled {
                let level = state.tool_manager.caveman_level();
                let (changed_files, backup_files) =
                    client_adapters::enable_caveman_integration(&level)
                        .map_err(|err| err.to_string())?;
                let _ = state.record_caveman_attribution(&level, &changed_files, &backup_files);
            } else {
                client_adapters::disable_caveman_integration().map_err(|err| err.to_string())?;
            }
            Ok(state.dashboard())
        }
        other => Err(format!("unknown addon: {other}")),
    }
}

#[tauri::command]
pub async fn uninstall_addon(
    state: State<'_, AppState>,
    id: String,
) -> Result<DashboardState, String> {
    match id.as_str() {
        "markitdown" => {
            let _ = client_adapters::disable_markitdown_integration(
                &state.tool_manager.markitdown_shim_path(),
            );
            state
                .tool_manager
                .uninstall_markitdown()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "rtk" => {
            client_adapters::set_rtk_enabled(
                false,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state
                .tool_manager
                .uninstall_rtk()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "ponytail" => {
            state
                .tool_manager
                .uninstall_ponytail()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "caveman" => {
            let _ = client_adapters::disable_caveman_integration();
            state
                .tool_manager
                .uninstall_caveman()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        other => Err(format!("unknown addon: {other}")),
    }
}

#[tauri::command]
pub async fn set_caveman_level(
    state: State<'_, AppState>,
    level: String,
) -> Result<DashboardState, String> {
    state
        .tool_manager
        .set_caveman_level(&level)
        .map_err(|err| err.to_string())?;
    // Rewrite the managed blocks with the new level body when enabled.
    if state.tool_manager.caveman_receipt_exists()
        && state
            .tool_manager
            .list_tools()
            .iter()
            .any(|tool| tool.id == "caveman" && tool.enabled)
    {
        let level = state.tool_manager.caveman_level();
        let (changed_files, backup_files) =
            client_adapters::enable_caveman_integration(&level).map_err(|err| err.to_string())?;
        let _ = state.record_caveman_attribution(&level, &changed_files, &backup_files);
    }
    Ok(state.dashboard())
}
