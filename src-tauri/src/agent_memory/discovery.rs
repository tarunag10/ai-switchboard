use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::dedup::{duplicate_tokens, estimate_tokens};
use super::rollback::is_eligible_managed_memory;
use super::secret_scan::{scan, SecretScanResult, SecretScanStatus};
use crate::repo_intelligence;

const MAX_DISCOVERED_FILES: usize = 128;
const MAX_DEPTH: usize = 10;
const MAX_MEMORY_FILE_BYTES: u64 = 256 * 1024;
const IGNORED_DIRS: [&str; 11] = [
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "coverage",
    ".next",
    ".venv",
    "vendor",
    "secrets",
    ".secrets",
];

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMemoryTarget {
    Codex,
    Claude,
    Shared,
    RepoMemoryMcp,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemorySnapshot {
    pub schema_version: u8,
    pub generated_at: DateTime<Utc>,
    pub repo_path: Option<String>,
    pub sources: Vec<AgentMemorySource>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemorySource {
    pub id: String,
    pub agent: AgentMemoryTarget,
    pub source_path: String,
    pub scope: String,
    pub status: String,
    pub managed_by_switchboard: bool,
    pub estimated_tokens: u64,
    pub duplicate_tokens: u64,
    pub cacheable_tokens: u64,
    pub freshness: String,
    pub secret_scan: SecretScanResult,
    pub recommended_action: String,
    pub preview_available: bool,
    pub rollback_available: bool,
    pub modified_at: Option<DateTime<Utc>>,
    /// Structural information only: user-authored instruction text is never
    /// included in a snapshot response.
    pub bounded_preview: AgentMemoryBoundedPreview,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemoryBoundedPreview {
    pub line_count: usize,
    pub character_count: usize,
    pub heading_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct DiscoveredMemory {
    pub source: AgentMemorySource,
    pub content: Option<String>,
}

pub fn get_snapshot(repo_path: Option<String>) -> Result<AgentMemorySnapshot, String> {
    let repo = repo_path.as_deref().map(validate_repo_path).transpose()?;
    let discovered = discover(repo.as_deref())?;
    Ok(snapshot_from_discovered(repo, discovered))
}

pub(crate) fn discover(repo: Option<&Path>) -> Result<Vec<DiscoveredMemory>, String> {
    let mut items = Vec::new();
    if let Some(repo) = repo {
        discover_repo_files(repo, repo, 0, &mut items)?;
    }
    discover_home_files(&mut items);
    discover_repo_intelligence(repo, &mut items);
    apply_duplicate_metrics(&mut items);
    Ok(items)
}

pub(crate) fn snapshot_from_discovered(
    repo: Option<PathBuf>,
    items: Vec<DiscoveredMemory>,
) -> AgentMemorySnapshot {
    let warnings = items
        .iter()
        .filter(|item| matches!(item.source.secret_scan.status, SecretScanStatus::Blocked))
        .map(|item| {
            format!(
                "{} is blocked because its memory content may contain a secret.",
                item.source.source_path
            )
        })
        .collect();
    AgentMemorySnapshot {
        schema_version: 1,
        generated_at: Utc::now(),
        repo_path: repo.map(|path| path.display().to_string()),
        sources: items.into_iter().map(|item| item.source).collect(),
        warnings,
    }
}

fn validate_repo_path(value: &str) -> Result<PathBuf, String> {
    let path = fs::canonicalize(value)
        .map_err(|error| format!("Could not resolve repository path: {error}"))?;
    if !path.is_dir() {
        return Err("Repository path must be a directory".to_string());
    }
    Ok(path)
}

fn discover_repo_files(
    root: &Path,
    directory: &Path,
    depth: usize,
    items: &mut Vec<DiscoveredMemory>,
) -> Result<(), String> {
    if depth > MAX_DEPTH || items.len() >= MAX_DISCOVERED_FILES {
        return Ok(());
    }
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Could not read {}: {error}", directory.display()))?;
    for entry in entries.flatten() {
        if items.len() >= MAX_DISCOVERED_FILES {
            break;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !IGNORED_DIRS.iter().any(|ignored| *ignored == name) {
                discover_repo_files(root, &path, depth + 1, items)?;
            }
            continue;
        }
        let agent = match name.as_str() {
            "AGENTS.md" => Some(AgentMemoryTarget::Codex),
            "CLAUDE.md" => Some(AgentMemoryTarget::Claude),
            _ => None,
        };
        if let Some(agent) = agent {
            items.push(read_source(&path, agent, repo_scope(root, &path), false));
        }
    }
    Ok(())
}

fn discover_home_files(items: &mut Vec<DiscoveredMemory>) {
    let Some(home) = dirs::home_dir() else { return };
    // Known global Codex/Claude instruction paths. Only existing regular files
    // are read; we never enumerate unrelated home-directory content.
    for (relative, agent) in [
        (".codex/AGENTS.md", AgentMemoryTarget::Codex),
        (".codex/instructions.md", AgentMemoryTarget::Codex),
        (".claude/CLAUDE.md", AgentMemoryTarget::Claude),
    ] {
        let path = home.join(relative);
        if path.is_file() {
            items.push(read_source(&path, agent, "global".to_string(), false));
        }
    }
}

fn discover_repo_intelligence(repo: Option<&Path>, items: &mut Vec<DiscoveredMemory>) {
    let Ok(Some(summary)) = repo_intelligence::load_latest_summary() else {
        return;
    };
    let matches_repo = repo
        .map(|path| path == Path::new(&summary.repo_root))
        .unwrap_or(false);
    if !matches_repo {
        return;
    }
    let tokens = summary.packs.iter().map(|pack| pack.estimated_tokens).sum();
    items.push(synthetic_source(
        "repo-intelligence-packs",
        AgentMemoryTarget::Shared,
        "Repo Intelligence context packs",
        "session",
        tokens,
        true,
    ));
    items.push(synthetic_source(
        "repo-memory-mcp",
        AgentMemoryTarget::RepoMemoryMcp,
        "Repo Memory MCP",
        "repo",
        0,
        true,
    ));
}

fn synthetic_source(
    id: &str,
    agent: AgentMemoryTarget,
    path: &str,
    scope: &str,
    tokens: u64,
    managed: bool,
) -> DiscoveredMemory {
    DiscoveredMemory {
        source: AgentMemorySource {
            id: id.to_string(),
            agent,
            source_path: path.to_string(),
            scope: scope.to_string(),
            status: "app-managed".to_string(),
            managed_by_switchboard: managed,
            estimated_tokens: tokens,
            duplicate_tokens: 0,
            cacheable_tokens: tokens,
            freshness: "unknown".to_string(),
            secret_scan: SecretScanResult {
                status: SecretScanStatus::Clear,
                reason: None,
                finding_count: 0,
                categories: vec![],
                affected_line_numbers: vec![],
            },
            recommended_action: "inspect".to_string(),
            preview_available: false,
            rollback_available: false,
            modified_at: None,
            bounded_preview: AgentMemoryBoundedPreview {
                line_count: 0,
                character_count: 0,
                heading_count: 0,
                truncated: false,
            },
        },
        content: None,
    }
}

fn read_source(
    path: &Path,
    agent: AgentMemoryTarget,
    scope: String,
    managed: bool,
) -> DiscoveredMemory {
    let metadata = fs::metadata(path).ok();
    let too_large = metadata
        .as_ref()
        .map(|metadata| metadata.len() > MAX_MEMORY_FILE_BYTES)
        .unwrap_or(false);
    let content = if too_large {
        None
    } else {
        fs::read_to_string(path).ok()
    };
    // A source is app-managed only when it carries a complete canonical
    // Switchboard memory boundary. A filename alone never grants write access.
    let managed = managed
        || content
            .as_deref()
            .map(is_eligible_managed_memory)
            .unwrap_or(false);
    let secret_scan = content
        .as_deref()
        .map(scan)
        .unwrap_or_else(|| SecretScanResult {
            status: SecretScanStatus::Unreadable,
            reason: Some("The file could not be read safely.".to_string()),
            finding_count: 0,
            categories: vec![],
            affected_line_numbers: vec![],
        });
    let modified_at = metadata
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| {
            DateTime::<Utc>::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
        });
    let status = if too_large {
        "blocked"
    } else if matches!(secret_scan.status, SecretScanStatus::Blocked) {
        "blocked"
    } else if content.is_some() {
        if managed {
            "app-managed"
        } else if freshness(modified_at.as_ref()) == "stale" {
            "stale"
        } else {
            "user-managed"
        }
    } else {
        "unreadable"
    };
    let rollback_available =
        managed && matches!(secret_scan.status, SecretScanStatus::Clear) && content.is_some();
    let text = content.as_deref().unwrap_or_default();
    let line_count = text.lines().count();
    let heading_count = text
        .lines()
        .filter(|line| line.trim_start().starts_with('#'))
        .count();
    let id = format!("{}:{}", agent_key(agent), path.display());
    DiscoveredMemory {
        source: AgentMemorySource {
            id,
            agent,
            source_path: path.display().to_string(),
            scope,
            status: status.to_string(),
            managed_by_switchboard: managed,
            estimated_tokens: if content.is_some() {
                estimate_tokens(text)
            } else {
                0
            },
            duplicate_tokens: 0,
            cacheable_tokens: if matches!(secret_scan.status, SecretScanStatus::Clear) {
                estimate_tokens(text)
            } else {
                0
            },
            freshness: freshness(modified_at.as_ref()),
            secret_scan,
            recommended_action: if too_large {
                "reduce_file_size".to_string()
            } else if content.is_some() {
                "preview_compaction".to_string()
            } else {
                "inspect_permissions".to_string()
            },
            preview_available: content.is_some() && !too_large,
            rollback_available,
            modified_at,
            bounded_preview: AgentMemoryBoundedPreview {
                line_count,
                character_count: text.len(),
                heading_count,
                truncated: too_large,
            },
        },
        content,
    }
}

fn apply_duplicate_metrics(items: &mut [DiscoveredMemory]) {
    let indexes: Vec<_> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            item.content
                .as_ref()
                .map(|content| (index, content.clone()))
        })
        .collect();
    let tokens = duplicate_tokens(
        &indexes
            .iter()
            .map(|(_, content)| content.clone())
            .collect::<Vec<_>>(),
    );
    for ((index, _), duplicate) in indexes.into_iter().zip(tokens) {
        items[index].source.duplicate_tokens = duplicate;
    }
}

fn repo_scope(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.parent())
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|_| "nested".to_string())
        .unwrap_or_else(|| "repo".to_string())
}
fn agent_key(agent: AgentMemoryTarget) -> &'static str {
    match agent {
        AgentMemoryTarget::Codex => "codex",
        AgentMemoryTarget::Claude => "claude",
        AgentMemoryTarget::Shared => "shared",
        AgentMemoryTarget::RepoMemoryMcp => "repo_memory_mcp",
    }
}
fn freshness(modified_at: Option<&DateTime<Utc>>) -> String {
    match modified_at {
        None => "unknown".to_string(),
        Some(value) if Utc::now().signed_duration_since(*value).num_days() > 90 => {
            "stale".to_string()
        }
        Some(_) => "recent".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn discovers_nested_agents_and_claude_memory() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("AGENTS.md"), "# Root\nAlways test.\n").unwrap();
        fs::create_dir_all(temp.path().join("apps/web/.claude")).unwrap();
        fs::write(
            temp.path().join("apps/web/.claude/CLAUDE.md"),
            "# Claude\nBe careful.\n",
        )
        .unwrap();
        let items = discover(Some(temp.path())).unwrap();
        let repo_items: Vec<_> = items
            .iter()
            .filter(|item| {
                item.source
                    .source_path
                    .starts_with(temp.path().to_string_lossy().as_ref())
            })
            .collect();
        assert_eq!(repo_items.len(), 2);
        assert!(items.iter().any(|item| item.source.scope == "nested"));
    }

    #[test]
    fn excludes_secret_directories_from_walk() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("secrets")).unwrap();
        fs::write(temp.path().join("secrets/AGENTS.md"), "not discoverable").unwrap();
        assert!(discover(Some(temp.path())).unwrap().iter().all(|item| !item
            .source
            .source_path
            .starts_with(temp.path().to_string_lossy().as_ref())));
    }
}
