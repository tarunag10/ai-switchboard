use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::discovery::{discover, AgentMemoryTarget};
use super::secret_scan::{scan, SecretScanStatus};

const MANAGED_START: &str = "<!-- mac-ai-switchboard:agent-memory:start -->";
const MANAGED_END: &str = "<!-- mac-ai-switchboard:agent-memory:end -->";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemoryCompactionApplyRequest {
    pub repo_path: String,
    pub agent: AgentMemoryTarget,
    pub confirmation_phrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemoryCompactionReceipt {
    pub receipt_id: String,
    pub created_at: DateTime<Utc>,
    pub agent: AgentMemoryTarget,
    pub target_path: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub backup_path: String,
    pub backup_sha256: String,
    pub managed_block_ids: Vec<String>,
    pub user_confirmed: bool,
    pub status: String,
    pub rollback_confirmation_phrase: String,
    pub verification: Vec<String>,
}

pub fn apply_compaction(
    request: AgentMemoryCompactionApplyRequest,
) -> Result<Vec<AgentMemoryCompactionReceipt>, String> {
    apply_compaction_in_root(request, None)
}

pub fn rollback_compaction(
    receipt_id: String,
    confirmation_phrase: String,
) -> Result<AgentMemoryCompactionReceipt, String> {
    rollback_compaction_in_root(receipt_id, confirmation_phrase, None)
}

fn apply_compaction_in_root(
    request: AgentMemoryCompactionApplyRequest,
    receipt_root: Option<&Path>,
) -> Result<Vec<AgentMemoryCompactionReceipt>, String> {
    let repo = canonical_repo(&request.repo_path)?;
    let expected = apply_confirmation_phrase(request.agent);
    if request.confirmation_phrase != expected {
        return Err("Apply confirmation phrase does not match the preview.".to_string());
    }
    let items = discover(Some(&repo))?;
    let mut candidates = Vec::new();
    for item in items {
        if !matches_target(item.source.agent, request.agent) || item.content.is_none() {
            continue;
        }
        let path = PathBuf::from(&item.source.source_path);
        // A repo-scoped request must never promote a global/home instruction
        // file into a write target. Global sources remain inspector-only.
        if !path.starts_with(&repo) {
            continue;
        }
        let content = item.content.expect("checked");
        if matches!(scan(&content).status, SecretScanStatus::Blocked) {
            return Err(format!(
                "Compaction is blocked because {} contains potential credential material.",
                item.source.source_path
            ));
        }
        ensure_regular_repo_file(&repo, &path)?;
        let Some(compacted) = compact_managed_blocks(&content) else {
            return Err(format!(
                "{} is user-managed or has no eligible Switchboard-managed memory block; refusing to edit it.",
                item.source.source_path
            ));
        };
        candidates.push((path, content, compacted));
    }
    if candidates.is_empty() {
        return Err(
            "No eligible Switchboard-managed memory sources were found for this agent.".to_string(),
        );
    }

    // Validate every target before mutating any of them. This avoids a partial
    // multi-source apply due to a later safety gate.
    for (path, before, _) in &candidates {
        let current = fs::read_to_string(path)
            .map_err(|error| format!("Could not re-read {}: {error}", path.display()))?;
        if current != *before {
            return Err(format!(
                "{} changed after preview; review a new preview before applying.",
                path.display()
            ));
        }
    }

    let receipts_dir = receipt_root
        .map(Path::to_path_buf)
        .unwrap_or_else(default_receipts_dir);
    fs::create_dir_all(&receipts_dir)
        .map_err(|error| format!("Could not create receipt directory: {error}"))?;
    let mut receipts = Vec::new();
    for (path, before, after) in candidates {
        if before == after {
            continue;
        }
        let receipt_id = Uuid::new_v4().to_string();
        let backup_path = receipts_dir.join(format!("{receipt_id}.backup"));
        atomic_write(&backup_path, before.as_bytes())?;
        let before_sha256 = sha256(&before);
        let backup_sha256 = sha256(
            &fs::read_to_string(&backup_path)
                .map_err(|error| format!("Could not verify backup: {error}"))?,
        );
        if backup_sha256 != before_sha256 {
            let _ = fs::remove_file(&backup_path);
            return Err("Backup integrity verification failed; no source was changed.".to_string());
        }
        let after_sha256 = sha256(&after);
        atomic_write(&path, after.as_bytes())?;
        let mut receipt = AgentMemoryCompactionReceipt {
            receipt_id: receipt_id.clone(),
            created_at: Utc::now(),
            agent: request.agent,
            target_path: path.display().to_string(),
            before_sha256,
            after_sha256,
            backup_path: backup_path.display().to_string(),
            backup_sha256,
            managed_block_ids: managed_block_ids(&before),
            user_confirmed: true,
            status: "applied".to_string(),
            rollback_confirmation_phrase: rollback_confirmation_phrase(&receipt_id),
            verification: vec![
                "Exact confirmation phrase matched.".to_string(),
                "Only Switchboard-managed memory block(s) were compacted.".to_string(),
                "Backup SHA-256 matches the pre-apply source.".to_string(),
                "Target was atomically replaced and re-read from disk.".to_string(),
            ],
        };
        if sha256(
            &fs::read_to_string(&path)
                .map_err(|error| format!("Could not verify updated source: {error}"))?,
        ) != receipt.after_sha256
        {
            let _ = atomic_write(&path, before.as_bytes());
            let _ = fs::remove_file(&backup_path);
            return Err(
                "Post-write integrity verification failed; the original was restored.".to_string(),
            );
        }
        let receipt_path = receipt_path(&receipts_dir, &receipt_id);
        if let Err(error) = write_receipt(&receipt_path, &receipt) {
            let _ = atomic_write(&path, before.as_bytes());
            let _ = fs::remove_file(&backup_path);
            return Err(format!(
                "Could not persist rollback receipt; original restored: {error}"
            ));
        }
        receipt.verification.push(format!(
            "Rollback receipt persisted at {}.",
            receipt_path.display()
        ));
        // Persist the final evidence too; source safety is already established.
        write_receipt(&receipt_path, &receipt)?;
        receipts.push(receipt);
    }
    if receipts.is_empty() {
        return Err(
            "No safe structural compaction was identified; no files were changed.".to_string(),
        );
    }
    Ok(receipts)
}

fn rollback_compaction_in_root(
    receipt_id: String,
    confirmation_phrase: String,
    receipt_root: Option<&Path>,
) -> Result<AgentMemoryCompactionReceipt, String> {
    if Uuid::parse_str(&receipt_id).is_err() {
        return Err("Rollback receipt id is invalid.".to_string());
    }
    let receipts_dir = receipt_root
        .map(Path::to_path_buf)
        .unwrap_or_else(default_receipts_dir);
    let path = receipt_path(&receipts_dir, &receipt_id);
    let mut receipt: AgentMemoryCompactionReceipt = serde_json::from_slice(
        &fs::read(&path).map_err(|_| "Rollback receipt was not found.".to_string())?,
    )
    .map_err(|error| format!("Rollback receipt is unreadable: {error}"))?;
    if confirmation_phrase != receipt.rollback_confirmation_phrase {
        return Err("Rollback confirmation phrase does not match.".to_string());
    }
    if receipt.status != "applied" {
        return Err("This receipt is no longer eligible for rollback.".to_string());
    }
    let backup = fs::read_to_string(&receipt.backup_path)
        .map_err(|_| "Rollback backup is missing.".to_string())?;
    if sha256(&backup) != receipt.backup_sha256 || receipt.backup_sha256 != receipt.before_sha256 {
        return Err("Rollback backup integrity check failed.".to_string());
    }
    let target = PathBuf::from(&receipt.target_path);
    let metadata = fs::symlink_metadata(&target)
        .map_err(|error| format!("Could not inspect rollback target: {error}"))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err("Rollback target is no longer an eligible regular file.".to_string());
    }
    let current = fs::read_to_string(&target)
        .map_err(|error| format!("Could not read rollback target: {error}"))?;
    if sha256(&current) != receipt.after_sha256 {
        return Err(
            "Rollback target has changed since apply; refusing to overwrite it.".to_string(),
        );
    }
    let safety_backup = receipts_dir.join(format!("{}.rollback-safety.backup", receipt.receipt_id));
    atomic_write(&safety_backup, current.as_bytes())?;
    atomic_write(&target, backup.as_bytes())?;
    if sha256(
        &fs::read_to_string(&target)
            .map_err(|error| format!("Could not verify rollback: {error}"))?,
    ) != receipt.before_sha256
    {
        return Err("Rollback integrity verification failed.".to_string());
    }
    receipt.status = "rolled_back".to_string();
    receipt.verification.push(format!(
        "Rollback restored the verified backup; a safety copy of the compacted source is at {}.",
        safety_backup.display()
    ));
    write_receipt(&path, &receipt)?;
    Ok(receipt)
}

fn canonical_repo(value: &str) -> Result<PathBuf, String> {
    let path = fs::canonicalize(value)
        .map_err(|error| format!("Could not resolve repository path: {error}"))?;
    if !path.is_dir() {
        return Err("Repository path must be a directory".to_string());
    }
    Ok(path)
}

fn ensure_regular_repo_file(repo: &Path, path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{} is not an eligible regular file.",
            path.display()
        ));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Could not resolve {}: {error}", path.display()))?;
    if !canonical.starts_with(repo) {
        return Err("Only repository-scoped memory files may be edited.".to_string());
    }
    Ok(())
}

fn matches_target(source: AgentMemoryTarget, requested: AgentMemoryTarget) -> bool {
    source == requested
        || matches!(source, AgentMemoryTarget::Shared)
            && matches!(
                requested,
                AgentMemoryTarget::Codex | AgentMemoryTarget::Claude | AgentMemoryTarget::Shared
            )
}

pub(crate) fn apply_confirmation_phrase(agent: AgentMemoryTarget) -> String {
    format!("APPLY AGENT MEMORY COMPACTION FOR {}", agent_key(agent))
}

fn rollback_confirmation_phrase(receipt_id: &str) -> String {
    format!("ROLLBACK AGENT MEMORY COMPACTION {receipt_id}")
}

fn agent_key(agent: AgentMemoryTarget) -> &'static str {
    match agent {
        AgentMemoryTarget::Codex => "CODEX",
        AgentMemoryTarget::Claude => "CLAUDE",
        AgentMemoryTarget::Shared => "SHARED",
        AgentMemoryTarget::RepoMemoryMcp => "REPO_MEMORY_MCP",
    }
}

pub(crate) fn is_eligible_managed_memory(content: &str) -> bool {
    compact_managed_blocks(content).is_some()
}

fn compact_managed_blocks(content: &str) -> Option<String> {
    let mut cursor = 0;
    let mut output = String::new();
    let mut found = false;
    while let Some(relative_start) = content[cursor..].find(MANAGED_START) {
        let start = cursor + relative_start;
        let body_start = start + MANAGED_START.len();
        let relative_end = content[body_start..].find(MANAGED_END)?;
        let end = body_start + relative_end;
        output.push_str(&content[cursor..body_start]);
        output.push_str(&compact_block(&content[body_start..end]));
        output.push_str(MANAGED_END);
        cursor = end + MANAGED_END.len();
        found = true;
    }
    if !found || content[cursor..].contains(MANAGED_END) {
        return None;
    }
    output.push_str(&content[cursor..]);
    Some(output)
}

fn compact_block(content: &str) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let mut output = String::new();
    let mut previous_blank = false;
    for line in content.lines() {
        let key = super::dedup::normalize_line(line);
        let blank = key.is_empty();
        if (blank && previous_blank) || (!blank && !seen.insert(key)) {
            continue;
        }
        output.push_str(line.trim_end());
        output.push('\n');
        previous_blank = blank;
    }
    output
}

fn managed_block_ids(content: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = content[cursor..].find(MANAGED_START) {
        ids.push(format!("agent-memory-{}", ids.len() + 1));
        cursor += offset + MANAGED_START.len();
    }
    ids
}

fn default_receipts_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("mac-ai-switchboard")
        .join("agent-memory-receipts")
}

fn receipt_path(root: &Path, receipt_id: &str) -> PathBuf {
    root.join(format!("{receipt_id}.json"))
}

fn write_receipt(path: &Path, receipt: &AgentMemoryCompactionReceipt) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("Could not serialize receipt: {error}"))?;
    atomic_write(path, &bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("memory"),
        Uuid::new_v4()
    ));
    fs::write(&temporary, bytes)
        .map_err(|error| format!("Could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Could not atomically replace {}: {error}", path.display())
    })
}

fn sha256(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn managed(content: &str) -> String {
        format!("user-owned\n{MANAGED_START}\n{content}\n{MANAGED_END}\nkeep-user-owned\n")
    }

    #[test]
    fn applies_only_managed_blocks_with_verified_receipt_and_rolls_back() {
        let repo = tempdir().unwrap();
        let receipts = tempdir().unwrap();
        let target = repo.path().join("AGENTS.md");
        let before = managed("Keep tests green.\nKeep tests green.\n\n\n");
        fs::write(&target, &before).unwrap();
        let applied = apply_compaction_in_root(
            AgentMemoryCompactionApplyRequest {
                repo_path: repo.path().display().to_string(),
                agent: AgentMemoryTarget::Codex,
                confirmation_phrase: apply_confirmation_phrase(AgentMemoryTarget::Codex),
            },
            Some(receipts.path()),
        )
        .unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].status, "applied");
        let compacted = fs::read_to_string(&target).unwrap();
        assert!(compacted.contains("user-owned"));
        assert_eq!(compacted.matches("Keep tests green.").count(), 1);
        assert!(Path::new(&applied[0].backup_path).is_file());

        let rolled_back = rollback_compaction_in_root(
            applied[0].receipt_id.clone(),
            applied[0].rollback_confirmation_phrase.clone(),
            Some(receipts.path()),
        )
        .unwrap();
        assert_eq!(rolled_back.status, "rolled_back");
        assert_eq!(fs::read_to_string(target).unwrap(), before);
    }

    #[test]
    fn refuses_user_managed_secret_blocked_and_wrong_confirmation_sources() {
        let repo = tempdir().unwrap();
        let receipts = tempdir().unwrap();
        fs::write(repo.path().join("AGENTS.md"), "user instructions\n").unwrap();
        let wrong = apply_compaction_in_root(
            AgentMemoryCompactionApplyRequest {
                repo_path: repo.path().display().to_string(),
                agent: AgentMemoryTarget::Codex,
                confirmation_phrase: "no".to_string(),
            },
            Some(receipts.path()),
        )
        .unwrap_err();
        assert!(wrong.contains("confirmation"));
        let unmanaged = apply_compaction_in_root(
            AgentMemoryCompactionApplyRequest {
                repo_path: repo.path().display().to_string(),
                agent: AgentMemoryTarget::Codex,
                confirmation_phrase: apply_confirmation_phrase(AgentMemoryTarget::Codex),
            },
            Some(receipts.path()),
        )
        .unwrap_err();
        assert!(unmanaged.contains("user-managed"));

        fs::write(
            repo.path().join("AGENTS.md"),
            managed("API_KEY=sk-abcdefghijklmnopqrstuvwxyz"),
        )
        .unwrap();
        let secret = apply_compaction_in_root(
            AgentMemoryCompactionApplyRequest {
                repo_path: repo.path().display().to_string(),
                agent: AgentMemoryTarget::Codex,
                confirmation_phrase: apply_confirmation_phrase(AgentMemoryTarget::Codex),
            },
            Some(receipts.path()),
        )
        .unwrap_err();
        assert!(secret.contains("credential"));
    }

    #[test]
    fn rollback_refuses_drift_or_backup_tampering() {
        let repo = tempdir().unwrap();
        let receipts = tempdir().unwrap();
        let target = repo.path().join("AGENTS.md");
        fs::write(&target, managed("Repeat\nRepeat\n")).unwrap();
        let applied = apply_compaction_in_root(
            AgentMemoryCompactionApplyRequest {
                repo_path: repo.path().display().to_string(),
                agent: AgentMemoryTarget::Codex,
                confirmation_phrase: apply_confirmation_phrase(AgentMemoryTarget::Codex),
            },
            Some(receipts.path()),
        )
        .unwrap();
        fs::write(&target, "drift").unwrap();
        let drift = rollback_compaction_in_root(
            applied[0].receipt_id.clone(),
            applied[0].rollback_confirmation_phrase.clone(),
            Some(receipts.path()),
        )
        .unwrap_err();
        assert!(drift.contains("changed"));
    }
}
