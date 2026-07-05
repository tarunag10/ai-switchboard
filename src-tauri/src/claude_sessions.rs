use crate::models::ClaudeCodeProject;
use crate::tool_manager::ToolManager;
use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn user_home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn claude_projects_dir() -> PathBuf {
    user_home_dir().join(".claude").join("projects")
}

#[derive(Debug, Default)]
pub(crate) struct ClaudeProjectScan {
    pub(crate) last_worked_at: Option<std::time::SystemTime>,
    pub(crate) session_files: Vec<PathBuf>,
    pub(crate) seen_session_files: HashSet<PathBuf>,
}

impl ClaudeProjectScan {
    pub(crate) fn add_session_files(&mut self, session_files: Vec<PathBuf>) {
        for session_file in session_files {
            let dedupe_key = canonical_session_file_path(&session_file);
            if self.seen_session_files.insert(dedupe_key) {
                self.session_files.push(session_file);
            }
        }
    }
}

pub(crate) fn build_claude_code_project(
    tool_manager: &ToolManager,
    project_path: String,
    scan: ClaudeProjectScan,
) -> Option<ClaudeCodeProject> {
    let last_worked_at: chrono::DateTime<Utc> = scan.last_worked_at?.into();
    let session_count = scan.session_files.len();
    let mut hasher = Sha256::new();
    hasher.update(project_path.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    let id = digest[..12].to_string();
    let display_name = Path::new(&project_path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| project_path.clone());

    let learn_summary = tool_manager.headroom_learn_project_summary(&project_path);
    let last_learn_ran_at = learn_summary.last_run_at;
    let has_persisted_learnings = learn_summary.has_persisted_learnings;
    let last_learn_pattern_count = learn_summary.pattern_count;
    let learn_time = last_learn_ran_at
        .as_ref()
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
        .map(|ts| ts.with_timezone(&Utc));
    let today = Utc::now().date_naive();
    let mut days_since_learn: HashSet<chrono::NaiveDate> = HashSet::new();
    let mut sessions_today: usize = 0;
    for file in &scan.session_files {
        let Ok(meta) = std::fs::metadata(file) else {
            continue;
        };
        let Ok(m) = meta.modified() else {
            continue;
        };
        let t: chrono::DateTime<Utc> = m.into();
        if t.date_naive() == today {
            sessions_today += 1;
        }
        if let Some(learn_time) = learn_time {
            if t > learn_time {
                days_since_learn.insert(t.date_naive());
            }
        }
    }
    let active_days_since_last_learn = if learn_time.is_some() {
        days_since_learn.len()
    } else {
        0
    };

    Some(ClaudeCodeProject {
        id,
        project_path,
        display_name,
        last_worked_at: last_worked_at.to_rfc3339(),
        session_count,
        sessions_today,
        last_learn_ran_at,
        has_persisted_learnings,
        active_days_since_last_learn,
        last_learn_pattern_count,
    })
}

pub(crate) fn list_session_jsonl_files(project_dir: &Path) -> Vec<PathBuf> {
    let mut files = std::fs::read_dir(project_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("jsonl"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    files
}

pub(crate) fn canonical_session_file_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn extract_cwd_from_session_file(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;

    for line in reader.lines().map_while(|line| line.ok()).take(300) {
        if !line.contains("\"cwd\"") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(cwd) = value.get("cwd").and_then(|item| item.as_str()) {
            if !cwd.trim().is_empty() {
                return Some(cwd.to_string());
            }
        }
    }

    None
}

pub(crate) fn decode_project_folder_name(folder_name: &str) -> String {
    // Claude Code's folder-name convention is lossy: it maps '/' to '-' without
    // escaping existing hyphens, so paths like `/a/b-c` and `/a/b/c` produce the
    // same folder. We mirror that convention here and accept the ambiguity --
    // the primary resolver (`extract_cwd_from_session_file`) reads the real cwd
    // from session JSONL, so this fallback only runs when that fails.
    if !folder_name.starts_with('-') {
        return folder_name.to_string();
    }
    let rebuilt = format!("/{}", folder_name.trim_start_matches('-').replace('-', "/"));
    if rebuilt.trim().is_empty() {
        folder_name.to_string()
    } else {
        rebuilt
    }
}

pub(crate) fn project_display_name(project_path: &str) -> String {
    Path::new(project_path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| name.to_string())
        .unwrap_or_else(|| project_path.to_string())
}

pub fn tail_lines(text: &str, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().map(|line| line.to_string()).collect();
    if lines.len() > max_lines {
        lines = lines.split_off(lines.len() - max_lines);
    }
    lines
}
