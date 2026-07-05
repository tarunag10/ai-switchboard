use std::time::{Duration, Instant};

use super::launch_profile::persist_launch_profile;
use super::{
    AppState, BootValidationOutcome, MAX_UPGRADE_AUTO_RETRIES, RUNTIME_UPGRADE_BOOT_MAX_SECS,
    RUNTIME_UPGRADE_STALL_GRACE_SECS, RUNTIME_UPGRADE_STALL_SILENCE_SECS,
};
use crate::models::{RuntimeUpgradeFailure, RuntimeUpgradeProgress};
use crate::runtime_boot_validation::{
    boot_validation_stalled, log_mtime_advanced, newest_proxy_log_mtime,
};
use crate::runtime_probe::{
    cpu_time_advanced, hf_cache_grew, hf_hub_cache_dir, probe_proxy_livez,
    proxy_port_accepts_connection, total_dir_size_bytes, tracked_process_cpu_time_secs,
};
use crate::tool_manager::HeadroomRelease;

pub(super) enum RuntimeMaintenancePlan {
    Upgrade(HeadroomRelease),
    RequirementsRepair,
}

/// Reasons `ensure_headroom_running` may have returned `Ok(())` without
/// actually spawning a tracked child. Captured immediately after the call so
/// a "Stalled" / "NotStarted" Sentry event can attribute the silent no-op.
#[derive(Debug, Clone)]
pub(super) struct PostSpawnSnapshot {
    pub(super) tracked_child: bool,
    pub(super) python_installed: bool,
    pub(super) proxy_bypass: bool,
    pub(super) pricing_allows_optimization: bool,
    pub(super) runtime_paused: bool,
    pub(super) proxy_reachable: bool,
    pub(super) ensure_error: Option<String>,
}

/// Emit the runtime upgrade progress event on the given AppHandle.
pub(super) fn emit_runtime_upgrade_progress(app: &tauri::AppHandle, state: &AppState) {
    use tauri::Emitter;
    let _ = app.emit("runtime_upgrade_progress", state.runtime_upgrade_progress());
}

/// Escape hatch: set `HEADROOM_SKIP_RUNTIME_UPGRADE=1` to boot past a
/// persistently-failing upgrade without editing disk state.
pub(super) fn runtime_upgrade_disabled_by_env() -> bool {
    matches!(
        std::env::var("HEADROOM_SKIP_RUNTIME_UPGRADE")
            .ok()
            .as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

impl AppState {
    pub(super) fn runtime_maintenance_plan_for_app_version(
        &self,
        current_app_version: &str,
    ) -> Option<RuntimeMaintenancePlan> {
        if runtime_upgrade_disabled_by_env() {
            log::debug!("HEADROOM_SKIP_RUNTIME_UPGRADE is set — skipping runtime upgrade check.");
            return None;
        }
        let profile = self.launch_profile.lock();
        let version_matches = profile
            .last_launched_app_version
            .as_deref()
            .map(|v| v == current_app_version)
            .unwrap_or(false);
        if version_matches {
            return None;
        }
        if let Some(failure) = profile.last_runtime_upgrade_failure.as_ref() {
            if failure.app_version == current_app_version
                && failure.attempts >= MAX_UPGRADE_AUTO_RETRIES
            {
                return None;
            }
        }
        drop(profile);
        if let Some(release) = self.tool_manager.check_headroom_upgrade() {
            return Some(RuntimeMaintenancePlan::Upgrade(release));
        }
        if self.tool_manager.requirements_are_stale() {
            return Some(RuntimeMaintenancePlan::RequirementsRepair);
        }
        None
    }

    /// Returns true if the app version changed since the last successful
    /// launch AND an actual upgrade is needed (either headroom-ai version
    /// mismatch or requirements lock drift). Also gates on the retry budget
    /// from any prior upgrade failure, and on `HEADROOM_SKIP_RUNTIME_UPGRADE`.
    pub fn should_run_runtime_upgrade(&self, app: &tauri::AppHandle) -> bool {
        self.runtime_maintenance_plan_for_app_version(&app.package_info().version.to_string())
            .is_some()
    }

    /// User-initiated retry of a previously-failed runtime upgrade. Resets
    /// the attempts counter so `should_run_runtime_upgrade` lets it through,
    /// then invokes `run_upgrade_with_ui` directly.
    ///
    /// `force_rebuild` is the "Retry with full rebuild" path — skips the
    /// in-place attempt and runs atomic rebuild from scratch. Use when the
    /// previous attempt installed cleanly but the proxy never booted (the
    /// ABI-mismatch failure mode).
    pub fn retry_runtime_upgrade(&self, app: &tauri::AppHandle, force_rebuild: bool) {
        {
            let mut profile = self.launch_profile.lock();
            if let Some(failure) = profile.last_runtime_upgrade_failure.as_mut() {
                failure.attempts = 0;
            }
            persist_launch_profile(&self.launch_profile_path, &profile);
        }
        self.run_upgrade_with_ui(app, force_rebuild);
    }

    pub fn runtime_upgrade_in_progress(&self) -> bool {
        *self.runtime_upgrade_in_progress.lock()
    }

    /// Adaptive boot validation loop. Probes `/livez` on the backend port
    /// (default 6768; may be a fallback in 6769..=6790) until the proxy
    /// responds, the proxy process exits, the log goes silent past the
    /// stall threshold, or `RUNTIME_UPGRADE_BOOT_MAX_SECS` elapses. On each
    /// pass through the loop, emits a progress update via `on_progress`.
    ///
    /// "Activity" is the union of four signals: (1) a write to any
    /// ``headroom-proxy*.log`` file, (2) growth in the HuggingFace hub
    /// cache, (3) a successful TCP connect to the backend loopback port,
    /// and (4) advancement of the tracked child's accumulated CPU time.
    /// Any one resets the silence timer. The HF signal is what keeps
    /// slow-but-progressing first-run downloads from being killed —
    /// when transformers/huggingface_hub is silently pulling multi-GB
    /// model weights, the python process writes nothing to its log,
    /// but the cache directory grows monotonically. The TCP signal
    /// covers the case where the proxy is alive and bound but its
    /// asyncio event loop is held by an in-flight forwarded request
    /// (e.g. a `POST /v1/messages` retrying against a 429-ing
    /// upstream) — the kernel still completes ``accept()`` even when
    /// uvicorn isn't draining the socket, so a successful connect
    /// proves the python process is alive even though no HTTP
    /// endpoint answers. The CPU-time signal covers a fourth case
    /// that all three above miss: lifespan-phase work that's neither
    /// writing logs nor downloading models nor yet bound to the port,
    /// e.g. ONNX graph compilation or eager-loading already-cached
    /// models. As long as the python process is burning CPU, it's
    /// not deadlocked.
    pub(crate) fn wait_for_boot_validation<F>(&self, mut on_progress: F) -> BootValidationOutcome
    where
        F: FnMut(std::time::Duration, bool),
    {
        // 5s is generous: /livez is a cheap endpoint, but the proxy event
        // loop can be held by the GIL while the pipeline chews through a
        // large Claude request (tokenization, ONNX inference, etc). The
        // previous 1.5s timeout false-fired during those bursts.
        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return BootValidationOutcome::TimedOut,
        };

        let logs_dir = self.tool_manager.logs_dir();
        let hf_cache = hf_hub_cache_dir();
        // Cap the walk so a warm cache (post-install: ~3-5 GB across tens
        // of thousands of files) doesn't dominate the 500ms loop tick.
        // 50k entries is well above any healthy first-run install.
        const HF_CACHE_WALK_CAP: usize = 50_000;

        // Capture the tracked PID once at loop entry. If `headroom_process`
        // is None now (e.g. ensure_headroom_running short-circuited or the
        // spawn errored), it stays None for the duration — capturing once
        // avoids re-acquiring the lock every 500ms.
        let tracked_pid: Option<u32> = self.headroom_process.lock().as_ref().map(|c| c.id());

        let start = Instant::now();
        let mut last_log_activity = start;
        let mut last_seen_mtime = newest_proxy_log_mtime(&logs_dir);
        let mut last_hf_size = hf_cache
            .as_deref()
            .map(|p| total_dir_size_bytes(p, HF_CACHE_WALK_CAP));
        let mut last_cpu_secs: Option<u64> = tracked_pid.and_then(tracked_process_cpu_time_secs);
        let mut last_progress = Instant::now()
            .checked_sub(Duration::from_secs(5))
            .unwrap_or_else(Instant::now);

        let max = Duration::from_secs(RUNTIME_UPGRADE_BOOT_MAX_SECS);
        let grace = Duration::from_secs(RUNTIME_UPGRADE_STALL_GRACE_SECS);
        let silence = Duration::from_secs(RUNTIME_UPGRADE_STALL_SILENCE_SECS);
        let progress_interval = Duration::from_secs(2);

        loop {
            if probe_proxy_livez(&client) {
                return BootValidationOutcome::Reachable;
            }

            if let Some(exit_status) = self.headroom_process_exited() {
                log::warn!(
                    "wait_for_boot_validation: tracked proxy child exited with status {exit_status}"
                );
                return BootValidationOutcome::ProcessExited;
            }

            let elapsed = start.elapsed();
            if elapsed >= max {
                return BootValidationOutcome::TimedOut;
            }

            // Refresh log activity observation.
            let current_mtime = newest_proxy_log_mtime(&logs_dir);
            if log_mtime_advanced(last_seen_mtime, current_mtime) {
                last_seen_mtime = current_mtime;
                last_log_activity = Instant::now();
            }

            // Refresh HF cache observation. Growth in this tree means the
            // proxy is downloading model weights — the most common
            // not-actually-stuck cause of log silence on first-run installs.
            if let Some(cache_path) = hf_cache.as_deref() {
                let current_size = total_dir_size_bytes(cache_path, HF_CACHE_WALK_CAP);
                if hf_cache_grew(last_hf_size, current_size) {
                    last_log_activity = Instant::now();
                }
                last_hf_size = Some(current_size);
            }

            // Refresh TCP-bound observation. If the kernel accepts a
            // connection on :6768, the python process is alive and
            // listening — even if the asyncio loop is currently held
            // by an in-flight forwarded request and no HTTP endpoint
            // answers. This is the load-bearing signal that keeps a
            // busy-but-alive proxy from being killed as "stalled".
            let port_bound = proxy_port_accepts_connection();
            if port_bound {
                last_log_activity = Instant::now();
            }

            // Refresh CPU-time observation. Catches lifespan-phase work
            // that's invisible to the three signals above — e.g. ONNX
            // graph compile or eager-loading pre-cached models, which
            // can sit silent for >90s while the python process is hot
            // on a CPU. Only fires for the tracked child; if we don't
            // own a Child handle (rare — ensure_headroom_running
            // short-circuited or errored), this signal is unavailable
            // and we lean on the other three.
            let mut cpu_advanced = false;
            if let Some(pid) = tracked_pid {
                let current_cpu_secs = tracked_process_cpu_time_secs(pid);
                if cpu_time_advanced(last_cpu_secs, current_cpu_secs) {
                    last_log_activity = Instant::now();
                    cpu_advanced = true;
                }
                last_cpu_secs = current_cpu_secs;
            }

            let activity_age = last_log_activity.elapsed();
            let has_recent_activity = activity_age < silence
                && (current_mtime.is_some()
                    || last_hf_size.unwrap_or(0) > 0
                    || port_bound
                    || cpu_advanced);

            // Past grace period and nothing has moved in either signal
            // for the silence window → treat as stalled.
            if boot_validation_stalled(elapsed, activity_age, grace, silence) {
                return BootValidationOutcome::Stalled;
            }

            if last_progress.elapsed() >= progress_interval {
                on_progress(elapsed, has_recent_activity);
                last_progress = Instant::now();
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    }

    pub fn runtime_upgrade_progress(&self) -> RuntimeUpgradeProgress {
        self.runtime_upgrade_progress.lock().clone()
    }

    pub fn runtime_upgrade_failure(&self) -> Option<RuntimeUpgradeFailure> {
        self.launch_profile
            .lock()
            .last_runtime_upgrade_failure
            .clone()
    }

    pub(super) fn set_upgrade_progress<F>(&self, mutate: F)
    where
        F: FnOnce(&mut RuntimeUpgradeProgress),
    {
        let mut p = self.runtime_upgrade_progress.lock();
        mutate(&mut p);
    }

    pub(super) fn stamp_app_version(&self, version: &str) {
        let mut profile = self.launch_profile.lock();
        profile.last_launched_app_version = Some(version.to_string());
        persist_launch_profile(&self.launch_profile_path, &profile);
    }

    /// True when the launch-profile stamp can be safely advanced to
    /// `current_app_version` from `warm_runtime_on_launch` even though no
    /// runtime maintenance ran.
    ///
    /// Refuses to stamp when:
    /// - the stamp already matches (no work; avoids a redundant disk write), or
    /// - there's an unresolved upgrade failure for this exact app version
    ///   (stamping would mask the failure record the retry banner relies on).
    pub(super) fn can_stamp_no_maintenance(&self, current_app_version: &str) -> bool {
        let profile = self.launch_profile.lock();
        if profile.last_launched_app_version.as_deref() == Some(current_app_version) {
            return false;
        }
        if let Some(failure) = profile.last_runtime_upgrade_failure.as_ref() {
            if failure.app_version == current_app_version {
                return false;
            }
        }
        true
    }

    pub(super) fn clear_upgrade_failure(&self) {
        let mut profile = self.launch_profile.lock();
        profile.last_runtime_upgrade_failure = None;
        persist_launch_profile(&self.launch_profile_path, &profile);
    }

    pub fn dismiss_upgrade_failure(&self) {
        self.clear_upgrade_failure();
        self.invalidate_runtime_status_cache();
    }

    pub(super) fn record_upgrade_failure(&self, mut failure: RuntimeUpgradeFailure) {
        let mut profile = self.launch_profile.lock();
        let attempts = match profile.last_runtime_upgrade_failure.as_ref() {
            Some(prev) if prev.app_version == failure.app_version => {
                prev.attempts.saturating_add(1)
            }
            _ => 1,
        };
        failure.attempts = attempts;
        if let Some(prev) = profile.last_runtime_upgrade_failure.as_ref() {
            if prev.app_version == failure.app_version {
                failure.first_attempt_at = prev.first_attempt_at;
            }
        }
        profile.last_runtime_upgrade_failure = Some(failure);
        persist_launch_profile(&self.launch_profile_path, &profile);
    }

    pub(super) fn upgrade_failure_attempts(&self, app_version: &str) -> u32 {
        self.launch_profile
            .lock()
            .last_runtime_upgrade_failure
            .as_ref()
            .filter(|f| f.app_version == app_version)
            .map(|f| f.attempts)
            .unwrap_or(0)
    }
}
