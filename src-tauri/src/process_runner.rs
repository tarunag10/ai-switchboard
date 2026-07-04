use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

/// Prepend a binary's own directory to PATH so an `#!/usr/bin/env node`
/// shebang (or similar) resolves the interpreter that nvm installs alongside
/// it. Falls back to the existing PATH when the binary has no parent.
pub(crate) fn path_with_binary_dir(binary: &Path) -> String {
    let existing = std::env::var("PATH").unwrap_or_default();
    match binary.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => {
            if existing.is_empty() {
                dir.display().to_string()
            } else {
                format!("{}:{}", dir.display(), existing)
            }
        }
        _ => existing,
    }
}

pub(crate) fn build_command(binary: &Path, args: &[&str], cwd: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(cwd)
        .env_remove("PYTHONHOME")
        // GUI apps inherit a minimal PATH lacking the nvm/homebrew bin dir, so a
        // CLI with a `#!/usr/bin/env node` shebang (e.g. codex) fails with exit
        // 127 / "env: node: No such file or directory". node lives alongside the
        // CLI in nvm's bin, so prepend the binary's own dir to PATH.
        .env("PATH", path_with_binary_dir(binary))
        .env_remove("PYTHONPATH")
        .env_remove("PYTHONSTARTUP")
        .env("PYTHONNOUSERSITE", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("LC_ALL", "C.UTF-8")
        .env("LANG", "C.UTF-8")
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .env("PIP_NO_INPUT", "1");
    command
}

/// Like `run_command` but streams stdout + stderr line-by-line through
/// `on_line` in real time. Captures everything for the structured failure
/// payload so error reporting is unchanged.
pub(crate) fn run_command_streaming<F>(
    binary: &Path,
    args: &[&str],
    cwd: &Path,
    on_line: &mut F,
) -> Result<()>
where
    F: FnMut(&str),
{
    use std::io::{BufRead, BufReader};
    use std::sync::mpsc;

    let mut cmd = build_command(binary, args, cwd);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting {} {}", binary.display(), args.join(" ")))?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    let (tx, rx) = mpsc::channel::<StreamedLine>();
    let tx_stdout = tx.clone();
    let tx_stderr = tx.clone();
    drop(tx);

    let stdout_handle = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = tx_stdout.send(StreamedLine {
                line,
                is_stderr: false,
            });
        }
    });
    let stderr_handle = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = tx_stderr.send(StreamedLine {
                line,
                is_stderr: true,
            });
        }
    });

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();

    while let Ok(streamed) = rx.recv() {
        on_line(&streamed.line);
        let sink = if streamed.is_stderr {
            &mut stderr_buf
        } else {
            &mut stdout_buf
        };
        sink.push_str(&streamed.line);
        sink.push('\n');
    }

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    let status = child
        .wait()
        .with_context(|| format!("waiting for {} {}", binary.display(), args.join(" ")))?;

    if !status.success() {
        return Err(anyhow::Error::new(CommandFailure {
            program: binary.display().to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout: stdout_buf,
            stderr: stderr_buf,
            exit_code: status.code(),
            signal: exit_status_signal(&status),
        }));
    }

    Ok(())
}

struct StreamedLine {
    line: String,
    is_stderr: bool,
}

pub(crate) fn run_command_with_timeout(
    binary: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Result<()> {
    use std::io::Read;
    use std::sync::mpsc;

    let mut cmd = build_command(binary, args, cwd);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting {} {}", binary.display(), args.join(" ")))?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    let (stdout_tx, stdout_rx) = mpsc::channel::<Vec<u8>>();
    let (stderr_tx, stderr_rx) = mpsc::channel::<Vec<u8>>();
    let stdout_handle = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        let _ = stdout_tx.send(buf);
    });
    let stderr_handle = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stderr);
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        let _ = stderr_tx.send(buf);
    });

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    timed_out = true;
                    let _ = child.kill();
                    break child.wait().with_context(|| {
                        format!("waiting for {} {}", binary.display(), args.join(" "))
                    })?;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(err).with_context(|| {
                    format!("waiting for {} {}", binary.display(), args.join(" "))
                });
            }
        }
    };

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();
    let stdout = String::from_utf8_lossy(&stdout_rx.recv().unwrap_or_default()).into_owned();
    let mut stderr = String::from_utf8_lossy(&stderr_rx.recv().unwrap_or_default()).into_owned();

    if timed_out {
        if !stderr.is_empty() && !stderr.ends_with('\n') {
            stderr.push('\n');
        }
        stderr.push_str(&format!(
            "command timed out after {}ms",
            timeout.as_millis()
        ));
        return Err(anyhow::Error::new(CommandFailure {
            program: binary.display().to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout,
            stderr,
            exit_code: None,
            signal: exit_status_signal(&status),
        }));
    }

    if !status.success() {
        return Err(anyhow::Error::new(CommandFailure {
            program: binary.display().to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout,
            stderr,
            exit_code: status.code(),
            signal: exit_status_signal(&status),
        }));
    }

    Ok(())
}

pub(crate) fn run_command(binary: &Path, args: &[&str], cwd: &Path) -> Result<()> {
    let output = build_command(binary, args, cwd)
        .output()
        .with_context(|| format!("starting {} {}", binary.display(), args.join(" ")))?;

    if !output.status.success() {
        return Err(anyhow::Error::new(CommandFailure {
            program: binary.display().to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
            signal: exit_status_signal(&output.status),
        }));
    }

    Ok(())
}

/// Structured failure from a shell-out. Carried through `anyhow::Error` so callers
/// can `.context()` as usual, and capture sites (e.g. Sentry) can downcast to pull
/// stdout/stderr into structured fields instead of a truncated message string.
#[derive(Debug)]
pub struct CommandFailure {
    pub program: String,
    pub args: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    /// Unix signal number when the child was killed by a signal (`exit_code` is
    /// `None` in that case). Lets us tell SIGKILL (9 - likely parent shutdown,
    /// OOM, or launchd) from SIGTERM (15 - graceful kill) in failure reports.
    pub signal: Option<i32>,
}

impl std::fmt::Display for CommandFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match (self.exit_code, self.signal) {
            (Some(code), _) => format!("exit {}", code),
            (None, Some(sig)) => format!("killed by signal {}", sig),
            (None, None) => "killed by signal".to_string(),
        };
        write!(
            f,
            "command failed ({}): {} {}\nstdout:\n{}\nstderr:\n{}",
            status,
            self.program,
            self.args.join(" "),
            self.stdout,
            self.stderr
        )
    }
}

impl std::error::Error for CommandFailure {}

/// Extract the Unix signal number that killed a child, or `None` on non-Unix
/// or when the process exited normally. Used to populate `CommandFailure.signal`
/// so failure reports distinguish SIGKILL from SIGTERM.
pub(crate) fn exit_status_signal(status: &std::process::ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}
