use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::{io::BufRead, io::BufReader, thread};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Emitter;

use crate::external_open;

static ACTIVE_REPO_MAP_PID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();
static REPO_MAP_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

fn active_repo_map_pid() -> &'static Mutex<Option<u32>> {
    ACTIVE_REPO_MAP_PID.get_or_init(|| Mutex::new(None))
}

fn tail(value: &str, max_chars: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max_chars {
        value.to_string()
    } else {
        chars[chars.len().saturating_sub(max_chars)..]
            .iter()
            .collect()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepoMapGenerationResponse {
    repo_path: String,
    out_dir: String,
    readme_path: String,
    compact_context_path: String,
    map: Value,
    compact_context: String,
    tool_log: Value,
    stdout_tail: String,
    stderr_tail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoMapGenerationEvent {
    repo_path: String,
    phase: &'static str,
    stream: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    progress_percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_tools: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_tools: Option<u8>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoMapPreflightTool {
    id: String,
    label: String,
    available: bool,
    detail: String,
    install_hint: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepoMapPreflightResponse {
    repo_path: String,
    exists: bool,
    is_directory: bool,
    has_package_json: bool,
    has_cargo_manifest: bool,
    tools: Vec<RepoMapPreflightTool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepoMapArtifactRequest {
    repo_path: Option<String>,
    artifact: String,
}

fn repo_map_default_repo() -> Result<PathBuf, String> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Could not resolve app repository root.".to_string())
}

fn repo_map_target_repo(repo_path: Option<String>) -> Result<PathBuf, String> {
    let target_repo = repo_path
        .map(PathBuf::from)
        .unwrap_or(repo_map_default_repo()?);
    if !target_repo.exists() {
        return Err(format!(
            "Repository path does not exist: {}.",
            target_repo.display()
        ));
    }
    if !target_repo.is_dir() {
        return Err(format!(
            "Repository path is not a directory: {}.",
            target_repo.display()
        ));
    }
    Ok(target_repo)
}

fn probe_repo_map_tool(
    id: &str,
    label: &str,
    command: &str,
    install_hint: Option<&str>,
) -> RepoMapPreflightTool {
    let output = Command::new("/bin/zsh").args(["-lc", command]).output();
    match output {
        Ok(output) if output.status.success() => RepoMapPreflightTool {
            id: id.to_string(),
            label: label.to_string(),
            available: true,
            detail: tail(&String::from_utf8_lossy(&output.stdout), 240),
            install_hint: None,
        },
        Ok(output) => RepoMapPreflightTool {
            id: id.to_string(),
            label: label.to_string(),
            available: false,
            detail: tail(&String::from_utf8_lossy(&output.stderr), 360),
            install_hint: install_hint.map(str::to_string),
        },
        Err(err) => RepoMapPreflightTool {
            id: id.to_string(),
            label: label.to_string(),
            available: false,
            detail: err.to_string(),
            install_hint: install_hint.map(str::to_string),
        },
    }
}

fn emit_repo_map_generation_event(
    app: &tauri::AppHandle,
    repo_path: &Path,
    phase: &'static str,
    stream: &'static str,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "repo_map_generation_event",
        RepoMapGenerationEvent {
            repo_path: repo_path.display().to_string(),
            phase,
            stream,
            message: message.into(),
            tool_id: None,
            tool_status: None,
            progress_percent: None,
            completed_tools: None,
            total_tools: None,
        },
    );
}

fn emit_repo_map_tool_progress_event(
    app: &tauri::AppHandle,
    repo_path: &Path,
    tool_id: String,
    tool_status: String,
    progress_percent: u8,
    completed_tools: u8,
    total_tools: u8,
    message: String,
) {
    let _ = app.emit(
        "repo_map_generation_event",
        RepoMapGenerationEvent {
            repo_path: repo_path.display().to_string(),
            phase: "running",
            stream: "status",
            message,
            tool_id: Some(tool_id),
            tool_status: Some(tool_status),
            progress_percent: Some(progress_percent.min(100)),
            completed_tools: Some(completed_tools),
            total_tools: Some(total_tools.max(1)),
        },
    );
}

fn parse_repo_map_tool_progress(line: &str) -> Option<(String, String, u8, u8, u8, String)> {
    let payload = serde_json::from_str::<Value>(line).ok()?;
    if payload.get("kind")?.as_str()? != "repo_map_tool_event" {
        return None;
    }
    let tool_id = payload.get("toolId")?.as_str()?.to_string();
    let status = payload.get("status")?.as_str()?.to_string();
    let percent = payload
        .get("progressPercent")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(100) as u8;
    let completed = payload
        .get("completedTools")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(u8::MAX as u64) as u8;
    let total = payload
        .get("totalTools")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .min(u8::MAX as u64) as u8;
    let message = payload
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Tool finished.")
        .to_string();
    Some((tool_id, status, percent, completed, total, message))
}

#[tauri::command]
pub async fn preflight_repo_map(
    repo_path: Option<String>,
) -> Result<RepoMapPreflightResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let target_repo = repo_path
            .map(PathBuf::from)
            .unwrap_or(repo_map_default_repo()?);
        let exists = target_repo.exists();
        let is_directory = target_repo.is_dir();
        let has_package_json = target_repo.join("package.json").exists();
        let has_cargo_manifest = target_repo.join("src-tauri/Cargo.toml").exists()
            || target_repo.join("Cargo.toml").exists();
        Ok(RepoMapPreflightResponse {
            repo_path: target_repo.display().to_string(),
            exists,
            is_directory,
            has_package_json,
            has_cargo_manifest,
            tools: vec![
                probe_repo_map_tool(
                    "node",
                    "Node.js",
                    "node --version",
                    Some("Install Node.js 22+."),
                ),
                probe_repo_map_tool(
                    "npx",
                    "npx",
                    "npx --version",
                    Some("Install Node.js/npm so npx is available."),
                ),
                probe_repo_map_tool(
                    "uv",
                    "uv",
                    "uv --version",
                    Some("Install uv: brew install uv"),
                ),
                probe_repo_map_tool(
                    "graphify",
                    "Graphify",
                    "uvx --from 'graphifyy[openai]' graphify --help >/dev/null && echo graphify-ready",
                    Some("Install on demand with uvx --from 'graphifyy[openai]' graphify --help"),
                ),
                probe_repo_map_tool(
                    "cargo",
                    "Cargo",
                    "cargo --version",
                    Some("Install Rust via rustup."),
                ),
                probe_repo_map_tool(
                    "graphviz",
                    "Graphviz",
                    "which gvpr",
                    Some("Optional SVG export: brew install graphviz"),
                ),
            ],
        })
    })
    .await
    .map_err(|err| format!("Repo map preflight failed: {err}"))?
}

#[tauri::command]
pub async fn generate_repo_map(
    app: tauri::AppHandle,
    repo_path: Option<String>,
) -> Result<RepoMapGenerationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        REPO_MAP_CANCEL_REQUESTED.store(false, Ordering::SeqCst);
        let app_repo = repo_map_default_repo()?;
        let target_repo = repo_map_target_repo(repo_path)?;
        let script_path = app_repo.join("scripts/generate-repo-map.mjs");
        if !script_path.exists() {
            return Err(format!(
                "Repo map generator missing at {}.",
                script_path.display()
            ));
        }
        if !target_repo.exists() {
            return Err(format!(
                "Repository path does not exist: {}.",
                target_repo.display()
            ));
        }

        emit_repo_map_generation_event(
            &app,
            &target_repo,
            "started",
            "status",
            "Starting Repo Map generator.",
        );

        {
            let active_pid = active_repo_map_pid()
                .lock()
                .map_err(|_| "Repo Map generation state is unavailable.")?;
            if active_pid.is_some() {
                return Err(
                    "Repo Map generation is already running; cancel it or wait before retrying."
                        .to_string(),
                );
            }
        }

        let mut child = Command::new("node")
            .arg(&script_path)
            .arg("--repo")
            .arg(&target_repo)
            .arg("--out")
            .arg("docs/repo-map")
            .arg("--run-tools")
            .current_dir(&target_repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("Failed to start repo map generator: {err}"))?;
        if let Ok(mut active_pid) = active_repo_map_pid().lock() {
            *active_pid = Some(child.id());
        }

        let stdout_buffer = Arc::new(Mutex::new(String::new()));
        let stderr_buffer = Arc::new(Mutex::new(String::new()));
        let mut readers = Vec::new();

        if let Some(stdout) = child.stdout.take() {
            let app = app.clone();
            let repo = target_repo.clone();
            let buffer = Arc::clone(&stdout_buffer);
            readers.push(thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(|line| line.ok()) {
                    if let Ok(mut output) = buffer.lock() {
                        output.push_str(&line);
                        output.push('\n');
                    }
                    if let Some((tool_id, status, percent, completed, total, message)) =
                        parse_repo_map_tool_progress(&line)
                    {
                        emit_repo_map_tool_progress_event(
                            &app, &repo, tool_id, status, percent, completed, total, message,
                        );
                    }
                    emit_repo_map_generation_event(&app, &repo, "running", "stdout", line);
                }
            }));
        }
        if let Some(stderr) = child.stderr.take() {
            let app = app.clone();
            let repo = target_repo.clone();
            let buffer = Arc::clone(&stderr_buffer);
            readers.push(thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(|line| line.ok()) {
                    if let Ok(mut output) = buffer.lock() {
                        output.push_str(&line);
                        output.push('\n');
                    }
                    emit_repo_map_generation_event(&app, &repo, "running", "stderr", line);
                }
            }));
        }

        let status = child
            .wait()
            .map_err(|err| format!("Repo map generator wait failed: {err}"))?;
        if let Ok(mut active_pid) = active_repo_map_pid().lock() {
            *active_pid = None;
        }
        for reader in readers {
            let _ = reader.join();
        }

        let stdout = stdout_buffer
            .lock()
            .map(|output| output.clone())
            .unwrap_or_default();
        let stderr = stderr_buffer
            .lock()
            .map(|output| output.clone())
            .unwrap_or_default();
        if !status.success() {
            if REPO_MAP_CANCEL_REQUESTED.load(Ordering::SeqCst) {
                emit_repo_map_generation_event(
                    &app,
                    &target_repo,
                    "cancelled",
                    "status",
                    "Repo Map generation cancelled by the user.",
                );
                return Err("Repo map generation cancelled.".to_string());
            }
            emit_repo_map_generation_event(
                &app,
                &target_repo,
                "failed",
                "status",
                format!("Repo Map generator exited with {status}."),
            );
            return Err(format!(
                "Repo map generator exited with {status}. {}",
                tail(&stderr, 1200)
            ));
        }
        let out_dir = target_repo.join("docs/repo-map");
        let readme_path = out_dir.join("README.md");
        let map_path = out_dir.join("repo-map.json");
        let compact_context_path = out_dir.join("COMPACT_CONTEXT.md");
        let tool_log_path = out_dir.join("tool-log.json");

        if !map_path.exists() {
            return Err(format!(
                "Repo map generation did not produce {}. {}",
                map_path.display(),
                tail(&stderr, 1200)
            ));
        }

        let map_text = std::fs::read_to_string(&map_path)
            .map_err(|err| format!("Failed to read {}: {err}", map_path.display()))?;
        let map: Value = serde_json::from_str(&map_text)
            .map_err(|err| format!("Failed to parse {}: {err}", map_path.display()))?;
        let compact_context = std::fs::read_to_string(&compact_context_path).unwrap_or_default();
        let tool_log_text = std::fs::read_to_string(&tool_log_path).unwrap_or_else(|_| "[]".into());
        let tool_log: Value = serde_json::from_str(&tool_log_text).unwrap_or_else(|_| json!([]));

        emit_repo_map_generation_event(
            &app,
            &target_repo,
            "finished",
            "status",
            "Repo Map artifacts are ready.",
        );

        Ok(RepoMapGenerationResponse {
            repo_path: target_repo.display().to_string(),
            out_dir: out_dir.display().to_string(),
            readme_path: readme_path.display().to_string(),
            compact_context_path: compact_context_path.display().to_string(),
            map,
            compact_context,
            tool_log,
            stdout_tail: tail(&stdout, 2000),
            stderr_tail: tail(&stderr, 2000),
        })
    })
    .await
    .map_err(|err| format!("Repo map worker failed: {err}"))?
}

/// Stop the active local generator process. This only targets the child
/// process owned by the current app instance and never touches repository
/// files or unrelated processes. A subsequent Generate action is an explicit
/// retry with a fresh process and fresh tool progress.
#[tauri::command]
pub fn cancel_repo_map_generation() -> Result<bool, String> {
    let pid = active_repo_map_pid()
        .lock()
        .map_err(|_| "Repo Map cancellation state is unavailable.")?
        .to_owned();
    let Some(pid) = pid else {
        return Ok(false);
    };
    REPO_MAP_CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    let status = Command::new("/bin/kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .map_err(|err| format!("Failed to cancel Repo Map generator: {err}"))?;
    if !status.success() {
        return Err(format!(
            "Repo Map generator cancellation exited with {status}."
        ));
    }
    Ok(true)
}

#[tauri::command]
pub fn open_repo_map_artifact(request: RepoMapArtifactRequest) -> Result<bool, String> {
    let target_repo = repo_map_target_repo(request.repo_path)?;
    let relative = match request.artifact.as_str() {
        "folder" => PathBuf::from("docs/repo-map"),
        "readme" => PathBuf::from("docs/repo-map/README.md"),
        "compactContext" => PathBuf::from("docs/repo-map/COMPACT_CONTEXT.md"),
        "graphTree" => PathBuf::from("graphify-out/GRAPH_TREE.html"),
        "graphJson" => PathBuf::from("graphify-out/graph.json"),
        "repoMapJson" => PathBuf::from("docs/repo-map/repo-map.json"),
        other => return Err(format!("Unsupported repo map artifact: {other}.")),
    };
    let path = target_repo.join(relative);
    if !path.exists() {
        return Err(format!("Artifact does not exist: {}.", path.display()));
    }
    external_open::open_local_path(&path)
        .map_err(|err| format!("Failed to open artifact: {err}"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::parse_repo_map_tool_progress;

    #[test]
    fn parses_bounded_content_free_tool_progress() {
        let event = parse_repo_map_tool_progress(
            r#"{"kind":"repo_map_tool_event","toolId":"graphify","status":"warning","progressPercent":140,"completedTools":2,"totalTools":5,"message":"graphify exited 1"}"#,
        )
        .expect("tool progress event");
        assert_eq!(event.0, "graphify");
        assert_eq!(event.1, "warning");
        assert_eq!(event.2, 100);
        assert_eq!(event.3, 2);
        assert_eq!(event.4, 5);
        assert_eq!(event.5, "graphify exited 1");
    }

    #[test]
    fn ignores_unrelated_generator_output() {
        assert!(parse_repo_map_tool_progress("Wrote docs/repo-map/repo-map.json").is_none());
    }
}
