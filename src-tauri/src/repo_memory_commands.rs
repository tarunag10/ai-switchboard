use tauri::State;

use crate::models::DashboardState;
use crate::state::AppState;

#[tauri::command]
pub fn install_repo_memory_mcp(state: State<'_, AppState>) -> Result<DashboardState, String> {
    state
        .tool_manager
        .install_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    Ok(state.dashboard())
}

#[tauri::command]
pub fn start_repo_memory_mcp(state: State<'_, AppState>) -> Result<DashboardState, String> {
    state
        .start_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    Ok(state.dashboard())
}

#[tauri::command]
pub fn stop_repo_memory_mcp(state: State<'_, AppState>) -> Result<DashboardState, String> {
    state
        .stop_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    Ok(state.dashboard())
}
