use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

const MAX_CAPTURE_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_STREAMING_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const STREAM_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Prepend a binary's own directory to PATH so an `#!/usr/bin/env node`
/// shebang (or similar) resolves the interpreter that nvm installs alongside
/// it. Falls back to the existing PATH when the binary has no parent.
pub(crate) fn path_with_binary_dir(binary: &Path) -> Result<OsString> {
    let existing = std::env::var_os("PATH");
    path_with_binary_dir_from(binary, existing.as_deref())
}

fn path_with_binary_dir_from(binary: &Path, existing: Option<&OsStr>) -> Result<OsString> {
    match binary.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => {
            let mut paths = vec![dir.to_path_buf()];
            if let Some(existing) = existing.filter(|path| !path.is_empty()) {
                paths.extend(std::env::split_paths(existing));
            }
            std::env::join_paths(paths)
                .with_context(|| format!("constructing PATH for {}", binary.display()))
        }
        _ => Ok(existing.unwrap_or_default().to_os_string()),
    }
}

#[cfg(unix)]
fn ensure_managed_process_execution_supported() -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn ensure_managed_process_execution_supported() -> Result<()> {
    Err(anyhow::anyhow!(
        "managed process execution is unsupported on non-Unix platforms until process-tree containment is available"
    ))
}

pub(crate) fn build_command(binary: &Path, args: &[&str], cwd: &Path) -> Result<Command> {
    ensure_managed_process_execution_supported()?;

    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(cwd)
        .env_remove("PYTHONHOME")
        // GUI apps inherit a minimal PATH lacking the nvm/homebrew bin dir, so a
        // CLI with a `#!/usr/bin/env node` shebang (e.g. codex) fails with exit
        // 127 / "env: node: No such file or directory". node lives alongside the
        // CLI in nvm's bin, so prepend the binary's own dir to PATH.
        .env("PATH", path_with_binary_dir(binary)?)
        .env_remove("PYTHONPATH")
        .env_remove("PYTHONSTARTUP")
        .env("PYTHONNOUSERSITE", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("LC_ALL", "C.UTF-8")
        .env("LANG", "C.UTF-8")
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .env("PIP_NO_INPUT", "1");
    #[cfg(unix)]
    configure_process_group(&mut command);
    Ok(command)
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    // Keep the launcher and every descendant in an app-owned process group so
    // timeout/error cleanup cannot leave grandchildren holding pipes or ports.
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
}

fn terminate_process_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        if let Ok(pid) = i32::try_from(child.id()) {
            if pid > 0 {
                // Negative PIDs address the entire process group created above.
                unsafe {
                    let _ = libc::kill(-pid, libc::SIGKILL);
                }
            }
        }
    }
    let _ = child.kill();
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
    run_command_streaming_with_timeout(binary, args, cwd, DEFAULT_STREAMING_TIMEOUT, on_line)
}

pub(crate) fn run_command_streaming_with_timeout<F>(
    binary: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
    on_line: &mut F,
) -> Result<()>
where
    F: FnMut(&str),
{
    use std::io::{BufRead, BufReader};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    };

    let mut cmd = build_command(binary, args, cwd)?;
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

    let stdout_capture_limit_exceeded = Arc::new(AtomicBool::new(false));
    let stderr_capture_limit_exceeded = Arc::new(AtomicBool::new(false));
    let stdout_reader_limit = Arc::clone(&stdout_capture_limit_exceeded);
    let stderr_reader_limit = Arc::clone(&stderr_capture_limit_exceeded);

    let stdout_handle = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut raw = Vec::new();
        let mut observed_bytes = 0usize;
        loop {
            raw.clear();
            match reader.read_until(b'\n', &mut raw) {
                Ok(0) => break,
                Ok(_) => {
                    observed_bytes = observed_bytes.saturating_add(raw.len());
                    if observed_bytes > MAX_CAPTURE_BYTES {
                        stdout_reader_limit.store(true, Ordering::Relaxed);
                    }
                    let line = String::from_utf8_lossy(&raw)
                        .trim_end_matches(['\r', '\n'])
                        .to_string();
                    let _ = tx_stdout.send(StreamedLine {
                        line,
                        is_stderr: false,
                    });
                }
                Err(_) => break,
            }
        }
    });
    let stderr_handle = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut raw = Vec::new();
        let mut observed_bytes = 0usize;
        loop {
            raw.clear();
            match reader.read_until(b'\n', &mut raw) {
                Ok(0) => break,
                Ok(_) => {
                    observed_bytes = observed_bytes.saturating_add(raw.len());
                    if observed_bytes > MAX_CAPTURE_BYTES {
                        stderr_reader_limit.store(true, Ordering::Relaxed);
                    }
                    let line = String::from_utf8_lossy(&raw)
                        .trim_end_matches(['\r', '\n'])
                        .to_string();
                    let _ = tx_stderr.send(StreamedLine {
                        line,
                        is_stderr: true,
                    });
                }
                Err(_) => break,
            }
        }
    });

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();
    let mut stdout_truncated = false;
    let mut stderr_truncated = false;
    let started = Instant::now();
    let mut child_status = None;
    let mut drain_deadline = None;
    let mut timed_out = false;
    let mut forced_pipe_cleanup = false;

    loop {
        if stdout_capture_limit_exceeded.load(Ordering::Relaxed)
            || stderr_capture_limit_exceeded.load(Ordering::Relaxed)
        {
            terminate_process_tree(&mut child);
            child_status =
                Some(child.wait().with_context(|| {
                    format!("waiting for {} {}", binary.display(), args.join(" "))
                })?);
            break;
        }

        if child_status.is_none() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    child_status = Some(status);
                    drain_deadline = Some(Instant::now() + STREAM_DRAIN_GRACE);
                }
                Ok(None) if started.elapsed() >= timeout => {
                    if stdout_capture_limit_exceeded.load(Ordering::Relaxed)
                        || stderr_capture_limit_exceeded.load(Ordering::Relaxed)
                    {
                        terminate_process_tree(&mut child);
                        child_status = Some(child.wait().with_context(|| {
                            format!("waiting for {} {}", binary.display(), args.join(" "))
                        })?);
                        break;
                    }
                    timed_out = true;
                    terminate_process_tree(&mut child);
                    child_status = Some(child.wait().with_context(|| {
                        format!("waiting for {} {}", binary.display(), args.join(" "))
                    })?);
                    break;
                }
                Ok(None) => {}
                Err(err) => {
                    terminate_process_tree(&mut child);
                    let _ = child.wait();
                    return Err(err).with_context(|| {
                        format!("waiting for {} {}", binary.display(), args.join(" "))
                    });
                }
            }
        } else if drain_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            // The launcher exited but a descendant may still hold a pipe.
            // Kill the app-owned group and detach the reader threads instead
            // of allowing a GUI command to wait forever for EOF.
            forced_pipe_cleanup = true;
            terminate_process_tree(&mut child);
            break;
        }

        match rx.recv_timeout(Duration::from_millis(25)) {
            Ok(streamed) => {
                on_line(&streamed.line);
                let sink = if streamed.is_stderr {
                    &mut stderr_buf
                } else {
                    &mut stdout_buf
                };
                if sink.len() < MAX_CAPTURE_BYTES {
                    let remaining = MAX_CAPTURE_BYTES - sink.len();
                    if streamed.line.len() + 1 <= remaining {
                        sink.push_str(&streamed.line);
                        sink.push('\n');
                    } else {
                        let copy = remaining.min(streamed.line.len());
                        sink.push_str(&String::from_utf8_lossy(&streamed.line.as_bytes()[..copy]));
                        sink.truncate(MAX_CAPTURE_BYTES);
                        if streamed.is_stderr {
                            stderr_truncated = true;
                        } else {
                            stdout_truncated = true;
                        }
                    }
                } else if streamed.is_stderr {
                    stderr_truncated = true;
                } else {
                    stdout_truncated = true;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }

    if !timed_out && !forced_pipe_cleanup {
        let _ = stdout_handle.join();
        let _ = stderr_handle.join();
    }
    stdout_truncated |= stdout_capture_limit_exceeded.load(Ordering::Relaxed);
    stderr_truncated |= stderr_capture_limit_exceeded.load(Ordering::Relaxed);

    let status = child_status.unwrap_or(
        child
            .wait()
            .with_context(|| format!("waiting for {} {}", binary.display(), args.join(" ")))?,
    );

    if timed_out {
        stderr_buf.push_str(&format!(
            "\ncommand timed out after {}ms",
            timeout.as_millis()
        ));
        return Err(anyhow::Error::new(CommandFailure {
            program: binary.display().to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout: stdout_buf,
            stderr: stderr_buf,
            exit_code: None,
            signal: exit_status_signal(&status),
        }));
    }

    if stdout_truncated || stderr_truncated {
        stderr_buf.push_str(&format!(
            "\ncommand output exceeded the {} byte capture limit",
            MAX_CAPTURE_BYTES
        ));
        return Err(anyhow::Error::new(CommandFailure {
            program: binary.display().to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout: stdout_buf,
            stderr: stderr_buf,
            exit_code: status.code(),
            signal: exit_status_signal(&status),
        }));
    }

    if forced_pipe_cleanup {
        stderr_buf.push_str("\nstream output pipes did not close after the process exited");
        return Err(anyhow::Error::new(CommandFailure {
            program: binary.display().to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout: stdout_buf,
            stderr: stderr_buf,
            exit_code: status.code(),
            signal: exit_status_signal(&status),
        }));
    }

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
    run_command_capture_with_timeout(binary, args, cwd, timeout).map(|_| ())
}

pub(crate) fn run_command_capture_with_timeout(
    binary: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Result<(String, String)> {
    use std::sync::mpsc;

    let mut cmd = build_command(binary, args, cwd)?;
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting {} {}", binary.display(), args.join(" ")))?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    let (stdout_tx, stdout_rx) = mpsc::channel::<CapturedStream>();
    let (stderr_tx, stderr_rx) = mpsc::channel::<CapturedStream>();
    let stdout_handle = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        let _ = stdout_tx.send(read_bounded(&mut reader));
    });
    let stderr_handle = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stderr);
        let _ = stderr_tx.send(read_bounded(&mut reader));
    });

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    timed_out = true;
                    terminate_process_tree(&mut child);
                    break child.wait().with_context(|| {
                        format!("waiting for {} {}", binary.display(), args.join(" "))
                    })?;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                terminate_process_tree(&mut child);
                let _ = child.wait();
                return Err(err).with_context(|| {
                    format!("waiting for {} {}", binary.display(), args.join(" "))
                });
            }
        }
    };

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();
    let stdout_capture = stdout_rx.recv().unwrap_or_default();
    let stderr_capture = stderr_rx.recv().unwrap_or_default();
    let stdout = String::from_utf8_lossy(&stdout_capture.bytes).into_owned();
    let mut stderr = String::from_utf8_lossy(&stderr_capture.bytes).into_owned();

    if stdout_capture.truncated || stderr_capture.truncated {
        if !stderr.is_empty() && !stderr.ends_with('\n') {
            stderr.push('\n');
        }
        stderr.push_str(&format!(
            "command output exceeded the {} byte capture limit",
            MAX_CAPTURE_BYTES
        ));
        return Err(anyhow::Error::new(CommandFailure {
            program: binary.display().to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout,
            stderr,
            exit_code: status.code(),
            signal: exit_status_signal(&status),
        }));
    }

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

    Ok((stdout, stderr))
}

#[derive(Default)]
struct CapturedStream {
    bytes: Vec<u8>,
    truncated: bool,
}

fn read_bounded(reader: &mut impl Read) -> CapturedStream {
    let mut captured = CapturedStream::default();
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                let remaining = MAX_CAPTURE_BYTES.saturating_sub(captured.bytes.len());
                let copy_count = remaining.min(count);
                captured.bytes.extend_from_slice(&buffer[..copy_count]);
                if copy_count < count {
                    captured.truncated = true;
                    break;
                }
            }
            Err(_) => break,
        }
    }
    captured
}

pub(crate) fn run_command(binary: &Path, args: &[&str], cwd: &Path) -> Result<()> {
    let output = build_command(binary, args, cwd)?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_with_binary_dir_prepends_parent_and_preserves_components() {
        let existing_paths = ["existing-one", "existing-two"];
        let existing = std::env::join_paths(existing_paths).expect("synthetic PATH");
        let binary = Path::new("tool-root").join("bin").join("switchboard");

        let augmented =
            path_with_binary_dir_from(&binary, Some(existing.as_os_str())).expect("augment PATH");
        let components = std::env::split_paths(&augmented).collect::<Vec<_>>();

        assert_eq!(
            components,
            vec![
                Path::new("tool-root").join("bin"),
                Path::new("existing-one").to_path_buf(),
                Path::new("existing-two").to_path_buf(),
            ]
        );
    }

    #[test]
    fn path_with_binary_dir_preserves_existing_path_for_bare_binary() {
        let existing =
            std::env::join_paths(["existing-one", "existing-two"]).expect("synthetic PATH");

        let unchanged =
            path_with_binary_dir_from(Path::new("switchboard"), Some(existing.as_os_str()))
                .expect("preserve PATH");

        assert_eq!(unchanged, existing);
    }

    #[cfg(not(unix))]
    #[test]
    fn build_command_fails_closed_without_process_tree_containment() {
        let error = build_command(Path::new("must-not-spawn"), &[], Path::new("."))
            .expect_err("non-Unix command construction must fail closed");

        assert_eq!(
            error.to_string(),
            "managed process execution is unsupported on non-Unix platforms until process-tree containment is available"
        );
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_descendants_in_the_app_owned_process_group() {
        use std::fs;
        use std::time::SystemTime;

        let path = std::env::temp_dir().join(format!(
            "switchboard-process-runner-{}-{}.pid",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let path_text = path.to_string_lossy().into_owned();
        let result = run_command_with_timeout(
            Path::new("/bin/sh"),
            &[
                "-c",
                &format!("sleep 30 & echo $! > '{}' ; wait", path_text),
            ],
            Path::new("/tmp"),
            Duration::from_millis(100),
        );
        assert!(result.is_err(), "timed out command must fail");
        let pid = fs::read_to_string(&path)
            .expect("child pid was written")
            .trim()
            .parse::<i32>()
            .expect("child pid is numeric");
        let _ = fs::remove_file(&path);

        for _ in 0..20 {
            let alive = unsafe { libc::kill(pid, 0) == 0 };
            if !alive {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let status = Command::new("/bin/sh")
            .args(["-c", &format!("kill -0 {}", pid)])
            .status()
            .expect("probe descendant");
        assert!(
            !status.success(),
            "timed-out descendant survived process-group cleanup"
        );
    }

    #[cfg(unix)]
    #[test]
    fn capture_runner_rejects_output_above_the_bound() {
        let result = run_command_capture_with_timeout(
            Path::new("/bin/sh"),
            &["-c", "yes x | head -c 3000000"],
            Path::new("/tmp"),
            Duration::from_secs(2),
        )
        .expect_err("oversized output must fail closed");
        assert!(result.to_string().contains("capture limit"));
    }

    #[cfg(unix)]
    #[test]
    fn streaming_runner_times_out_and_preserves_lossy_output() {
        let mut lines = Vec::new();
        let result = run_command_streaming_with_timeout(
            Path::new("/bin/sh"),
            &["-c", "printf '\\377tail\\n'; sleep 30"],
            Path::new("/tmp"),
            Duration::from_millis(100),
            &mut |line| lines.push(line.to_string()),
        );
        let error = result
            .expect_err("streaming command must time out")
            .to_string();
        assert!(error.contains("timed out"));
        assert!(lines.iter().any(|line| line.contains("tail")));
    }

    #[cfg(unix)]
    #[test]
    fn streaming_runner_rejects_retained_output_above_the_bound() {
        let error = run_command_streaming_with_timeout(
            Path::new("/bin/sh"),
            &["-c", "yes x | head -c 3000000"],
            Path::new("/tmp"),
            Duration::from_secs(2),
            &mut |_| {},
        )
        .expect_err("streaming output must fail closed")
        .to_string();
        assert!(error.contains("capture limit"), "unexpected error: {error}");
    }
}
