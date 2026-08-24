#[cfg(any(unix, test))]
use std::ffi::OsStr;
#[cfg(unix)]
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
#[cfg(unix)]
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[cfg(unix)]
const SHELL_LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);
const SMOKE_TEST_TIMEOUT: Duration = Duration::from_secs(3);
#[cfg(any(windows, test))]
const WINDOWS_PATH_LOOKUP_FAILED_DIAGNOSTIC: &str =
    "cli_discovery_windows_path_lookup_failed_closed";
#[cfg(not(any(unix, windows)))]
const UNSUPPORTED_PLATFORM_DIAGNOSTIC: &str = "cli_discovery_unsupported_platform_failed_closed";

pub fn detect_claude_cli() -> Option<PathBuf> {
    detect_cli("claude")
}

pub fn detect_codex_cli() -> Option<PathBuf> {
    detect_cli("codex")
}

/// Checks only whether a fixed, known CLI candidate exists as a regular file.
///
/// This intentionally does not consult the login shell, resolve `PATH`, read a
/// CLI version, or start a process. Callers must not treat a positive result
/// as proof that the candidate is runnable.
pub fn known_cli_candidate_present_without_start(name: &str) -> bool {
    if !matches!(name, "claude" | "claude-code" | "codex") {
        return false;
    }
    has_regular_file_candidate(known_path_candidates(home_dir(), name))
}

fn has_regular_file_candidate(candidates: impl IntoIterator<Item = PathBuf>) -> bool {
    candidates.into_iter().any(|candidate| candidate.is_file())
}

fn detect_cli(name: &str) -> Option<PathBuf> {
    if let Some(path) = probe_known_paths(name) {
        return Some(path);
    }
    probe_via_platform(name)
}

fn probe_known_paths(name: &str) -> Option<PathBuf> {
    first_runnable(known_path_candidates(home_dir(), name).into_iter())
}

fn known_path_candidates(home: PathBuf, name: &str) -> Vec<PathBuf> {
    known_path_candidates_for(home, name, cfg!(unix))
}

fn known_path_candidates_for(home: PathBuf, name: &str, unix_layout: bool) -> Vec<PathBuf> {
    if !unix_layout {
        return Vec::new();
    }

    vec![
        home.join(".claude").join("local").join(name),
        PathBuf::from(format!("/opt/homebrew/bin/{name}")),
        PathBuf::from(format!("/usr/local/bin/{name}")),
        home.join(".npm-global").join("bin").join(name),
        home.join(".volta").join("bin").join(name),
        home.join(".bun").join("bin").join(name),
        PathBuf::from(format!("/usr/bin/{name}")),
    ]
}

fn first_runnable<I: Iterator<Item = PathBuf>>(candidates: I) -> Option<PathBuf> {
    candidates
        .into_iter()
        .find(|candidate| is_runnable(candidate))
}

#[cfg(unix)]
fn probe_via_platform(name: &str) -> Option<PathBuf> {
    probe_via_login_shell(name)
}

#[cfg(windows)]
fn probe_via_platform(name: &str) -> Option<PathBuf> {
    let candidate = std::env::var_os("PATH").and_then(|path_var| {
        crate::client_detection::find_on_path_entries(std::env::split_paths(&path_var), &[name])
    });
    let candidate = first_runnable(candidate.into_iter());
    if candidate.is_none() {
        eprintln!("{WINDOWS_PATH_LOOKUP_FAILED_DIAGNOSTIC}:{name}");
    }
    candidate
}

#[cfg(not(any(unix, windows)))]
fn probe_via_platform(name: &str) -> Option<PathBuf> {
    eprintln!("{UNSUPPORTED_PLATFORM_DIAGNOSTIC}:{name}");
    None
}

#[cfg(any(unix, test))]
fn unix_login_shell_spec(shell: Option<&OsStr>) -> (PathBuf, &'static str) {
    let shell = shell
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"));
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("zsh");
    let flags = match shell_name {
        "fish" => "-lc",
        _ => "-ilc",
    };
    (shell, flags)
}

#[cfg(unix)]
fn probe_via_login_shell(name: &str) -> Option<PathBuf> {
    let configured_shell = std::env::var_os("SHELL");
    let (shell, flags) = unix_login_shell_spec(configured_shell.as_deref());

    let mut command = Command::new(&shell);
    command.arg(flags).arg(format!("command -v {name}"));
    read_path_from_shell(command, SHELL_LOOKUP_TIMEOUT)
}

/// Spawns `command`, reads the first non-empty line from its stdout, kills
/// the child, and returns the line as a validated `PathBuf`. The timeout
/// bounds how long we wait for that first line — NOT how long we wait for
/// the child to exit. Interactive shells (`-ilc`) print the `command -v`
/// result immediately but then run through `.zshrc`, so waiting for exit
/// before reading stdout was dropping valid paths on the floor.
#[cfg(unix)]
fn read_path_from_shell(mut command: Command, timeout: Duration) -> Option<PathBuf> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            let _ = tx.send(trimmed);
            return;
        }
    });

    let first_line = rx.recv_timeout(timeout).ok();
    let _ = child.kill();
    let _ = child.wait();

    let first_line = first_line?;
    if first_line.is_empty() {
        return None;
    }
    let path = PathBuf::from(first_line);
    if is_runnable(&path) {
        Some(path)
    } else {
        None
    }
}

fn is_executable(path: &Path) -> bool {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(_) => return false,
    };
    if !meta.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

/// `is_executable` only checks the POSIX exec bit; on dual-architecture Macs
/// an Intel-only Homebrew remnant in `/usr/local/bin` will satisfy the bit but
/// the kernel returns `ENOEXEC` when we try to run it. Smoke-test by spawning
/// `<path> --version` with a short timeout and rejecting anything that fails
/// to spawn or exits non-zero.
///
/// PATH augmentation: the candidate's parent directory is prepended to PATH
/// for the smoke test. nvm/volta/bun/asdf-managed `claude` installs are
/// `#!/usr/bin/env node` scripts with `node` colocated in the same bin dir;
/// without this, GUI launches inherit launchd's bare PATH and `env` fails to
/// resolve `node`, exit 127, and we'd reject a perfectly working `claude`.
fn is_runnable(path: &Path) -> bool {
    if !is_executable(path) {
        return false;
    }
    let mut command = Command::new(path);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = path.parent() {
        let mut entries = vec![dir.to_path_buf()];
        if let Some(existing) = std::env::var_os("PATH") {
            entries.extend(std::env::split_paths(&existing));
        }
        let augmented = match std::env::join_paths(entries) {
            Ok(augmented) => augmented,
            Err(_) => return false,
        };
        command.env("PATH", augmented);
    }
    let child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return false,
    };
    matches!(wait_with_timeout(child, SMOKE_TEST_TIMEOUT), Some(status) if status.success())
}

fn wait_with_timeout(mut child: Child, timeout: Duration) -> Option<ExitStatus> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn home_dir() -> PathBuf {
    dirs::home_dir()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::time::Instant;

    #[derive(Debug, PartialEq, Eq)]
    enum DiscoveryFallback {
        UnixLoginShell,
        WindowsPath,
        FailClosed,
    }

    fn discovery_fallback_for_target(is_unix: bool, is_windows: bool) -> DiscoveryFallback {
        match (is_unix, is_windows) {
            (true, false) => DiscoveryFallback::UnixLoginShell,
            (false, true) => DiscoveryFallback::WindowsPath,
            _ => DiscoveryFallback::FailClosed,
        }
    }

    struct ScopedTempDir(PathBuf);
    impl ScopedTempDir {
        fn new(label: &str) -> Self {
            let base = std::env::temp_dir().join(format!(
                "headroom_claude_cli_{}_{}",
                label,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&base);
            fs::create_dir_all(&base).unwrap();
            Self(base)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for ScopedTempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct EnvRestore {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvRestore {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn is_executable_accepts_executable_files() {
        let tmp = ScopedTempDir::new("is_exec_ok");
        let path = tmp.path().join("claude");
        make_executable(&path);
        assert!(is_executable(&path));
    }

    #[test]
    #[cfg(unix)]
    fn is_executable_rejects_non_executable_files() {
        let tmp = ScopedTempDir::new("is_exec_no");
        let path = tmp.path().join("not_exec");
        fs::write(&path, "").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&path, perms).unwrap();
        assert!(!is_executable(&path));
    }

    #[test]
    fn is_executable_rejects_missing_path() {
        assert!(!is_executable(Path::new("/nonexistent/claude")));
    }

    #[test]
    fn is_executable_rejects_directories() {
        let tmp = ScopedTempDir::new("is_exec_dir");
        assert!(!is_executable(tmp.path()));
    }

    #[test]
    #[cfg(unix)]
    fn is_runnable_accepts_working_executable() {
        let tmp = ScopedTempDir::new("runnable_ok");
        let path = tmp.path().join("claude");
        make_executable(&path);
        assert!(is_runnable(&path));
    }

    #[test]
    #[cfg(unix)]
    fn is_runnable_rejects_executable_that_fails_to_spawn() {
        // Regression: an x86_64-only Homebrew leftover at /usr/local/bin/claude
        // on an arm64 Mac satisfied the POSIX exec bit but the kernel returned
        // ENOEXEC when we tried to run it. The Python `mcp install` then hit
        // the same ENOEXEC via shutil.which, raising an uncaught OSError and
        // surfacing as "Headroom MCP install exited non-zero" in Sentry.
        let tmp = ScopedTempDir::new("runnable_enoexec");
        let path = tmp.path().join("claude");
        fs::write(&path, b"\x00\x01\x02\x03not a binary").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        assert!(!is_runnable(&path));
    }

    #[test]
    #[cfg(unix)]
    fn is_runnable_rejects_executable_that_exits_non_zero() {
        let tmp = ScopedTempDir::new("runnable_exit1");
        let path = tmp.path().join("claude");
        fs::write(&path, "#!/bin/sh\nexit 1\n").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        assert!(!is_runnable(&path));
    }

    #[test]
    #[cfg(unix)]
    fn is_runnable_rejects_non_executable_file() {
        let tmp = ScopedTempDir::new("runnable_no_x");
        let path = tmp.path().join("claude");
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        // Exec bit not set — must short-circuit without spawning.
        assert!(!is_runnable(&path));
    }

    #[test]
    #[cfg(unix)]
    fn first_runnable_walks_past_broken_candidates() {
        // Reproduces the production scenario: an Intel-only `/usr/local/bin/claude`
        // remnant on an arm64 Mac is the second candidate examined; the first
        // candidate (`~/.claude/local/claude`) does not exist; we want detection
        // to skip the broken candidate and find the working one further down.
        let tmp = ScopedTempDir::new("first_runnable_walk");
        let broken = tmp.path().join("usr_local_claude");
        fs::write(&broken, b"\x00\x01\x02\x03not a binary").unwrap();
        let mut perms = fs::metadata(&broken).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&broken, perms).unwrap();

        let working = tmp.path().join("npm_global_claude");
        make_executable(&working);

        let candidates = vec![
            tmp.path().join("does_not_exist"),
            broken.clone(),
            working.clone(),
            tmp.path().join("never_reached"), // would fail if we kept walking
        ];

        assert_eq!(
            first_runnable(candidates.into_iter()).as_deref(),
            Some(working.as_path())
        );
    }

    #[test]
    #[cfg(unix)]
    fn first_runnable_returns_none_when_all_candidates_broken() {
        let tmp = ScopedTempDir::new("first_runnable_none");
        let broken = tmp.path().join("broken");
        fs::write(&broken, b"\x00\x01\x02\x03").unwrap();
        let mut perms = fs::metadata(&broken).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&broken, perms).unwrap();

        let candidates = vec![tmp.path().join("missing"), broken];
        assert!(first_runnable(candidates.into_iter()).is_none());
    }

    #[test]
    fn known_path_candidates_includes_apple_silicon_and_intel_homebrew_in_order() {
        // Apple Silicon Homebrew (/opt/homebrew/bin) must be examined before
        // Intel Homebrew (/usr/local/bin) so that arm64 Macs that ALSO have an
        // Intel-only `claude` left behind in /usr/local/bin reach the working
        // arm64 binary first. The bug we fixed only surfaced because all
        // earlier candidates were missing.
        let candidates = known_path_candidates_for(PathBuf::from("/Users/test"), "claude", true);
        let opt = candidates
            .iter()
            .position(|p| p == Path::new("/opt/homebrew/bin/claude"));
        let usr = candidates
            .iter()
            .position(|p| p == Path::new("/usr/local/bin/claude"));
        assert!(opt.is_some() && usr.is_some());
        assert!(opt.unwrap() < usr.unwrap());
    }

    #[test]
    fn known_candidate_matrix_keeps_unix_paths_off_windows() {
        let home = PathBuf::from("/platform-neutral-home");
        let unix = known_path_candidates_for(home.clone(), "claude", true);
        let windows = known_path_candidates_for(home, "claude", false);

        assert!(!unix.is_empty());
        assert!(unix
            .iter()
            .any(|candidate| candidate == Path::new("/opt/homebrew/bin/claude")));
        assert!(windows.is_empty());
    }

    #[test]
    fn discovery_fallback_platform_matrix_never_selects_a_shell_for_windows() {
        assert_eq!(
            discovery_fallback_for_target(true, false),
            DiscoveryFallback::UnixLoginShell
        );
        assert_eq!(
            discovery_fallback_for_target(false, true),
            DiscoveryFallback::WindowsPath
        );
        assert_eq!(
            discovery_fallback_for_target(false, false),
            DiscoveryFallback::FailClosed
        );
        assert_eq!(
            WINDOWS_PATH_LOOKUP_FAILED_DIAGNOSTIC,
            "cli_discovery_windows_path_lookup_failed_closed"
        );
    }

    #[test]
    fn injected_windows_path_lookup_expands_pathext_without_shell_or_spawn() {
        let tmp = ScopedTempDir::new("windows_pathext_plan");
        let candidate = tmp.path().join("codex.cmd");
        fs::write(&candidate, "not executed").unwrap();

        let detected = crate::client_detection::find_on_path_entries_for_target(
            vec![tmp.path().to_path_buf()],
            &["codex"],
            switchboard_runtime::executable_search::ExecutableSearchPlatform::Windows,
            Some("cmd;.EXE"),
        );

        assert_eq!(detected.as_deref(), Some(candidate.as_path()));
    }

    #[test]
    fn injected_non_windows_path_lookup_does_not_accept_windows_extensions() {
        let tmp = ScopedTempDir::new("unix_ignores_pathext");
        fs::write(tmp.path().join("codex.cmd"), "not executed").unwrap();

        let unix = crate::client_detection::find_on_path_entries_for_target(
            vec![tmp.path().to_path_buf()],
            &["codex"],
            switchboard_runtime::executable_search::ExecutableSearchPlatform::Unix,
            Some(".cmd"),
        );
        let unsupported = crate::client_detection::find_on_path_entries_for_target(
            vec![tmp.path().to_path_buf()],
            &["codex"],
            switchboard_runtime::executable_search::ExecutableSearchPlatform::Unsupported,
            Some(".cmd"),
        );

        assert!(unix.is_none());
        assert!(unsupported.is_none());
    }

    #[test]
    fn unix_shell_selection_defaults_to_zsh_and_uses_fish_login_flags() {
        let (default_shell, default_flags) = unix_login_shell_spec(None);
        assert_eq!(default_shell, Path::new("/bin/zsh"));
        assert_eq!(default_flags, "-ilc");

        let (fish, fish_flags) = unix_login_shell_spec(Some(OsStr::new("/opt/homebrew/bin/fish")));
        assert_eq!(fish, Path::new("/opt/homebrew/bin/fish"));
        assert_eq!(fish_flags, "-lc");
    }

    #[test]
    fn metadata_only_candidate_check_never_requires_a_runnable_binary() {
        let tmp = ScopedTempDir::new("metadata_only_candidate");
        let candidate = tmp.path().join("codex");
        fs::write(&candidate, "not executable").unwrap();

        assert!(has_regular_file_candidate(vec![candidate.clone()]));
        assert!(!is_executable(&candidate));
        assert!(!known_cli_candidate_present_without_start("arbitrary-name"));
    }

    #[test]
    #[serial]
    #[cfg(unix)]
    fn is_runnable_finds_colocated_interpreter_via_augmented_path() {
        // Regression: nvm/volta/bun/asdf installs of `claude` are
        // `#!/usr/bin/env <interp>` scripts with the interpreter colocated in
        // the same bin/. GUI launches inherit launchd's bare PATH, so without
        // augmenting PATH with the candidate's parent, `env` exits 127 and we
        // reject a working `claude`. Simulate by writing a script that shebangs
        // a colocated fake interpreter, then strip PATH so the test inherits
        // nothing useful.
        let tmp = ScopedTempDir::new("runnable_colocated_interp");
        let bin = tmp.path().join("bin");
        fs::create_dir_all(&bin).unwrap();

        let interp = bin.join("fakenode");
        fs::write(&interp, "#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(&interp).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&interp, perms).unwrap();

        let claude = bin.join("claude");
        fs::write(&claude, "#!/usr/bin/env fakenode\n").unwrap();
        let mut perms = fs::metadata(&claude).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&claude, perms).unwrap();

        // Strip PATH so the only way `env fakenode` can resolve is via the
        // augmentation `is_runnable` adds.
        let _path = EnvRestore::set("PATH", "/usr/bin:/bin");
        let result = is_runnable(&claude);
        assert!(result, "is_runnable must augment PATH with the candidate's bin dir so colocated interpreters resolve");
    }

    #[test]
    #[cfg(unix)]
    fn is_runnable_kills_and_rejects_a_hung_executable() {
        // A binary that hangs forever must not stall detection. We override
        // the timeout indirectly by invoking wait_with_timeout directly with
        // a short bound.
        let tmp = ScopedTempDir::new("runnable_hang");
        let path = tmp.path().join("claude");
        fs::write(&path, "#!/bin/sh\nsleep 30\n").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();

        let child = Command::new(&path)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let start = Instant::now();
        let result = wait_with_timeout(child, Duration::from_millis(200));
        let elapsed = start.elapsed();
        assert!(result.is_none());
        assert!(
            elapsed < Duration::from_secs(1),
            "timeout should bound the wait; took {elapsed:?}",
        );
    }

    #[test]
    #[cfg(unix)]
    fn read_path_from_shell_returns_path_before_shell_exits() {
        // Regression: interactive shells print the `command -v claude` output
        // immediately but keep running through `.zshrc`. Previously we waited
        // for the child to exit before reading stdout, so a slow shell init
        // would cause a timeout even when the path was already on the pipe.
        let tmp = ScopedTempDir::new("probe_slow_shell");
        let fake_claude = tmp.path().join("claude");
        make_executable(&fake_claude);
        let claude_str = fake_claude.display().to_string();

        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg(format!("echo {claude_str}; sleep 30"));

        let start = Instant::now();
        let got = read_path_from_shell(cmd, Duration::from_secs(2));
        let elapsed = start.elapsed();

        assert_eq!(got.as_deref(), Some(fake_claude.as_path()));
        assert!(
            elapsed < Duration::from_secs(2),
            "should return as soon as the first line arrives, not wait for the sleep; took {elapsed:?}",
        );
    }

    #[test]
    #[cfg(unix)]
    fn read_path_from_shell_times_out_when_no_output() {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg("sleep 30");

        let start = Instant::now();
        let got = read_path_from_shell(cmd, Duration::from_millis(200));
        let elapsed = start.elapsed();

        assert!(got.is_none());
        assert!(
            elapsed < Duration::from_secs(1),
            "timeout should bound the wait; took {elapsed:?}",
        );
    }

    #[test]
    #[serial]
    #[cfg(windows)]
    fn windows_path_lookup_uses_existing_pathext_resolution() {
        let tmp = ScopedTempDir::new("windows_pathext");
        let candidate = tmp.path().join("codex.cmd");
        fs::write(&candidate, "@exit /b 0\r\n").unwrap();
        let _path = EnvRestore::set("PATH", &tmp.path().display().to_string());
        let _pathext = EnvRestore::set("PATHEXT", ".CMD;.EXE");

        assert_eq!(
            probe_via_platform("codex").as_deref(),
            Some(candidate.as_path())
        );
    }
}
