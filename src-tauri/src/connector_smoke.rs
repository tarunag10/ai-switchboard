use std::process::Command;
use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectorSmokeTestResult {
    client_id: String,
    supported: bool,
    launched: bool,
    success: bool,
    summary: String,
    stdout_tail: String,
    stderr_tail: String,
}

fn connector_smoke_working_dir() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn tail_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }
    value
        .chars()
        .skip(char_count.saturating_sub(max_chars))
        .collect()
}

pub(crate) fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(crate) fn connector_smoke_shell_command(client_id: &str, prompt: &str) -> Option<String> {
    let quoted_prompt = shell_single_quote(prompt);
    match client_id {
        "codex" => Some(format!(
            "codex exec --ephemeral --sandbox read-only --skip-git-repo-check --ignore-rules {quoted_prompt}"
        )),
        "claude_code" => Some(format!(
            "claude --print --no-session-persistence --permission-mode dontAsk --tools '' --output-format text {quoted_prompt}"
        )),
        _ => None,
    }
}

fn connector_smoke_command(client_id: &str, prompt: &str) -> Option<Command> {
    let shell_command = connector_smoke_shell_command(client_id, prompt)?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut command = Command::new(shell);
    command.args(["-lc", &shell_command]);
    Some(command)
}

#[tauri::command]
pub async fn run_connector_smoke_test(
    client_id: String,
) -> Result<ConnectorSmokeTestResult, String> {
    let prompt = "Reply with exactly: switchboard verification ok";
    let Some(mut command) = connector_smoke_command(&client_id, prompt) else {
        return Ok(ConnectorSmokeTestResult {
            client_id,
            supported: false,
            launched: false,
            success: false,
            summary: "One-click test is available for Claude Code and Codex. For this connector, send a tiny prompt manually and watch this screen for verification.".into(),
            stdout_tail: String::new(),
            stderr_tail: String::new(),
        });
    };

    command.current_dir(connector_smoke_working_dir());
    command.env("NO_COLOR", "1");
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| format!("Could not launch {client_id} smoke test: {err}"))?;

    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(250));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{client_id} smoke test timed out after 90 seconds."
                ));
            }
            Err(err) => return Err(format!("{client_id} smoke test failed to run: {err}")),
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Could not collect {client_id} smoke test output: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();
    Ok(ConnectorSmokeTestResult {
        client_id,
        supported: true,
        launched: true,
        success,
        summary: if success {
            "Test prompt sent. Waiting for the local proxy to confirm the request.".into()
        } else {
            format!(
                "Test prompt exited with status {}. Open the connector and send a tiny prompt manually.",
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "terminated".into())
            )
        },
        stdout_tail: tail_text(&stdout, 800),
        stderr_tail: tail_text(&stderr, 1200),
    })
}
