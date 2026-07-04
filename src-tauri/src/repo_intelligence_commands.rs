use tauri::State;

use crate::models::{
    RepoAgentHandoffResponse, RepoContextPackResponse, RepoDependentsResponse,
    RepoIndexFreshnessResponse, RepoIntelligenceManifestResponse, RepoIntelligenceSummary,
    RepoSymbolSearchResponse,
};
use crate::repo_intelligence;
use crate::state::AppState;

#[tauri::command]
pub fn build_repo_intelligence_summary(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<RepoIntelligenceSummary, String> {
    let summary = repo_intelligence::summarize_repo(repo_path).map_err(|err| err.to_string())?;
    repo_intelligence::save_latest_summary(&summary).map_err(|err| err.to_string())?;
    if let Err(err) = state.record_repo_intelligence_attribution(&summary) {
        log::warn!("could not record Repo Intelligence attribution event: {err:#}");
    }
    Ok(summary)
}

#[tauri::command]
pub fn get_latest_repo_intelligence_summary() -> Result<Option<RepoIntelligenceSummary>, String> {
    repo_intelligence::load_latest_summary().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn clear_repo_intelligence_summary() -> Result<bool, String> {
    repo_intelligence::clear_latest_summary().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_repo_intelligence_context_pack(
    pack_id: Option<String>,
) -> Result<Option<RepoContextPackResponse>, String> {
    repo_intelligence::latest_context_pack(pack_id.as_deref()).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn search_repo_intelligence_symbols(
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Option<RepoSymbolSearchResponse>, String> {
    repo_intelligence::latest_symbol_search(query.as_deref(), limit).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_repo_intelligence_dependents(
    target: String,
    limit: Option<usize>,
) -> Result<Option<RepoDependentsResponse>, String> {
    repo_intelligence::latest_dependents_search(&target, limit).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_repo_intelligence_manifest() -> Result<Option<RepoIntelligenceManifestResponse>, String>
{
    repo_intelligence::latest_manifest().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_repo_manifest() -> Result<Option<RepoIntelligenceManifestResponse>, String> {
    repo_intelligence::latest_manifest().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_repo_pack(pack_id: Option<String>) -> Result<Option<RepoContextPackResponse>, String> {
    repo_intelligence::latest_context_pack(pack_id.as_deref()).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_agent_handoff(
    agent_id: String,
    task_type: Option<String>,
) -> Result<Option<RepoAgentHandoffResponse>, String> {
    repo_intelligence::latest_agent_handoff(&agent_id, task_type.as_deref())
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_index_freshness() -> Result<RepoIndexFreshnessResponse, String> {
    repo_intelligence::latest_index_freshness().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn clear_repo_index() -> Result<bool, String> {
    repo_intelligence::clear_latest_summary().map_err(|err| err.to_string())
}
