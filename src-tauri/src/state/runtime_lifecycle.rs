use super::*;

impl AppState {
    pub fn ensure_headroom_running(&self) -> Result<()> {
        if !self.tool_manager.python_runtime_installed() {
            return Ok(());
        }

        // Suppress the gate guards while a runtime upgrade is mid-validation.
        // The post-install boot validation in `run_upgrade_with_ui` calls
        // back into this function to bring the new venv up; if any of the
        // three gates below fires there, we silent-Ok-exit, the post-spawn
        // snapshot finds nothing running, and a perfectly good upgrade gets
        // rolled back as `not_started`. Routing isn't affected: client-side
        // configuration (`disable_client_setup`/`clear_client_setups`) is
        // mutated by whoever asserted the gate, so Claude Code is already
        // pointed direct-to-Anthropic regardless of whether Python is
        // bound on :6768. After validation, `run_upgrade_with_ui` calls
        // `stop_headroom()` if a gate is still active so we don't leave
        // the validation Python running where the user expected it down.
        let in_upgrade_validation = *self.runtime_upgrade_in_progress.lock();

        if !in_upgrade_validation {
            // When the pricing gate has flipped on `proxy_bypass`, Python is
            // intentionally down — the Rust intercept is routing direct to
            // Anthropic. Don't restart Python here; that would just defeat the
            // gate and (via the watchdog's failure path) eventually auto-pause
            // the runtime.
            if self.proxy_bypass.load(std::sync::atomic::Ordering::Acquire) {
                log::debug!("ensure_headroom_running: short-circuit (proxy_bypass active)");
                return Ok(());
            }

            if !self.pricing_allows_optimization() {
                self.enforce_pricing_gate();
                self.stop_python_if_gated();
                return Ok(());
            }

            if self.runtime_is_paused() {
                return Ok(());
            }
        }

        // Tear down any orphan proxy from an older desktop build BEFORE taking
        // the lifecycle lock, since `stop_headroom` acquires the same lock.
        // The orphan check: a proxy is reachable, but its argv is missing flags
        // this build relies on (e.g. --log-messages, --learn). Without this we
        // would happily reuse a v0.2.x proxy that pre-dates the Activity feed.
        if is_headroom_proxy_reachable()
            && !crate::tool_manager::running_proxy_matches_expected_args()
        {
            log::debug!(
                "headroom proxy is reachable but its argv predates this build; restarting it"
            );
            self.stop_headroom();
        }

        // Serialize lifecycle transitions so launch warm-up, tray open, and the
        // watchdog cannot race into concurrent proxy spawns before the backend
        // port is reachable and `headroom_process` has been recorded.
        let _lifecycle_guard = self.lifecycle_lock.lock();

        // Another caller may have brought the runtime up while we waited.
        if !self.tool_manager.python_runtime_installed() {
            return Ok(());
        }
        // Same upgrade-validation suppression as above. Re-read the flag
        // because the upgrade could have completed between the two reads
        // (lifecycle_lock can block for the duration of another spawn).
        if !*self.runtime_upgrade_in_progress.lock() {
            if !self.pricing_allows_optimization() {
                self.enforce_pricing_gate();
                return Ok(());
            }
            if self.runtime_is_paused() {
                return Ok(());
            }
        }

        // If the proxy is already live (e.g. started externally, or by us under
        // the lifecycle lock just above), treat runtime as healthy without
        // forcing another launcher.
        self.ensure_proxy_intercept_running();
        if is_headroom_proxy_reachable() {
            *self.last_startup_error.lock() = None;
            return Ok(());
        }

        let mut existing_backend_alive = false;
        {
            let mut process = self.headroom_process.lock();

            if let Some(existing) = process.as_mut() {
                match existing.try_wait() {
                    Ok(None) => {
                        existing_backend_alive = proxy_port_accepts_connection();
                    }
                    Ok(Some(_)) | Err(_) => {
                        *process = None;
                    }
                }
            }
        } // release lock before the blocking start

        if existing_backend_alive {
            self.ensure_proxy_intercept_running();
            for _ in 0..8 {
                if is_headroom_proxy_reachable() {
                    *self.last_startup_error.lock() = None;
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            *self.last_startup_error.lock() = Some(
                "Headroom backend is alive, but the client-facing proxy on 127.0.0.1:6767 is not ready."
                    .to_string(),
            );
            return Ok(());
        }

        self.set_runtime_starting(true);
        // During upgrade boot validation, reclaim 6768 even from a still-healthy
        // old proxy — we're replacing it, so leaving it alone would strand the
        // new venv unable to bind and roll the upgrade back as `not_started`.
        let reclaim_healthy_orphan = *self.runtime_upgrade_in_progress.lock();
        let started = self
            .tool_manager
            .start_headroom_background(reclaim_healthy_orphan);
        self.set_runtime_starting(false);

        match started {
            Ok(child) => {
                *self.headroom_process.lock() = Some(child);
                *self.last_startup_error.lock() = None;
                Ok(())
            }
            Err(err) => {
                *self.last_startup_error.lock() = Some(format!("{err:#}"));
                Err(err)
            }
        }
    }

    pub fn runtime_status(&self) -> RuntimeStatus {
        // Multiple pollers (tray icon updater at 260ms, proxy watchdog at 5s,
        // frontend interval at 3s, ad-hoc pre-warms) all land here and each
        // uncached call does a blocking HTTP `/readyz` plus several file
        // stats. A short TTL collapses them into one fetch without any
        // perceptible staleness — the longest-cadence caller is 5s, so 2s
        // TTL gives each poll a fresh read while deduping within bursts.
        const TTL: Duration = Duration::from_secs(2);
        {
            let cache = self.cached_runtime_status.lock();
            if let Some((status, at)) = cache.as_ref() {
                if at.elapsed() < TTL {
                    return status.clone();
                }
            }
        }
        let status = self.compute_runtime_status();
        *self.cached_runtime_status.lock() = Some((status.clone(), Instant::now()));
        status
    }

    fn compute_runtime_status(&self) -> RuntimeStatus {
        let installed = self.tool_manager.python_runtime_installed();
        let paused = self.runtime_is_paused();
        let auto_paused = self.runtime_is_auto_paused();
        let proxy_reachable = is_headroom_proxy_reachable();
        let mcp_configured = self.tool_manager.headroom_mcp_configured();
        let mcp_error = self.tool_manager.headroom_mcp_error();
        let repo_memory_mcp_configured = self.tool_manager.repo_memory_mcp_configured();
        let repo_memory_mcp_error = self.tool_manager.repo_memory_mcp_error();
        let repo_memory_mcp_service = self.tool_manager.repo_memory_mcp_service_status();
        self.supervise_repo_memory_mcp_if_due(repo_memory_mcp_configured);
        let repo_memory_mcp_session = self.repo_memory_mcp_state.lock().clone();
        let ml_installed = self.tool_manager.headroom_ml_installed();
        let platform = current_platform();
        let support_tier = current_platform_support_tier();
        let headroom_learn_disabled_reason = headroom_learn_platform_message();
        let kompress_enabled = if installed && proxy_reachable {
            self.tool_manager.headroom_kompress_enabled()
        } else {
            None
        };
        let rtk_installed = self.tool_manager.rtk_installed();
        let rtk_version = self.tool_manager.installed_rtk_version();
        let (rtk_path_configured, rtk_hook_configured) =
            rtk_integration_status().unwrap_or((false, false));
        let rtk_gain_summary = self.cached_rtk_gain_summary();
        let rtk_daily_stats = self.cached_rtk_daily_stats().unwrap_or_default();
        if let Some(stats) = rtk_gain_summary.as_ref() {
            self.savings_tracker.lock().observe_rtk_gain_summary(stats);
        }
        let headroom_pid = {
            let mut process = self.headroom_process.lock();
            if let Some(existing) = process.as_mut() {
                match existing.try_wait() {
                    Ok(None) => Some(existing.id()),
                    Ok(Some(_)) | Err(_) => {
                        *process = None;
                        None
                    }
                }
            } else {
                None
            }
        };
        let launch_agent_status = launch_agent_runtime_status();
        let backend_status = backend_runtime_status();

        let effective_running = installed && !paused && proxy_reachable;

        let startup_error = self.last_startup_error.lock().clone();
        let startup_error_hint = startup_error.as_deref().and_then(classify_startup_error);

        let repo_memory_mcp_supervision_status = repo_memory_mcp_supervision_status(
            &repo_memory_mcp_session,
            repo_memory_mcp_configured,
            std::process::id(),
            repo_memory_mcp_service.as_ref(),
        );
        self.record_repo_memory_mcp_supervision(&repo_memory_mcp_supervision_status);
        let repo_memory_mcp_session = self.repo_memory_mcp_state.lock().clone();

        RuntimeStatus {
            platform: platform.into(),
            support_tier: support_tier.into(),
            installed,
            running: effective_running,
            starting: self.runtime_is_starting() && !effective_running,
            paused,
            auto_paused,
            proxy_reachable,
            proxy_bind_address: "127.0.0.1:6767".to_string(),
            proxy_auth_status: "loopback_validated_unauthenticated".to_string(),
            proxy_auth_detail:
                "Intercept binds only to 127.0.0.1 and rejects browser Origin/non-loopback Host requests; managed clients do not yet support a shared per-session auth header."
                    .to_string(),
            headroom_pid,
            launch_agent_status,
            backend_status,
            mcp_configured,
            mcp_error,
            repo_memory_mcp_configured,
            repo_memory_mcp_error,
            repo_memory_mcp_active: repo_memory_mcp_session.active
                && repo_memory_mcp_configured == Some(true)
                && repo_memory_mcp_service_healthy(repo_memory_mcp_service.as_ref())
                && repo_memory_mcp_supervision_status == "verified_active",
            repo_memory_mcp_last_started_at: repo_memory_mcp_session.last_started_at,
            repo_memory_mcp_last_checked_at: repo_memory_mcp_session.last_checked_at,
            repo_memory_mcp_supervision_status,
            repo_memory_mcp_service,
            ml_installed,
            kompress_enabled,
            headroom_learn_supported: headroom_learn_disabled_reason.is_none(),
            headroom_learn_disabled_reason,
            startup_error,
            startup_error_hint,
            runtime_upgrade_failure: self.runtime_upgrade_failure(),
            rtk: RtkRuntimeStatus {
                installed: rtk_installed,
                enabled: !is_rtk_disabled(),
                version: rtk_version,
                path_configured: rtk_path_configured,
                hook_configured: rtk_hook_configured,
                total_commands: rtk_gain_summary.as_ref().map(|stats| stats.total_commands),
                total_input: rtk_gain_summary.as_ref().map(|stats| stats.total_input),
                total_output: rtk_gain_summary.as_ref().map(|stats| stats.total_output),
                total_saved: rtk_gain_summary.as_ref().map(|stats| stats.total_saved),
                avg_savings_pct: rtk_gain_summary.as_ref().map(|stats| stats.avg_savings_pct),
                total_time_ms: rtk_gain_summary.as_ref().map(|stats| stats.total_time_ms),
                avg_time_ms: rtk_gain_summary.as_ref().and_then(|stats| stats.avg_time_ms),
                daily: rtk_daily_stats,
            },
        }
    }

    pub fn set_runtime_paused(&self, paused: bool) {
        let mut runtime_paused = self.runtime_paused.lock();
        *runtime_paused = paused;
        drop(runtime_paused);
        self.invalidate_runtime_status_cache();
    }

    pub fn runtime_is_paused(&self) -> bool {
        *self.runtime_paused.lock()
    }

    pub fn set_runtime_auto_paused(&self, auto_paused: bool) {
        self.runtime_auto_paused
            .store(auto_paused, std::sync::atomic::Ordering::Release);
        self.invalidate_runtime_status_cache();
    }

    pub fn runtime_is_auto_paused(&self) -> bool {
        self.runtime_auto_paused
            .load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn set_runtime_starting(&self, starting: bool) {
        let mut runtime_starting = self.runtime_starting.lock();
        *runtime_starting = starting;
        drop(runtime_starting);
        self.invalidate_runtime_status_cache();
    }

    /// Drops the cached `RuntimeStatus` so the next call recomputes. Wired
    /// into every path that mutates visible runtime state (pause, resume,
    /// starting, upgrade phase) so user-initiated changes show up on the
    /// tray icon and settings UI within one tray-updater tick instead of
    /// waiting out the 2s TTL.
    pub fn invalidate_runtime_status_cache(&self) {
        *self.cached_runtime_status.lock() = None;
    }

    pub fn runtime_is_starting(&self) -> bool {
        *self.runtime_starting.lock()
    }

    pub fn resume_runtime(&self) -> Result<()> {
        self.set_runtime_paused(false);
        // Any successful resume clears the auto-pause flag so the self-heal
        // loop stops retrying and the banner drops the "stopped unexpectedly"
        // framing.
        self.set_runtime_auto_paused(false);
        // User explicitly resuming = "go back to optimizing." Clear bypass
        // so `ensure_headroom_running` doesn't short-circuit on the bypass
        // check (state.rs ~2247). If pricing still says we're gated, the
        // next pricing poll will re-set bypass; if not, Python comes up
        // and traffic flows through optimization again.
        self.proxy_bypass
            .store(false, std::sync::atomic::Ordering::Release);
        self.ensure_headroom_running()
    }

    pub fn stop_headroom(&self) {
        let _lifecycle_guard = self.lifecycle_lock.lock();
        self.set_runtime_starting(false);
        let mut process = self.headroom_process.lock();

        if let Some(mut child) = process.take() {
            let pid = child.id() as i32;
            let _ = std::process::Command::new("/bin/kill")
                .arg("-TERM")
                .arg(format!("-{pid}"))
                .status();
            // Bounded wait: a backend that ignores SIGTERM (mid-request, stuck
            // shutdown) must not block this caller forever. stop_headroom runs
            // on the UI thread during restart_app, so an unbounded child.wait()
            // freezes the app ("not responding"). Give it ~2s, then SIGKILL the
            // process group and reap.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) | Err(_) => break,
                    Ok(None) => {
                        if std::time::Instant::now() >= deadline {
                            let _ = std::process::Command::new("/bin/kill")
                                .arg("-KILL")
                                .arg(format!("-{pid}"))
                                .status();
                            let _ = child.wait();
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
        }

        // Also clean up detached/orphaned Headroom-managed headroom proxies
        // so quitting the UI cannot leave the background listener behind.
        // We deliberately drop the port number from the match pattern: the
        // proxy may have fallen back to 6769..=6790 if 6768 was foreign-held,
        // and the python module path / entrypoint subcommand is unique enough
        // to identify our proxies regardless of port.
        let managed_python = self.tool_manager.managed_python();
        let command_patterns = [
            format!("{} -m headroom.proxy.server", managed_python.display()),
            format!(
                "{} proxy --port",
                self.tool_manager.headroom_entrypoint().display()
            ),
        ];
        for pattern in command_patterns {
            if let Err(err) = kill_processes_by_command_pattern(&pattern) {
                log::warn!("failed to clean detached headroom proxy processes: {err}");
            }
        }
    }

    /// One-shot, best-effort prefetch of the Kompress ML model on a fresh
    /// install. Blocks (run on a background thread) — downloads the ~260MB
    /// model the proxy would otherwise fetch lazily on first request, so a new
    /// user has ML compression ready before any traffic and never sees a
    /// lingering "Kompress disabled" banner.
    ///
    /// Skips immediately (no work) when: already attempted this launch, the
    /// runtime isn't installed/reachable, the `[ml]` extras aren't installed,
    /// the model is already cached, or Kompress already reports enabled.
    ///
    /// On a successful download, if the proxy has been idle (no recent
    /// proxy-log activity) it does one graceful restart so startup eager-load
    /// re-reports `Kompress: ENABLED`. If the proxy is actively serving, it
    /// skips the restart — `headroom_kompress_enabled` detects the lazy-load
    /// marker on the next request instead, so the status still flips on its own.
    pub fn maybe_prefetch_kompress(&self) {
        // One-shot guard: claim the attempt; bail if another call already did.
        if self
            .kompress_prefetch_attempted
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_err()
        {
            return;
        }

        if !self.tool_manager.python_runtime_installed() || !is_headroom_proxy_reachable() {
            return;
        }
        // Only meaningful when the ML extras are present but the model isn't
        // loaded yet. If ml isn't installed, prefetch can't help; if Kompress
        // already reports enabled, there's nothing to do.
        if self.tool_manager.headroom_ml_installed() != Some(true) {
            return;
        }
        if self.tool_manager.kompress_model_cached()
            || self.tool_manager.headroom_kompress_enabled() == Some(true)
        {
            return;
        }

        log::info!("kompress prefetch: downloading model on fresh install");
        match self.tool_manager.prefetch_kompress_model() {
            Ok(crate::tool_manager::KompressPrefetchOutcome::Downloaded) => {}
            Ok(crate::tool_manager::KompressPrefetchOutcome::Failed { cause }) => {
                // Reported to Sentry: the cause distinguishes systemic failures
                // (network / disk / native abort) worth acting on in aggregate.
                log::warn!("kompress prefetch download error: {cause}");
                return;
            }
            Err(err) => {
                log::warn!("kompress prefetch failed: {err:#}");
                return;
            }
        }
        log::info!("kompress prefetch: model cached");

        // Invalidate the runtime-status cache so the freshly-cached state is
        // reflected on the next poll regardless of the restart decision.
        *self.cached_runtime_status.lock() = None;

        // Surface "enabled" proactively only when safe: a restart drops any
        // in-flight request, so we require the proxy to be idle first.
        if self.runtime_is_paused() || self.runtime_is_starting() {
            return;
        }
        let idle = newest_proxy_log_mtime(&self.tool_manager.logs_dir())
            .and_then(|mtime| std::time::SystemTime::now().duration_since(mtime).ok())
            .map(|age| age >= std::time::Duration::from_secs(20))
            .unwrap_or(true);
        if !idle {
            log::info!("kompress prefetch: proxy busy, deferring restart to lazy-load detection");
            return;
        }

        log::info!("kompress prefetch: restarting proxy to load cached model");
        self.stop_headroom();
        if let Err(err) = self.ensure_headroom_running() {
            log::warn!("kompress prefetch: restart after download failed: {err:#}");
        }
        *self.cached_runtime_status.lock() = None;
    }
}
