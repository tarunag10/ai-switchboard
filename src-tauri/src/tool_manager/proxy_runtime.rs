use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::backend_port;
use crate::models::SavingsMode;

pub(super) fn is_local_proxy_reachable() -> bool {
    // Check headroom's actual backend port, not the intercept port (6767),
    // because the intercept starts before headroom and would always be reachable.
    let address: SocketAddr = ([127, 0, 0, 1], backend_port::get()).into();
    TcpStream::connect_timeout(&address, Duration::from_millis(180)).is_ok()
}

pub(super) enum PortState {
    Free,
    HeadroomRunning,
    ForeignOccupant(String),
}

pub(super) fn diagnose_proxy_port(port: u16) -> PortState {
    // If we can bind the port, nothing is there.
    if TcpListener::bind(("127.0.0.1", port)).is_ok() {
        return PortState::Free;
    }

    // Port is held. Probe it: headroom's proxy speaks HTTP and, for an
    // unrecognized path, responds with an HTTP status line. A foreign
    // non-HTTP service (SSH, Redis, etc.) will not.
    let headroom_like = probe_headroom_http(port, Duration::from_millis(400));
    if headroom_like {
        PortState::HeadroomRunning
    } else {
        PortState::ForeignOccupant(lsof_listener(port).unwrap_or_else(|| "unknown process".into()))
    }
}

pub(super) fn repair_console_script_interpreter(entrypoint: &Path, python: &Path) -> Result<()> {
    if !entrypoint.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(entrypoint)
        .with_context(|| format!("reading {}", entrypoint.display()))?;
    let python = python.to_string_lossy();
    let mut changed = false;
    let mut updated_lines = Vec::new();
    for line in content.lines() {
        if line.starts_with("'''exec' \"") && !line.contains(&python[..]) {
            if let Some(rest_start) = line.find("\" \"$0\" \"$@\"") {
                let mut updated = String::from("'''exec' \"");
                updated.push_str(&python);
                updated.push_str(&line[rest_start..]);
                updated_lines.push(updated);
                changed = true;
                continue;
            }
        }
        updated_lines.push(line.to_string());
    }
    if !changed {
        return Ok(());
    }
    let mut updated = updated_lines.join("\n");
    if content.ends_with('\n') {
        updated.push('\n');
    }
    std::fs::write(entrypoint, updated)
        .with_context(|| format!("rewriting stale interpreter in {}", entrypoint.display()))?;
    Ok(())
}

fn probe_headroom_http(port: u16, timeout: Duration) -> bool {
    use std::io::{Read, Write};
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, timeout) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    if stream
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut buf = [0u8; 16];
    match stream.read(&mut buf) {
        Ok(n) if n >= 5 => buf[..5].eq_ignore_ascii_case(b"HTTP/"),
        _ => false,
    }
}

fn lsof_listener(port: u16) -> Option<String> {
    // Only `-iTCP:{port}` — a bare `-iTCP` here would OR with the port
    // selector (lsof ORs `-i` options) and match every listening socket on
    // the machine, so `nth(1)` would return an unrelated daemon's pid.
    let output = Command::new("/usr/sbin/lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let mut fields = line.split_whitespace();
    let cmd = fields.next()?;
    let pid = fields.next()?;
    Some(format!("{cmd} pid {pid}"))
}

/// Extract the numeric pid from a `"cmd pid 1234"` string returned by
/// [`lsof_listener`]. Returns None for the `"unknown process"` placeholder
/// or any unparseable shape. Companion to `port_conflict::parse_occupant`,
/// which works on the full bail string instead of the lsof detail.
pub(super) fn parse_pid_from_lsof_detail(detail: &str) -> Option<u32> {
    let idx = detail.rfind(" pid ")?;
    detail[idx + " pid ".len()..].trim().parse().ok()
}

/// Bail message when a previous (still-alive) headroom proxy holds the port.
/// Extracted as a function so the exact format is testable against
/// `port_conflict::is_port_conflict` and `state::classify_startup_error`.
pub(super) fn format_already_running_bail(port: u16) -> String {
    format!(
        "headroom proxy already running on port {port} (likely a stale process from a prior session). \
         Run `lsof -iTCP:{port} -sTCP:LISTEN` to find and kill it, then retry."
    )
}

/// True when `/readyz` on the backend `port` answers with a 2xx — i.e. a
/// genuinely healthy headroom proxy is serving there. Used to avoid killing a
/// live backend during port reclaim. Short timeout: a hung orphan won't answer
/// in time and a healthy one answers in milliseconds.
pub(super) fn probe_backend_readyz_ok(port: u16) -> bool {
    let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
    else {
        return false;
    };
    matches!(
        client.get(format!("http://127.0.0.1:{port}/readyz")).send(),
        Ok(resp) if resp.status().is_success()
    )
}

/// Poll until `port` is bindable or `timeout` elapses. Returns true once the
/// port is free. A killed listener's socket is released as soon as the owning
/// process dies, so this normally returns within a couple of poll intervals.
pub(super) fn wait_for_port_free(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// Reclaim `port` from an orphaned headroom proxy left behind by a prior
/// session. We only get here from the `HeadroomRunning` spawn pre-flight, which
/// is unreachable when a healthy proxy is up (its `/readyz` would have
/// satisfied `is_headroom_proxy_reachable` and short-circuited
/// `ensure_headroom_running`). Still, re-confirm health on the backend port
/// directly before killing — if it answers 2xx the backend is live (e.g. the
/// 6767 intercept is wedged while 6768 is fine) and we leave it alone. On any
/// failure to reclaim (no pid, refuses to die, healthy) we fall back to the
/// original bail so the caller's classification and user guidance are
/// unchanged.
///
/// `force_unhealthy_too`: during upgrade boot validation the orphan on 6768 is
/// the *old* version we are replacing — a still-healthy old worker (left when
/// `stop_headroom`'s argv pattern-kill missed the real socket holder) must be
/// killed anyway, or the new venv can't bind and the upgrade rolls back as
/// `not_started`. When set, skip the readyz health guard and reclaim regardless.
pub(super) fn reclaim_orphan_proxy(port: u16, force_unhealthy_too: bool) -> Result<()> {
    if !force_unhealthy_too && probe_backend_readyz_ok(port) {
        bail!("{}", format_already_running_bail(port));
    }
    let Some(pid) = lsof_listener(port)
        .as_deref()
        .and_then(parse_pid_from_lsof_detail)
    else {
        bail!("{}", format_already_running_bail(port));
    };

    log::warn!("[backend_port] reclaiming orphaned headroom proxy pid {pid} on port {port}");
    let _ = Command::new("/bin/kill").arg(pid.to_string()).status();
    if !wait_for_port_free(port, Duration::from_secs(3)) {
        let _ = Command::new("/bin/kill")
            .arg("-KILL")
            .arg(pid.to_string())
            .status();
        if !wait_for_port_free(port, Duration::from_secs(2)) {
            bail!("{}", format_already_running_bail(port));
        }
    }

    sentry::with_scope(
        |scope| {
            scope.set_tag("flow", "orphan_proxy_reclaimed");
            scope.set_extra("port", port.into());
            scope.set_extra("occupant_pid", pid.into());
        },
        || {
            sentry::capture_message(
                &format!("orphan_proxy_reclaimed: killed pid {pid} holding port {port}"),
                sentry::Level::Info,
            );
        },
    );
    Ok(())
}

/// Bail message when 6768 is foreign-held AND every port in the fallback
/// range is also taken. Must contain `"is occupied by a non-headroom process"`
/// so `port_conflict::is_port_conflict` continues to match, and the
/// `(occupant)` parenthetical so `port_conflict::parse_occupant` can extract
/// the cmd/pid for the persistent-conflict marker.
pub(super) fn format_all_foreign_bail(
    default_port: u16,
    occupant: &str,
    range: (u16, u16),
) -> String {
    let (start, end) = range;
    format!(
        "port {default_port} is occupied by a non-headroom process ({occupant}) and fallback ports {start}-{end} are also unavailable; cannot start proxy. \
         Reboot to clear stuck listeners, then relaunch Mac AI Switchboard."
    )
}

pub(crate) fn tail_log_file(path: &Path, max_lines: usize) -> String {
    let Ok(file) = std::fs::File::open(path) else {
        return String::new();
    };
    let mut lines: std::collections::VecDeque<String> =
        std::collections::VecDeque::with_capacity(max_lines);
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if lines.len() == max_lines {
            lines.pop_front();
        }
        lines.push_back(redact_sensitive(&line));
    }
    lines.into_iter().collect::<Vec<_>>().join("\n")
}

/// Strip Anthropic API keys and bearer tokens from log content before it gets
/// handed to Sentry. Without this, Sentry's default PII scrubber sees one
/// `sk-ant-…` and replaces the entire `proxy_log_tail` field with `[Filtered]`,
/// which is the single most diagnostic field in `proxy_unreachable_post_boot`.
/// Pre-redact so the rest of the line survives the scrubber.
pub(super) fn redact_sensitive(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &line[i..];
        if let Some(consumed) = match_redactable(rest) {
            out.push_str("[REDACTED]");
            i += consumed;
        } else {
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// If `rest` starts with a redactable token, return the byte length to skip.
fn match_redactable(rest: &str) -> Option<usize> {
    if let Some(after) = rest.strip_prefix("sk-ant-") {
        let token_len = after
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
            .count();
        return Some("sk-ant-".len() + token_len);
    }
    for prefix in ["Bearer ", "bearer "] {
        if let Some(after) = rest.strip_prefix(prefix) {
            let token_len = after
                .bytes()
                .take_while(|b| {
                    b.is_ascii_alphanumeric()
                        || matches!(*b, b'-' | b'_' | b'.' | b'~' | b'+' | b'/' | b'=')
                })
                .count();
            if token_len >= 8 {
                return Some(prefix.len() + token_len);
            }
        }
    }
    None
}

/// Newest `headroom-proxy*.log` in the logs directory, if any.
pub(crate) fn newest_proxy_log_path(logs_dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(logs_dir).ok()?;
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("headroom-proxy") || !name_str.ends_with(".log") {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                let path = entry.path();
                newest = Some(match newest {
                    Some((prev_time, prev_path)) if prev_time > mtime => (prev_time, prev_path),
                    _ => (mtime, path),
                });
            }
        }
    }
    newest.map(|(_, p)| p)
}

pub(super) fn headroom_python_startup_args() -> Vec<String> {
    // The `python -m headroom.proxy.server` argparse does NOT define the learn
    // flags (--learn, --no-memory-tools, --no-memory-context, --memory-db-path);
    // those live only on the `headroom proxy` click entrypoint. Passing them
    // here makes argparse exit 2, so the fallback would always fail and mask
    // the real entrypoint failure under spurious noise. Keep this variant to
    // server-supported flags only.
    let mut args = vec![
        "-m".to_string(),
        "headroom.proxy.server".to_string(),
        "--port".to_string(),
        headroom_proxy_port(),
        "--no-http2".to_string(),
    ];
    if crate::message_logging::full_message_logging_active() {
        args.push("--log-messages".to_string());
    }
    args
}

pub(super) fn headroom_entrypoint_startup_args(savings_mode: &SavingsMode) -> Vec<String> {
    // HTTP/2 on the entrypoint is controlled via the HEADROOM_HTTP2 env var
    // (set to "false" in the spawn env). Older bundled runtimes ignored this var
    // and ran HTTP/2 unconditionally, which surfaced as SSLV3_ALERT_BAD_RECORD_MAC
    // under multi-tab concurrency; the runtime bundled with this build honors it.
    let mut args = vec![
        "proxy".to_string(),
        "--port".to_string(),
        headroom_proxy_port(),
    ];
    if crate::message_logging::full_message_logging_active() {
        args.push("--log-messages".to_string());
    }
    if matches!(savings_mode, SavingsMode::Aggressive) {
        args.push("--intercept-tool-results".to_string());
    }
    args.extend(headroom_learn_startup_args());
    args
}

fn headroom_proxy_port() -> String {
    backend_port::get().to_string()
}

pub(super) fn apply_savings_mode_env(command: &mut Command, savings_mode: &SavingsMode) {
    command.env(
        "HEADROOM_SAVINGS_PROFILE",
        match savings_mode {
            SavingsMode::Balanced => "balanced",
            SavingsMode::Aggressive => "aggressive",
        },
    );
    if matches!(savings_mode, SavingsMode::Aggressive) {
        command
            .env("HEADROOM_TARGET_RATIO", "0.45")
            .env("HEADROOM_MIN_TOKENS", "120")
            .env("HEADROOM_SMART_CRUSHER_COMPACTION", "1")
            .env("HEADROOM_CODE_AWARE_ENABLED", "1")
            .env("HEADROOM_PROTECT_RECENT", "2");
    }
}

/// Flags whose presence in the running proxy's argv we treat as proof that it
/// was started by this build. If any of these are missing, the proxy was
/// spawned by an older desktop (or by something else) and we restart it.
#[allow(dead_code)]
fn expected_proxy_arg_signature() -> Vec<&'static str> {
    expected_proxy_arg_signature_for(&crate::client_adapters::load_savings_mode())
}

fn expected_proxy_arg_signature_for(savings_mode: &SavingsMode) -> Vec<&'static str> {
    let mut flags = vec![
        "--port",
        "--learn",
        "--no-memory-tools",
        "--no-memory-context",
        "--memory-db-path",
    ];
    if crate::message_logging::full_message_logging_active() {
        flags.push("--log-messages");
    }
    if matches!(savings_mode, SavingsMode::Aggressive) {
        flags.push("--intercept-tool-results");
    }
    flags
}

/// Returns the full command line of whatever process is currently listening on
/// the proxy port, or `None` if we couldn't determine it.
pub fn running_proxy_argv() -> Option<String> {
    let pid = lsof_listener_pid(backend_port::get())?;
    ps_command(pid)
}

/// True if the running proxy's argv contains every flag we expect this build
/// to pass. Used to detect proxies left over from an older desktop version.
pub fn running_proxy_matches_expected_args() -> bool {
    let Some(argv) = running_proxy_argv() else {
        return false;
    };
    proxy_argv_contains_expected_flags(&argv)
}

fn proxy_argv_contains_expected_flags(argv: &str) -> bool {
    proxy_argv_contains_expected_flags_for(argv, &crate::client_adapters::load_savings_mode())
}

pub(super) fn proxy_argv_contains_expected_flags_for(
    argv: &str,
    savings_mode: &SavingsMode,
) -> bool {
    expected_proxy_arg_signature_for(savings_mode)
        .iter()
        .all(|flag| argv_contains_flag(argv, flag))
}

/// Whitespace-aware containment check so `--port` doesn't match `--port-foo`
/// and `--learn` doesn't match `--no-learn`.
fn argv_contains_flag(argv: &str, flag: &str) -> bool {
    argv.split_whitespace().any(|tok| tok == flag)
}

fn lsof_listener_pid(port: u16) -> Option<u32> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-Fp"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| line.strip_prefix('p').and_then(|n| n.trim().parse().ok()))
}

fn ps_command(pid: u32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// If `log_tail` shows pydantic refusing to import because the installed
/// `pydantic-core` doesn't match what the bundled pydantic wants, return the
/// version pydantic wants. The error message is the source of truth — pydantic
/// prints the exact pinned version it expects.
///
/// Example line we match:
///     SystemError: The installed pydantic-core version (2.46.3) is
///     incompatible with the current pydantic version, which requires 2.41.5.
pub(super) fn extract_required_pydantic_core_version(log_tail: &str) -> Option<String> {
    if !log_tail.contains("pydantic-core") {
        return None;
    }
    let marker = "which requires ";
    let idx = log_tail.find(marker)?;
    let after = &log_tail[idx + marker.len()..];
    let version: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let trimmed = version.trim_end_matches('.');
    if trimmed.is_empty() || !trimmed.contains('.') {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Make a string safe to use as part of a filename: replace path separators
/// (`/`, `\`) and other characters that have meaning to the filesystem with
/// `_`, then truncate so absurdly long argv strings don't blow past
/// per-component name limits (255 bytes on most filesystems).
pub(super) fn sanitize_log_variant(raw: &str) -> String {
    const MAX_LEN: usize = 80;
    let mut out: String = raw
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '\0' | '\n' | '\r' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    if out.len() > MAX_LEN {
        out.truncate(MAX_LEN);
    }
    out
}

/// Args that enable passive learning: the proxy extracts patterns from live
/// traffic into the memory store, but does not inject memory tools or context
/// into requests (so the model's view of the conversation is unchanged).
fn headroom_learn_startup_args() -> Vec<String> {
    vec![
        "--learn".to_string(),
        "--no-memory-tools".to_string(),
        "--no-memory-context".to_string(),
        "--memory-db-path".to_string(),
        crate::headroom_memory_db_path().display().to_string(),
    ]
}
