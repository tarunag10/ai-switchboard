use super::*;

impl ToolManager {
    /// `reclaim_healthy_orphan`: forwarded to `reclaim_orphan_proxy` so an
    /// upgrade boot validation replaces even a still-healthy old proxy squatting
    /// on 6768. Pass `false` for normal launch (leave a live backend alone).
    pub fn start_headroom_background(&self, reclaim_healthy_orphan: bool) -> Result<Child> {
        let mut allow_repair = true;
        'attempt: loop {
            let python = self.managed_python();
            if !python.exists() {
                bail!("headroom managed python not found at {}", python.display());
            }

            let entrypoint = self.headroom_entrypoint();

            let mut failures: Vec<HeadroomStartupFailure> = Vec::new();
            let logs_dir = self.runtime.logs_dir();
            std::fs::create_dir_all(&logs_dir)
                .with_context(|| format!("creating {}", logs_dir.display()))?;

            // Pre-flight: 6768 may already be held. Three cases:
            //   * Free → spawn on 6768.
            //   * HeadroomRunning → an orphaned proxy from a prior session is
            //     squatting on the port (a healthy one would have satisfied
            //     `is_headroom_proxy_reachable` upstream, so `ensure_headroom_running`
            //     would never have reached the spawn path). Reclaim the port by
            //     terminating it, then spawn the fresh runtime. Only bail if it
            //     turns out to be genuinely health-serving or we can't free it.
            //   * ForeignOccupant → try to fall back to a port in
            //     6769..=6790. Only bail if every fallback is also taken.
            // The chosen port is stored in `backend_port` so the intercept,
            // health probes, and spawn args all pick it up.
            let initial_state = diagnose_proxy_port(backend_port::DEFAULT_BACKEND_PORT);
            match initial_state {
                PortState::Free => {
                    backend_port::set(backend_port::DEFAULT_BACKEND_PORT);
                }
                PortState::HeadroomRunning => {
                    reclaim_orphan_proxy(
                        backend_port::DEFAULT_BACKEND_PORT,
                        reclaim_healthy_orphan,
                    )?;
                    backend_port::set(backend_port::DEFAULT_BACKEND_PORT);
                }
                PortState::ForeignOccupant(detail) => {
                    let pid = parse_pid_from_lsof_detail(&detail);
                    let try_bind = |port: u16| TcpListener::bind(("127.0.0.1", port)).is_ok();
                    match backend_port::select_fallback(detail.clone(), pid, try_bind) {
                        Ok(SelectedFallback {
                            port,
                            original_occupant,
                            original_pid,
                        }) => {
                            backend_port::set(port);
                            log::warn!(
                                "[backend_port] {} held by {}; falling back to {}",
                                backend_port::DEFAULT_BACKEND_PORT,
                                original_occupant,
                                port,
                            );
                            sentry::with_scope(
                                |scope| {
                                    scope.set_tag("flow", "backend_port_fallback");
                                    scope.set_tag(
                                        "occupant_cmd",
                                        original_occupant
                                            .split(" pid ")
                                            .next()
                                            .unwrap_or("unknown"),
                                    );
                                    scope.set_extra(
                                        "original_port",
                                        backend_port::DEFAULT_BACKEND_PORT.into(),
                                    );
                                    scope.set_extra("chosen_port", port.into());
                                    if let Some(p) = original_pid {
                                        scope.set_extra("occupant_pid", p.into());
                                    }
                                },
                                || {
                                    sentry::capture_message(
                                        &format!(
                                            "backend_port_fallback: {} held by {}, using {}",
                                            backend_port::DEFAULT_BACKEND_PORT,
                                            original_occupant,
                                            port,
                                        ),
                                        sentry::Level::Info,
                                    );
                                },
                            );
                        }
                        Err(AllForeign {
                            original_occupant,
                            fallback_range,
                            ..
                        }) => {
                            bail!(
                                "{}",
                                format_all_foreign_bail(
                                    backend_port::DEFAULT_BACKEND_PORT,
                                    &original_occupant,
                                    fallback_range,
                                )
                            );
                        }
                    }
                }
            }

            // Construct spawn variants AFTER pre-flight so `--port` reflects any
            // fallback chosen above. The arg helpers read `backend_port::get()`
            // eagerly; building them earlier bakes in the stale default and the
            // proxy ends up trying to bind the foreign-held port.
            // Use the console_scripts entrypoint when available to avoid the Python
            // -m double-import RuntimeWarning. Fall back to -m if missing.
            let savings_mode = crate::client_adapters::load_savings_mode();
            repair_console_script_interpreter(&entrypoint, &python)?;
            let startup_variants: Vec<(PathBuf, Vec<String>)> = if entrypoint.exists() {
                vec![
                    (entrypoint, headroom_entrypoint_startup_args(&savings_mode)),
                    (python.clone(), headroom_python_startup_args()),
                ]
            } else {
                vec![(python.clone(), headroom_python_startup_args())]
            };

            for (executable, args) in &startup_variants {
                let variant = if args.is_empty() {
                    "default".to_string()
                } else {
                    sanitize_log_variant(&args.join("-"))
                };
                let log_path = logs_dir.join(format!("headroom-{variant}.log"));
                let log_file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .with_context(|| format!("opening {}", log_path.display()))?;

                // Wrap with `nice` so headroom yields CPU to foreground apps
                // (Claude Code, terminal, etc.) when the machine is contended.
                // On idle systems headroom still runs at full speed.
                let mut command = Command::new("/usr/bin/nice");
                command
                    .arg("-n")
                    .arg("5")
                    .arg(executable)
                    .args(args)
                    .current_dir(&self.runtime.root_dir)
                    .process_group(0)
                    .env("PYTHONNOUSERSITE", "1")
                    .env("PYTHONUNBUFFERED", "1")
                    .env("PYTHONFAULTHANDLER", "1")
                    .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
                    .env("PIP_NO_INPUT", "1")
                    // Force huggingface_hub off the native `hf_xet` downloader.
                    // Its Rust extension can SIGABRT ("Fatal Python error: Aborted")
                    // inside xet_get while pulling kompress-int8.onnx during
                    // eager_load_compressors, killing the interpreter before it
                    // binds the port (Sentry: never opened port within timeout).
                    // The SIGABRT is uncatchable in Python; disabling xet falls
                    // back to the stable HTTPS download path.
                    .env("HF_HUB_DISABLE_XET", "1")
                    .env("HEADROOM_SDK", "headroom-desktop-proxy")
                    .env("HEADROOM_HTTP2", "false")
                    // Disable the HTTP/1.1 keep-alive pool for the upstream
                    // (proxy -> api.anthropic.com) client. Claude Code cancels
                    // streaming requests constantly (ESC, aborted tool calls,
                    // subagent cancellations), which can leave a pooled TLS
                    // connection desynced; reusing it surfaces as
                    // "SSLV3_ALERT_BAD_RECORD_MAC" on the next request. The
                    // proxy's retry path does not catch SSL/RemoteProtocolError,
                    // so the raw error leaks back to the client. Fresh
                    // connection per request avoids reuse of a poisoned socket.
                    .env("HEADROOM_MAX_KEEPALIVE", "0")
                    // Optimization mode. Always token: maximize raw-token savings via
                    // prior-turn compression. (Cache mode and the auth-based auto-switch
                    // were removed; cache mode contributed no measurable savings over
                    // Claude Code's native prefix caching.)
                    .env("HEADROOM_MODE", "token")
                    // Enable plain user-message text compression in addition to
                    // tool results. Primary motive is Codex/OpenAI: with a 0.5
                    // read-discount and 0.0 write-penalty, compressing user text
                    // clears the force-compress threshold and yields real savings.
                    // This is the only desktop-side lever (the `headroom proxy`
                    // entrypoint reads this env only; it exposes no CLI flag), and
                    // it is process-global on the single shared proxy. Anthropic
                    // blast radius is small: its 0.9 read-discount means already-
                    // cached content almost never busts the frozen prefix, and
                    // protect_recent/min_tokens guard the just-typed prompt.
                    .env("HEADROOM_COMPRESS_USER_MESSAGES", "1")
                    // Output-token shaping (new in headroom-ai 0.27.0). The proxy
                    // never emits output tokens, so this works request-side: it
                    // appends a byte-stable verbosity instruction to the TAIL of
                    // the system prompt (after the cache_control breakpoint, so the
                    // provider prefix cache is preserved) and lowers an
                    // already-present output_config.effort on mechanically-classified
                    // turns. Off by default upstream; enabled here. Effort router
                    // and mechanical-effort use upstream defaults (on, "low"). The
                    // shaper only ever lowers an effort the client already sent and
                    // never toggles thinking.type, so it cannot 400 a model that
                    // lacks effort support.
                    .env("HEADROOM_OUTPUT_SHAPER", "1")
                    // Pin the steering level explicitly. An explicit env is the
                    // manual-override tier in the shaper's level resolution, so it
                    // wins over the per-user learned level written to verbosity.json
                    // by the baseline-seeding `learn --verbosity` run. That keeps
                    // steering uniform/predictable across users while the seeded
                    // baseline still feeds the /stats savings estimate. Level 2 =
                    // skip pre/postamble, don't restate in-context code/tool output.
                    .env("HEADROOM_VERBOSITY_LEVEL", "2");
                apply_savings_mode_env(&mut command, &savings_mode);
                let mut child = command
                    .stdin(Stdio::null())
                    .stdout(Stdio::from(
                        log_file
                            .try_clone()
                            .with_context(|| format!("cloning {}", log_path.display()))?,
                    ))
                    .stderr(Stdio::from(log_file))
                    .spawn()
                    .with_context(|| {
                        format!(
                            "starting headroom background process: {} {}",
                            executable.display(),
                            args.join(" ")
                        )
                    })?;

                let mut startup_ok = false;
                let mut reason: Option<String> = None;

                let startup_polls = (HEADROOM_STARTUP_TIMEOUT_MS / HEADROOM_STARTUP_POLL_MS).max(1);
                for _ in 0..startup_polls {
                    thread::sleep(Duration::from_millis(HEADROOM_STARTUP_POLL_MS));
                    if is_local_proxy_reachable() {
                        startup_ok = true;
                        break;
                    }

                    match child.try_wait() {
                        Ok(Some(status)) => {
                            reason = Some(format!(
                                "exited with status {} before opening port {}",
                                status,
                                headroom_proxy_port()
                            ));
                            break;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            reason = Some(format!("wait check failed: {}", err));
                            break;
                        }
                    }
                }

                if startup_ok {
                    return Ok(child);
                }

                // Timeout path (process still alive, port never opened): send SIGABRT
                // so PYTHONFAULTHANDLER=1 dumps all-thread tracebacks to the log file
                // before the process dies. Skip if the process already exited on its own.
                if reason.is_none() {
                    let _ = Command::new("/bin/kill")
                        .arg("-ABRT")
                        .arg(child.id().to_string())
                        .status();
                    thread::sleep(Duration::from_millis(500));
                }

                let _ = child.kill();
                let _ = child.wait();

                let reason = reason.unwrap_or_else(|| {
                    format!(
                        "never opened port {} within {}ms",
                        headroom_proxy_port(),
                        HEADROOM_STARTUP_TIMEOUT_MS
                    )
                });
                failures.push(HeadroomStartupFailure {
                    program: executable.display().to_string(),
                    args: args.iter().map(|s| s.to_string()).collect(),
                    log_path: log_path.display().to_string(),
                    log_tail: tail_log_file(&log_path, 80),
                    reason,
                });
            }

            // All variants failed. If the proxy crashed because the venv has a
            // pydantic / pydantic-core skew (e.g. a partial upgrade left
            // pydantic-core ahead of pydantic), pin pydantic-core back to the
            // version pydantic asks for and retry once. The error message itself
            // tells us the required version — see extract_required_pydantic_core_version.
            if allow_repair {
                if let Some(target) = failures
                    .iter()
                    .find_map(|f| extract_required_pydantic_core_version(&f.log_tail))
                {
                    log::warn!(
                        "headroom proxy failed with pydantic-core/pydantic skew; \
                     reinstalling pydantic-core=={target} and retrying"
                    );
                    match self.repair_pydantic_core(&target) {
                        Ok(()) => {
                            log::warn!("pydantic-core repair succeeded; retrying headroom startup");
                            allow_repair = false;
                            continue 'attempt;
                        }
                        Err(repair_err) => {
                            log::error!("pydantic-core repair failed: {repair_err:#}");
                        }
                    }
                }
            }

            let last = failures
                .pop()
                .expect("at least one startup variant attempted");
            let prior_summary = if failures.is_empty() {
                String::new()
            } else {
                let joined = failures
                    .iter()
                    .map(|f| format!("{} {} {}", f.program, f.args.join(" "), f.reason))
                    .collect::<Vec<_>>()
                    .join("; ");
                format!(" (prior attempts: {})", joined)
            };
            return Err(anyhow::Error::from(last).context(format!(
                "unable to keep headroom running in background{}",
                prior_summary
            )));
        }
    }

    pub fn latest_tool_log_path(&self, tool_id: &str) -> Option<PathBuf> {
        let logs_dir = self.runtime.logs_dir();
        let entries = std::fs::read_dir(&logs_dir).ok()?;
        let prefix = format!("{tool_id}-");
        let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with(&prefix) && name.ends_with(".log"))
                    .unwrap_or(false)
            })
            .filter_map(|path| {
                let modified = std::fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .ok()?;
                Some((modified, path))
            })
            .collect();

        candidates.sort_by_key(|(modified, _)| *modified);
        candidates.last().map(|(_, path)| path.clone())
    }

    pub fn read_headroom_log_tail(&self, max_lines: usize) -> Result<Vec<String>> {
        self.read_tool_log_tail("headroom", max_lines)
    }

    pub fn read_tool_log_tail(&self, tool_id: &str, max_lines: usize) -> Result<Vec<String>> {
        let Some(path) = self.latest_tool_log_path(tool_id) else {
            return Ok(Vec::new());
        };

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let lines = content
            .lines()
            .rev()
            .take(max_lines)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|line| line.to_string())
            .collect();
        Ok(lines)
    }

    fn latest_tool_log_marker_state(
        &self,
        tool_id: &str,
        enabled_markers: &[&str],
        disabled_markers: &[&str],
    ) -> Option<bool> {
        let path = self.latest_tool_log_path(tool_id)?;
        self.scan_file_for_marker_state_cached(tool_id, &path, enabled_markers, disabled_markers)
    }

    pub(super) fn scan_file_for_marker_state_cached(
        &self,
        cache_key: &str,
        path: &Path,
        enabled_markers: &[&str],
        disabled_markers: &[&str],
    ) -> Option<bool> {
        let modified = std::fs::metadata(path).ok()?.modified().ok()?;

        {
            let cache = self.log_marker_cache.lock();
            if let Some(cached) = cache.as_ref() {
                if cached.tool_id == cache_key && cached.path == path && cached.modified == modified
                {
                    return cached.result;
                }
            }
        }

        let content = std::fs::read_to_string(path).ok()?;

        let mut result: Option<bool> = None;
        for line in content.lines().rev() {
            let lowered = line.to_ascii_lowercase();
            if enabled_markers
                .iter()
                .any(|marker| lowered.contains(marker))
            {
                result = Some(true);
                break;
            }
            if disabled_markers
                .iter()
                .any(|marker| lowered.contains(marker))
            {
                result = Some(false);
                break;
            }
        }

        let mut cache = self.log_marker_cache.lock();
        *cache = Some(ToolLogMarkerCache {
            tool_id: cache_key.to_string(),
            path: path.to_path_buf(),
            modified,
            result,
        });

        result
    }

    pub fn headroom_mcp_configured(&self) -> Option<bool> {
        self.read_headroom_receipt()?
            .get("mcp")?
            .get("configured")?
            .as_bool()
    }

    pub fn headroom_mcp_error(&self) -> Option<String> {
        self.read_headroom_receipt()?
            .get("mcp")?
            .get("error")?
            .as_str()
            .map(|value| value.to_string())
    }

    pub fn headroom_mcp_install_method(&self) -> Option<String> {
        self.read_headroom_receipt()?
            .get("mcp")?
            .get("installMethod")?
            .as_str()
            .map(|value| value.to_string())
    }

    pub fn headroom_ml_installed(&self) -> Option<bool> {
        self.read_headroom_receipt()?
            .get("ml")?
            .get("installed")?
            .as_bool()
    }

    pub fn headroom_kompress_enabled(&self) -> Option<bool> {
        // The `headroom` Python package attaches a RotatingFileHandler to its
        // `headroom` root logger with `propagate = False` (see helpers.py:
        // `_setup_file_logging`). Proxy-logger INFO lines — including the
        // `Kompress: ENABLED/not installed/disabled` startup markers — go to
        // `~/.headroom/logs/proxy.log` only, never to the stderr stream that
        // our Tauri-spawned log captures. Probe that file first; fall back to
        // the spawn-time tool log (covers older headroom versions that do
        // propagate to stderr).
        // Positive markers: the startup `Kompress: ENABLED` line (cache hit at
        // eager-preload) AND the lazy-load success lines emitted on first use
        // when the model was downloaded after a cold-cache startup. The scan
        // returns the most recent marker, so a lazy load flips the status to
        // enabled without waiting for a backend restart.
        const KOMPRESS_ENABLED_MARKERS: &[&str] = &[
            "kompress: enabled",
            "kompress onnx loaded",
            "kompress pytorch loaded",
        ];
        const KOMPRESS_DISABLED_MARKERS: &[&str] =
            &["kompress: not installed", "kompress: disabled"];
        if let Some(path) = headroom_propagated_proxy_log_path() {
            if let Some(state) = self.scan_file_for_marker_state_cached(
                "headroom-proxy-log",
                &path,
                KOMPRESS_ENABLED_MARKERS,
                KOMPRESS_DISABLED_MARKERS,
            ) {
                return Some(state);
            }
        }

        self.latest_tool_log_marker_state(
            "headroom",
            KOMPRESS_ENABLED_MARKERS,
            KOMPRESS_DISABLED_MARKERS,
        )
    }

    /// True if the Kompress model snapshot is already present in the
    /// HuggingFace hub cache (`$HOME/.cache/huggingface/hub/
    /// models--chopratejas--kompress-v2-base/snapshots/<rev>`). Used as the
    /// prefetch idempotency guard so we never re-download an existing model.
    pub fn kompress_model_cached(&self) -> bool {
        let Some(home) = dirs::home_dir() else {
            return false;
        };
        let snapshots = home
            .join(".cache")
            .join("huggingface")
            .join("hub")
            .join("models--chopratejas--kompress-v2-base")
            .join("snapshots");
        std::fs::read_dir(&snapshots)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
    }

    /// Download the Kompress model (~260MB) into the HF cache by running the
    /// bundled venv python's loader with network enabled. Blocks until the
    /// download finishes — call this on a background thread. Output is captured
    /// to `logs/kompress-prefetch.log`.
    ///
    /// This front-loads the download the proxy would otherwise do lazily on
    /// first request, so a fresh install has ML compression ready before any
    /// traffic. It is best-effort: on failure the proxy's own lazy-load path
    /// still downloads on first use. On a non-zero exit the returned
    /// [`KompressPrefetchOutcome::Failed`] carries a short, Sentry-friendly
    /// cause read from the tail of the prefetch log.
    pub fn prefetch_kompress_model(&self) -> Result<KompressPrefetchOutcome> {
        let python = self.managed_python();
        if !python.exists() {
            bail!("headroom managed python not found at {}", python.display());
        }
        let logs_dir = self.runtime.logs_dir();
        std::fs::create_dir_all(&logs_dir)
            .with_context(|| format!("creating {}", logs_dir.display()))?;
        let log_path = logs_dir.join("kompress-prefetch.log");
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("opening {}", log_path.display()))?;

        let status = Command::new(&python)
            .arg("-c")
            .arg(
                "from headroom.transforms.kompress_compressor import KompressCompressor; \
                 KompressCompressor().preload(allow_download=True)",
            )
            .current_dir(&self.runtime.root_dir)
            .env("PYTHONNOUSERSITE", "1")
            .env("PYTHONUNBUFFERED", "1")
            // Same xet guard as the proxy spawn: the native hf_xet downloader
            // can SIGABRT mid-pull; the HTTPS fallback is stable.
            .env("HF_HUB_DISABLE_XET", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::from(
                log_file
                    .try_clone()
                    .with_context(|| format!("cloning {}", log_path.display()))?,
            ))
            .stderr(Stdio::from(log_file))
            .status()
            .with_context(|| format!("running kompress prefetch via {}", python.display()))?;

        if status.success() {
            Ok(KompressPrefetchOutcome::Downloaded)
        } else {
            Ok(KompressPrefetchOutcome::Failed {
                cause: summarize_kompress_prefetch_failure(&log_path),
            })
        }
    }

    fn read_headroom_receipt(&self) -> Option<Value> {
        let path = self.runtime.tools_dir.join("headroom.json");
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Returns the installed Headroom version from the tool receipt, if any.
    pub fn installed_headroom_version(&self) -> Option<String> {
        self.read_headroom_receipt()?
            .get("version")?
            .as_str()
            .map(|v| v.to_string())
    }

    pub(super) fn installed_requirements_lock_sha(&self) -> Option<String> {
        self.read_headroom_receipt()?
            .get("artifact")?
            .get("requirementsLockSha256")?
            .as_str()
            .map(|v| v.to_string())
    }

    /// Returns the pinned release if the installed version differs from the pin.
    pub fn check_headroom_upgrade(&self) -> Option<HeadroomRelease> {
        let installed = self.installed_headroom_version()?;
        if installed == HEADROOM_PINNED_VERSION {
            return None;
        }
        Some(HeadroomRelease {
            version: HEADROOM_PINNED_VERSION.into(),
            wheel_url: HEADROOM_PINNED_WHEEL_URL.into(),
            sha256: HEADROOM_PINNED_SHA256.into(),
        })
    }

    /// Returns true if the compiled requirements lock differs from what was
    /// used during the last headroom install.
    ///
    /// As a side effect, if the stored sha is a known legacy value whose
    /// pinned versions are byte-identical to the current lock, rewrites the
    /// receipt with the new-format sha and returns false. This avoids a
    /// purely cosmetic reinstall on the 0.2.50 → 0.3.0 jump.
    pub fn requirements_are_stale(&self) -> bool {
        let Some(stored) = self.installed_requirements_lock_sha() else {
            return true;
        };
        let current = requirements_lock_sha(bootstrap_requirements_lock());
        if stored == current {
            return false;
        }
        if LEGACY_REQUIREMENTS_LOCK_SHAS.contains(&stored.as_str()) {
            if let Err(err) = self.write_requirements_lock_sha_to_receipt(&current) {
                log::warn!("failed to migrate legacy requirementsLockSha256: {err}");
            }
            return false;
        }
        true
    }

    fn write_requirements_lock_sha_to_receipt(&self, sha: &str) -> Result<()> {
        let receipt_path = self.runtime.tools_dir.join("headroom.json");
        let bytes = std::fs::read(&receipt_path)
            .with_context(|| format!("reading {}", receipt_path.display()))?;
        let mut receipt: Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", receipt_path.display()))?;
        if let Some(artifact) = receipt.get_mut("artifact").and_then(|a| a.as_object_mut()) {
            artifact.insert("requirementsLockSha256".into(), json!(sha));
        } else {
            return Ok(());
        }
        std::fs::write(&receipt_path, serde_json::to_vec(&receipt)?)
            .with_context(|| format!("writing {}", receipt_path.display()))?;
        Ok(())
    }

    pub fn repair_stale_requirements_with_progress<F>(&self, mut progress: F) -> Result<()>
    where
        F: FnMut(BootstrapStepUpdate),
    {
        let requirements_lock = bootstrap_requirements_lock();
        let lock_path = self.write_headroom_requirements_lock(requirements_lock)?;

        progress(BootstrapStepUpdate {
            step: "Repairing dependencies",
            message: "Repairing Headroom's bundled dependencies.".into(),
            eta_seconds: 60,
            percent: 40,
        });

        let deps_start = Instant::now();
        let progress_ref = std::cell::RefCell::new(&mut progress);
        let mut dep_counter: u32 = 0;
        run_pip_install_with_retries_streaming(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--find-links",
                VENDOR_WHEELS_INDEX_URL,
                "--extra-index-url",
                "https://pypi.org/simple",
                "--upgrade",
                "--requirement",
                lock_path.to_string_lossy().as_ref(),
            ],
            &self.runtime.root_dir,
            |line| {
                if let Some(update) =
                    pip_line_to_progress(line, deps_start.elapsed(), &mut dep_counter, 40, 82)
                {
                    if let Ok(mut cb) = progress_ref.try_borrow_mut() {
                        (cb)(BootstrapStepUpdate {
                            step: "Repairing dependencies",
                            message: update.message,
                            eta_seconds: update.eta_seconds,
                            percent: update.percent,
                        });
                    }
                }
            },
        )
        .context("repairing stale headroom requirements")?;

        progress(BootstrapStepUpdate {
            step: "Configuring integrations",
            message: "Setting up Headroom MCP integration.".into(),
            eta_seconds: 5,
            percent: 88,
        });

        let mcp_install = match self.install_headroom_mcp() {
            Ok(method) => json!({
                "configured": true,
                "proxyUrl": HEADROOM_PROXY_URL,
                "installMethod": method.as_str(),
            }),
            Err(err) => {
                log::info!("headroom MCP setup skipped during repair: {err:#}");
                json!({ "configured": false, "proxyUrl": HEADROOM_PROXY_URL, "error": err.to_string() })
            }
        };

        self.update_headroom_receipt_after_requirements_repair(
            requirements_lock_sha(requirements_lock),
            mcp_install,
        )?;

        progress(BootstrapStepUpdate {
            step: "Repair complete",
            message: "Headroom dependency repair finished.".into(),
            eta_seconds: 0,
            percent: 95,
        });

        Ok(())
    }
}
