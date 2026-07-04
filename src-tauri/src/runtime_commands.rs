use tauri::{AppHandle, Manager, State};

use crate::models::{BootstrapProgress, RuntimeStatus, RuntimeUpgradeProgress};
use crate::state::AppState;

#[tauri::command]
pub fn get_bootstrap_progress(state: State<'_, AppState>) -> BootstrapProgress {
    state.bootstrap_progress()
}

#[tauri::command]
pub fn get_runtime_upgrade_progress(state: State<'_, AppState>) -> RuntimeUpgradeProgress {
    state.runtime_upgrade_progress()
}

#[tauri::command]
pub fn retry_runtime_upgrade(app: AppHandle) -> Result<(), String> {
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app_clone.state();
        state.retry_runtime_upgrade(&app_clone, false);
    });
    Ok(())
}

/// User-initiated recovery path. Same flow as `retry_runtime_upgrade` but
/// skips the in-place upgrade attempt and goes straight to atomic rebuild.
/// Surfaced as the "Retry with full rebuild" button on a boot-validation
/// failure: the in-place pip succeeded (smoke test passed) but the proxy
/// never booted, which usually means stale native libs from the previous
/// pin survived the upgrade. The rebuild path nukes the venv and starts
/// fresh, fixing the broken state at the cost of re-downloading wheels.
#[tauri::command]
pub fn retry_runtime_upgrade_with_rebuild(app: AppHandle) -> Result<(), String> {
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app_clone.state();
        state.retry_runtime_upgrade(&app_clone, true);
    });
    Ok(())
}

#[tauri::command]
pub fn dismiss_runtime_upgrade_failure(state: State<'_, AppState>) -> Result<(), String> {
    state.dismiss_upgrade_failure();
    Ok(())
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, AppState>) -> RuntimeStatus {
    state.runtime_status()
}
