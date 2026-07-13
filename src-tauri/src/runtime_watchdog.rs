use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::{
    analytics, port_conflict, runtime_commands, runtime_diagnostics, runtime_probe,
    show_notification_impl, tool_manager,
};

// Set when the watchdog has captured a Sentry event for the current "down
// episode". Reset whenever the proxy is observed reachable again, so a
// subsequent crash re-fires.
static WATCHDOG_DOWN_CAPTURED: AtomicBool = AtomicBool::new(false);

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

/// give up: pause the runtime, flip `proxy_bypass=true` so the Rust intercept
/// passes traffic straight through to api.anthropic.com, and notify the user.
/// The user's `~/.claude/settings.json` env, hook, and shell blocks stay
/// intact — `start_headroom` clears bypass and brings Python back up without
/// needing to re-install anything on disk.
pub(crate) fn spawn_proxy_watchdog(app: AppHandle) {
    const POLL: std::time::Duration = std::time::Duration::from_secs(5);
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    // If a tick takes far longer than POLL of wall time, the system was
    // suspended (laptop sleep, App Nap throttle). Don't blame Python for
    // not responding to the first probe after resume — uvicorn's event
    // loop may need a beat to catch up before /readyz answers.
    const RESUME_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(30);

    std::thread::spawn(move || {
        let mut consecutive_failures: u32 = 0;
        let mut last_tick_wall = std::time::SystemTime::now();
        let mut auto_pause_next_retry: Option<std::time::Instant> = None;
        let mut auto_pause_failed: u32 = 0;
        let mut hung_kill_attempted = false;
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

            if runtime.auto_paused {
                let due = auto_pause_next_retry
                    .map(|t| std::time::Instant::now() >= t)
                    .unwrap_or(true);
                if due {
                    log::info!(
                        "watchdog: auto-resume attempt (failed_attempts={auto_pause_failed}); killing wedged proxy and restarting"
                    );
                    state.stop_headroom();
                    consecutive_failures = 0;
                    hung_kill_attempted = false;
                    if let Err(err) = state.resume_runtime() {
                        log::info!("watchdog: auto-resume resume_runtime failed: {err:#}");
                    }
                    auto_pause_next_retry = None;
                }
                continue;
            }

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
                auto_pause_failed = 0;
                auto_pause_next_retry = None;
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

            if just_resumed {
                log::info!(
                    "watchdog: probe skipped (system resumed after {elapsed:?}); resetting failure counter"
                );
                consecutive_failures = 0;
                continue;
            }

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

            if runtime_probe::proxy_port_accepts_connection()
                && !runtime_probe::intercept_port_accepts_connection()
            {
                state.ensure_proxy_intercept_running();
                consecutive_failures = 0;
                continue;
            }

            if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                let backend_readyz_outcome = runtime_diagnostics::probe_backend_readyz_outcome();
                if backend_readyz_outcome == "ok" {
                    log::info!(
                        "watchdog: backend /readyz answers ok after {consecutive_failures} intercept failures; skipping auto-pause and resetting counter"
                    );
                    consecutive_failures = 0;
                    continue;
                }
                if runtime_diagnostics::readyz_failure_is_upstream_only(&backend_readyz_outcome) {
                    log::info!(
                        "watchdog: backend /readyz 503 (upstream-only) after {consecutive_failures} failures; process healthy, skipping auto-pause"
                    );
                    consecutive_failures = 0;
                    continue;
                }
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
                log::info!(
                    "watchdog: giving up after {MAX_CONSECUTIVE_FAILURES} failures; pausing runtime and bypassing to Anthropic"
                );
                capture_watchdog_give_up(
                    &state,
                    consecutive_failures,
                    bypass_active,
                    backend_readyz_outcome,
                );
                state
                    .proxy_bypass
                    .store(true, std::sync::atomic::Ordering::Release);
                state.set_runtime_paused(true);
                state.set_runtime_auto_paused(true);
                state.stop_headroom();
                analytics::track_event(&app, "runtime_auto_paused", None);
                let _ = show_notification_impl(
                    &app,
                    "AI Switchboard paused the engine",
                    "The Headroom engine could not restart its proxy. Requests are passing through unmodified — AI Switchboard will keep retrying automatically, or open the app and hit Resume.",
                    Some("connectors".into()),
                );
                auto_pause_next_retry = Some(
                    std::time::Instant::now()
                        + runtime_diagnostics::auto_resume_backoff(auto_pause_failed),
                );
                auto_pause_failed = auto_pause_failed.saturating_add(1);
                consecutive_failures = 0;
                continue;
            }

            match state.ensure_headroom_running() {
                Ok(()) => port_conflict::note_proxy_started(&app),
                Err(err) => {
                    log::info!("watchdog: ensure_headroom_running failed: {err:#}");
                    port_conflict::note_proxy_failed(&app, &err, false);
                }
            }
        }
    });
}
