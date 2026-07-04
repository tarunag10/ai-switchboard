use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

use crate::client_adapters;
use crate::client_footprint;
use crate::dedicated_cleanup_rollback;
use crate::models::{
    ManagedConfigApplyPreview, ManagedConfigApplyResult, ManagedFootprintReport,
    ManagedRollbackExecutionResult, ManagedRollbackPreview, ManagedRollbackUndoAllExecutionResult,
    ManagedRollbackUndoAllPreview, UninstallDryRunReport,
};
use crate::state::AppState;

#[tauri::command]
pub fn preview_managed_rollback(record_id: String) -> Result<ManagedRollbackPreview, String> {
    client_adapters::preview_managed_rollback(&record_id).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn execute_managed_rollback(
    record_id: String,
    backup_path: String,
    confirmation_phrase: String,
) -> Result<ManagedRollbackExecutionResult, String> {
    client_adapters::execute_managed_rollback(&record_id, &backup_path, &confirmation_phrase)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn preview_dedicated_cleanup_rollback(
    app: AppHandle,
    record_id: String,
) -> Result<ManagedRollbackPreview, String> {
    let state: tauri::State<'_, AppState> = app.state();
    dedicated_cleanup_rollback::preview_dedicated_cleanup_rollback_inner(Some(&state), record_id)
}

#[tauri::command]
pub fn execute_dedicated_cleanup_rollback(
    app: AppHandle,
    record_id: String,
    confirmation_phrase: String,
) -> Result<ManagedRollbackExecutionResult, String> {
    if dedicated_cleanup_rollback::is_login_item_record(&record_id) {
        let _ = app.autolaunch().disable();
    }
    let state: tauri::State<'_, AppState> = app.state();
    dedicated_cleanup_rollback::execute_dedicated_cleanup_rollback_inner(
        Some(&state),
        record_id,
        confirmation_phrase,
    )
}

#[tauri::command]
pub fn preview_managed_config_apply(
    record_id: String,
) -> Result<ManagedConfigApplyPreview, String> {
    client_adapters::preview_managed_config_apply(&record_id).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn execute_managed_config_apply(
    record_id: String,
    confirmation_phrase: String,
) -> Result<ManagedConfigApplyResult, String> {
    client_adapters::execute_managed_config_apply(&record_id, &confirmation_phrase)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn preview_managed_rollback_undo_all() -> ManagedRollbackUndoAllPreview {
    client_adapters::preview_managed_rollback_undo_all()
}

#[tauri::command]
pub fn execute_managed_rollback_undo_all(
    confirmation_phrase: String,
) -> Result<ManagedRollbackUndoAllExecutionResult, String> {
    client_adapters::execute_managed_rollback_undo_all(&confirmation_phrase)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_managed_footprint() -> ManagedFootprintReport {
    client_footprint::get_managed_footprint()
}

#[tauri::command]
pub fn get_uninstall_dry_run_report() -> UninstallDryRunReport {
    client_footprint::uninstall_dry_run_report()
}
