use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::models::{BootstrapProgress, RuntimeStatus, RuntimeUpgradeProgress};
use crate::runtime_diagnostics::{
    capture_bootstrap_failure, classify_bootstrap_failure, user_message_for, BootstrapFailureKind,
};
use crate::state::AppState;
use crate::{analytics, client_adapters, switchboard_commands};

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
