use std::path::PathBuf;
use std::time::Duration;
use tauri::State;

use crate::client_adapters;
use crate::models::DashboardState;
use crate::process_runner::run_command_capture_with_timeout;
use crate::state::AppState;

#[tauri::command]
pub fn convert_markitdown_file(state: State<'_, AppState>, path: String) -> Result<String, String> {
    if !state.tool_manager.markitdown_installed() || !state.tool_manager.tool_enabled("markitdown")
    {
        return Err("MarkItDown must be installed and enabled before conversion.".into());
    }
    let source = PathBuf::from(path.trim());
    if source.as_os_str().is_empty() {
        return Err("Choose a document to convert.".into());
    }
    let source = source
        .canonicalize()
        .map_err(|error| format!("Cannot access the selected document: {error}"))?;
    let metadata = std::fs::metadata(&source)
        .map_err(|error| format!("Cannot inspect the selected document: {error}"))?;
    if !metadata.is_file() {
        return Err("The selected path is not a regular file.".into());
    }
    if metadata.len() > 25 * 1024 * 1024 {
        return Err("The selected document exceeds the 25 MB local conversion limit.".into());
    }
    let source_arg = source.to_string_lossy().into_owned();
    let (stdout, stderr) = run_command_capture_with_timeout(
        &state.tool_manager.markitdown_entrypoint(),
        &[source_arg.as_str()],
        &state.tool_manager.runtime_root_dir(),
        Duration::from_secs(120),
    )
    .map_err(|error| format!("MarkItDown conversion failed: {error}"))?;
    if stdout.trim().is_empty() {
        return Err(if stderr.trim().is_empty() {
            "MarkItDown returned no Markdown output.".into()
        } else {
            format!("MarkItDown returned no Markdown output: {}", stderr.trim())
        });
    }
    Ok(stdout)
}

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
            let integration = client_adapters::enable_markitdown_integration(
                &state.tool_manager.markitdown_entrypoint(),
                &state.tool_manager.markitdown_shim_path(),
                &state.tool_manager.managed_python(),
            );
            let (changed_files, backup_files) = match integration {
                Ok(result) => result,
                Err(err) => {
                    let cleanup = client_adapters::disable_markitdown_integration(
                        &state.tool_manager.markitdown_shim_path(),
                    )
                    .and_then(|_| state.tool_manager.uninstall_markitdown());
                    return Err(match cleanup {
                        Ok(()) => format!("markitdown activation failed and was rolled back: {err:#}"),
                        Err(cleanup_err) => format!(
                            "markitdown activation failed; rollback also failed: {err:#}; cleanup: {cleanup_err:#}"
                        ),
                    });
                }
            };
            let _ = state.record_markitdown_attribution(&changed_files, &backup_files);
            Ok(state.dashboard())
        }
        "rtk" => {
            state
                .tool_manager
                .install_rtk()
                .map_err(|err| err.to_string())?;
            let integration = client_adapters::set_rtk_enabled(
                true,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            );
            if let Err(err) = integration {
                let cleanup = client_adapters::set_rtk_enabled(
                    false,
                    &state.tool_manager.rtk_entrypoint(),
                    &state.tool_manager.managed_python(),
                )
                .and_then(|_| state.tool_manager.uninstall_rtk());
                return Err(match cleanup {
                    Ok(()) => format!("rtk activation failed and was rolled back: {err:#}"),
                    Err(cleanup_err) => format!(
                        "rtk activation failed; rollback also failed: {err:#}; cleanup: {cleanup_err:#}"
                    ),
                });
            }
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
            let integration = client_adapters::enable_caveman_integration(&level);
            let (changed_files, backup_files) = match integration {
                Ok(result) => result,
                Err(err) => {
                    let cleanup = client_adapters::disable_caveman_integration()
                        .and_then(|_| state.tool_manager.uninstall_caveman());
                    return Err(match cleanup {
                        Ok(()) => format!("caveman activation failed and was rolled back: {err:#}"),
                        Err(cleanup_err) => format!(
                            "caveman activation failed; rollback also failed: {err:#}; cleanup: {cleanup_err:#}"
                        ),
                    });
                }
            };
            let _ = state.record_caveman_attribution(&level, &changed_files, &backup_files);
            Ok(state.dashboard())
        }
        "leanctx" => {
            state
                .tool_manager
                .install_leanctx_sidecar()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "response-cache" | "semantic-cache" => {
            state
                .semantic_cache
                .set_enabled(false)
                .map_err(|err| err.to_string())?;
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
                let integration = client_adapters::enable_markitdown_integration(
                    &state.tool_manager.markitdown_entrypoint(),
                    &state.tool_manager.markitdown_shim_path(),
                    &state.tool_manager.managed_python(),
                );
                let (changed_files, backup_files) = match integration {
                    Ok(result) => result,
                    Err(err) => {
                        let rollback = client_adapters::disable_markitdown_integration(
                            &state.tool_manager.markitdown_shim_path(),
                        )
                        .and_then(|_| state.tool_manager.set_markitdown_enabled(false));
                        return Err(match rollback {
                            Ok(()) => format!("markitdown enable failed and was rolled back: {err:#}"),
                            Err(rollback_err) => format!(
                                "markitdown enable failed; rollback also failed: {err:#}; cleanup: {rollback_err:#}"
                            ),
                        });
                    }
                };
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
                let integration = client_adapters::enable_caveman_integration(&level);
                let (changed_files, backup_files) = match integration {
                    Ok(result) => result,
                    Err(err) => {
                        let rollback = client_adapters::disable_caveman_integration()
                            .and_then(|_| state.tool_manager.set_caveman_enabled(false));
                        return Err(match rollback {
                            Ok(()) => format!("caveman enable failed and was rolled back: {err:#}"),
                            Err(rollback_err) => format!(
                                "caveman enable failed; rollback also failed: {err:#}; cleanup: {rollback_err:#}"
                            ),
                        });
                    }
                };
                let _ = state.record_caveman_attribution(&level, &changed_files, &backup_files);
            } else {
                client_adapters::disable_caveman_integration().map_err(|err| err.to_string())?;
            }
            Ok(state.dashboard())
        }
        "leanctx" => {
            state
                .tool_manager
                .set_leanctx_enabled(enabled)
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "response-cache" | "semantic-cache" => {
            if enabled
                && matches!(
                    client_adapters::load_switchboard_mode(),
                    Some(crate::models::SwitchboardMode::Off | crate::models::SwitchboardMode::Rtk)
                )
            {
                return Err(
                        "Exact Response Cache requires Full or Headroom mode; Off and RTK-only modes do not serve cached provider responses."
                        .into(),
                );
            }
            state
                .semantic_cache
                .set_enabled(enabled)
                .map_err(|err| err.to_string())?;
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
        "leanctx" => {
            state
                .tool_manager
                .uninstall_leanctx_sidecar()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "response-cache" | "semantic-cache" => {
            state
                .semantic_cache
                .set_enabled(false)
                .map_err(|err| err.to_string())?;
            state
                .semantic_cache
                .clear()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        other => Err(format!("unknown addon: {other}")),
    }
}

#[tauri::command]
pub fn get_leanctx_sidecar_status(
    state: State<'_, AppState>,
) -> crate::tool_manager::LeanctxSidecarStatus {
    state.tool_manager.leanctx_sidecar_status()
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
