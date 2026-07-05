use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use tauri::{AppHandle, Manager, State};

use crate::models::{
    AppliedPatterns, AppliedSection, HeadroomLearnPrereqStatus, HeadroomLearnStatus, LiveLearning,
};
use crate::state::AppState;
use crate::{claude_cli, client_adapters};

/// Which coding agent a Headroom Learn run targets. Determines the session
/// source, the analysis backend, and which context/memory files get written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LearnAgent {
    Claude,
    Codex,
}

impl LearnAgent {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "claude" => Ok(LearnAgent::Claude),
            "codex" => Ok(LearnAgent::Codex),
            other => Err(format!("Unknown Headroom Learn agent: {other}")),
        }
    }
}

pub(crate) fn detect_headroom_learn_prereq_status() -> HeadroomLearnPrereqStatus {
    let claude_path = claude_cli::detect_claude_cli();
    let codex_path = client_adapters::detect_codex_cli();
    HeadroomLearnPrereqStatus {
        claude_cli_available: claude_path.is_some(),
        claude_cli_path: claude_path.map(|p| p.display().to_string()),
        codex_cli_available: codex_path.is_some(),
        codex_cli_path: codex_path.map(|p| p.display().to_string()),
        codex_logged_in: client_adapters::codex_logged_in(),
    }
}

pub(crate) fn check_headroom_learn_prereqs(
    agent: LearnAgent,
    platform_disabled_reason: Option<&str>,
    prereq: &HeadroomLearnPrereqStatus,
) -> Result<(), String> {
    if let Some(reason) = platform_disabled_reason {
        return Err(reason.to_string());
    }
    match agent {
        LearnAgent::Claude => {
            if !prereq.claude_cli_available {
                return Err(
                    "Install the Claude Code CLI (`claude`) to enable Headroom Learn.".into(),
                );
            }
        }
        LearnAgent::Codex => {
            if !prereq.codex_cli_available {
                return Err(
                    "Install the Codex CLI (`codex`) to enable Headroom Learn for Codex.".into(),
                );
            }
            if !prereq.codex_logged_in {
                return Err("Sign in to the Codex CLI with your ChatGPT account to enable Headroom Learn for Codex.".into());
            }
        }
    }
    Ok(())
}

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
pub async fn start_headroom_learn(
    app: AppHandle,
    agent: String,
    project_path: Option<String>,
) -> Result<(), String> {
    let agent = LearnAgent::parse(&agent)?;
    if matches!(agent, LearnAgent::Claude) && project_path.is_none() {
        return Err("A project path is required for Claude Headroom Learn.".into());
    }
    check_headroom_learn_prereqs(
        agent,
        crate::state::headroom_learn_platform_message().as_deref(),
        &detect_headroom_learn_prereq_status(),
    )?;

    // Codex isn't project-organized, so its run-status is keyed on a stable id.
    let run_key = match agent {
        LearnAgent::Claude => project_path.clone().unwrap_or_default(),
        LearnAgent::Codex => "codex".to_string(),
    };
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.begin_headroom_learn_run(&run_key)?;
    }

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app_handle.state();
        let run = execute_headroom_learn_run(&state, agent, project_path.as_deref());
        state.complete_headroom_learn_run(run.success, run.summary, run.error, run.output_tail);
    });

    Ok(())
}

struct HeadroomLearnRunResult {
    success: bool,
    summary: String,
    error: Option<String>,
    output_tail: Vec<String>,
}

/// Detect `headroom.learn.analyzer` warnings that mean the LLM never produced
/// recommendations even though the CLI exited 0. Returns a user-facing message
/// joining all such warnings, or None if the run was clean.
pub(crate) fn extract_llm_failure_warnings(stderr: &str) -> Option<String> {
    const MARKER: &str = "LLM analysis failed:";
    let messages: Vec<String> = stderr
        .lines()
        .filter_map(|line| {
            line.split_once(MARKER)
                .map(|(_, rest)| format!("{} {}", MARKER, rest.trim()))
        })
        .collect();
    if messages.is_empty() {
        None
    } else {
        Some(messages.join("\n"))
    }
}

fn execute_headroom_learn_run(
    state: &AppState,
    agent: LearnAgent,
    project_path: Option<&str>,
) -> HeadroomLearnRunResult {
    // `run_id` keys the run-status + log file; `project_name` is the user-facing
    // label. Codex isn't project-organized, so it uses a stable "codex" id.
    let (run_id, project_name): (&str, String) = match agent {
        LearnAgent::Claude => {
            let path = project_path.unwrap_or("");
            let name = Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path)
                .to_string();
            (path, name)
        }
        LearnAgent::Codex => ("codex", "Codex sessions".to_string()),
    };
    let entrypoint = state.tool_manager.headroom_entrypoint();
    if !entrypoint.exists() {
        return HeadroomLearnRunResult {
            success: false,
            summary: format!("headroom learn failed for {project_name}."),
            error: Some(format!(
                "Headroom entrypoint not found at {}",
                entrypoint.display()
            )),
            output_tail: Vec::new(),
        };
    }
    // Pre-flight: the Claude scan passes --project to the CLI, where Click's
    // Path(readable=True) rejects a missing/unreadable dir with exit 2. That's a
    // user-environment condition (project moved/deleted, or macOS TCC blocking
    // ~/Documents et al.), not an app bug, so short-circuit here instead of
    // spawning and reporting the failure to Sentry. read_dir mirrors Click's
    // readability check and surfaces both the missing-path and TCC-denied cases.
    if let LearnAgent::Claude = agent {
        let path = project_path.unwrap_or_default();
        if path.is_empty() || std::fs::read_dir(path).is_err() {
            return HeadroomLearnRunResult {
                success: false,
                summary: format!("headroom learn failed for {project_name}."),
                error: Some(format!(
                    "Project path is not readable: {path}\n\
                     It may have been moved or deleted, or Headroom needs \
                     Files & Folders / Full Disk Access to read it."
                )),
                output_tail: Vec::new(),
            };
        }
    }

    let cli_path = match agent {
        LearnAgent::Claude => claude_cli::detect_claude_cli(),
        LearnAgent::Codex => client_adapters::detect_codex_cli(),
    };

    let mut command = Command::new(&entrypoint);
    command.arg("learn").arg("--apply");
    match agent {
        LearnAgent::Claude => {
            // Per-project Claude scan; writes CLAUDE.md / MEMORY.md for the
            // passed --project.
            command
                .arg("--project")
                .arg(project_path.unwrap_or_default())
                .arg("--agent")
                .arg("claude")
                .env("HEADROOM_LEARN_CLI", "claude");
        }
        LearnAgent::Codex => {
            // Codex scans all of ~/.codex/sessions (no --project) and writes
            // ~/.codex/AGENTS.md + instructions.md. Force --model codex-cli so
            // analysis runs through `codex exec` on the user's ChatGPT
            // subscription rather than auto-detecting an API key or the claude CLI.
            command
                .arg("--agent")
                .arg("codex")
                .arg("--model")
                .arg("codex-cli")
                .env("HEADROOM_LEARN_CLI", "codex");
        }
    }
    command
        // Run from an app-owned directory. For Claude the project is passed
        // explicitly via --project, so CWD is irrelevant; running elsewhere also
        // avoids getcwd() EPERM in spawned CLI shells when the project lives in a
        // TCC-protected location. The entrypoint's parent (inside Application
        // Support) is always accessible.
        .current_dir(
            entrypoint
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| std::path::PathBuf::from("/")),
        )
        .env("PYTHONNOUSERSITE", "1")
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .env("PIP_NO_INPUT", "1")
        // Force the selected CLI backend: the analyzer picks LiteLLM over
        // HEADROOM_LEARN_CLI / --model codex-cli when any of these keys is set
        // in the parent env.
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("GEMINI_API_KEY")
        // Don't pin ANTHROPIC_MODEL here: it's a LiteLLM identifier that the
        // analyzer never reads on the CLI path. Worse, it's inherited by the
        // spawned `claude -p` subprocess, where Claude Code's CLI does honor it -
        // and "claude-sonnet-4-6" is not a valid Claude Code model alias,
        // routing the call to a slow/hung path past 120s.
        .env_remove("ANTHROPIC_MODEL");
    if let Some(dir) = cli_path.as_ref().and_then(|p| p.parent()) {
        let existing = std::env::var("PATH").unwrap_or_default();
        let augmented = if existing.is_empty() {
            dir.display().to_string()
        } else {
            format!("{}:{}", dir.display(), existing)
        };
        command.env("PATH", augmented);
    }
    let output = command.output();

    let (summary, success, error, output_tail, stdout, stderr, status_copy) = match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let merged = if stderr.trim().is_empty() {
                stdout.clone()
            } else if stdout.trim().is_empty() {
                stderr.clone()
            } else {
                format!("{stdout}\n{stderr}")
            };
            let output_tail = crate::state::tail_lines(&merged, 32);
            if output.status.success() {
                if let Some(warnings) = extract_llm_failure_warnings(&stderr) {
                    (
                        format!(
                            "headroom learn could not produce recommendations for {project_name}."
                        ),
                        false,
                        Some(warnings),
                        output_tail,
                        stdout,
                        stderr,
                        output.status.to_string(),
                    )
                } else {
                    (
                        format!("headroom learn completed for {project_name}."),
                        true,
                        None,
                        output_tail,
                        stdout,
                        stderr,
                        output.status.to_string(),
                    )
                }
            } else {
                let fail_tail = if output_tail.is_empty() {
                    "No output captured.".to_string()
                } else {
                    output_tail.join("\n")
                };
                let exit_code_str = output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into());
                let signal_num: Option<i32> = {
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        output.status.signal()
                    }
                    #[cfg(not(unix))]
                    {
                        None
                    }
                };
                // First non-empty line of stderr (or stdout if stderr empty),
                // truncated, used both in the message and the fingerprint so
                // events group by failure mode instead of the capture-site stack.
                let signature_source = if !stderr.trim().is_empty() {
                    stderr.as_str()
                } else {
                    stdout.as_str()
                };
                let signature: String = signature_source
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or("no output")
                    .chars()
                    .take(160)
                    .collect();
                let stderr_head: String = stderr.chars().take(2000).collect();
                let stdout_head: String = stdout.chars().take(2000).collect();
                let cli_path_str = cli_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "not_found".into());
                let summary_msg =
                    format!("headroom learn failed (exit={exit_code_str}) {signature}");
                let fingerprint: [&str; 3] =
                    ["headroom_learn", exit_code_str.as_str(), signature.as_str()];
                // Defense in depth against a TOCTOU race: the path can become
                // unreadable between the pre-flight read_dir check and the CLI
                // run. Click reports that as exit 2 with "is not readable" - a
                // user-environment condition, not an app bug, so don't report it.
                let user_env_condition = signature.contains("is not readable");
                if !user_env_condition {
                    sentry::with_scope(
                        |scope| {
                            scope.set_tag("flow", "headroom_learn");
                            scope.set_tag(
                                "learn_agent",
                                match agent {
                                    LearnAgent::Claude => "claude",
                                    LearnAgent::Codex => "codex",
                                },
                            );
                            scope.set_tag("exit_code", &exit_code_str);
                            scope.set_extra("exit_status", output.status.to_string().into());
                            scope.set_extra(
                                "signal",
                                signal_num
                                    .map(|s| s.to_string().into())
                                    .unwrap_or(serde_json::Value::Null),
                            );
                            scope.set_extra("output_tail", fail_tail.clone().into());
                            scope.set_extra("stderr_head", stderr_head.into());
                            scope.set_extra("stdout_head", stdout_head.into());
                            scope.set_extra("cli_path", cli_path_str.into());
                            scope.set_extra("project_name", project_name.to_string().into());
                            scope.set_fingerprint(Some(fingerprint.as_slice()));
                        },
                        || {
                            sentry::capture_message(&summary_msg, sentry::Level::Error);
                        },
                    );
                }
                (
                    format!("headroom learn failed for {project_name}."),
                    false,
                    Some(format!(
                        "headroom learn exited with {}.\n{}",
                        output.status, fail_tail
                    )),
                    output_tail,
                    stdout,
                    stderr,
                    output.status.to_string(),
                )
            }
        }
        Err(err) => {
            sentry::capture_message(
                &format!("headroom learn spawn failed: {err}"),
                sentry::Level::Error,
            );
            (
                format!("headroom learn failed for {project_name}."),
                false,
                Some(format!("Could not start headroom learn: {err}")),
                Vec::new(),
                String::new(),
                String::new(),
                "spawn_error".to_string(),
            )
        }
    };

    let log_path = state.tool_manager.headroom_learn_log_path(run_id);
    let log_content = format!(
        "[{}] headroom learn --agent {} (target={})\nstatus: {}\n\n--- stdout ---\n{}\n\n--- stderr ---\n{}\n",
        Utc::now().to_rfc3339(),
        match agent {
            LearnAgent::Claude => "claude",
            LearnAgent::Codex => "codex",
        },
        run_id,
        status_copy,
        stdout,
        stderr
    );
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(log_path, log_content);

    HeadroomLearnRunResult {
        success,
        summary,
        error,
        output_tail,
    }
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
    let memory_md = crate::headroom_learn::claude_project_memory_file(project_path);

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
        "memory" => crate::headroom_learn::claude_project_memory_file(&project_path),
        other => return Err(format!("Unknown file_kind: {other}")),
    };
    if !path.exists() {
        return Err(format!("{} does not exist.", path.display()));
    }
    let content =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let updated =
        crate::headroom_learn::delete_applied_bullet(&content, &section_title, &bullet_text);
    if updated == content {
        return Ok(());
    }
    std::fs::write(&path, updated).map_err(|err| format!("write {}: {err}", path.display()))?;
    Ok(())
}

fn read_applied_block(path: &Path) -> Vec<AppliedSection> {
    match std::fs::read_to_string(path) {
        Ok(content) => crate::headroom_learn::parse_headroom_learn_block(&content),
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
