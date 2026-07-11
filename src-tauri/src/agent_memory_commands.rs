use crate::agent_memory::{
    self, AgentMemoryCompactionApplyRequest, AgentMemoryCompactionPreview,
    AgentMemoryCompactionReceipt, AgentMemorySnapshot, AgentMemoryTarget,
};

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
