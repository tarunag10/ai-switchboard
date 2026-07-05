use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::models::{BootstrapProgress, RuntimeStatus, RuntimeUpgradeProgress};
use crate::state::AppState;
use crate::{analytics, client_adapters, process_runner, switchboard_commands};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum BootstrapFailureKind {
    /// Corporate proxy / AV / VPN injecting a self-signed root, so pip can't
    /// verify pypi.org or github.com. Not our bug, but users here are stuck
    /// until they configure `REQUESTS_CA_BUNDLE` or disable TLS inspection.
    SslInterception,
    /// Python's `tempfile` couldn't create a directory in any candidate
    /// location (TMPDIR, /tmp, /var/tmp, /usr/tmp, cwd). Disk full, TCC
    /// blocking writes, or a stale macOS per-user temp dir. Not our bug,
    /// but the default "couldn't download a file" message is misleading
    /// because pip never even got to the network.
    NoUsableTempDir,
    /// Transient network/download problem: the server returned a 5xx (e.g.
    /// GitHub's 504 Gateway Time-out on a release asset), the connection was
    /// reset, DNS failed, or a request timed out. Not our bug and not the
    /// user's environment - it's self-recoverable, so we frame it softly and
    /// the user just needs to click Try again.
    NetworkDownload,
    Other,
}

impl BootstrapFailureKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            BootstrapFailureKind::SslInterception => "ssl_interception",
            BootstrapFailureKind::NoUsableTempDir => "no_usable_tempdir",
            BootstrapFailureKind::NetworkDownload => "network_download",
            BootstrapFailureKind::Other => "other",
        }
    }
}

#[tauri::command]
pub fn bootstrap_runtime(
    state: State<'_, AppState>,
) -> Result<crate::models::DashboardState, String> {
    state
        .tool_manager
        .bootstrap_all()
        .map_err(|err| err.to_string())?;

    if switchboard_commands::saved_switchboard_mode_wants_rtk() {
        if let Err(err) = client_adapters::ensure_rtk_integrations(
            &state.tool_manager.rtk_entrypoint(),
            &state.tool_manager.managed_python(),
        ) {
            log::warn!("RTK integrations failed after bootstrap_runtime: {err:#}");
        }
    }

    if !switchboard_commands::saved_switchboard_mode_wants_headroom() {
        state.stop_headroom();
        state.set_runtime_paused(true);
        state.set_runtime_auto_paused(false);
        return Ok(state.dashboard());
    }

    state
        .ensure_headroom_running()
        .map_err(|err| format!("bootstrap complete but failed to start headroom: {err}"))?;

    Ok(state.dashboard())
}

pub(crate) fn classify_bootstrap_failure(err: &anyhow::Error) -> BootstrapFailureKind {
    // pip/venv failures surface as CommandFailure, where stdout/stderr carry the
    // real signal. Our own reqwest downloads (Python runtime, rtk binary) have no
    // CommandFailure, so fall back to the formatted error chain for those.
    let cmd_failure = err
        .chain()
        .find_map(|e| e.downcast_ref::<process_runner::CommandFailure>());
    let haystack = match cmd_failure {
        Some(failure) => format!("{}\n{}", failure.stdout, failure.stderr),
        None => format!("{err:#}"),
    };

    if haystack.contains("CERTIFICATE_VERIFY_FAILED")
        || haystack.contains("self-signed certificate in certificate chain")
        || haystack.contains("self signed certificate in certificate chain")
    {
        BootstrapFailureKind::SslInterception
    } else if haystack.contains("No usable temporary directory found") {
        BootstrapFailureKind::NoUsableTempDir
    } else if is_network_download_signal(&haystack) {
        BootstrapFailureKind::NetworkDownload
    } else {
        BootstrapFailureKind::Other
    }
}

/// True when a bootstrap failure looks like a transient network/download
/// problem (server 5xx, connection reset, DNS failure, request timeout) rather
/// than a configuration or environment fault. These are self-recoverable: the
/// user just needs to retry, so we frame them softly and report them to Sentry
/// as warnings instead of errors.
pub(crate) fn is_network_download_signal(text: &str) -> bool {
    // Signatures from reqwest (`error_for_status`, transport errors) and curl/pip
    // network failures. Lowercased once; keep entries lowercase.
    const SIGNALS: &[&str] = &[
        "http status server error", // reqwest error_for_status on any 5xx
        "gateway time-out",         // 502/504 from GitHub's edge
        "bad gateway",
        "service unavailable",
        "error sending request",
        "operation timed out",
        "connection timed out",
        "timed out",
        "connection refused",
        "connection reset",
        "connection closed",
        "tcp connect error",
        "dns error",
        "failed to lookup address",
        "could not resolve host",
        "network is unreachable",
        "temporary failure in name resolution",
    ];
    let lower = text.to_ascii_lowercase();
    SIGNALS.iter().any(|signal| lower.contains(signal))
}

pub(crate) fn user_message_for(kind: BootstrapFailureKind) -> &'static str {
    match kind {
        BootstrapFailureKind::SslInterception => {
            "Installation failed: your network is intercepting secure connections \
             (self-signed certificate in the TLS chain), so Headroom can't verify \
             pypi.org or github.com. This usually means a corporate proxy, VPN, or \
             antivirus is inspecting HTTPS traffic. Set the REQUESTS_CA_BUNDLE \
             environment variable to your organization's CA bundle, or disable TLS \
             inspection for pypi.org, files.pythonhosted.org, and github.com, then \
             restart the app. Open a GitHub Issue from Support if you need help."
        }
        BootstrapFailureKind::NoUsableTempDir => {
            "Installation failed: Headroom can't create temporary files on this Mac. \
             This usually means your disk is full, or security software (like an MDM \
             profile or endpoint protection) is blocking writes to /tmp and \
             /var/folders. Free up disk space, restart your Mac, and try again. \
             If it still fails, open a GitHub Issue from Support."
        }
        BootstrapFailureKind::NetworkDownload => {
            "Couldn't reach the download server. This is usually a temporary \
             network or server hiccup, not a problem with your Mac. Check your \
             internet connection and click Try again. If it keeps failing, a \
             firewall, VPN, or corporate proxy may be blocking pypi.org and \
             files.pythonhosted.org - try another network or contact \
             the Support page."
        }
        BootstrapFailureKind::Other => {
            "Installation failed: Headroom couldn't download a required file. \
             Check your internet connection, then click Try again. \
             If this keeps happening, open a GitHub Issue from Support."
        }
    }
}

/// Report a bootstrap failure to Sentry. If the error chain contains a
/// `CommandFailure`, its full stdout/stderr/exit_code are sent as structured
/// `extra` fields (which Sentry does NOT truncate at the 8KB message cap),
/// so we can actually see why pip/venv failed on the user's machine.
pub(crate) fn capture_bootstrap_failure(err: &anyhow::Error, kind: BootstrapFailureKind) {
    let technical_err = format!("{err:#}");
    let cmd_failure = err
        .chain()
        .find_map(|e| e.downcast_ref::<process_runner::CommandFailure>());

    // Match against stderr (where the real signal lives for CommandFailure)
    // in addition to the error chain. For non-CommandFailure paths the
    // chain is all we have.
    let endpoint_protection_suspected = crate::is_endpoint_protection_signal(&technical_err)
        || cmd_failure
            .map(|f| crate::is_endpoint_protection_signal(&f.stderr))
            .unwrap_or(false);

    // ENOSPC is environmental; skip the Sentry capture (see notes on
    // `capture_upgrade_failure`).
    let disk_full = crate::is_disk_full_signal(&technical_err)
        || cmd_failure
            .map(|f| crate::is_disk_full_signal(&f.stderr))
            .unwrap_or(false);
    if disk_full {
        log::warn!(
            "skipping Sentry capture for bootstrap_failed ({}): disk full (ENOSPC)",
            kind.as_str()
        );
        return;
    }

    // Transient network/download failures are self-recoverable via the retry
    // button; report them as warnings so they don't pollute the error feed.
    let level = match kind {
        BootstrapFailureKind::NetworkDownload => sentry::Level::Warning,
        _ => sentry::Level::Error,
    };

    if let Some(failure) = cmd_failure {
        sentry::with_scope(
            |scope| {
                scope.set_tag("failure_kind", kind.as_str());
                scope.set_tag(
                    "endpoint_protection_suspected",
                    if endpoint_protection_suspected {
                        "true"
                    } else {
                        "false"
                    },
                );
                scope.set_extra("program", failure.program.clone().into());
                scope.set_extra("args", failure.args.join(" ").into());
                scope.set_extra(
                    "exit_code",
                    failure
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "signal".into())
                        .into(),
                );
                scope.set_extra(
                    "signal",
                    failure
                        .signal
                        .map(|s| s.to_string().into())
                        .unwrap_or(serde_json::Value::Null),
                );
                scope.set_extra("stdout", failure.stdout.clone().into());
                scope.set_extra("stderr", failure.stderr.clone().into());
                scope.set_extra("error_chain", technical_err.clone().into());
            },
            || {
                sentry::capture_message("bootstrap_failed (install_runtime)", level);
            },
        );
    } else {
        sentry::with_scope(
            |scope| {
                scope.set_tag("failure_kind", kind.as_str());
                scope.set_tag(
                    "endpoint_protection_suspected",
                    if endpoint_protection_suspected {
                        "true"
                    } else {
                        "false"
                    },
                );
                scope.set_extra("error_chain", technical_err.clone().into());
            },
            || {
                sentry::capture_message(
                    &format!("bootstrap_failed (install_runtime): {technical_err}"),
                    level,
                );
            },
        );
    }
}

/// Pure payload for watchdog give-up capture. Built before any Sentry side
/// effects so it can be unit-tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchdogGiveUpReport {
    pub message: String,
    pub tracked_child_exit_status: String,
    pub bypass_active: bool,
    pub runtime_upgrade_in_progress: bool,
    pub consecutive_failures: u32,
    pub log_tail: Option<String>,
    pub last_startup_error: Option<String>,
    pub tracked_pid: Option<u32>,
    pub port_accepts_tcp: bool,
    pub process_cpu_secs: Option<u64>,
    pub log_silent_secs: Option<u64>,
    pub backend_readyz_outcome: String,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_watchdog_give_up_report(
    consecutive_failures: u32,
    bypass_active: bool,
    runtime_upgrade_in_progress: bool,
    exit_status: Option<String>,
    log_tail: Option<String>,
    last_startup_error: Option<String>,
    tracked_pid: Option<u32>,
    port_accepts_tcp: bool,
    process_cpu_secs: Option<u64>,
    log_silent_secs: Option<u64>,
    backend_readyz_outcome: String,
) -> WatchdogGiveUpReport {
    WatchdogGiveUpReport {
        message: format!(
            "proxy_unreachable_post_boot (auto_paused after {consecutive_failures} failures)"
        ),
        tracked_child_exit_status: exit_status
            .unwrap_or_else(|| "still_alive_or_untracked".to_string()),
        bypass_active,
        runtime_upgrade_in_progress,
        consecutive_failures,
        log_tail: log_tail.filter(|s| !s.is_empty()),
        last_startup_error: last_startup_error.filter(|s| !s.is_empty()),
        tracked_pid,
        port_accepts_tcp,
        process_cpu_secs,
        log_silent_secs,
        backend_readyz_outcome,
    }
}

/// Probe `/readyz` on the backend port directly (bypassing the Rust intercept
/// on 6767) and classify the outcome for inclusion in watchdog decisions.
pub(crate) fn probe_backend_readyz_outcome() -> String {
    probe_backend_readyz_outcome_with_timeout(std::time::Duration::from_millis(1500))
}

/// Same probe as [`probe_backend_readyz_outcome`] but with a caller-chosen
/// timeout. The watchdog uses a longer budget to confirm a failure before
/// counting a strike.
pub(crate) fn probe_backend_readyz_outcome_with_timeout(timeout: std::time::Duration) -> String {
    let port = crate::backend_port::get();
    let client = match reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
    {
        Ok(c) => c,
        Err(err) => return format!("error: {err}"),
    };
    let url = format!("http://127.0.0.1:{port}/readyz");
    match client.get(&url).send() {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                "ok".to_string()
            } else if status.as_u16() == 503 {
                match response.text() {
                    Ok(body) => match serde_json::from_str::<serde_json::Value>(&body) {
                        Ok(json) => {
                            let csv = readyz_failed_checks_csv(&json);
                            if csv.is_empty() {
                                "http_503".to_string()
                            } else {
                                format!("http_503:{csv}")
                            }
                        }
                        Err(_) => "http_503".to_string(),
                    },
                    Err(_) => "http_503".to_string(),
                }
            } else {
                format!("http_{}", status.as_u16())
            }
        }
        Err(err) => {
            if err.is_timeout() {
                "timeout".to_string()
            } else if err.is_connect() {
                "refused".to_string()
            } else {
                format!("error: {err}")
            }
        }
    }
}

/// Comma-joined, sorted names of the unhealthy components in a `/readyz`
/// payload. Empty when the body has no `checks` object or every check is ready.
pub(crate) fn readyz_failed_checks_csv(body: &serde_json::Value) -> String {
    let Some(checks) = body.get("checks").and_then(|c| c.as_object()) else {
        return String::new();
    };
    let mut failed: Vec<&str> = checks
        .iter()
        .filter(|(_, v)| v.get("ready").and_then(|r| r.as_bool()) == Some(false))
        .map(|(name, _)| name.as_str())
        .collect();
    failed.sort_unstable();
    failed.join(",")
}

fn parse_readyz_failed_checks(outcome: &str) -> Option<Vec<&str>> {
    outcome
        .strip_prefix("http_503:")
        .map(|rest| rest.split(',').filter(|s| !s.is_empty()).collect())
}

pub(crate) fn readyz_failure_is_upstream_only(outcome: &str) -> bool {
    matches!(parse_readyz_failed_checks(outcome), Some(checks) if checks == ["upstream"])
}

pub(crate) fn readyz_failure_has_core_unhealthy(outcome: &str) -> bool {
    parse_readyz_failed_checks(outcome)
        .map(|checks| checks.iter().any(|c| *c != "upstream"))
        .unwrap_or(false)
}

/// Whether two cumulative CPU samples (`ps -o time=`, whole seconds) taken
/// `elapsed_secs` apart represent a process actively burning CPU.
pub(crate) fn cpu_rate_indicates_burn(before: u64, after: u64, elapsed_secs: f64) -> bool {
    elapsed_secs > 0.0 && (after.saturating_sub(before) as f64) / elapsed_secs > 0.5
}

/// All inputs must be in their ready state for the proxy to be supposed-up.
pub(crate) fn watchdog_should_be_up(
    installed: bool,
    paused: bool,
    starting: bool,
    upgrading: bool,
    bypass: bool,
) -> bool {
    installed && !paused && !starting && !upgrading && !bypass
}

/// Backoff schedule for the self-heal auto-resume loop after watchdog give-up.
pub(crate) fn auto_resume_backoff(failed_attempts: u32) -> std::time::Duration {
    let secs = match failed_attempts {
        0 => 30,
        1 => 60,
        2 => 120,
        _ => 300,
    };
    std::time::Duration::from_secs(secs)
}

#[tauri::command]
pub fn get_bootstrap_progress(state: State<'_, AppState>) -> BootstrapProgress {
    state.bootstrap_progress()
}

#[tauri::command]
pub fn get_runtime_upgrade_progress(state: State<'_, AppState>) -> RuntimeUpgradeProgress {
    state.runtime_upgrade_progress()
}

#[tauri::command]
pub fn retry_runtime_upgrade(app: AppHandle) -> Result<(), String> {
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app_clone.state();
        state.retry_runtime_upgrade(&app_clone, false);
    });
    Ok(())
}

/// User-initiated recovery path. Same flow as `retry_runtime_upgrade` but
/// skips the in-place upgrade attempt and goes straight to atomic rebuild.
/// Surfaced as the "Retry with full rebuild" button on a boot-validation
/// failure: the in-place pip succeeded (smoke test passed) but the proxy
/// never booted, which usually means stale native libs from the previous
/// pin survived the upgrade. The rebuild path nukes the venv and starts
/// fresh, fixing the broken state at the cost of re-downloading wheels.
#[tauri::command]
pub fn retry_runtime_upgrade_with_rebuild(app: AppHandle) -> Result<(), String> {
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app_clone.state();
        state.retry_runtime_upgrade(&app_clone, true);
    });
    Ok(())
}

#[tauri::command]
pub fn dismiss_runtime_upgrade_failure(state: State<'_, AppState>) -> Result<(), String> {
    state.dismiss_upgrade_failure();
    Ok(())
}

fn emit_bootstrap_progress(app: &AppHandle, state: &AppState) {
    let _ = app.emit("bootstrap_progress", state.bootstrap_progress());
}

#[tauri::command]
pub(crate) fn start_bootstrap(app: AppHandle) -> Result<(), String> {
    let already_installed = {
        let state: tauri::State<'_, AppState> = app.state();
        let already_installed = state.tool_manager.python_runtime_installed();
        state.begin_bootstrap()?;
        emit_bootstrap_progress(&app, &state);
        already_installed
    };

    if already_installed {
        analytics::track_event(
            &app,
            "bootstrap_skipped",
            Some(json!({ "reason": "already_installed" })),
        );
    } else {
        analytics::track_event(&app, "bootstrap_started", None);
    }

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app_handle.state();
        let wants_headroom = switchboard_commands::saved_switchboard_mode_wants_headroom();

        if !already_installed {
            let result = state.tool_manager.bootstrap_all_with_progress(|step| {
                state.update_bootstrap_step(step);
                emit_bootstrap_progress(&app_handle, &state);
            });
            if let Err(err) = result {
                let kind = classify_bootstrap_failure(&err);
                capture_bootstrap_failure(&err, kind);
                state.mark_bootstrap_failed(user_message_for(kind));
                emit_bootstrap_progress(&app_handle, &state);
                analytics::track_event(
                    &app_handle,
                    "bootstrap_failed",
                    Some(json!({ "phase": "install_runtime", "kind": kind.as_str() })),
                );
                return;
            }

            if switchboard_commands::saved_switchboard_mode_wants_rtk() {
                if let Err(err) = client_adapters::ensure_rtk_integrations(
                    &state.tool_manager.rtk_entrypoint(),
                    &state.tool_manager.managed_python(),
                ) {
                    log::warn!("RTK integrations failed after start_bootstrap thread: {err:#}");
                }
            }
        }

        if !wants_headroom {
            state.stop_headroom();
            state.set_runtime_paused(true);
            state.set_runtime_auto_paused(false);
            state.mark_bootstrap_complete();
            emit_bootstrap_progress(&app_handle, &state);
            analytics::track_event(
                &app_handle,
                "bootstrap_completed",
                Some(json!({ "headroom_started": false })),
            );
            return;
        }

        // Show "Starting Headroom" in the install loader while we wait for the
        // proxy to come up. This runs for both fresh installs and already-installed
        // re-runs. On a fresh machine macOS Gatekeeper scans the entire venv on
        // first execution (30-60s); keeping `complete: false` here means the user
        // cannot click Continue until the proxy is actually reachable.
        state.mark_bootstrap_proxy_starting();
        emit_bootstrap_progress(&app_handle, &state);

        // Hold `runtime_starting = true` for the entire spawn + wait window so
        // the tray spinner and UI share a single source of truth for "headroom
        // is booting but not yet serving". `ensure_headroom_running` toggles
        // this flag internally, but flips it back to false the instant
        // `start_headroom_background()` returns (process spawn only, not
        // readiness) — so we re-assert it here, *after* that call, and clear
        // it only once the proxy is reachable (or we time out). This mirrors
        // `warm_runtime_on_launch`.
        // Seed the output-shaper savings baseline BEFORE starting the proxy
        // (runtime is installed by this point). The proxy's recorder loads the
        // baseline once at boot and clobbers a later write on flush, so seeding
        // first is what lets the dashboard estimate appear without an app
        // relaunch. Idempotent and bounded; we are on the bootstrap thread, so
        // the one-time scan does not block the UI.
        state.tool_manager.seed_verbosity_baseline_if_needed();

        let ensure_result = state.ensure_headroom_running();
        state.set_runtime_starting(true);

        if let Err(err) = ensure_result {
            log::debug!("headroom auto-start failed after bootstrap: {err}");
            // Bootstrap finishes and immediately tries to start the proxy;
            // a port conflict here counts as a "fresh launch" stuck case.
            let handled = crate::port_conflict::note_proxy_failed(&app_handle, &err, true);
            if !handled {
                crate::capture_headroom_start_failure(
                    "headroom auto-start failed after bootstrap",
                    &err,
                );
            }
            // Fall through so the user is not stuck on the install loader
            // indefinitely. The test screen will show a retry option.
        } else {
            crate::port_conflict::note_proxy_started(&app_handle);
            // The intercept layer on 6767 is always bound by the Rust app, so
            // reachability really means "headroom's backend on 6768 is up".
            // We probe it by hitting 6767/health — the intercept forwards to
            // 6768 and returns 502 until the backend actually responds, so a
            // 2xx confirms the full chain is live. Gatekeeper's first-launch
            // scan of the bundled venv can take 30-60s, so we wait up to 60s
            // to match the ETA shown to the user.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while std::time::Instant::now() < deadline {
                if crate::state::headroom_proxy_reachable() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }

        state.set_runtime_starting(false);
        state.mark_bootstrap_complete();
        emit_bootstrap_progress(&app_handle, &state);
        analytics::track_event(&app_handle, "bootstrap_completed", None);
    });

    Ok(())
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, AppState>) -> RuntimeStatus {
    state.runtime_status()
}
