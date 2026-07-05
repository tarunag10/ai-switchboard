use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri::State;

use crate::models::{
    AppliedPatterns, AppliedSection, HeadroomLearnPrereqStatus, HeadroomLearnStatus, LiveLearning,
};
use crate::state::AppState;

#[tauri::command]
pub fn get_headroom_learn_status(
    state: State<'_, AppState>,
    project_path: Option<String>,
) -> HeadroomLearnStatus {
    state.headroom_learn_status(project_path.as_deref())
}

#[tauri::command]
pub fn get_headroom_learn_prereq_status(
    state: State<'_, AppState>,
    force: Option<bool>,
) -> HeadroomLearnPrereqStatus {
    if force.unwrap_or(false) {
        state.invalidate_headroom_learn_prereq_cache();
    }
    state.headroom_learn_prereq_status()
}

#[tauri::command]
pub async fn list_live_learnings(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<Vec<LiveLearning>, String> {
    let memory_path = crate::headroom_memory_db_path();
    if !memory_path.exists() {
        return Ok(Vec::new());
    }
    let stdout = memory_export_cached(&state, &memory_path)?;
    parse_live_learnings(&stdout, &project_path)
}

#[tauri::command]
pub async fn list_live_learnings_for_projects(
    state: State<'_, AppState>,
    project_paths: Vec<String>,
) -> Result<HashMap<String, Vec<LiveLearning>>, String> {
    let memory_path = crate::headroom_memory_db_path();
    if !memory_path.exists() {
        return Ok(empty_live_learnings_for_projects(&project_paths));
    }
    let stdout = memory_export_cached(&state, &memory_path)?;
    aggregate_live_learnings(&stdout, &project_paths)
}

pub(crate) fn empty_live_learnings_for_projects(
    project_paths: &[String],
) -> HashMap<String, Vec<LiveLearning>> {
    let mut out = HashMap::with_capacity(project_paths.len());
    for p in project_paths {
        out.insert(p.clone(), Vec::new());
    }
    out
}

pub(crate) fn aggregate_live_learnings(
    stdout: &str,
    project_paths: &[String],
) -> Result<HashMap<String, Vec<LiveLearning>>, String> {
    let mut out = HashMap::with_capacity(project_paths.len());
    for p in project_paths {
        let learnings = parse_live_learnings(stdout, p)?;
        out.insert(p.clone(), learnings);
    }
    Ok(out)
}

pub(crate) fn memory_export_cached(
    state: &State<'_, AppState>,
    memory_path: &Path,
) -> Result<String, String> {
    if let Some(cached) = state.cached_memory_export() {
        return Ok(cached);
    }
    let entrypoint = state.tool_manager.headroom_entrypoint();
    let stdout = run_memory_export(&entrypoint, memory_path)?;
    state.store_memory_export(stdout.clone());
    Ok(stdout)
}

#[tauri::command]
pub async fn delete_live_learning(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<(), String> {
    let memory_path = crate::headroom_memory_db_path();
    if !memory_path.exists() {
        return Err("Memory database does not exist.".into());
    }
    let entrypoint = state.tool_manager.headroom_entrypoint();
    let output = Command::new(&entrypoint)
        .arg("memory")
        .arg("delete")
        .arg(&memory_id)
        .arg("--force")
        .arg("--db-path")
        .arg(&memory_path)
        .env("PYTHONNOUSERSITE", "1")
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "headroom memory delete failed ({}): {}",
            output.status,
            stderr.trim()
        ));
    }
    state.invalidate_memory_export_cache();
    Ok(())
}

#[tauri::command]
pub async fn list_applied_patterns(project_path: String) -> Result<AppliedPatterns, String> {
    Ok(read_applied_patterns_for_project(&project_path))
}

#[tauri::command]
pub async fn list_applied_patterns_for_projects(
    project_paths: Vec<String>,
) -> Result<HashMap<String, AppliedPatterns>, String> {
    let mut out = HashMap::with_capacity(project_paths.len());
    for p in project_paths {
        let patterns = read_applied_patterns_for_project(&p);
        out.insert(p, patterns);
    }
    Ok(out)
}

pub(crate) fn read_applied_patterns_for_project(project_path: &str) -> AppliedPatterns {
    let claude_md = PathBuf::from(project_path).join("CLAUDE.md");
    let memory_md = crate::tool_manager::claude_project_memory_file(project_path);

    AppliedPatterns {
        claude_md: read_applied_block(&claude_md),
        memory_md: read_applied_block(&memory_md),
    }
}

#[tauri::command]
pub async fn delete_applied_pattern(
    project_path: String,
    file_kind: String,
    section_title: String,
    bullet_text: String,
) -> Result<(), String> {
    let path = match file_kind.as_str() {
        "claude" => PathBuf::from(&project_path).join("CLAUDE.md"),
        "memory" => crate::tool_manager::claude_project_memory_file(&project_path),
        other => return Err(format!("Unknown file_kind: {other}")),
    };
    if !path.exists() {
        return Err(format!("{} does not exist.", path.display()));
    }
    let content =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let updated =
        crate::tool_manager::delete_applied_bullet(&content, &section_title, &bullet_text);
    if updated == content {
        return Ok(());
    }
    std::fs::write(&path, updated).map_err(|err| format!("write {}: {err}", path.display()))?;
    Ok(())
}

fn read_applied_block(path: &Path) -> Vec<AppliedSection> {
    match std::fs::read_to_string(path) {
        Ok(content) => crate::tool_manager::parse_headroom_learn_block(&content),
        Err(_) => Vec::new(),
    }
}

/// Shells `headroom memory export --db-path <db>` and returns raw JSON stdout.
fn run_memory_export(entrypoint: &Path, db_path: &Path) -> Result<String, String> {
    let output = Command::new(entrypoint)
        .arg("memory")
        .arg("export")
        .arg("--db-path")
        .arg(db_path)
        .env("PYTHONNOUSERSITE", "1")
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!("headroom memory export exited {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn parse_live_learnings(
    json: &str,
    project_path: &str,
) -> Result<Vec<LiveLearning>, String> {
    #[derive(serde::Deserialize)]
    struct Raw {
        id: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        created_at: Option<String>,
        #[serde(default)]
        importance: Option<f64>,
        #[serde(default)]
        metadata: serde_json::Value,
        #[serde(default)]
        entity_refs: Vec<String>,
    }

    let raws: Vec<Raw> = serde_json::from_str(json.trim()).map_err(|err| err.to_string())?;
    let mut out: Vec<LiveLearning> = Vec::new();
    for r in raws {
        let source = r
            .metadata
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if source != "traffic_learner" {
            continue;
        }
        if !pattern_matches_project(&r.content, &r.entity_refs, project_path) {
            continue;
        }
        let category = r
            .metadata
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let evidence_count = r
            .metadata
            .get("evidence_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        out.push(LiveLearning {
            id: r.id,
            content: r.content,
            category,
            importance: r.importance.unwrap_or(0.5),
            evidence_count,
            created_at: r.created_at.unwrap_or_default(),
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// True if any absolute path in `content` or `entity_refs` is under `project_path`.
pub(crate) fn pattern_matches_project(
    content: &str,
    entity_refs: &[String],
    project_path: &str,
) -> bool {
    let root = project_path.trim_end_matches('/');
    if root.is_empty() {
        return false;
    }
    let needle_slash = format!("{root}/");
    if content.contains(root) {
        // Guard against /x/ab matching /x/a: require exact or followed by /.
        if content.contains(&needle_slash)
            || content.contains(&format!("{root}\""))
            || content.contains(&format!("{root}`"))
        {
            return true;
        }
    }
    for r in entity_refs {
        if r == root || r.starts_with(&needle_slash) {
            return true;
        }
    }
    false
}
