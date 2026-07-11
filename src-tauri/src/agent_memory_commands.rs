use crate::agent_memory::{
    self, AgentMemoryCompactionApplyRequest, AgentMemoryCompactionPreview,
    AgentMemoryCompactionReceipt, AgentMemorySnapshot, AgentMemoryTarget,
};
use crate::models::AgentMemorySessionManifest;
use crate::state::AppState;
use tauri::State;

/// Returns only local source metadata and structural previews; raw instruction
/// text and secrets are never sent through this command.
#[tauri::command]
pub fn get_agent_memory_snapshot(repo_path: Option<String>) -> Result<AgentMemorySnapshot, String> {
    agent_memory::get_snapshot(repo_path)
}

/// Creates a non-mutating structural compaction preview.
#[tauri::command]
pub fn preview_agent_memory_compaction(
    repo_path: String,
    agent: AgentMemoryTarget,
) -> Result<AgentMemoryCompactionPreview, String> {
    agent_memory::preview_compaction(repo_path, agent)
}

/// Prepares copy-safe Agent Memory metadata for a session. Only a complete
/// secret-safe preview with real before/after counts produces a durable,
/// explicitly-estimated savings event.
#[tauri::command]
pub fn prepare_agent_memory_session_handoff(
    state: State<'_, AppState>,
    repo_path: String,
    agent: AgentMemoryTarget,
) -> Result<AgentMemorySessionManifest, String> {
    let manifest = agent_memory::build_session_manifest(repo_path, agent)?;
    state
        .record_agent_memory_attribution(&manifest)
        .map_err(|error| error.to_string())?;
    Ok(manifest)
}

/// Applies only Switchboard-owned managed-memory blocks after the caller sends
/// the exact confirmation phrase returned by the preview contract.
#[tauri::command]
pub fn apply_agent_memory_compaction(
    repo_path: String,
    agent: AgentMemoryTarget,
    confirmation_phrase: String,
) -> Result<Vec<AgentMemoryCompactionReceipt>, String> {
    agent_memory::apply_compaction(AgentMemoryCompactionApplyRequest {
        repo_path,
        agent,
        confirmation_phrase,
    })
}

/// Restores one verified compaction receipt after exact confirmation. Refuses
/// missing/tampered backups and targets that drifted after apply.
#[tauri::command]
pub fn rollback_agent_memory_compaction(
    receipt_id: String,
    confirmation_phrase: String,
) -> Result<AgentMemoryCompactionReceipt, String> {
    agent_memory::rollback_compaction(receipt_id, confirmation_phrase)
}
