mod activity_commands;
mod activity_facts;
mod addon_commands;
mod analytics;
mod app_services_commands;
mod app_update_commands;
mod backend_port;
mod bearer;
mod claude_cli;
mod claude_sessions;
mod cli_discovery;
mod client_adapters;
mod client_cleanup;
mod client_connector_status;
mod client_connectors;
mod client_footprint;
mod client_paths;
mod client_setup_commands;
mod codex_threads;
mod connector_smoke;
mod dashboard_commands;
mod dedicated_cleanup_rollback;
mod device;
mod doctor;
mod external_open;
mod insights;
mod keychain;
mod learning_commands;
mod local_mode;
mod logging;
mod managed_files;
mod memory_scrubber;
mod message_logging;
mod message_settings_commands;
mod models;
mod optimization;
mod optimization_commands;
mod port_conflict;
mod pricing;
mod pricing_commands;
mod process_runner;
mod proxy_intercept;
mod release_evidence;
mod repo_intelligence;
mod repo_intelligence_commands;
mod repo_map;
mod repo_memory_commands;
mod rollback_commands;
mod runtime_commands;
mod runtime_diagnostics;
mod runtime_distribution;
mod runtime_probe;
mod state;
mod storage;
mod switchboard_commands;
mod tool_manager;
mod tray_runtime;
mod tray_window;

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

use chrono::Local;
use serde_json::{json, Value};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;
use tauri::{AppHandle, Rect, Window, WindowEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::state::AppState;

const SENTRY_DSN: Option<&str> = option_env!("HEADROOM_SENTRY_DSN");
const AUTOSTART_LAUNCH_ARG: &str = "--autostart";
const MAIN_WINDOW_BLUR_HIDE_DELAY_MS: u64 = 150;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuitSource {
    SettingsButton,
    TrayMenu,
}

impl QuitSource {
    fn label(self) -> &'static str {
        match self {
            Self::SettingsButton => "settings_button",
            Self::TrayMenu => "tray_menu",
        }
    }
}

// Set when the watchdog has captured a Sentry event for the current "down
// episode". Reset whenever the proxy is observed reachable again, so a
// subsequent crash re-fires.
static WATCHDOG_DOWN_CAPTURED: AtomicBool = AtomicBool::new(false);

// Set after the first port-conflict start failure has been captured this
// session. Subsequent in-session port conflicts stay silent so the dashboard
// doesn't drown in the sleep/wake / kill -9 race noise.
static PORT_CONFLICT_CAPTURED: AtomicBool = AtomicBool::new(false);

// Guards the quit-time `clear_client_setups()` so it runs at most once per
// process. The exit handler fires for both `ExitRequested` and `Exit`, and a
// second `clear_client_setups()` call is destructive: its `disable_client_setup`
// loop wipes `remembered_clients` and then skips the snapshot re-save because
// `configured_clients` is already empty, leaving nothing for the next launch's
// `restore_client_setups()` to bring back.
static EXIT_CLEAR_DONE: AtomicBool = AtomicBool::new(false);

/// Best-effort: schedule the running `.app` bundle to be moved to the user's
/// Trash once this process exits. Returns the bundle path that was scheduled,
/// or `None` if there is no enclosing bundle, it is App-Translocated, or the
/// detached helper could not be spawned.
///
/// We can't delete our own running bundle inline, so we spawn a detached shell
/// that waits for our PID to exit (mirroring the `restart_app` relauncher) and
/// then `mv`s the bundle into `~/.Trash`. `mv` is used rather than a Finder
/// "delete" because by the time it runs the app is gone and could not answer a
/// Finder automation (TCC) prompt; moving into `~/.Trash` needs no such
/// permission and keeps the uninstall recoverable.
#[cfg(target_os = "macos")]
fn schedule_app_bundle_trash() -> Option<std::path::PathBuf> {
    let bundle = app_update_commands::current_app_bundle_path()?;

    // App Translocation: the app was launched quarantined (e.g. straight from a
    // DMG, never moved to /Applications) and runs from a randomized read-only
    // copy under `.../AppTranslocation/...`. Trashing that copy does nothing
    // useful and leaves the real install in place, so skip it.
    if bundle.to_string_lossy().contains("/AppTranslocation/") {
        log::warn!(
            "uninstall: skipping app-bundle removal; running from translocated path {bundle:?}"
        );
        return None;
    }

    let pid = std::process::id();
    let quoted = app_update_commands::shell_quote_path(&bundle);
    let log_quoted = app_update_commands::shell_quote_path(&logging::log_path());
    let cmd = format!(
        "alive=1; \
         for i in $(seq 1 100); do \
           if ! kill -0 {pid} 2>/dev/null; then alive=0; break; fi; \
           sleep 0.1; \
         done; \
         if [ \"$alive\" = 1 ]; then kill -9 {pid} 2>/dev/null; sleep 0.5; fi; \
         base=$(basename {quoted}); \
         dest=\"$HOME/.Trash/$base\"; \
         if [ -e \"$dest\" ]; then dest=\"$HOME/.Trash/${{base%.app}} $(date +%s).app\"; fi; \
         mv -f {quoted} \"$dest\"; rc=$?; \
         echo \"$(date '+%Y-%m-%d %H:%M:%S') uninstall: mv {quoted} -> $dest exited rc=$rc (alive=$alive)\" >> {log_quoted}",
        pid = pid,
        quoted = quoted,
        log_quoted = log_quoted,
    );
    match Command::new("/bin/sh").arg("-c").arg(cmd).spawn() {
        Ok(_) => {
            log::info!("uninstall: scheduled app-bundle trash for {bundle:?}");
            Some(bundle)
        }
        Err(err) => {
            log::error!("uninstall: failed to spawn app-bundle trasher: {err}");
            None
        }
    }
}

#[tauri::command]
fn show_notification(
    app: AppHandle,
    title: String,
    body: String,
    action: Option<String>,
) -> Result<(), String> {
    show_notification_impl(&app, &title, &body, action)
}

#[cfg(target_os = "macos")]
pub(crate) fn show_notification_impl(
    app: &AppHandle,
    title: &str,
    body: &str,
    _action: Option<String>,
) -> Result<(), String> {
    let title = title.to_string();
    let body = body.to_string();
    let identifier = if tauri::is_dev() {
        "com.apple.Terminal".to_string()
    } else {
        app.config().identifier.clone()
    };

    std::thread::spawn(move || {
        // set_application is guarded by a Once internally, so repeat calls are cheap.
        let _ = mac_notification_sys::set_application(&identifier);
        let _ = mac_notification_sys::Notification::new()
            .title(&title)
            .message(&body)
            // Waiting for clicks spins a private NSRunLoop in mac-notification-sys
            // and can hold a full CPU core while the notification is pending.
            .asynchronous(true)
            .send();
    });
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn show_notification_impl(
    app: &AppHandle,
    title: &str,
    body: &str,
    _action: Option<String>,
) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| format!("Could not show notification: {e}"))
}

/// True when a Headroom proxy startup error chain looks like an environmental
/// port conflict (another process — possibly a stale headroom child — holds
/// the proxy port). Used to route these failures to a separate, rate-limited
/// Sentry fingerprint so the dashboard isn't drowned in non-actionable noise.
pub(crate) fn is_port_conflict_failure(technical_err: &str) -> bool {
    port_conflict::is_port_conflict(technical_err)
        || technical_err.contains("headroom proxy already running on port")
}

/// Report a headroom proxy startup failure to Sentry. If the error chain
/// contains a `HeadroomStartupFailure`, its log tail, log path, and invocation
/// are sent as structured `extra` fields so we can see what Python printed
/// before failing to bind the port.
pub(crate) fn capture_headroom_start_failure(context: &str, err: &anyhow::Error) {
    let technical_err = format!("{err:#}");

    // Environmental failures: another process holds port 6768, or a stale
    // headroom proxy is still bound. The user gets an actionable hint via
    // `state::classify_startup_error` and the persistent-conflict case is
    // surfaced separately by `port_conflict::note_proxy_failed`. Capture once
    // per session at Warning level under a distinct fingerprint so the
    // dashboard sees real failures (stale child holding the port,
    // sleep/wake race) without drowning in non-actionable noise.
    let is_port_conflict = is_port_conflict_failure(&technical_err);

    let startup_failure = err
        .chain()
        .find_map(|e| e.downcast_ref::<tool_manager::HeadroomStartupFailure>());

    let headline = format!("{context}: {technical_err}");
    let truncated = headline.chars().take(400).collect::<String>();

    if is_port_conflict {
        if PORT_CONFLICT_CAPTURED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        sentry::with_scope(
            |scope| {
                let fp: &[&str] = &["proxy_start_port_conflict"];
                scope.set_fingerprint(Some(fp));
                if let Some(failure) = startup_failure {
                    scope.set_extra("program", failure.program.clone().into());
                    scope.set_extra("args", failure.args.join(" ").into());
                    scope.set_extra("log_path", failure.log_path.clone().into());
                    scope.set_extra("log_tail", failure.log_tail.clone().into());
                    scope.set_extra("reason", failure.reason.clone().into());
                }
                scope.set_extra("error_chain", technical_err.clone().into());
            },
            || {
                sentry::capture_message(&truncated, sentry::Level::Warning);
            },
        );
        return;
    }

    if let Some(failure) = startup_failure {
        sentry::with_scope(
            |scope| {
                scope.set_extra("program", failure.program.clone().into());
                scope.set_extra("args", failure.args.join(" ").into());
                scope.set_extra("log_path", failure.log_path.clone().into());
                scope.set_extra("log_tail", failure.log_tail.clone().into());
                scope.set_extra("reason", failure.reason.clone().into());
                scope.set_extra("error_chain", technical_err.clone().into());
            },
            || {
                sentry::capture_message(&truncated, sentry::Level::Error);
            },
        );
    } else {
        sentry::capture_message(&truncated, sentry::Level::Error);
    }
}

/// Capture once per "down episode" when the watchdog gives up on restarting
/// the proxy. Fires before stop_headroom tears down the tracked child handle
/// and proxy log, so the payload reflects the failure we're recovering from.
///
/// `backend_readyz_outcome` is probed by the watchdog before deciding to give
/// up (so the rescue path can inspect it) and threaded through here to avoid
/// a second probe.
fn capture_watchdog_give_up(
    state: &AppState,
    consecutive_failures: u32,
    bypass_active: bool,
    backend_readyz_outcome: String,
) {
    if WATCHDOG_DOWN_CAPTURED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let exit_status = state.headroom_process_exited();
    let upgrade_in_progress = state.runtime_upgrade_in_progress();
    let logs_dir = state.tool_manager.logs_dir();
    let log_tail = tool_manager::newest_proxy_log_path(&logs_dir)
        .map(|path| tool_manager::tail_log_file(&path, 100));
    let last_startup_error = state.last_startup_error.lock().clone();

    let tracked_pid: Option<u32> = state
        .headroom_process
        .lock()
        .as_ref()
        .map(|child| child.id());
    let port_accepts_tcp = crate::runtime_probe::proxy_port_accepts_connection();
    let process_cpu_secs =
        tracked_pid.and_then(crate::runtime_probe::tracked_process_cpu_time_secs);
    // CPU *rate*, not cumulative. `process_cpu_secs` is lifetime CPU
    // (`ps -o time=`); any long-lived-but-now-idle process carries a large
    // cumulative value, so using it as a deadlock proxy mislabels a healthy
    // idle process as a deadlock (Sentry proxy_unreachable_post_boot showed 12s
    // cumulative + 28min silent flagged as Error). Re-sample over a ~4s window
    // and defer the rate judgement to `cpu_rate_indicates_burn`.
    let cpu_actively_burning = match (tracked_pid, process_cpu_secs) {
        (Some(pid), Some(before)) => {
            let started = std::time::Instant::now();
            std::thread::sleep(std::time::Duration::from_secs(4));
            let elapsed = started.elapsed().as_secs_f64();
            crate::runtime_probe::tracked_process_cpu_time_secs(pid)
                .map(|after| runtime_diagnostics::cpu_rate_indicates_burn(before, after, elapsed))
                .unwrap_or(false)
        }
        _ => false,
    };
    let log_silent_secs = crate::state::newest_proxy_log_mtime(&logs_dir).and_then(|mtime| {
        std::time::SystemTime::now()
            .duration_since(mtime)
            .ok()
            .map(|d| d.as_secs())
    });

    let report = runtime_diagnostics::build_watchdog_give_up_report(
        consecutive_failures,
        bypass_active,
        upgrade_in_progress,
        exit_status,
        log_tail,
        last_startup_error,
        tracked_pid,
        port_accepts_tcp,
        process_cpu_secs,
        log_silent_secs,
        backend_readyz_outcome,
    );

    // Default to Warning: give-up is the documented recovery path, not a
    // bug. Escalate to Error only when there's a real signal something is
    // stuck — spawn keeps erroring, or the child is alive and *actively*
    // burning CPU (likely deadlock) while the log has gone quiet. Plain
    // network/restart blips stay at Warning so they don't pollute the Error
    // inbox.
    let cpu_deadlock_signal = cpu_actively_burning && report.log_silent_secs.unwrap_or(0) >= 30;
    let level = if report.last_startup_error.is_some() || cpu_deadlock_signal {
        sentry::Level::Error
    } else {
        sentry::Level::Warning
    };

    sentry::with_scope(
        |scope| {
            let fp: &[&str] = &["proxy_unreachable_post_boot"];
            scope.set_fingerprint(Some(fp));
            scope.set_extra(
                "tracked_child_exit_status",
                report.tracked_child_exit_status.clone().into(),
            );
            scope.set_extra("bypass_active", report.bypass_active.into());
            scope.set_extra(
                "runtime_upgrade_in_progress",
                report.runtime_upgrade_in_progress.into(),
            );
            scope.set_extra(
                "consecutive_failures",
                (report.consecutive_failures as i64).into(),
            );
            if let Some(tail) = &report.log_tail {
                scope.set_extra("proxy_log_tail", tail.clone().into());
            }
            if let Some(err) = &report.last_startup_error {
                scope.set_extra("last_startup_error", err.clone().into());
            }
            if let Some(pid) = report.tracked_pid {
                scope.set_extra("tracked_pid", (pid as i64).into());
            }
            scope.set_extra("port_accepts_tcp", report.port_accepts_tcp.into());
            if let Some(cpu) = report.process_cpu_secs {
                scope.set_extra("process_cpu_secs", (cpu as i64).into());
            }
            if let Some(silent) = report.log_silent_secs {
                scope.set_extra("log_silent_secs", (silent as i64).into());
            }
            scope.set_extra(
                "backend_readyz_outcome",
                report.backend_readyz_outcome.clone().into(),
            );
        },
        || {
            sentry::capture_message(&report.message, level);
        },
    );
}

/// Diagnostic snapshot taken at the moment a boot-validation failure is
/// captured. Distinguishes "the new proxy never spawned" (tracked_child=false)
/// from "spawned but crashed before writing logs" (no new log) from "spawned
/// and bound but unreachable" (port_bound=true, log written, /livez never
/// answered). None for install-phase failures where no proxy launch happened.
///
/// When `tracked_child` is false, the secondary fields below identify which
/// `ensure_headroom_running` short-circuit fired or whether the spawn errored
/// outright — without these, every "Stalled" / "NotStarted" event looks
/// identical in Sentry.
#[derive(Default, Clone)]
pub(crate) struct UpgradeBootDiagnostics {
    pub tracked_child: bool,
    pub new_proxy_log_written: bool,
    pub proxy_port_bound: bool,
    pub python_installed: bool,
    pub proxy_bypass: bool,
    pub pricing_allows_optimization: bool,
    pub runtime_paused: bool,
    pub ensure_error: Option<String>,
    /// Last ~100 lines of pip stdout/stderr from the install pass that
    /// produced the venv we're now booting. Pip can return exit 0 while
    /// leaving the venv broken (skipped packages, ABI-mismatched native
    /// deps); this tail is the only forensic record of what pip actually
    /// did. Empty string when no pip ran (e.g. requirements-repair).
    pub pip_output_tail: String,
}

/// Report a runtime upgrade failure to Sentry. `phase` is "install" for
/// pip/smoke-test failures, "boot_validation" for "installed but didn't boot".
/// `outcome` is the BootValidationOutcome label when phase is boot_validation.
pub(crate) fn capture_upgrade_failure(
    err: &anyhow::Error,
    restored: bool,
    phase: &str,
    outcome: Option<&str>,
    duration_ms: Option<u64>,
    target_version: Option<&str>,
    fallback_version: Option<&str>,
    log_tail: Option<&str>,
    boot_diagnostics: Option<UpgradeBootDiagnostics>,
) {
    let technical_err = format!("{err:#}");
    let cmd_failure = err
        .chain()
        .find_map(|e| e.downcast_ref::<process_runner::CommandFailure>());

    // ENOSPC is environmental — the user can't fix it by retrying, and the
    // pip log dump bloats Sentry with thousands of "Requirement already
    // satisfied" lines per report. Drop the Sentry capture; the user still
    // sees the disk-full hint via `classify_upgrade_error`, and the local
    // failure is recorded by the caller's `record_upgrade_failure` +
    // analytics::track_event.
    let cmd_stderr = cmd_failure.map(|f| f.stderr.as_str()).unwrap_or("");
    if is_disk_full_signal(&technical_err) || is_disk_full_signal(cmd_stderr) {
        log::warn!(
            "skipping Sentry capture for runtime_upgrade_failed ({phase}): disk full (ENOSPC)"
        );
        return;
    }

    // Sentry drops extras larger than ~16KB. Cap the tail aggressively so the
    // tail's tail (where the panic/error usually lives) survives.
    let log_tail_capped = log_tail.map(|s| {
        if s.len() > 12_000 {
            let cut = s.len() - 12_000;
            format!("[truncated {cut} bytes]\n...{}", &s[cut..])
        } else {
            s.to_string()
        }
    });

    let outcome_for_fingerprint = outcome.unwrap_or("none");
    let fingerprint: [&str; 3] = ["runtime_upgrade", phase, outcome_for_fingerprint];

    // Bake diagnostic fields into the message so they appear in the issue
    // title/preview without requiring a drill-down into tags. The first ~400
    // chars of the err chain are usually enough to disambiguate.
    let mut summary = format!("runtime_upgrade_failed ({phase})");
    if let Some(o) = outcome {
        summary.push_str(&format!(" outcome={o}"));
    }
    if let Some(d) = duration_ms {
        summary.push_str(&format!(" duration_ms={d}"));
    }
    let err_capped: String = technical_err.chars().take(400).collect();
    summary.push_str(&format!(" err={err_capped}"));

    let endpoint_protection_suspected = is_endpoint_protection_signal(&technical_err);

    sentry::with_scope(
        |scope| {
            scope.set_tag("flow", "runtime_upgrade");
            scope.set_tag("upgrade_phase", phase);
            scope.set_tag(
                "endpoint_protection_suspected",
                if endpoint_protection_suspected {
                    "true"
                } else {
                    "false"
                },
            );
            if let Some(o) = outcome {
                scope.set_tag("outcome", o);
            }
            if let Some(t) = target_version {
                scope.set_tag("target_version", t);
            }
            if let Some(f) = fallback_version {
                scope.set_tag("fallback_version", f);
            }
            scope.set_extra("rollback_restored", restored.into());
            scope.set_extra("error_chain", technical_err.clone().into());
            if let Some(d) = duration_ms {
                scope.set_extra("duration_ms", d.into());
            }
            if let Some(tail) = log_tail_capped.as_deref() {
                scope.set_extra("log_tail", tail.into());
            }
            if let Some(diag) = boot_diagnostics.as_ref() {
                scope.set_tag(
                    "tracked_child",
                    if diag.tracked_child { "true" } else { "false" },
                );
                scope.set_tag(
                    "new_proxy_log_written",
                    if diag.new_proxy_log_written {
                        "true"
                    } else {
                        "false"
                    },
                );
                scope.set_tag(
                    "proxy_port_bound",
                    if diag.proxy_port_bound {
                        "true"
                    } else {
                        "false"
                    },
                );
                scope.set_extra("tracked_child", diag.tracked_child.into());
                scope.set_extra("new_proxy_log_written", diag.new_proxy_log_written.into());
                scope.set_extra("proxy_port_bound", diag.proxy_port_bound.into());
                scope.set_extra("python_installed", diag.python_installed.into());
                scope.set_extra("proxy_bypass", diag.proxy_bypass.into());
                scope.set_extra(
                    "pricing_allows_optimization",
                    diag.pricing_allows_optimization.into(),
                );
                scope.set_extra("runtime_paused", diag.runtime_paused.into());
                if let Some(err) = diag.ensure_error.as_deref() {
                    scope.set_extra("ensure_headroom_running_error", err.into());
                }
                if !diag.pip_output_tail.is_empty() {
                    // Cap aggressively — Sentry drops extras > ~16KB and the
                    // tail (where pip warnings/skips/successfully-installed
                    // lines live) is the most informative part.
                    let tail = if diag.pip_output_tail.len() > 12_000 {
                        let cut = diag.pip_output_tail.len() - 12_000;
                        format!(
                            "[truncated {cut} bytes]\n...{}",
                            &diag.pip_output_tail[cut..]
                        )
                    } else {
                        diag.pip_output_tail.clone()
                    };
                    scope.set_extra("pip_install_output", tail.into());
                }
            }
            if let Some(failure) = cmd_failure {
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
            }
            scope.set_fingerprint(Some(fingerprint.as_slice()));
        },
        || {
            // Build the anyhow chain as exception values. With at least one
            // exception present, the AttachStacktraceIntegration attaches the
            // stacktrace to the exception rather than emitting a synthetic
            // thread frame full of sentry/backtrace internals.
            let mut exception_values: Vec<sentry::protocol::Exception> = err
                .chain()
                .map(|e| sentry::protocol::Exception {
                    ty: "anyhow::Error".to_string(),
                    value: Some(e.to_string()),
                    ..Default::default()
                })
                .collect();
            // Sentry convention: innermost cause first.
            exception_values.reverse();

            let event = sentry::protocol::Event {
                message: Some(summary.clone()),
                level: sentry::protocol::Level::Error,
                exception: exception_values.into(),
                ..Default::default()
            };
            sentry::capture_event(event);
        },
    );
}

/// High-confidence signatures that an install/runtime failure was caused by
/// endpoint-protection software (antivirus or EDR) blocking the freshly
/// installed native code. Conservative on purpose — we only match patterns
/// that are unlikely to surface from anything else, so the user-facing hint
/// stays trustworthy. If the matcher grows past ~6 patterns we should split
/// it by failure surface (install vs runtime) and consider tightening.
///
/// Input is matched case-insensitively.
pub(crate) fn is_endpoint_protection_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    // Apple's loader rejecting a fresh signature (codesign tampered or not
    // recognized by the kernel — almost always EDR injecting/rewriting).
    if lower.contains("code signature invalid")
        || lower.contains("code signature could not be verified")
    {
        return true;
    }
    // `dlopen` reports the "tried: ... (operation not permitted)" suffix when
    // a sandbox/AV blocks a freshly-extracted .so/.dylib. The "library not
    // loaded" prefix alone is too noisy (covers ordinary missing-dep cases),
    // so require the "not permitted" companion.
    if (lower.contains("library not loaded") || lower.contains("dlopen"))
        && lower.contains("not permitted")
    {
        return true;
    }
    // SIGKILL with no app-side cause is the classic EDR signature — the
    // process is killed before it can write a useful error. Plain "killed"
    // is too noisy (covers OOM, user pkill), so require the explicit signal
    // marker. CommandFailure formats this as "signal=9" or "Killed: 9".
    if lower.contains("signal=9") || lower.contains("killed: 9") || lower.contains("exit code 137")
    {
        return true;
    }
    // `Operation not permitted` paired with a freshly-installed native
    // extension path strongly implicates AV that hooks open(2)/exec(2). The
    // bare phrase appears in too many unrelated permission errors, so we
    // gate it on "site-packages" (where pip just wrote the file) or ".so" /
    // ".dylib" appearing in the same chain.
    if lower.contains("operation not permitted")
        && (lower.contains("site-packages") || lower.contains(".so") || lower.contains(".dylib"))
    {
        return true;
    }
    false
}

/// True when an install/upgrade failure was caused by the user's disk
/// running out of space. ENOSPC is environmental — the user can't fix it
/// by retrying, only by freeing space — so we use this to drop noisy
/// pip-log Sentry reports and emit a single clear local log line instead.
/// The user-facing hint is produced separately by `classify_upgrade_error`.
pub(crate) fn is_disk_full_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("no space left on device")
        || lower.contains("errno 28")
        || lower.contains("enospc")
        || lower.contains("disk full")
}

/// Shared hint copy for endpoint-protection failures. Two variants because
/// the install-time and runtime surfaces want slightly different "what to
/// do" wording (retry the install vs allow the runtime dir + click Retry).
const ENDPOINT_PROTECTION_HINT_INSTALL: &str =
    "Looks like endpoint protection (antivirus or EDR) blocked the new native code. \
     Allow Headroom in your security software, then retry.";

const ENDPOINT_PROTECTION_HINT_RUNTIME: &str =
    "A Headroom component was killed at launch — usually endpoint protection (antivirus or EDR) \
     interfering with freshly-installed code. Allow `~/Library/Application Support/Headroom` \
     in your security software, then click Retry.";

pub(crate) fn endpoint_protection_hint_install() -> String {
    ENDPOINT_PROTECTION_HINT_INSTALL.to_string()
}

pub(crate) fn endpoint_protection_hint_runtime() -> String {
    ENDPOINT_PROTECTION_HINT_RUNTIME.to_string()
}

/// Map common runtime-upgrade failure modes to a short user-facing hint.
pub(crate) fn classify_upgrade_error(err: &anyhow::Error) -> Option<String> {
    let chain_raw = format!("{err:#}");
    // Endpoint protection check uses the raw chain (the matcher does its own
    // case-folding) so signal patterns like "signal=9" match exactly.
    if is_endpoint_protection_signal(&chain_raw) {
        return Some(endpoint_protection_hint_install());
    }
    let chain = chain_raw.to_ascii_lowercase();
    if chain.contains("network")
        || chain.contains("timed out")
        || chain.contains("dns")
        || chain.contains("connection refused")
        || chain.contains("could not resolve")
    {
        return Some("Couldn't reach PyPI. Check your network and retry.".into());
    }
    if chain.contains("no space") || chain.contains("disk full") || chain.contains("enospc") {
        return Some(
            "Not enough disk space to install the update. Free up space and retry.".into(),
        );
    }
    if chain.contains("sha256") || chain.contains("checksum") || chain.contains("digest") {
        return Some("The downloaded wheel's checksum didn't match. Retry to redownload.".into());
    }
    if chain.contains("import") && chain.contains("smoke test") {
        return Some(
            "The new Headroom version couldn't be imported. Try retrying or reinstalling.".into(),
        );
    }
    if chain.contains("resolution") || chain.contains("no matching distribution") {
        return Some(
            "Pip couldn't resolve dependencies for the new version. Please report this.".into(),
        );
    }
    None
}

/// Rescan cadence for the Claude projects cache. This keeps Optimize mostly
/// warm without doing filesystem-heavy project scans every minute forever.
const CLAUDE_PROJECTS_WARM_INTERVAL: std::time::Duration = std::time::Duration::from_secs(75);

/// Keeps `list_claude_code_projects` cache warm on a background thread so the
/// IPC path never pays the projects-dir scan (hundreds of `stat` calls plus
/// per-project metadata reads). Pure cache-fill with no side effects —
/// `list_claude_code_projects` is idempotent and only writes to its own
/// cache slot.
fn spawn_claude_projects_warmer(app: AppHandle) {
    std::thread::spawn(move || {
        // Stagger from the activity observer so both background threads
        // don't simultaneously contend on fs / IPC at boot.
        std::thread::sleep(std::time::Duration::from_secs(5));
        loop {
            let state: tauri::State<'_, AppState> = app.state();
            let _ = state.list_claude_code_projects();
            std::thread::sleep(CLAUDE_PROJECTS_WARM_INTERVAL);
        }
    });
}

#[tauri::command]
fn show_dashboard_window(app: AppHandle) -> Result<(), String> {
    if !onboarding_complete(&app) {
        show_launcher_window(&app).map_err(|err| err.to_string())?;
        return Err("Complete onboarding before opening the tray dashboard.".into());
    }

    ensure_runtime_ready_for_tray(&app);
    hide_launcher_window(&app).map_err(|err| err.to_string())?;
    show_main_window(&app, None).map_err(|err| err.to_string())
}

#[tauri::command]
async fn pause_headroom(app: AppHandle) -> Result<(), String> {
    let state: tauri::State<'_, AppState> = app.state();
    state.set_runtime_paused(true);
    // A deliberate user pause is not an auto-pause; clear the flag so the
    // self-heal loop doesn't fight the user by auto-resuming.
    state.set_runtime_auto_paused(false);
    state.stop_headroom();
    client_adapters::clear_client_setups().map_err(|err| err.to_string())?;
    analytics::track_event(&app, "runtime_paused", None);
    Ok(())
}

#[tauri::command]
async fn start_headroom(app: AppHandle) -> Result<(), String> {
    let state: tauri::State<'_, AppState> = app.state();
    state.resume_runtime().map_err(|err| err.to_string())?;
    std::thread::spawn(|| {
        client_adapters::restore_client_setups();
    });
    analytics::track_event(&app, "runtime_resumed", None);
    Ok(())
}

/// Hard kill + restart of the proxy, wired to the "Resume" button on the
/// paused/auto-paused banner. Unlike `start_headroom`/`resume_runtime` — which
/// no-op when the tracked child is alive-but-hung — this kills the process
/// group first (`stop_headroom` SIGKILLs the group and reaps orphans), so a
/// wedged process is actually replaced by a fresh one. This is the one-click
/// equivalent of the manual quit-and-relaunch users do today.
#[tauri::command]
async fn force_restart_headroom(app: AppHandle) -> Result<(), String> {
    let state: tauri::State<'_, AppState> = app.state();
    state.stop_headroom();
    state.set_runtime_auto_paused(false);
    state.resume_runtime().map_err(|err| err.to_string())?;
    std::thread::spawn(|| {
        client_adapters::restore_client_setups();
    });
    analytics::track_event(&app, "runtime_force_restarted", None);
    Ok(())
}

#[tauri::command]
fn hide_launcher_animated(app: AppHandle) {
    // The launcher close animation now lives in the webview/CSS layer.
    // Keep the backend hide on the straightforward window path instead of
    // mutating window geometry from a background thread.
    let _ = hide_launcher_window(&app);
}

#[tauri::command]
async fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|err| err.to_string())
}

#[tauri::command]
async fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|err| err.to_string())?;
    } else {
        manager.disable().map_err(|err| err.to_string())?;
    }
    manager.is_enabled().map_err(|err| err.to_string())
}

#[tauri::command]
async fn set_rtk_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let state: tauri::State<'_, AppState> = app.state();
    client_adapters::set_rtk_enabled(
        enabled,
        &state.tool_manager.rtk_entrypoint(),
        &state.tool_manager.managed_python(),
    )
    .map_err(|err| err.to_string())?;
    state.invalidate_runtime_status_cache();
    Ok(!client_adapters::is_rtk_disabled())
}

#[tauri::command]
fn uninstall_and_quit(app: AppHandle) -> Result<Vec<String>, String> {
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.stop_headroom();
        // Ponytail lives in Claude Code's plugin registry, outside Headroom's
        // own footprint that perform_full_cleanup() wipes, so remove it here
        // while we still have the ToolManager. Best-effort.
        if let Err(err) = state.tool_manager.uninstall_ponytail() {
            log::warn!("uninstall: removing ponytail plugin failed: {err:#}");
        }
    }

    // Turn off the login item if it was ever enabled, so the system stops
    // listing Headroom as a background item even if the user later reinstalls.
    let _ = app.autolaunch().disable();

    let removed = append_scheduled_app_bundle_cleanup(client_adapters::perform_full_cleanup());

    analytics::track_event(
        &app,
        "uninstall_completed",
        Some(json!({ "removed_paths": removed.len() })),
    );
    analytics::shutdown(&app);
    if let Some(client) = sentry::Hub::current().client() {
        client.flush(Some(std::time::Duration::from_secs(2)));
    }

    let handle = app.clone();
    // Give the frontend a moment to receive the command response before the
    // process exits, so the confirmation toast can render.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        handle.exit(0);
    });

    Ok(removed)
}

#[cfg(target_os = "macos")]
fn append_scheduled_app_bundle_cleanup(mut removed: Vec<String>) -> Vec<String> {
    // Trash the running .app bundle itself once we exit. Best-effort and
    // macOS-only; everything above only removed Headroom's on-disk footprint
    // (config, runtime, caches), not the application.
    if let Some(bundle) = schedule_app_bundle_trash() {
        removed.push(bundle.display().to_string());
    }
    removed
}

#[cfg(not(target_os = "macos"))]
fn append_scheduled_app_bundle_cleanup(removed: Vec<String>) -> Vec<String> {
    removed
}

#[tauri::command]
fn quit_headroom(app: AppHandle) {
    exit_headroom(&app, QuitSource::SettingsButton);
}

fn launched_from_autostart() -> bool {
    std::env::args().any(|arg| arg == AUTOSTART_LAUNCH_ARG)
}

fn exit_headroom(app: &AppHandle, source: QuitSource) {
    let runtime_paused = {
        let state: tauri::State<'_, AppState> = app.state();
        let runtime_paused = state.runtime_is_paused();
        state.stop_headroom();
        let _ = client_adapters::clear_client_setups();
        runtime_paused
    };

    analytics::track_event(
        app,
        "app_quit_requested",
        Some(app_quit_requested_properties(source, runtime_paused)),
    );
    analytics::shutdown(app);
    if let Some(client) = sentry::Hub::current().client() {
        client.flush(Some(std::time::Duration::from_secs(2)));
    }
    app.exit(0);
}

fn app_quit_requested_properties(source: QuitSource, runtime_paused: bool) -> Value {
    json!({
        "source": source.label(),
        "runtime_paused": runtime_paused,
    })
}

pub fn run() {
    let _sentry = if local_mode::enabled() {
        None
    } else {
        SENTRY_DSN.map(|dsn| {
            sentry::init((
                dsn,
                sentry::ClientOptions {
                    release: sentry::release_name!(),
                    attach_stacktrace: true,
                    ..Default::default()
                },
            ))
        })
    };

    // Initialize the panic-safe file logger after Sentry so warn!/error!
    // records flow into Sentry too. Failure here cannot abort startup.
    let _ = logging::init();

    let args = std::env::args().collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--print-managed-footprint") {
        match serde_json::to_string_pretty(&client_footprint::get_managed_footprint()) {
            Ok(report) => {
                println!("{report}");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("failed to build managed footprint report: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--uninstall-dry-run") {
        match serde_json::to_string_pretty(&client_footprint::uninstall_dry_run_report()) {
            Ok(report) => {
                println!("{report}");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("failed to build uninstall dry-run report: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--disable-routing") {
        match client_adapters::clear_client_setups() {
            Ok(()) => {
                println!("disabled managed routing");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("failed to disable managed routing: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--disable-rtk") {
        let runtime = tool_manager::ManagedRuntime::bootstrap_root(&storage::app_data_dir());
        match client_adapters::set_rtk_enabled(
            false,
            &runtime.bin_dir.join("rtk"),
            &runtime.venv_dir.join("bin").join("python"),
        ) {
            Ok(()) => {
                println!("disabled managed RTK integration");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("failed to disable RTK integration: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--disable-markitdown") {
        let runtime = tool_manager::ManagedRuntime::bootstrap_root(&storage::app_data_dir());
        match client_adapters::disable_markitdown_integration(&runtime.bin_dir.join("markitdown")) {
            Ok(changed) => {
                println!("disabled managed MarkItDown integration changed={changed}");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("failed to disable MarkItDown integration: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--disable-caveman") {
        match client_adapters::disable_caveman_integration() {
            Ok(changed) => {
                println!("disabled managed Caveman integration changed={changed}");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("failed to disable Caveman integration: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--uninstall-managed-config") {
        let removed = client_adapters::perform_full_cleanup();
        match serde_json::to_string_pretty(&removed) {
            Ok(report) => {
                println!("{report}");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("failed to serialize cleanup report: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--purge-logs") {
        let activity_facts = storage::config_file(&storage::app_data_dir(), "activity-facts.json");
        match serde_json::to_string_pretty(&message_logging::purge_message_logs(&activity_facts)) {
            Ok(report) => {
                println!("{report}");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("failed to serialize log purge report: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--doctor-reset") {
        let routing = client_adapters::clear_client_setups();
        let activity_facts = storage::config_file(&storage::app_data_dir(), "activity-facts.json");
        let purge = message_logging::purge_message_logs(&activity_facts);
        match routing {
            Ok(()) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&purge)
                        .unwrap_or_else(|_| "{\"logsPurged\":true}".to_string())
                );
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("doctor reset partially failed while disabling routing: {err}");
                std::process::exit(1);
            }
        }
    }

    if let Some(index) = args
        .iter()
        .position(|arg| arg == "--restore-codex-thread-db-backup")
    {
        let Some(path) = args.get(index + 1) else {
            eprintln!("missing path for --restore-codex-thread-db-backup");
            std::process::exit(2);
        };
        match codex_threads::restore_codex_thread_db_backup(path) {
            Ok(result) => match serde_json::to_string_pretty(&result) {
                Ok(report) => {
                    println!("{report}");
                    std::process::exit(0);
                }
                Err(err) => {
                    eprintln!("failed to serialize Codex restore result: {err}");
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("failed to restore Codex thread DB backup: {err}");
                std::process::exit(1);
            }
        }
    }

    // Raise the open-file soft limit to the hard limit. macOS launches GUI apps
    // with RLIMIT_NOFILE soft = 256, which the intercept proxy exhausts under
    // bursty load (each proxied request holds a client + backend FD), producing
    // EMFILE on accept(). The hard limit is far higher; the kernel clamps to
    // kern.maxfilesperproc if rlim_max is RLIM_INFINITY.
    #[cfg(unix)]
    unsafe {
        let mut lim = std::mem::zeroed::<libc::rlimit>();
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) == 0 && lim.rlim_cur < lim.rlim_max {
            lim.rlim_cur = lim.rlim_max;
            let _ = libc::setrlimit(libc::RLIMIT_NOFILE, &lim);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let has_display =
            std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();
        if !has_display {
            log::error!(
                "Headroom requires a graphical display. Set DISPLAY or WAYLAND_DISPLAY before launching."
            );
            std::process::exit(1);
        }
    }

    let state = AppState::new().expect("failed to create app state");

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Second launch: focus the existing window and exit the new process.
            let _ = show_launcher_window(app);
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args([AUTOSTART_LAUNCH_ARG])
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_deep_link::init());

    builder
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                // Accessory policy makes this a menu-bar-only app (no dock icon).
                // Do NOT also call set_dock_visibility(false): it uses Carbon's
                // TransformProcessType, which Apple warns must not be mixed with
                // setActivationPolicy on the same process and intermittently
                // registers a dock icon. LSUIElement=true in Info.plist already
                // covers the packaged bundle.
                app.set_activation_policy(ActivationPolicy::Accessory);
            }

            let launched_from_autostart = launched_from_autostart();
            // Autostart is opt-in. Users enable it explicitly from Settings,
            // which avoids triggering macOS's "Background item added" prompt
            // on first launch.

            app.manage(analytics::AnalyticsClient::new(
                app.package_info().version.to_string(),
            ));
            app.manage(TraySessionSavings(Mutex::new(0.0)));
            setup_tray(app.handle())?;
            spawn_tray_runtime_icon_updater(app.handle().clone());
            spawn_tray_savings_updater(app.handle().clone());
            spawn_proxy_watchdog(app.handle().clone());
            activity_commands::spawn_activity_observer(app.handle().clone());
            spawn_claude_projects_warmer(app.handle().clone());
            let state: tauri::State<'_, AppState> = app.state();
            let app_handle = app.handle().clone();
            analytics::set_headroom_ai_version(
                &app_handle,
                state.tool_manager.installed_headroom_version(),
            );
            analytics::track_event(
                &app_handle,
                "app_started",
                Some(json!({
                    "launch_experience": state.launch_experience_label(),
                    "launch_count": state.launch_count(),
                    "runtime_installed": state.tool_manager.python_runtime_installed(),
                    "autostart_launch": launched_from_autostart
                })),
            );
            // Wire up the bearer-triggered identity-pusher worker. The
            // intercept thread sends a signal here every time it captures a
            // bearer whose value differs from what was previously in the
            // slot; the worker calls `pricing::warm_and_push_identity`,
            // which warms the OAuth profile cache and posts the populated
            // IdentityPayload to `desktop/grace/start`. Throttled to one
            // OAuth fetch per 24 h once the identity is complete.
            //
            // Each iteration is wrapped in `catch_unwind` so a panic inside
            // the HTTP / parsing stack doesn't silently kill the worker
            // thread (which would leave bearer signals piling up in the
            // channel forever). On panic we log + report and resume the
            // recv loop on the next signal.
            let (fresh_bearer_tx, fresh_bearer_rx) = std::sync::mpsc::channel::<()>();
            state.set_fresh_bearer_notifier(fresh_bearer_tx.clone());
            let app_handle_for_pusher = app.handle().clone();
            std::thread::Builder::new()
                .name("identity-pusher".into())
                .spawn(move || {
                    while fresh_bearer_rx.recv().is_ok() {
                        // Coalesce: drain any signals that piled up while
                        // we were processing the previous one.
                        while fresh_bearer_rx.try_recv().is_ok() {}
                        let app_handle = app_handle_for_pusher.clone();
                        let result =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                                let state: tauri::State<'_, AppState> = app_handle.state();
                                pricing::warm_and_push_identity(&state);
                            }));
                        if result.is_err() {
                            log::error!(
                                "identity-pusher worker panicked during warm_and_push_identity"
                            );
                            sentry::capture_message(
                                "identity-pusher worker panicked",
                                sentry::Level::Error,
                            );
                        }
                    }
                })
                .expect("spawn identity pusher");

            let wants_headroom = switchboard_commands::saved_switchboard_mode_wants_headroom();
            if wants_headroom {
                // Start the intercept layer before anything else touches port 6767.
                state.ensure_proxy_intercept_running();
            }
            if state.should_present_on_launch() && !launched_from_autostart {
                let _ = show_launcher_window(app.handle());
            }
            if wants_headroom && state.tool_manager.python_runtime_installed() {
                state.set_runtime_starting(true);
            }
            // Strip noisy traffic_learner error_recovery patterns before the
            // proxy starts re-flushing them. See memory_scrubber for context.
            std::thread::spawn(|| {
                memory_scrubber::scrub_all(&headroom_memory_db_path());
            });
            std::thread::spawn(move || {
                let state: tauri::State<'_, AppState> = app_handle.state();
                state.warm_runtime_on_launch(&app_handle);
            });
            if wants_headroom {
                // Restore previously connected client integrations in the background.
                std::thread::spawn(|| {
                    client_adapters::restore_client_setups();
                    // restore_client_setups only retags Codex threads back to the
                    // headroom provider for clients in `remembered_clients`, which a
                    // plain Cmd-Q / dock quit / app-update restart never populates
                    // (only pause and the Settings "Quit" do). Those exit paths still
                    // run the quit-time headroom->openai retag, so without this the
                    // Codex history menu stays empty after an update restart. Mirror
                    // the quit retag whenever Codex is still configured.
                    if client_adapters::is_codex_enabled() {
                        codex_threads::retag_codex_threads_to_headroom();
                    }
                });
            }

            // headroom:// deep link — Polar's checkout success page redirects
            // here. Triggers an immediate pricing refresh so the gate releases
            // within seconds of payment instead of waiting for the 5s poll.
            use tauri_plugin_deep_link::DeepLinkExt;
            let deep_link_app = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                // NOTE: never call `eprintln!`/`println!` here. When macOS
                // launches the app fresh via a URL scheme, stderr is not
                // connected to a valid fd and any stdio write panics with
                // EIO. Use `log::*` (panic-safe file logger) instead.
                //
                // This callback is invoked synchronously from tao's
                // `application:openURLs:` handler, which is `extern "C"` —
                // any panic that escapes here aborts the whole process via
                // `panic_cannot_unwind`. Wrap the body in `catch_unwind` so
                // an internal failure degrades gracefully instead.
                let deep_link_app = deep_link_app.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    for url in event.urls() {
                        if url.scheme() == "headroom" {
                            let app_handle = deep_link_app.clone();
                            let _ = show_launcher_window(&app_handle);
                            // Run the reconciliation on a worker thread — the
                            // deep-link callback is on the main thread and we
                            // don't want pricing's blocking HTTP call there.
                            std::thread::spawn(move || {
                                let state: tauri::State<'_, AppState> = app_handle.state();
                                match pricing::get_pricing_status(&state) {
                                    Ok(status) => {
                                        state.apply_pricing_gate_status(&status);
                                        state
                                            .apply_codex_pricing_gate_status(status.codex.as_ref());
                                        let _ = app_handle.emit("pricing-refreshed", &status);
                                    }
                                    Err(err) => {
                                        sentry::capture_message(
                                            &format!("deep link pricing refresh failed: {err}"),
                                            sentry::Level::Warning,
                                        );
                                    }
                                }
                            });
                            // Only handle the first headroom:// URL in the batch.
                            break;
                        }
                    }
                }));
                if result.is_err() {
                    sentry::capture_message("deep link callback panicked", sentry::Level::Error);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| handle_window_event(window, event))
        .manage(state)
        .manage(app_update_commands::PendingAppUpdate(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            dashboard_commands::get_dashboard_state,
            dashboard_commands::get_savings_attribution_events,
            dashboard_commands::get_savings_attribution_counters,
            dashboard_commands::record_measured_savings_attribution,
            rollback_commands::preview_managed_config_apply,
            rollback_commands::execute_managed_config_apply,
            rollback_commands::preview_managed_rollback,
            rollback_commands::execute_managed_rollback,
            rollback_commands::preview_dedicated_cleanup_rollback,
            rollback_commands::execute_dedicated_cleanup_rollback,
            rollback_commands::get_managed_footprint,
            rollback_commands::get_uninstall_dry_run_report,
            rollback_commands::preview_managed_rollback_undo_all,
            rollback_commands::execute_managed_rollback_undo_all,
            repo_intelligence_commands::build_repo_intelligence_summary,
            repo_intelligence_commands::get_latest_repo_intelligence_summary,
            repo_intelligence_commands::get_repo_intelligence_context_pack,
            repo_intelligence_commands::search_repo_intelligence_symbols,
            repo_intelligence_commands::get_repo_intelligence_dependents,
            repo_intelligence_commands::get_repo_intelligence_manifest,
            repo_intelligence_commands::clear_repo_intelligence_summary,
            repo_intelligence_commands::get_repo_manifest,
            repo_map::preflight_repo_map,
            repo_map::generate_repo_map,
            repo_map::open_repo_map_artifact,
            repo_intelligence_commands::get_repo_pack,
            repo_intelligence_commands::get_agent_handoff,
            repo_intelligence_commands::get_index_freshness,
            repo_intelligence_commands::clear_repo_index,
            app_update_commands::get_app_update_configuration,
            release_evidence::load_release_readiness_report,
            release_evidence::refresh_release_readiness_report,
            release_evidence::run_release_evidence_command,
            app_update_commands::check_for_app_update,
            app_update_commands::install_app_update,
            app_update_commands::restart_app,
            app_update_commands::show_app_update_notification,
            show_notification,
            addon_commands::install_addon,
            addon_commands::set_addon_enabled,
            addon_commands::uninstall_addon,
            addon_commands::set_caveman_level,
            repo_memory_commands::install_repo_memory_mcp,
            repo_memory_commands::start_repo_memory_mcp,
            repo_memory_commands::stop_repo_memory_mcp,
            runtime_commands::bootstrap_runtime,
            runtime_commands::start_bootstrap,
            runtime_commands::get_bootstrap_progress,
            runtime_commands::get_runtime_upgrade_progress,
            runtime_commands::retry_runtime_upgrade,
            runtime_commands::retry_runtime_upgrade_with_rebuild,
            runtime_commands::dismiss_runtime_upgrade_failure,
            runtime_commands::get_runtime_status,
            switchboard_commands::get_switchboard_state,
            switchboard_commands::get_doctor_report,
            switchboard_commands::run_doctor_repair,
            switchboard_commands::set_switchboard_mode,
            switchboard_commands::set_savings_mode,
            activity_commands::get_headroom_logs,
            activity_commands::get_headroom_request_count,
            activity_commands::get_headroom_request_counts_by_agent,
            optimization_commands::get_optimization_snapshot,
            optimization_commands::run_preemptive_compaction,
            optimization_commands::get_optimization_action_policy,
            optimization_commands::set_optimization_action_policy,
            optimization_commands::validate_model_routing,
            activity_commands::get_rtk_activity,
            activity_commands::get_tool_logs,
            activity_commands::get_claude_code_projects,
            activity_commands::get_claude_usage,
            activity_commands::get_claude_profile,
            pricing_commands::get_headroom_pricing_status,
            pricing_commands::request_headroom_auth_code,
            pricing_commands::verify_headroom_auth_code,
            pricing_commands::sign_out_headroom_account,
            pricing_commands::activate_headroom_account,
            pricing_commands::create_headroom_checkout_session,
            pricing_commands::change_headroom_subscription_plan,
            pricing_commands::reactivate_headroom_subscription,
            pricing_commands::get_headroom_billing_portal_url,
            activity_commands::get_activity_feed,
            message_settings_commands::get_message_logging_settings,
            message_settings_commands::set_message_logging_settings,
            message_settings_commands::enable_full_message_logging,
            message_settings_commands::disable_full_message_logging,
            message_settings_commands::purge_message_logs,
            message_settings_commands::get_codex_thread_retagging_settings,
            message_settings_commands::set_codex_thread_retagging_settings,
            message_settings_commands::restore_codex_thread_db_backup,
            learning_commands::list_live_learnings,
            learning_commands::list_live_learnings_for_projects,
            learning_commands::delete_live_learning,
            learning_commands::list_applied_patterns,
            learning_commands::list_applied_patterns_for_projects,
            learning_commands::delete_applied_pattern,
            learning_commands::get_headroom_learn_status,
            learning_commands::get_headroom_learn_prereq_status,
            learning_commands::start_headroom_learn,
            activity_commands::get_transformations_feed,
            client_setup_commands::apply_client_setup,
            client_setup_commands::verify_client_setup,
            connector_smoke::run_connector_smoke_test,
            client_setup_commands::get_client_connectors,
            client_setup_commands::disable_client_setup,
            client_setup_commands::clear_client_setups,
            pause_headroom,
            start_headroom,
            force_restart_headroom,
            app_services_commands::track_analytics_event,
            show_dashboard_window,
            app_services_commands::open_headroom_dashboard,
            app_services_commands::open_external_link,
            app_services_commands::submit_contact_request,
            hide_launcher_animated,
            complete_setup_wizard,
            accept_terms,
            get_autostart_enabled,
            set_autostart_enabled,
            set_rtk_enabled,
            uninstall_and_quit,
            quit_headroom,
            #[cfg(debug_assertions)]
            switchboard_commands::debug_force_proxy_bypass
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Tear down the proxy on every exit path (Cmd-Q, dock quit, signal,
            // or our explicit quit/restart commands). Without this, the proxy
            // outlives the desktop and the next launch reuses an orphan.
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                let state: tauri::State<'_, AppState> = app.state();
                state.stop_headroom();
                // Gracefully reverse every client's base-URL override (and shell
                // blocks) on quit so Claude Code / Codex fall back to talking
                // directly to their native providers while Headroom is not
                // running, instead of pointing at a now-dead proxy on 6767. The
                // snapshot is remembered so the next launch's
                // restore_client_setups re-applies it. Guarded to run once: the
                // exit handler fires for both ExitRequested and Exit, and a
                // second clear_client_setups wipes the remembered snapshot.
                if !EXIT_CLEAR_DONE.swap(true, Ordering::AcqRel) {
                    if let Err(err) = client_adapters::clear_client_setups() {
                        log::warn!("exit: clear_client_setups failed: {err}");
                    }
                }
                // Hand Codex threads back to the native provider so its history
                // menu stays whole while Headroom is not running. Cmd-Q / dock
                // quit / signals skip exit_headroom -> clear_client_setups, so
                // this is the only retag they get; the next launch re-applies the
                // headroom tag via restore_client_setups. Best-effort.
                codex_threads::retag_codex_threads_to_native();
            }
        });
}

pub fn headroom_memory_db_path() -> std::path::PathBuf {
    crate::storage::memory_db_path(&crate::storage::app_data_dir())
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show =
        tauri::menu::MenuItem::with_id(app, "show", "Show Mac AI Switchboard", true, None::<&str>)?;
    let release_readiness = tauri::menu::MenuItem::with_id(
        app,
        "release-readiness",
        "Release Readiness",
        true,
        None::<&str>,
    )?;
    let rollback_center = tauri::menu::MenuItem::with_id(
        app,
        "rollback-center",
        "Rollback Center",
        true,
        None::<&str>,
    )?;
    let quit =
        tauri::menu::MenuItem::with_id(app, "quit", "Quit Mac AI Switchboard", true, None::<&str>)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let menu = tauri::menu::Menu::with_items(
        app,
        &[
            &show,
            &release_readiness,
            &rollback_center,
            &separator,
            &quit,
        ],
    )?;
    let popup_menu = menu.clone();
    let mut tray_builder = tauri::tray::TrayIconBuilder::with_id("headroom-tray")
        .menu(&menu)
        .icon_as_template(false)
        .tooltip("Mac AI Switchboard")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let _ = toggle_main_window(tray.app_handle(), Some(rect));
            }

            if let TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let window = app
                    .get_webview_window("main")
                    .or_else(|| app.get_webview_window("launcher"));

                if let Some(window) = window {
                    let _ = window.popup_menu(&popup_menu);
                }
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if onboarding_complete(app) {
                    let _ = hide_launcher_window(app);
                    let _ = show_main_window(app, None);
                    let app_bg = app.clone();
                    std::thread::spawn(move || ensure_runtime_ready_for_tray(&app_bg));
                } else {
                    let _ = show_launcher_window(app);
                }
            }
            "release-readiness" | "rollback-center" => {
                if onboarding_complete(app) {
                    let _ = hide_launcher_window(app);
                    let _ = show_main_window(app, None);
                    let _ = app.emit(
                        "notification-clicked",
                        serde_json::json!({ "action": event.id.as_ref() }),
                    );
                } else {
                    let _ = show_launcher_window(app);
                }
            }
            "quit" => {
                exit_headroom(app, QuitSource::TrayMenu);
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder.build(app)?;

    Ok(())
}

fn spawn_tray_runtime_icon_updater(app: AppHandle) {
    let icons = match tray_runtime::build_tray_runtime_icons() {
        Ok(icons) => icons,
        Err(err) => {
            sentry::capture_message(
                &format!("failed to build runtime tray icons: {err}"),
                sentry::Level::Warning,
            );
            return;
        }
    };

    std::thread::spawn(move || {
        let mut frame_index = 0usize;
        let mut last_non_booting: Option<tray_runtime::TrayRuntimeVisual> = None;
        let mut last_displayed_dollars: Option<u32> = None;
        let mut last_tooltip: Option<String> = None;
        let mut unhealthy_streak: u8 = 0;
        let mut last_connector_check = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(60))
            .unwrap_or_else(std::time::Instant::now);
        let mut cached_connector_enabled: bool =
            client_adapters::is_claude_code_enabled() || client_adapters::is_codex_enabled();

        loop {
            // Re-check connectors at most every ~2s, regardless of whether the
            // tick rate is booting-fast (260ms) or idle-slow (1500ms). Time-based
            // instead of tick-count based so the cadence stays correct across the
            // adaptive sleep below. "Connected" means any supported connector
            // (Claude Code or Codex) is routing through Headroom.
            if last_connector_check.elapsed() >= std::time::Duration::from_secs(2) {
                cached_connector_enabled = client_adapters::is_claude_code_enabled()
                    || client_adapters::is_codex_enabled();
                last_connector_check = std::time::Instant::now();
            }

            let raw_visual = {
                let state: tauri::State<'_, AppState> = app.state();
                let runtime = state.runtime_status();
                if runtime.running {
                    if cached_connector_enabled {
                        tray_runtime::TrayRuntimeVisual::Running
                    } else {
                        tray_runtime::TrayRuntimeVisual::Disconnected
                    }
                } else if runtime.starting {
                    tray_runtime::TrayRuntimeVisual::Booting
                } else if runtime.paused {
                    tray_runtime::TrayRuntimeVisual::Paused
                } else if runtime.installed && !runtime.proxy_reachable {
                    // Runtime should be up (installed, not paused, not booting)
                    // but the proxy isn't answering. Treat as unhealthy so the
                    // user has a visible signal the watchdog is working on it.
                    tray_runtime::TrayRuntimeVisual::Unhealthy
                } else {
                    tray_runtime::TrayRuntimeVisual::Off
                }
            };
            let visual = tray_runtime::debounced_tray_runtime_visual(
                raw_visual,
                last_non_booting,
                &mut unhealthy_streak,
            );

            if let Some(tray) = app.tray_by_id("headroom-tray") {
                let tooltip = match visual {
                    tray_runtime::TrayRuntimeVisual::Booting => {
                        "Mac AI Switchboard — starting engine"
                    }
                    tray_runtime::TrayRuntimeVisual::Running => {
                        "Mac AI Switchboard — engine active"
                    }
                    tray_runtime::TrayRuntimeVisual::Paused => {
                        "Mac AI Switchboard — engine paused (Claude Code or Codex running normally)"
                    }
                    tray_runtime::TrayRuntimeVisual::Unhealthy => {
                        "Mac AI Switchboard — engine unreachable, attempting restart"
                    }
                    tray_runtime::TrayRuntimeVisual::Disconnected => {
                        "Mac AI Switchboard — Claude Code or Codex not connected"
                    }
                    tray_runtime::TrayRuntimeVisual::Off => "Mac AI Switchboard — off",
                };

                let mut icon_changed = false;
                match visual {
                    tray_runtime::TrayRuntimeVisual::Booting => {
                        let icon =
                            icons.booting_frames[frame_index % icons.booting_frames.len()].clone();
                        let _ = tray.set_icon(Some(icon));
                        icon_changed = true;
                        frame_index = (frame_index + 1) % icons.booting_frames.len();
                        last_non_booting = Some(tray_runtime::TrayRuntimeVisual::Booting);
                    }
                    tray_runtime::TrayRuntimeVisual::Running => {
                        let dollars = {
                            let savings_state: tauri::State<'_, TraySessionSavings> = app.state();
                            let v = *savings_state.0.lock();
                            let d = v.floor() as u32;
                            #[cfg(debug_assertions)]
                            let d = d.max(1);
                            d
                        };
                        let changed_visual =
                            last_non_booting != Some(tray_runtime::TrayRuntimeVisual::Running);
                        let changed_dollars = last_displayed_dollars != Some(dollars);
                        if changed_visual || changed_dollars {
                            let (bw, bh) = icons.running_dims;
                            let (new_rgba, new_w, new_h) = tray_runtime::build_running_with_savings(
                                &icons.running_rgba,
                                bw,
                                bh,
                                dollars,
                            );
                            let _ = tray.set_icon(Some(tauri::image::Image::new_owned(
                                new_rgba, new_w, new_h,
                            )));
                            icon_changed = true;
                            last_non_booting = Some(tray_runtime::TrayRuntimeVisual::Running);
                            last_displayed_dollars = Some(dollars);
                        }
                    }
                    tray_runtime::TrayRuntimeVisual::Off => {
                        if last_non_booting != Some(tray_runtime::TrayRuntimeVisual::Off) {
                            let _ = tray.set_icon(Some(icons.off.clone()));
                            icon_changed = true;
                            last_non_booting = Some(tray_runtime::TrayRuntimeVisual::Off);
                        }
                    }
                    tray_runtime::TrayRuntimeVisual::Paused => {
                        if last_non_booting != Some(tray_runtime::TrayRuntimeVisual::Paused) {
                            let _ = tray.set_icon(Some(icons.paused.clone()));
                            icon_changed = true;
                            last_non_booting = Some(tray_runtime::TrayRuntimeVisual::Paused);
                            last_displayed_dollars = None;
                        }
                    }
                    tray_runtime::TrayRuntimeVisual::Unhealthy => {
                        if last_non_booting != Some(tray_runtime::TrayRuntimeVisual::Unhealthy) {
                            let _ = tray.set_icon(Some(icons.off.clone()));
                            icon_changed = true;
                            last_non_booting = Some(tray_runtime::TrayRuntimeVisual::Unhealthy);
                            last_displayed_dollars = None;
                        }
                    }
                    tray_runtime::TrayRuntimeVisual::Disconnected => {
                        if last_non_booting != Some(tray_runtime::TrayRuntimeVisual::Disconnected) {
                            let _ = tray.set_icon(Some(icons.off.clone()));
                            icon_changed = true;
                            // Only notify when transitioning from a healthy running
                            // state — not on first boot or from other non-running states.
                            if last_non_booting == Some(tray_runtime::TrayRuntimeVisual::Running) {
                                let _ = show_notification_impl(
                                    &app,
                                    "Headroom",
                                    "Claude Code or Codex is disconnected — open Headroom to re-enable.",
                                    Some("connectors".into()),
                                );
                            }
                            last_non_booting = Some(tray_runtime::TrayRuntimeVisual::Disconnected);
                            last_displayed_dollars = None;
                        }
                    }
                }

                // set_icon clobbers the tooltip on macOS, so re-apply whenever
                // we just swapped the icon — not only on tooltip text change.
                let tooltip_changed = last_tooltip.as_deref() != Some(tooltip);
                if icon_changed || tooltip_changed {
                    if let Err(err) = tray.set_tooltip(Some(tooltip)) {
                        log::warn!("tray: set_tooltip failed: {err}");
                    }
                    last_tooltip = Some(tooltip.to_string());
                }
            } else {
                break;
            }

            // Only transitional states need quick polling. In steady state the
            // tray icon is unchanged, and `runtime_status()` is one of the few
            // always-on paths that can still hit the local proxy / filesystem.
            let sleep = match visual {
                tray_runtime::TrayRuntimeVisual::Booting => std::time::Duration::from_millis(260),
                tray_runtime::TrayRuntimeVisual::Unhealthy => {
                    std::time::Duration::from_millis(1500)
                }
                _ => std::time::Duration::from_secs(5),
            };
            std::thread::sleep(sleep);
        }
    });
}

/// Should the watchdog expect the Python proxy to be reachable right now?
///
/// Every 5s, check whether the Python proxy is actually reachable while the
/// app thinks the runtime should be up. If it isn't, try to restart via
/// `ensure_headroom_running`. After 3 consecutive failures (~15s down) we
/// give up: pause the runtime, flip `proxy_bypass=true` so the Rust intercept
/// passes traffic straight through to api.anthropic.com, and notify the user.
/// The user's `~/.claude/settings.json` env, hook, and shell blocks stay
/// intact — `start_headroom` clears bypass and brings Python back up without
/// needing to re-install anything on disk.
fn spawn_proxy_watchdog(app: AppHandle) {
    const POLL: std::time::Duration = std::time::Duration::from_secs(5);
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    // If a tick takes far longer than POLL of wall time, the system was
    // suspended (laptop sleep, App Nap throttle). Don't blame Python for
    // not responding to the first probe after resume — uvicorn's event
    // loop may need a beat to catch up before /readyz answers.
    const RESUME_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(30);

    std::thread::spawn(move || {
        let mut consecutive_failures: u32 = 0;
        // Wall-clock (not `Instant`) timestamp of the previous tick. On macOS
        // `Instant`/`mach_absolute_time` FREEZES while the system is asleep, so
        // a laptop that slept for minutes (common on battery) would measure as
        // only a few seconds of monotonic time and the `just_resumed` guard
        // below would never fire — the watchdog would count the sleep as 3
        // backend failures and auto-pause a perfectly healthy process. The
        // wall clock advances across sleep, so the resume gap is real.
        let mut last_tick_wall = std::time::SystemTime::now();
        // Self-heal scheduling after a give-up auto-pause. `auto_pause_next_retry`
        // is the earliest Instant at which we re-attempt a resume; `auto_pause_failed`
        // counts failed attempts to grow the backoff (see `auto_resume_backoff`).
        let mut auto_pause_next_retry: Option<std::time::Instant> = None;
        let mut auto_pause_failed: u32 = 0;
        // Set after a forced kill+restart of a hung process. Prevents the
        // hung-kill path from looping forever if the new process also hangs:
        // on the second trip through MAX_CONSECUTIVE_FAILURES we fall through
        // to the permanent give-up path instead. Resets when the proxy
        // recovers so a later hang triggers another rescue attempt.
        let mut hung_kill_attempted = false;
        // Fire the one-shot Kompress model prefetch the first time we observe a
        // healthy proxy this launch. `maybe_prefetch_kompress` is itself guarded
        // and no-ops when the model is already cached; this flag just avoids
        // spawning a throwaway thread on every subsequent tick.
        let mut kompress_prefetch_spawned = false;

        loop {
            std::thread::sleep(POLL);
            let now_wall = std::time::SystemTime::now();
            let elapsed = now_wall
                .duration_since(last_tick_wall)
                .unwrap_or(std::time::Duration::ZERO);
            last_tick_wall = now_wall;
            let just_resumed = elapsed > RESUME_THRESHOLD;

            let state: tauri::State<'_, AppState> = app.state();
            let runtime = state.runtime_status();

            // Self-heal: if a previous give-up auto-paused the runtime, keep
            // trying to bring it back on a backoff instead of staying dead
            // until the user intervenes. A deliberate user pause
            // (auto_paused=false) is never retried here. We clear the pause and
            // hard-restart, then let the normal path below own the outcome:
            // it either observes the proxy recover or re-gives-up, which
            // reschedules the next retry with a longer backoff.
            if runtime.auto_paused {
                let due = auto_pause_next_retry
                    .map(|t| std::time::Instant::now() >= t)
                    .unwrap_or(true);
                if due {
                    log::info!(
                        "watchdog: auto-resume attempt (failed_attempts={auto_pause_failed}); killing wedged proxy and restarting"
                    );
                    // Replace the wedged child outright — `resume_runtime` ->
                    // `ensure_headroom_running` no-ops on an alive-but-hung
                    // process (try_wait says running), so a plain resume can't
                    // fix it. stop_headroom SIGKILLs the group and reaps orphans.
                    state.stop_headroom();
                    consecutive_failures = 0;
                    hung_kill_attempted = false;
                    if let Err(err) = state.resume_runtime() {
                        // resume_runtime already cleared the auto_paused flag;
                        // the normal path will re-give-up and reschedule.
                        log::info!("watchdog: auto-resume resume_runtime failed: {err:#}");
                    }
                    auto_pause_next_retry = None;
                }
                continue;
            }

            // Only care when the runtime is supposed to be up: installed,
            // not paused by the user, not mid-boot, not mid-upgrade, and not
            // intentionally bypassed. When `proxy_bypass` is set the pricing
            // gate has stopped Python on purpose; the Rust intercept is
            // routing direct to api.anthropic.com, so trying to restart the
            // backend would just thrash and eventually trip the auto-pause
            // path below.
            let bypass_active = state
                .proxy_bypass
                .load(std::sync::atomic::Ordering::Acquire);
            let should_be_up = runtime_commands::watchdog_should_be_up(
                runtime.installed,
                runtime.paused,
                runtime.starting,
                state.runtime_upgrade_in_progress(),
                bypass_active,
            );
            if !should_be_up {
                if consecutive_failures > 0 {
                    log::debug!(
                        "watchdog: skip restart (installed={}, paused={}, starting={}, upgrading={}, bypass={}); resetting failure counter",
                        runtime.installed,
                        runtime.paused,
                        runtime.starting,
                        state.runtime_upgrade_in_progress(),
                        bypass_active
                    );
                }
                consecutive_failures = 0;
                continue;
            }

            if runtime.proxy_reachable {
                consecutive_failures = 0;
                hung_kill_attempted = false;
                // Healthy again — reset the self-heal backoff so a future
                // wedge starts its retries fresh at 30s.
                auto_pause_failed = 0;
                auto_pause_next_retry = None;
                // End of "down episode" — re-arm Sentry capture so a future
                // crash fires a fresh event.
                WATCHDOG_DOWN_CAPTURED.store(false, Ordering::Release);
                if !kompress_prefetch_spawned {
                    kompress_prefetch_spawned = true;
                    let app_clone = app.clone();
                    std::thread::spawn(move || {
                        let state: tauri::State<'_, AppState> = app_clone.state();
                        state.maybe_prefetch_kompress();
                    });
                }
                continue;
            }

            // System resumed from sleep/throttle — give Python one POLL to
            // catch up before counting failures. Without this, the watchdog
            // probes a still-paged-out uvicorn 3× in 15s and auto-pauses a
            // process that would have recovered on its own.
            if just_resumed {
                log::info!(
                    "watchdog: probe skipped (system resumed after {elapsed:?}); resetting failure counter"
                );
                consecutive_failures = 0;
                continue;
            }

            // Tolerant confirmation before counting a strike. The standard
            // reachability check (`is_headroom_proxy_reachable`) uses a tight
            // 1.5s timeout via the 6767 intercept; the backend runs niced
            // (`nice -n 5`), so under heavy compression/embedding load a
            // perfectly healthy proxy can miss that window. Re-probe the
            // backend's /readyz directly with a 5s budget — if it answers, the
            // process is alive and merely busy, so don't count it as down.
            let tolerant_outcome = runtime_diagnostics::probe_backend_readyz_outcome_with_timeout(
                std::time::Duration::from_secs(5),
            );
            if tolerant_outcome == "ok" {
                log::info!(
                    "watchdog: backend /readyz answered on tolerant 5s re-probe; not counting failure"
                );
                consecutive_failures = 0;
                continue;
            }
            // A 503 whose only failing check is upstream connectivity means the
            // process itself is alive and healthy — only the cached upstream
            // probe is down (network blip / sleep-wake). /readyz is a readiness
            // signal, not a liveness one; don't count it as the process dying.
            if runtime_diagnostics::readyz_failure_is_upstream_only(&tolerant_outcome) {
                log::info!(
                    "watchdog: backend /readyz 503 with only upstream unhealthy (transient connectivity); not counting failure"
                );
                consecutive_failures = 0;
                continue;
            }

            consecutive_failures = consecutive_failures.saturating_add(1);
            log::info!(
                "watchdog: proxy unreachable (failure {consecutive_failures}/{MAX_CONSECUTIVE_FAILURES}, bypass={bypass_active}), attempting restart"
            );

            // If the Python backend is still accepting loopback connections
            // but the client-facing 6767 intercept is gone, restarting Python
            // does not help: clients are pointed at the Rust front door. Respawn
            // the intercept in-process and let the next poll confirm readiness.
            if runtime_probe::proxy_port_accepts_connection()
                && !runtime_probe::intercept_port_accepts_connection()
            {
                state.ensure_proxy_intercept_running();
                consecutive_failures = 0;
                continue;
            }

            if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                // Before pausing, probe the backend directly on its loopback
                // port. `is_headroom_proxy_reachable` goes through the Rust
                // intercept on 6767, which forwards to Python on 6768 with a
                // 1.5s timeout — a slow cold-boot (ONNX embedder downloading
                // model.onnx from huggingface during lifespan startup) can
                // make 6767 time out while the backend was about to recover.
                // If the backend now answers /readyz directly, treat the 3
                // intercept failures as a transient blip rather than a dead
                // process: reset the counter and keep probing. We're already
                // 15s into the down episode, so one extra POLL of patience is
                // cheap compared to auto-pausing a process that just came up.
                let backend_readyz_outcome = runtime_diagnostics::probe_backend_readyz_outcome();
                if backend_readyz_outcome == "ok" {
                    log::info!(
                        "watchdog: backend /readyz answers ok after {consecutive_failures} intercept failures; skipping auto-pause and resetting counter"
                    );
                    consecutive_failures = 0;
                    continue;
                }
                // Upstream-only 503: process alive and answering, only the
                // cached upstream-connectivity probe is failing. Bypassing to
                // Anthropic routes to the same unreachable upstream and buys
                // nothing, and the process self-heals on the next 30s upstream
                // re-check — so keep it up instead of auto-pausing. Backstops
                // the same guard at the tolerant re-probe above.
                if runtime_diagnostics::readyz_failure_is_upstream_only(&backend_readyz_outcome) {
                    log::info!(
                        "watchdog: backend /readyz 503 (upstream-only) after {consecutive_failures} failures; process healthy, skipping auto-pause"
                    );
                    consecutive_failures = 0;
                    continue;
                }
                // Wedged backend: /readyz never responds ("timeout", the event
                // loop is held), or it 503s with a *core* component unhealthy
                // (startup/cache/memory/etc. failed to initialize), or it 503s
                // with a body we couldn't read/parse (bare "http_503" — the
                // status line came back but the body read timed out under load).
                // All three mean the process is alive and answering HTTP but not
                // ready, a state a clean restart may clear. ensure_headroom_running
                // returns Ok immediately when try_wait says the child is still
                // alive, so the three restart attempts above were all no-ops.
                // Kill the stuck process and start fresh before giving up
                // permanently. Once per down episode (hung_kill_attempted) so a
                // persistently-wedged new process doesn't loop; it falls through
                // to the give-up path below.
                if (backend_readyz_outcome == "timeout"
                    || backend_readyz_outcome == "http_503"
                    || runtime_diagnostics::readyz_failure_has_core_unhealthy(
                        &backend_readyz_outcome,
                    ))
                    && !hung_kill_attempted
                {
                    log::info!(
                        "watchdog: backend wedged ({backend_readyz_outcome}) after {consecutive_failures} failures; force-killing and restarting"
                    );
                    hung_kill_attempted = true;
                    state.stop_headroom();
                    consecutive_failures = 0;
                    match state.ensure_headroom_running() {
                        Ok(()) => port_conflict::note_proxy_started(&app),
                        Err(err) => {
                            log::warn!("watchdog: hung-kill restart failed: {err:#}");
                            port_conflict::note_proxy_failed(&app, &err, false);
                        }
                    }
                    continue;
                }
                // Cold-boot rescue. "refused" means the backend port never
                // bound; combined with a tracked child that is still alive,
                // that is the signature of a process mid-cold-boot — uvicorn's
                // lifespan is synchronously pulling multi-GB model weights from
                // HuggingFace (kompress-base, ModernBERT, MiniLM) before it
                // binds. A watchdog-initiated restart spawns via
                // `start_headroom_background`, which returns before /readyz is
                // up and clears `starting` immediately, so the 15s give-up
                // clock ticks against a download that legitimately needs
                // minutes (see Sentry `proxy_unreachable_post_boot`). Hand the
                // child to the same boot-validation loop the launch path uses:
                // it waits out HF-cache growth / CPU / log activity under a
                // 600s ceiling, so a real download survives while a genuine
                // pre-bind hang still stalls out (~90s) and falls through to
                // the auto-pause below. Scoped to "refused" on purpose: a bound
                // "timeout" is the deadlock the hung-kill path already owns, and
                // a bound child would let /livez answer green and thrash this
                // loop forever.
                if backend_readyz_outcome == "refused" && state.tracked_child_alive() {
                    log::info!(
                        "watchdog: backend refused after {consecutive_failures} failures but tracked child is alive; waiting out cold boot before auto-pausing"
                    );
                    let outcome = state.wait_for_boot_validation(|_elapsed, _active| {});
                    if outcome.is_ok() {
                        log::info!(
                            "watchdog: cold boot completed (backend reachable); resetting failure counter"
                        );
                        consecutive_failures = 0;
                        hung_kill_attempted = false;
                        WATCHDOG_DOWN_CAPTURED.store(false, Ordering::Release);
                        continue;
                    }
                    log::info!(
                        "watchdog: cold-boot wait ended without reachability ({}); proceeding to auto-pause",
                        outcome.label()
                    );
                }
                // info! not warn!/error!: this is the documented recovery
                // path (flip bypass, pause runtime, notify user). FileLogger
                // forwards both warn! and error! to Sentry as capture_message,
                // which would produce a payload-less duplicate of the
                // structured event built by capture_watchdog_give_up below —
                // that one already carries the exit status, log tail, and
                // backend probe.
                log::info!(
                    "watchdog: giving up after {MAX_CONSECUTIVE_FAILURES} failures; pausing runtime and bypassing to Anthropic"
                );
                // Capture once per down episode, BEFORE stop_headroom tears
                // down the tracked child and the proxy log handle, so the
                // exit status and log tail reflect the failure we're about
                // to recover from.
                capture_watchdog_give_up(
                    &*state,
                    consecutive_failures,
                    bypass_active,
                    backend_readyz_outcome,
                );
                // Flip bypass FIRST so the Rust intercept passes new
                // requests straight through to Anthropic instead of returning
                // 502 in the window between Python being torn down and the
                // user noticing. See proxy_intercept.rs:161 — without this,
                // every request lands on the unreachable backend branch.
                state
                    .proxy_bypass
                    .store(true, std::sync::atomic::Ordering::Release);
                state.set_runtime_paused(true);
                // Mark this as an AUTO pause (distinct from a user pause) so the
                // self-heal loop above will keep retrying and the UI shows the
                // "stopped unexpectedly" banner with a Resume button.
                state.set_runtime_auto_paused(true);
                state.stop_headroom();
                analytics::track_event(&app, "runtime_auto_paused", None);
                let _ = show_notification_impl(
                    &app,
                    "Mac AI Switchboard paused the engine",
                    "The Headroom engine could not restart its proxy. Requests are passing through unmodified — Mac AI Switchboard will keep retrying automatically, or open the app and hit Resume.",
                    Some("connectors".into()),
                );
                // Arm the self-heal: first retry after 30s, backing off on
                // repeated failures (auto_resume_backoff). The retry runs in the
                // `runtime.auto_paused` branch at the top of the loop.
                auto_pause_next_retry = Some(
                    std::time::Instant::now()
                        + runtime_diagnostics::auto_resume_backoff(auto_pause_failed),
                );
                auto_pause_failed = auto_pause_failed.saturating_add(1);
                consecutive_failures = 0;
                continue;
            }

            // Otherwise try to bring it back.
            match state.ensure_headroom_running() {
                Ok(()) => port_conflict::note_proxy_started(&app),
                Err(err) => {
                    // info! not warn!: FileLogger forwards warn!/error! to
                    // Sentry as a payload-less capture_message. This fires on
                    // every failed retry during a down episode; the structured,
                    // actionable signal is capture_watchdog_give_up above, sent
                    // once per episode after MAX_CONSECUTIVE_FAILURES.
                    log::info!("watchdog: ensure_headroom_running failed: {err:#}");
                    // In-session retry: don't bump the launch counter.
                    port_conflict::note_proxy_failed(&app, &err, false);
                }
            }
        }
    });
}

fn spawn_tray_savings_updater(app: AppHandle) {
    // The tray icon's dollar badge only redraws when the integer value
    // changes (see `changed_dollars` in `spawn_tray_runtime_icon_updater`),
    // so polling faster than the number ticks up is wasted work. 20s is
    // fast enough that the badge feels live during active traffic and slow
    // enough that `build_dashboard` runs ~3x/min instead of 12x/min.
    const INTERVAL: std::time::Duration = std::time::Duration::from_secs(20);
    std::thread::spawn(move || loop {
        std::thread::sleep(INTERVAL);
        let state: tauri::State<'_, AppState> = app.state();
        let dashboard = state.dashboard();
        let today_key = Local::now().format("%Y-%m-%d").to_string();
        let savings: f64 = dashboard
            .hourly_savings
            .iter()
            .filter(|p| p.hour.starts_with(&today_key))
            .map(|p| p.estimated_savings_usd)
            .sum();
        let savings_state: tauri::State<'_, TraySessionSavings> = app.state();
        *savings_state.0.lock() = savings;
        let _ = app.emit("savings-today-updated", savings);
    });
}

fn handle_window_event(window: &Window, event: &WindowEvent) {
    match event {
        WindowEvent::Focused(false) => {
            if window.label() == "main" {
                let window = window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(
                        MAIN_WINDOW_BLUR_HIDE_DELAY_MS,
                    ));

                    let still_unfocused = matches!(window.is_focused(), Ok(false));
                    let still_visible = matches!(window.is_visible(), Ok(true));
                    if still_unfocused && still_visible {
                        let _ = window.hide();
                    }
                });
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = window.hide();
        }
        _ => {}
    }
}

struct TraySessionSavings(Mutex<f64>);

fn toggle_main_window(app: &AppHandle, anchor_rect: Option<Rect>) -> tauri::Result<()> {
    if !onboarding_complete(app) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        show_launcher_window(app)?;
        return Ok(());
    }

    hide_launcher_window(app)?;

    let Some(window) = app.get_webview_window("main") else {
        return Err(tauri::Error::WebviewNotFound);
    };

    if window.is_visible()? {
        window.hide()?;
    } else {
        show_main_window(app, anchor_rect)?;
        // Start/verify headroom in the background so the window appears immediately.
        let app_bg = app.clone();
        std::thread::spawn(move || ensure_runtime_ready_for_tray(&app_bg));
    }

    Ok(())
}

fn ensure_runtime_ready_for_tray(app: &AppHandle) {
    let state: tauri::State<'_, AppState> = app.state();
    if state.runtime_is_paused() {
        return;
    }
    match state.ensure_headroom_running() {
        Ok(()) => port_conflict::note_proxy_started(app),
        Err(err) => {
            // Tray open is in-session (not a fresh launch); pass false so the
            // launch counter is preserved instead of double-counting clicks.
            let handled = port_conflict::note_proxy_failed(app, &err, false);
            if !handled {
                capture_headroom_start_failure("ensure_runtime_ready_for_tray failed", &err);
            }
        }
    }
}

fn onboarding_complete(app: &AppHandle) -> bool {
    let state: tauri::State<'_, AppState> = app.state();
    if !state.tool_manager.python_runtime_installed() {
        return false;
    }
    // Only require wizard completion on the very first launch. Existing users
    // (launch_count > 1) already went through setup before this flag existed.
    state.setup_wizard_complete() || state.launch_count() > 1
}

#[tauri::command]
fn complete_setup_wizard(state: tauri::State<'_, AppState>) {
    state.mark_setup_wizard_complete();
}

#[tauri::command]
async fn accept_terms(app: AppHandle, version: u32) {
    // Local acceptance is the authoritative gate (works offline / pre-signin).
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.mark_terms_accepted(version);
    }
    // Best-effort: tell the server now. `fetch_grace_start` is blocking, so
    // run it off the IPC thread; failures are swallowed and the value rides
    // along on the next identity push regardless.
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app.state();
        crate::pricing::push_terms_acceptance(&state, version);
    });
}

fn show_main_window(app: &AppHandle, anchor_rect: Option<Rect>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Err(tauri::Error::WebviewNotFound);
    };

    if let Some(rect) = anchor_rect {
        tray_window::position_tray_window(&window, rect)?;
    }

    window.show()?;
    let _ = window.unminimize();
    window.set_focus()?;
    Ok(())
}

fn show_launcher_window(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("launcher") else {
        return Err(tauri::Error::WebviewNotFound);
    };

    let _ = window.center();
    window.show()?;
    let _ = window.unminimize();
    let _ = window.center();
    window.set_focus()?;
    Ok(())
}

fn hide_launcher_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("launcher") {
        if window.is_visible()? {
            window.hide()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        app_quit_requested_properties, classify_upgrade_error, is_disk_full_signal,
        is_endpoint_protection_signal, is_port_conflict_failure, QuitSource,
    };
    use crate::activity_commands::{
        count_memories_created_today, fetch_transformations_feed_from,
        parse_request_count_from_stats_body, parse_request_counts_by_agent,
    };
    use crate::app_services_commands::{
        reject_contact_request_in_local_only, validate_contact_request_url,
    };
    // Local-only contact guard certification signal:
    // reject_contact_request_in_local_only()? returns
    // "Support/contact requests are disabled in local-only mode."
    use crate::app_update_commands::{
        app_update_notification_body, beta_channel_enabled_from, build_release_updater_config,
        install_pending_update, is_prerelease_version, noop_app_update_progress_emitter,
        parse_updater_endpoint_list, resolve_release_updater_config, select_updater_endpoints,
        store_checked_update, AppUpdateProgress, AppUpdateProgressEmitter, AvailableAppUpdate,
        InstallPendingUpdateFuture, InstallableAppUpdate,
    };
    use crate::dashboard_commands::{lifetime_token_milestone_kind, zero_spend_affected_days};
    use crate::learning_commands::{
        aggregate_live_learnings, check_headroom_learn_prereqs, delete_applied_pattern,
        empty_live_learnings_for_projects, extract_llm_failure_warnings, parse_live_learnings,
        pattern_matches_project, read_applied_patterns_for_project, LearnAgent,
    };
    use crate::models::{
        DailySavingsPoint, HeadroomLearnPrereqStatus, ManagedRollbackExecutionStatus,
        RepoFileIndexEntry, RepoIndexMetadata, RepoIntelligenceSummary,
    };
    use crate::repo_intelligence;
    use crate::runtime_commands::watchdog_should_be_up;
    use crate::runtime_diagnostics::{
        auto_resume_backoff, build_watchdog_give_up_report, classify_bootstrap_failure,
        cpu_rate_indicates_burn, is_network_download_signal, readyz_failed_checks_csv,
        readyz_failure_has_core_unhealthy, readyz_failure_is_upstream_only, BootstrapFailureKind,
    };
    use crate::state::AppState;
    use crate::tray_runtime::{debounced_tray_runtime_visual, TrayRuntimeVisual};
    use crate::tray_window::{
        compute_tray_window_position, physical_rect_from_rect, MonitorBounds, PhysicalRect,
    };
    use chrono::{TimeZone, Utc};
    use parking_lot::Mutex;
    use serde_json::{json, Value};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    struct LocalOnlyEnvGuard {
        prev_local: Option<std::ffi::OsString>,
        prev_remote: Option<std::ffi::OsString>,
    }

    impl LocalOnlyEnvGuard {
        fn enabled() -> Self {
            let prev_local = std::env::var_os("HEADROOM_LOCAL_ONLY");
            let prev_remote = std::env::var_os("HEADROOM_REMOTE_SERVICES");
            std::env::set_var("HEADROOM_LOCAL_ONLY", "1");
            std::env::remove_var("HEADROOM_REMOTE_SERVICES");
            Self {
                prev_local,
                prev_remote,
            }
        }
    }

    impl Drop for LocalOnlyEnvGuard {
        fn drop(&mut self) {
            match self.prev_local.take() {
                Some(value) => std::env::set_var("HEADROOM_LOCAL_ONLY", value),
                None => std::env::remove_var("HEADROOM_LOCAL_ONLY"),
            }
            match self.prev_remote.take() {
                Some(value) => std::env::set_var("HEADROOM_REMOTE_SERVICES", value),
                None => std::env::remove_var("HEADROOM_REMOTE_SERVICES"),
            }
        }
    }

    struct AppStorageEnvGuard {
        prev_xdg: Option<std::ffi::OsString>,
        prev_home: Option<std::ffi::OsString>,
    }

    impl AppStorageEnvGuard {
        fn isolated(root: &std::path::Path) -> Self {
            let prev_xdg = std::env::var_os("XDG_DATA_HOME");
            let prev_home = std::env::var_os("HOME");
            std::env::set_var("XDG_DATA_HOME", root);
            std::env::set_var("HOME", root);
            Self {
                prev_xdg,
                prev_home,
            }
        }
    }

    impl Drop for AppStorageEnvGuard {
        fn drop(&mut self) {
            match self.prev_xdg.take() {
                Some(value) => std::env::set_var("XDG_DATA_HOME", value),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
            match self.prev_home.take() {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
    }
    use tauri::{LogicalPosition, LogicalSize, PhysicalSize, Position, Rect, Size};

    const TEST_UPDATER_PUBLIC_KEY: &str = "test-updater-public-key";

    struct FakePendingUpdate {
        metadata: AvailableAppUpdate,
        install_result: Result<(), String>,
    }

    impl InstallableAppUpdate for FakePendingUpdate {
        fn metadata(&self) -> AvailableAppUpdate {
            self.metadata.clone()
        }

        fn install(self, _progress: AppUpdateProgressEmitter) -> InstallPendingUpdateFuture {
            Box::pin(async move { self.install_result })
        }
    }

    fn sample_available_update(version: &str) -> AvailableAppUpdate {
        AvailableAppUpdate {
            current_version: "0.2.9".into(),
            version: version.into(),
            published_at: Some("2026-04-02T12:00:00Z".into()),
            notes: Some("Bug fixes.".into()),
        }
    }

    fn daily_point(
        date: &str,
        savings_usd: f64,
        tokens_saved: u64,
        cost_usd: f64,
        tokens_sent: u64,
    ) -> DailySavingsPoint {
        DailySavingsPoint {
            date: date.into(),
            estimated_savings_usd: savings_usd,
            estimated_tokens_saved: tokens_saved,
            actual_cost_usd: cost_usd,
            total_tokens_sent: tokens_sent,
        }
    }

    fn repo_summary_fixture(repo_root: String, indexed_at: &str) -> RepoIntelligenceSummary {
        RepoIntelligenceSummary {
            indexed_at: indexed_at.to_string(),
            repo_root,
            indexer_version: Some(repo_intelligence::current_indexer_version().to_string()),
            total_files: 1,
            indexed_files: 1,
            skipped_files: 0,
            estimated_full_scan_tokens: 10,
            role_counts: BTreeMap::new(),
            index_metadata: Some(RepoIndexMetadata {
                schema_version: 1,
                indexer_version: repo_intelligence::current_indexer_version().to_string(),
                parser_version: "metadata-fingerprint-v1".to_string(),
                cache_key: "test".to_string(),
                cache_state: "unchanged".to_string(),
                generated_at: indexed_at.to_string(),
                previous_indexed_at: None,
                file_count: 1,
                indexed_file_count: 1,
                skipped_file_count: 0,
                file_fingerprints: vec![RepoFileIndexEntry {
                    path: "src/App.tsx".to_string(),
                    bytes: 10,
                    modified_unix_ms: 0,
                    fingerprint: "abc123".to_string(),
                }],
                skipped_files: Vec::new(),
                graph_inputs: Vec::new(),
            }),
            graph: None,
            packs: Vec::new(),
        }
    }

    #[test]
    fn repo_intelligence_doctor_issue_reports_missing_moved_and_healthy_indexes() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 28, 12, 0, 0)
            .single()
            .expect("valid time");
        let missing = repo_summary_fixture(
            "/tmp/mac-ai-switchboard-missing-repo-for-doctor".to_string(),
            "2026-06-28T10:00:00Z",
        );
        let missing_issue =
            crate::doctor::repo_intelligence_doctor_issue(&missing, now).expect("missing issue");
        assert_eq!(missing_issue.id, "repo_intelligence_repo_missing");
        assert_eq!(
            missing_issue.repair_action.as_deref(),
            Some("clear_repo_intelligence_index")
        );

        let moved_root = tempfile::tempdir().expect("create moved repo root");
        let moved = repo_summary_fixture(
            moved_root.path().to_string_lossy().to_string(),
            "2026-06-28T10:00:00Z",
        );
        let moved_issue =
            crate::doctor::repo_intelligence_doctor_issue(&moved, now).expect("moved issue");
        assert_eq!(moved_issue.id, "repo_intelligence_repo_moved");
        assert!(moved_issue.body.contains("file map no longer matches"));
        assert_eq!(
            moved_issue.repair_action.as_deref(),
            Some("clear_repo_intelligence_index")
        );

        std::fs::create_dir_all(moved_root.path().join("src")).expect("create src");
        std::fs::write(moved_root.path().join("src/App.tsx"), "export {}\n")
            .expect("write indexed file");
        assert!(
            crate::doctor::repo_intelligence_doctor_issue(&moved, now).is_none(),
            "existing indexed file should keep the saved index healthy"
        );

        let mut missing_metadata = moved.clone();
        missing_metadata.index_metadata = None;
        let missing_metadata_issue =
            crate::doctor::repo_intelligence_doctor_issue(&missing_metadata, now)
                .expect("metadata issue");
        assert_eq!(missing_metadata_issue.id, "repo_intelligence_index_health");
        assert!(missing_metadata_issue.body.contains("metadata_missing"));
        assert!(missing_metadata_issue.body.contains("unavailable"));

        let mut parser_mismatch = moved.clone();
        parser_mismatch
            .index_metadata
            .as_mut()
            .expect("fixture metadata")
            .parser_version = "older-parser-v0".to_string();
        let parser_mismatch_issue =
            crate::doctor::repo_intelligence_doctor_issue(&parser_mismatch, now)
                .expect("parser issue");
        assert_eq!(parser_mismatch_issue.id, "repo_intelligence_index_health");
        assert!(parser_mismatch_issue.body.contains("version_mismatch"));

        let mut indexer_mismatch = moved.clone();
        indexer_mismatch.indexer_version = Some("path-graph-v2".to_string());
        let indexer_mismatch_issue =
            crate::doctor::repo_intelligence_doctor_issue(&indexer_mismatch, now)
                .expect("indexer issue");
        assert_eq!(indexer_mismatch_issue.id, "repo_intelligence_index_health");
        assert!(indexer_mismatch_issue.body.contains("indexer health"));
        assert!(indexer_mismatch_issue.body.contains("version_mismatch"));
    }

    #[test]
    #[serial_test::serial]
    fn clear_repo_intelligence_index_repairs_corrupt_saved_summary() {
        let scratch = tempfile::tempdir().expect("scratch");
        let _guard = AppStorageEnvGuard::isolated(scratch.path());
        let path = crate::storage::config_file(
            &crate::storage::app_data_dir(),
            "repo-intelligence-latest.json",
        );
        std::fs::create_dir_all(path.parent().expect("summary parent"))
            .expect("create repo intelligence config dir");
        std::fs::write(&path, b"{not valid json").expect("write corrupt summary");

        let corrupt = crate::repo_intelligence::load_latest_summary()
            .expect_err("corrupt summary should be unreadable");
        assert!(corrupt
            .to_string()
            .contains("parsing repo intelligence summary"));

        crate::switchboard_commands::clear_repo_intelligence_index()
            .expect("clear corrupt repo index");

        assert!(!path.exists());
        assert!(crate::repo_intelligence::load_latest_summary()
            .expect("cleared summary should read as none")
            .is_none());
    }

    #[test]
    #[serial_test::serial]
    fn dedicated_cleanup_rollback_clears_only_repo_intelligence_summary() {
        let scratch = tempfile::tempdir().expect("scratch");
        let _guard = AppStorageEnvGuard::isolated(scratch.path());
        let repo = tempfile::tempdir().expect("repo");
        let source = repo.path().join("src").join("App.tsx");
        std::fs::create_dir_all(source.parent().expect("source parent"))
            .expect("create source parent");
        std::fs::write(&source, "export const untouched = true;\n").expect("write source");
        let before = std::fs::read_to_string(&source).expect("read source before");
        let summary = repo_summary_fixture(
            repo.path().to_string_lossy().to_string(),
            "2026-06-28T10:00:00Z",
        );
        crate::repo_intelligence::save_latest_summary(&summary).expect("save summary");

        let preview = crate::dedicated_cleanup_rollback::preview_dedicated_cleanup_rollback_inner(
            None,
            "repo-intelligence".to_string(),
        )
        .expect("preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(
            preview.confirmation_phrase,
            "Clear repo-intelligence-latest.json for Repo Intelligence"
        );
        assert!(preview.marker_present);
        assert!(preview.backup_path.is_none());
        assert!(preview
            .evidence
            .join(" ")
            .contains("User repositories are not modified"));

        let result = crate::dedicated_cleanup_rollback::execute_dedicated_cleanup_rollback_inner(
            None,
            "repo-intelligence".to_string(),
            "Clear repo-intelligence-latest.json for Repo Intelligence".to_string(),
        )
        .expect("execute cleanup");
        assert_eq!(result.record_id, "repo-intelligence");
        assert_eq!(
            result.restored_from,
            "Switchboard-managed Repo Intelligence latest-summary metadata removed."
        );
        assert!(result.safety_backup_path.is_none());
        assert!(result
            .verification
            .join(" ")
            .contains("User repositories were not modified"));
        assert!(!crate::repo_intelligence::latest_summary_path().exists());
        assert_eq!(
            std::fs::read_to_string(&source).expect("read source after"),
            before
        );
    }

    #[test]
    #[serial_test::serial]
    fn dedicated_cleanup_rollback_removes_managed_launch_agents_only() {
        let scratch = tempfile::tempdir().expect("scratch");
        let _guard = AppStorageEnvGuard::isolated(scratch.path());
        let launch_agents = scratch.path().join("Library").join("LaunchAgents");
        std::fs::create_dir_all(&launch_agents).expect("create launch agents");
        let managed = launch_agents.join("com.tarunagarwal.mac-ai-switchboard.plist");
        let legacy = launch_agents.join("Headroom.plist");
        let unrelated = launch_agents.join("com.example.other.plist");
        std::fs::write(&managed, "<plist>managed</plist>\n").expect("write managed");
        std::fs::write(&legacy, "<plist>legacy</plist>\n").expect("write legacy");
        std::fs::write(&unrelated, "<plist>keep</plist>\n").expect("write unrelated");

        let preview = crate::dedicated_cleanup_rollback::preview_dedicated_cleanup_rollback_inner(
            None,
            "login-item".to_string(),
        )
        .expect("preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(
            preview.confirmation_phrase,
            "Remove com.tarunagarwal.mac-ai-switchboard LaunchAgent for Launch at login"
        );
        assert!(preview.target_path.contains("com.tarunagarwal"));
        assert!(preview.target_path.contains("Headroom.plist"));
        assert!(preview
            .evidence
            .join(" ")
            .contains("~/Library/LaunchAgents"));

        let result = crate::dedicated_cleanup_rollback::execute_dedicated_cleanup_rollback_inner(
            None,
            "login-item".to_string(),
            "Remove com.tarunagarwal.mac-ai-switchboard LaunchAgent for Launch at login"
                .to_string(),
        )
        .expect("execute cleanup");
        assert_eq!(result.record_id, "login-item");
        assert!(result.restored_from.contains("LaunchAgent"));
        assert!(result
            .verification
            .join(" ")
            .contains("No shell, client, repo, Keychain, or runtime storage"));
        assert!(!managed.exists());
        assert!(!legacy.exists());
        assert!(unrelated.exists());
    }

    #[test]
    #[serial_test::serial]
    fn dedicated_cleanup_rollback_removes_ponytail_receipt_only() {
        let scratch = tempfile::tempdir().expect("scratch");
        let _guard = AppStorageEnvGuard::isolated(scratch.path());
        let state = AppState::new_in(scratch.path().join("state")).expect("app state");
        let tools_dir = state
            .tool_manager
            .managed_python()
            .ancestors()
            .nth(4)
            .expect("managed python under headroom root")
            .join("tools");
        std::fs::create_dir_all(&tools_dir).expect("create tools");
        let ponytail_receipt = tools_dir.join("ponytail.json");
        let unrelated = tools_dir.join("markitdown.json");
        std::fs::write(&ponytail_receipt, br#"{"version":"latest","enabled":true}"#)
            .expect("write ponytail receipt");
        std::fs::write(&unrelated, br#"{"version":"keep"}"#).expect("write unrelated receipt");

        let preview = crate::dedicated_cleanup_rollback::preview_dedicated_cleanup_rollback_inner(
            Some(&state),
            "plugins-backups".to_string(),
        )
        .expect("preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(
            preview.confirmation_phrase,
            "Remove headroom:addon for Add-ons"
        );
        assert!(preview
            .evidence
            .join(" ")
            .contains("no-op without the app receipt"));

        let result = crate::dedicated_cleanup_rollback::execute_dedicated_cleanup_rollback_inner(
            Some(&state),
            "plugins-backups".to_string(),
            "Remove headroom:addon for Add-ons".to_string(),
        )
        .expect("execute cleanup");
        assert_eq!(result.record_id, "plugins-backups");
        assert!(result.restored_from.contains("Ponytail"));
        assert!(!ponytail_receipt.exists());
        assert!(unrelated.exists());
        assert!(result
            .verification
            .join(" ")
            .contains("No add-on backup files were swept"));
    }

    #[test]
    #[serial_test::serial]
    fn dedicated_cleanup_rollback_removes_managed_runtime_storage_only() {
        let scratch = tempfile::tempdir().expect("scratch");
        let _guard = AppStorageEnvGuard::isolated(scratch.path());
        let app_dir = crate::storage::app_data_dir();
        let legacy_dir = app_dir
            .parent()
            .expect("app data parent")
            .join(crate::storage::LEGACY_STORAGE_DIR_NAME);
        let dot_headroom = scratch.path().join(".headroom");
        let preferences = scratch
            .path()
            .join("Library")
            .join("Preferences")
            .join("com.tarunagarwal.mac-ai-switchboard.plist");
        std::fs::create_dir_all(&app_dir).expect("create app storage");
        std::fs::write(app_dir.join("runtime.json"), "{}").expect("write app storage");
        std::fs::create_dir_all(&legacy_dir).expect("create legacy storage");
        std::fs::write(legacy_dir.join("legacy.json"), "{}").expect("write legacy storage");
        std::fs::create_dir_all(&dot_headroom).expect("create dot runtime");
        std::fs::write(dot_headroom.join("receipt.json"), "{}").expect("write dot runtime");
        std::fs::create_dir_all(preferences.parent().expect("prefs parent")).expect("create prefs");
        std::fs::write(&preferences, "prefs").expect("write prefs");

        let preview = crate::dedicated_cleanup_rollback::preview_dedicated_cleanup_rollback_inner(
            None,
            "managed-storage".to_string(),
        )
        .expect("preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(
            preview.confirmation_phrase,
            "Delete managed storage for Mac AI Switchboard runtime"
        );

        let result = crate::dedicated_cleanup_rollback::execute_dedicated_cleanup_rollback_inner(
            None,
            "managed-storage".to_string(),
            preview.confirmation_phrase,
        )
        .expect("execute cleanup");

        assert!(!app_dir.exists());
        assert!(!legacy_dir.exists());
        assert!(!dot_headroom.exists());
        assert!(preferences.exists());
        assert!(result
            .verification
            .join(" ")
            .contains("App support storage"));
    }

    #[test]
    #[serial_test::serial]
    fn dedicated_cleanup_rollback_removes_app_state_only() {
        let scratch = tempfile::tempdir().expect("scratch");
        let _guard = AppStorageEnvGuard::isolated(scratch.path());
        let app_dir = crate::storage::app_data_dir();
        std::fs::create_dir_all(&app_dir).expect("create app storage");
        std::fs::write(app_dir.join("runtime.json"), "{}").expect("write app storage");
        let library = scratch.path().join("Library");
        let preferences = library
            .join("Preferences")
            .join("com.tarunagarwal.mac-ai-switchboard.plist");
        let caches = library
            .join("Caches")
            .join("com.tarunagarwal.mac-ai-switchboard");
        let logs = library.join("Logs").join("Mac AI Switchboard");
        let webkit = library
            .join("WebKit")
            .join("com.tarunagarwal.mac-ai-switchboard");
        for path in [
            preferences.clone(),
            caches.join("cache.db"),
            logs.join("app.log"),
            webkit.join("data"),
        ] {
            std::fs::create_dir_all(path.parent().expect("state parent"))
                .expect("create state parent");
            std::fs::write(path, "state").expect("write state file");
        }

        let preview = crate::dedicated_cleanup_rollback::preview_dedicated_cleanup_rollback_inner(
            None,
            "app-state".to_string(),
        )
        .expect("preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(
            preview.confirmation_phrase,
            "Delete com.tarunagarwal.mac-ai-switchboard app state"
        );

        let result = crate::dedicated_cleanup_rollback::execute_dedicated_cleanup_rollback_inner(
            None,
            "app-state".to_string(),
            preview.confirmation_phrase,
        )
        .expect("execute cleanup");

        assert!(!preferences.exists());
        assert!(!caches.exists());
        assert!(!logs.exists());
        assert!(!webkit.exists());
        assert!(app_dir.exists());
        assert!(result
            .verification
            .join(" ")
            .contains("App-state cleanup completed"));
    }

    #[test]
    fn zero_spend_ignores_days_with_only_cli_filtering_savings() {
        // CLI/RTK filtering inflates the token total but never the compression
        // dollar figure (those tokens never reach a model request), so a day with
        // token savings but zero compression-USD is not an anomaly.
        let days = vec![daily_point("2026-06-16", 0.0, 5_000, 0.0, 0)];
        assert!(zero_spend_affected_days(&days).is_empty());
    }

    #[test]
    fn zero_spend_flags_compression_savings_with_no_spend() {
        // Compression dollars recorded but the spend pipeline reported nothing.
        let days = vec![daily_point("2026-06-16", 0.12, 5_000, 0.0, 0)];
        assert_eq!(zero_spend_affected_days(&days), vec!["2026-06-16"]);
    }

    #[test]
    fn zero_spend_ignores_compression_days_that_recorded_spend() {
        let days = vec![daily_point("2026-06-16", 0.12, 5_000, 0.34, 8_000)];
        assert!(zero_spend_affected_days(&days).is_empty());
    }

    #[test]
    fn zero_spend_ignores_pre_schema_cutoff_days() {
        // Pre-v6 records deserialize spend fields as 0; never flag them.
        let days = vec![daily_point("2026-04-12", 0.12, 5_000, 0.0, 0)];
        assert!(zero_spend_affected_days(&days).is_empty());
    }

    #[test]
    fn app_quit_requested_properties_include_source_and_runtime_state() {
        assert_eq!(
            app_quit_requested_properties(QuitSource::SettingsButton, false),
            json!({
                "source": "settings_button",
                "runtime_paused": false,
            })
        );
        assert_eq!(
            app_quit_requested_properties(QuitSource::TrayMenu, true),
            json!({
                "source": "tray_menu",
                "runtime_paused": true,
            })
        );
    }

    #[test]
    fn tray_visual_keeps_running_during_brief_unhealthy_probe_blips() {
        let mut unhealthy_streak = 0;

        for _ in 0..7 {
            assert_eq!(
                debounced_tray_runtime_visual(
                    TrayRuntimeVisual::Unhealthy,
                    Some(TrayRuntimeVisual::Running),
                    &mut unhealthy_streak,
                ),
                TrayRuntimeVisual::Running
            );
        }

        assert_eq!(
            debounced_tray_runtime_visual(
                TrayRuntimeVisual::Unhealthy,
                Some(TrayRuntimeVisual::Running),
                &mut unhealthy_streak,
            ),
            TrayRuntimeVisual::Unhealthy
        );
    }

    #[test]
    fn tray_visual_resets_unhealthy_streak_after_recovery() {
        let mut unhealthy_streak = 0;

        assert_eq!(
            debounced_tray_runtime_visual(
                TrayRuntimeVisual::Unhealthy,
                Some(TrayRuntimeVisual::Running),
                &mut unhealthy_streak,
            ),
            TrayRuntimeVisual::Running
        );
        assert_eq!(
            debounced_tray_runtime_visual(
                TrayRuntimeVisual::Running,
                Some(TrayRuntimeVisual::Running),
                &mut unhealthy_streak,
            ),
            TrayRuntimeVisual::Running
        );
        assert_eq!(unhealthy_streak, 0);
    }

    #[test]
    fn updater_endpoint_parser_accepts_json_arrays() {
        let parsed = parse_updater_endpoint_list(
            r#"["https://updates.example.com/latest.json", " https://backup.example.com/feed "]"#,
        )
        .expect("json endpoint list");

        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed[0].as_str(),
            "https://updates.example.com/latest.json"
        );
        assert_eq!(parsed[1].as_str(), "https://backup.example.com/feed");
    }

    #[test]
    fn updater_endpoint_parser_accepts_comma_or_newline_lists() {
        let parsed = parse_updater_endpoint_list(
            "https://updates.example.com/latest.json,\nhttps://backup.example.com/feed",
        )
        .expect("delimited endpoint list");

        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed[0].as_str(),
            "https://updates.example.com/latest.json"
        );
        assert_eq!(parsed[1].as_str(), "https://backup.example.com/feed");
    }

    #[test]
    fn updater_endpoint_parser_rejects_empty_or_insecure_values() {
        let empty = parse_updater_endpoint_list(" \n , ").expect_err("empty list should fail");
        assert!(empty.contains("HEADROOM_UPDATER_ENDPOINTS"));

        let insecure = parse_updater_endpoint_list("http://updates.example.com/latest.json")
            .expect_err("http endpoint should fail");
        assert!(insecure.contains("must use HTTPS"));
    }

    #[test]
    #[test]
    #[serial_test::serial]
    fn local_only_blocks_contact_request_before_url_or_email_validation() {
        let _local_only = LocalOnlyEnvGuard::enabled();
        let err =
            reject_contact_request_in_local_only().expect_err("local-only blocks contact requests");

        assert_eq!(
            err,
            "Support/contact requests are disabled in local-only mode."
        );
    }

    #[test]
    fn contact_request_url_validator_rejects_ssrf_and_injection_shapes() {
        assert!(validate_contact_request_url(
            "https://github.com/tarunag10/mac-ai-switchboard/issues",
        )
        .is_some());
        for raw in [
            "http://github.com/tarunag10/mac-ai-switchboard/issues",
            "https://127.0.0.1/contact",
            "https://localhost/contact",
            "https://10.0.0.4/contact",
            "https://user:pass@github.com/tarunag10/mac-ai-switchboard/issues",
            "https://github.com.evil.example/contact",
            "https://github.com/tarunag10/mac-ai-switchboard/issues\nhttps://evil.example",
        ] {
            assert!(
                validate_contact_request_url(raw).is_none(),
                "{raw} should be rejected"
            );
        }
    }

    #[test]
    fn connector_smoke_shell_command_uses_login_shell_safe_fixed_prompts() {
        assert_eq!(
            crate::connector_smoke::shell_single_quote("don't drift"),
            "'don'\"'\"'t drift'"
        );
        let codex = crate::connector_smoke::connector_smoke_shell_command("codex", "say it's ok")
            .expect("codex smoke supported");
        assert!(codex.starts_with("codex exec --ephemeral --sandbox read-only"));
        assert!(codex.contains("--skip-git-repo-check --ignore-rules"));
        assert!(codex.ends_with("'say it'\"'\"'s ok'"));

        let claude = crate::connector_smoke::connector_smoke_shell_command("claude_code", "verify")
            .expect("claude smoke supported");
        assert!(claude.starts_with("claude --print --no-session-persistence"));
        assert!(claude.contains("--tools '' --output-format text 'verify'"));
        assert!(
            crate::connector_smoke::connector_smoke_shell_command("cursor", "verify").is_none()
        );
    }

    #[test]
    fn prerelease_versions_are_detected() {
        assert!(is_prerelease_version("0.2.44-rc.1"));
        assert!(is_prerelease_version("0.2.44-staging"));
        assert!(!is_prerelease_version("0.2.44"));
        assert!(!is_prerelease_version("1.0.0"));
    }

    #[test]
    fn beta_channel_enabled_from_recognises_truthy_env_values() {
        assert!(beta_channel_enabled_from(Some("1"), false));
        assert!(beta_channel_enabled_from(Some("true"), false));
        assert!(beta_channel_enabled_from(Some("TRUE"), false));
        assert!(beta_channel_enabled_from(Some(" yes "), false));
    }

    #[test]
    fn beta_channel_enabled_from_rejects_other_env_values() {
        assert!(!beta_channel_enabled_from(None, false));
        assert!(!beta_channel_enabled_from(Some(""), false));
        assert!(!beta_channel_enabled_from(Some("0"), false));
        assert!(!beta_channel_enabled_from(Some("false"), false));
        assert!(!beta_channel_enabled_from(Some("no"), false));
    }

    #[test]
    fn beta_channel_enabled_from_honours_sentinel_file() {
        assert!(beta_channel_enabled_from(None, true));
        assert!(beta_channel_enabled_from(Some("0"), true));
    }

    #[test]
    fn select_updater_endpoints_uses_stable_when_not_preferring_staging() {
        assert_eq!(
            select_updater_endpoints(Some("https://stable"), Some("https://staging"), false),
            Some("https://stable")
        );
        assert_eq!(
            select_updater_endpoints(Some("https://stable"), None, false),
            Some("https://stable")
        );
        assert_eq!(
            select_updater_endpoints(None, Some("https://staging"), false),
            None
        );
    }

    #[test]
    fn select_updater_endpoints_prefers_staging_when_available() {
        assert_eq!(
            select_updater_endpoints(Some("https://stable"), Some("https://staging"), true),
            Some("https://staging")
        );
    }

    #[test]
    fn select_updater_endpoints_falls_back_to_stable_when_staging_missing() {
        assert_eq!(
            select_updater_endpoints(Some("https://stable"), None, true),
            Some("https://stable")
        );
        assert_eq!(select_updater_endpoints(None, None, true), None);
    }

    #[test]
    fn resolve_release_updater_config_picks_stable_for_stable_version_with_beta_off() {
        let config = resolve_release_updater_config(
            "0.3.0",
            false,
            Some(TEST_UPDATER_PUBLIC_KEY),
            Some("https://stable.example.com/latest.json"),
            Some("https://staging.example.com/latest.json"),
            false,
        )
        .expect("config")
        .expect("Some(config)");

        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(
            config.endpoints[0].as_str(),
            "https://stable.example.com/latest.json"
        );
    }

    #[test]
    fn resolve_release_updater_config_picks_staging_when_beta_channel_on() {
        let config = resolve_release_updater_config(
            "0.3.0",
            true,
            Some(TEST_UPDATER_PUBLIC_KEY),
            Some("https://stable.example.com/latest.json"),
            Some("https://staging.example.com/latest.json"),
            false,
        )
        .expect("config")
        .expect("Some(config)");

        assert_eq!(
            config.endpoints[0].as_str(),
            "https://staging.example.com/latest.json"
        );
    }

    #[test]
    fn resolve_release_updater_config_picks_staging_for_prerelease_even_with_beta_off() {
        let config = resolve_release_updater_config(
            "0.3.1-rc.2",
            false,
            Some(TEST_UPDATER_PUBLIC_KEY),
            Some("https://stable.example.com/latest.json"),
            Some("https://staging.example.com/latest.json"),
            false,
        )
        .expect("config")
        .expect("Some(config)");

        assert_eq!(
            config.endpoints[0].as_str(),
            "https://staging.example.com/latest.json"
        );
    }

    #[test]
    fn resolve_release_updater_config_falls_back_to_stable_when_staging_unconfigured() {
        let config = resolve_release_updater_config(
            "0.3.0",
            true,
            Some(TEST_UPDATER_PUBLIC_KEY),
            Some("https://stable.example.com/latest.json"),
            None,
            false,
        )
        .expect("config")
        .expect("Some(config)");

        assert_eq!(
            config.endpoints[0].as_str(),
            "https://stable.example.com/latest.json"
        );
    }

    #[test]
    fn resolve_release_updater_config_disables_updates_when_unconfigured_in_release() {
        let config = resolve_release_updater_config("0.3.0", false, None, None, None, false)
            .expect("config");

        assert!(config.is_none());
    }

    #[test]
    fn resolve_release_updater_config_disables_updates_in_debug_when_unconfigured() {
        let result = resolve_release_updater_config("0.3.0", true, None, None, None, true)
            .expect("debug config resolves to None");
        assert!(result.is_none());
    }

    #[test]
    fn resolve_release_updater_config_errors_when_pubkey_missing() {
        let err = resolve_release_updater_config(
            "0.3.0",
            false,
            None,
            Some("https://stable.example.com/latest.json"),
            None,
            false,
        )
        .expect_err("missing pubkey error");
        assert!(err.contains("HEADROOM_UPDATER_PUBLIC_KEY"));
    }

    #[test]
    fn resolve_release_updater_config_errors_when_endpoints_missing() {
        let err = resolve_release_updater_config(
            "0.3.0",
            false,
            Some(TEST_UPDATER_PUBLIC_KEY),
            None,
            None,
            false,
        )
        .expect_err("missing endpoints error");
        assert!(err.contains("HEADROOM_UPDATER_ENDPOINTS"));
    }

    #[test]
    fn updater_release_config_accepts_explicit_feed() {
        let config = build_release_updater_config(
            TEST_UPDATER_PUBLIC_KEY,
            "https://updates.example.com/latest.json",
        )
        .expect("explicit updater config");

        assert_eq!(config.pubkey, TEST_UPDATER_PUBLIC_KEY);
        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(
            config.endpoints[0].as_str(),
            "https://updates.example.com/latest.json"
        );
    }

    #[test]
    fn app_update_notification_body_mentions_the_target_version() {
        assert_eq!(
            app_update_notification_body("0.3.0"),
            "AI Switchboard for Mac 0.3.0 is ready to install. Open AI Switchboard for Mac to review the release and install it."
        );
        assert_eq!(
            app_update_notification_body("   "),
            "An AI Switchboard for Mac update is ready to install. Open AI Switchboard for Mac to review the release and install it."
        );
    }

    #[test]
    fn macos_notifications_do_not_wait_for_clicks() {
        let source = include_str!("lib.rs");
        let start = source
            .find("#[cfg(target_os = \"macos\")]\npub(crate) fn show_notification_impl")
            .expect("macOS notification implementation exists");
        let rest = &source[start..];
        let end = rest
            .find("\n#[cfg(not(target_os = \"macos\"))]")
            .expect("non-macOS notification implementation follows macOS implementation");
        let macos_impl = &rest[..end];

        assert!(
            macos_impl.contains(".asynchronous(true)"),
            "macOS notifications must be fire-and-forget so they do not spin a click-wait run loop"
        );
        assert!(
            !macos_impl.contains(".wait_for_click("),
            "wait_for_click caused Headroom to hold a full CPU core while notifications were pending"
        );
    }

    #[test]
    fn store_checked_update_tracks_available_update_metadata() {
        let pending = Mutex::new(None);
        let metadata = sample_available_update("0.3.0");

        let result = store_checked_update(
            Ok(Some(FakePendingUpdate {
                metadata: metadata.clone(),
                install_result: Ok(()),
            })),
            &pending,
        )
        .expect("available update");

        assert_eq!(result, Some(metadata.clone()));
        let stored = pending.lock();
        assert_eq!(
            stored.as_ref().expect("pending update").metadata(),
            metadata
        );
    }

    #[test]
    fn store_checked_update_clears_pending_update_when_feed_is_current() {
        let pending = Mutex::new(Some(FakePendingUpdate {
            metadata: sample_available_update("0.3.0"),
            install_result: Ok(()),
        }));

        let result =
            store_checked_update::<FakePendingUpdate>(Ok(None), &pending).expect("no update");

        assert_eq!(result, None);
        assert!(pending.lock().is_none());
    }

    #[test]
    fn store_checked_update_preserves_pending_update_when_check_errors() {
        let existing = sample_available_update("0.3.0");
        let pending = Mutex::new(Some(FakePendingUpdate {
            metadata: existing.clone(),
            install_result: Ok(()),
        }));

        let error =
            store_checked_update::<FakePendingUpdate>(Err("feed unavailable".into()), &pending)
                .expect_err("check failure should bubble up");

        assert_eq!(error, "feed unavailable");
        let stored = pending.lock();
        assert_eq!(
            stored.as_ref().expect("pending update").metadata(),
            existing
        );
    }

    #[test]
    fn install_pending_update_requires_a_checked_update() {
        let pending = Mutex::new(None::<FakePendingUpdate>);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let error = runtime
            .block_on(install_pending_update(
                &pending,
                noop_app_update_progress_emitter(),
            ))
            .expect_err("missing update should fail");

        assert_eq!(error, "No downloaded update is ready to install.");
    }

    #[test]
    fn install_pending_update_runs_the_installer_and_clears_the_slot() {
        let pending = Mutex::new(Some(FakePendingUpdate {
            metadata: sample_available_update("0.3.0"),
            install_result: Ok(()),
        }));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        runtime
            .block_on(install_pending_update(
                &pending,
                noop_app_update_progress_emitter(),
            ))
            .expect("install succeeds");

        assert!(pending.lock().is_none());
    }

    #[test]
    fn install_pending_update_forwards_progress_to_emitter() {
        struct ProgressEmittingFake {
            metadata: AvailableAppUpdate,
            events: Vec<AppUpdateProgress>,
        }

        impl InstallableAppUpdate for ProgressEmittingFake {
            fn metadata(&self) -> AvailableAppUpdate {
                self.metadata.clone()
            }

            fn install(self, progress: AppUpdateProgressEmitter) -> InstallPendingUpdateFuture {
                Box::pin(async move {
                    for event in self.events {
                        progress(event);
                    }
                    Ok(())
                })
            }
        }

        let pending = Mutex::new(Some(ProgressEmittingFake {
            metadata: sample_available_update("0.3.0"),
            events: vec![
                AppUpdateProgress::Downloading {
                    downloaded: 1_024,
                    total: Some(2_048),
                },
                AppUpdateProgress::Downloading {
                    downloaded: 2_048,
                    total: Some(2_048),
                },
                AppUpdateProgress::Installing,
            ],
        }));
        let captured: Arc<Mutex<Vec<AppUpdateProgress>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_for_emit = Arc::clone(&captured);
        let emitter: AppUpdateProgressEmitter = Arc::new(move |event| {
            captured_for_emit.lock().push(event);
        });

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        runtime
            .block_on(install_pending_update(&pending, emitter))
            .expect("install succeeds");

        let events = captured.lock().clone();
        assert_eq!(
            events,
            vec![
                AppUpdateProgress::Downloading {
                    downloaded: 1_024,
                    total: Some(2_048),
                },
                AppUpdateProgress::Downloading {
                    downloaded: 2_048,
                    total: Some(2_048),
                },
                AppUpdateProgress::Installing,
            ]
        );
    }

    #[test]
    fn app_update_progress_serializes_with_phase_tag() {
        let downloading = serde_json::to_value(&AppUpdateProgress::Downloading {
            downloaded: 1024,
            total: Some(4096),
        })
        .expect("serialize downloading");
        assert_eq!(
            downloading,
            serde_json::json!({
                "phase": "downloading",
                "downloaded": 1024,
                "total": 4096,
            })
        );

        let installing =
            serde_json::to_value(&AppUpdateProgress::Installing).expect("serialize installing");
        assert_eq!(installing, serde_json::json!({ "phase": "installing" }));

        let unknown_total = serde_json::to_value(&AppUpdateProgress::Downloading {
            downloaded: 512,
            total: None,
        })
        .expect("serialize downloading with unknown total");
        assert_eq!(
            unknown_total,
            serde_json::json!({
                "phase": "downloading",
                "downloaded": 512,
                "total": null,
            })
        );
    }

    #[test]
    fn install_pending_update_returns_install_failures_after_taking_the_slot() {
        let pending = Mutex::new(Some(FakePendingUpdate {
            metadata: sample_available_update("0.3.0"),
            install_result: Err("signature mismatch".into()),
        }));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let error = runtime
            .block_on(install_pending_update(
                &pending,
                noop_app_update_progress_emitter(),
            ))
            .expect_err("install failure");

        assert_eq!(error, "signature mismatch");
        assert!(pending.lock().is_none());
    }

    #[test]
    fn tray_window_position_clamps_to_right_monitor_edge() {
        let target = compute_tray_window_position(
            PhysicalRect {
                x: 1430,
                y: 0,
                width: 24,
                height: 24,
            },
            PhysicalSize::new(760, 560),
            Some(MonitorBounds {
                x: 0,
                y: 0,
                width: 1440,
                height: 900,
            }),
        );

        assert_eq!(target.x, 680);
        assert_eq!(target.y, 34);
    }

    #[test]
    fn tray_window_position_moves_above_when_bottom_would_overflow() {
        let target = compute_tray_window_position(
            PhysicalRect {
                x: 500,
                y: 730,
                width: 24,
                height: 24,
            },
            PhysicalSize::new(760, 560),
            Some(MonitorBounds {
                x: 0,
                y: 0,
                width: 1440,
                height: 900,
            }),
        );

        assert_eq!(target.x, 132);
        assert_eq!(target.y, 160);
    }

    #[test]
    fn logical_tray_rects_are_converted_with_scale_factor() {
        let rect = Rect {
            position: Position::Logical(LogicalPosition::new(100.0, 20.0)),
            size: Size::Logical(LogicalSize::new(12.0, 12.0)),
        };

        let physical = physical_rect_from_rect(rect, 2.0);

        assert_eq!(
            physical,
            PhysicalRect {
                x: 200,
                y: 40,
                width: 24,
                height: 24,
            }
        );
    }

    #[test]
    fn token_milestone_kind_labels_first_and_repeating_thresholds() {
        assert_eq!(lifetime_token_milestone_kind(1_000_000), "first_1m");
        assert_eq!(lifetime_token_milestone_kind(5_000_000), "first_5m");
        assert_eq!(lifetime_token_milestone_kind(10_000_000), "first_10m");
        assert_eq!(lifetime_token_milestone_kind(20_000_000), "repeating_10m");
    }

    fn learn_prereq(
        claude: bool,
        codex_cli: bool,
        codex_logged_in: bool,
    ) -> HeadroomLearnPrereqStatus {
        HeadroomLearnPrereqStatus {
            claude_cli_available: claude,
            claude_cli_path: claude.then(|| "/usr/bin/claude".to_string()),
            codex_cli_available: codex_cli,
            codex_cli_path: codex_cli.then(|| "/usr/bin/codex".to_string()),
            codex_logged_in,
        }
    }

    #[test]
    fn check_headroom_learn_prereqs_passes_when_cli_available() {
        let prereq = learn_prereq(true, false, false);
        assert!(check_headroom_learn_prereqs(LearnAgent::Claude, None, &prereq).is_ok());
    }

    #[test]
    fn check_headroom_learn_prereqs_returns_install_message_when_cli_missing() {
        let prereq = learn_prereq(false, false, false);
        let err = check_headroom_learn_prereqs(LearnAgent::Claude, None, &prereq).unwrap_err();
        assert!(
            err.contains("Install the Claude Code CLI"),
            "expected install hint, got: {err}"
        );
    }

    #[test]
    fn check_headroom_learn_prereqs_prefers_platform_message_over_cli_check() {
        let prereq = learn_prereq(false, false, false);
        let err =
            check_headroom_learn_prereqs(LearnAgent::Claude, Some("Linux not supported"), &prereq)
                .unwrap_err();
        assert_eq!(err, "Linux not supported");
    }

    #[test]
    fn check_headroom_learn_prereqs_codex_passes_when_cli_present_and_logged_in() {
        let prereq = learn_prereq(false, true, true);
        assert!(check_headroom_learn_prereqs(LearnAgent::Codex, None, &prereq).is_ok());
    }

    #[test]
    fn check_headroom_learn_prereqs_codex_requires_cli_install() {
        let prereq = learn_prereq(true, false, false);
        let err = check_headroom_learn_prereqs(LearnAgent::Codex, None, &prereq).unwrap_err();
        assert!(
            err.contains("Install the Codex CLI"),
            "expected codex install hint, got: {err}"
        );
    }

    #[test]
    fn check_headroom_learn_prereqs_codex_requires_login_when_cli_present() {
        let prereq = learn_prereq(false, true, false);
        let err = check_headroom_learn_prereqs(LearnAgent::Codex, None, &prereq).unwrap_err();
        assert!(
            err.contains("Sign in to the Codex CLI"),
            "expected codex sign-in hint, got: {err}"
        );
    }

    #[test]
    fn fetch_transformations_feed_decodes_proxy_response() {
        std::env::set_var("HEADROOM_FULL_MESSAGE_LOGGING", "0");
        let app_storage_temp = tempfile::tempdir().expect("app storage tempdir");
        let _app_storage = AppStorageEnvGuard::isolated(app_storage_temp.path());
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let body = serde_json::json!({
                "log_full_messages": true,
                "transformations": [{
                    "request_id": "req-1",
                    "timestamp": "2026-04-21T10:00:00Z",
                    "provider": "anthropic",
                    "model": "claude-sonnet-4-6",
                    "input_tokens_original": 1000,
                    "input_tokens_optimized": 250,
                    "tokens_saved": 750,
                    "savings_percent": 75.0,
                    "transforms_applied": ["interceptor:ast-grep"],
                    "request_messages": [{
                        "role": "user",
                        "content": "sk-ant-test Authorization: Bearer abcdefghijklmnop"
                    }]
                }]
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let result =
            fetch_transformations_feed_from(&format!("http://127.0.0.1:{port}"), 50).unwrap();
        server.join().unwrap();

        assert!(result.proxy_reachable);
        assert!(!result.log_full_messages);
        assert_eq!(result.message_log_retention_hours, 24);
        assert_eq!(result.transformations.len(), 1);
        let event = &result.transformations[0];
        assert_eq!(event.request_id.as_deref(), Some("req-1"));
        assert_eq!(event.provider.as_deref(), Some("anthropic"));
        assert_eq!(event.tokens_saved, Some(750));
        assert_eq!(event.transforms_applied, vec!["interceptor:ast-grep"]);
        let redacted = serde_json::to_string(&event.request_messages).unwrap();
        assert!(!redacted.contains("sk-ant-test"));
        assert!(!redacted.contains("abcdefghijklmnop"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn fetch_transformations_feed_returns_error_on_non_2xx_status() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let response =
                "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).unwrap();
        });

        let err =
            fetch_transformations_feed_from(&format!("http://127.0.0.1:{port}"), 50).unwrap_err();
        server.join().unwrap();
        assert!(
            err.contains("503"),
            "expected status code in error, got: {err}"
        );
    }

    #[test]
    fn count_memories_created_today_only_counts_today_entries() {
        use chrono::TimeZone;
        let json = r#"[
            {"id":"a","created_at":"2026-04-22T10:00:00"},
            {"id":"b","created_at":"2026-04-22T23:59:59"},
            {"id":"c","created_at":"2026-04-21T23:00:00"},
            {"id":"d","created_at":null},
            {"id":"e"}
        ]"#;
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 22, 12, 0, 0).unwrap();
        assert_eq!(count_memories_created_today(json, now).unwrap(), 2);
    }

    #[test]
    fn count_memories_created_today_accepts_rfc3339_with_tz() {
        use chrono::TimeZone;
        let json = r#"[
            {"id":"a","created_at":"2026-04-22T10:00:00Z"},
            {"id":"b","created_at":"2026-04-22T02:00:00-09:00"}
        ]"#;
        // 2026-04-22T02:00:00-09:00 == 2026-04-22T11:00:00Z, both land on today.
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 22, 12, 0, 0).unwrap();
        assert_eq!(count_memories_created_today(json, now).unwrap(), 2);
    }

    #[test]
    fn count_memories_created_today_handles_empty_and_errors() {
        let now = chrono::Utc::now();
        assert_eq!(count_memories_created_today("[]", now).unwrap(), 0);
        assert!(count_memories_created_today("not json", now).is_err());
    }

    #[test]
    fn pattern_matches_project_requires_path_boundary() {
        assert!(pattern_matches_project(
            "File `/x/a/b/foo.py` missing",
            &[],
            "/x/a/b",
        ));
        // /x/ab must not match when root is /x/a
        assert!(!pattern_matches_project(
            "File `/x/ab/foo.py` missing",
            &[],
            "/x/a",
        ));
    }

    #[test]
    fn pattern_matches_project_via_entity_refs() {
        assert!(pattern_matches_project(
            "Command failed",
            &["/x/a/tool.py".to_string()],
            "/x/a",
        ));
    }

    #[test]
    fn parse_live_learnings_filters_and_parses() {
        let json = serde_json::to_string(&json!([
            {
                "id": "1",
                "content": "Pattern mentioning /x/a/foo.py",
                "created_at": "2026-04-22T10:00:00Z",
                "importance": 0.8,
                "metadata": {
                    "source": "traffic_learner",
                    "category": "environment",
                    "evidence_count": 3
                },
                "entity_refs": []
            },
            {
                "id": "2",
                "content": "Unrelated project /y/z",
                "metadata": {"source": "traffic_learner", "category": "environment"},
                "entity_refs": []
            },
            {
                "id": "3",
                "content": "/x/a/bar.py",
                "metadata": {"source": "other"},
                "entity_refs": []
            }
        ]))
        .unwrap();

        let learnings = parse_live_learnings(&json, "/x/a").unwrap();
        assert_eq!(learnings.len(), 1);
        assert_eq!(learnings[0].id, "1");
        assert_eq!(learnings[0].category, "environment");
        assert_eq!(learnings[0].evidence_count, 3);
        assert_eq!(learnings[0].importance, 0.8);
    }

    #[test]
    fn aggregate_live_learnings_returns_entry_per_path_including_empty() {
        let json = serde_json::to_string(&json!([
            {
                "id": "a1",
                "content": "Pattern in /x/a/foo.py",
                "metadata": {"source": "traffic_learner", "category": "environment"},
                "entity_refs": []
            },
            {
                "id": "b1",
                "content": "Pattern in /x/b/bar.py",
                "metadata": {"source": "traffic_learner", "category": "environment"},
                "entity_refs": []
            }
        ]))
        .unwrap();

        let paths = vec![
            "/x/a".to_string(),
            "/x/b".to_string(),
            "/x/empty".to_string(),
        ];
        let map = aggregate_live_learnings(&json, &paths).unwrap();

        assert_eq!(map.len(), 3, "one entry per requested path");
        assert_eq!(map.get("/x/a").unwrap().len(), 1);
        assert_eq!(map.get("/x/a").unwrap()[0].id, "a1");
        assert_eq!(map.get("/x/b").unwrap().len(), 1);
        assert_eq!(map.get("/x/b").unwrap()[0].id, "b1");
        assert!(
            map.get("/x/empty").unwrap().is_empty(),
            "paths with no matches get an empty Vec, not a missing key",
        );
    }

    #[test]
    fn aggregate_live_learnings_bubbles_json_errors() {
        let paths = vec!["/x/a".to_string()];
        let err = aggregate_live_learnings("not json", &paths).unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn empty_live_learnings_for_projects_fills_each_path_with_empty_vec() {
        let paths = vec!["/x/a".to_string(), "/x/b".to_string()];
        let map = empty_live_learnings_for_projects(&paths);
        assert_eq!(map.len(), 2);
        assert!(map.get("/x/a").unwrap().is_empty());
        assert!(map.get("/x/b").unwrap().is_empty());
    }

    #[test]
    fn fetch_transformations_feed_returns_error_when_proxy_unreachable() {
        // Bind and immediately drop a listener so we know the port is free.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let err =
            fetch_transformations_feed_from(&format!("http://127.0.0.1:{port}"), 50).unwrap_err();
        assert!(!err.is_empty(), "expected a non-empty error message");
    }

    // ── classify_bootstrap_failure ───────────────────────────────────────────

    fn make_command_failure(stderr: &str) -> crate::process_runner::CommandFailure {
        crate::process_runner::CommandFailure {
            program: "/usr/bin/pip".into(),
            args: vec!["install".into()],
            stdout: String::new(),
            stderr: stderr.into(),
            exit_code: Some(1),
            signal: None,
        }
    }

    #[test]
    fn classify_bootstrap_failure_flags_certificate_verify_failed_as_ssl_interception() {
        let err: anyhow::Error = make_command_failure(
            "ssl.SSLError: [SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed",
        )
        .into();
        assert!(matches!(
            classify_bootstrap_failure(&err),
            BootstrapFailureKind::SslInterception
        ));
    }

    #[test]
    fn classify_bootstrap_failure_flags_self_signed_with_hyphen_as_ssl_interception() {
        let err: anyhow::Error = make_command_failure(
            "Could not fetch URL: self-signed certificate in certificate chain",
        )
        .into();
        assert!(matches!(
            classify_bootstrap_failure(&err),
            BootstrapFailureKind::SslInterception
        ));
    }

    #[test]
    fn classify_bootstrap_failure_flags_self_signed_without_hyphen_as_ssl_interception() {
        let err: anyhow::Error = make_command_failure(
            "Could not fetch URL: self signed certificate in certificate chain",
        )
        .into();
        assert!(matches!(
            classify_bootstrap_failure(&err),
            BootstrapFailureKind::SslInterception
        ));
    }

    #[test]
    fn classify_bootstrap_failure_flags_no_usable_temporary_directory() {
        let err: anyhow::Error = make_command_failure(
            "FileNotFoundError: [Errno 2] No usable temporary directory found in \
             ['/var/folders/lp/.../T/', '/tmp', '/var/tmp', '/usr/tmp', \
             '/Users/x/Library/Application Support/Headroom/headroom']",
        )
        .into();
        assert!(matches!(
            classify_bootstrap_failure(&err),
            BootstrapFailureKind::NoUsableTempDir
        ));
    }

    #[test]
    fn classify_bootstrap_failure_flags_pip_connection_reset_as_network() {
        let err: anyhow::Error =
            make_command_failure("ConnectionResetError: [Errno 54] Connection reset by peer")
                .into();
        assert!(matches!(
            classify_bootstrap_failure(&err),
            BootstrapFailureKind::NetworkDownload
        ));
    }

    #[test]
    fn classify_bootstrap_failure_returns_other_for_unrelated_command_errors() {
        let err: anyhow::Error =
            make_command_failure("ModuleNotFoundError: No module named 'headroom'").into();
        assert!(matches!(
            classify_bootstrap_failure(&err),
            BootstrapFailureKind::Other
        ));
    }

    #[test]
    fn classify_bootstrap_failure_returns_other_for_unrecognized_non_command_chain() {
        let err = anyhow::anyhow!("something unexpected went wrong");
        assert!(matches!(
            classify_bootstrap_failure(&err),
            BootstrapFailureKind::Other
        ));
    }

    // ── read_applied_patterns_for_project + delete_applied_pattern ───────────

    fn write_claude_md_with_headroom_block(dir: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("CLAUDE.md");
        let content = "\
# Project notes

Some unrelated content.

<!-- headroom:learn:start -->
## Headroom Learned Patterns
*Auto-generated by `headroom learn`*

### First Section
- First bullet.
- Second bullet.

### Second Section
- Third bullet.
<!-- headroom:learn:end -->
";
        std::fs::write(&path, content).expect("write CLAUDE.md");
        path
    }

    #[test]
    fn read_applied_patterns_returns_empty_when_no_files_exist() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = read_applied_patterns_for_project(tmp.path().to_str().unwrap());
        assert!(result.claude_md.is_empty(), "no CLAUDE.md → empty sections");
        // memory.md lives under ~/.claude — we don't override HOME here, so we
        // can't assert it's empty. The CLAUDE.md side covers the parsing path.
    }

    #[test]
    fn read_applied_patterns_parses_claude_md_headroom_block() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_claude_md_with_headroom_block(tmp.path());

        let result = read_applied_patterns_for_project(tmp.path().to_str().unwrap());
        let titles: Vec<&str> = result.claude_md.iter().map(|s| s.title.as_str()).collect();
        assert!(
            titles.iter().any(|t| *t == "First Section"),
            "first section parsed, got titles: {titles:?}"
        );
        assert!(
            titles.iter().any(|t| *t == "Second Section"),
            "second section parsed, got titles: {titles:?}"
        );
        let first = result
            .claude_md
            .iter()
            .find(|s| s.title == "First Section")
            .expect("first section");
        assert_eq!(first.bullets.len(), 2);
    }

    #[tokio::test]
    async fn delete_applied_pattern_removes_one_bullet_and_keeps_section() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_claude_md_with_headroom_block(tmp.path());

        delete_applied_pattern(
            tmp.path().to_str().unwrap().to_string(),
            "claude".into(),
            "First Section".into(),
            "First bullet.".into(),
        )
        .await
        .expect("delete bullet");

        let result = read_applied_patterns_for_project(tmp.path().to_str().unwrap());
        let first = result
            .claude_md
            .iter()
            .find(|s| s.title == "First Section")
            .expect("First Section preserved when one of two bullets deleted");
        assert_eq!(first.bullets, vec!["Second bullet.".to_string()]);
        assert!(
            result.claude_md.iter().any(|s| s.title == "Second Section"),
            "other sections preserved"
        );
    }

    #[tokio::test]
    async fn delete_applied_pattern_drops_last_section_and_keeps_block_parseable() {
        // Regression: deleting the last bullet in the last section used to
        // truncate the block's trailing end marker, leaving the file
        // unparseable. After the fix, the block must still be reparseable
        // and the surviving section intact.
        let tmp = tempfile::tempdir().expect("tempdir");
        write_claude_md_with_headroom_block(tmp.path());

        delete_applied_pattern(
            tmp.path().to_str().unwrap().to_string(),
            "claude".into(),
            "Second Section".into(),
            "Third bullet.".into(),
        )
        .await
        .expect("delete bullet");

        let result = read_applied_patterns_for_project(tmp.path().to_str().unwrap());
        let titles: Vec<&str> = result.claude_md.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["First Section"],
            "Second Section dropped, First Section preserved"
        );
        let first = result
            .claude_md
            .iter()
            .find(|s| s.title == "First Section")
            .expect("First Section");
        assert_eq!(
            first.bullets,
            vec!["First bullet.".to_string(), "Second bullet.".to_string()]
        );

        // The on-disk file should still contain the end marker so a future
        // read won't return an empty result.
        let on_disk = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
        assert!(
            on_disk.contains("<!-- headroom:learn:end -->"),
            "end marker preserved on disk, got:\n{on_disk}"
        );
    }

    #[tokio::test]
    async fn delete_applied_pattern_rejects_unknown_file_kind() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_claude_md_with_headroom_block(tmp.path());

        let err = delete_applied_pattern(
            tmp.path().to_str().unwrap().to_string(),
            "garbage".into(),
            "First Section".into(),
            "First bullet.".into(),
        )
        .await
        .expect_err("unknown file_kind rejected");
        assert!(
            err.contains("Unknown file_kind"),
            "expected Unknown file_kind error, got: {err}"
        );
    }

    #[test]
    fn watchdog_should_be_up_requires_runtime_installed() {
        // Even if every other gate is "ready", a missing runtime means the
        // watchdog should not expect Python to be reachable yet.
        assert!(!watchdog_should_be_up(false, false, false, false, false));
    }

    #[test]
    fn watchdog_should_be_up_when_all_gates_clear() {
        // Installed, not paused, not booting, not upgrading, not bypassed —
        // this is the one input combination that must return true.
        assert!(watchdog_should_be_up(true, false, false, false, false));
    }

    #[test]
    fn watchdog_should_be_up_respects_user_pause() {
        assert!(!watchdog_should_be_up(true, true, false, false, false));
    }

    #[test]
    fn watchdog_should_be_up_skips_during_boot() {
        assert!(!watchdog_should_be_up(true, false, true, false, false));
    }

    #[test]
    fn watchdog_should_be_up_skips_during_runtime_upgrade() {
        assert!(!watchdog_should_be_up(true, false, false, true, false));
    }

    /// Critical regression guard. Removing the bypass clause from
    /// `watchdog_should_be_up` would silently turn the watchdog into a thrash
    /// loop the moment the pricing gate fires — it would keep restarting
    /// Python while the bypass forwarder is doing its job, eventually
    /// tripping the auto-pause path that strips Claude Code's env var.
    #[test]
    fn watchdog_should_be_up_skips_when_pricing_gate_bypassed() {
        assert!(!watchdog_should_be_up(true, false, false, false, true));
    }

    #[test]
    fn auto_resume_backoff_escalates_then_caps() {
        use std::time::Duration;
        // 30s -> 1m -> 2m for the first three attempts, then a 5m cap that holds
        // for all later attempts so a persistent outage retries indefinitely
        // without hammering restart.
        assert_eq!(auto_resume_backoff(0), Duration::from_secs(30));
        assert_eq!(auto_resume_backoff(1), Duration::from_secs(60));
        assert_eq!(auto_resume_backoff(2), Duration::from_secs(120));
        assert_eq!(auto_resume_backoff(3), Duration::from_secs(300));
        assert_eq!(auto_resume_backoff(50), Duration::from_secs(300));
    }

    #[test]
    fn is_port_conflict_failure_matches_non_headroom_bail() {
        assert!(is_port_conflict_failure(
            "port 6768 is occupied by a non-headroom process (python3.1 pid 1073); ..."
        ));
    }

    #[test]
    fn is_port_conflict_failure_matches_already_running_message() {
        // Distinct from a foreign-process conflict: a stale headroom child
        // still bound to the port.
        assert!(is_port_conflict_failure(
            "spawn aborted: headroom proxy already running on port 6768"
        ));
    }

    #[test]
    fn is_port_conflict_failure_rejects_unrelated_errors() {
        // Generic startup failures must NOT route to the rate-limited port-
        // conflict fingerprint — they need the Error-level capture.
        assert!(!is_port_conflict_failure(
            "ModuleNotFoundError: No module named 'headroom'"
        ));
        assert!(!is_port_conflict_failure(
            "venv interpreter exited with status 1"
        ));
        assert!(!is_port_conflict_failure(""));
    }

    #[test]
    fn parse_request_count_reads_nested_requests_total() {
        let body = json!({
            "requests": { "total": 42, "active": 1 },
            "tokens": { "saved": 100 }
        })
        .to_string();
        assert_eq!(parse_request_count_from_stats_body(&body), Some(42));
    }

    #[test]
    fn parse_request_count_falls_back_to_legacy_keys() {
        // Older /stats payloads exposed the count under flat keys. The
        // verification poller has to keep working against any of them or it
        // will get stuck on a runtime mid-upgrade between schema versions.
        let body = json!({ "total_requests": 7 }).to_string();
        assert_eq!(parse_request_count_from_stats_body(&body), Some(7));

        let body = json!({ "totalRequests": 9 }).to_string();
        assert_eq!(parse_request_count_from_stats_body(&body), Some(9));

        let body = json!({ "nested": { "requests_total": 11 } }).to_string();
        assert_eq!(parse_request_count_from_stats_body(&body), Some(11));
    }

    #[test]
    fn parse_request_count_returns_none_when_absent() {
        let body = json!({ "tokens": { "saved": 100 } }).to_string();
        assert_eq!(parse_request_count_from_stats_body(&body), None);
        assert_eq!(parse_request_count_from_stats_body("not json"), None);
    }

    #[test]
    fn parse_request_counts_by_agent_keys_by_agent_id() {
        let body = json!({
            "agent_usage": {
                "agents": [
                    { "agent": "claude-code", "requests": 5 },
                    { "agent": "codex", "requests": 2 }
                ]
            }
        })
        .to_string();
        let counts = parse_request_counts_by_agent(&body).unwrap();
        assert_eq!(counts.get("claude-code"), Some(&5));
        assert_eq!(counts.get("codex"), Some(&2));

        // Proxy up, no traffic yet: empty map, not None.
        let empty = json!({ "agent_usage": { "agents": [] } }).to_string();
        assert!(parse_request_counts_by_agent(&empty).unwrap().is_empty());

        // Unparseable body is None so the poller treats it as unreachable.
        assert!(parse_request_counts_by_agent("not json").is_none());
    }

    #[test]
    fn build_watchdog_give_up_report_uses_exit_status_when_present() {
        let report = build_watchdog_give_up_report(
            3,
            false,
            false,
            Some("exit status: 1".to_string()),
            Some("Traceback (most recent call last):\n  ...".to_string()),
            None,
            None,
            false,
            None,
            None,
            "ok".to_string(),
        );
        assert_eq!(report.tracked_child_exit_status, "exit status: 1");
        assert_eq!(report.consecutive_failures, 3);
        assert_eq!(
            report.message,
            "proxy_unreachable_post_boot (auto_paused after 3 failures)"
        );
        assert_eq!(
            report.log_tail.as_deref(),
            Some("Traceback (most recent call last):\n  ...")
        );
    }

    #[test]
    fn build_watchdog_give_up_report_falls_back_when_child_untracked() {
        // headroom_process_exited returns None when no Child handle is held
        // or the OS hasn't reaped the child. Payload must still be useful.
        let report = build_watchdog_give_up_report(
            5,
            true,
            false,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            "refused".to_string(),
        );
        assert_eq!(report.tracked_child_exit_status, "still_alive_or_untracked");
        assert!(report.bypass_active);
        assert!(report.log_tail.is_none());
    }

    #[test]
    fn build_watchdog_give_up_report_drops_empty_log_tail() {
        // tail_log_file returns "" when the log file is missing or unreadable.
        // Empty tails must not become an empty `proxy_log_tail` Sentry extra.
        let report = build_watchdog_give_up_report(
            3,
            false,
            false,
            None,
            Some(String::new()),
            None,
            None,
            false,
            None,
            None,
            "timeout".to_string(),
        );
        assert!(report.log_tail.is_none());
    }

    #[test]
    fn build_watchdog_give_up_report_propagates_upgrade_flag() {
        let report = build_watchdog_give_up_report(
            3,
            false,
            true,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            "timeout".to_string(),
        );
        assert!(report.runtime_upgrade_in_progress);
    }

    #[test]
    fn build_watchdog_give_up_report_carries_last_startup_error() {
        let report = build_watchdog_give_up_report(
            3,
            false,
            false,
            None,
            None,
            Some("Address already in use (os error 48)".to_string()),
            None,
            false,
            None,
            None,
            "refused".to_string(),
        );
        assert_eq!(
            report.last_startup_error.as_deref(),
            Some("Address already in use (os error 48)")
        );
    }

    #[test]
    fn build_watchdog_give_up_report_drops_empty_last_startup_error() {
        let report = build_watchdog_give_up_report(
            3,
            false,
            false,
            None,
            None,
            Some(String::new()),
            None,
            false,
            None,
            None,
            "ok".to_string(),
        );
        assert!(report.last_startup_error.is_none());
    }

    #[test]
    fn build_watchdog_give_up_report_carries_diagnostic_fields() {
        // Busy-event-loop signature: process alive, port still binds,
        // backend /readyz times out, log silent for ~30s.
        let report = build_watchdog_give_up_report(
            3,
            false,
            false,
            None,
            None,
            None,
            Some(54321),
            true,
            Some(120),
            Some(30),
            "timeout".to_string(),
        );
        assert_eq!(report.tracked_pid, Some(54321));
        assert!(report.port_accepts_tcp);
        assert_eq!(report.process_cpu_secs, Some(120));
        assert_eq!(report.log_silent_secs, Some(30));
        assert_eq!(report.backend_readyz_outcome, "timeout");
    }

    #[test]
    fn readyz_failed_checks_csv_lists_only_unhealthy_sorted() {
        let body = serde_json::json!({
            "checks": {
                "startup": { "ready": true },
                "upstream": { "ready": false },
                "memory": { "ready": false },
                "cache": { "ready": true },
            }
        });
        assert_eq!(readyz_failed_checks_csv(&body), "memory,upstream");
    }

    #[test]
    fn readyz_failed_checks_csv_empty_when_all_ready_or_no_checks() {
        let all_ready = serde_json::json!({ "checks": { "upstream": { "ready": true } } });
        assert_eq!(readyz_failed_checks_csv(&all_ready), "");
        let no_checks = serde_json::json!({ "ready": false });
        assert_eq!(readyz_failed_checks_csv(&no_checks), "");
    }

    #[test]
    fn readyz_failure_is_upstream_only_matches_only_upstream() {
        assert!(readyz_failure_is_upstream_only("http_503:upstream"));
        assert!(!readyz_failure_is_upstream_only("http_503:upstream,memory"));
        assert!(!readyz_failure_is_upstream_only("http_503:memory"));
        assert!(!readyz_failure_is_upstream_only("http_503"));
        assert!(!readyz_failure_is_upstream_only("ok"));
        assert!(!readyz_failure_is_upstream_only("timeout"));
    }

    #[test]
    fn readyz_failure_has_core_unhealthy_ignores_upstream_only() {
        assert!(readyz_failure_has_core_unhealthy("http_503:memory"));
        assert!(readyz_failure_has_core_unhealthy(
            "http_503:upstream,memory"
        ));
        assert!(readyz_failure_has_core_unhealthy(
            "http_503:startup,upstream"
        ));
        assert!(!readyz_failure_has_core_unhealthy("http_503:upstream"));
        assert!(!readyz_failure_has_core_unhealthy("http_503"));
        assert!(!readyz_failure_has_core_unhealthy("ok"));
        assert!(!readyz_failure_has_core_unhealthy("timeout"));
    }

    #[test]
    fn cpu_rate_indicates_burn_separates_spin_from_boundary_tick() {
        // Real spin: ~1 CPU-sec per wall-sec over the window.
        assert!(cpu_rate_indicates_burn(100, 104, 4.0));
        // Lone boundary tick: a single +1 over a ~4s window is rate 0.25.
        assert!(!cpu_rate_indicates_burn(100, 101, 4.0));
        // Idle: counter flat.
        assert!(!cpu_rate_indicates_burn(100, 100, 4.0));
        // Exactly at the 0.5 threshold does not count (strictly greater).
        assert!(!cpu_rate_indicates_burn(100, 102, 4.0));
        assert!(cpu_rate_indicates_burn(100, 103, 4.0));
    }

    #[test]
    fn cpu_rate_indicates_burn_guards_degenerate_inputs() {
        // Zero elapsed: avoid divide-by-zero, report not burning.
        assert!(!cpu_rate_indicates_burn(100, 200, 0.0));
        // `ps` counter going backwards (pid reuse / sampling skew): saturating
        // sub yields 0, not a panic or huge rate.
        assert!(!cpu_rate_indicates_burn(200, 100, 4.0));
    }

    #[test]
    fn extract_llm_failure_warnings_returns_none_for_clean_stderr() {
        let stderr =
            "2026-05-04 09:00:00,000 - headroom.learn.analyzer - INFO - using claude CLI backend\n";
        assert!(extract_llm_failure_warnings(stderr).is_none());
    }

    #[test]
    fn extract_llm_failure_warnings_extracts_single_timeout() {
        let stderr = "2026-05-03 22:18:50,070 - headroom.learn.analyzer - WARNING - LLM analysis failed: `claude -p` did not respond within 120s. Check network connectivity or try a different backend with --model <litellm-model-name>.\n";
        let extracted = extract_llm_failure_warnings(stderr).expect("warning extracted");
        assert!(extracted.starts_with("LLM analysis failed:"));
        assert!(extracted.contains("did not respond within 120s"));
    }

    #[test]
    fn extract_llm_failure_warnings_joins_multiple_lines() {
        let stderr = "\
2026-05-03 22:18:50,070 - headroom.learn.analyzer - WARNING - LLM analysis failed: `claude -p` did not respond within 120s.
2026-05-03 22:20:50,749 - headroom.learn.analyzer - WARNING - LLM analysis failed: `claude -p` did not respond within 120s.
";
        let extracted = extract_llm_failure_warnings(stderr).expect("warnings extracted");
        assert_eq!(extracted.matches("LLM analysis failed:").count(), 2);
        assert!(extracted.contains('\n'));
    }

    #[test]
    fn classify_bootstrap_failure_flags_github_504_as_network() {
        // Mirrors the reqwest chain produced when error_for_status hits a 504 on
        // a GitHub release asset (the install_rtk download path).
        let err = anyhow::anyhow!(
            "HTTP status server error (504 Gateway Time-out) for url \
             (https://github.com/rtk-ai/rtk/releases/download/v0.42.0/rtk-aarch64-apple-darwin.tar.gz)"
        )
        .context("downloading https://github.com/rtk-ai/rtk/releases/download/v0.42.0/rtk-aarch64-apple-darwin.tar.gz");
        assert!(matches!(
            classify_bootstrap_failure(&err),
            BootstrapFailureKind::NetworkDownload
        ));
    }

    #[test]
    fn is_network_download_signal_matches_transient_failures() {
        for sample in [
            "HTTP status server error (504 Gateway Time-out)",
            "error sending request for url (https://pypi.org/...)",
            "tcp connect error: Connection refused (os error 61)",
            "dns error: failed to lookup address information",
            "operation timed out",
        ] {
            assert!(is_network_download_signal(sample), "should match: {sample}");
        }
    }

    #[test]
    fn is_network_download_signal_ignores_config_failures() {
        assert!(!is_network_download_signal("CERTIFICATE_VERIFY_FAILED"));
        assert!(!is_network_download_signal(
            "No usable temporary directory found"
        ));
        assert!(!is_network_download_signal(
            "checksum mismatch for ...: expected abc, got def"
        ));
    }

    // Endpoint-protection signature matcher: kept conservative on purpose, so
    // every match here represents a pattern we believe is high-confidence AV/
    // EDR interference. Adding looser patterns dilutes the user-facing hint.

    #[test]
    fn is_endpoint_protection_signal_matches_code_signature_failures() {
        assert!(is_endpoint_protection_signal(
            "dyld[1234]: code signature invalid for '/path/to/_mmh3.so'"
        ));
        assert!(is_endpoint_protection_signal(
            "ERROR: code signature could not be verified for headroom_core"
        ));
    }

    #[test]
    fn is_endpoint_protection_signal_matches_dlopen_not_permitted() {
        let raw = "ImportError: dlopen(/Users/x/site-packages/torch/lib/libtorch.dylib, 0x0006): \
                   tried: '/Users/x/site-packages/torch/lib/libtorch.dylib' (operation not permitted)";
        assert!(is_endpoint_protection_signal(raw));

        // "Library not loaded" variant of the same dyld error.
        let raw2 = "Library not loaded: @rpath/libonnxruntime.dylib \
                    Reason: tried: '...' (operation not permitted)";
        assert!(is_endpoint_protection_signal(raw2));
    }

    #[test]
    fn is_endpoint_protection_signal_matches_sigkill_signatures() {
        assert!(is_endpoint_protection_signal(
            "command exited with signal=9 (no stderr)"
        ));
        assert!(is_endpoint_protection_signal("headroom: Killed: 9"));
        assert!(is_endpoint_protection_signal(
            "exit code 137 from /venv/bin/python -m headroom.proxy.server"
        ));
    }

    #[test]
    fn is_endpoint_protection_signal_matches_fresh_so_permission_denial() {
        assert!(is_endpoint_protection_signal(
            "open() Operation not permitted on /Users/x/site-packages/mmh3.cpython-312-darwin.so"
        ));
        assert!(is_endpoint_protection_signal(
            "Operation not permitted: cannot exec /venv/lib/libtorch_python.dylib"
        ));
    }

    #[test]
    fn is_endpoint_protection_signal_does_not_overmatch_benign_errors() {
        // Bare "killed" with no signal marker — could be OOM, user pkill, etc.
        assert!(!is_endpoint_protection_signal(
            "process killed before completing"
        ));
        // "Library not loaded" without the "not permitted" gate — ordinary
        // missing-dep error, very common during dev.
        assert!(!is_endpoint_protection_signal(
            "Library not loaded: @rpath/libfoo.dylib — Reason: image not found"
        ));
        // "Operation not permitted" without a fresh-extension context — could
        // be any random filesystem permission issue.
        assert!(!is_endpoint_protection_signal(
            "Operation not permitted on /private/var/db/foo.txt"
        ));
        // Generic network/disk errors must not falsely trigger.
        assert!(!is_endpoint_protection_signal(
            "Could not resolve host: pypi.org"
        ));
        assert!(!is_endpoint_protection_signal("ENOSPC: no space left"));
    }

    #[test]
    fn is_disk_full_signal_matches_pip_enospc_failures() {
        assert!(is_disk_full_signal(
            "ERROR: Could not install packages due to an OSError: [Errno 28] No space left on device"
        ));
        assert!(is_disk_full_signal(
            "OSError: [Errno 28] No space left on device"
        ));
        assert!(is_disk_full_signal("ENOSPC: no space left"));
        assert!(is_disk_full_signal("disk full"));
        // Case-insensitive.
        assert!(is_disk_full_signal("NO SPACE LEFT ON DEVICE"));
    }

    #[test]
    fn is_disk_full_signal_does_not_overmatch() {
        assert!(!is_disk_full_signal("network unreachable"));
        assert!(!is_disk_full_signal("permission denied"));
        assert!(!is_disk_full_signal("Could not resolve host: pypi.org"));
    }

    #[test]
    fn classify_upgrade_error_returns_endpoint_protection_hint_before_other_classifiers() {
        // Even when the error contains a "network" keyword (which would
        // otherwise hit the network classifier), the AV signal wins because
        // it's a more specific match for the actual cause.
        let err =
            anyhow::anyhow!("network unreachable during install — child exited with signal=9");
        let hint = classify_upgrade_error(&err).expect("must classify");
        assert!(
            hint.contains("endpoint protection"),
            "expected EDR hint, got: {hint}"
        );
    }

    #[test]
    fn load_release_readiness_report_reads_json_when_present() {
        let path = std::env::temp_dir().join(format!(
            "mac-ai-switchboard-release-readiness-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, r#"{"status":"blocked"}"#).unwrap();

        let payload = crate::release_evidence::load_release_readiness_report_from(&path).unwrap();

        assert_eq!(payload.report_path, path.to_string_lossy());
        assert_eq!(
            payload
                .report
                .unwrap()
                .get("status")
                .and_then(Value::as_str),
            Some("blocked")
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_release_readiness_report_tolerates_missing_file() {
        let path = std::env::temp_dir().join(format!(
            "mac-ai-switchboard-missing-release-readiness-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let payload = crate::release_evidence::load_release_readiness_report_from(&path).unwrap();

        assert_eq!(payload.report_path, path.to_string_lossy());
        assert!(payload.report.is_none());
    }

    #[test]
    fn release_evidence_command_rejects_unallowlisted_commands() {
        let err =
            crate::release_evidence::run_release_evidence_command("build-mac-dmg".to_string())
                .unwrap_err();

        assert!(
            err.contains(
                "enabled only for static-preflight, desktop-validation, local-dmg-build-install, local-installed-smoke, local-mode-relaunch-smoke, rollback-center-validation, doctor-repair-validation, uninstall-validation, repo-intelligence-validation, repo-memory-mcp-validation, local-only-network-validation, and release-report"
            ),
            "unexpected error: {err}"
        );
    }
}
