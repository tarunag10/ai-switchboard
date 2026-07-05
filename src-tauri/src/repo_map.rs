use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::external_open;

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
    repo_path: Option<String>,
) -> Result<RepoMapGenerationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
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

        let output = Command::new("node")
            .arg(&script_path)
            .arg("--repo")
            .arg(&target_repo)
            .arg("--out")
            .arg("docs/repo-map")
            .arg("--run-tools")
            .current_dir(&target_repo)
            .output()
            .map_err(|err| format!("Failed to start repo map generator: {err}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
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
