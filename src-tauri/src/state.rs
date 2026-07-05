use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Arc;

use parking_lot::Mutex;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::activity_facts::{ActivityFacts, WeeklyTotals};
use crate::analytics;
use crate::bearer::{BearerToken, BEARER_TOKEN_TTL};
pub(crate) use crate::claude_sessions::tail_lines;
use crate::claude_sessions::{
    build_claude_code_project, claude_projects_dir, decode_project_folder_name,
    extract_cwd_from_session_file, list_session_jsonl_files, project_display_name,
    ClaudeProjectScan,
};
use crate::client_adapters::{
    detect_clients, ensure_rtk_integrations, is_rtk_disabled, rtk_integration_status,
};
use crate::insights::generate_daily_insights;
use crate::models::{
    ActivityEvent, BackendRuntimeStatus, BootstrapProgress, ClaudeAccountProfile,
    ClaudeCodeProject, ClientStatus, CodexAccountProfile, CodexRateLimitSnapshot, DailyInsight,
    DailySavingsPoint, DashboardState, HeadroomLearnPrereqStatus, HeadroomLearnStatus,
    HourlySavingsPoint, LaunchAgentRuntimeStatus, LaunchExperience, RtkRuntimeStatus,
    RuntimeStatus, RuntimeUpgradeFailure, RuntimeUpgradeProgress, SavingsAttributionConfidence,
    SavingsAttributionCounter, SavingsAttributionEvent, SavingsAttributionScope,
    SavingsAttributionSource, SwitchboardMode, TransformationFeedEvent, UpgradeFailurePhase,
    UsageEvent,
};
use crate::pricing;
use crate::runtime_boot_validation::boot_validation_message;
pub use crate::runtime_boot_validation::BootValidationOutcome;
pub(crate) use crate::runtime_boot_validation::{log_mtime_advanced, newest_proxy_log_mtime};
use crate::runtime_probe::{intercept_port_accepts_connection, proxy_port_accepts_connection};
use crate::startup_error::classify_startup_error;
use crate::storage::{app_data_dir, config_file, ensure_data_dirs, telemetry_file};
use crate::tool_manager::{
    BootstrapStepUpdate, ManagedRuntime, RtkGainSummary, RuntimeMaintenanceKind, ToolManager,
};

mod launch_profile;
mod repo_memory_mcp;
mod runtime_maintenance;
use launch_profile::{
    persist_last_known_good_plan, persist_launch_profile, LastKnownGoodPlan, LaunchProfile,
};
use repo_memory_mcp::{
    repo_memory_mcp_service_healthy, repo_memory_mcp_supervision_status, RepoMemoryMcpSessionState,
};
use runtime_maintenance::{
    emit_runtime_upgrade_progress, PostSpawnSnapshot, RuntimeMaintenancePlan,
};

/// After this many consecutive failed auto-attempts at the same app version,
/// we stop auto-retrying and surface a persistent banner with a Retry button.
pub const MAX_UPGRADE_AUTO_RETRIES: u32 = 2;

/// Current Terms-of-Service version the user must have accepted to use the app.
/// BUMP THIS whenever the bundled Mac AI Switchboard Terms of Use change: a release
/// shipping a higher value forces every user to re-accept on first launch,
/// because their locally-stored `accepted_terms_version` will be lower.
pub const REQUIRED_TERMS_VERSION: u32 = 2;

/// Deprecated dashboard field retained for older frontends; Terms are bundled in-app.
pub const TERMS_URL: &str = "";

const CAVEMAN_TEMPLATE_BASELINE_TOKENS: u64 = 480;
const CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS: u64 = 180;
const PONYTAIL_TEMPLATE_BASELINE_TOKENS: u64 = 1_400;
const PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS: u64 = 520;
const MARKITDOWN_TEMPLATE_BASELINE_TOKENS: u64 = 3_200;
const MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS: u64 = 900;

/// Absolute maximum time we'll wait for the new proxy to come up during
/// boot validation, regardless of observed activity. Bounded so an
/// indefinitely-hung process is still detected eventually. Adaptive stall
/// detection (below) normally fires long before this.
pub const RUNTIME_UPGRADE_BOOT_MAX_SECS: u64 = 600;

/// Once this much wall-time has elapsed without /livez success, start
/// checking the proxy log's mtime (and the HF cache size) for progress.
/// Before this, we stay quiet — most fast boots finish well under this
/// threshold.
pub const RUNTIME_UPGRADE_STALL_GRACE_SECS: u64 = 60;

/// If neither the proxy log nor the HF cache has grown in this long
/// (AND we're past the grace period), the proxy is considered stalled
/// and we roll back. Bumped from 45s → 90s after a real first-run upgrade
/// failed: the python process printed its banner, then went silent for
/// ~50s while loading multi-GB ONNX models from the freshly-downloaded
/// HF cache. The log was idle but the proxy was making progress.
pub const RUNTIME_UPGRADE_STALL_SILENCE_SECS: u64 = 90;

#[derive(Debug, Default, Clone)]
pub struct PendingMilestones {
    pub token: Vec<u64>,
}

#[derive(Debug, Default, Clone)]
pub struct ActivityObservation {
    #[allow(dead_code)] // read by tests; production callers discard observations
    pub fresh: Vec<ActivityEvent>,
}

/// One-shot probe of the new proxy. Hits `/livez` on the backend port
/// directly first (bypasses the intercept layer on 6767). Falls back to
/// `/health` for older headroom-ai versions that don't expose `/livez`, then
/// through the intercept layer on 6767 as a last resort — which also succeeds
/// if the proxy is alive but too CPU-saturated to answer a direct probe
/// quickly, since the intercept has its own retry + longer timeout path.
pub struct AppState {
    pub tool_manager: ToolManager,
    pub recent_usage: Mutex<Vec<UsageEvent>>,
    pub headroom_process: Mutex<Option<Child>>,
    lifecycle_lock: Mutex<()>,
    /// Held for the full duration of a runtime upgrade. A second call to
    /// `run_upgrade_with_ui` tries `try_lock` and bails if already held.
    upgrade_lock: Mutex<()>,
    pub runtime_paused: Mutex<bool>,
    /// True when the watchdog auto-paused the runtime after giving up on
    /// restarting a wedged/unreachable proxy — as opposed to a deliberate
    /// user pause (`runtime_paused` with this false). Drives the self-heal
    /// auto-resume loop (only auto-paused runtimes are retried) and the
    /// "stopped unexpectedly" banner. Cleared by any successful resume and by
    /// an explicit user pause.
    pub runtime_auto_paused: AtomicBool,
    pub runtime_starting: Mutex<bool>,
    /// True while an atomic runtime upgrade is running (install + boot validation).
    /// Gates the watchdog from auto-pausing during the ~minutes-long upgrade.
    pub runtime_upgrade_in_progress: Mutex<bool>,
    pub runtime_upgrade_progress: Mutex<RuntimeUpgradeProgress>,
    pub last_startup_error: Mutex<Option<String>>,
    pub bootstrap_progress: Mutex<BootstrapProgress>,
    pub headroom_learn_state: Mutex<HeadroomLearnRuntimeState>,
    /// Last Claude AI OAuth bearer token seen passing through the proxy intercept.
    /// Only populated when the user runs Claude Code authenticated via Claude AI (not API key).
    /// Wrapped in Arc so the proxy_intercept task can share it without going through AppState.
    pub claude_bearer_token: Arc<Mutex<Option<BearerToken>>>,
    /// Latest Codex (OpenAI) rate-limit snapshot captured by the proxy intercept
    /// from `x-codex-*` response headers. Wrapped in Arc so the proxy_intercept
    /// task can update it without going through AppState; read by
    /// `pricing::fetch_codex_usage` to drive the Codex usage gauge.
    pub codex_rate_limits: Arc<Mutex<Option<CodexRateLimitSnapshot>>>,
    /// OpenAI/ChatGPT plan decoded from the latest Codex OAuth bearer JWT seen by
    /// the proxy intercept (`proxy_intercept::decode_codex_plan_tier`). Read by
    /// `pricing::fetch_codex_usage` to pick the recommended upgrade tier.
    pub codex_plan_tier: Arc<Mutex<Option<crate::models::CodexPlanTier>>>,
    /// When true, the Rust intercept on :6767 forwards traffic directly to
    /// api.anthropic.com instead of the Python proxy on :6768. Flipped on by
    /// `enforce_pricing_gate` once a Pro/Max user crosses the disable threshold
    /// without a Headroom subscription, so existing CC sessions stay alive
    /// while optimization is genuinely off.
    pub proxy_bypass: Arc<AtomicBool>,
    /// Codex-only parallel to `proxy_bypass`: when true, the intercept forwards
    /// OpenAI-path (Codex) traffic directly to api.openai.com while leaving the
    /// Python proxy up for Claude. Flipped by `apply_codex_pricing_gate_status`
    /// once a free user crosses the weekly Codex disable threshold, so Codex
    /// gating never pauses Claude optimization for mixed users.
    pub codex_bypass: Arc<AtomicBool>,
    /// Sender used by the Rust intercept to notify the identity worker when it
    /// captures a fresh OAuth bearer. Stored so repair/startup paths can respawn
    /// the 6767 intercept if the thread exits while the app process remains up.
    fresh_bearer_tx: Mutex<Option<crate::proxy_intercept::FreshBearerNotifier>>,
    /// Debounce streak for `codex_bypass`, mirroring `pricing_gate_violation_streak`.
    codex_gate_violation_streak: Arc<AtomicU32>,
    /// Number of consecutive `apply_pricing_gate_status` calls that reported
    /// `optimization_allowed=false` while bypass was off. Acts as a debounce:
    /// the ungated→gated transition only fires once this hits
    /// `PRICING_GATE_DEBOUNCE_POLLS`. Reset to 0 on any ungated poll. Prevents
    /// a single bad pricing read (network blip, brief utilization spike) from
    /// flipping the gate off and back on within minutes.
    pricing_gate_violation_streak: Arc<AtomicU32>,
    launch_profile: Mutex<LaunchProfile>,
    launch_profile_path: std::path::PathBuf,
    last_known_good_plan: Mutex<Option<LastKnownGoodPlan>>,
    last_known_good_plan_path: std::path::PathBuf,
    repo_memory_mcp_state: Mutex<RepoMemoryMcpSessionState>,
    repo_memory_mcp_state_path: std::path::PathBuf,
    savings_tracker: Mutex<SavingsTracker>,
    activity_facts: Mutex<ActivityFacts>,
    cached_clients: Mutex<Option<(Vec<ClientStatus>, Instant)>>,
    cached_headroom_stats: Mutex<Option<(Option<HeadroomDashboardStats>, Instant)>>,
    /// `(history, fetched_at, fresh)` — `fresh` is false when `history` is a
    /// retained last-good value served because the latest fetch failed (proxy
    /// paused/unreachable), so it re-probes on the short miss TTL.
    cached_headroom_history: Mutex<Option<(Option<HeadroomSavingsHistoryResponse>, Instant, bool)>>,
    cached_rtk_gain_summary: Mutex<Option<(Option<RtkGainSummary>, Instant)>>,
    cached_rtk_today_stats: Mutex<Option<(Option<crate::models::RtkTodayStats>, Instant)>>,
    cached_rtk_daily_stats: Mutex<Option<(Option<Vec<crate::models::RtkDailyStats>>, Instant)>>,
    cached_claude_profile: Mutex<Option<(Option<String>, ClaudeAccountProfile, Instant)>>,
    /// TTL-cached Codex identity profile, the Codex analog of
    /// `cached_claude_profile`. Built by `pricing::detect_codex_profile` from
    /// `~/.codex/auth.json` + the live `codex_plan_tier` slot; no network fetch,
    /// so the cache is a plain value + timestamp.
    cached_codex_profile: Mutex<Option<(Option<CodexAccountProfile>, Instant)>>,
    /// When the current run of transient profile-fetch failures began. Set the
    /// first time we suppress a transient error (and serve the last good
    /// profile), cleared on the next successful fetch. Once the run exceeds
    /// `STALE_PROFILE_ESCALATE_AFTER` we stop suppressing and surface the
    /// banner — the token-rotation gap has lasted long enough to be real.
    stale_profile_since: Mutex<Option<Instant>>,
    /// Last `IdentityFingerprint` we successfully posted to
    /// `desktop/grace/start`. Used by the bearer-triggered identity-pusher
    /// worker to skip redundant posts when the same Claude account/plan is
    /// already on file with headroom-web.
    last_pushed_identity_fingerprint: Mutex<Option<crate::pricing::IdentityFingerprint>>,
    /// When we most recently completed a fresh OAuth profile fetch that
    /// returned a *complete* identity (UUID + email + non-Unknown plan
    /// tier). The identity-pusher worker uses this to throttle further
    /// `/api/oauth/profile` calls to ~once per 24 h once we already know
    /// who the user is. `Instant`, so it resets on app restart — first
    /// post-restart bearer always triggers a fresh fetch.
    last_complete_identity_fetch_at: Mutex<Option<Instant>>,
    /// Cached stdout of `headroom memory export`. Shared by every OptimizePanel
    /// that mounts at once — without it, N panels = N Python cold-starts.
    cached_memory_export: Mutex<Option<(String, Instant)>>,
    /// Cached result of `list_claude_code_projects`. Scanning the projects dir,
    /// reading session files, and computing per-project learn metadata is the
    /// main cost of opening the Optimize tab. TTL is short enough that
    /// just-finished learn runs appear promptly once their explicit
    /// invalidation fires.
    cached_claude_code_projects: Mutex<Option<(Vec<ClaudeCodeProject>, Instant)>>,
    /// Cached `detect_headroom_learn_prereq_status`. The Claude CLI location
    /// can't change without explicit user action during a session, and the
    /// fallback shell probe can take up to 2s, so we keep this sticky and
    /// expose an invalidator for the user's "Re-check" button.
    cached_headroom_learn_prereq: Mutex<Option<HeadroomLearnPrereqStatus>>,
    /// Cached `runtime_status()` output. The tray-icon updater, proxy
    /// watchdog, and frontend pollers all ask for runtime status on tight
    /// intervals; each uncached call hits `is_headroom_proxy_reachable`
    /// (blocking HTTP) plus a handful of file stats. A short TTL dedupes
    /// the work across all those callers without any visible lag.
    cached_runtime_status: Mutex<Option<(RuntimeStatus, Instant)>>,
    /// Set once we've kicked off (or skipped) the one-shot Kompress model
    /// prefetch for this app launch, so `maybe_prefetch_kompress` never fires
    /// the ~260MB download more than once per process.
    kompress_prefetch_attempted: AtomicBool,
    /// Latched true the first time native savings history loads this process.
    /// Drives the Home chart's startup loading state so the sparse tracker-only
    /// layer is never shown before the full history merges in.
    savings_history_loaded: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct HeadroomLearnRuntimeState {
    running: bool,
    project_path: Option<String>,
    started_at: Option<chrono::DateTime<Utc>>,
    finished_at: Option<chrono::DateTime<Utc>>,
    success: Option<bool>,
    summary: String,
    error: Option<String>,
    output_tail: Vec<String>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        Self::new_in(app_data_dir())
    }

    pub(crate) fn new_in(base_dir: PathBuf) -> Result<Self> {
        ensure_data_dirs(&base_dir)?;

        let runtime = ManagedRuntime::bootstrap_root(&base_dir);
        let tool_manager = ToolManager::new(runtime);
        let (launch_profile, launch_profile_path) = LaunchProfile::load_or_create(&base_dir)?;
        let (last_known_good_plan, last_known_good_plan_path) = LastKnownGoodPlan::load(&base_dir);
        let repo_memory_mcp_state_path = config_file(&base_dir, "repo-memory-mcp-session.json");
        let repo_memory_mcp_state =
            RepoMemoryMcpSessionState::load(&repo_memory_mcp_state_path).unwrap_or_default();
        let savings_tracker = SavingsTracker::load_or_create(&base_dir)?;
        let activity_facts = ActivityFacts::load_or_create(&base_dir)?;

        let state = Self {
            tool_manager,
            recent_usage: Mutex::new(Vec::new()),
            headroom_process: Mutex::new(None),
            lifecycle_lock: Mutex::new(()),
            upgrade_lock: Mutex::new(()),
            runtime_paused: Mutex::new(false),
            runtime_auto_paused: AtomicBool::new(false),
            runtime_starting: Mutex::new(false),
            runtime_upgrade_in_progress: Mutex::new(false),
            runtime_upgrade_progress: Mutex::new(RuntimeUpgradeProgress {
                running: false,
                complete: false,
                failed: false,
                current_step: "Idle".into(),
                message: String::new(),
                overall_percent: 0,
                from_version: None,
                to_version: None,
            }),
            last_startup_error: Mutex::new(None),
            bootstrap_progress: Mutex::new(BootstrapProgress {
                running: false,
                complete: false,
                failed: false,
                current_step: "Idle".into(),
                message: "Installer has not started.".into(),
                current_step_eta_seconds: 0,
                overall_percent: 0,
            }),
            claude_bearer_token: Arc::new(Mutex::new(None)),
            codex_rate_limits: Arc::new(Mutex::new(None)),
            codex_plan_tier: Arc::new(Mutex::new(None)),
            proxy_bypass: Arc::new(AtomicBool::new(false)),
            codex_bypass: Arc::new(AtomicBool::new(false)),
            fresh_bearer_tx: Mutex::new(None),
            codex_gate_violation_streak: Arc::new(AtomicU32::new(0)),
            pricing_gate_violation_streak: Arc::new(AtomicU32::new(0)),
            headroom_learn_state: Mutex::new(HeadroomLearnRuntimeState {
                running: false,
                project_path: None,
                started_at: None,
                finished_at: None,
                success: None,
                summary: "Select a project to run headroom learn.".into(),
                error: None,
                output_tail: Vec::new(),
            }),
            launch_profile: Mutex::new(launch_profile),
            launch_profile_path,
            last_known_good_plan: Mutex::new(last_known_good_plan),
            last_known_good_plan_path,
            repo_memory_mcp_state: Mutex::new(repo_memory_mcp_state),
            repo_memory_mcp_state_path,
            savings_tracker: Mutex::new(savings_tracker),
            activity_facts: Mutex::new(activity_facts),
            cached_clients: Mutex::new(None),
            cached_headroom_stats: Mutex::new(None),
            cached_headroom_history: Mutex::new(None),
            cached_rtk_gain_summary: Mutex::new(None),
            cached_rtk_today_stats: Mutex::new(None),
            cached_rtk_daily_stats: Mutex::new(None),
            cached_claude_profile: Mutex::new(None),
            cached_codex_profile: Mutex::new(None),
            stale_profile_since: Mutex::new(None),
            last_pushed_identity_fingerprint: Mutex::new(None),
            last_complete_identity_fetch_at: Mutex::new(None),
            cached_memory_export: Mutex::new(None),
            cached_claude_code_projects: Mutex::new(None),
            cached_headroom_learn_prereq: Mutex::new(None),
            cached_runtime_status: Mutex::new(None),
            kompress_prefetch_attempted: AtomicBool::new(false),
            savings_history_loaded: AtomicBool::new(false),
        };

        Ok(state)
    }

    pub fn set_fresh_bearer_notifier(
        &self,
        fresh_bearer_tx: crate::proxy_intercept::FreshBearerNotifier,
    ) {
        *self.fresh_bearer_tx.lock() = Some(fresh_bearer_tx);
    }

    pub fn ensure_proxy_intercept_running(&self) {
        if intercept_port_accepts_connection() {
            return;
        }
        let Some(fresh_bearer_tx) = self.fresh_bearer_tx.lock().clone() else {
            log::warn!("cannot respawn proxy intercept: bearer notifier is not initialized");
            return;
        };
        log::warn!("proxy intercept on 127.0.0.1:6767 is not accepting connections; respawning");
        crate::proxy_intercept::spawn(
            Arc::clone(&self.claude_bearer_token),
            Arc::clone(&self.codex_rate_limits),
            Arc::clone(&self.codex_plan_tier),
            Arc::clone(&self.proxy_bypass),
            Arc::clone(&self.codex_bypass),
            fresh_bearer_tx,
        );
        self.invalidate_runtime_status_cache();
    }

    pub fn warm_runtime_on_launch(&self, app: &tauri::AppHandle) {
        // Always check for a mid-upgrade interrupt first. If the last app
        // run was killed between move-aside and commit, the venv.backup/
        // dir holds the real working environment and the live venv is a
        // partial install. Restore before doing anything else.
        let _ = self.tool_manager.recover_from_interrupted_upgrade();

        if !self.tool_manager.python_runtime_installed() {
            // First-run; start_bootstrap (wizard) handles install.
            return;
        }

        let saved_mode = crate::client_adapters::load_switchboard_mode();
        let wants_headroom = matches!(
            saved_mode,
            Some(SwitchboardMode::Headroom | SwitchboardMode::Full) | None
        );
        let wants_rtk = matches!(
            saved_mode,
            Some(SwitchboardMode::Rtk | SwitchboardMode::Full) | None
        );

        if !wants_headroom {
            self.stop_headroom();
            self.set_runtime_paused(true);
            self.set_runtime_auto_paused(false);
        }

        self.set_runtime_starting(true);
        self.enforce_pricing_gate();
        self.stop_python_if_gated();

        // rtk is pinned to a specific version in source. On an app upgrade the
        // bundled binary on disk can be stale because bootstrap only runs on
        // first-run. Reinstall if the receipt's version doesn't match the
        // pinned version. install_rtk hits GitHub Releases, so this needs
        // network — failure here is logged and we move on.
        match self.tool_manager.ensure_rtk_current() {
            Ok(true) => log::info!("rtk refreshed to pinned version on launch"),
            Ok(false) => {}
            Err(err) => log::warn!("rtk version check on launch failed: {err}"),
        }

        if wants_rtk {
            if let Err(err) = ensure_rtk_integrations(
                &self.tool_manager.rtk_entrypoint(),
                &self.tool_manager.managed_python(),
            ) {
                log::warn!("RTK integrations failed during warm_runtime_on_launch: {err:#}");
            }
        }

        // App-version-triggered atomic runtime upgrade. Replaces the old
        // receipt-vs-pinned drift path.
        if self.should_run_runtime_upgrade(app) {
            // Auto-trigger never forces rebuild — that's reserved for the
            // user-facing "Retry with full rebuild" recovery flow.
            self.run_upgrade_with_ui(app, false);
        } else {
            // No Python maintenance needed, but the desktop app version may
            // still have moved (cosmetic-only release on the same headroom-ai
            // pin). Without this stamp the launch profile drifts: every
            // version in the chain that ships the same headroom-ai never
            // gets recorded, and `previous_app_version` reads back as
            // whatever desktop version most recently changed the Python pin
            // — which can be many releases stale.
            let current_app_version = app.package_info().version.to_string();
            if self.can_stamp_no_maintenance(&current_app_version) {
                self.stamp_app_version(&current_app_version);
            }
        }

        // Independent of the upgrade: if MCP is not configured (e.g. it failed
        // during a prior install), retry it now.
        if wants_headroom {
            if let Err(err) = self.tool_manager.ensure_mcp_configured() {
                // install_headroom_mcp captures rich structured data to Sentry
                // at the failure site; log to file only to avoid a duplicate
                // (and stripped) Sentry event from the FileLogger forwarder.
                log::info!("headroom MCP configuration failed: {err:#}");
            }
        } else {
            self.set_runtime_starting(false);
            self.invalidate_runtime_status_cache();
            return;
        }

        // Seed the output-shaper savings baseline BEFORE starting the proxy.
        // This is the launch path for already-installed users (start_bootstrap
        // only runs the first-install wizard), so without it the seeding never
        // runs after an app update. It must precede proxy start: the recorder
        // loads the baseline once at boot and clobbers a later-written one on
        // flush, so seeding first is what lets the number appear without an app
        // relaunch. Idempotent and bounded; we are already on a background
        // thread, so the one-time scan does not block the UI.
        self.tool_manager.seed_verbosity_baseline_if_needed();

        match self.ensure_headroom_running() {
            Ok(()) => {
                crate::port_conflict::note_proxy_started(app);
            }
            Err(err) => {
                log::debug!("failed to auto-start headroom during app launch: {err}");
                let handled = crate::port_conflict::note_proxy_failed(app, &err, true);
                if !handled {
                    crate::capture_headroom_start_failure(
                        "headroom auto-start failed during launch",
                        &err,
                    );
                }
            }
        }

        // Hold `starting` until the probe `runtime_status()` uses
        // (`is_headroom_proxy_reachable` → 6767/readyz) actually returns true.
        // `wait_for_boot_validation` accepts /livez, which can flip green
        // before /readyz does; clearing `starting` on livez alone opens a
        // window where the UI poller sees !running && !starting and fires
        // the "Headroom isn't running" notification while readiness is still
        // loading.
        //
        // 5-minute ceiling: cold-boot in the Python proxy synchronously warms
        // an ONNX embedder (hf_hub_download of all-MiniLM-L6-v2), which on
        // first launch or with a slow network can hold /readyz at 503 for
        // 30s+. The old 60s deadline cleared `starting` before /readyz came
        // up, letting the watchdog auto-pause a process that was about to
        // recover — see Sentry `proxy_unreachable_post_boot`. The loop breaks
        // immediately on reachability, so a longer ceiling has no cost in the
        // happy path; this only changes behavior for genuinely slow boots.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
        while std::time::Instant::now() < deadline {
            if is_headroom_proxy_reachable() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }

        self.set_runtime_starting(false);
    }

    /// Run a full atomic runtime upgrade with UI progress + boot validation.
    ///
    /// Acquires `upgrade_lock` to guard against concurrent launches. Stops
    /// the proxy, runs `atomic_upgrade_headroom`, then validates the new
    /// runtime by waiting for proxy reachability. On boot-validation failure,
    /// rolls back to the previous venv and records a failure so the UI can
    /// render a retry banner.
    ///
    /// `force_rebuild` skips the in-place upgrade attempt and goes straight
    /// to atomic rebuild. Set by the user-facing "Retry with full rebuild"
    /// flow when an in-place upgrade installed cleanly but the proxy
    /// failed to boot — typically an ABI mismatch in native deps that pip
    /// can't detect.
    pub fn run_upgrade_with_ui(&self, app: &tauri::AppHandle, force_rebuild: bool) {
        let _guard = match self.upgrade_lock.try_lock() {
            Some(g) => g,
            None => {
                log::debug!("run_upgrade_with_ui: upgrade already running; skipping");
                return;
            }
        };

        let current_app_version = app.package_info().version.to_string();
        let maintenance_plan =
            match self.runtime_maintenance_plan_for_app_version(&current_app_version) {
                Some(plan) => plan,
                None => {
                    // App version changed but no runtime maintenance is actually
                    // needed — just stamp the version.
                    self.stamp_app_version(&current_app_version);
                    return;
                }
            };
        let maintenance_kind = match &maintenance_plan {
            RuntimeMaintenancePlan::Upgrade(_) => RuntimeMaintenanceKind::Upgrade,
            RuntimeMaintenancePlan::RequirementsRepair => {
                RuntimeMaintenanceKind::RequirementsRepair
            }
        };
        let target_version = match &maintenance_plan {
            RuntimeMaintenancePlan::Upgrade(release) => release.version().to_string(),
            RuntimeMaintenancePlan::RequirementsRepair => self
                .tool_manager
                .installed_headroom_version()
                .unwrap_or_else(|| "unknown".into()),
        };
        let installed_version = self.tool_manager.installed_headroom_version();

        // User-facing from/to are the app versions — headroom-ai versions are
        // an implementation detail tracked in the failure record only.
        let previous_app_version = self.launch_profile.lock().last_launched_app_version.clone();

        // Snapshot the newest proxy log mtime BEFORE we stop the old proxy and
        // install the new one. At failure time we compare against this to tell
        // "the new proxy wrote some logs (so it at least started python)" from
        // "the new proxy never produced any log activity (likely failed to
        // spawn or crashed pre-import)".
        let pre_upgrade_log_mtime = newest_proxy_log_mtime(&self.tool_manager.logs_dir());

        *self.runtime_upgrade_in_progress.lock() = true;
        self.invalidate_runtime_status_cache();

        // Set up progress state + emit initial event.
        self.set_upgrade_progress(|p| {
            p.running = true;
            p.complete = false;
            p.failed = false;
            p.current_step = "Preparing update".into();
            p.message = "Wrapping up the Headroom engine update.".into();
            p.overall_percent = 0;
            p.from_version = previous_app_version.clone();
            p.to_version = Some(current_app_version.clone());
        });
        emit_runtime_upgrade_progress(app, self);

        self.stop_headroom();

        analytics::track_event(
            app,
            "runtime_upgrade_started",
            Some(serde_json::json!({
                "maintenance_kind": match maintenance_kind {
                    RuntimeMaintenanceKind::Upgrade => "upgrade",
                    RuntimeMaintenanceKind::RequirementsRepair => "requirements_repair",
                },
                "from_version": installed_version,
                "to_version": target_version,
                "app_version": current_app_version,
            })),
        );

        let start = std::time::Instant::now();
        let app_for_progress = app.clone();
        // SAFETY: self has a stable address for the duration of this call; the
        // closure runs inline and does not outlive this scope.
        let self_ptr: *const AppState = self as *const AppState;
        let progress = move |step: BootstrapStepUpdate| {
            let state_ref = unsafe { &*self_ptr };
            state_ref.set_upgrade_progress(|p| {
                p.current_step = step.step.to_string();
                p.message = step.message.clone();
                p.overall_percent = step.percent;
            });
            emit_runtime_upgrade_progress(&app_for_progress, state_ref);
        };

        use crate::tool_manager::UpgradeOutcome;
        let needs_commit_or_rollback = matches!(maintenance_kind, RuntimeMaintenanceKind::Upgrade);
        // Ok carries the pip-output tail captured during install — empty
        // string for RequirementsRepair (no pip ran in our wrapper) and for
        // any path that didn't request a capture. Held across the
        // install→boot-validation boundary so a later boot-validation
        // failure can attach it to the Sentry event.
        let install_result: Result<String, (bool, anyhow::Error)> = match maintenance_plan {
            RuntimeMaintenancePlan::Upgrade(release) => {
                match self
                    .tool_manager
                    .atomic_upgrade_headroom(&release, progress, force_rebuild)
                {
                    UpgradeOutcome::InstalledPendingValidation { pip_output_tail } => {
                        Ok(pip_output_tail)
                    }
                    UpgradeOutcome::InstallFailed { restored, error } => Err((restored, error)),
                }
            }
            RuntimeMaintenancePlan::RequirementsRepair => self
                .tool_manager
                .repair_stale_requirements_with_progress(progress)
                .map(|()| String::new())
                .map_err(|error| (false, error)),
        };
        let install_pip_output_tail: String = match install_result {
            Err((restored, error)) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                log::warn!(
                    "run_upgrade_with_ui: install failed after {duration_ms}ms (restored={restored}): {error:#}"
                );
                let restarted = self.ensure_headroom_running().is_ok();
                let hint = crate::classify_upgrade_error(&error);
                let fallback_hint = match maintenance_kind {
                    RuntimeMaintenanceKind::Upgrade if restored && restarted => {
                        Some("Restarted the Headroom engine with the previous runtime.".into())
                    }
                    RuntimeMaintenanceKind::Upgrade if restored => {
                        Some("Restored the previous runtime, but the Headroom engine still needs a manual restart.".into())
                    }
                    RuntimeMaintenanceKind::Upgrade => {
                        Some("Headroom engine update failed and the previous runtime could not be restored automatically.".into())
                    }
                    RuntimeMaintenanceKind::RequirementsRepair if restarted => {
                        Some("Restarted the Headroom engine with the existing runtime.".into())
                    }
                    RuntimeMaintenanceKind::RequirementsRepair => {
                        Some("Dependency repair failed and Headroom could not be restarted automatically.".into())
                    }
                };
                self.record_upgrade_failure(RuntimeUpgradeFailure {
                    app_version: current_app_version.clone(),
                    target_headroom_version: target_version.clone(),
                    fallback_headroom_version: installed_version.clone(),
                    failure_phase: UpgradeFailurePhase::Install,
                    attempts: 0, // filled in by record_upgrade_failure
                    first_attempt_at: Utc::now(),
                    last_attempt_at: Utc::now(),
                    error_message: format!("{error:#}"),
                    error_hint: hint.or(fallback_hint),
                    rollback_restored: restored || restarted,
                });
                crate::capture_upgrade_failure(
                    &error,
                    restored,
                    "install",
                    None,
                    Some(duration_ms),
                    Some(target_version.as_str()),
                    installed_version.as_deref(),
                    None,
                    None,
                );
                analytics::track_event(
                    app,
                    "runtime_upgrade_failed",
                    Some(serde_json::json!({
                        "phase": "install",
                        "maintenance_kind": match maintenance_kind {
                            RuntimeMaintenanceKind::Upgrade => "upgrade",
                            RuntimeMaintenanceKind::RequirementsRepair => "requirements_repair",
                        },
                        "attempt": self.upgrade_failure_attempts(&current_app_version),
                        "app_version": current_app_version,
                        "restored": restored,
                        "restarted": restarted,
                        "duration_ms": duration_ms,
                    })),
                );
                self.set_upgrade_progress(|p| {
                    p.running = false;
                    p.complete = false;
                    p.failed = true;
                    p.current_step = "Install failed".into();
                    p.message = match maintenance_kind {
                        RuntimeMaintenanceKind::Upgrade if restored && restarted => {
                            "Headroom engine update couldn't install. The previous runtime was restored and restarted.".into()
                        }
                        RuntimeMaintenanceKind::Upgrade if restored => {
                            "Headroom engine update couldn't install. The previous runtime was restored, but it still needs a restart.".into()
                        }
                        RuntimeMaintenanceKind::Upgrade => {
                            "Headroom engine update couldn't install, and the previous runtime could not be restored automatically.".into()
                        }
                        RuntimeMaintenanceKind::RequirementsRepair if restarted => {
                            "Headroom engine dependency repair failed. Restarted the engine with the existing runtime.".into()
                        }
                        RuntimeMaintenanceKind::RequirementsRepair => {
                            "Headroom engine dependency repair failed, and the engine could not be restarted automatically.".into()
                        }
                    };
                    p.overall_percent = 100;
                });
                emit_runtime_upgrade_progress(app, self);
                *self.runtime_upgrade_in_progress.lock() = false;
                self.invalidate_runtime_status_cache();
                return;
            }
            Ok(tail) => tail,
        };

        // Boot validation: start the proxy and wait for reachability.
        self.set_upgrade_progress(|p| {
            p.current_step = "Verifying update".into();
            p.message =
                "Launching updated Headroom. This can take a minute — Headroom may need to download new ML models.".into();
            p.overall_percent = 97;
        });
        emit_runtime_upgrade_progress(app, self);

        let ensure_err = self
            .ensure_headroom_running()
            .err()
            .map(|err| format!("{err:#}"));
        if let Some(err) = ensure_err.as_deref() {
            log::warn!("run_upgrade_with_ui: new proxy failed to spawn: {err}");
        }
        // Snapshot the conditions that gate ensure_headroom_running so a
        // silent short-circuit ("we returned Ok(()) but never spawned") is
        // attributable in Sentry instead of surfacing as a blank "Stalled".
        let post_spawn = PostSpawnSnapshot {
            tracked_child: self.headroom_process.lock().is_some(),
            python_installed: self.tool_manager.python_runtime_installed(),
            proxy_bypass: self.proxy_bypass.load(std::sync::atomic::Ordering::Acquire),
            pricing_allows_optimization: self.pricing_allows_optimization(),
            runtime_paused: self.runtime_is_paused(),
            proxy_reachable: is_headroom_proxy_reachable(),
            ensure_error: ensure_err,
        };
        log::info!(
            "run_upgrade_with_ui: post-spawn tracked_child={} python_installed={} \
             proxy_bypass={} pricing_allows_optimization={} runtime_paused={} \
             proxy_reachable={} ensure_error={:?}",
            post_spawn.tracked_child,
            post_spawn.python_installed,
            post_spawn.proxy_bypass,
            post_spawn.pricing_allows_optimization,
            post_spawn.runtime_paused,
            post_spawn.proxy_reachable,
            post_spawn.ensure_error,
        );

        let outcome = if !post_spawn.tracked_child && !post_spawn.proxy_reachable {
            // No child to wait on AND nothing already listening on :6768.
            // wait_for_boot_validation would burn ~120s of grace+silence
            // here for nothing — bail with a distinct outcome so the
            // failure path knows it's a non-start, not a hang.
            log::warn!(
                "run_upgrade_with_ui: skipping boot validation — no tracked child and no reachable proxy"
            );
            BootValidationOutcome::NotStarted
        } else {
            let app_for_progress = app.clone();
            let self_ptr_progress: *const AppState = self as *const AppState;
            self.wait_for_boot_validation(move |elapsed, active| {
                let state_ref = unsafe { &*self_ptr_progress };
                let elapsed_secs = elapsed.as_secs();
                let message = boot_validation_message(elapsed_secs, active);
                // Gently creep 97 → 99.5 over the max budget so the bar keeps
                // moving — the user sees *something* happen during long waits.
                let percent = 97
                    + ((elapsed_secs as u128 * 250 / RUNTIME_UPGRADE_BOOT_MAX_SECS as u128).min(250)
                        as u8)
                        / 100;
                state_ref.set_upgrade_progress(|p| {
                    p.message = message;
                    p.overall_percent = percent.min(99);
                });
                emit_runtime_upgrade_progress(&app_for_progress, state_ref);
            })
        };
        let boot_ok = outcome.is_ok();
        let outcome_label = outcome.label();
        let duration_ms = start.elapsed().as_millis() as u64;
        log::debug!(
            "run_upgrade_with_ui: boot validation {outcome_label} after {}s",
            duration_ms / 1000
        );

        if boot_ok {
            if needs_commit_or_rollback {
                if let Err(err) = self.tool_manager.commit_headroom_upgrade() {
                    log::warn!("commit_headroom_upgrade: non-fatal: {err:#}");
                }
            }
            self.stamp_app_version(&current_app_version);
            self.clear_upgrade_failure();
            self.set_upgrade_progress(|p| {
                p.running = false;
                p.complete = true;
                p.failed = false;
                p.current_step = "Done".into();
                p.message = match maintenance_kind {
                    RuntimeMaintenanceKind::Upgrade => {
                        format!(
                            "Headroom engine updated for Mac AI Switchboard {current_app_version}."
                        )
                    }
                    RuntimeMaintenanceKind::RequirementsRepair => {
                        "Headroom engine repair completed.".into()
                    }
                };
                p.overall_percent = 100;
            });
            emit_runtime_upgrade_progress(app, self);
            analytics::track_event(
                app,
                "runtime_upgrade_completed",
                Some(serde_json::json!({
                    "maintenance_kind": match maintenance_kind {
                        RuntimeMaintenanceKind::Upgrade => "upgrade",
                        RuntimeMaintenanceKind::RequirementsRepair => "requirements_repair",
                    },
                    "from_version": installed_version,
                    "to_version": target_version,
                    "duration_ms": duration_ms,
                })),
            );
            analytics::set_headroom_ai_version(app, self.tool_manager.installed_headroom_version());
            // ensure_headroom_running's gate guards were suppressed during
            // validation so a gated user's brand-new venv could actually be
            // validated (otherwise we'd commit untested or roll back a
            // perfectly good install). Now that the upgrade has committed,
            // restore the gate state by stopping the validation Python if any
            // gate is asserting Python should be down. Client-side routing is
            // already pointed direct-to-Anthropic by whoever asserted the
            // gate, so the validation Python wasn't receiving traffic anyway.
            let gate_wants_python_down =
                self.proxy_bypass.load(std::sync::atomic::Ordering::Acquire)
                    || !self.pricing_allows_optimization()
                    || self.runtime_is_paused();
            if gate_wants_python_down {
                log::info!(
                    "run_upgrade_with_ui: validation succeeded; stopping validation Python because a gate is active"
                );
                self.stop_headroom();
            }
            *self.runtime_upgrade_in_progress.lock() = false;
            self.invalidate_runtime_status_cache();
            return;
        }

        // Boot validation failed — roll back to the previous venv when we have
        // one, otherwise leave the repaired runtime in place and surface the
        // failure so the next launch can retry.
        log::warn!(
            "run_upgrade_with_ui: boot validation failed ({}); rolling back to {:?}",
            outcome_label,
            installed_version
        );
        // Diagnostics for Sentry — capture before stop_headroom() tears down
        // the tracked child and the proxy port. These three booleans
        // distinguish the failure modes that all surface as "Stalled":
        //   tracked_child=false → ensure_headroom_running silently no-op'd
        //   new_proxy_log_written=false → spawn happened but python never
        //                                 reached the logging setup
        //   proxy_port_bound=false → uvicorn never reached its bind() call
        let new_proxy_log_written = log_mtime_advanced(
            pre_upgrade_log_mtime,
            newest_proxy_log_mtime(&self.tool_manager.logs_dir()),
        );
        let boot_diagnostics = crate::UpgradeBootDiagnostics {
            tracked_child: self.headroom_process.lock().is_some(),
            new_proxy_log_written,
            proxy_port_bound: proxy_port_accepts_connection(),
            python_installed: post_spawn.python_installed,
            proxy_bypass: post_spawn.proxy_bypass,
            pricing_allows_optimization: post_spawn.pricing_allows_optimization,
            runtime_paused: post_spawn.runtime_paused,
            ensure_error: post_spawn.ensure_error.clone(),
            pip_output_tail: install_pip_output_tail.clone(),
        };

        // Capture the tail of the proxy log BEFORE stop_headroom runs — for
        // a process that crashed on its own, we want what was written right
        // before the exit. Skip when no fresh writes happened during this
        // validation window: the on-disk log is from a previous run and is
        // actively misleading (the May 2026 incident showed 30 lines from a
        // healthy proxy 16h before the failure).
        let log_tail = if new_proxy_log_written {
            crate::tool_manager::newest_proxy_log_path(&self.tool_manager.logs_dir())
                .map(|path| crate::tool_manager::tail_log_file(&path, 30))
                .filter(|s| !s.is_empty())
        } else {
            None
        };

        self.stop_headroom();
        let rollback_result = if needs_commit_or_rollback {
            self.tool_manager.rollback_headroom_upgrade()
        } else {
            Ok(())
        };
        let rollback_restored = needs_commit_or_rollback && rollback_result.is_ok();
        if let Err(err) = rollback_result {
            log::error!("run_upgrade_with_ui: rollback failed: {err:#}");
        }
        analytics::set_headroom_ai_version(app, self.tool_manager.installed_headroom_version());
        let restarted = self.ensure_headroom_running().is_ok();

        let err_msg = match log_tail.as_deref() {
            Some(tail) => format!(
                "Headroom maintenance for app {} failed boot validation ({}, ran {}ms; internal headroom-ai target: {}, fallback: {:?}).\n\n--- last proxy log lines ---\n{}",
                current_app_version,
                outcome_label,
                duration_ms,
                target_version,
                installed_version,
                tail
            ),
            None => format!(
                "Headroom maintenance for app {} failed boot validation ({}, ran {}ms; internal headroom-ai target: {}, fallback: {:?}).\n\n(no new proxy log lines written during validation window)",
                current_app_version,
                outcome_label,
                duration_ms,
                target_version,
                installed_version
            ),
        };
        // Info-level: capture_upgrade_failure below fires a fully-tagged
        // Level::Error Sentry event with target/fallback versions, log tail,
        // boot diagnostics, and pip output. A warn! here would just produce
        // a duplicate, less informative event.
        log::info!("run_upgrade_with_ui: {err_msg}");
        let err = anyhow::anyhow!("{}", err_msg);
        // The rollback restores the bundled headroom-ai Python package, not the
        // desktop app itself — so user-facing rollback strings reference the
        // Python target/fallback versions (e.g. 0.20.15 → 0.19.0) rather than
        // the desktop app version (which never reverts).
        let fallback_pkg_label = installed_version
            .clone()
            .unwrap_or_else(|| "the previous version".into());
        let error_hint = match maintenance_kind {
            RuntimeMaintenanceKind::Upgrade if rollback_restored && restarted => Some(format!(
                "Reverted to headroom-ai {} and restarted it.",
                fallback_pkg_label
            )),
            RuntimeMaintenanceKind::Upgrade if rollback_restored => {
                Some(format!("Reverted to headroom-ai {}.", fallback_pkg_label))
            }
            RuntimeMaintenanceKind::RequirementsRepair if restarted => Some(
                "Headroom restarted with the repaired runtime, but validation still failed.".into(),
            ),
            RuntimeMaintenanceKind::RequirementsRepair => None,
            _ => None,
        };
        self.record_upgrade_failure(RuntimeUpgradeFailure {
            app_version: current_app_version.clone(),
            target_headroom_version: target_version.clone(),
            fallback_headroom_version: installed_version.clone(),
            failure_phase: if maintenance_kind == RuntimeMaintenanceKind::Upgrade {
                UpgradeFailurePhase::BootValidation
            } else {
                UpgradeFailurePhase::Install
            },
            attempts: 0,
            first_attempt_at: Utc::now(),
            last_attempt_at: Utc::now(),
            error_message: err_msg.clone(),
            error_hint,
            rollback_restored: rollback_restored || restarted,
        });
        crate::capture_upgrade_failure(
            &err,
            rollback_restored || restarted,
            if maintenance_kind == RuntimeMaintenanceKind::Upgrade {
                "boot_validation"
            } else {
                "requirements_repair_boot_validation"
            },
            Some(outcome_label),
            Some(duration_ms),
            Some(target_version.as_str()),
            installed_version.as_deref(),
            log_tail.as_deref(),
            Some(boot_diagnostics),
        );
        analytics::track_event(
            app,
            "runtime_upgrade_failed",
            Some(serde_json::json!({
                "phase": "boot_validation",
                "maintenance_kind": match maintenance_kind {
                    RuntimeMaintenanceKind::Upgrade => "upgrade",
                    RuntimeMaintenanceKind::RequirementsRepair => "requirements_repair",
                },
                "attempt": self.upgrade_failure_attempts(&current_app_version),
                "app_version": current_app_version,
                "restored": rollback_restored,
                "restarted": restarted,
                "duration_ms": duration_ms,
            })),
        );
        // Reuse the headroom-ai labels constructed above for the error_hint —
        // same rationale: rollback is about the Python package, not the app.
        let target_pkg_label = target_version.clone();
        self.set_upgrade_progress(|p| {
            p.running = false;
            p.complete = false;
            p.failed = true;
            p.current_step = "Update didn't start".into();
            p.message = match maintenance_kind {
                RuntimeMaintenanceKind::Upgrade if rollback_restored && restarted => {
                    format!(
                        "headroom-ai {} installed but didn't start. Reverted to headroom-ai {} and restarted it.",
                        target_pkg_label, fallback_pkg_label
                    )
                }
                RuntimeMaintenanceKind::Upgrade if rollback_restored => {
                    format!(
                        "headroom-ai {} installed but didn't start. Reverted to headroom-ai {}.",
                        target_pkg_label, fallback_pkg_label
                    )
                }
                RuntimeMaintenanceKind::Upgrade => format!(
                    "headroom-ai {} installed but didn't start, and rollback failed. Reinstall from the Dashboard.",
                    target_pkg_label
                ),
                RuntimeMaintenanceKind::RequirementsRepair if restarted => {
                    "Headroom runtime repair finished, but startup validation still failed after restart.".into()
                }
                RuntimeMaintenanceKind::RequirementsRepair => {
                    "Headroom runtime repair finished, but startup validation failed. Reinstall from the Dashboard.".into()
                }
            };
            p.overall_percent = 100;
        });
        emit_runtime_upgrade_progress(app, self);
        *self.runtime_upgrade_in_progress.lock() = false;
        self.invalidate_runtime_status_cache();
    }

    /// Returns true if the tracked Headroom process has DEFINITIVELY exited.
    ///
    /// Only reports exited on `Ok(Some(status))` — i.e., the OS told us the
    /// child reaped. `None` (no tracked child) is NOT treated as exited,
    /// because `ensure_headroom_running` intentionally skips spawning when
    /// the intercept layer already reports the proxy reachable; in that
    /// case there's a live proxy we just don't own the Child handle for.
    /// `Err` (child was reaped by someone else) is also not treated as
    /// exited — the OS-level process may well still be serving traffic.
    pub(crate) fn headroom_process_exited(&self) -> Option<String> {
        let mut guard = self.headroom_process.lock();
        match guard.as_mut() {
            None => None,
            Some(child) => match child.try_wait() {
                Ok(Some(status)) => Some(format!("{status}")),
                Ok(None) => None,
                Err(err) => {
                    log::warn!(
                        "headroom_process_exited: try_wait returned Err (treating as still alive): {err}"
                    );
                    None
                }
            },
        }
    }

    /// True iff we own a tracked proxy child that has not yet exited.
    /// Distinguishes "alive (possibly still cold-booting)" from both
    /// "exited/crashed" and "no tracked child at all". `headroom_process_exited`
    /// collapses the latter two into `None`, but the watchdog needs to tell
    /// them apart: an unreachable backend whose tracked child is still alive is
    /// a download-in-progress worth waiting on, whereas a missing or exited
    /// child is a genuine failure to auto-pause immediately.
    pub(crate) fn tracked_child_alive(&self) -> bool {
        let mut guard = self.headroom_process.lock();
        match guard.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    pub fn launch_count(&self) -> u64 {
        self.launch_profile.lock().launch_count
    }

    pub fn launch_experience_label(&self) -> &'static str {
        match self.launch_profile.lock().launch_experience {
            LaunchExperience::FirstRun => "first_run",
            LaunchExperience::Resume => "resume",
            LaunchExperience::Dashboard => "dashboard",
        }
    }

    pub fn should_present_on_launch(&self) -> bool {
        true
    }

    pub fn setup_wizard_complete(&self) -> bool {
        self.launch_profile.lock().setup_wizard_complete
    }

    pub fn mark_setup_wizard_complete(&self) {
        let mut profile = self.launch_profile.lock();
        if profile.setup_wizard_complete {
            return;
        }
        profile.setup_wizard_complete = true;
        persist_launch_profile(&self.launch_profile_path, &profile);
    }

    pub fn accepted_terms_version(&self) -> u32 {
        self.launch_profile.lock().accepted_terms_version
    }

    pub fn mark_terms_accepted(&self, version: u32) {
        let mut profile = self.launch_profile.lock();
        if profile.accepted_terms_version >= version {
            return;
        }
        profile.accepted_terms_version = version;
        persist_launch_profile(&self.launch_profile_path, &profile);
    }

    pub fn cached_clients(&self) -> Vec<ClientStatus> {
        const TTL: Duration = Duration::from_secs(8);
        let mut cache = self.cached_clients.lock();
        if let Some((ref clients, at)) = *cache {
            if at.elapsed() < TTL {
                return clients.clone();
            }
        }
        let clients = detect_clients();
        *cache = Some((clients.clone(), Instant::now()));
        clients
    }

    pub fn cached_memory_export(&self) -> Option<String> {
        // Long TTL is safe because:
        //   - live-learning deletion explicitly calls `invalidate_memory_export_cache`
        //   - the activity observer background thread keeps the cache warm on an
        //     independent cadence, so cache misses rarely land on the IPC path
        const TTL: Duration = Duration::from_secs(60);
        let cache = self.cached_memory_export.lock();
        if let Some((ref s, at)) = *cache {
            if at.elapsed() < TTL {
                return Some(s.clone());
            }
        }
        None
    }

    pub fn store_memory_export(&self, stdout: String) {
        *self.cached_memory_export.lock() = Some((stdout, Instant::now()));
    }

    pub fn invalidate_memory_export_cache(&self) {
        *self.cached_memory_export.lock() = None;
    }

    /// Returns the captured Claude bearer token if it is still within its TTL.
    /// Returns `None` if no token has been captured or the last capture is
    /// stale — in either case the caller should prompt the user to send a
    /// fresh request through the proxy.
    pub fn current_bearer_token(&self) -> Option<String> {
        self.claude_bearer_token
            .lock()
            .as_ref()
            .and_then(|token| token.value_if_fresh(BEARER_TOKEN_TTL).map(str::to_string))
    }

    pub fn cached_claude_profile(&self) -> ClaudeAccountProfile {
        const TTL: Duration = Duration::from_secs(300);
        // How long a run of transient profile-fetch failures may suppress the
        // banner before we surface it. Comfortably longer than a normal token
        // rotation gap, short enough that a genuinely expired/revoked token
        // still gets the user's attention.
        const STALE_PROFILE_ESCALATE_AFTER: Duration = Duration::from_secs(15 * 60);

        let current_token = self.current_bearer_token();

        {
            let cache = self.cached_claude_profile.lock();
            if let Some((cached_token, profile, at)) = &*cache {
                if *cached_token == current_token && at.elapsed() < TTL {
                    return profile.clone();
                }
            }
        }

        let detection = pricing::detect_claude_profile_uncached(self);
        let profile = detection.profile;
        if pricing::is_identity_complete(&profile) {
            self.record_complete_identity_fetch();
            *self.stale_profile_since.lock() = None;
        }

        // During a token-rotation gap the captured bearer is briefly stale and
        // Anthropic rejects the profile fetch (401/403, or a 5xx/network blip).
        // Rather than flashing an alarming "sign out" banner, keep serving the
        // last identity-complete profile until a fresh bearer flows through and
        // the next fetch succeeds. We re-key it to the current token so repeated
        // UI polls within this gap don't re-hit Anthropic with the stale token.
        //
        // If the failures persist past STALE_PROFILE_ESCALATE_AFTER the gap is
        // no longer a momentary rotation blip, so we stop suppressing and let
        // the real error (and its banner) through.
        if detection.error_is_transient && !pricing::is_identity_complete(&profile) {
            let escalate = {
                let mut since = self.stale_profile_since.lock();
                let started = since.get_or_insert_with(Instant::now);
                started.elapsed() >= STALE_PROFILE_ESCALATE_AFTER
            };
            if !escalate {
                let mut cache = self.cached_claude_profile.lock();
                if let Some((_, prev, _)) = cache.as_ref() {
                    if pricing::is_identity_complete(prev) {
                        let good = prev.clone();
                        *cache = Some((current_token, good.clone(), Instant::now()));
                        return good;
                    }
                }
            }
        }

        let mut cache = self.cached_claude_profile.lock();
        *cache = Some((current_token, profile.clone(), Instant::now()));
        profile
    }

    /// True iff a `desktop/grace/start` post with this exact set of Claude
    /// fields has already been recorded as successful in this session.
    /// Identity-pusher worker uses this to skip repeat posts when the bearer
    /// rotates but the underlying account/plan has not changed.
    pub fn identity_fingerprint_already_pushed(
        &self,
        fp: &crate::pricing::IdentityFingerprint,
    ) -> bool {
        self.last_pushed_identity_fingerprint
            .lock()
            .as_ref()
            .map(|prev| prev == fp)
            .unwrap_or(false)
    }

    /// Mark the given fingerprint as the most recent one we've pushed to
    /// `desktop/grace/start`. Called by the worker after a successful post,
    /// and by the sign-in / activation paths that send the same payload.
    pub fn record_pushed_identity_fingerprint(&self, fp: crate::pricing::IdentityFingerprint) {
        *self.last_pushed_identity_fingerprint.lock() = Some(fp);
    }

    /// True iff a fresh OAuth profile fetch returned a *complete* identity
    /// (UUID + email + non-Unknown plan tier) within `max_age`. The
    /// identity-pusher worker uses this to throttle further OAuth calls.
    pub fn complete_identity_fetched_within(&self, max_age: Duration) -> bool {
        self.last_complete_identity_fetch_at
            .lock()
            .as_ref()
            .map(|at| at.elapsed() < max_age)
            .unwrap_or(false)
    }

    /// Record that we just successfully fetched a complete OAuth identity.
    /// Called from `cached_claude_profile()` whenever a fresh fetch returns
    /// a fully populated profile, so every code path that re-warms the
    /// profile cache contributes to the throttle window.
    fn record_complete_identity_fetch(&self) {
        *self.last_complete_identity_fetch_at.lock() = Some(Instant::now());
    }

    /// The most recent classifier output that was something other than
    /// `Unknown`. Used by the pricing gate to keep applying real thresholds
    /// when a transient OAuth-profile fetch returns sparse fields and the
    /// live classifier returns Unknown.
    pub fn last_known_good_plan_tier(&self) -> Option<crate::models::ClaudePlanTier> {
        self.last_known_good_plan
            .lock()
            .as_ref()
            .map(|p| p.plan_tier.clone())
    }

    /// Persist a classifier result if it carries real signal. Unknown is
    /// silently ignored — it's "we don't know yet", never an authoritative
    /// downgrade.
    pub fn record_known_good_plan_tier(&self, tier: &crate::models::ClaudePlanTier) {
        if matches!(tier, crate::models::ClaudePlanTier::Unknown) {
            return;
        }
        let entry = LastKnownGoodPlan {
            plan_tier: tier.clone(),
            recorded_at: Utc::now(),
        };
        {
            let mut cache = self.last_known_good_plan.lock();
            if let Some(existing) = cache.as_ref() {
                // Same tier as before — skip the disk write to avoid touching
                // the file on every classification refresh.
                if matches!(
                    (&existing.plan_tier, tier),
                    (
                        crate::models::ClaudePlanTier::Free,
                        crate::models::ClaudePlanTier::Free
                    ) | (
                        crate::models::ClaudePlanTier::Pro,
                        crate::models::ClaudePlanTier::Pro
                    ) | (
                        crate::models::ClaudePlanTier::Max5x,
                        crate::models::ClaudePlanTier::Max5x
                    ) | (
                        crate::models::ClaudePlanTier::Max20x,
                        crate::models::ClaudePlanTier::Max20x
                    )
                ) {
                    return;
                }
            }
            *cache = Some(entry.clone());
        }
        persist_last_known_good_plan(&self.last_known_good_plan_path, &entry);
    }

    fn cached_headroom_stats(&self) -> Option<HeadroomDashboardStats> {
        // Dashboard polls at 5s; a 4s TTL caused every poll to miss and
        // re-fetch from the proxy. 12s gives at least one cache hit between
        // dashboard refreshes while keeping session savings visibly fresh.
        const TTL: Duration = Duration::from_secs(12);
        let mut cache = self.cached_headroom_stats.lock();
        if let Some((stats, at)) = cache.as_ref() {
            if at.elapsed() < TTL {
                return stats.clone();
            }
        }
        let stats = fetch_headroom_dashboard_stats();
        *cache = Some((stats.clone(), Instant::now()));
        stats
    }

    fn cached_headroom_history(&self) -> Option<HeadroomSavingsHistoryResponse> {
        // Lifetime history moves slowly — the daily/hourly buckets that drive
        // the Home charts only change a handful of times per minute under
        // active traffic. A 30s TTL absorbs most dashboard polls while still
        // updating the chart's most-recent bucket within one full refresh.
        const TTL: Duration = Duration::from_secs(30);
        // A miss (backend not yet reachable on cold start, or a retained
        // last-good value while the proxy is paused) is cached briefly so the
        // chart resolves/recovers within a few seconds, instead of holding the
        // startup loading state or stale data for a full 30s.
        const MISS_TTL: Duration = Duration::from_secs(3);
        let mut cache = self.cached_headroom_history.lock();
        if let Some((history, at, fresh)) = cache.as_ref() {
            let ttl = if *fresh { TTL } else { MISS_TTL };
            if at.elapsed() < ttl {
                return history.clone();
            }
        }
        match fetch_headroom_savings_history() {
            Some(history) => {
                *cache = Some((Some(history.clone()), Instant::now(), true));
                Some(history)
            }
            None => {
                // Retain the last good history so a transient proxy pause
                // doesn't revert the Home chart to the sparse tracker-only
                // layer. Mark it stale so we re-probe on the short miss TTL and
                // recover quickly once the proxy returns.
                let retained = cache.as_ref().and_then(|(h, _, _)| h.clone());
                *cache = Some((retained.clone(), Instant::now(), false));
                retained
            }
        }
    }

    fn cached_rtk_gain_summary(&self) -> Option<RtkGainSummary> {
        const TTL: Duration = Duration::from_secs(10);
        let mut cache = self.cached_rtk_gain_summary.lock();
        if let Some((stats, at)) = cache.as_ref() {
            if at.elapsed() < TTL {
                return stats.clone();
            }
        }
        let stats = self.tool_manager.rtk_gain_summary();
        *cache = Some((stats.clone(), Instant::now()));
        stats
    }

    fn cached_rtk_today_stats(&self) -> Option<crate::models::RtkTodayStats> {
        const TTL: Duration = Duration::from_secs(10);
        let mut cache = self.cached_rtk_today_stats.lock();
        if let Some((stats, at)) = cache.as_ref() {
            if at.elapsed() < TTL {
                return stats.clone();
            }
        }
        let stats = self.tool_manager.rtk_today_stats();
        *cache = Some((stats.clone(), Instant::now()));
        stats
    }

    fn cached_rtk_daily_stats(&self) -> Option<Vec<crate::models::RtkDailyStats>> {
        const TTL: Duration = Duration::from_secs(10);
        let mut cache = self.cached_rtk_daily_stats.lock();
        if let Some((stats, at)) = cache.as_ref() {
            if at.elapsed() < TTL {
                return stats.clone();
            }
        }
        let stats = self.tool_manager.rtk_daily_stats();
        *cache = Some((stats.clone(), Instant::now()));
        stats
    }

    pub fn dashboard(&self) -> DashboardState {
        // Callers that take this read-only path (tray updater, bootstrap
        // finalize, account activation) must NOT drain pending milestones —
        // doing so silently consumes crossings before `get_dashboard_state`
        // can fire the aptabase event and the in-app notification.
        self.build_dashboard(false).0
    }

    /// Observe a batch of transformations into ActivityFacts (for feed
    /// synthetic-event detection: new-model / daily-record / all-time-record),
    /// persist any changes, and return the emitted synthetic events plus the
    /// current bounded history of recent synthetic events.
    pub fn observe_activity_from_transformations(
        &self,
        transformations: &[TransformationFeedEvent],
    ) -> ActivityObservation {
        let mut facts = self.activity_facts.lock();
        let mut fresh: Vec<ActivityEvent> = Vec::new();
        let mut ordered: Vec<&TransformationFeedEvent> = transformations.iter().collect();
        // Feed arrives newest-first; observe oldest-first so records update in order.
        ordered.sort_by(|a, b| {
            a.timestamp
                .clone()
                .unwrap_or_default()
                .cmp(&b.timestamp.clone().unwrap_or_default())
        });
        for transformation in ordered {
            let observed_at = transformation
                .timestamp
                .as_deref()
                .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            fresh.extend(facts.observe_transformation(transformation, observed_at));
        }

        let _ = facts.save_if_dirty();
        ActivityObservation { fresh }
    }

    pub fn observe_learnings_today(
        &self,
        patterns_today: u32,
        project_inputs: Vec<crate::activity_facts::LearningsProjectInput>,
        active_project_path: Option<&str>,
    ) -> crate::models::LearningsMilestoneEvent {
        let mut facts = self.activity_facts.lock();
        let event = facts.observe_learnings_today(
            patterns_today,
            project_inputs,
            active_project_path,
            Utc::now(),
        );
        let _ = facts.save_if_dirty();
        event
    }

    /// Scan the Claude Code project list for candidates that should be
    /// prompted to run Train. Delegates the decision logic and bookkeeping
    /// (fire-once for never-trained, 7-day cooldown for stale) to
    /// `ActivityFacts::observe_train_suggestions`.
    pub fn observe_train_suggestions(&self, projects: &[ClaudeCodeProject]) -> Vec<ActivityEvent> {
        let mut facts = self.activity_facts.lock();
        let events = facts.observe_train_suggestions(projects, Utc::now());
        let _ = facts.save_if_dirty();
        events
    }

    /// Read-only snapshot of the latest-of-kind slots. The `get_activity_feed`
    /// IPC command wraps this straight into the response; observation runs on
    /// a backend timer and is the sole writer.
    pub fn activity_feed_snapshot(&self) -> crate::models::ActivityFeedSnapshot {
        let mut snapshot = self.activity_facts.lock().activity_feed_snapshot();
        snapshot.rtk_today = self.cached_rtk_today_stats();
        snapshot
    }

    pub fn purge_message_logs(&self) -> crate::models::PurgeResult {
        let mut facts = self.activity_facts.lock();
        facts.reset_for_message_log_purge();
        crate::message_logging::purge_message_logs(facts.path())
    }

    pub fn savings_attribution_events(&self) -> Vec<SavingsAttributionEvent> {
        self.savings_tracker.lock().attribution_events()
    }

    pub fn savings_attribution_counters(&self) -> Vec<SavingsAttributionCounter> {
        aggregate_savings_attribution_counters(&self.savings_tracker.lock().attribution_events())
    }

    pub fn record_repo_intelligence_attribution(
        &self,
        summary: &crate::models::RepoIntelligenceSummary,
    ) -> Result<()> {
        let Some(event) = build_repo_intelligence_attribution_event(summary) else {
            return Ok(());
        };
        self.savings_tracker.lock().append_attribution_event(&event)
    }

    pub fn record_markitdown_attribution(
        &self,
        changed_files: &[String],
        backup_files: &[String],
    ) -> Result<()> {
        let Some(event) = build_addon_attribution_event(
            "markitdown",
            None,
            Some(changed_files),
            Some(backup_files),
            None,
        ) else {
            return Ok(());
        };
        self.savings_tracker.lock().append_attribution_event(&event)
    }

    pub fn record_caveman_attribution(
        &self,
        caveman_level: &str,
        changed_files: &[String],
        backup_files: &[String],
    ) -> Result<()> {
        let Some(event) = build_addon_attribution_event(
            "caveman",
            Some(caveman_level),
            Some(changed_files),
            Some(backup_files),
            None,
        ) else {
            return Ok(());
        };
        self.savings_tracker.lock().append_attribution_event(&event)
    }

    pub fn record_ponytail_attribution(&self, registered_hosts: &[String]) -> Result<()> {
        let Some(event) =
            build_addon_attribution_event("ponytail", None, None, None, Some(registered_hosts))
        else {
            return Ok(());
        };
        self.savings_tracker.lock().append_attribution_event(&event)
    }

    /// Emit a weekly recap rolling up the 7 days ending last Sunday.
    /// Previously Monday-only; now runs on any day whose check is due so the
    /// first launch after an upgrade catches up on last week's recap if it
    /// was missed. Two gates: `weekly_recap_check_due` (once per 24h) and
    /// the per-week key inside `maybe_record_weekly_recap`.
    pub fn maybe_emit_weekly_recap(&self) -> Option<ActivityEvent> {
        let now = Utc::now();
        // Cheap pre-check — skip aggregation entirely if we've already
        // checked within 24h. The callee re-checks defensively.
        if !self.activity_facts.lock().weekly_recap_check_due(now) {
            return None;
        }

        let today = Local::now().date_naive();
        let recap_monday = most_recent_monday(today);
        let start = recap_monday.checked_sub_days(chrono::Days::new(7))?;
        let end = recap_monday.pred_opt()?;

        let totals = {
            let tracker = self.savings_tracker.lock();
            aggregate_weekly_totals(&tracker.daily_savings, start, end)
        };

        let mut facts = self.activity_facts.lock();
        let event = facts.maybe_record_weekly_recap(recap_monday, totals, now);
        let _ = facts.save_if_dirty();
        event
    }

    pub fn record_measured_addon_attribution(
        &self,
        source: SavingsAttributionSource,
        label: &str,
        baseline_tokens: u64,
        optimized_tokens: u64,
        request_delta: usize,
        detail: impl Into<String>,
    ) -> Result<()> {
        if baseline_tokens <= optimized_tokens || request_delta == 0 {
            return Ok(());
        }

        let delta_tokens = baseline_tokens.saturating_sub(optimized_tokens);
        let event = SavingsAttributionEvent {
            schema_version: 1,
            id: Uuid::new_v4().to_string(),
            observed_at: Utc::now(),
            scope: SavingsAttributionScope::Session,
            source,
            confidence: SavingsAttributionConfidence::Measured,
            delta_tokens_saved: delta_tokens,
            delta_usd: 0.0,
            total_tokens_sent: optimized_tokens,
            request_delta,
            evidence: vec![format!(
                "{label} measured {delta_tokens} saved tokens from {baseline_tokens} before to {optimized_tokens} after. {}",
                detail.into()
            )],
        };

        self.savings_tracker.lock().append_attribution_event(&event)
    }

    pub fn dashboard_with_pending_milestones(&self) -> (DashboardState, PendingMilestones) {
        self.build_dashboard(true)
    }

    fn build_dashboard(
        &self,
        drain_pending_milestones: bool,
    ) -> (DashboardState, PendingMilestones) {
        let tools = self.tool_manager.list_tools();
        let clients = self.cached_clients();
        let recent_usage = self.recent_usage.lock().clone();
        let insights = build_insights(
            &recent_usage,
            &clients,
            self.tool_manager.python_runtime_installed(),
        );
        let (mut snapshot, mut daily_savings, mut hourly_savings) = {
            let tracker = self.savings_tracker.lock();
            (
                tracker.snapshot(),
                tracker.daily_savings(),
                tracker.hourly_savings(),
            )
        };
        let mut pending_milestones = PendingMilestones::default();

        let stats = self.cached_headroom_stats();
        let history = self.cached_headroom_history();
        if history.is_some() {
            self.savings_history_loaded
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }

        if let Some(stats) = stats.as_ref() {
            if let Some((updated, updated_daily, updated_hourly, milestones)) =
                self.record_savings_snapshot(stats, drain_pending_milestones)
            {
                snapshot = updated;
                daily_savings = updated_daily;
                hourly_savings = updated_hourly;
                pending_milestones = milestones;
            }
        }

        if let Some(stats) = stats.as_ref() {
            if let Some(requests) = stats.session_requests {
                snapshot.session_requests = requests;
            }
            if let Some(saved_usd) = stats.session_estimated_savings_usd {
                snapshot.session_estimated_savings_usd = saved_usd;
            }
            if let Some(saved_tokens) = stats.session_estimated_tokens_saved {
                snapshot.session_estimated_tokens_saved = saved_tokens;
            }
            if let Some(savings_pct) = stats.session_savings_pct {
                snapshot.session_savings_pct = savings_pct;
            }
        }

        let output_reduction = stats
            .as_ref()
            .and_then(|s| s.output_reduction.as_ref())
            .map(|o| crate::models::OutputReduction {
                method: o.method.clone(),
                reduction_percent: o.reduction_percent,
                ci_low_percent: o.ci_low_percent,
                ci_high_percent: o.ci_high_percent,
                requests: o.requests,
            });

        if let Some(history) = history.as_ref() {
            if let Some(saved_usd) = history.lifetime_estimated_savings_usd {
                snapshot.lifetime_estimated_savings_usd = saved_usd;
            }
            if let Some(saved_tokens) = history.lifetime_estimated_tokens_saved {
                snapshot.lifetime_estimated_tokens_saved = saved_tokens;
            }
            let cutoff_date = savings_history_cutoff_date();
            let cutoff_hour = format!("{cutoff_date}T00:00");
            let native_daily = history.daily_savings();
            let native_hourly = history.hourly_savings();

            // Lock the backend's authoritative settled rollups into the local
            // archive so they survive its history trimming and fill gaps from
            // periods the app wasn't running.
            {
                let today_key = local_day_key(Local::now());
                let mut tracker = self.savings_tracker.lock();
                if tracker.ingest_native_rollups(
                    &native_daily,
                    &native_hourly,
                    &cutoff_date,
                    &today_key,
                ) {
                    let _ = tracker.persist_state();
                }
            }

            daily_savings = merge_daily_savings(daily_savings, native_daily, &cutoff_date);
            hourly_savings = merge_hourly_savings(hourly_savings, native_hourly, &cutoff_hour);
        }

        let (launch_experience, accepted_terms_version) = {
            let profile = self.launch_profile.lock();
            (
                profile.launch_experience.clone(),
                profile.accepted_terms_version,
            )
        };

        (
            DashboardState {
                app_version: "0.0.0".into(),
                launch_experience,
                bootstrap_complete: self.tool_manager.python_runtime_installed(),
                python_runtime_installed: self.tool_manager.python_runtime_installed(),
                lifetime_requests: snapshot.lifetime_requests,
                lifetime_estimated_savings_usd: snapshot.lifetime_estimated_savings_usd,
                lifetime_estimated_tokens_saved: snapshot.lifetime_estimated_tokens_saved,
                session_requests: snapshot.session_requests,
                session_estimated_savings_usd: snapshot.session_estimated_savings_usd,
                session_estimated_tokens_saved: snapshot.session_estimated_tokens_saved,
                session_savings_pct: snapshot.session_savings_pct,
                output_reduction,
                daily_savings,
                hourly_savings,
                savings_history_loaded: self
                    .savings_history_loaded
                    .load(std::sync::atomic::Ordering::Relaxed),
                tools,
                clients,
                recent_usage,
                insights,
                required_terms_version: REQUIRED_TERMS_VERSION,
                accepted_terms_version,
                terms_url: TERMS_URL.to_string(),
            },
            pending_milestones,
        )
    }

    /// Cache TTL for `list_claude_code_projects`. Long enough that rapid tab
    /// switches and pre-warms hit the cache instead of re-scanning the
    /// projects directory. A dedicated background thread
    /// (`spawn_claude_projects_warmer`) keeps this fresh at ~75s cadence so
    /// most Optimize opens still avoid a cold filesystem scan.
    /// Completed learn runs explicitly invalidate via
    /// `invalidate_claude_code_projects_cache`, so staleness isn't a concern
    /// for learn-driven UI updates.
    const CLAUDE_PROJECTS_CACHE_TTL: Duration = Duration::from_secs(90);

    pub fn list_claude_code_projects(&self) -> Result<Vec<ClaudeCodeProject>> {
        if let Some(cached) = self.cached_claude_code_projects_fresh() {
            return Ok(cached);
        }
        let projects = self.list_claude_code_projects_uncached()?;
        *self.cached_claude_code_projects.lock() = Some((projects.clone(), Instant::now()));
        Ok(projects)
    }

    fn cached_claude_code_projects_fresh(&self) -> Option<Vec<ClaudeCodeProject>> {
        let cache = self.cached_claude_code_projects.lock();
        if let Some((ref projects, at)) = *cache {
            if at.elapsed() < Self::CLAUDE_PROJECTS_CACHE_TTL {
                return Some(projects.clone());
            }
        }
        None
    }

    pub fn invalidate_claude_code_projects_cache(&self) {
        *self.cached_claude_code_projects.lock() = None;
    }

    pub fn headroom_learn_prereq_status(&self) -> HeadroomLearnPrereqStatus {
        if let Some(cached) = self.cached_headroom_learn_prereq.lock().clone() {
            return cached;
        }
        let status = crate::learning_commands::detect_headroom_learn_prereq_status();
        *self.cached_headroom_learn_prereq.lock() = Some(status.clone());
        status
    }

    pub fn invalidate_headroom_learn_prereq_cache(&self) {
        *self.cached_headroom_learn_prereq.lock() = None;
    }

    fn list_claude_code_projects_uncached(&self) -> Result<Vec<ClaudeCodeProject>> {
        let projects_dir = claude_projects_dir();
        if !projects_dir.exists() {
            return Ok(Vec::new());
        }

        let mut grouped_projects = BTreeMap::<String, ClaudeProjectScan>::new();
        let entries = std::fs::read_dir(&projects_dir)
            .with_context(|| format!("reading {}", projects_dir.display()))?;

        for entry in entries.filter_map(|item| item.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let folder_name = entry
                .file_name()
                .to_str()
                .map(|value| value.to_string())
                .unwrap_or_default();
            if folder_name.is_empty() || folder_name.starts_with('.') {
                continue;
            }

            let session_files = list_session_jsonl_files(&path);
            if session_files.is_empty() {
                continue;
            }

            let latest_file = session_files
                .iter()
                .max_by_key(|file| {
                    std::fs::metadata(file)
                        .and_then(|meta| meta.modified())
                        .ok()
                })
                .cloned();
            let Some(latest_file) = latest_file else {
                continue;
            };

            let Some(modified) = std::fs::metadata(&latest_file)
                .and_then(|meta| meta.modified())
                .ok()
            else {
                continue;
            };

            let project_path = extract_cwd_from_session_file(&latest_file)
                .unwrap_or_else(|| decode_project_folder_name(&folder_name));
            // Skip ghost projects: `~/.claude/projects/` holds session files
            // for folders that have since been moved or deleted. Falling back
            // to the raw (non-canonical) path surfaces these as live projects,
            // triggers Train suggestions that can never resolve, and — when a
            // ghost shares a basename with a real project — makes the Activity
            // tile look like it's nagging about the working copy.
            let project_path = match std::fs::canonicalize(&project_path) {
                Ok(p) => p.to_string_lossy().into_owned(),
                Err(_) => continue,
            };
            if project_path.trim().is_empty() {
                continue;
            }
            let scan = grouped_projects.entry(project_path).or_default();
            scan.last_worked_at = scan.last_worked_at.max(Some(modified));
            scan.add_session_files(session_files);
        }

        let mut projects = Vec::new();
        for (project_path, scan) in grouped_projects {
            let Some(project) = build_claude_code_project(&self.tool_manager, project_path, scan)
            else {
                continue;
            };
            projects.push(project);
        }

        projects.sort_by(|left, right| right.last_worked_at.cmp(&left.last_worked_at));
        Ok(projects)
    }

    pub fn begin_headroom_learn_run(&self, project_path: &str) -> Result<(), String> {
        if project_path.trim().is_empty() {
            return Err("Select a project before running headroom learn.".into());
        }
        if !self.tool_manager.python_runtime_installed() {
            return Err("Install Headroom runtime before running headroom learn.".into());
        }
        if !self.tool_manager.headroom_entrypoint().exists() {
            return Err("Headroom runtime is not available yet.".into());
        }
        let project = Path::new(project_path);
        if !project.exists() {
            return Err(format!(
                "Project path does not exist: {}",
                project.display()
            ));
        }
        if !project.is_dir() {
            return Err(format!(
                "Project path is not a directory: {}",
                project.display()
            ));
        }

        let mut state = self.headroom_learn_state.lock();
        if state.running {
            return Err("headroom learn is already running.".into());
        }

        state.running = true;
        state.project_path = Some(project_path.to_string());
        state.started_at = Some(Utc::now());
        state.finished_at = None;
        state.success = None;
        state.summary = format!(
            "Running headroom learn for {}.",
            project
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(project_path)
        );
        state.error = None;
        state.output_tail = Vec::new();
        Ok(())
    }

    pub fn complete_headroom_learn_run(
        &self,
        success: bool,
        summary: String,
        error: Option<String>,
        output_tail: Vec<String>,
    ) {
        let mut state = self.headroom_learn_state.lock();
        state.running = false;
        state.finished_at = Some(Utc::now());
        state.success = Some(success);
        state.summary = summary;
        state.error = error;
        state.output_tail = output_tail;
        drop(state);
        // A completed run rewrites CLAUDE.md / MEMORY.md and updates the learn
        // log's mtime, so the cached project list (which depends on both) is
        // now stale — force a fresh scan on the next read.
        self.invalidate_claude_code_projects_cache();
    }

    pub fn headroom_learn_status(
        &self,
        selected_project_path: Option<&str>,
    ) -> HeadroomLearnStatus {
        let state = self.headroom_learn_state.lock().clone();

        let current_project_path = state.project_path.clone();
        let lookup_project_path = selected_project_path
            .map(|path| path.to_string())
            .or_else(|| current_project_path.clone());
        let project_display_name = current_project_path.as_deref().map(project_display_name);
        let last_run_at = lookup_project_path
            .as_deref()
            .and_then(|path| self.tool_manager.headroom_learn_last_run_at(path));
        let started_at = state.started_at.map(|value| value.to_rfc3339());
        let finished_at = state.finished_at.map(|value| value.to_rfc3339());
        let elapsed_seconds = if state.running {
            state
                .started_at
                .map(|started| (Utc::now() - started).num_seconds().max(0) as u64)
        } else {
            match (state.started_at, state.finished_at) {
                (Some(started), Some(finished)) => {
                    Some((finished - started).num_seconds().max(0) as u64)
                }
                _ => None,
            }
        };
        let progress_percent = if state.running {
            let elapsed = elapsed_seconds.unwrap_or(0) as f64;
            (8.0 + (1.0 - (-elapsed / 36.0).exp()) * 84.0).round() as u8
        } else if state.finished_at.is_some() {
            100
        } else {
            0
        };

        HeadroomLearnStatus {
            running: state.running,
            project_path: current_project_path,
            project_display_name,
            started_at,
            finished_at,
            elapsed_seconds,
            progress_percent,
            summary: state.summary,
            success: state.success,
            error: state.error,
            last_run_at,
            output_tail: state.output_tail,
        }
    }

    fn record_savings_snapshot(
        &self,
        stats: &HeadroomDashboardStats,
        drain_pending_milestones: bool,
    ) -> Option<(
        SavingsTotalsSnapshot,
        Vec<DailySavingsPoint>,
        Vec<HourlySavingsPoint>,
        PendingMilestones,
    )> {
        let mut tracker = self.savings_tracker.lock();
        let snapshot = tracker.observe(stats)?;
        let daily_savings = tracker.daily_savings();
        let hourly_savings = tracker.hourly_savings();
        let _ = maybe_append_measured_headroom_attribution(&mut tracker, stats);
        let milestones = if drain_pending_milestones {
            PendingMilestones {
                token: tracker.take_pending_lifetime_token_milestones(),
            }
        } else {
            PendingMilestones::default()
        };
        Some((snapshot, daily_savings, hourly_savings, milestones))
    }

    pub fn bootstrap_progress(&self) -> BootstrapProgress {
        self.bootstrap_progress.lock().clone()
    }

    pub fn begin_bootstrap(&self) -> Result<(), String> {
        let python_installed = self.tool_manager.python_runtime_installed();
        let mut progress = self.bootstrap_progress.lock();
        let (next, result) = begin_bootstrap_transition(&progress, python_installed);
        *progress = next;
        result
    }

    pub fn update_bootstrap_step(&self, step: BootstrapStepUpdate) {
        let mut progress = self.bootstrap_progress.lock();
        *progress = apply_bootstrap_step(&progress, step);
    }

    pub fn mark_bootstrap_proxy_starting(&self) {
        let mut progress = self.bootstrap_progress.lock();
        *progress = BootstrapProgress {
            running: true,
            complete: false,
            failed: false,
            current_step: "Starting Headroom engine".into(),
            message:
                "Starting the Headroom engine for the first time (this can take ~1-2 minutes)…"
                    .into(),
            current_step_eta_seconds: 45,
            overall_percent: 95,
        };
    }

    pub fn mark_bootstrap_complete(&self) {
        let mut progress = self.bootstrap_progress.lock();
        *progress = bootstrap_complete_state();
    }

    pub fn mark_bootstrap_failed<S: Into<String>>(&self, message: S) {
        let mut progress = self.bootstrap_progress.lock();
        *progress = bootstrap_failed_state(&progress, message.into());
    }

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

    fn pricing_allows_optimization(&self) -> bool {
        pricing::get_pricing_status(self)
            .map(|status| status.optimization_allowed)
            .unwrap_or(true)
    }

    /// Flip the bypass flag based on current pricing. Safe to call while
    /// holding `lifecycle_lock` — this never tries to acquire it. Stopping
    /// the Python proxy is `stop_python_if_gated`'s job (it does take the
    /// lock) and must be invoked separately.
    ///
    /// Does NOT touch `client-setup.json`, `~/.claude/settings.json`, or
    /// shell blocks. Those are durable user setup, not runtime state — the
    /// bypass flag alone is enough to make the Rust intercept pass traffic
    /// straight through to api.anthropic.com while Python is down.
    fn enforce_pricing_gate(&self) {
        match pricing::get_pricing_status(self) {
            Ok(status) if !status.optimization_allowed => {
                self.proxy_bypass
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            Ok(_) => {
                self.proxy_bypass
                    .store(false, std::sync::atomic::Ordering::Release);
            }
            Err(_) => {}
        }
    }

    /// Stop the Python proxy when pricing currently disallows optimization.
    /// Acquires `lifecycle_lock`, so callers MUST NOT already hold it.
    fn stop_python_if_gated(&self) {
        if !self.pricing_allows_optimization() {
            self.stop_headroom();
        }
    }

    /// Reconcile the runtime against a freshly evaluated pricing status.
    /// Detects gated→ungated and ungated→gated transitions and runs the
    /// matching side-effects (start/stop the Python proxy, flip the bypass
    /// flag). Idempotent on no-op cases — safe to call from every pricing
    /// poll.
    ///
    /// The ungated→gated transition is debounced: the bypass flip only
    /// fires once `optimization_allowed=false` has been observed for
    /// `PRICING_GATE_DEBOUNCE_POLLS` consecutive polls. The gated→ungated
    /// direction has no debounce — recovery should be immediate.
    ///
    /// Acquires `lifecycle_lock` (via `stop_headroom` / `ensure_headroom_running`),
    /// so callers MUST NOT already hold it.
    pub fn apply_pricing_gate_status(&self, status: &crate::models::HeadroomPricingStatus) {
        let was_bypassed = self.proxy_bypass.load(std::sync::atomic::Ordering::Acquire);
        let should_bypass = !status.optimization_allowed;

        if should_bypass {
            // Once bypassed, the streak is moot — keep it pinned at the
            // threshold so a future gated→ungated→gated re-flip still
            // requires a full debounce window.
            if was_bypassed {
                return;
            }
            let prev = self
                .pricing_gate_violation_streak
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            let streak = prev.saturating_add(1);
            if streak < PRICING_GATE_DEBOUNCE_POLLS {
                log::info!(
                    "pricing_gate: gated reading {streak}/{PRICING_GATE_DEBOUNCE_POLLS} — debouncing before bypass flip"
                );
                return;
            }
            // Transition: ungated → gated. Flip bypass FIRST so the Rust
            // intercept passes new requests straight through to Anthropic
            // while we're tearing Python down — otherwise there's a window
            // where 6767 → 6768 connect fails and Claude Code sees 502.
            self.proxy_bypass
                .store(true, std::sync::atomic::Ordering::Release);
            self.stop_headroom();
        } else {
            // Any ungated reading clears the violation streak so a later
            // gated reading starts the debounce window over.
            self.pricing_gate_violation_streak
                .store(0, std::sync::atomic::Ordering::Release);
            if was_bypassed {
                // Transition: gated → ungated (e.g., user just upgraded or
                // weekly usage rolled over). Clear bypass and bring Python
                // back online. No client_setups restore needed — gating
                // never tore them down.
                self.proxy_bypass
                    .store(false, std::sync::atomic::Ordering::Release);
                if let Err(err) = self.ensure_headroom_running() {
                    log::warn!(
                        "apply_pricing_gate_status: ensure_headroom_running failed: {err:#}"
                    );
                }
            }
        }
    }

    pub fn codex_plan_tier(&self) -> crate::models::CodexPlanTier {
        (*self.codex_plan_tier.lock()).unwrap_or(crate::models::CodexPlanTier::Unknown)
    }

    /// TTL-cached Codex identity profile, the Codex analog of
    /// `cached_claude_profile`. Reads `~/.codex/auth.json` at most once per TTL.
    /// `None` when nothing is known yet (no auth.json and no live capture).
    pub fn cached_codex_profile(&self) -> Option<CodexAccountProfile> {
        const TTL: Duration = Duration::from_secs(300);
        {
            let cache = self.cached_codex_profile.lock();
            if let Some((profile, at)) = &*cache {
                if at.elapsed() < TTL {
                    return profile.clone();
                }
            }
        }
        let profile = pricing::detect_codex_profile(self);
        *self.cached_codex_profile.lock() = Some((profile.clone(), Instant::now()));
        profile
    }

    /// Codex-only parallel to `apply_pricing_gate_status`. Flips `codex_bypass`
    /// from the Codex gate's `optimization_allowed`, debounced the same way.
    /// Unlike the Claude gate this NEVER stops the Python backend — enforcement
    /// is per-request in the intercept (OpenAI-path traffic forwards direct),
    /// so a Codex overage can't pause Claude optimization for a mixed user.
    pub fn apply_codex_pricing_gate_status(&self, codex: Option<&crate::models::CodexUsage>) {
        let was_bypassed = self.codex_bypass.load(std::sync::atomic::Ordering::Acquire);
        // No Codex usage signal yet → leave the current state untouched rather
        // than clearing a gate that a transient empty poll didn't disprove.
        let Some(codex) = codex else {
            return;
        };
        let should_bypass = !codex.optimization_allowed;

        if should_bypass {
            if was_bypassed {
                return;
            }
            let prev = self
                .codex_gate_violation_streak
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            let streak = prev.saturating_add(1);
            if streak < PRICING_GATE_DEBOUNCE_POLLS {
                log::info!(
                    "codex_gate: gated reading {streak}/{PRICING_GATE_DEBOUNCE_POLLS} — debouncing before bypass flip"
                );
                return;
            }
            self.codex_bypass
                .store(true, std::sync::atomic::Ordering::Release);
        } else {
            self.codex_gate_violation_streak
                .store(0, std::sync::atomic::Ordering::Release);
            if was_bypassed {
                self.codex_bypass
                    .store(false, std::sync::atomic::Ordering::Release);
            }
        }
    }
}

/// Number of consecutive gated pricing polls required before flipping
/// `proxy_bypass` on. With the React UI's 60s focused / 600s blurred poll
/// cadence, 2 polls = 1–10 minutes minimum before a gated state takes effect.
/// Tuned to ride out single-poll spikes (Anthropic returning a stale or
/// momentary high utilization, transient network failures clearing auth
/// state) without delaying real threshold crossings meaningfully.
const PRICING_GATE_DEBOUNCE_POLLS: u32 = 2;

pub(crate) fn current_platform() -> &'static str {
    std::env::consts::OS
}

pub(crate) fn current_platform_support_tier() -> &'static str {
    match current_platform() {
        "linux" => "experimental",
        _ => "stable",
    }
}

pub(crate) fn headroom_learn_platform_message() -> Option<String> {
    match current_platform() {
        "linux" => Some(
            "Headroom Learn is disabled on Linux preview builds. Core proxy routing works, but Learn and secure API key storage are not production-ready yet."
                .into(),
        ),
        _ => None,
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        let mut process = self.headroom_process.lock();
        if let Some(mut child) = process.take() {
            let pid = child.id() as i32;
            let _ = std::process::Command::new("/bin/kill")
                .arg("-TERM")
                .arg(format!("-{pid}"))
                .status();
            let _ = child.wait();
        }
    }
}

fn user_home_dir() -> PathBuf {
    dirs::home_dir()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
}

fn launch_agent_runtime_status() -> LaunchAgentRuntimeStatus {
    const APP_BUNDLE_ID: &str = "com.tarunagarwal.mac-ai-switchboard";
    const LEGACY_LABEL: &str = "Headroom";
    let launch_agents_dir = user_home_dir().join("Library").join("LaunchAgents");
    let managed_path = launch_agents_dir.join(format!("{APP_BUNDLE_ID}.plist"));
    let legacy_path = launch_agents_dir.join("Headroom.plist");
    let (loaded, load_detail) = launch_agent_loaded_status(APP_BUNDLE_ID);
    let (legacy_loaded, legacy_load_detail) = launch_agent_loaded_status(LEGACY_LABEL);
    LaunchAgentRuntimeStatus {
        installed: managed_path.exists(),
        path: Some(managed_path.display().to_string()),
        label: APP_BUNDLE_ID.to_string(),
        loaded,
        load_detail,
        legacy_installed: legacy_path.exists(),
        legacy_path: Some(legacy_path.display().to_string()),
        legacy_label: LEGACY_LABEL.to_string(),
        legacy_loaded,
        legacy_load_detail,
    }
}

fn launch_agent_loaded_status(label: &str) -> (Option<bool>, Option<String>) {
    #[cfg(target_os = "macos")]
    {
        let uid = unsafe { libc::getuid() };
        let target = format!("gui/{uid}/{label}");
        match Command::new("launchctl").args(["print", &target]).output() {
            Ok(output) if output.status.success() => (
                Some(true),
                Some(format!("launchctl reports {target} is loaded.")),
            ),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let combined = format!("{stderr}{stdout}").to_lowercase();
                if combined.contains("could not find service")
                    || combined.contains("service is not loaded")
                    || combined.contains("no such process")
                    || output.status.code() == Some(113)
                {
                    (
                        Some(false),
                        Some(format!("launchctl does not report {target} as loaded.")),
                    )
                } else {
                    (
                        None,
                        Some(format!(
                            "launchctl could not determine {target} load state."
                        )),
                    )
                }
            }
            Err(err) => (
                None,
                Some(format!("launchctl load-state check failed: {err}")),
            ),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = label;
        (
            None,
            Some("LaunchAgent load-state checks are only available on macOS.".to_string()),
        )
    }
}

fn backend_runtime_status() -> BackendRuntimeStatus {
    let port = crate::backend_port::get();
    BackendRuntimeStatus {
        reachable: proxy_port_accepts_connection(),
        bind_address: format!("127.0.0.1:{port}"),
        port,
        default_port: crate::backend_port::DEFAULT_BACKEND_PORT,
        fallback_range_start: crate::backend_port::FALLBACK_RANGE_START,
        fallback_range_end: crate::backend_port::FALLBACK_RANGE_END,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SavingsTotalsSnapshot {
    session_requests: usize,
    session_estimated_savings_usd: f64,
    session_estimated_tokens_saved: u64,
    session_savings_pct: f64,
    lifetime_requests: usize,
    lifetime_estimated_savings_usd: f64,
    lifetime_estimated_tokens_saved: u64,
}

const FIRST_LIFETIME_TOKEN_MILESTONES: [u64; 3] = [100_000, 1_000_000, 5_000_000];
const REPEATING_LIFETIME_TOKEN_MILESTONE_STEP: u64 = 10_000_000;

const FIRST_LIFETIME_USD_MILESTONES: [u64; 3] = [10, 50, 100];
const REPEATING_LIFETIME_USD_MILESTONE_STEP: u64 = 100;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct SavingsRecord {
    /// Schema version for forward-compatibility and migration detection.
    /// v0 = legacy (USD derived from tokens/10000)
    /// v2 = day-scoped deltas
    /// v3 = session-scoped deltas matching Headroom /stats
    /// v4 = session-scoped deltas plus actual usage totals
    /// v5 = v4 plus hour-scoped bucket keys
    /// v6 = v5 plus spend metrics sourced from /stats actual-input fields only
    /// v7 = v6 plus spend backfills distributed across session history
    schema_version: u8,
    id: String,
    observed_at: chrono::DateTime<Utc>,
    day_key: String,
    hour_key: String,
    session_requests: usize,
    session_estimated_savings_usd: f64,
    session_estimated_tokens_saved: u64,
    session_actual_cost_usd: f64,
    session_total_tokens_sent: u64,
    delta_requests: usize,
    delta_estimated_savings_usd: f64,
    delta_estimated_tokens_saved: u64,
    delta_actual_cost_usd: f64,
    delta_total_tokens_sent: u64,
    source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavingsObservation {
    observed_at: chrono::DateTime<Utc>,
    last_activity_at: Option<chrono::DateTime<Utc>>,
    session_requests: usize,
    session_estimated_savings_usd: f64,
    session_estimated_tokens_saved: u64,
    session_actual_cost_usd: f64,
    session_total_tokens_sent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RtkSavingsObservation {
    observed_at: chrono::DateTime<Utc>,
    total_commands: u64,
    total_input: u64,
    total_output: u64,
    total_saved: u64,
    total_time_ms: u64,
}

fn build_repo_intelligence_attribution_event(
    summary: &crate::models::RepoIntelligenceSummary,
) -> Option<SavingsAttributionEvent> {
    let full_scan_tokens = summary.estimated_full_scan_tokens;
    let best_pack = summary
        .packs
        .iter()
        .filter(|pack| pack.estimated_tokens > 0)
        .min_by(|left, right| {
            left.estimated_tokens
                .cmp(&right.estimated_tokens)
                .then_with(|| {
                    right
                        .savings_vs_full_scan_pct
                        .total_cmp(&left.savings_vs_full_scan_pct)
                })
                .then_with(|| left.title.cmp(&right.title))
        })?;
    let delta_tokens = full_scan_tokens.saturating_sub(best_pack.estimated_tokens);
    if delta_tokens == 0 {
        return None;
    }

    Some(SavingsAttributionEvent {
        schema_version: 1,
        id: Uuid::new_v4().to_string(),
        observed_at: Utc::now(),
        scope: SavingsAttributionScope::Session,
        source: SavingsAttributionSource::RepoIntelligence,
        confidence: SavingsAttributionConfidence::Estimated,
        delta_tokens_saved: delta_tokens,
        delta_usd: 0.0,
        total_tokens_sent: 0,
        request_delta: 1,
        evidence: vec![
            format!(
                "Estimated from Repo Intelligence best-pack delta: full scan {full_scan_tokens} tokens vs '{}' pack {} tokens.",
                best_pack.title, best_pack.estimated_tokens
            ),
            "Repo Intelligence savings estimate is local context avoided, not provider-spend dollars."
                .to_string(),
        ],
    })
}

fn build_addon_attribution_event(
    addon_id: &str,
    caveman_level: Option<&str>,
    changed_files: Option<&[String]>,
    backup_files: Option<&[String]>,
    ponytail_hosts: Option<&[String]>,
) -> Option<SavingsAttributionEvent> {
    let (source, label, baseline, optimized, confidence, evidence_subject, runtime_event_count) =
        match addon_id {
            "markitdown" => {
                let changed_files = changed_files?;
                if changed_files.is_empty() {
                    return None;
                }
                (
                SavingsAttributionSource::Markitdown,
                "MarkItDown",
                MARKITDOWN_TEMPLATE_BASELINE_TOKENS,
                MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS,
                SavingsAttributionConfidence::Estimated,
                "MarkItDown managed hook or instruction guidance was written into connected client files after the console script was smoke-tested",
                changed_files.len(),
            )
            }
            "ponytail" => {
                let hosts = ponytail_hosts?;
                if hosts.is_empty() {
                    return None;
                }
                (
                    SavingsAttributionSource::Ponytail,
                    "Ponytail",
                    PONYTAIL_TEMPLATE_BASELINE_TOKENS,
                    PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS,
                    SavingsAttributionConfidence::Estimated,
                    "Ponytail plugin registration was verified in connected agent hosts",
                    hosts.len(),
                )
            }
            "caveman" => {
                let changed_files = changed_files?;
                if changed_files.is_empty() {
                    return None;
                }
                let compact = caveman_level
                    .map(|level| level == crate::tool_manager::CAVEMAN_LEVEL_COMPACT_CHINESE)
                    .unwrap_or(false);
                (
                    if compact {
                        SavingsAttributionSource::CompactChinese
                    } else {
                        SavingsAttributionSource::Caveman
                    },
                    if compact {
                        "Compact Chinese"
                    } else {
                        "Caveman"
                    },
                    CAVEMAN_TEMPLATE_BASELINE_TOKENS,
                    CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS,
                    SavingsAttributionConfidence::Estimated,
                    if compact {
                        "Compact Chinese managed guidance was written into connected client instruction files"
                    } else {
                        "Caveman managed guidance was written into connected client instruction files"
                    },
                    changed_files.len(),
                )
            }
            _ => return None,
        };
    let delta_tokens = baseline.saturating_sub(optimized);
    if delta_tokens == 0 {
        return None;
    }
    let confidence_label = match confidence {
        SavingsAttributionConfidence::Measured => "Measured",
        SavingsAttributionConfidence::Estimated => "Estimated",
        SavingsAttributionConfidence::Inferred => "Inferred",
    };

    let mut evidence = vec![
        format!(
            "{confidence_label} from {label} template delta: baseline {baseline} tokens vs optimized {optimized} tokens."
        ),
        format!("{evidence_subject}; local workflow estimate, not provider-spend dollars."),
    ];
    if addon_id == "caveman" {
        let changed = changed_files.unwrap_or(&[]);
        evidence.push(format!(
            "Managed guidance changed {} client instruction file{}: {}.",
            changed.len(),
            if changed.len() == 1 { "" } else { "s" },
            changed.join(", ")
        ));
        if let Some(backups) = backup_files.filter(|backups| !backups.is_empty()) {
            evidence.push(format!(
                "Backups created for reversible guidance writes: {}.",
                backups.join(", ")
            ));
        }
    }
    if addon_id == "markitdown" {
        let changed = changed_files.unwrap_or(&[]);
        evidence.push(format!(
            "Managed MarkItDown integration changed {} client artifact{}: {}.",
            changed.len(),
            if changed.len() == 1 { "" } else { "s" },
            changed.join(", ")
        ));
        if let Some(backups) = backup_files.filter(|backups| !backups.is_empty()) {
            evidence.push(format!(
                "Backups created for reversible MarkItDown integration writes: {}.",
                backups.join(", ")
            ));
        }
    }
    if addon_id == "ponytail" {
        let hosts = ponytail_hosts.unwrap_or(&[]);
        evidence.push(format!(
            "Ponytail plugin registered with {} agent host{}: {}.",
            hosts.len(),
            if hosts.len() == 1 { "" } else { "s" },
            hosts.join(", ")
        ));
    }

    Some(SavingsAttributionEvent {
        schema_version: 1,
        id: Uuid::new_v4().to_string(),
        observed_at: Utc::now(),
        scope: SavingsAttributionScope::Session,
        source,
        confidence,
        delta_tokens_saved: delta_tokens,
        delta_usd: 0.0,
        total_tokens_sent: 0,
        request_delta: runtime_event_count.max(1),
        evidence,
    })
}

impl SavingsObservation {
    fn last_activity_at(&self) -> chrono::DateTime<Utc> {
        self.last_activity_at.unwrap_or(self.observed_at)
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
struct DailySavingsBucket {
    estimated_savings_usd: f64,
    estimated_tokens_saved: u64,
    actual_cost_usd: f64,
    total_tokens_sent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedSavingsState {
    schema_version: u8,
    session_requests: usize,
    session_estimated_savings_usd: f64,
    session_estimated_tokens_saved: u64,
    session_savings_pct: f64,
    lifetime_requests: usize,
    lifetime_estimated_savings_usd: f64,
    lifetime_estimated_tokens_saved: u64,
    last_observation: Option<SavingsObservation>,
    last_rtk_observation: Option<RtkSavingsObservation>,
    display_session_baseline: Option<SavingsObservation>,
    session_savings_history: Vec<HeadroomSavingsHistoryPoint>,
    session_hourly_buckets: BTreeMap<String, DailySavingsBucket>,
    daily_savings: BTreeMap<String, DailySavingsBucket>,
    hourly_savings: BTreeMap<String, DailySavingsBucket>,
}

fn maybe_append_measured_headroom_attribution(
    tracker: &mut SavingsTracker,
    stats: &HeadroomDashboardStats,
) -> Result<()> {
    let optimized_tokens = stats.session_total_tokens_sent.unwrap_or(0);
    let saved_tokens = stats.session_estimated_tokens_saved.unwrap_or(0);
    let request_delta = stats.session_requests.unwrap_or(0);
    if optimized_tokens == 0 || saved_tokens == 0 || request_delta == 0 {
        return Ok(());
    }

    let baseline_tokens = optimized_tokens.saturating_add(saved_tokens);
    let event = SavingsAttributionEvent {
        schema_version: 1,
        id: Uuid::new_v4().to_string(),
        observed_at: Utc::now(),
        scope: SavingsAttributionScope::Session,
        source: SavingsAttributionSource::HeadroomEngine,
        confidence: SavingsAttributionConfidence::Measured,
        delta_tokens_saved: saved_tokens,
        delta_usd: 0.0,
        total_tokens_sent: optimized_tokens,
        request_delta,
        evidence: vec![format!(
            "Headroom /stats measured {saved_tokens} saved tokens from {baseline_tokens} before to {optimized_tokens} after using session_estimated_tokens_saved and session_total_tokens_sent."
        )],
    };

    tracker.append_attribution_event(&event)
}

struct SavingsTracker {
    records_path: std::path::PathBuf,
    attribution_events_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
    session_requests: usize,
    session_estimated_savings_usd: f64,
    session_estimated_tokens_saved: u64,
    session_savings_pct: f64,
    lifetime_requests: usize,
    lifetime_estimated_savings_usd: f64,
    lifetime_estimated_tokens_saved: u64,
    last_observation: Option<SavingsObservation>,
    last_rtk_observation: Option<RtkSavingsObservation>,
    display_session_baseline: Option<SavingsObservation>,
    session_savings_history: Vec<HeadroomSavingsHistoryPoint>,
    session_hourly_buckets: BTreeMap<String, DailySavingsBucket>,
    daily_savings: BTreeMap<String, DailySavingsBucket>,
    hourly_savings: BTreeMap<String, DailySavingsBucket>,
    pending_lifetime_token_milestones: Vec<u64>,
    pending_lifetime_usd_milestones: Vec<u64>,
    // Write throttle — only flush to disk at most once per minute
    last_written_at: Option<std::time::Instant>,
}

impl SavingsTracker {
    fn load_or_create(base_dir: &Path) -> Result<Self> {
        let records_path = telemetry_file(base_dir, "savings-records.jsonl");
        let attribution_events_path = telemetry_file(base_dir, "savings-attribution-events.jsonl");
        let state_path = config_file(base_dir, "savings-state.json");
        if !records_path.exists() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&records_path)
                .with_context(|| format!("creating {}", records_path.display()))?;
        }
        if !attribution_events_path.exists() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&attribution_events_path)
                .with_context(|| format!("creating {}", attribution_events_path.display()))?;
        }

        let persisted_state = load_persisted_savings_state(&state_path).ok().flatten();

        let mut tracker = Self {
            records_path,
            attribution_events_path,
            state_path,
            session_requests: 0,
            session_estimated_savings_usd: 0.0,
            session_estimated_tokens_saved: 0,
            session_savings_pct: 0.0,
            lifetime_requests: persisted_state
                .as_ref()
                .map_or(0, |state| state.lifetime_requests),
            lifetime_estimated_savings_usd: persisted_state
                .as_ref()
                .map_or(0.0, |state| state.lifetime_estimated_savings_usd),
            lifetime_estimated_tokens_saved: persisted_state
                .as_ref()
                .map_or(0, |state| state.lifetime_estimated_tokens_saved),
            last_observation: persisted_state
                .as_ref()
                .and_then(|state| state.last_observation.clone()),
            last_rtk_observation: persisted_state
                .as_ref()
                .and_then(|state| state.last_rtk_observation.clone()),
            display_session_baseline: persisted_state
                .as_ref()
                .and_then(|state| state.display_session_baseline.clone()),
            session_savings_history: persisted_state
                .as_ref()
                .map_or_else(Vec::new, |state| state.session_savings_history.clone()),
            session_hourly_buckets: persisted_state
                .as_ref()
                .map_or_else(BTreeMap::new, |state| state.session_hourly_buckets.clone()),
            daily_savings: persisted_state
                .as_ref()
                .map_or_else(BTreeMap::new, |state| state.daily_savings.clone()),
            hourly_savings: persisted_state
                .as_ref()
                .map_or_else(BTreeMap::new, |state| state.hourly_savings.clone()),
            pending_lifetime_token_milestones: Vec::new(),
            pending_lifetime_usd_milestones: Vec::new(),
            last_written_at: None,
        };
        tracker.persist_state()?;
        Ok(tracker)
    }

    fn snapshot(&self) -> SavingsTotalsSnapshot {
        let baseline = self.display_session_baseline.as_ref();
        let session_requests = baseline.map_or(self.session_requests, |baseline| {
            self.session_requests
                .saturating_sub(baseline.session_requests)
        });
        let session_estimated_savings_usd =
            baseline.map_or(self.session_estimated_savings_usd, |baseline| {
                (self.session_estimated_savings_usd - baseline.session_estimated_savings_usd)
                    .max(0.0)
            });
        let session_estimated_tokens_saved =
            baseline.map_or(self.session_estimated_tokens_saved, |baseline| {
                self.session_estimated_tokens_saved
                    .saturating_sub(baseline.session_estimated_tokens_saved)
            });
        let session_savings_pct = if let Some(baseline) = baseline {
            let total_tokens_sent = self
                .last_observation
                .as_ref()
                .map(|observation| observation.session_total_tokens_sent)
                .unwrap_or(0)
                .saturating_sub(baseline.session_total_tokens_sent);
            let total_before = session_estimated_tokens_saved.saturating_add(total_tokens_sent);
            if total_before > 0 {
                session_estimated_tokens_saved as f64 / total_before as f64 * 100.0
            } else {
                0.0
            }
        } else {
            self.session_savings_pct
        };

        SavingsTotalsSnapshot {
            session_requests,
            session_estimated_savings_usd,
            session_estimated_tokens_saved,
            session_savings_pct,
            lifetime_requests: self.lifetime_requests,
            lifetime_estimated_savings_usd: self.lifetime_estimated_savings_usd,
            lifetime_estimated_tokens_saved: self.lifetime_estimated_tokens_saved,
        }
    }

    fn daily_savings(&self) -> Vec<DailySavingsPoint> {
        self.daily_savings
            .iter()
            .map(|(date, bucket)| DailySavingsPoint {
                date: date.clone(),
                estimated_savings_usd: bucket.estimated_savings_usd,
                estimated_tokens_saved: bucket.estimated_tokens_saved,
                actual_cost_usd: bucket.actual_cost_usd,
                total_tokens_sent: bucket.total_tokens_sent,
            })
            .collect()
    }

    fn hourly_savings(&self) -> Vec<HourlySavingsPoint> {
        self.hourly_savings
            .iter()
            .map(|(hour, bucket)| HourlySavingsPoint {
                hour: hour.clone(),
                estimated_savings_usd: bucket.estimated_savings_usd,
                estimated_tokens_saved: bucket.estimated_tokens_saved,
                actual_cost_usd: bucket.actual_cost_usd,
                total_tokens_sent: bucket.total_tokens_sent,
                // The local pre-cutoff tracker has no provider dimension.
                by_provider: Vec::new(),
            })
            .collect()
    }

    fn attribution_events(&self) -> Vec<SavingsAttributionEvent> {
        let Ok(text) = std::fs::read_to_string(&self.attribution_events_path) else {
            return Vec::new();
        };

        text.lines()
            .filter_map(|line| serde_json::from_str::<SavingsAttributionEvent>(line).ok())
            .collect()
    }

    fn observe_rtk_gain_summary(&mut self, stats: &RtkGainSummary) {
        let previous = self.last_rtk_observation.clone();
        let reset_detected = previous.as_ref().is_some_and(|prev| {
            stats.total_saved < prev.total_saved || stats.total_commands < prev.total_commands
        });
        let (delta_tokens, delta_commands, delta_input, delta_output, delta_time_ms) =
            match previous.as_ref() {
                Some(prev) if !reset_detected => (
                    stats.total_saved.saturating_sub(prev.total_saved),
                    stats.total_commands.saturating_sub(prev.total_commands),
                    stats.total_input.saturating_sub(prev.total_input),
                    stats.total_output.saturating_sub(prev.total_output),
                    stats.total_time_ms.saturating_sub(prev.total_time_ms),
                ),
                Some(_) => (
                    stats.total_saved,
                    stats.total_commands,
                    stats.total_input,
                    stats.total_output,
                    stats.total_time_ms,
                ),
                None => (0, 0, 0, 0, 0),
            };

        if delta_tokens > 0 || delta_commands > 0 {
            let mut evidence = vec![
                "Measured from positive RTK gain counter deltas.".to_string(),
                "RTK savings are local command-output tokens and are not model-spend dollars."
                    .to_string(),
            ];
            if delta_input > 0 || delta_output > 0 {
                evidence.push(format!(
                    "RTK delta included {delta_input} input tokens, {delta_output} output tokens, and {delta_tokens} saved tokens."
                ));
            }
            if stats.avg_savings_pct > 0.0 {
                evidence.push(format!(
                    "RTK reported {:.1}% average savings across all recorded command output.",
                    stats.avg_savings_pct
                ));
            }
            if delta_time_ms > 0 {
                evidence.push(format!(
                    "RTK delta included {delta_time_ms}ms processing time."
                ));
            }

            let event = SavingsAttributionEvent {
                schema_version: 1,
                id: Uuid::new_v4().to_string(),
                observed_at: Utc::now(),
                scope: SavingsAttributionScope::Session,
                source: SavingsAttributionSource::Rtk,
                confidence: SavingsAttributionConfidence::Measured,
                delta_tokens_saved: delta_tokens,
                delta_usd: 0.0,
                total_tokens_sent: 0,
                request_delta: delta_commands as usize,
                evidence,
            };
            let _ = self.append_attribution_event(&event);
        }

        self.last_rtk_observation = Some(RtkSavingsObservation {
            observed_at: Utc::now(),
            total_commands: stats.total_commands,
            total_input: stats.total_input,
            total_output: stats.total_output,
            total_saved: stats.total_saved,
            total_time_ms: stats.total_time_ms,
        });
        let _ = self.persist_state();
    }

    /// Fold the backend's authoritative rollups into the local archive so they
    /// survive its history trimming and fill gaps from periods the app wasn't
    /// running. Only settled days in `[cutoff_date, today_key)` are written:
    /// today's live buckets are left to `observe`, and pre-cutoff days are
    /// skipped (pre-v6 schema drift). Native values overwrite the tracker's own
    /// observed values for those keys, mirroring the display-time merge.
    /// Returns true if any bucket changed (caller should persist).
    fn ingest_native_rollups(
        &mut self,
        daily: &[DailySavingsPoint],
        hourly: &[HourlySavingsPoint],
        cutoff_date: &str,
        today_key: &str,
    ) -> bool {
        let cutoff_hour = format!("{cutoff_date}T00:00");
        let mut changed = false;
        for point in daily {
            if point.date.as_str() < cutoff_date || point.date.as_str() >= today_key {
                continue;
            }
            let bucket = DailySavingsBucket {
                estimated_savings_usd: point.estimated_savings_usd,
                estimated_tokens_saved: point.estimated_tokens_saved,
                actual_cost_usd: point.actual_cost_usd,
                total_tokens_sent: point.total_tokens_sent,
            };
            if self.daily_savings.get(&point.date) != Some(&bucket) {
                self.daily_savings.insert(point.date.clone(), bucket);
                changed = true;
            }
        }
        for point in hourly {
            if point.hour.as_str() < cutoff_hour.as_str()
                || day_key_from_hour_key(&point.hour).as_str() >= today_key
            {
                continue;
            }
            let bucket = DailySavingsBucket {
                estimated_savings_usd: point.estimated_savings_usd,
                estimated_tokens_saved: point.estimated_tokens_saved,
                actual_cost_usd: point.actual_cost_usd,
                total_tokens_sent: point.total_tokens_sent,
            };
            if self.hourly_savings.get(&point.hour) != Some(&bucket) {
                self.hourly_savings.insert(point.hour.clone(), bucket);
                changed = true;
            }
        }
        changed
    }

    fn take_pending_lifetime_token_milestones(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_lifetime_token_milestones)
    }

    #[cfg(test)]
    fn take_pending_lifetime_usd_milestones(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_lifetime_usd_milestones)
    }

    fn observe(&mut self, stats: &HeadroomDashboardStats) -> Option<SavingsTotalsSnapshot> {
        let session_tokens_saved = stats.session_estimated_tokens_saved?;
        let session_savings_usd = stats.session_estimated_savings_usd.unwrap_or(0.0).max(0.0);
        let session_requests = stats.session_requests.unwrap_or(0);
        let session_total_tokens_sent = stats.session_total_tokens_sent;
        let session_actual_cost_usd = stats.session_actual_cost_usd.map(|value| value.max(0.0));
        let first_observation = self.last_observation.is_none();
        let previous = self.last_observation.clone();
        let requests_went_back = previous.as_ref().is_some_and(|prev| {
            stats.session_requests.is_some() && session_requests < prev.session_requests
        });
        let reset_detected = previous.as_ref().is_some_and(|prev| {
            session_tokens_saved < prev.session_estimated_tokens_saved
                || session_total_tokens_sent.is_some_and(|value| {
                    prev.session_total_tokens_sent > 0 && value < prev.session_total_tokens_sent
                })
                || session_actual_cost_usd.is_some_and(|value| {
                    prev.session_actual_cost_usd > 0.0
                        && value + 0.000_001 < prev.session_actual_cost_usd
                })
                || requests_went_back
        });
        let rollover_display_session = previous.as_ref().is_some_and(|prev| {
            should_rollover_display_session(prev.last_activity_at(), Utc::now())
        });

        let (
            delta_requests,
            delta_usd,
            delta_tokens,
            delta_actual_cost_usd,
            delta_total_tokens_sent,
        ) = if let Some(prev) = previous.as_ref() {
            if reset_detected {
                (
                    session_requests,
                    session_savings_usd,
                    session_tokens_saved,
                    session_actual_cost_usd.unwrap_or(0.0),
                    session_total_tokens_sent.unwrap_or(0),
                )
            } else {
                (
                    session_requests.saturating_sub(prev.session_requests),
                    (session_savings_usd - prev.session_estimated_savings_usd).max(0.0),
                    session_tokens_saved.saturating_sub(prev.session_estimated_tokens_saved),
                    session_actual_cost_usd.map_or(0.0, |value| {
                        if prev.session_actual_cost_usd > 0.0 {
                            (value - prev.session_actual_cost_usd).max(0.0)
                        } else {
                            0.0
                        }
                    }),
                    session_total_tokens_sent.map_or(0, |value| {
                        if prev.session_total_tokens_sent > 0 {
                            value.saturating_sub(prev.session_total_tokens_sent)
                        } else {
                            0
                        }
                    }),
                )
            }
        } else {
            (
                session_requests,
                session_savings_usd,
                session_tokens_saved,
                session_actual_cost_usd.unwrap_or(0.0),
                session_total_tokens_sent.unwrap_or(0),
            )
        };
        if reset_detected {
            self.session_savings_history.clear();
        }
        self.session_savings_history =
            merge_session_savings_history(&self.session_savings_history, &stats.savings_history);

        let previous_session_hourly_buckets = self.session_hourly_buckets.clone();
        let current_session_hourly_buckets =
            derive_session_hourly_buckets(stats, &self.session_savings_history);
        let current_session_hourly_buckets_map = current_session_hourly_buckets
            .iter()
            .cloned()
            .collect::<BTreeMap<_, _>>();
        let session_buckets_changed = !current_session_hourly_buckets.is_empty()
            && current_session_hourly_buckets_map != previous_session_hourly_buckets;
        let delta_hourly_buckets = if first_observation || reset_detected {
            current_session_hourly_buckets.clone()
        } else {
            diff_hourly_buckets(
                &previous_session_hourly_buckets,
                &current_session_hourly_buckets,
            )
        };

        self.session_requests = session_requests;
        self.session_estimated_savings_usd = session_savings_usd;
        self.session_estimated_tokens_saved = session_tokens_saved;
        self.session_savings_pct = stats.session_savings_pct.unwrap_or(0.0);
        if reset_detected {
            self.display_session_baseline = None;
        } else if rollover_display_session {
            self.display_session_baseline = previous.clone();
        }

        let changed = delta_requests > 0
            || delta_tokens > 0
            || delta_total_tokens_sent > 0
            || delta_usd > 0.000_001
            || delta_actual_cost_usd > 0.000_001
            || session_buckets_changed;
        if delta_requests > 0 || delta_tokens > 0 || delta_usd > 0.000_001 {
            let event = SavingsAttributionEvent {
                schema_version: 1,
                id: Uuid::new_v4().to_string(),
                observed_at: Utc::now(),
                scope: SavingsAttributionScope::Session,
                source: SavingsAttributionSource::HeadroomEngine,
                confidence: SavingsAttributionConfidence::Measured,
                delta_tokens_saved: delta_tokens,
                delta_usd,
                total_tokens_sent: delta_total_tokens_sent,
                request_delta: delta_requests,
                evidence: vec![
                    "Measured from positive Headroom /stats session deltas.".to_string(),
                    "Source excludes RTK, Repo Intelligence, Ponytail, Caveman, Compact Chinese, and MarkItDown until those emit source-specific counters.".to_string(),
                ],
            };
            let _ = self.append_attribution_event(&event);
        }
        let previous_lifetime_tokens_saved = self.lifetime_estimated_tokens_saved;
        let previous_lifetime_estimated_savings_usd = self.lifetime_estimated_savings_usd;
        if delta_requests > 0 || delta_tokens > 0 || delta_usd > 0.0 {
            self.lifetime_requests = self.lifetime_requests.saturating_add(delta_requests);
            self.lifetime_estimated_savings_usd += delta_usd;
            self.lifetime_estimated_tokens_saved = self
                .lifetime_estimated_tokens_saved
                .saturating_add(delta_tokens);
        }
        self.pending_lifetime_token_milestones
            .extend(lifetime_token_milestones_crossed(
                previous_lifetime_tokens_saved,
                self.lifetime_estimated_tokens_saved,
            ));
        self.pending_lifetime_usd_milestones
            .extend(lifetime_usd_milestones_crossed(
                previous_lifetime_estimated_savings_usd,
                self.lifetime_estimated_savings_usd,
            ));

        let baseline_hourly_buckets = if (first_observation || reset_detected)
            && (session_requests > 0
                || session_tokens_saved > 0
                || session_savings_usd > 0.0
                || session_total_tokens_sent.unwrap_or(0) > 0
                || session_actual_cost_usd.unwrap_or(0.0) > 0.0)
        {
            self.ingest_hourly_buckets(&current_session_hourly_buckets);
            current_session_hourly_buckets.clone()
        } else {
            Vec::new()
        };
        if !first_observation && !reset_detected && session_buckets_changed {
            self.replace_session_hourly_buckets(
                &previous_session_hourly_buckets,
                &current_session_hourly_buckets,
            );
        }
        if first_observation || reset_detected {
            self.session_hourly_buckets = current_session_hourly_buckets_map;
        } else if session_buckets_changed {
            self.session_hourly_buckets = current_session_hourly_buckets_map;
        }
        if reset_detected && current_session_hourly_buckets.is_empty() {
            self.session_hourly_buckets.clear();
        }

        self.last_observation = Some(SavingsObservation {
            session_requests,
            session_estimated_savings_usd: session_savings_usd,
            session_estimated_tokens_saved: session_tokens_saved,
            observed_at: Utc::now(),
            last_activity_at: Some(if changed {
                Utc::now()
            } else {
                previous
                    .as_ref()
                    .map(|prev| prev.last_activity_at())
                    .unwrap_or_else(Utc::now)
            }),
            session_actual_cost_usd: session_actual_cost_usd.unwrap_or(
                previous
                    .as_ref()
                    .map_or(0.0, |prev| prev.session_actual_cost_usd),
            ),
            session_total_tokens_sent: session_total_tokens_sent.unwrap_or(
                previous
                    .as_ref()
                    .map_or(0, |prev| prev.session_total_tokens_sent),
            ),
        });

        let now = std::time::Instant::now();
        let has_any_value = session_requests > 0
            || session_tokens_saved > 0
            || session_savings_usd > 0.0
            || session_total_tokens_sent.unwrap_or(0) > 0
            || session_actual_cost_usd.unwrap_or(0.0) > 0.0;
        let should_write = has_any_value
            && (first_observation
                || reset_detected
                || (changed
                    && self
                        .last_written_at
                        .map_or(true, |t| now.duration_since(t).as_secs() >= 60)));
        if should_write {
            self.last_written_at = Some(now);
            if first_observation || reset_detected {
                for record in build_hourly_backfill_records(
                    &baseline_hourly_buckets,
                    session_requests,
                    session_savings_usd,
                    session_tokens_saved,
                    session_actual_cost_usd.unwrap_or(0.0),
                    session_total_tokens_sent.unwrap_or(0),
                ) {
                    let _ = self.append_record(&record);
                }
            } else {
                if baseline_hourly_buckets.is_empty()
                    && delta_requests == 0
                    && delta_hourly_buckets.is_empty()
                {
                } else if baseline_hourly_buckets.is_empty() {
                    let record = SavingsRecord {
                        schema_version: 7,
                        id: Uuid::new_v4().to_string(),
                        observed_at: Utc::now(),
                        day_key: local_day_key(Local::now()),
                        hour_key: local_hour_key(Local::now()),
                        session_requests,
                        session_estimated_savings_usd: session_savings_usd,
                        session_estimated_tokens_saved: session_tokens_saved,
                        session_actual_cost_usd: session_actual_cost_usd.unwrap_or(0.0),
                        session_total_tokens_sent: session_total_tokens_sent.unwrap_or(0),
                        delta_requests,
                        delta_estimated_savings_usd: 0.0,
                        delta_estimated_tokens_saved: 0,
                        delta_actual_cost_usd: 0.0,
                        delta_total_tokens_sent: 0,
                        source: "headroom_dashboard".into(),
                    };
                    let _ = self.append_record(&record);
                } else {
                    for record in build_hourly_delta_records(
                        &baseline_hourly_buckets,
                        session_requests,
                        session_savings_usd,
                        session_tokens_saved,
                        session_actual_cost_usd.unwrap_or(0.0),
                        session_total_tokens_sent.unwrap_or(0),
                        delta_requests,
                    ) {
                        let _ = self.append_record(&record);
                    }
                }
            }
        }
        let _ = self.persist_state();

        Some(self.snapshot())
    }

    fn ingest_hourly_buckets(&mut self, buckets: &[(String, DailySavingsBucket)]) {
        for (hour_key, bucket) in buckets {
            self.add_hourly_delta(
                hour_key,
                bucket.estimated_savings_usd,
                bucket.estimated_tokens_saved,
                bucket.actual_cost_usd,
                bucket.total_tokens_sent,
            );
            self.add_daily_delta(
                &day_key_from_hour_key(hour_key),
                bucket.estimated_savings_usd,
                bucket.estimated_tokens_saved,
                bucket.actual_cost_usd,
                bucket.total_tokens_sent,
            );
        }
    }

    fn replace_session_hourly_buckets(
        &mut self,
        previous: &BTreeMap<String, DailySavingsBucket>,
        current: &[(String, DailySavingsBucket)],
    ) {
        for (hour_key, bucket) in previous {
            self.subtract_hourly_delta(
                hour_key,
                bucket.estimated_savings_usd,
                bucket.estimated_tokens_saved,
                bucket.actual_cost_usd,
                bucket.total_tokens_sent,
            );
            self.subtract_daily_delta(
                &day_key_from_hour_key(hour_key),
                bucket.estimated_savings_usd,
                bucket.estimated_tokens_saved,
                bucket.actual_cost_usd,
                bucket.total_tokens_sent,
            );
        }
        self.ingest_hourly_buckets(current);
    }

    fn add_daily_delta(
        &mut self,
        day_key: &str,
        usd: f64,
        tokens: u64,
        actual_cost_usd: f64,
        total_tokens_sent: u64,
    ) {
        if usd <= 0.0 && tokens == 0 && actual_cost_usd <= 0.0 && total_tokens_sent == 0 {
            return;
        }
        let entry = self.daily_savings.entry(day_key.to_string()).or_default();
        entry.estimated_savings_usd += usd.max(0.0);
        entry.estimated_tokens_saved = entry.estimated_tokens_saved.saturating_add(tokens);
        entry.actual_cost_usd += actual_cost_usd.max(0.0);
        entry.total_tokens_sent = entry.total_tokens_sent.saturating_add(total_tokens_sent);
    }

    fn subtract_daily_delta(
        &mut self,
        day_key: &str,
        usd: f64,
        tokens: u64,
        actual_cost_usd: f64,
        total_tokens_sent: u64,
    ) {
        let mut should_remove = false;
        if let Some(entry) = self.daily_savings.get_mut(day_key) {
            entry.estimated_savings_usd = (entry.estimated_savings_usd - usd.max(0.0)).max(0.0);
            entry.estimated_tokens_saved = entry.estimated_tokens_saved.saturating_sub(tokens);
            entry.actual_cost_usd = (entry.actual_cost_usd - actual_cost_usd.max(0.0)).max(0.0);
            entry.total_tokens_sent = entry.total_tokens_sent.saturating_sub(total_tokens_sent);
            should_remove = entry.estimated_savings_usd <= 0.0
                && entry.estimated_tokens_saved == 0
                && entry.actual_cost_usd <= 0.0
                && entry.total_tokens_sent == 0;
        }
        if should_remove {
            self.daily_savings.remove(day_key);
        }
    }

    fn add_hourly_delta(
        &mut self,
        hour_key: &str,
        usd: f64,
        tokens: u64,
        actual_cost_usd: f64,
        total_tokens_sent: u64,
    ) {
        if usd <= 0.0 && tokens == 0 && actual_cost_usd <= 0.0 && total_tokens_sent == 0 {
            return;
        }
        let entry = self.hourly_savings.entry(hour_key.to_string()).or_default();
        entry.estimated_savings_usd += usd.max(0.0);
        entry.estimated_tokens_saved = entry.estimated_tokens_saved.saturating_add(tokens);
        entry.actual_cost_usd += actual_cost_usd.max(0.0);
        entry.total_tokens_sent = entry.total_tokens_sent.saturating_add(total_tokens_sent);
    }

    fn subtract_hourly_delta(
        &mut self,
        hour_key: &str,
        usd: f64,
        tokens: u64,
        actual_cost_usd: f64,
        total_tokens_sent: u64,
    ) {
        let mut should_remove = false;
        if let Some(entry) = self.hourly_savings.get_mut(hour_key) {
            entry.estimated_savings_usd = (entry.estimated_savings_usd - usd.max(0.0)).max(0.0);
            entry.estimated_tokens_saved = entry.estimated_tokens_saved.saturating_sub(tokens);
            entry.actual_cost_usd = (entry.actual_cost_usd - actual_cost_usd.max(0.0)).max(0.0);
            entry.total_tokens_sent = entry.total_tokens_sent.saturating_sub(total_tokens_sent);
            should_remove = entry.estimated_savings_usd <= 0.0
                && entry.estimated_tokens_saved == 0
                && entry.actual_cost_usd <= 0.0
                && entry.total_tokens_sent == 0;
        }
        if should_remove {
            self.hourly_savings.remove(hour_key);
        }
    }

    fn append_record(&self, record: &SavingsRecord) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.records_path)
            .with_context(|| format!("opening {}", self.records_path.display()))?;
        let serialized = serde_json::to_string(record).context("serializing savings record")?;
        use std::io::Write;
        file.write_all(serialized.as_bytes())
            .with_context(|| format!("writing {}", self.records_path.display()))?;
        file.write_all(b"\n")
            .with_context(|| format!("writing {}", self.records_path.display()))?;
        Ok(())
    }

    fn append_attribution_event(&self, event: &SavingsAttributionEvent) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.attribution_events_path)
            .with_context(|| format!("opening {}", self.attribution_events_path.display()))?;
        let serialized =
            serde_json::to_string(event).context("serializing savings attribution event")?;
        use std::io::Write;
        file.write_all(serialized.as_bytes())
            .with_context(|| format!("writing {}", self.attribution_events_path.display()))?;
        file.write_all(b"\n")
            .with_context(|| format!("writing {}", self.attribution_events_path.display()))?;
        Ok(())
    }

    fn persisted_state(&self) -> PersistedSavingsState {
        PersistedSavingsState {
            schema_version: 3,
            session_requests: self.session_requests,
            session_estimated_savings_usd: self.session_estimated_savings_usd,
            session_estimated_tokens_saved: self.session_estimated_tokens_saved,
            session_savings_pct: self.session_savings_pct,
            lifetime_requests: self.lifetime_requests,
            lifetime_estimated_savings_usd: self.lifetime_estimated_savings_usd,
            lifetime_estimated_tokens_saved: self.lifetime_estimated_tokens_saved,
            last_observation: self.last_observation.clone(),
            last_rtk_observation: self.last_rtk_observation.clone(),
            display_session_baseline: self.display_session_baseline.clone(),
            session_savings_history: self.session_savings_history.clone(),
            session_hourly_buckets: self.session_hourly_buckets.clone(),
            daily_savings: self.daily_savings.clone(),
            hourly_savings: self.hourly_savings.clone(),
        }
    }

    fn persist_state(&mut self) -> Result<()> {
        let serialized = serde_json::to_vec_pretty(&self.persisted_state())
            .context("serializing savings state")?;
        std::fs::write(&self.state_path, serialized)
            .with_context(|| format!("writing {}", self.state_path.display()))?;
        Ok(())
    }
}

/// The Monday of the week that contains `d`, or `d` itself if it already is
/// a Monday. Used by the weekly recap: the recap for `d` covers the 7 days
/// ending the day before this Monday.
fn aggregate_savings_attribution_counters(
    events: &[SavingsAttributionEvent],
) -> Vec<SavingsAttributionCounter> {
    let mut counters: Vec<SavingsAttributionCounter> = Vec::new();
    for event in events {
        let index = counters
            .iter()
            .position(|counter| counter.source == event.source && counter.scope == event.scope);
        let entry = if let Some(index) = index {
            &mut counters[index]
        } else {
            counters.push(SavingsAttributionCounter {
                source: event.source.clone(),
                scope: event.scope.clone(),
                event_count: 0,
                runtime_event_count: 0,
                measured_event_count: 0,
                estimated_event_count: 0,
                inferred_event_count: 0,
                delta_tokens_saved: 0,
                total_tokens_sent: 0,
                last_seen_at: None,
            });
            counters.last_mut().expect("counter just pushed")
        };
        entry.event_count = entry.event_count.saturating_add(1);
        entry.runtime_event_count = entry
            .runtime_event_count
            .saturating_add(event.request_delta as u64);
        match event.confidence {
            SavingsAttributionConfidence::Measured => {
                entry.measured_event_count = entry.measured_event_count.saturating_add(1);
            }
            SavingsAttributionConfidence::Estimated => {
                entry.estimated_event_count = entry.estimated_event_count.saturating_add(1);
            }
            SavingsAttributionConfidence::Inferred => {
                entry.inferred_event_count = entry.inferred_event_count.saturating_add(1);
            }
        }
        entry.delta_tokens_saved = entry
            .delta_tokens_saved
            .saturating_add(event.delta_tokens_saved);
        entry.total_tokens_sent = entry
            .total_tokens_sent
            .saturating_add(event.total_tokens_sent);
        if entry
            .last_seen_at
            .map(|last| event.observed_at > last)
            .unwrap_or(true)
        {
            entry.last_seen_at = Some(event.observed_at);
        }
    }
    counters
}

fn most_recent_monday(d: chrono::NaiveDate) -> chrono::NaiveDate {
    let days_past = d.weekday().num_days_from_monday() as u64;
    d.checked_sub_days(chrono::Days::new(days_past))
        .unwrap_or(d)
}

fn aggregate_weekly_totals(
    daily_savings: &BTreeMap<String, DailySavingsBucket>,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> WeeklyTotals {
    let start_key = start.format("%Y-%m-%d").to_string();
    let end_key = end.format("%Y-%m-%d").to_string();
    let mut total_tokens_saved: u64 = 0;
    let mut total_savings_usd: f64 = 0.0;
    let mut active_days: u32 = 0;
    for (day_key, bucket) in daily_savings.range(start_key..=end_key) {
        let has_activity = bucket.estimated_tokens_saved > 0 || bucket.estimated_savings_usd > 0.0;
        if has_activity {
            active_days += 1;
        }
        total_tokens_saved = total_tokens_saved.saturating_add(bucket.estimated_tokens_saved);
        total_savings_usd += bucket.estimated_savings_usd;
        let _ = day_key;
    }
    WeeklyTotals {
        total_tokens_saved,
        total_savings_usd,
        active_days,
    }
}

fn lifetime_usd_milestones_crossed(previous_usd: f64, current_usd: f64) -> Vec<u64> {
    let previous = previous_usd.max(0.0).floor() as u64;
    let current = current_usd.max(0.0).floor() as u64;
    if current <= previous {
        return Vec::new();
    }

    let mut milestones = FIRST_LIFETIME_USD_MILESTONES
        .into_iter()
        .filter(|threshold| previous < *threshold && current >= *threshold)
        .collect::<Vec<_>>();

    let first_repeating_index = previous / REPEATING_LIFETIME_USD_MILESTONE_STEP + 1;
    let last_repeating_index = current / REPEATING_LIFETIME_USD_MILESTONE_STEP;
    for index in first_repeating_index..=last_repeating_index {
        let dollars = index.saturating_mul(REPEATING_LIFETIME_USD_MILESTONE_STEP);
        if !milestones.contains(&dollars) {
            milestones.push(dollars);
        }
    }

    milestones
}

fn lifetime_token_milestones_crossed(previous_total: u64, current_total: u64) -> Vec<u64> {
    if current_total <= previous_total {
        return Vec::new();
    }

    let mut milestones = FIRST_LIFETIME_TOKEN_MILESTONES
        .into_iter()
        .filter(|threshold| previous_total < *threshold && current_total >= *threshold)
        .collect::<Vec<_>>();

    let first_repeating_index = previous_total / REPEATING_LIFETIME_TOKEN_MILESTONE_STEP + 1;
    let last_repeating_index = current_total / REPEATING_LIFETIME_TOKEN_MILESTONE_STEP;
    for index in first_repeating_index..=last_repeating_index {
        milestones.push(index.saturating_mul(REPEATING_LIFETIME_TOKEN_MILESTONE_STEP));
    }

    milestones
}

fn load_persisted_savings_state(path: &Path) -> Result<Option<PersistedSavingsState>> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let persisted = serde_json::from_slice::<PersistedSavingsState>(&bytes)
        .with_context(|| format!("parsing {}", path.display()))?;
    if persisted.schema_version == 3 {
        Ok(Some(persisted))
    } else {
        Ok(None)
    }
}

fn build_insights(
    recent_usage: &[UsageEvent],
    clients: &[ClientStatus],
    python_runtime_installed: bool,
) -> Vec<DailyInsight> {
    let mut insights = generate_daily_insights(recent_usage);

    if !python_runtime_installed {
        insights.push(DailyInsight {
            id: "runtime-missing".into(),
            category: crate::models::InsightCategory::Health,
            severity: crate::models::InsightSeverity::Warning,
            title: "Managed Python runtime not installed".into(),
            recommendation:
                "Complete bootstrap so Headroom can be installed into Headroom-managed storage."
                    .into(),
            evidence:
                "Headroom keeps the initial app download small and installs tools after first launch."
                    .into(),
            related_workspace: None,
        });
    }

    if clients.iter().all(|client| !client.installed) {
        insights.push(DailyInsight {
            id: "clients-missing".into(),
            category: crate::models::InsightCategory::Workflow,
            severity: crate::models::InsightSeverity::Info,
            title: "No supported clients detected yet".into(),
            recommendation:
                "Install a supported client to start routing requests through Headroom.".into(),
            evidence: "Client adapters look for known local executables during startup.".into(),
            related_workspace: None,
        });
    }

    insights
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
struct HeadroomSavingsHistoryPoint {
    timestamp: chrono::DateTime<Utc>,
    total_tokens_saved: u64,
}

#[derive(Debug, Default, Clone)]
struct HeadroomDashboardStats {
    session_requests: Option<usize>,
    session_estimated_savings_usd: Option<f64>,
    session_estimated_tokens_saved: Option<u64>,
    session_savings_pct: Option<f64>,
    session_actual_cost_usd: Option<f64>,
    session_total_tokens_sent: Option<u64>,
    savings_history: Vec<HeadroomSavingsHistoryPoint>,
    output_reduction: Option<OutputReduction>,
}

/// Counterfactual output-token reduction from the proxy's output shaper,
/// parsed from `/stats` (`savings.by_layer.output_shaping`). `method` is
/// "estimated" (synthetic control vs a learned baseline) or "measured" (A/B
/// holdout); the percentage always carries a 95% confidence band. Only
/// populated when the proxy reports `available: true` (i.e. a baseline exists).
#[derive(Debug, Clone)]
struct OutputReduction {
    method: String,
    reduction_percent: f64,
    ci_low_percent: f64,
    ci_high_percent: f64,
    requests: u64,
}

/// One provider's slice of a rollup bucket's delta, parsed from the upstream
/// `by_provider` map (`anthropic` / `openai` / `unknown`). Field names mirror the
/// bucket total; `hourly_savings` maps these to the display `ProviderSavingsPoint`.
#[derive(Debug, Default, Clone)]
struct ProviderRollupDelta {
    provider: String,
    tokens_saved: u64,
    compression_savings_usd_delta: f64,
    total_input_tokens_delta: u64,
    total_input_cost_usd_delta: f64,
}

#[derive(Debug, Default, Clone)]
struct HeadroomSavingsRollupPoint {
    timestamp: chrono::DateTime<Utc>,
    tokens_saved: u64,
    compression_savings_usd_delta: f64,
    total_input_tokens_delta: u64,
    total_input_cost_usd_delta: f64,
    by_provider: Vec<ProviderRollupDelta>,
}

#[derive(Debug, Default, Clone)]
struct HeadroomSavingsHistoryResponse {
    lifetime_estimated_savings_usd: Option<f64>,
    lifetime_estimated_tokens_saved: Option<u64>,
    hourly: Vec<HeadroomSavingsRollupPoint>,
    daily: Vec<HeadroomSavingsRollupPoint>,
}

impl HeadroomSavingsHistoryResponse {
    fn daily_savings(&self) -> Vec<DailySavingsPoint> {
        self.daily
            .iter()
            .map(|point| DailySavingsPoint {
                date: local_day_key(point.timestamp.with_timezone(&Local)),
                estimated_savings_usd: point.compression_savings_usd_delta,
                estimated_tokens_saved: point.tokens_saved,
                actual_cost_usd: point.total_input_cost_usd_delta,
                total_tokens_sent: point.total_input_tokens_delta,
            })
            .collect()
    }

    fn hourly_savings(&self) -> Vec<HourlySavingsPoint> {
        self.hourly
            .iter()
            .map(|point| HourlySavingsPoint {
                hour: local_hour_key(point.timestamp.with_timezone(&Local)),
                estimated_savings_usd: point.compression_savings_usd_delta,
                estimated_tokens_saved: point.tokens_saved,
                actual_cost_usd: point.total_input_cost_usd_delta,
                total_tokens_sent: point.total_input_tokens_delta,
                by_provider: point
                    .by_provider
                    .iter()
                    .map(|p| crate::models::ProviderSavingsPoint {
                        provider: p.provider.clone(),
                        estimated_savings_usd: p.compression_savings_usd_delta,
                        estimated_tokens_saved: p.tokens_saved,
                        actual_cost_usd: p.total_input_cost_usd_delta,
                        total_tokens_sent: p.total_input_tokens_delta,
                    })
                    .collect(),
            })
            .collect()
    }
}

fn fetch_headroom_dashboard_stats() -> Option<HeadroomDashboardStats> {
    if !is_headroom_proxy_reachable() {
        return None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .ok()?;

    let hosts = ["127.0.0.1", "localhost"];

    for host in hosts {
        let url = format!("http://{host}:6767/stats");
        let response = match client.get(&url).send() {
            Ok(response) if response.status().is_success() => response,
            _ => continue,
        };

        let body = match response.text() {
            Ok(body) => body,
            Err(_) => continue,
        };

        if let Some(parsed) = parse_headroom_stats_from_json(&body) {
            return Some(parsed);
        }
    }

    None
}

fn fetch_headroom_savings_history() -> Option<HeadroomSavingsHistoryResponse> {
    if !is_headroom_proxy_reachable() {
        return None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .ok()?;

    let hosts = ["127.0.0.1", "localhost"];

    for host in hosts {
        let url = format!("http://{host}:6767/stats-history");
        let response = match client.get(&url).send() {
            Ok(response) if response.status().is_success() => response,
            _ => continue,
        };

        let body = match response.text() {
            Ok(body) => body,
            Err(_) => continue,
        };

        if let Some(parsed) = parse_headroom_stats_history_from_json(&body) {
            return Some(parsed);
        }
    }

    None
}

/// Parse the output-shaper reduction estimate from a `/stats` payload. Lives
/// under `savings.by_layer.output_shaping`, with `tokens.output_reduction` as a
/// fallback. Returns `None` unless the proxy reports `available: true`, so the
/// dashboard hides the stat until a baseline has been seeded.
fn parse_output_reduction(root: &Value) -> Option<OutputReduction> {
    let node = value_at_path(root, &["savings", "by_layer", "output_shaping"])
        .or_else(|| value_at_path(root, &["tokens", "output_reduction"]))?;

    if !node
        .get("available")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }

    Some(OutputReduction {
        method: node
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("estimated")
            .to_string(),
        reduction_percent: node
            .get("reduction_percent")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        ci_low_percent: node
            .get("ci_low_percent")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        ci_high_percent: node
            .get("ci_high_percent")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        requests: node.get("requests").and_then(Value::as_u64).unwrap_or(0),
    })
}

fn parse_headroom_stats_from_json(body: &str) -> Option<HeadroomDashboardStats> {
    let root = serde_json::from_str::<Value>(body).ok()?;

    let path_requests = value_at_path_u64(&root, &["requests", "total"])
        .and_then(|value| usize::try_from(value).ok());
    let path_tokens = value_at_path_u64(&root, &["tokens", "saved"])
        .or_else(|| value_at_path_u64(&root, &["tokens", "compression_saved"]))
        .or_else(|| value_at_path_u64(&root, &["compression", "tokens_saved"]));
    let path_usd = value_at_path_f64(&root, &["cost", "compression_savings_usd"])
        .or_else(|| value_at_path_f64(&root, &["cost", "compression_saved_usd"]))
        .or_else(|| value_at_path_f64(&root, &["compression", "savings_usd"]));
    let path_actual_cost_usd = value_at_path_f64(&root, &["cost", "total_input_cost_usd"])
        .or_else(|| value_at_path_f64(&root, &["cost", "cost_with_headroom_usd"]))
        .or_else(|| value_at_path_f64(&root, &["cost", "actual_input_cost_usd"]))
        .or_else(|| value_at_path_f64(&root, &["cost", "input_actual_cost_usd"]))
        .or_else(|| value_at_path_f64(&root, &["cost", "input_cost_usd"]))
        .or_else(|| value_at_path_f64(&root, &["cost", "actual_cost_usd"]))
        .or_else(|| value_at_path_f64(&root, &["cost", "actual_usd"]))
        .or_else(|| value_at_path_f64(&root, &["cost", "actual_input_usd"]));
    let path_savings_pct = value_at_path_f64(&root, &["tokens", "savings_percent"]);
    let requests = path_requests.or_else(|| {
        find_u64_key_recursive(
            &root,
            &["total_requests", "totalRequests", "requests_total"],
        )
        .and_then(|value| usize::try_from(value).ok())
    });

    let tokens = path_tokens.or_else(|| {
        find_u64_key_recursive(
            &root,
            &[
                "compressionTokensSaved",
                "compression_tokens_saved",
                "totalCompressionTokensSaved",
                "total_compression_tokens_saved",
            ],
        )
    });

    let usd = path_usd.or_else(|| {
        find_f64_key_recursive(
            &root,
            &[
                "compressionSavingsUsd",
                "compression_savings_usd",
                "compressionSavedUsd",
                "compression_saved_usd",
                "compressionCostSavedUsd",
                "compression_cost_saved_usd",
            ],
        )
    });
    // Denominator for the savings ratio = "new input" this turn only.
    // Claude Code re-sends the entire conversation every turn; the cached
    // prefix (cache_read tokens) is forwarded but Headroom deliberately never
    // compresses it -- doing so would bust the provider prefix cache for a net
    // loss. Counting those re-sent cached tokens in the denominator drove the
    // displayed ratio toward zero as sessions grew longer and caching got more
    // effective. Under provider prompt caching, genuinely-new content lands in
    // cache_write (1.25x), NOT in uncached_input -- which collapses to ~0 and
    // would blow the ratio up to ~100%. New input we can actually compress is
    // cache_write + uncached, so measure compression against that.
    let new_input_tokens = {
        let cache_write =
            value_at_path_u64(&root, &["prefix_cache", "totals", "cache_write_tokens"]);
        let uncached =
            value_at_path_u64(&root, &["prefix_cache", "totals", "uncached_input_tokens"]).or_else(
                || find_u64_key_recursive(&root, &["uncachedInputTokens", "uncached_input_tokens"]),
            );
        match (cache_write, uncached) {
            (None, None) => None,
            (write, uncached) => Some(write.unwrap_or(0).saturating_add(uncached.unwrap_or(0))),
        }
    };
    let total_after_compression = value_at_path_u64(&root, &["tokens", "input"])
        .or_else(|| value_at_path_u64(&root, &["cost", "total_input_tokens"]))
        .or_else(|| value_at_path_u64(&root, &["tokens", "actual_input_tokens"]))
        .or_else(|| value_at_path_u64(&root, &["tokens", "input_tokens"]))
        .or_else(|| value_at_path_u64(&root, &["tokens", "total_after_compression"]))
        .or_else(|| value_at_path_u64(&root, &["tokens", "after_compression"]))
        .or_else(|| value_at_path_u64(&root, &["tokens", "sent"]))
        .or_else(|| {
            find_u64_key_recursive(
                &root,
                &[
                    "actualInputTokens",
                    "actual_input_tokens",
                    "totalInputTokens",
                    "total_input_tokens",
                    "inputTokens",
                    "input_tokens",
                    "totalAfterCompression",
                    "total_after_compression",
                    "tokensSent",
                    "tokens_sent",
                    "totalTokensSent",
                    "total_tokens_sent",
                ],
            )
        });
    // Prefer new input (cache_write + uncached); fall back to total forwarded
    // tokens for proxy builds that do not report prefix-cache totals (back-compat).
    // Filter the primary to >0 *before* the fallback: new_input_tokens is
    // Some(0) on a fully-cached snapshot, and `.or` only fires on None -- without
    // this the Some(0) skips the fallback and is then dropped, losing a valid count.
    let session_total_tokens_sent = new_input_tokens
        .filter(|value| *value > 0)
        .or(total_after_compression)
        .filter(|value| *value > 0);
    // Ratio against new input: saved / (saved + uncached_forwarded). The
    // proxy's own `tokens.savings_percent` is computed against the cache-
    // polluted denominator, so it is only a last-resort fallback.
    let session_savings_pct = tokens
        .and_then(|saved| {
            session_total_tokens_sent.and_then(|sent| {
                let total_before = saved.saturating_add(sent);
                if total_before > 0 {
                    Some(saved as f64 / total_before as f64 * 100.0)
                } else {
                    None
                }
            })
        })
        .or(path_savings_pct);
    let actual_cost_usd = path_actual_cost_usd.or_else(|| {
        find_f64_key_recursive(
            &root,
            &[
                "totalInputCostUsd",
                "total_input_cost_usd",
                "costWithHeadroomUsd",
                "cost_with_headroom_usd",
                "actualInputCostUsd",
                "actual_input_cost_usd",
                "inputActualCostUsd",
                "input_actual_cost_usd",
                "inputCostUsd",
                "input_cost_usd",
                "actualCostUsd",
                "actual_cost_usd",
                "actualUsd",
                "actual_usd",
                "actualInputUsd",
                "actual_input_usd",
            ],
        )
    });
    let savings_history = value_at_path(&root, &["compression_savings_history"])
        .or_else(|| value_at_path(&root, &["compression", "savings_history"]))
        .or_else(|| value_at_path(&root, &["savings_history"]))
        .and_then(parse_savings_history)
        .unwrap_or_default();

    let output_reduction = parse_output_reduction(&root);

    if requests.is_none()
        && tokens.is_none()
        && usd.is_none()
        && session_total_tokens_sent.is_none()
        && actual_cost_usd.is_none()
        && output_reduction.is_none()
    {
        None
    } else {
        Some(HeadroomDashboardStats {
            session_requests: requests,
            session_estimated_savings_usd: usd,
            session_estimated_tokens_saved: tokens,
            session_savings_pct,
            session_actual_cost_usd: actual_cost_usd.map(|value| value.max(0.0)),
            session_total_tokens_sent,
            savings_history,
            output_reduction,
        })
    }
}

/// True when the upstream stored history hit its point-count cap and older
/// checkpoints were trimmed away. In that state the oldest surviving rollup
/// bucket carries a spurious carried-over cumulative as its delta.
fn upstream_history_trimmed(root: &Value) -> bool {
    let stored = value_at_path_u64(root, &["history_summary", "stored_points"]);
    let cap = value_at_path_u64(root, &["retention", "max_history_points"]);
    matches!((stored, cap), (Some(stored), Some(cap)) if cap > 0 && stored >= cap)
}

/// Remove the oldest bucket (smallest timestamp) from a rollup series.
fn drop_oldest_rollup_bucket(series: &mut Vec<HeadroomSavingsRollupPoint>) {
    if let Some((idx, _)) = series
        .iter()
        .enumerate()
        .min_by_key(|(_, point)| point.timestamp)
    {
        series.remove(idx);
    }
}

fn parse_headroom_stats_history_from_json(body: &str) -> Option<HeadroomSavingsHistoryResponse> {
    let root = serde_json::from_str::<Value>(body).ok()?;
    let lifetime_estimated_tokens_saved = value_at_path_u64(&root, &["lifetime", "tokens_saved"]);
    let lifetime_estimated_savings_usd =
        value_at_path_f64(&root, &["lifetime", "compression_savings_usd"]);
    let mut hourly = value_at_path(&root, &["series", "hourly"])
        .and_then(parse_savings_rollup_series)
        .unwrap_or_default();
    let mut daily = value_at_path(&root, &["series", "daily"])
        .and_then(parse_savings_rollup_series)
        .unwrap_or_default();

    // When the upstream stored history has been trimmed (point-count cap
    // reached), the backend's rollup diffs the oldest surviving checkpoint from
    // a zero baseline, dumping its entire cumulative into the first bucket's
    // delta. That produces a huge spurious spike at the window's leading edge
    // that slides forward as old checkpoints age out. Drop that boundary bucket
    // so the chart shows real per-bucket savings. Untrimmed histories (new
    // users) keep their genuine first bucket. Lifetime totals are unaffected.
    if upstream_history_trimmed(&root) {
        drop_oldest_rollup_bucket(&mut daily);
        drop_oldest_rollup_bucket(&mut hourly);
    }

    if lifetime_estimated_tokens_saved.is_none()
        && lifetime_estimated_savings_usd.is_none()
        && hourly.is_empty()
        && daily.is_empty()
    {
        None
    } else {
        Some(HeadroomSavingsHistoryResponse {
            lifetime_estimated_savings_usd: lifetime_estimated_savings_usd
                .map(|value| value.max(0.0)),
            lifetime_estimated_tokens_saved,
            hourly,
            daily,
        })
    }
}

fn value_at_path_u64(root: &Value, path: &[&str]) -> Option<u64> {
    let value = value_at_path(root, path)?;
    parse_u64_value(value)
}

fn value_at_path_f64(root: &Value, path: &[&str]) -> Option<f64> {
    let value = value_at_path(root, path)?;
    parse_f64_value(value)
}

fn value_at_path<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        match current {
            Value::Object(map) => {
                current = map.get(*segment)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn parse_savings_history(value: &Value) -> Option<Vec<HeadroomSavingsHistoryPoint>> {
    let Value::Array(items) = value else {
        return None;
    };
    let points = items
        .iter()
        .filter_map(parse_savings_history_point)
        .collect::<Vec<_>>();
    Some(points)
}

fn parse_savings_rollup_series(value: &Value) -> Option<Vec<HeadroomSavingsRollupPoint>> {
    let Value::Array(items) = value else {
        return None;
    };
    let points = items
        .iter()
        .filter_map(parse_savings_rollup_point)
        .collect::<Vec<_>>();
    Some(points)
}

fn parse_savings_history_point(value: &Value) -> Option<HeadroomSavingsHistoryPoint> {
    match value {
        Value::Array(items) if items.len() >= 2 => {
            let timestamp = items.first()?.as_str().and_then(parse_history_timestamp)?;
            let total_tokens_saved = parse_u64_value(items.get(1)?)?;
            Some(HeadroomSavingsHistoryPoint {
                timestamp,
                total_tokens_saved,
            })
        }
        Value::Object(map) => {
            let timestamp = map
                .get("timestamp")
                .and_then(|value| value.as_str())
                .and_then(parse_history_timestamp)?;
            let total_tokens_saved = map
                .get("total_tokens_saved")
                .or_else(|| map.get("tokens_saved"))
                .and_then(parse_u64_value)?;
            Some(HeadroomSavingsHistoryPoint {
                timestamp,
                total_tokens_saved,
            })
        }
        _ => None,
    }
}

fn parse_savings_rollup_point(value: &Value) -> Option<HeadroomSavingsRollupPoint> {
    let Value::Object(map) = value else {
        return None;
    };

    let timestamp = map
        .get("timestamp")
        .and_then(|value| value.as_str())
        .and_then(parse_history_timestamp)?;

    Some(HeadroomSavingsRollupPoint {
        timestamp,
        tokens_saved: map
            .get("tokens_saved")
            .and_then(parse_u64_value)
            .unwrap_or_default(),
        compression_savings_usd_delta: map
            .get("compression_savings_usd_delta")
            .and_then(parse_f64_value)
            .unwrap_or_default()
            .max(0.0),
        total_input_tokens_delta: map
            .get("total_input_tokens_delta")
            .and_then(parse_u64_value)
            .unwrap_or_default(),
        total_input_cost_usd_delta: map
            .get("total_input_cost_usd_delta")
            .and_then(parse_f64_value)
            .unwrap_or_default()
            .max(0.0),
        by_provider: parse_rollup_by_provider(map.get("by_provider")),
    })
}

/// Parse the upstream `by_provider` map (`{ "anthropic": { tokens_saved, ... }, ... }`)
/// into a deterministically-ordered list. Missing/empty yields an empty Vec, so
/// pre-feature buckets carry no provider breakdown.
fn parse_rollup_by_provider(value: Option<&Value>) -> Vec<ProviderRollupDelta> {
    let Some(Value::Object(providers)) = value else {
        return Vec::new();
    };
    let mut out: Vec<ProviderRollupDelta> = providers
        .iter()
        .map(|(provider, entry)| {
            let get_u64 = |key: &str| entry.get(key).and_then(parse_u64_value).unwrap_or_default();
            let get_f64 = |key: &str| {
                entry
                    .get(key)
                    .and_then(parse_f64_value)
                    .unwrap_or_default()
                    .max(0.0)
            };
            ProviderRollupDelta {
                provider: provider.clone(),
                tokens_saved: get_u64("tokens_saved"),
                compression_savings_usd_delta: get_f64("compression_savings_usd_delta"),
                total_input_tokens_delta: get_u64("total_input_tokens_delta"),
                total_input_cost_usd_delta: get_f64("total_input_cost_usd_delta"),
            }
        })
        .collect();
    out.sort_by(|a, b| a.provider.cmp(&b.provider));
    out
}

fn parse_history_timestamp(text: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .and_then(|timestamp| Local.from_local_datetime(&timestamp).single())
                .map(|timestamp| timestamp.with_timezone(&Utc))
        })
}

fn local_day_key(timestamp: chrono::DateTime<Local>) -> String {
    timestamp.format("%Y-%m-%d").to_string()
}

// Boundary between local tracker (pre-cutoff, authoritative) and /stats-history
// (cutoff and later, authoritative). Release builds pin to the date the schema
// stabilized; debug builds track "today" so dev sessions never fall behind the
// history source while iterating.
fn savings_history_cutoff_date() -> String {
    if cfg!(debug_assertions) {
        local_day_key(Local::now())
    } else {
        "2026-06-02".to_string()
    }
}

fn local_hour_key(timestamp: chrono::DateTime<Local>) -> String {
    timestamp.format("%Y-%m-%dT%H:00").to_string()
}

fn day_key_from_hour_key(hour_key: &str) -> String {
    hour_key.split('T').next().unwrap_or(hour_key).to_string()
}

fn should_rollover_display_session(
    last_activity_at: chrono::DateTime<Utc>,
    now: chrono::DateTime<Utc>,
) -> bool {
    let last_local = last_activity_at.with_timezone(&Local);
    let now_local = now.with_timezone(&Local);
    now_local.date_naive() > last_local.date_naive()
        && now.signed_duration_since(last_activity_at) >= chrono::Duration::hours(1)
}

fn derive_session_buckets_with_key<F>(
    stats: &HeadroomDashboardStats,
    history: &[HeadroomSavingsHistoryPoint],
    bucket_key_for_timestamp: F,
) -> Vec<(String, DailySavingsBucket)>
where
    F: Fn(chrono::DateTime<Local>) -> String,
{
    let total_tokens = stats.session_estimated_tokens_saved.unwrap_or(0);
    let total_usd = stats.session_estimated_savings_usd.unwrap_or(0.0).max(0.0);
    let total_tokens_sent = stats.session_total_tokens_sent.unwrap_or(0);
    let total_actual_cost_usd = stats.session_actual_cost_usd.unwrap_or(0.0).max(0.0);
    if total_tokens == 0
        && total_usd <= 0.0
        && total_tokens_sent == 0
        && total_actual_cost_usd <= 0.0
    {
        return Vec::new();
    }

    let mut buckets = BTreeMap::<String, DailySavingsBucket>::new();
    let Some(first_point) = history.first().copied() else {
        return Vec::new();
    };
    let mut previous_total = first_point.total_tokens_saved;
    let mut history_total = 0u64;

    for point in history.iter().copied().skip(1) {
        let delta_tokens = point.total_tokens_saved.saturating_sub(previous_total);
        previous_total = point.total_tokens_saved;
        if delta_tokens == 0 {
            continue;
        }
        history_total = history_total.saturating_add(delta_tokens);
        let bucket_key = bucket_key_for_timestamp(point.timestamp.with_timezone(&Local));
        let entry = buckets.entry(bucket_key).or_default();
        entry.estimated_tokens_saved = entry.estimated_tokens_saved.saturating_add(delta_tokens);
    }

    if buckets.is_empty() || history_total == 0 || history_total > total_tokens {
        return Vec::new();
    }

    if total_tokens > 0 && total_usd > 0.0 {
        let usd_per_token = total_usd / total_tokens as f64;
        for bucket in buckets.values_mut() {
            bucket.estimated_savings_usd = bucket.estimated_tokens_saved as f64 * usd_per_token;
        }
    }

    if total_tokens > 0 && total_tokens_sent > 0 {
        let keys = buckets.keys().cloned().collect::<Vec<_>>();
        for key in keys.iter() {
            let bucket = buckets.get_mut(key).expect("bucket exists");
            bucket.total_tokens_sent = ((bucket.estimated_tokens_saved as u128
                * total_tokens_sent as u128)
                / total_tokens as u128) as u64;
        }
    }

    if total_tokens > 0 && total_actual_cost_usd > 0.0 {
        let keys = buckets.keys().cloned().collect::<Vec<_>>();
        for key in keys.iter() {
            let bucket = buckets.get_mut(key).expect("bucket exists");
            bucket.actual_cost_usd = total_actual_cost_usd
                * (bucket.estimated_tokens_saved as f64 / total_tokens as f64);
        }
    }

    buckets.into_iter().collect()
}

fn merge_session_savings_history(
    existing: &[HeadroomSavingsHistoryPoint],
    incoming: &[HeadroomSavingsHistoryPoint],
) -> Vec<HeadroomSavingsHistoryPoint> {
    let mut merged = BTreeMap::new();
    for point in existing.iter().chain(incoming.iter()) {
        merged
            .entry(point.timestamp)
            .and_modify(|value: &mut u64| *value = (*value).max(point.total_tokens_saved))
            .or_insert(point.total_tokens_saved);
    }

    let mut normalized = Vec::with_capacity(merged.len());
    let mut previous_total = 0u64;
    for (timestamp, total_tokens_saved) in merged {
        if !normalized.is_empty() && total_tokens_saved < previous_total {
            continue;
        }
        previous_total = total_tokens_saved;
        normalized.push(HeadroomSavingsHistoryPoint {
            timestamp,
            total_tokens_saved,
        });
    }
    normalized
}

fn derive_session_hourly_buckets(
    stats: &HeadroomDashboardStats,
    history: &[HeadroomSavingsHistoryPoint],
) -> Vec<(String, DailySavingsBucket)> {
    derive_session_buckets_with_key(stats, history, local_hour_key)
}

fn diff_hourly_buckets(
    previous: &BTreeMap<String, DailySavingsBucket>,
    current: &[(String, DailySavingsBucket)],
) -> Vec<(String, DailySavingsBucket)> {
    current
        .iter()
        .filter_map(|(hour_key, bucket)| {
            let prior = previous.get(hour_key).copied().unwrap_or_default();
            let delta = DailySavingsBucket {
                estimated_savings_usd: (bucket.estimated_savings_usd - prior.estimated_savings_usd)
                    .max(0.0),
                estimated_tokens_saved: bucket
                    .estimated_tokens_saved
                    .saturating_sub(prior.estimated_tokens_saved),
                actual_cost_usd: (bucket.actual_cost_usd - prior.actual_cost_usd).max(0.0),
                total_tokens_sent: bucket
                    .total_tokens_sent
                    .saturating_sub(prior.total_tokens_sent),
            };
            if delta.estimated_savings_usd <= 0.0
                && delta.estimated_tokens_saved == 0
                && delta.actual_cost_usd <= 0.0
                && delta.total_tokens_sent == 0
            {
                None
            } else {
                Some((hour_key.clone(), delta))
            }
        })
        .collect()
}

fn build_hourly_backfill_records(
    buckets: &[(String, DailySavingsBucket)],
    session_requests: usize,
    session_savings_usd: f64,
    session_tokens_saved: u64,
    session_actual_cost_usd: f64,
    session_total_tokens_sent: u64,
) -> Vec<SavingsRecord> {
    if buckets.is_empty() {
        return vec![SavingsRecord {
            schema_version: 7,
            id: Uuid::new_v4().to_string(),
            observed_at: Utc::now(),
            day_key: local_day_key(Local::now()),
            hour_key: local_hour_key(Local::now()),
            session_requests,
            session_estimated_savings_usd: session_savings_usd,
            session_estimated_tokens_saved: session_tokens_saved,
            session_actual_cost_usd,
            session_total_tokens_sent,
            delta_requests: session_requests,
            delta_estimated_savings_usd: 0.0,
            delta_estimated_tokens_saved: 0,
            delta_actual_cost_usd: 0.0,
            delta_total_tokens_sent: 0,
            source: "headroom_dashboard_backfill".into(),
        }];
    }

    let latest_index = buckets.len() - 1;
    buckets
        .iter()
        .enumerate()
        .map(|(index, (hour_key, bucket))| SavingsRecord {
            schema_version: 7,
            id: Uuid::new_v4().to_string(),
            observed_at: Utc::now(),
            day_key: day_key_from_hour_key(hour_key),
            hour_key: hour_key.clone(),
            session_requests: if index == latest_index {
                session_requests
            } else {
                0
            },
            session_estimated_savings_usd: if index == latest_index {
                session_savings_usd
            } else {
                0.0
            },
            session_estimated_tokens_saved: if index == latest_index {
                session_tokens_saved
            } else {
                0
            },
            session_actual_cost_usd: if index == latest_index {
                session_actual_cost_usd
            } else {
                0.0
            },
            session_total_tokens_sent: if index == latest_index {
                session_total_tokens_sent
            } else {
                0
            },
            delta_requests: if index == latest_index {
                session_requests
            } else {
                0
            },
            delta_estimated_savings_usd: bucket.estimated_savings_usd,
            delta_estimated_tokens_saved: bucket.estimated_tokens_saved,
            delta_actual_cost_usd: bucket.actual_cost_usd,
            delta_total_tokens_sent: bucket.total_tokens_sent,
            source: "headroom_dashboard_backfill".into(),
        })
        .collect()
}

fn build_hourly_delta_records(
    buckets: &[(String, DailySavingsBucket)],
    session_requests: usize,
    session_savings_usd: f64,
    session_tokens_saved: u64,
    session_actual_cost_usd: f64,
    session_total_tokens_sent: u64,
    delta_requests: usize,
) -> Vec<SavingsRecord> {
    if buckets.is_empty() {
        return Vec::new();
    }

    let latest_index = buckets.len() - 1;
    buckets
        .iter()
        .enumerate()
        .filter(|(_, (_, bucket))| bucket.actual_cost_usd > 0.0)
        .map(|(index, (hour_key, bucket))| SavingsRecord {
            schema_version: 7,
            id: Uuid::new_v4().to_string(),
            observed_at: Utc::now(),
            day_key: day_key_from_hour_key(hour_key),
            hour_key: hour_key.clone(),
            session_requests: if index == latest_index {
                session_requests
            } else {
                0
            },
            session_estimated_savings_usd: if index == latest_index {
                session_savings_usd
            } else {
                0.0
            },
            session_estimated_tokens_saved: if index == latest_index {
                session_tokens_saved
            } else {
                0
            },
            session_actual_cost_usd: if index == latest_index {
                session_actual_cost_usd
            } else {
                0.0
            },
            session_total_tokens_sent: if index == latest_index {
                session_total_tokens_sent
            } else {
                0
            },
            delta_requests: if index == latest_index {
                delta_requests
            } else {
                0
            },
            delta_estimated_savings_usd: bucket.estimated_savings_usd,
            delta_estimated_tokens_saved: bucket.estimated_tokens_saved,
            delta_actual_cost_usd: bucket.actual_cost_usd,
            delta_total_tokens_sent: bucket.total_tokens_sent,
            source: "headroom_dashboard".into(),
        })
        .collect()
}

fn find_u64_key_recursive(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                if keys
                    .iter()
                    .any(|candidate| key.eq_ignore_ascii_case(candidate))
                {
                    if let Some(parsed) = parse_u64_value(v) {
                        return Some(parsed);
                    }
                }
                if let Some(found) = find_u64_key_recursive(v, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_u64_key_recursive(item, keys)),
        _ => None,
    }
}

fn find_f64_key_recursive(value: &Value, keys: &[&str]) -> Option<f64> {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                if keys
                    .iter()
                    .any(|candidate| key.eq_ignore_ascii_case(candidate))
                {
                    if let Some(parsed) = parse_f64_value(v) {
                        return Some(parsed);
                    }
                }
                if let Some(found) = find_f64_key_recursive(v, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_f64_key_recursive(item, keys)),
        _ => None,
    }
}

fn parse_u64_value(value: &Value) -> Option<u64> {
    match value {
        Value::Number(num) => num
            .as_u64()
            .or_else(|| {
                num.as_i64()
                    .and_then(|v| if v >= 0 { Some(v as u64) } else { None })
            })
            .or_else(|| {
                num.as_f64()
                    .and_then(|v| if v >= 0.0 { Some(v as u64) } else { None })
            }),
        Value::String(text) => parse_u64_from_text(text),
        _ => None,
    }
}

fn parse_f64_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(num) => num.as_f64(),
        Value::String(text) => parse_f64_from_text(text),
        _ => None,
    }
}

fn parse_u64_from_text(text: &str) -> Option<u64> {
    let mut numeric = String::new();
    let mut started = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            numeric.push(ch);
            started = true;
            continue;
        }
        if started && (ch == ',' || ch == '_') {
            continue;
        }
        if started {
            break;
        }
    }
    if numeric.is_empty() {
        None
    } else {
        numeric.parse::<u64>().ok()
    }
}

fn parse_f64_from_text(text: &str) -> Option<f64> {
    let mut numeric = String::new();
    let mut started = false;
    for ch in text.chars() {
        let is_numeric = ch.is_ascii_digit() || ch == '.' || ch == '-';
        if is_numeric {
            numeric.push(ch);
            started = true;
            continue;
        }
        if started && (ch == ',' || ch == '_' || ch == '$' || ch.is_ascii_whitespace()) {
            continue;
        }
        if started {
            break;
        }
    }
    if numeric.is_empty() || numeric == "-" || numeric == "." {
        None
    } else {
        numeric.parse::<f64>().ok()
    }
}

pub(crate) fn headroom_proxy_reachable() -> bool {
    is_headroom_proxy_reachable()
}

fn is_headroom_proxy_reachable() -> bool {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    ["127.0.0.1", "localhost"].iter().any(|host| {
        client
            .get(format!("http://{host}:6767/readyz"))
            .send()
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    })
}

fn kill_processes_by_command_pattern(pattern: &str) -> Result<()> {
    #[cfg(unix)]
    {
        let status = Command::new("pkill")
            .args(["-f", pattern])
            .status()
            .with_context(|| format!("running pkill for pattern '{pattern}'"))?;

        if status.success() || status.code() == Some(1) {
            return Ok(());
        }

        return Err(anyhow!(
            "pkill exited with status {:?} for pattern '{}'",
            status.code(),
            pattern
        ));
    }

    #[cfg(not(unix))]
    {
        let _ = pattern;
        Ok(())
    }
}

/// Merge daily savings from tracker (pre-cutoff) and native headroom history (post-cutoff).
/// For days before `cutoff_date` (exclusive), the tracker is preferred.
/// For days on/after `cutoff_date`, native history is preferred.
/// Falls back to whichever source has data when the preferred one is absent.
fn merge_daily_savings(
    tracker: Vec<DailySavingsPoint>,
    history: Vec<DailySavingsPoint>,
    cutoff_date: &str,
) -> Vec<DailySavingsPoint> {
    use std::collections::BTreeMap;
    let mut by_date: BTreeMap<String, DailySavingsPoint> = BTreeMap::new();
    // Post-cutoff: history wins, tracker fills gaps so today's local activity still shows.
    // Pre-cutoff: tracker-only; history is ignored to avoid pulling in pre-v6 schema drift.
    for p in history {
        if p.date.as_str() >= cutoff_date {
            by_date.insert(p.date.clone(), p);
        }
    }
    for p in tracker {
        if p.date.as_str() < cutoff_date {
            by_date.insert(p.date.clone(), p);
        } else {
            by_date.entry(p.date.clone()).or_insert(p);
        }
    }
    by_date.into_values().collect()
}

/// Same logic as `merge_daily_savings` but for hourly buckets keyed by hour string.
fn merge_hourly_savings(
    tracker: Vec<HourlySavingsPoint>,
    history: Vec<HourlySavingsPoint>,
    cutoff_hour: &str,
) -> Vec<HourlySavingsPoint> {
    use std::collections::BTreeMap;
    let mut by_hour: BTreeMap<String, HourlySavingsPoint> = BTreeMap::new();
    for p in history {
        if p.hour.as_str() >= cutoff_hour {
            by_hour.insert(p.hour.clone(), p);
        }
    }
    for p in tracker {
        if p.hour.as_str() < cutoff_hour {
            by_hour.insert(p.hour.clone(), p);
        } else {
            by_hour.entry(p.hour.clone()).or_insert(p);
        }
    }
    by_hour.into_values().collect()
}

fn begin_bootstrap_transition(
    current: &BootstrapProgress,
    python_installed: bool,
) -> (BootstrapProgress, Result<(), String>) {
    if python_installed {
        return (
            BootstrapProgress {
                running: false,
                complete: true,
                failed: false,
                current_step: "Install complete".into(),
                message: "Managed runtime already installed.".into(),
                current_step_eta_seconds: 0,
                overall_percent: 100,
            },
            Ok(()),
        );
    }
    if current.running {
        return (current.clone(), Err("Bootstrap is already running.".into()));
    }
    (
        BootstrapProgress {
            running: true,
            complete: false,
            failed: false,
            current_step: "Preparing install".into(),
            message: "Initializing installer workflow.".into(),
            current_step_eta_seconds: 3,
            overall_percent: 2,
        },
        Ok(()),
    )
}

fn apply_bootstrap_step(
    _current: &BootstrapProgress,
    step: BootstrapStepUpdate,
) -> BootstrapProgress {
    BootstrapProgress {
        running: true,
        complete: false,
        failed: false,
        current_step: step.step.into(),
        message: step.message,
        current_step_eta_seconds: step.eta_seconds,
        overall_percent: step.percent,
    }
}

fn bootstrap_complete_state() -> BootstrapProgress {
    BootstrapProgress {
        running: false,
        complete: true,
        failed: false,
        current_step: "Install complete".into(),
        message: "AI Switchboard for Mac is ready.".into(),
        current_step_eta_seconds: 0,
        overall_percent: 100,
    }
}

fn bootstrap_failed_state(current: &BootstrapProgress, message: String) -> BootstrapProgress {
    BootstrapProgress {
        running: false,
        complete: false,
        failed: true,
        current_step: "Install failed".into(),
        message,
        current_step_eta_seconds: 0,
        overall_percent: current.overall_percent.max(1),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use chrono::{Datelike, Local, TimeZone, Timelike, Utc};

    use crate::storage::{config_file, ensure_data_dirs, telemetry_file};

    use crate::models::{
        ActivityEvent, BootstrapProgress, DailySavingsPoint, HourlySavingsPoint, RepoContextPack,
        RepoIntelligenceSummary, RuntimeUpgradeFailure, SavingsAttributionConfidence,
        SavingsAttributionEvent, SavingsAttributionScope, SavingsAttributionSource,
        UpgradeFailurePhase,
    };
    use crate::runtime_boot_validation::boot_validation_stalled;
    use crate::tool_manager::{BootstrapStepUpdate, RtkGainSummary};

    use super::{
        aggregate_weekly_totals, apply_bootstrap_step, begin_bootstrap_transition,
        bootstrap_complete_state, bootstrap_failed_state, lifetime_token_milestones_crossed,
        lifetime_usd_milestones_crossed, log_mtime_advanced, merge_daily_savings,
        merge_hourly_savings, most_recent_monday, parse_headroom_stats_from_json,
        parse_headroom_stats_history_from_json, AppState, BootValidationOutcome, ClaudeProjectScan,
        DailySavingsBucket, HeadroomDashboardStats, HeadroomSavingsHistoryPoint,
        PersistedSavingsState, SavingsObservation, SavingsTracker,
    };

    #[test]
    fn boot_validation_stalled_within_grace_window_is_never_stalled() {
        use std::time::Duration;
        let grace = Duration::from_secs(60);
        let silence = Duration::from_secs(90);
        // Inside grace, ignore activity_age entirely.
        assert!(!boot_validation_stalled(
            Duration::from_secs(30),
            Duration::from_secs(120),
            grace,
            silence,
        ));
        // Boundary: elapsed == grace is NOT past grace (strict >).
        assert!(!boot_validation_stalled(
            Duration::from_secs(60),
            Duration::from_secs(120),
            grace,
            silence,
        ));
    }

    #[test]
    fn boot_validation_stalled_past_grace_with_recent_activity_is_not_stalled() {
        use std::time::Duration;
        let grace = Duration::from_secs(60);
        let silence = Duration::from_secs(90);
        // Past grace but log/HF moved within the silence window.
        assert!(!boot_validation_stalled(
            Duration::from_secs(120),
            Duration::from_secs(30),
            grace,
            silence,
        ));
        // Boundary: activity_age == silence is NOT past silence.
        assert!(!boot_validation_stalled(
            Duration::from_secs(120),
            Duration::from_secs(90),
            grace,
            silence,
        ));
    }

    #[test]
    fn boot_validation_stalled_past_grace_and_silence_fires() {
        use std::time::Duration;
        let grace = Duration::from_secs(60);
        let silence = Duration::from_secs(90);
        // Past grace, activity stale → stalled.
        assert!(boot_validation_stalled(
            Duration::from_secs(120),
            Duration::from_secs(91),
            grace,
            silence,
        ));
        // Reproduces the original Sentry trace shape (with old 45s
        // silence): 64.7s elapsed, ~50s of silence past mtime → stall.
        assert!(boot_validation_stalled(
            Duration::from_secs(64),
            Duration::from_secs(50),
            Duration::from_secs(60),
            Duration::from_secs(45),
        ));
        // Same trace with the new 90s silence and (no) HF growth signal:
        // would still stall, but only after another 40s. Without HF
        // growth refreshing activity_age, this is the worst-case bound.
        assert!(!boot_validation_stalled(
            Duration::from_secs(64),
            Duration::from_secs(50),
            Duration::from_secs(60),
            Duration::from_secs(90),
        ));
    }

    #[test]
    fn boot_validation_outcome_labels_are_stable() {
        // These labels become Sentry tags and analytics dimensions —
        // changing them silently invalidates dashboards.
        assert_eq!(BootValidationOutcome::Reachable.label(), "reachable");
        assert_eq!(
            BootValidationOutcome::ProcessExited.label(),
            "process_exited"
        );
        assert_eq!(BootValidationOutcome::Stalled.label(), "stalled");
        assert_eq!(BootValidationOutcome::TimedOut.label(), "timed_out");
        assert_eq!(BootValidationOutcome::NotStarted.label(), "not_started");
        assert!(BootValidationOutcome::Reachable.is_ok());
        assert!(!BootValidationOutcome::NotStarted.is_ok());
    }

    #[test]
    fn log_mtime_advanced_detects_first_observation_and_new_writes() {
        use std::time::{Duration, SystemTime};
        let t1 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let t2 = t1 + Duration::from_secs(1);

        // First time we see a log file.
        assert!(log_mtime_advanced(None, Some(t1)));
        // Newer write after a previous observation.
        assert!(log_mtime_advanced(Some(t1), Some(t2)));
        // No change.
        assert!(!log_mtime_advanced(Some(t1), Some(t1)));
        // Log "vanished" (shouldn't happen on a healthy boot, but the
        // function must not declare activity in that case).
        assert!(!log_mtime_advanced(Some(t1), None));
        // Both None — pre-first-write state, no activity.
        assert!(!log_mtime_advanced(None, None));
    }

    #[test]
    fn launch_profile_missing_new_fields_deserialize_as_none() {
        // Legacy profile JSON from before we added last_launched_app_version
        // and last_runtime_upgrade_failure. Must still parse.
        let legacy = br#"{
            "launch_count": 3,
            "launch_experience": "resume",
            "lifetime_requests": 0,
            "lifetime_estimated_savings_usd": 0.0,
            "lifetime_estimated_tokens_saved": 0
        }"#;
        let profile: super::LaunchProfile =
            serde_json::from_slice(legacy).expect("legacy profile parses");
        assert!(profile.last_launched_app_version.is_none());
        assert!(profile.last_runtime_upgrade_failure.is_none());
        assert!(!profile.setup_wizard_complete);
        // Legacy profiles predate terms gating: default to 0 so the gate
        // re-prompts once REQUIRED_TERMS_VERSION > 0.
        assert_eq!(profile.accepted_terms_version, 0);
    }

    #[test]
    fn persist_launch_profile_round_trips_new_fields() {
        let id = uuid::Uuid::new_v4();
        let path = std::env::temp_dir().join(format!("headroom-launch-profile-test-{}.json", id));
        let profile = super::LaunchProfile {
            launch_count: 1,
            launch_experience: crate::models::LaunchExperience::Resume,
            lifetime_requests: 0,
            lifetime_estimated_savings_usd: 0.0,
            lifetime_estimated_tokens_saved: 0,
            setup_wizard_complete: true,
            last_launched_app_version: Some("0.2.50".into()),
            last_runtime_upgrade_failure: Some(crate::models::RuntimeUpgradeFailure {
                app_version: "0.2.50".into(),
                target_headroom_version: "0.8.2".into(),
                fallback_headroom_version: Some("0.6.5".into()),
                failure_phase: crate::models::UpgradeFailurePhase::BootValidation,
                attempts: 2,
                first_attempt_at: Utc::now(),
                last_attempt_at: Utc::now(),
                error_message: "timed out".into(),
                error_hint: Some("Reverted to 0.6.5".into()),
                rollback_restored: true,
            }),
            accepted_terms_version: 3,
        };
        super::persist_launch_profile(&path, &profile);

        let bytes = std::fs::read(&path).expect("persisted");
        let round_tripped: super::LaunchProfile =
            serde_json::from_slice(&bytes).expect("re-parses");
        assert_eq!(
            round_tripped.last_launched_app_version.as_deref(),
            Some("0.2.50")
        );
        let failure = round_tripped
            .last_runtime_upgrade_failure
            .expect("failure present");
        assert_eq!(failure.attempts, 2);
        assert_eq!(failure.target_headroom_version, "0.8.2");
        assert_eq!(
            failure.failure_phase,
            crate::models::UpgradeFailurePhase::BootValidation
        );
        assert_eq!(round_tripped.accepted_terms_version, 3);
        let _ = std::fs::remove_file(&path);
    }

    fn make_tracker() -> SavingsTracker {
        let id = uuid::Uuid::new_v4();
        let records_path = std::env::temp_dir().join(format!("headroom-savings-test-{}.jsonl", id));
        let attribution_events_path =
            std::env::temp_dir().join(format!("headroom-savings-attribution-{}.jsonl", id));
        let state_path = std::env::temp_dir().join(format!("headroom-savings-state-{}.json", id));
        SavingsTracker {
            records_path,
            attribution_events_path,
            state_path,
            session_requests: 0,
            session_estimated_savings_usd: 0.0,
            session_estimated_tokens_saved: 0,
            session_savings_pct: 0.0,
            lifetime_requests: 0,
            lifetime_estimated_savings_usd: 0.0,
            lifetime_estimated_tokens_saved: 0,
            last_observation: None,
            last_rtk_observation: None,
            display_session_baseline: None,
            session_savings_history: Vec::new(),
            session_hourly_buckets: std::collections::BTreeMap::new(),
            daily_savings: std::collections::BTreeMap::new(),
            hourly_savings: std::collections::BTreeMap::new(),
            pending_lifetime_token_milestones: Vec::new(),
            pending_lifetime_usd_milestones: Vec::new(),
            last_written_at: None,
        }
    }

    fn history_point_at(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        total_tokens_saved: u64,
    ) -> HeadroomSavingsHistoryPoint {
        HeadroomSavingsHistoryPoint {
            timestamp: Utc
                .with_ymd_and_hms(year, month, day, hour, 0, 0)
                .single()
                .expect("valid timestamp"),
            total_tokens_saved,
        }
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()))
    }

    fn write_headroom_receipt(base_dir: &PathBuf, version: &str, requirements_lock_sha256: &str) {
        let runtime = crate::tool_manager::ManagedRuntime::bootstrap_root(base_dir);
        fs::create_dir_all(&runtime.tools_dir).expect("create tools dir");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            format!(
                r#"{{
                    "version":"{}",
                    "artifact":{{"requirementsLockSha256":"{}"}}
                }}"#,
                version, requirements_lock_sha256
            ),
        )
        .expect("write receipt");
    }

    #[test]
    fn lifetime_usd_milestones_first_and_repeating() {
        assert_eq!(lifetime_usd_milestones_crossed(0.0, 5.0), Vec::<u64>::new());
        assert_eq!(lifetime_usd_milestones_crossed(9.99, 10.01), vec![10]);
        assert_eq!(
            lifetime_usd_milestones_crossed(5.0, 120.0),
            vec![10, 50, 100]
        );
        assert_eq!(
            lifetime_usd_milestones_crossed(200.0, 205.0),
            Vec::<u64>::new()
        );
        assert_eq!(
            lifetime_usd_milestones_crossed(199.5, 301.0),
            vec![200, 300]
        );
    }

    #[test]
    fn savings_tracker_queues_pending_usd_milestones_on_observe() {
        let mut tracker = make_tracker();
        tracker.lifetime_estimated_savings_usd = 7.5;
        let stats = HeadroomDashboardStats {
            output_reduction: None,
            session_requests: Some(1),
            session_estimated_savings_usd: Some(60.0),
            session_estimated_tokens_saved: Some(1),
            session_savings_pct: Some(1.0),
            session_actual_cost_usd: Some(0.0),
            session_total_tokens_sent: Some(0),
            savings_history: Vec::new(),
        };
        tracker.observe(&stats);
        let milestones = tracker.take_pending_lifetime_usd_milestones();
        assert_eq!(milestones, vec![10, 50]);
    }

    #[test]
    fn savings_tracker_appends_measured_headroom_attribution_events() {
        let mut tracker = make_tracker();
        let stats = HeadroomDashboardStats {
            output_reduction: None,
            session_requests: Some(3),
            session_estimated_savings_usd: Some(1.25),
            session_estimated_tokens_saved: Some(2500),
            session_savings_pct: Some(25.0),
            session_actual_cost_usd: Some(2.0),
            session_total_tokens_sent: Some(7500),
            savings_history: Vec::new(),
        };

        tracker.observe(&stats);
        let events = tracker.attribution_events();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.schema_version, 1);
        assert_eq!(event.scope, SavingsAttributionScope::Session);
        assert_eq!(event.source, SavingsAttributionSource::HeadroomEngine);
        assert_eq!(event.confidence, SavingsAttributionConfidence::Measured);
        assert_eq!(event.delta_tokens_saved, 2500);
        assert!((event.delta_usd - 1.25).abs() < 1e-9);
        assert_eq!(event.total_tokens_sent, 7500);
        assert_eq!(event.request_delta, 3);
        assert!(event.evidence.join(" ").contains("Headroom /stats"));
        assert!(event.evidence.join(" ").contains("Ponytail"));
        let _ = std::fs::remove_file(&tracker.records_path);
        let _ = std::fs::remove_file(&tracker.attribution_events_path);
        let _ = std::fs::remove_file(&tracker.state_path);
    }

    #[test]
    fn savings_tracker_appends_measured_rtk_attribution_events_from_deltas() {
        let mut tracker = make_tracker();

        tracker.observe_rtk_gain_summary(&RtkGainSummary {
            total_commands: 10,
            total_input: 1_200,
            total_output: 200,
            total_saved: 1000,
            avg_savings_pct: 70.0,
            total_time_ms: 400,
            avg_time_ms: Some(40),
        });
        assert!(
            tracker.attribution_events().is_empty(),
            "first RTK observation establishes the baseline"
        );

        tracker.observe_rtk_gain_summary(&RtkGainSummary {
            total_commands: 13,
            total_input: 1_800,
            total_output: 350,
            total_saved: 1450,
            avg_savings_pct: 72.0,
            total_time_ms: 550,
            avg_time_ms: Some(42),
        });

        let events = tracker.attribution_events();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.source, SavingsAttributionSource::Rtk);
        assert_eq!(event.confidence, SavingsAttributionConfidence::Measured);
        assert_eq!(event.delta_tokens_saved, 450);
        assert_eq!(event.delta_usd, 0.0);
        assert_eq!(event.request_delta, 3);
        assert!(event.evidence.join(" ").contains("RTK gain counter"));
        assert!(event.evidence.join(" ").contains("local command-output"));
        assert!(event
            .evidence
            .join(" ")
            .contains("600 input tokens, 150 output tokens, and 450 saved tokens"));
        assert!(event.evidence.join(" ").contains("150ms processing time"));
        let _ = std::fs::remove_file(&tracker.records_path);
        let _ = std::fs::remove_file(&tracker.attribution_events_path);
        let _ = std::fs::remove_file(&tracker.state_path);
    }

    #[test]
    fn savings_attribution_events_ignore_malformed_jsonl_lines() {
        let tracker = make_tracker();
        let event = SavingsAttributionEvent {
            schema_version: 1,
            id: "event-1".into(),
            observed_at: Utc::now(),
            scope: SavingsAttributionScope::Session,
            source: SavingsAttributionSource::HeadroomEngine,
            confidence: SavingsAttributionConfidence::Measured,
            delta_tokens_saved: 42,
            delta_usd: 0.21,
            total_tokens_sent: 100,
            request_delta: 1,
            evidence: vec!["Measured from test fixture.".into()],
        };
        std::fs::write(
            &tracker.attribution_events_path,
            format!(
                "not-json\n{}\n",
                serde_json::to_string(&event).expect("serialize attribution event")
            ),
        )
        .expect("write attribution file");

        let events = tracker.attribution_events();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "event-1");
        assert_eq!(events[0].delta_tokens_saved, 42);
        let _ = std::fs::remove_file(&tracker.attribution_events_path);
    }

    #[test]
    fn savings_attribution_event_schema_accepts_source_specific_future_events() {
        let text = r#"{
            "schemaVersion": 1,
            "id": "repo-event-1",
            "observedAt": "2026-06-28T10:00:00Z",
            "scope": "session",
            "source": "repo_intelligence",
            "confidence": "estimated",
            "deltaTokensSaved": 1200,
            "deltaUsd": 0.0,
            "totalTokensSent": 0,
            "requestDelta": 1,
            "evidence": ["Estimated from a Repo Intelligence pack before/after token delta."]
        }"#;

        let event: SavingsAttributionEvent =
            serde_json::from_str(text).expect("source-specific event decodes");

        assert_eq!(event.source, SavingsAttributionSource::RepoIntelligence);
        assert_eq!(event.confidence, SavingsAttributionConfidence::Estimated);
        assert_eq!(event.delta_tokens_saved, 1200);

        let compact_chinese = serde_json::json!({
            "schemaVersion": 1,
            "id": "compact-chinese-event-1",
            "observedAt": "2026-06-28T10:01:00Z",
            "scope": "session",
            "source": "compact_chinese",
            "confidence": "inferred",
            "deltaTokensSaved": 300,
            "deltaUsd": 0.0,
            "totalTokensSent": 0,
            "requestDelta": 1,
            "evidence": ["Inferred from Compact Chinese private handoff profile."]
        });
        let event: SavingsAttributionEvent =
            serde_json::from_value(compact_chinese).expect("compact chinese event decodes");

        assert_eq!(event.source, SavingsAttributionSource::CompactChinese);
        assert_eq!(event.confidence, SavingsAttributionConfidence::Inferred);
    }

    fn repo_summary_for_attribution(
        full_scan: u64,
        packs: Vec<RepoContextPack>,
    ) -> RepoIntelligenceSummary {
        RepoIntelligenceSummary {
            indexed_at: "2026-06-28T10:00:00Z".into(),
            repo_root: "/tmp/example".into(),
            indexer_version: Some("test".into()),
            total_files: 3,
            indexed_files: 3,
            skipped_files: 0,
            estimated_full_scan_tokens: full_scan,
            role_counts: Default::default(),
            index_metadata: None,
            graph: None,
            packs,
        }
    }

    fn repo_pack_for_attribution(
        title: &str,
        estimated_tokens: u64,
        savings: f64,
    ) -> RepoContextPack {
        RepoContextPack {
            id: title.to_ascii_lowercase().replace(' ', "_"),
            title: title.into(),
            purpose: "test pack".into(),
            files: Vec::new(),
            estimated_tokens,
            savings_vs_full_scan_pct: savings,
        }
    }

    #[test]
    fn repo_intelligence_attribution_event_uses_best_pack_delta() {
        let summary = repo_summary_for_attribution(
            10_000,
            vec![
                repo_pack_for_attribution("Verification", 4_000, 60.0),
                repo_pack_for_attribution("Implementation", 2_500, 75.0),
            ],
        );

        let event = super::build_repo_intelligence_attribution_event(&summary)
            .expect("repo intelligence attribution event");

        assert_eq!(event.source, SavingsAttributionSource::RepoIntelligence);
        assert_eq!(event.confidence, SavingsAttributionConfidence::Estimated);
        assert_eq!(event.delta_tokens_saved, 7_500);
        assert_eq!(event.delta_usd, 0.0);
        assert_eq!(event.request_delta, 1);
        let evidence = event.evidence.join(" ");
        assert!(evidence.contains("full scan 10000 tokens"));
        assert!(evidence.contains("Implementation"));
        assert!(evidence.contains("not provider-spend dollars"));
    }

    #[test]
    fn repo_intelligence_attribution_event_skips_zero_delta() {
        let summary = repo_summary_for_attribution(
            1_000,
            vec![repo_pack_for_attribution("Full", 1_000, 0.0)],
        );

        assert!(super::build_repo_intelligence_attribution_event(&summary).is_none());
    }

    #[test]
    fn addon_attribution_event_records_estimated_markitdown_delta() {
        let changed_files = vec![
            "/tmp/headroom-markitdown-read.sh".to_string(),
            "/tmp/CLAUDE.md".to_string(),
        ];
        let backup_files = vec!["/tmp/CLAUDE.md.bak".to_string()];
        let event = super::build_addon_attribution_event(
            "markitdown",
            None,
            Some(&changed_files),
            Some(&backup_files),
            None,
        )
        .expect("markitdown attribution");

        assert_eq!(event.source, SavingsAttributionSource::Markitdown);
        assert_eq!(event.confidence, SavingsAttributionConfidence::Estimated);
        assert_eq!(event.delta_tokens_saved, 2_300);
        assert_eq!(event.request_delta, 2);
        let evidence = event.evidence.join(" ");
        assert!(evidence.contains("baseline 3200 tokens"));
        assert!(evidence.contains("optimized 900 tokens"));
        assert!(evidence.contains("smoke-tested"));
        assert!(evidence.contains("changed 2 client artifacts"));
        assert!(evidence.contains("headroom-markitdown-read.sh"));
        assert!(evidence.contains("Backups created"));
        assert!(evidence.contains("not provider-spend dollars"));
    }

    #[test]
    fn addon_attribution_event_skips_markitdown_without_changed_artifacts() {
        assert!(
            super::build_addon_attribution_event("markitdown", None, None, None, None).is_none()
        );
        assert!(
            super::build_addon_attribution_event("markitdown", None, Some(&[]), None, None)
                .is_none()
        );
    }

    #[test]
    fn addon_attribution_event_separates_compact_chinese_from_caveman() {
        let changed_files = vec!["/tmp/CLAUDE.md".to_string(), "/tmp/AGENTS.md".to_string()];
        let backup_files = vec!["/tmp/CLAUDE.md.bak".to_string()];
        let caveman = super::build_addon_attribution_event(
            "caveman",
            Some("scoped"),
            Some(&changed_files),
            Some(&backup_files),
            None,
        )
        .expect("caveman attribution");
        let compact = super::build_addon_attribution_event(
            "caveman",
            Some(crate::tool_manager::CAVEMAN_LEVEL_COMPACT_CHINESE),
            Some(&changed_files),
            Some(&backup_files),
            None,
        )
        .expect("compact chinese attribution");

        assert_eq!(caveman.source, SavingsAttributionSource::Caveman);
        assert_eq!(compact.source, SavingsAttributionSource::CompactChinese);
        assert_eq!(caveman.confidence, SavingsAttributionConfidence::Estimated);
        assert_eq!(compact.confidence, SavingsAttributionConfidence::Estimated);
        assert_eq!(caveman.delta_tokens_saved, 300);
        assert_eq!(caveman.request_delta, 2);
        assert_eq!(compact.delta_tokens_saved, 300);
        assert_eq!(compact.request_delta, 2);
        let evidence = caveman.evidence.join(" ");
        assert!(evidence.contains("changed 2 client instruction files"));
        assert!(evidence.contains("/tmp/CLAUDE.md"));
        assert!(evidence.contains("Backups created"));
    }

    #[test]
    fn addon_attribution_event_skips_caveman_without_changed_files() {
        assert!(super::build_addon_attribution_event(
            "caveman",
            Some("scoped"),
            Some(&[]),
            None,
            None,
        )
        .is_none());
    }

    #[test]
    fn savings_attribution_counters_group_addon_sources() {
        let now = Utc::now();
        let events = vec![
            SavingsAttributionEvent {
                schema_version: 1,
                id: "evt-1".to_string(),
                observed_at: now,
                scope: SavingsAttributionScope::Session,
                source: SavingsAttributionSource::Caveman,
                confidence: SavingsAttributionConfidence::Measured,
                delta_tokens_saved: 120,
                delta_usd: 0.0,
                total_tokens_sent: 1_000,
                request_delta: 1,
                evidence: vec!["measured caveman delta".to_string()],
            },
            SavingsAttributionEvent {
                schema_version: 1,
                id: "evt-2".to_string(),
                observed_at: now,
                scope: SavingsAttributionScope::Session,
                source: SavingsAttributionSource::Caveman,
                confidence: SavingsAttributionConfidence::Estimated,
                delta_tokens_saved: 80,
                delta_usd: 0.0,
                total_tokens_sent: 500,
                request_delta: 1,
                evidence: vec!["estimated caveman delta".to_string()],
            },
        ];

        let counters = super::aggregate_savings_attribution_counters(&events);
        assert_eq!(counters.len(), 1);
        assert_eq!(counters[0].source, SavingsAttributionSource::Caveman);
        assert_eq!(counters[0].event_count, 2);
        assert_eq!(counters[0].runtime_event_count, 2);
        assert_eq!(counters[0].measured_event_count, 1);
        assert_eq!(counters[0].estimated_event_count, 1);
        assert_eq!(counters[0].inferred_event_count, 0);
        assert_eq!(counters[0].delta_tokens_saved, 200);
        assert_eq!(counters[0].total_tokens_sent, 1_500);
    }

    #[test]
    fn addon_attribution_event_records_estimated_ponytail_host_registration() {
        let hosts = vec!["Claude Code".to_string(), "Codex".to_string()];
        let event =
            super::build_addon_attribution_event("ponytail", None, None, None, Some(&hosts))
                .expect("ponytail attribution");

        assert_eq!(event.source, SavingsAttributionSource::Ponytail);
        assert_eq!(event.confidence, SavingsAttributionConfidence::Estimated);
        assert_eq!(event.delta_tokens_saved, 880);
        assert_eq!(event.request_delta, 2);
        let evidence = event.evidence.join(" ");
        assert!(evidence.contains("registered with 2 agent hosts"));
        assert!(evidence.contains("Claude Code"));
        assert!(evidence.contains("Codex"));
    }

    #[test]
    fn addon_attribution_event_skips_ponytail_without_registered_hosts() {
        assert!(
            super::build_addon_attribution_event("ponytail", None, None, None, Some(&[]),)
                .is_none()
        );
    }

    #[test]
    fn aggregate_weekly_totals_sums_active_days_in_window() {
        use std::collections::BTreeMap;
        let mut daily: BTreeMap<String, DailySavingsBucket> = BTreeMap::new();
        daily.insert(
            "2026-04-19".into(), // outside window (Sunday of week before)
            DailySavingsBucket {
                estimated_savings_usd: 1.0,
                estimated_tokens_saved: 50,
                actual_cost_usd: 0.0,
                total_tokens_sent: 0,
            },
        );
        daily.insert(
            "2026-04-20".into(),
            DailySavingsBucket {
                estimated_savings_usd: 2.5,
                estimated_tokens_saved: 200,
                actual_cost_usd: 0.0,
                total_tokens_sent: 0,
            },
        );
        daily.insert(
            "2026-04-23".into(),
            DailySavingsBucket {
                estimated_savings_usd: 1.0,
                estimated_tokens_saved: 100,
                actual_cost_usd: 0.0,
                total_tokens_sent: 0,
            },
        );
        daily.insert(
            "2026-04-26".into(),
            DailySavingsBucket {
                estimated_savings_usd: 0.0,
                estimated_tokens_saved: 0, // zero activity day — not counted
                actual_cost_usd: 0.0,
                total_tokens_sent: 0,
            },
        );
        daily.insert(
            "2026-04-27".into(), // outside window (today Monday)
            DailySavingsBucket {
                estimated_savings_usd: 99.0,
                estimated_tokens_saved: 9999,
                actual_cost_usd: 0.0,
                total_tokens_sent: 0,
            },
        );
        let start = chrono::NaiveDate::from_ymd_opt(2026, 4, 20).unwrap();
        let end = chrono::NaiveDate::from_ymd_opt(2026, 4, 26).unwrap();
        let totals = aggregate_weekly_totals(&daily, start, end);
        assert_eq!(totals.active_days, 2);
        assert_eq!(totals.total_tokens_saved, 300);
        assert!((totals.total_savings_usd - 3.5).abs() < 1e-9);
    }

    #[test]
    fn most_recent_monday_maps_every_weekday_to_this_weeks_monday() {
        use chrono::NaiveDate;
        // Monday 2026-04-27 — itself.
        assert_eq!(
            most_recent_monday(NaiveDate::from_ymd_opt(2026, 4, 27).unwrap()),
            NaiveDate::from_ymd_opt(2026, 4, 27).unwrap()
        );
        // Wednesday 2026-04-29 — back to Monday 27.
        assert_eq!(
            most_recent_monday(NaiveDate::from_ymd_opt(2026, 4, 29).unwrap()),
            NaiveDate::from_ymd_opt(2026, 4, 27).unwrap()
        );
        // Sunday 2026-05-03 — back to Monday 27 (6 days back).
        assert_eq!(
            most_recent_monday(NaiveDate::from_ymd_opt(2026, 5, 3).unwrap()),
            NaiveDate::from_ymd_opt(2026, 4, 27).unwrap()
        );
    }

    #[test]
    fn observe_activity_separates_fresh_from_recent_across_calls() {
        use crate::models::TransformationFeedEvent;
        let base_dir = temp_test_dir("headroom-activity-observation");
        let state = AppState::new_in(base_dir.clone()).expect("app state");

        let transformation = TransformationFeedEvent {
            request_id: Some("r1".into()),
            timestamp: Some("2026-04-22T10:00:00Z".into()),
            provider: Some("anthropic".into()),
            model: Some("claude-opus-4-7".into()),
            input_tokens_original: Some(10_000),
            input_tokens_optimized: Some(2_000),
            tokens_saved: Some(8_000),
            savings_percent: Some(80.0),
            transforms_applied: vec!["kompress".into()],
            workspace: Some("/Users/u/Code/demo".into()),
            turn_id: None,
            request_messages: None,
            compressed_messages: None,
        };

        let first = state.observe_activity_from_transformations(&[transformation.clone()]);
        assert!(
            !first.fresh.is_empty(),
            "first observation should emit fresh events"
        );
        // First compression that beats the zero baseline emits a Daily+AllTime
        // Record.
        assert!(
            first
                .fresh
                .iter()
                .any(|e| matches!(e, ActivityEvent::Record(_))),
            "first record should fire"
        );
        // Snapshot after the first observation has the record slot populated.
        let first_snapshot = state.activity_feed_snapshot();
        assert!(first_snapshot.record.is_some());
        assert!(first_snapshot.transformation.is_some());

        let second = state.observe_activity_from_transformations(&[transformation]);
        assert!(
            second.fresh.is_empty(),
            "second observation of same transformation should emit no fresh events"
        );
        // Snapshot still carries the slots across the no-op second call.
        let second_snapshot = state.activity_feed_snapshot();
        assert!(second_snapshot.record.is_some());
        assert!(second_snapshot.transformation.is_some());

        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn dashboard_includes_managed_tools() {
        let base_dir = temp_test_dir("headroom-app-state");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        let dashboard = state.dashboard();

        assert!(dashboard.tools.iter().any(|tool| tool.id == "headroom"));
        assert!(dashboard.tools.iter().any(|tool| tool.id == "rtk"));
        assert!(dashboard
            .insights
            .iter()
            .any(|insight| !insight.title.is_empty()));

        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn proxy_bypass_initialises_to_false() {
        let base_dir = temp_test_dir("headroom-bypass-init");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        assert!(
            !state
                .proxy_bypass
                .load(std::sync::atomic::Ordering::Acquire),
            "fresh AppState must default to bypass=off so the intercept routes through the Python proxy"
        );
        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    fn pricing_status_with_optimization(allowed: bool) -> crate::models::HeadroomPricingStatus {
        use crate::models::{
            ClaudeAccountProfile, ClaudeAuthMethod, ClaudePlanTier, HeadroomPricingStatus,
        };
        let now = chrono::Utc::now();
        HeadroomPricingStatus {
            authenticated: true,
            local_grace_started_at: now,
            local_grace_ends_at: now,
            local_grace_active: false,
            account_sync_error: None,
            needs_authentication: false,
            optimization_allowed: allowed,
            should_nudge: false,
            nudge_level: 0,
            gate_reason: None,
            gate_message: String::new(),
            nudge_threshold_percent: None,
            effective_nudge_thresholds_percent: None,
            disable_threshold_percent: None,
            effective_disable_threshold_percent: None,
            recommended_subscription_tier: None,
            tier_mismatch: None,
            claude: ClaudeAccountProfile {
                auth_method: ClaudeAuthMethod::Unknown,
                email: None,
                display_name: None,
                account_uuid: None,
                organization_uuid: None,
                billing_type: None,
                account_created_at: None,
                subscription_created_at: None,
                has_extra_usage_enabled: false,
                plan_tier: ClaudePlanTier::Unknown,
                plan_detection_source: None,
                organization_type: None,
                rate_limit_tier: None,
                weekly_utilization_pct: None,
                five_hour_utilization_pct: None,
                extra_usage_monthly_limit: None,
                profile_fetch_error: None,
            },
            codex: None,
            account: None,
            launch_discount_active: false,
            active_percent_off: 0,
            pricing_cohorts: Vec::new(),
        }
    }

    #[test]
    fn apply_pricing_gate_status_flips_bypass_on_for_gated_status() {
        let base_dir = temp_test_dir("headroom-bypass-on");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        assert!(!state
            .proxy_bypass
            .load(std::sync::atomic::Ordering::Acquire));

        // Debounce: first gated reading just bumps the streak.
        state.apply_pricing_gate_status(&pricing_status_with_optimization(false));
        assert!(
            !state
                .proxy_bypass
                .load(std::sync::atomic::Ordering::Acquire),
            "first gated reading must not flip bypass yet"
        );

        // Second consecutive gated reading crosses the debounce threshold.
        state.apply_pricing_gate_status(&pricing_status_with_optimization(false));
        assert!(
            state
                .proxy_bypass
                .load(std::sync::atomic::Ordering::Acquire),
            "second consecutive gated reading must flip bypass=true"
        );
        fs::remove_dir_all(base_dir).ok();
    }

    fn codex_usage_with_optimization(allowed: bool) -> crate::models::CodexUsage {
        crate::models::CodexUsage {
            limit_name: None,
            primary: None,
            secondary: None,
            credits_balance: None,
            credits_unlimited: false,
            optimization_allowed: allowed,
            should_nudge: false,
            nudge_level: 0,
            gate_reason: None,
            recommended_subscription_tier: None,
            weekly_used_percent: None,
            gate_message: String::new(),
        }
    }

    #[test]
    fn apply_codex_gate_flips_codex_bypass_without_stopping_backend() {
        let base_dir = temp_test_dir("headroom-codex-bypass");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        assert!(!state
            .codex_bypass
            .load(std::sync::atomic::Ordering::Acquire));
        assert!(
            !state
                .proxy_bypass
                .load(std::sync::atomic::Ordering::Acquire),
            "Claude bypass must stay untouched by the Codex gate"
        );

        // Debounce: first gated reading just bumps the streak.
        state.apply_codex_pricing_gate_status(Some(&codex_usage_with_optimization(false)));
        assert!(!state
            .codex_bypass
            .load(std::sync::atomic::Ordering::Acquire));

        // Second consecutive gated reading crosses the debounce threshold.
        state.apply_codex_pricing_gate_status(Some(&codex_usage_with_optimization(false)));
        assert!(state
            .codex_bypass
            .load(std::sync::atomic::Ordering::Acquire));
        // Crucially the Claude-wide bypass never flipped, so Claude stays optimized.
        assert!(!state
            .proxy_bypass
            .load(std::sync::atomic::Ordering::Acquire));

        // An ungated reading clears the Codex bypass again.
        state.apply_codex_pricing_gate_status(Some(&codex_usage_with_optimization(true)));
        assert!(!state
            .codex_bypass
            .load(std::sync::atomic::Ordering::Acquire));

        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn apply_codex_gate_ignores_absent_usage() {
        let base_dir = temp_test_dir("headroom-codex-bypass-none");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        // Flip it on first.
        state.apply_codex_pricing_gate_status(Some(&codex_usage_with_optimization(false)));
        state.apply_codex_pricing_gate_status(Some(&codex_usage_with_optimization(false)));
        assert!(state
            .codex_bypass
            .load(std::sync::atomic::Ordering::Acquire));
        // A poll with no Codex signal must leave the gate as-is, not clear it.
        state.apply_codex_pricing_gate_status(None);
        assert!(state
            .codex_bypass
            .load(std::sync::atomic::Ordering::Acquire));
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn apply_pricing_gate_status_resets_streak_on_ungated_reading() {
        let base_dir = temp_test_dir("headroom-bypass-debounce-reset");
        let state = AppState::new_in(base_dir.clone()).expect("app state");

        // One gated reading bumps the streak to 1.
        state.apply_pricing_gate_status(&pricing_status_with_optimization(false));
        assert!(!state
            .proxy_bypass
            .load(std::sync::atomic::Ordering::Acquire));

        // Ungated reading resets the streak — a single-poll spike clears.
        state.apply_pricing_gate_status(&pricing_status_with_optimization(true));

        // Now another gated reading is the first of a new window, not the
        // second of the old one. Bypass must still be off.
        state.apply_pricing_gate_status(&pricing_status_with_optimization(false));
        assert!(
            !state
                .proxy_bypass
                .load(std::sync::atomic::Ordering::Acquire),
            "an intervening ungated reading must reset the debounce streak"
        );
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn apply_pricing_gate_status_clears_bypass_for_ungated_status() {
        let base_dir = temp_test_dir("headroom-bypass-off");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        // Pre-set the flag, simulating that the gate fired earlier.
        state
            .proxy_bypass
            .store(true, std::sync::atomic::Ordering::Release);

        state.apply_pricing_gate_status(&pricing_status_with_optimization(true));

        assert!(
            !state
                .proxy_bypass
                .load(std::sync::atomic::Ordering::Acquire),
            "ungated status must clear bypass — this is the upgrade-recovery path"
        );
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn apply_pricing_gate_status_is_idempotent_when_state_already_matches() {
        let base_dir = temp_test_dir("headroom-bypass-noop");
        let state = AppState::new_in(base_dir.clone()).expect("app state");

        // Already off + ungated status → still off (no transition triggered).
        state.apply_pricing_gate_status(&pricing_status_with_optimization(true));
        assert!(!state
            .proxy_bypass
            .load(std::sync::atomic::Ordering::Acquire));

        // Two consecutive gated readings cross the debounce threshold and flip.
        state.apply_pricing_gate_status(&pricing_status_with_optimization(false));
        state.apply_pricing_gate_status(&pricing_status_with_optimization(false));
        assert!(state
            .proxy_bypass
            .load(std::sync::atomic::Ordering::Acquire));

        // Already on + gated status → still on.
        state.apply_pricing_gate_status(&pricing_status_with_optimization(false));
        assert!(state
            .proxy_bypass
            .load(std::sync::atomic::Ordering::Acquire));

        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn last_known_good_plan_returns_none_on_fresh_install() {
        let base_dir = temp_test_dir("headroom-last-known-good-fresh");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        assert!(state.last_known_good_plan_tier().is_none());
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn record_known_good_plan_tier_skips_unknown() {
        use crate::models::ClaudePlanTier;
        let base_dir = temp_test_dir("headroom-last-known-good-skip-unknown");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        state.record_known_good_plan_tier(&ClaudePlanTier::Pro);
        state.record_known_good_plan_tier(&ClaudePlanTier::Unknown);
        assert!(matches!(
            state.last_known_good_plan_tier(),
            Some(ClaudePlanTier::Pro)
        ));
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn last_known_good_plan_persists_across_appstate_reload() {
        use crate::models::ClaudePlanTier;
        let base_dir = temp_test_dir("headroom-last-known-good-persist");
        {
            let state = AppState::new_in(base_dir.clone()).expect("app state");
            state.record_known_good_plan_tier(&ClaudePlanTier::Max5x);
        }
        let reloaded = AppState::new_in(base_dir.clone()).expect("reloaded app state");
        assert!(matches!(
            reloaded.last_known_good_plan_tier(),
            Some(ClaudePlanTier::Max5x)
        ));
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn record_known_good_plan_tier_overwrites_with_newer_known_tier() {
        use crate::models::ClaudePlanTier;
        let base_dir = temp_test_dir("headroom-last-known-good-overwrite");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        state.record_known_good_plan_tier(&ClaudePlanTier::Pro);
        state.record_known_good_plan_tier(&ClaudePlanTier::Max20x);
        assert!(matches!(
            state.last_known_good_plan_tier(),
            Some(ClaudePlanTier::Max20x)
        ));
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn runtime_maintenance_plan_prefers_requirements_repair_when_only_lock_is_stale() {
        let base_dir = temp_test_dir("headroom-maintenance-repair");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        write_headroom_receipt(
            &base_dir,
            crate::tool_manager::HEADROOM_PINNED_VERSION,
            "stale",
        );

        let plan = state.runtime_maintenance_plan_for_app_version(env!("CARGO_PKG_VERSION"));
        assert!(matches!(
            plan,
            Some(super::RuntimeMaintenancePlan::RequirementsRepair)
        ));

        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn runtime_maintenance_plan_prefers_upgrade_over_requirements_repair() {
        let base_dir = temp_test_dir("headroom-maintenance-upgrade");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        write_headroom_receipt(&base_dir, "0.6.5", "stale");

        let plan = state.runtime_maintenance_plan_for_app_version(env!("CARGO_PKG_VERSION"));
        match plan {
            Some(super::RuntimeMaintenancePlan::Upgrade(release)) => {
                assert_eq!(
                    release.version(),
                    crate::tool_manager::HEADROOM_PINNED_VERSION
                );
            }
            _ => panic!("expected version upgrade plan"),
        }

        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn runtime_maintenance_plan_skips_when_current_app_version_already_succeeded() {
        let base_dir = temp_test_dir("headroom-maintenance-stamped");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        write_headroom_receipt(&base_dir, "0.9.7", "stale");
        state.stamp_app_version(env!("CARGO_PKG_VERSION"));

        let plan = state.runtime_maintenance_plan_for_app_version(env!("CARGO_PKG_VERSION"));
        assert!(plan.is_none());

        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn runtime_maintenance_plan_skips_when_retry_budget_is_exhausted() {
        let base_dir = temp_test_dir("headroom-maintenance-budget");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        write_headroom_receipt(&base_dir, "0.6.5", "stale");

        for _ in 0..super::MAX_UPGRADE_AUTO_RETRIES {
            state.record_upgrade_failure(RuntimeUpgradeFailure {
                app_version: env!("CARGO_PKG_VERSION").into(),
                target_headroom_version: "0.8.2".into(),
                fallback_headroom_version: Some("0.6.5".into()),
                failure_phase: UpgradeFailurePhase::Install,
                attempts: 0,
                first_attempt_at: Utc::now(),
                last_attempt_at: Utc::now(),
                error_message: "failed".into(),
                error_hint: None,
                rollback_restored: true,
            });
        }

        let plan = state.runtime_maintenance_plan_for_app_version(env!("CARGO_PKG_VERSION"));
        assert!(plan.is_none());

        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn can_stamp_no_maintenance_allows_stamp_when_version_changed_with_no_failure() {
        let base_dir = temp_test_dir("can-stamp-fresh");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        // Stamp set to an older app version, no failure record.
        state.stamp_app_version("0.3.6-rc.3");
        assert!(state.can_stamp_no_maintenance("0.3.12-rc.3"));
        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn can_stamp_no_maintenance_skips_stamp_when_already_current() {
        let base_dir = temp_test_dir("can-stamp-idempotent");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        state.stamp_app_version("0.3.12-rc.3");
        assert!(!state.can_stamp_no_maintenance("0.3.12-rc.3"));
        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn can_stamp_no_maintenance_skips_stamp_when_failure_recorded_for_current_version() {
        let base_dir = temp_test_dir("can-stamp-with-failure");
        let state = AppState::new_in(base_dir.clone()).expect("app state");
        state.stamp_app_version("0.3.6-rc.3");
        state.record_upgrade_failure(RuntimeUpgradeFailure {
            app_version: "0.3.12-rc.3".into(),
            target_headroom_version: "0.20.15".into(),
            fallback_headroom_version: Some("0.19.0".into()),
            failure_phase: UpgradeFailurePhase::BootValidation,
            attempts: 0,
            first_attempt_at: Utc::now(),
            last_attempt_at: Utc::now(),
            error_message: "failed".into(),
            error_hint: None,
            rollback_restored: true,
        });
        assert!(!state.can_stamp_no_maintenance("0.3.12-rc.3"));
        // Still allows stamping for an unrelated future version, since the
        // failure record is keyed on the specific version that failed.
        assert!(state.can_stamp_no_maintenance("0.3.13"));
        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn lifetime_token_milestones_include_firsts_and_repeating_tens() {
        assert_eq!(
            lifetime_token_milestones_crossed(0, 5_000_000),
            vec![100_000, 1_000_000, 5_000_000]
        );
        assert_eq!(
            lifetime_token_milestones_crossed(9_500_000, 21_000_000),
            vec![10_000_000, 20_000_000]
        );
        assert_eq!(lifetime_token_milestones_crossed(0, 150_000), vec![100_000]);
    }

    #[test]
    fn tracker_queues_new_lifetime_token_milestones_once() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(1),
                session_estimated_savings_usd: Some(1.0),
                session_estimated_tokens_saved: Some(12_000_000),
                session_savings_pct: Some(50.0),
                session_actual_cost_usd: Some(0.5),
                session_total_tokens_sent: Some(12_000_000),
                savings_history: Vec::new(),
            })
            .expect("snapshot");

        assert_eq!(
            tracker.take_pending_lifetime_token_milestones(),
            vec![100_000, 1_000_000, 5_000_000, 10_000_000]
        );
        assert!(tracker.take_pending_lifetime_token_milestones().is_empty());

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(2),
                session_estimated_savings_usd: Some(2.0),
                session_estimated_tokens_saved: Some(21_000_000),
                session_savings_pct: Some(50.0),
                session_actual_cost_usd: Some(1.0),
                session_total_tokens_sent: Some(21_000_000),
                savings_history: Vec::new(),
            })
            .expect("snapshot");

        assert_eq!(
            tracker.take_pending_lifetime_token_milestones(),
            vec![20_000_000]
        );
    }

    #[test]
    fn dashboard_read_path_preserves_pending_milestones_for_analytics() {
        // Regression guard: `state.dashboard()` (tray updater, bootstrap
        // finalize, account activation) must not drain pending milestones.
        // Only `dashboard_with_pending_milestones()` — the path that actually
        // fires the aptabase event, pricing report, and in-app notification —
        // may consume them. A prior refactor drained on every call, so the
        // tray updater's 5s heartbeat silently ate ~50-100% of crossings.
        let base_dir = temp_test_dir("headroom-milestone-preservation");
        let state = AppState::new_in(base_dir.clone()).expect("app state");

        let stats = HeadroomDashboardStats {
            output_reduction: None,
            session_requests: Some(1),
            session_estimated_savings_usd: Some(1.0),
            session_estimated_tokens_saved: Some(1_500_000),
            session_savings_pct: Some(50.0),
            session_actual_cost_usd: Some(0.5),
            session_total_tokens_sent: Some(1_500_000),
            savings_history: Vec::new(),
        };

        let (_, _, _, read_only) = state
            .record_savings_snapshot(&stats, false)
            .expect("snapshot");
        assert!(
            read_only.token.is_empty(),
            "read-only path must not surface milestones"
        );

        let (_, _, _, drained) = state
            .record_savings_snapshot(&stats, true)
            .expect("snapshot");
        assert_eq!(
            drained.token,
            vec![100_000, 1_000_000],
            "drain=true must surface milestones queued by the earlier read-only observe"
        );

        let (_, _, _, drained_again) = state
            .record_savings_snapshot(&stats, true)
            .expect("snapshot");
        assert!(
            drained_again.token.is_empty(),
            "second drain finds nothing: milestones fire exactly once"
        );

        fs::remove_dir_all(base_dir).expect("remove temp dir");
    }

    #[test]
    fn session_counters_follow_headroom_stats() {
        let mut tracker = make_tracker();

        let first = tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(10),
                session_estimated_savings_usd: Some(1.2),
                session_estimated_tokens_saved: Some(1_200),
                session_savings_pct: Some(24.0),
                session_actual_cost_usd: Some(3.8),
                session_total_tokens_sent: Some(3_800),
                savings_history: Vec::new(),
            })
            .expect("first snapshot");
        assert_eq!(first.session_requests, 10);
        assert_eq!(first.session_estimated_tokens_saved, 1_200);
        assert!((first.session_estimated_savings_usd - 1.2).abs() < 1e-9);
        assert!((first.session_savings_pct - 24.0).abs() < 1e-9);
        assert_eq!(first.lifetime_requests, 10);
        assert_eq!(first.lifetime_estimated_tokens_saved, 1_200);
        assert!((first.lifetime_estimated_savings_usd - 1.2).abs() < 1e-9);

        let second = tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(12),
                session_estimated_savings_usd: Some(1.5),
                session_estimated_tokens_saved: Some(1_500),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(4.5),
                session_total_tokens_sent: Some(4_500),
                savings_history: Vec::new(),
            })
            .expect("second snapshot");
        assert_eq!(second.session_requests, 12);
        assert_eq!(second.session_estimated_tokens_saved, 1_500);
        assert!((second.session_estimated_savings_usd - 1.5).abs() < 1e-9);
        assert_eq!(second.lifetime_requests, 12);
        assert_eq!(second.lifetime_estimated_tokens_saved, 1_500);
        assert!((second.lifetime_estimated_savings_usd - 1.5).abs() < 1e-9);
    }

    #[test]
    fn new_session_resets_live_session_and_keeps_lifetime() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(10),
                session_estimated_savings_usd: Some(1.0),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(20.0),
                session_actual_cost_usd: Some(4.0),
                session_total_tokens_sent: Some(4_000),
                savings_history: Vec::new(),
            })
            .expect("initial session");

        let reset = tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(2),
                session_estimated_savings_usd: Some(0.2),
                session_estimated_tokens_saved: Some(200),
                session_savings_pct: Some(18.0),
                session_actual_cost_usd: Some(0.9),
                session_total_tokens_sent: Some(900),
                savings_history: Vec::new(),
            })
            .expect("reset snapshot");
        assert_eq!(reset.session_requests, 2);
        assert_eq!(reset.session_estimated_tokens_saved, 200);
        assert!((reset.session_estimated_savings_usd - 0.2).abs() < 1e-9);
        assert_eq!(reset.lifetime_requests, 12);
        assert_eq!(reset.lifetime_estimated_tokens_saved, 1_200);
        assert!((reset.lifetime_estimated_savings_usd - 1.2).abs() < 1e-9);
    }

    #[test]
    fn first_observation_backfills_daily_history_from_headroom() {
        let mut tracker = make_tracker();
        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(4),
                session_estimated_savings_usd: Some(0.5),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.3),
                session_total_tokens_sent: Some(3_000),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 11, 0),
                    history_point_at(2026, 3, 20, 12, 400),
                    history_point_at(2026, 3, 21, 12, 1_000),
                ],
            })
            .expect("snapshot");

        let daily = tracker.daily_savings();
        let expected_days = [
            Utc.with_ymd_and_hms(2026, 3, 20, 12, 0, 0)
                .single()
                .expect("day one")
                .with_timezone(&Local)
                .format("%Y-%m-%d")
                .to_string(),
            Utc.with_ymd_and_hms(2026, 3, 21, 12, 0, 0)
                .single()
                .expect("day two")
                .with_timezone(&Local)
                .format("%Y-%m-%d")
                .to_string(),
        ];
        assert_eq!(daily.len(), 2);
        assert_eq!(daily[0].date, expected_days[0]);
        assert_eq!(daily[0].estimated_tokens_saved, 400);
        assert_eq!(daily[0].total_tokens_sent, 1_200);
        assert_eq!(daily[1].date, expected_days[1]);
        assert_eq!(daily[1].estimated_tokens_saved, 600);
        assert_eq!(daily[1].total_tokens_sent, 1_800);
        assert!(
            (daily[0].estimated_savings_usd + daily[1].estimated_savings_usd - 0.5).abs() < 1e-9
        );
        assert!((daily[0].actual_cost_usd - 0.12).abs() < 1e-9);
        assert!((daily[1].actual_cost_usd - 0.18).abs() < 1e-9);
    }

    #[test]
    fn first_observation_backfills_hourly_history_for_today() {
        let mut tracker = make_tracker();
        let today = Local::now().date_naive();

        // Pick three local-time hours today and convert to UTC components for
        // history_point_at. Feeding the local date directly into UTC builders
        // breaks in any TZ where local-hour-N maps to a different UTC date.
        let to_utc_components = |local_hour: u32| -> (i32, u32, u32, u32) {
            let utc = Local
                .with_ymd_and_hms(today.year(), today.month(), today.day(), local_hour, 0, 0)
                .single()
                .expect("unambiguous local time")
                .with_timezone(&Utc);
            (utc.year(), utc.month(), utc.day(), utc.hour())
        };
        let (y0, m0, d0, h0) = to_utc_components(8);
        let (y1, m1, d1, h1) = to_utc_components(9);
        let (y2, m2, d2, h2) = to_utc_components(15);
        let expected_first_hour = super::local_hour_key(
            Utc.with_ymd_and_hms(y1, m1, d1, h1, 0, 0)
                .single()
                .expect("valid timestamp")
                .with_timezone(&Local),
        );
        let expected_second_hour = super::local_hour_key(
            Utc.with_ymd_and_hms(y2, m2, d2, h2, 0, 0)
                .single()
                .expect("valid timestamp")
                .with_timezone(&Local),
        );

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(4),
                session_estimated_savings_usd: Some(0.5),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.3),
                session_total_tokens_sent: Some(3_000),
                savings_history: vec![
                    history_point_at(y0, m0, d0, h0, 0),
                    history_point_at(y1, m1, d1, h1, 400),
                    history_point_at(y2, m2, d2, h2, 1_000),
                ],
            })
            .expect("snapshot");

        let today_key = today.format("%Y-%m-%d").to_string();
        let hourly = tracker
            .hourly_savings()
            .into_iter()
            .filter(|point| point.hour.starts_with(&format!("{today_key}T")))
            .collect::<Vec<_>>();
        assert_eq!(hourly.len(), 2);
        assert_eq!(hourly[0].hour, expected_first_hour);
        assert_eq!(hourly[0].estimated_tokens_saved, 400);
        assert_eq!(hourly[1].hour, expected_second_hour);
        assert_eq!(hourly[1].estimated_tokens_saved, 600);
        assert_eq!(hourly[0].total_tokens_sent, 1_200);
        assert_eq!(hourly[1].total_tokens_sent, 1_800);
    }

    #[test]
    fn claude_project_scan_dedupes_repeated_session_files() {
        let test_dir = temp_test_dir("headroom-project-scan");
        fs::create_dir_all(&test_dir).expect("create temp dir");
        let session_file = test_dir.join("session.jsonl");
        fs::write(&session_file, "{\"cwd\":\"/tmp/project\"}\n").expect("write session file");

        let mut scan = ClaudeProjectScan::default();
        scan.add_session_files(vec![session_file.clone(), session_file]);

        assert_eq!(scan.session_files.len(), 1);

        fs::remove_dir_all(&test_dir).expect("remove temp dir");
    }

    #[cfg(unix)]
    #[test]
    fn claude_project_scan_dedupes_symlinked_session_files() {
        use std::os::unix::fs::symlink;

        let test_dir = temp_test_dir("headroom-project-scan-symlink");
        fs::create_dir_all(&test_dir).expect("create temp dir");
        let real_dir = test_dir.join("real");
        let alias_dir = test_dir.join("alias");
        fs::create_dir_all(&real_dir).expect("create real dir");
        symlink(&real_dir, &alias_dir).expect("create alias symlink");

        let real_file = real_dir.join("session.jsonl");
        let alias_file = alias_dir.join("session.jsonl");
        fs::write(&real_file, "{\"cwd\":\"/tmp/project\"}\n").expect("write session file");

        let mut scan = ClaudeProjectScan::default();
        scan.add_session_files(vec![real_file, alias_file]);

        assert_eq!(scan.session_files.len(), 1);

        fs::remove_dir_all(&test_dir).expect("remove temp dir");
    }

    #[test]
    fn parse_headroom_stats_uses_compression_scoped_savings_fields() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "persistent_savings": {
                    "lifetime": {
                        "tokens_saved": 2400,
                        "compression_savings_usd": 0.84
                    }
                },
                "requests": { "total": 5 },
                "tokens": {
                    "saved": 1200,
                    "total_after_compression": 3600
                },
                "cost": {
                    "compression_savings_usd": 0.42,
                    "savings_usd": 9.99,
                    "net_savings_usd": 8.88,
                    "actual_cost_usd": 1.23
                },
                "savings_history": [
                    ["2026-03-21T10:00:00Z", 1200]
                ]
            }"#,
        )
        .expect("parsed stats");

        assert_eq!(parsed.session_requests, Some(5));
        assert_eq!(parsed.session_estimated_tokens_saved, Some(1_200));
        assert_eq!(parsed.session_estimated_savings_usd, Some(0.42));
        assert_eq!(parsed.session_actual_cost_usd, Some(1.23));
        assert_eq!(parsed.session_total_tokens_sent, Some(3_600));
        assert_eq!(parsed.savings_history.len(), 1);
    }

    #[test]
    fn parse_output_reduction_reads_available_estimate_from_by_layer() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 5 },
                "tokens": { "saved": 1200 },
                "savings": {
                    "by_layer": {
                        "output_shaping": {
                            "available": true,
                            "method": "estimated",
                            "reduction_percent": 18.4,
                            "ci_low_percent": 9.1,
                            "ci_high_percent": 27.7,
                            "requests": 340
                        }
                    }
                }
            }"#,
        )
        .expect("parsed stats");

        let reduction = parsed.output_reduction.expect("output reduction present");
        assert_eq!(reduction.method, "estimated");
        assert_eq!(reduction.reduction_percent, 18.4);
        assert_eq!(reduction.ci_low_percent, 9.1);
        assert_eq!(reduction.ci_high_percent, 27.7);
        assert_eq!(reduction.requests, 340);
    }

    #[test]
    fn parse_output_reduction_is_none_when_unavailable() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 5 },
                "tokens": { "saved": 1200 },
                "savings": {
                    "by_layer": {
                        "output_shaping": { "available": false }
                    }
                }
            }"#,
        )
        .expect("parsed stats");
        assert!(parsed.output_reduction.is_none());
    }

    #[test]
    fn parse_output_reduction_falls_back_to_tokens_block() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 5 },
                "tokens": {
                    "saved": 1200,
                    "output_reduction": {
                        "available": true,
                        "method": "measured",
                        "reduction_percent": 22.0,
                        "ci_low_percent": 15.0,
                        "ci_high_percent": 29.0,
                        "requests": 90
                    }
                }
            }"#,
        )
        .expect("parsed stats");
        let reduction = parsed.output_reduction.expect("output reduction present");
        assert_eq!(reduction.method, "measured");
        assert_eq!(reduction.requests, 90);
    }

    #[test]
    fn parse_headroom_stats_ratio_uses_new_input_not_cached_prefix() {
        // The cached prefix (cache_read) is re-sent every turn but never
        // compressed; it must not inflate the savings denominator. Under prompt
        // caching, new content lands in cache_write (here 7_000) plus any
        // uncached input (1_000), so new input is 8_000 and the ratio is
        // 2000 / (2000 + 8000) = 20% -- the 92_000 cache_read is excluded.
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 7 },
                "tokens": {
                    "saved": 2000,
                    "input": 100000
                },
                "prefix_cache": {
                    "totals": {
                        "cache_read_tokens": 92000,
                        "cache_write_tokens": 7000,
                        "uncached_input_tokens": 1000
                    }
                }
            }"#,
        )
        .expect("parsed stats");

        assert_eq!(parsed.session_estimated_tokens_saved, Some(2_000));
        // Denominator is new input (cache_write + uncached), not the 100_000
        // forwarded total and not uncached alone.
        assert_eq!(parsed.session_total_tokens_sent, Some(8_000));
        let pct = parsed.session_savings_pct.expect("savings pct");
        assert!((pct - 20.0).abs() < 1e-9, "expected 20%, got {pct}");
    }

    #[test]
    fn parse_headroom_stats_falls_back_to_total_when_new_input_is_zero() {
        // Fully-cached snapshot: prefix_cache.totals is present but cache_write
        // and uncached are both 0, so new_input_tokens is Some(0). The fallback
        // to the forwarded total (50_000) must still apply -- otherwise the
        // Some(0) skips `.or` and is dropped, leaving savings with zero spend.
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "tokens": {
                    "saved": 2000,
                    "input": 50000
                },
                "prefix_cache": {
                    "totals": {
                        "cache_read_tokens": 92000,
                        "cache_write_tokens": 0,
                        "uncached_input_tokens": 0
                    }
                }
            }"#,
        )
        .expect("parsed stats");

        assert_eq!(parsed.session_estimated_tokens_saved, Some(2_000));
        assert_eq!(parsed.session_total_tokens_sent, Some(50_000));
    }

    #[test]
    fn parse_headroom_stats_history_reads_hourly_and_daily_rollups() {
        let parsed = parse_headroom_stats_history_from_json(
            r#"{
                "lifetime": {
                    "tokens_saved": 205,
                    "compression_savings_usd": 0.205
                },
                "series": {
                    "hourly": [
                        {
                            "timestamp": "2026-03-27T09:00:00Z",
                            "tokens_saved": 150,
                            "compression_savings_usd_delta": 0.15,
                            "total_tokens_saved": 150,
                            "compression_savings_usd": 0.15
                        },
                        {
                            "timestamp": "2026-03-27T10:00:00Z",
                            "tokens_saved": 25,
                            "compression_savings_usd_delta": 0.025,
                            "total_tokens_saved": 175,
                            "compression_savings_usd": 0.175
                        }
                    ],
                    "daily": [
                        {
                            "timestamp": "2026-03-27T00:00:00Z",
                            "tokens_saved": 175,
                            "compression_savings_usd_delta": 0.175,
                            "total_tokens_saved": 175,
                            "compression_savings_usd": 0.175
                        }
                    ]
                }
            }"#,
        )
        .expect("parsed history");

        assert_eq!(parsed.lifetime_estimated_tokens_saved, Some(205));
        assert_eq!(parsed.lifetime_estimated_savings_usd, Some(0.205));
        assert_eq!(parsed.hourly.len(), 2);
        assert_eq!(parsed.hourly[0].tokens_saved, 150);
        assert!((parsed.hourly[0].compression_savings_usd_delta - 0.15).abs() < 1e-9);
        assert_eq!(parsed.daily.len(), 1);

        let daily_points = parsed.daily_savings();
        assert_eq!(daily_points.len(), 1);
        assert_eq!(daily_points[0].date, "2026-03-27");
        assert_eq!(daily_points[0].estimated_tokens_saved, 175);
        assert!((daily_points[0].estimated_savings_usd - 0.175).abs() < 1e-9);
        assert_eq!(daily_points[0].actual_cost_usd, 0.0);
        assert_eq!(daily_points[0].total_tokens_sent, 0);

        let hourly_points = parsed.hourly_savings();
        assert_eq!(hourly_points.len(), 2);
        let expected_hour = Utc
            .with_ymd_and_hms(2026, 3, 27, 9, 0, 0)
            .single()
            .expect("hour")
            .with_timezone(&Local)
            .format("%Y-%m-%dT%H:00")
            .to_string();
        assert_eq!(hourly_points[0].hour, expected_hour);
        assert_eq!(hourly_points[0].estimated_tokens_saved, 150);
        assert!((hourly_points[0].estimated_savings_usd - 0.15).abs() < 1e-9);
        // No by_provider in this fixture -> empty breakdown.
        assert!(hourly_points[0].by_provider.is_empty());
    }

    #[test]
    fn parse_headroom_stats_history_drops_carryover_boundary_when_trimmed() {
        // stored_points == max_history_points => the stored history was trimmed,
        // so the oldest rollup bucket (10:00 / day-of) carries a spurious
        // cumulative delta and must be dropped; the real 11:00 bucket stays.
        let body = r#"{
            "lifetime": { "tokens_saved": 1000, "compression_savings_usd": 10.0 },
            "retention": { "max_history_points": 5000 },
            "history_summary": { "stored_points": 5000 },
            "series": {
                "hourly": [
                    {
                        "timestamp": "2026-06-09T10:00:00Z",
                        "tokens_saved": 900,
                        "compression_savings_usd_delta": 9.0,
                        "total_tokens_saved": 900,
                        "compression_savings_usd": 9.0
                    },
                    {
                        "timestamp": "2026-06-09T11:00:00Z",
                        "tokens_saved": 100,
                        "compression_savings_usd_delta": 1.0,
                        "total_tokens_saved": 1000,
                        "compression_savings_usd": 10.0
                    }
                ],
                "daily": [
                    {
                        "timestamp": "2026-06-09T00:00:00Z",
                        "tokens_saved": 900,
                        "compression_savings_usd_delta": 9.0,
                        "total_tokens_saved": 900,
                        "compression_savings_usd": 9.0
                    },
                    {
                        "timestamp": "2026-06-10T00:00:00Z",
                        "tokens_saved": 100,
                        "compression_savings_usd_delta": 1.0,
                        "total_tokens_saved": 1000,
                        "compression_savings_usd": 10.0
                    }
                ]
            }
        }"#;
        let parsed = parse_headroom_stats_history_from_json(body).expect("parsed history");

        // Boundary bucket dropped; lifetime totals untouched.
        assert_eq!(parsed.lifetime_estimated_tokens_saved, Some(1000));
        assert_eq!(parsed.daily.len(), 1);
        assert_eq!(parsed.daily[0].tokens_saved, 100);
        assert_eq!(parsed.hourly.len(), 1);
        assert_eq!(parsed.hourly[0].tokens_saved, 100);
    }

    #[test]
    fn parse_headroom_stats_history_keeps_first_bucket_when_not_trimmed() {
        // stored_points < max_history_points => untrimmed (new user); the first
        // bucket is the genuine origin and must be preserved.
        let body = r#"{
            "lifetime": { "tokens_saved": 1000, "compression_savings_usd": 10.0 },
            "retention": { "max_history_points": 5000 },
            "history_summary": { "stored_points": 12 },
            "series": {
                "daily": [
                    {
                        "timestamp": "2026-06-09T00:00:00Z",
                        "tokens_saved": 900,
                        "compression_savings_usd_delta": 9.0,
                        "total_tokens_saved": 900,
                        "compression_savings_usd": 9.0
                    },
                    {
                        "timestamp": "2026-06-10T00:00:00Z",
                        "tokens_saved": 100,
                        "compression_savings_usd_delta": 1.0,
                        "total_tokens_saved": 1000,
                        "compression_savings_usd": 10.0
                    }
                ]
            }
        }"#;
        let parsed = parse_headroom_stats_history_from_json(body).expect("parsed history");
        assert_eq!(parsed.daily.len(), 2);
        assert_eq!(parsed.daily[0].tokens_saved, 900);
    }

    #[test]
    fn parse_headroom_stats_history_attributes_hourly_by_provider() {
        let parsed = parse_headroom_stats_history_from_json(
            r#"{
                "series": {
                    "hourly": [
                        {
                            "timestamp": "2026-03-27T09:00:00Z",
                            "tokens_saved": 140,
                            "compression_savings_usd_delta": 0.14,
                            "total_input_tokens_delta": 200,
                            "total_input_cost_usd_delta": 0.40,
                            "by_provider": {
                                "openai": {
                                    "tokens_saved": 40,
                                    "compression_savings_usd_delta": 0.04,
                                    "total_input_tokens_delta": 80,
                                    "total_input_cost_usd_delta": 0.16
                                },
                                "anthropic": {
                                    "tokens_saved": 100,
                                    "compression_savings_usd_delta": 0.10,
                                    "total_input_tokens_delta": 120,
                                    "total_input_cost_usd_delta": 0.24
                                }
                            }
                        }
                    ]
                }
            }"#,
        )
        .expect("parsed history");

        // Parsed rollup keeps every provider, sorted by name for stable display.
        let providers = &parsed.hourly[0].by_provider;
        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].provider, "anthropic");
        assert_eq!(providers[1].provider, "openai");

        // hourly_savings() maps the delta fields onto the display point.
        let hourly_points = parsed.hourly_savings();
        let by_provider = &hourly_points[0].by_provider;
        assert_eq!(by_provider.len(), 2);
        let anthropic = &by_provider[0];
        assert_eq!(anthropic.provider, "anthropic");
        assert_eq!(anthropic.estimated_tokens_saved, 100);
        assert!((anthropic.estimated_savings_usd - 0.10).abs() < 1e-9);
        assert_eq!(anthropic.total_tokens_sent, 120);
        assert!((anthropic.actual_cost_usd - 0.24).abs() < 1e-9);
        let openai = &by_provider[1];
        assert_eq!(openai.provider, "openai");
        assert_eq!(openai.estimated_tokens_saved, 40);
        // Per-provider tokens-saved sum back to the bucket total.
        assert_eq!(
            anthropic.estimated_tokens_saved + openai.estimated_tokens_saved,
            hourly_points[0].estimated_tokens_saved
        );
    }

    #[test]
    fn parse_headroom_stats_accepts_naive_local_savings_history_timestamps() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 5 },
                "tokens": {
                    "input": 3600,
                    "saved": 1200
                },
                "cost": {
                    "compression_savings_usd": 0.42,
                    "total_input_cost_usd": 0.08
                },
                "savings_history": [
                    ["2026-03-24T11:52:00.866732", 1200]
                ]
            }"#,
        )
        .expect("parsed stats");

        assert_eq!(parsed.savings_history.len(), 1);
    }

    #[test]
    fn parse_headroom_stats_prefers_actual_input_cost_and_ignores_generic_total_cost() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 5 },
                "tokens": {
                    "saved": 1200,
                    "actual_input_tokens": 3600
                },
                "cost": {
                    "compression_savings_usd": 0.42,
                    "actual_input_cost_usd": 0.08,
                    "total_usd": 1.75
                }
            }"#,
        )
        .expect("parsed stats");

        assert_eq!(parsed.session_actual_cost_usd, Some(0.08));
        assert_eq!(parsed.session_total_tokens_sent, Some(3_600));
    }

    #[test]
    fn parse_headroom_stats_reads_total_input_fields_from_stats_cost_block() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 5 },
                "tokens": {
                    "input": 3600,
                    "saved": 1200
                },
                "cost": {
                    "compression_savings_usd": 0.42,
                    "total_input_cost_usd": 0.08,
                    "cost_with_headroom_usd": 0.08
                }
            }"#,
        )
        .expect("parsed stats");

        assert_eq!(parsed.session_actual_cost_usd, Some(0.08));
        assert_eq!(parsed.session_total_tokens_sent, Some(3_600));
    }

    #[test]
    fn parse_headroom_stats_does_not_derive_spend_when_actual_cost_is_missing() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 5 },
                "tokens": {
                    "saved": 1200,
                    "total_after_compression": 3600
                },
                "cost": {
                    "compression_savings_usd": 0.42,
                    "total_usd": 1.75
                }
            }"#,
        )
        .expect("parsed stats");

        assert_eq!(parsed.session_actual_cost_usd, None);
        assert_eq!(parsed.session_total_tokens_sent, Some(3_600));
    }

    #[test]
    fn parse_headroom_stats_does_not_derive_tokens_sent_when_missing() {
        let parsed = parse_headroom_stats_from_json(
            r#"{
                "requests": { "total": 5 },
                "tokens": {
                    "saved": 1200,
                    "savings_percent": 25.0
                },
                "cost": {
                    "compression_savings_usd": 0.42,
                    "actual_input_cost_usd": 0.08
                }
            }"#,
        )
        .expect("parsed stats");

        assert_eq!(parsed.session_total_tokens_sent, None);
        assert_eq!(parsed.session_actual_cost_usd, Some(0.08));
    }

    #[test]
    fn first_observation_without_savings_history_does_not_invent_hourly_bucket_totals() {
        let mut tracker = make_tracker();
        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(4),
                session_estimated_savings_usd: Some(0.5),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.3),
                session_total_tokens_sent: Some(3_000),
                savings_history: Vec::new(),
            })
            .expect("snapshot");

        assert!(tracker.hourly_savings().is_empty());
        assert!(tracker.daily_savings().is_empty());
    }

    #[test]
    fn spend_backfill_is_distributed_across_existing_session_hours() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(4),
                session_estimated_savings_usd: Some(0.5),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.0),
                session_total_tokens_sent: Some(3_000),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 11, 0),
                    history_point_at(2026, 3, 20, 12, 400),
                    history_point_at(2026, 3, 21, 12, 1_000),
                ],
            })
            .expect("first snapshot");

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(4),
                session_estimated_savings_usd: Some(0.5),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.3),
                session_total_tokens_sent: Some(3_000),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 11, 0),
                    history_point_at(2026, 3, 20, 12, 400),
                    history_point_at(2026, 3, 21, 12, 1_000),
                ],
            })
            .expect("second snapshot");

        let daily = tracker.daily_savings();
        assert_eq!(daily.len(), 2);
        assert!((daily[0].actual_cost_usd - 0.12).abs() < 1e-9);
        assert!((daily[1].actual_cost_usd - 0.18).abs() < 1e-9);
    }

    #[test]
    fn incremental_updates_use_savings_history_hour_keys_instead_of_now() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(1),
                session_estimated_savings_usd: Some(0.2),
                session_estimated_tokens_saved: Some(400),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.12),
                session_total_tokens_sent: Some(1_200),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 8, 0),
                    history_point_at(2026, 3, 20, 9, 400),
                ],
            })
            .expect("first snapshot");

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(2),
                session_estimated_savings_usd: Some(0.5),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.3),
                session_total_tokens_sent: Some(3_000),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 9, 400),
                    history_point_at(2026, 3, 20, 10, 1_000),
                ],
            })
            .expect("second snapshot");

        let hourly = tracker.hourly_savings();
        let expected_first_hour = Utc
            .with_ymd_and_hms(2026, 3, 20, 9, 0, 0)
            .single()
            .expect("first hour")
            .with_timezone(&Local)
            .format("%Y-%m-%dT%H:00")
            .to_string();
        let expected_second_hour = Utc
            .with_ymd_and_hms(2026, 3, 20, 10, 0, 0)
            .single()
            .expect("second hour")
            .with_timezone(&Local)
            .format("%Y-%m-%dT%H:00")
            .to_string();

        assert_eq!(hourly.len(), 2);
        assert_eq!(hourly[0].hour, expected_first_hour);
        assert_eq!(hourly[0].estimated_tokens_saved, 400);
        assert_eq!(hourly[1].hour, expected_second_hour);
        assert_eq!(hourly[1].estimated_tokens_saved, 600);
        assert_eq!(hourly[1].total_tokens_sent, 1_800);
    }

    #[test]
    fn observing_repairs_stale_current_session_hourly_overlay() {
        let mut tracker = make_tracker();
        tracker.last_observation = Some(SavingsObservation {
            observed_at: Utc::now(),
            last_activity_at: Some(Utc::now()),
            session_requests: 10,
            session_estimated_savings_usd: 10.0,
            session_estimated_tokens_saved: 10_000,
            session_actual_cost_usd: 1.0,
            session_total_tokens_sent: 5_000,
        });
        tracker.session_hourly_buckets.insert(
            "2026-03-24T13:00".into(),
            DailySavingsBucket {
                estimated_savings_usd: 20.0,
                estimated_tokens_saved: 6_000_000,
                actual_cost_usd: 0.01,
                total_tokens_sent: 600_000,
            },
        );
        tracker.hourly_savings.insert(
            "2026-03-24T13:00".into(),
            DailySavingsBucket {
                estimated_savings_usd: 20.0,
                estimated_tokens_saved: 6_000_000,
                actual_cost_usd: 0.01,
                total_tokens_sent: 600_000,
            },
        );
        tracker.daily_savings.insert(
            "2026-03-24".into(),
            DailySavingsBucket {
                estimated_savings_usd: 20.0,
                estimated_tokens_saved: 6_000_000,
                actual_cost_usd: 0.01,
                total_tokens_sent: 600_000,
            },
        );

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(11),
                session_estimated_savings_usd: Some(10.1),
                session_estimated_tokens_saved: Some(10_200),
                session_savings_pct: Some(50.0),
                session_actual_cost_usd: Some(1.01),
                session_total_tokens_sent: Some(5_100),
                savings_history: vec![
                    history_point_at(2026, 3, 24, 11, 0),
                    history_point_at(2026, 3, 24, 12, 10_200),
                ],
            })
            .expect("snapshot");

        let hourly = tracker.hourly_savings();
        assert_eq!(hourly.len(), 1);
        assert_eq!(hourly[0].estimated_tokens_saved, 10_200);
        assert!((hourly[0].estimated_savings_usd - 10.1).abs() < 1e-9);
    }

    #[test]
    fn persisted_session_history_prevents_rolling_window_from_reassigning_older_hour() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(2),
                session_estimated_savings_usd: Some(0.5),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.3),
                session_total_tokens_sent: Some(3_000),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 8, 0),
                    history_point_at(2026, 3, 20, 9, 400),
                    history_point_at(2026, 3, 20, 10, 1_000),
                ],
            })
            .expect("first snapshot");

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(3),
                session_estimated_savings_usd: Some(0.6),
                session_estimated_tokens_saved: Some(1_200),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.36),
                session_total_tokens_sent: Some(3_600),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 10, 1_000),
                    history_point_at(2026, 3, 20, 10, 1_200),
                ],
            })
            .expect("second snapshot");

        let hourly = tracker.hourly_savings();
        assert_eq!(hourly.len(), 2);
        assert_eq!(hourly[0].estimated_tokens_saved, 400);
        assert_eq!(hourly[1].estimated_tokens_saved, 800);
    }

    #[test]
    fn single_visible_history_point_does_not_invent_hourly_attribution() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(1),
                session_estimated_savings_usd: Some(0.2),
                session_estimated_tokens_saved: Some(400),
                session_savings_pct: Some(25.0),
                session_actual_cost_usd: Some(0.12),
                session_total_tokens_sent: Some(1_200),
                savings_history: vec![history_point_at(2026, 3, 20, 9, 400)],
            })
            .expect("snapshot");

        assert!(tracker.hourly_savings().is_empty());
        assert!(tracker.daily_savings().is_empty());
    }

    #[test]
    fn visible_hours_only_get_attributable_tokens_sent_and_spend() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(5),
                session_estimated_savings_usd: Some(10.0),
                session_estimated_tokens_saved: Some(10_000),
                session_savings_pct: Some(50.0),
                session_actual_cost_usd: Some(4.0),
                session_total_tokens_sent: Some(8_000),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 8, 7_000),
                    history_point_at(2026, 3, 20, 9, 8_000),
                    history_point_at(2026, 3, 20, 10, 10_000),
                ],
            })
            .expect("snapshot");

        let hourly = tracker.hourly_savings();
        assert_eq!(hourly.len(), 2);
        assert_eq!(hourly[0].estimated_tokens_saved, 1_000);
        assert_eq!(hourly[1].estimated_tokens_saved, 2_000);
        assert_eq!(hourly[0].total_tokens_sent, 800);
        assert_eq!(hourly[1].total_tokens_sent, 1_600);
        assert!((hourly[0].actual_cost_usd - 0.4).abs() < 1e-9);
        assert!((hourly[1].actual_cost_usd - 0.8).abs() < 1e-9);
    }

    #[test]
    fn rolling_window_does_not_dump_unattributable_remainder_into_last_hour() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(5),
                session_estimated_savings_usd: Some(10.0),
                session_estimated_tokens_saved: Some(10_000),
                session_savings_pct: Some(50.0),
                session_actual_cost_usd: Some(4.0),
                session_total_tokens_sent: Some(8_000),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 8, 0),
                    history_point_at(2026, 3, 20, 9, 4_000),
                    history_point_at(2026, 3, 20, 10, 7_000),
                ],
            })
            .expect("first snapshot");

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(6),
                session_estimated_savings_usd: Some(10.0),
                session_estimated_tokens_saved: Some(10_000),
                session_savings_pct: Some(50.0),
                session_actual_cost_usd: Some(4.0),
                session_total_tokens_sent: Some(8_000),
                savings_history: vec![
                    history_point_at(2026, 3, 20, 10, 7_000),
                    history_point_at(2026, 3, 20, 11, 10_000),
                ],
            })
            .expect("second snapshot");

        let hourly = tracker.hourly_savings();
        assert_eq!(hourly.len(), 3);
        assert_eq!(hourly[2].estimated_tokens_saved, 3_000);
        assert_eq!(hourly[2].total_tokens_sent, 2_400);
        assert!((hourly[2].actual_cost_usd - 1.2).abs() < 1e-9);
    }

    #[test]
    fn missing_optional_spend_fields_do_not_trigger_session_reset() {
        let mut tracker = make_tracker();

        tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(10),
                session_estimated_savings_usd: Some(1.0),
                session_estimated_tokens_saved: Some(1_000),
                session_savings_pct: Some(20.0),
                session_actual_cost_usd: Some(4.0),
                session_total_tokens_sent: Some(4_000),
                savings_history: Vec::new(),
            })
            .expect("first snapshot");

        let second = tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(11),
                session_estimated_savings_usd: Some(1.2),
                session_estimated_tokens_saved: Some(1_200),
                session_savings_pct: Some(20.0),
                session_actual_cost_usd: None,
                session_total_tokens_sent: None,
                savings_history: Vec::new(),
            })
            .expect("second snapshot");

        assert!((second.lifetime_estimated_savings_usd - 1.2).abs() < 1e-9);
        assert_eq!(second.lifetime_estimated_tokens_saved, 1_200);
        assert_eq!(second.lifetime_requests, 11);
    }

    #[test]
    fn overnight_inactivity_rolls_only_the_display_session() {
        let mut tracker = make_tracker();
        let now = Utc::now();
        let prior_activity = (now - chrono::Duration::hours(2))
            .with_timezone(&Local)
            .date_naive()
            .pred_opt()
            .expect("prior day")
            .and_hms_opt(23, 0, 0)
            .expect("valid time")
            .and_local_timezone(Local)
            .single()
            .expect("local timestamp")
            .with_timezone(&Utc);

        tracker.last_observation = Some(SavingsObservation {
            observed_at: now - chrono::Duration::minutes(5),
            last_activity_at: Some(prior_activity),
            session_requests: 10,
            session_estimated_savings_usd: 5.0,
            session_estimated_tokens_saved: 1_000,
            session_actual_cost_usd: 2.0,
            session_total_tokens_sent: 4_000,
        });
        tracker.session_requests = 10;
        tracker.session_estimated_savings_usd = 5.0;
        tracker.session_estimated_tokens_saved = 1_000;
        tracker.session_savings_pct = 20.0;
        tracker.lifetime_requests = 10;
        tracker.lifetime_estimated_savings_usd = 5.0;
        tracker.lifetime_estimated_tokens_saved = 1_000;

        let snapshot = tracker
            .observe(&HeadroomDashboardStats {
                output_reduction: None,
                session_requests: Some(11),
                session_estimated_savings_usd: Some(5.5),
                session_estimated_tokens_saved: Some(1_100),
                session_savings_pct: Some(21.57),
                session_actual_cost_usd: Some(2.4),
                session_total_tokens_sent: Some(4_400),
                savings_history: Vec::new(),
            })
            .expect("snapshot");

        assert_eq!(snapshot.session_requests, 1);
        assert_eq!(snapshot.session_estimated_tokens_saved, 100);
        assert!((snapshot.session_estimated_savings_usd - 0.5).abs() < 1e-9);
        assert!((snapshot.session_savings_pct - 20.0).abs() < 1e-9);
        assert_eq!(snapshot.lifetime_requests, 11);
        assert_eq!(snapshot.lifetime_estimated_tokens_saved, 1_100);
    }

    #[test]
    fn load_or_create_ignores_old_persisted_snapshot_schema() {
        let base_dir = std::env::temp_dir().join(format!(
            "headroom-savings-state-test-{}",
            uuid::Uuid::new_v4()
        ));
        ensure_data_dirs(&base_dir).expect("create temp dirs");

        std::fs::write(telemetry_file(&base_dir, "savings-records.jsonl"), "")
            .expect("write empty journal");
        let persisted = PersistedSavingsState {
            schema_version: 1,
            session_requests: 5,
            session_estimated_savings_usd: 0.9,
            session_estimated_tokens_saved: 900,
            session_savings_pct: 18.0,
            lifetime_requests: 12,
            lifetime_estimated_savings_usd: 2.4,
            lifetime_estimated_tokens_saved: 2_400,
            last_observation: Some(SavingsObservation {
                observed_at: Utc::now(),
                last_activity_at: Some(Utc::now()),
                session_requests: 5,
                session_estimated_savings_usd: 0.9,
                session_estimated_tokens_saved: 900,
                session_actual_cost_usd: 0.0,
                session_total_tokens_sent: 0,
            }),
            last_rtk_observation: None,
            display_session_baseline: None,
            session_savings_history: Vec::new(),
            session_hourly_buckets: std::collections::BTreeMap::new(),
            daily_savings: std::collections::BTreeMap::new(),
            hourly_savings: std::collections::BTreeMap::new(),
        };
        std::fs::write(
            config_file(&base_dir, "savings-state.json"),
            serde_json::to_vec_pretty(&persisted).expect("serialize persisted state"),
        )
        .expect("write persisted state");

        let tracker = SavingsTracker::load_or_create(&base_dir).expect("load tracker");
        assert!((tracker.lifetime_estimated_savings_usd - 0.0).abs() < 1e-9);
        assert_eq!(tracker.lifetime_estimated_tokens_saved, 0);
        assert_eq!(tracker.lifetime_requests, 0);

        let _ = std::fs::remove_dir_all(base_dir);
    }

    fn daily(date: &str, tokens: u64, usd: f64) -> DailySavingsPoint {
        DailySavingsPoint {
            date: date.to_string(),
            estimated_tokens_saved: tokens,
            estimated_savings_usd: usd,
            actual_cost_usd: 0.0,
            total_tokens_sent: 0,
        }
    }

    fn hourly(hour: &str, tokens: u64) -> HourlySavingsPoint {
        HourlySavingsPoint {
            hour: hour.to_string(),
            estimated_tokens_saved: tokens,
            estimated_savings_usd: 0.0,
            actual_cost_usd: 0.0,
            total_tokens_sent: 0,
            by_provider: Vec::new(),
        }
    }

    #[test]
    fn ingest_native_rollups_writes_settled_days_only_and_is_idempotent() {
        let mut tracker = make_tracker();
        let cutoff = "2026-06-02";
        let today = "2026-06-16";
        let native_daily = vec![
            daily("2026-06-01", 999, 9.99), // pre-cutoff -> skipped
            daily("2026-06-10", 100, 1.0),  // settled -> ingested
            daily("2026-06-16", 500, 5.0),  // today -> left to observe
        ];
        let native_hourly = vec![
            hourly("2026-06-10T09:00", 40), // settled day -> ingested
            hourly("2026-06-16T09:00", 60), // today -> skipped
        ];

        assert!(tracker.ingest_native_rollups(&native_daily, &native_hourly, cutoff, today));

        let daily_dates: Vec<String> = tracker
            .daily_savings()
            .into_iter()
            .map(|p| p.date)
            .collect();
        assert_eq!(daily_dates, vec!["2026-06-10"]);
        let hourly_keys: Vec<String> = tracker
            .hourly_savings()
            .into_iter()
            .map(|p| p.hour)
            .collect();
        assert_eq!(hourly_keys, vec!["2026-06-10T09:00"]);

        // Re-ingesting identical data must not report a change (no needless persist).
        assert!(!tracker.ingest_native_rollups(&native_daily, &native_hourly, cutoff, today));
    }

    #[test]
    fn ingest_native_rollups_overwrites_stale_tracker_value_with_authoritative() {
        let mut tracker = make_tracker();
        // A prior, approximate self-observed value for a settled day.
        assert!(tracker.ingest_native_rollups(
            &[daily("2026-06-10", 50, 0.5)],
            &[],
            "2026-06-02",
            "2026-06-16",
        ));
        // Backend reports the authoritative (different) value -> overwrite + change.
        assert!(tracker.ingest_native_rollups(
            &[daily("2026-06-10", 100, 1.0)],
            &[],
            "2026-06-02",
            "2026-06-16",
        ));
        let point = tracker
            .daily_savings()
            .into_iter()
            .find(|p| p.date == "2026-06-10")
            .expect("settled day present");
        assert_eq!(point.estimated_tokens_saved, 100);
    }

    #[test]
    fn ingest_native_rollups_leaves_days_absent_from_native_untouched() {
        // Guards the integrity property: once the trimmed boundary bucket is
        // dropped by the parser it is absent from `native`, so ingestion must
        // never clobber the good value archived on the prior render.
        let mut tracker = make_tracker();
        assert!(tracker.ingest_native_rollups(
            &[daily("2026-06-10", 100, 1.0)],
            &[],
            "2026-06-02",
            "2026-06-16",
        ));
        // Next render: June 10 is now the dropped boundary (absent); only the
        // newer settled day arrives.
        assert!(tracker.ingest_native_rollups(
            &[daily("2026-06-11", 70, 0.7)],
            &[],
            "2026-06-02",
            "2026-06-16",
        ));
        let by_date: std::collections::BTreeMap<String, u64> = tracker
            .daily_savings()
            .into_iter()
            .map(|p| (p.date, p.estimated_tokens_saved))
            .collect();
        assert_eq!(by_date.get("2026-06-10"), Some(&100)); // preserved
        assert_eq!(by_date.get("2026-06-11"), Some(&70)); // newly archived
    }

    // merge_daily_savings

    #[test]
    fn merge_daily_tracker_preferred_before_cutoff() {
        let tracker = vec![daily("2026-04-13", 500, 1.0)];
        let history = vec![daily("2026-04-13", 999, 2.0)];
        let result = merge_daily_savings(tracker, history, "2026-04-20");
        assert_eq!(result.len(), 1);
        // tracker wins pre-cutoff
        assert_eq!(result[0].estimated_tokens_saved, 500);
    }

    #[test]
    fn merge_daily_history_preferred_on_and_after_cutoff() {
        let tracker = vec![daily("2026-04-20", 100, 0.5)];
        let history = vec![daily("2026-04-20", 800, 2.0)];
        let result = merge_daily_savings(tracker, history, "2026-04-20");
        assert_eq!(result.len(), 1);
        // history wins on cutoff date
        assert_eq!(result[0].estimated_tokens_saved, 800);
    }

    #[test]
    fn merge_daily_fallback_when_only_tracker_has_post_cutoff_day() {
        let tracker = vec![daily("2026-04-21", 300, 1.2)];
        let result = merge_daily_savings(tracker, vec![], "2026-04-20");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].estimated_tokens_saved, 300);
    }

    #[test]
    fn merge_daily_drops_history_pre_cutoff() {
        // Pre-cutoff is tracker-only: empty tracker + pre-cutoff history => no entry.
        // This protects against pre-v6 schema drift leaking into the graph.
        let history = vec![daily("2026-04-10", 400, 1.5)];
        let result = merge_daily_savings(vec![], history, "2026-04-20");
        assert!(result.is_empty());
    }

    #[test]
    fn merge_daily_combines_days_from_both_sources() {
        let tracker = vec![daily("2026-04-10", 200, 0.8), daily("2026-04-13", 300, 1.0)];
        let history = vec![daily("2026-04-20", 500, 2.0), daily("2026-04-21", 600, 2.5)];
        let mut result = merge_daily_savings(tracker, history, "2026-04-20");
        result.sort_by(|a, b| a.date.cmp(&b.date));
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].date, "2026-04-10");
        assert_eq!(result[3].date, "2026-04-21");
    }

    // merge_hourly_savings

    #[test]
    fn merge_hourly_tracker_preferred_before_cutoff() {
        let tracker = vec![hourly("2026-04-13T10:00", 500)];
        let history = vec![hourly("2026-04-13T10:00", 999)];
        let result = merge_hourly_savings(tracker, history, "2026-04-20T00:00");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].estimated_tokens_saved, 500);
    }

    #[test]
    fn merge_hourly_history_preferred_on_and_after_cutoff() {
        let tracker = vec![hourly("2026-04-20T09:00", 100)];
        let history = vec![hourly("2026-04-20T09:00", 800)];
        let result = merge_hourly_savings(tracker, history, "2026-04-20T00:00");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].estimated_tokens_saved, 800);
    }

    #[test]
    fn merge_hourly_drops_history_pre_cutoff() {
        // Pre-cutoff is tracker-only: empty tracker + pre-cutoff history => no entries.
        let tracker: Vec<HourlySavingsPoint> = vec![];
        let history = vec![
            hourly("2026-04-13T09:00", 400),
            hourly("2026-04-13T10:00", 600),
        ];
        let result = merge_hourly_savings(tracker, history, "2026-04-20T00:00");
        assert!(result.is_empty());
    }

    #[test]
    fn tracker_observe_called_updates_hourly_savings_even_with_history_present() {
        // Regression: tracker.observe() must be called regardless of whether native
        // history is available, so that hourly buckets stay current.
        let today = chrono::Local::now();
        let hp = |hour: u32, total: u64| -> HeadroomSavingsHistoryPoint {
            history_point_at(today.year(), today.month(), today.day(), hour, total)
        };
        let mut tracker = make_tracker();

        // First observation: 1_000 tokens saved, history shows 0→1_000 across hours 9→10.
        tracker.observe(&HeadroomDashboardStats {
            output_reduction: None,
            session_requests: Some(1),
            session_estimated_savings_usd: Some(1.0),
            session_estimated_tokens_saved: Some(1_000),
            session_savings_pct: Some(30.0),
            session_actual_cost_usd: Some(0.5),
            session_total_tokens_sent: Some(3_000),
            savings_history: vec![hp(9, 0), hp(10, 1_000)],
        });
        let total_first: u64 = tracker
            .hourly_savings()
            .iter()
            .map(|p| p.estimated_tokens_saved)
            .sum();

        // Second observation: 3_000 tokens saved, history adds hour 11.
        tracker.observe(&HeadroomDashboardStats {
            output_reduction: None,
            session_requests: Some(3),
            session_estimated_savings_usd: Some(3.0),
            session_estimated_tokens_saved: Some(3_000),
            session_savings_pct: Some(30.0),
            session_actual_cost_usd: Some(1.5),
            session_total_tokens_sent: Some(9_000),
            savings_history: vec![hp(9, 0), hp(10, 1_000), hp(11, 3_000)],
        });
        let total_second: u64 = tracker
            .hourly_savings()
            .iter()
            .map(|p| p.estimated_tokens_saved)
            .sum();

        assert!(
            total_second > total_first,
            "hourly savings should grow with each observe call: first={total_first} second={total_second}"
        );
    }

    fn idle_progress() -> BootstrapProgress {
        BootstrapProgress {
            running: false,
            complete: false,
            failed: false,
            current_step: String::new(),
            message: String::new(),
            current_step_eta_seconds: 0,
            overall_percent: 0,
        }
    }

    #[test]
    fn begin_bootstrap_skips_install_when_python_already_installed() {
        let (next, result) = begin_bootstrap_transition(&idle_progress(), true);
        assert!(result.is_ok());
        assert!(next.complete);
        assert!(!next.running);
        assert!(!next.failed);
        assert_eq!(next.overall_percent, 100);
    }

    #[test]
    fn begin_bootstrap_starts_when_python_missing() {
        let (next, result) = begin_bootstrap_transition(&idle_progress(), false);
        assert!(result.is_ok());
        assert!(next.running);
        assert!(!next.complete);
        assert!(!next.failed);
        assert_eq!(next.overall_percent, 2);
    }

    #[test]
    fn begin_bootstrap_rejects_reentry_while_running() {
        let running = BootstrapProgress {
            running: true,
            overall_percent: 42,
            ..idle_progress()
        };
        let (next, result) = begin_bootstrap_transition(&running, false);
        assert!(result.is_err());
        // State is preserved when re-entry is rejected.
        assert_eq!(next.overall_percent, 42);
        assert!(next.running);
    }

    #[test]
    fn begin_bootstrap_after_failure_restarts_cleanly() {
        let failed = BootstrapProgress {
            failed: true,
            overall_percent: 50,
            message: "boom".into(),
            ..idle_progress()
        };
        let (next, result) = begin_bootstrap_transition(&failed, false);
        assert!(result.is_ok());
        assert!(next.running);
        assert!(!next.failed);
        assert_eq!(next.overall_percent, 2);
    }

    #[test]
    fn apply_step_normalizes_into_running_state() {
        let failed = BootstrapProgress {
            failed: true,
            ..idle_progress()
        };
        let next = apply_bootstrap_step(
            &failed,
            BootstrapStepUpdate {
                step: "Downloading Python",
                message: "Fetching runtime".into(),
                eta_seconds: 30,
                percent: 40,
            },
        );
        assert!(next.running);
        assert!(!next.failed);
        assert!(!next.complete);
        assert_eq!(next.current_step, "Downloading Python");
        assert_eq!(next.overall_percent, 40);
        assert_eq!(next.current_step_eta_seconds, 30);
    }

    #[test]
    fn complete_state_pins_to_full_progress() {
        let next = bootstrap_complete_state();
        assert!(next.complete);
        assert!(!next.running);
        assert!(!next.failed);
        assert_eq!(next.overall_percent, 100);
    }

    #[test]
    fn failed_state_preserves_current_percent_with_min_of_one() {
        let current = BootstrapProgress {
            running: true,
            overall_percent: 72,
            ..idle_progress()
        };
        let next = bootstrap_failed_state(&current, "download error".into());
        assert!(next.failed);
        assert!(!next.running);
        assert!(!next.complete);
        assert_eq!(next.overall_percent, 72);
        assert_eq!(next.message, "download error");
    }

    #[test]
    fn failed_state_floors_zero_percent_to_one() {
        let next = bootstrap_failed_state(&idle_progress(), "early failure".into());
        assert_eq!(next.overall_percent, 1);
        assert!(next.failed);
    }

    #[test]
    fn repo_memory_mcp_supervision_distinguishes_stale_active_state() {
        let current_pid = 42;
        let healthy_service = crate::models::RepoMemoryMcpServiceStatus {
            managed_by_app: true,
            read_only: true,
            transport: "stdio".to_string(),
            command: "node repo-intelligence.mjs --mcp-serve".to_string(),
            descriptor_path: "/tmp/repo-memory.json".to_string(),
            descriptor_present: true,
            script_path: "/tmp/repo-intelligence.mjs".to_string(),
            script_present: true,
            node_available: true,
            healthy: true,
            issues: Vec::new(),
        };
        let active = super::repo_memory_mcp::RepoMemoryMcpSessionState {
            active: true,
            last_started_at: None,
            last_checked_at: None,
            supervision_status: None,
            supervisor_pid: None,
        };
        assert_eq!(
            super::repo_memory_mcp::repo_memory_mcp_supervision_status(
                &active,
                Some(true),
                current_pid,
                Some(&healthy_service)
            ),
            "active"
        );
        assert_eq!(
            super::repo_memory_mcp::repo_memory_mcp_supervision_status(
                &active,
                Some(false),
                current_pid,
                Some(&healthy_service)
            ),
            "stale_config"
        );
        assert_eq!(
            super::repo_memory_mcp::repo_memory_mcp_supervision_status(
                &active,
                None,
                current_pid,
                Some(&healthy_service)
            ),
            "unknown_active"
        );

        let broken_service = crate::models::RepoMemoryMcpServiceStatus {
            script_present: false,
            healthy: false,
            issues: vec!["script_missing".to_string()],
            ..healthy_service.clone()
        };
        assert!(!super::repo_memory_mcp::repo_memory_mcp_service_healthy(
            Some(&broken_service)
        ));
        assert_eq!(broken_service.issues, vec!["script_missing"]);
        assert_eq!(
            super::repo_memory_mcp::repo_memory_mcp_supervision_status(
                &active,
                Some(true),
                current_pid,
                Some(&broken_service)
            ),
            "service_unhealthy"
        );

        let verified_this_process = super::repo_memory_mcp::RepoMemoryMcpSessionState {
            active: true,
            last_started_at: None,
            last_checked_at: None,
            supervision_status: Some("verified_active".to_string()),
            supervisor_pid: Some(current_pid),
        };
        assert_eq!(
            super::repo_memory_mcp::repo_memory_mcp_supervision_status(
                &verified_this_process,
                Some(true),
                current_pid,
                Some(&healthy_service)
            ),
            "verified_active"
        );

        let verified_previous_process = super::repo_memory_mcp::RepoMemoryMcpSessionState {
            active: true,
            last_started_at: None,
            last_checked_at: None,
            supervision_status: Some("verified_active".to_string()),
            supervisor_pid: Some(current_pid + 1),
        };
        assert_eq!(
            super::repo_memory_mcp::repo_memory_mcp_supervision_status(
                &verified_previous_process,
                Some(true),
                current_pid,
                Some(&healthy_service)
            ),
            "restart_required"
        );

        let stopped = super::repo_memory_mcp::RepoMemoryMcpSessionState::default();
        assert_eq!(
            super::repo_memory_mcp::repo_memory_mcp_supervision_status(
                &stopped,
                Some(true),
                current_pid,
                Some(&healthy_service)
            ),
            "configured"
        );
        assert_eq!(
            super::repo_memory_mcp::repo_memory_mcp_supervision_status(
                &stopped,
                Some(false),
                current_pid,
                Some(&healthy_service)
            ),
            "needs_attention"
        );
    }

    #[test]
    fn repo_memory_mcp_supervision_due_requires_current_active_verified_session() {
        let current_pid = 42;
        let now = Utc
            .with_ymd_and_hms(2026, 6, 30, 10, 0, 0)
            .single()
            .unwrap();
        let stale_check = Some(
            now - chrono::Duration::seconds(
                super::repo_memory_mcp::REPO_MEMORY_MCP_SUPERVISION_INTERVAL_SECS + 1,
            ),
        );
        let fresh_check = Some(now - chrono::Duration::seconds(60));
        let active = super::repo_memory_mcp::RepoMemoryMcpSessionState {
            active: true,
            last_started_at: Some(now - chrono::Duration::minutes(30)),
            last_checked_at: stale_check,
            supervision_status: Some("verified_active".to_string()),
            supervisor_pid: Some(current_pid),
        };

        assert!(super::repo_memory_mcp::repo_memory_mcp_supervision_due(
            &active,
            Some(true),
            current_pid,
            now
        ));

        let fresh = super::repo_memory_mcp::RepoMemoryMcpSessionState {
            last_checked_at: fresh_check,
            ..active.clone()
        };
        assert!(!super::repo_memory_mcp::repo_memory_mcp_supervision_due(
            &fresh,
            Some(true),
            current_pid,
            now
        ));

        let previous_process = super::repo_memory_mcp::RepoMemoryMcpSessionState {
            supervisor_pid: Some(current_pid + 1),
            ..active.clone()
        };
        assert!(super::repo_memory_mcp::repo_memory_mcp_supervision_due(
            &previous_process,
            Some(true),
            current_pid,
            now
        ));

        let not_configured = super::repo_memory_mcp::RepoMemoryMcpSessionState { ..active.clone() };
        assert!(!super::repo_memory_mcp::repo_memory_mcp_supervision_due(
            &not_configured,
            Some(false),
            current_pid,
            now
        ));
    }

    #[test]
    fn headroom_stats_snapshot_records_measured_attribution() {
        let mut tracker = make_tracker();
        let stats = HeadroomDashboardStats {
            session_requests: Some(4),
            session_estimated_savings_usd: Some(0.0),
            session_estimated_tokens_saved: Some(2_500),
            session_savings_pct: Some(25.0),
            session_actual_cost_usd: Some(0.0),
            session_total_tokens_sent: Some(7_500),
            savings_history: Vec::new(),
            output_reduction: None,
        };

        super::maybe_append_measured_headroom_attribution(&mut tracker, &stats)
            .expect("record measured event");
        let events = tracker.attribution_events();
        let event = events.last().expect("measured event");

        assert_eq!(event.source, SavingsAttributionSource::HeadroomEngine);
        assert_eq!(event.confidence, SavingsAttributionConfidence::Measured);
        assert_eq!(event.delta_tokens_saved, 2_500);
        assert_eq!(event.total_tokens_sent, 7_500);
        assert_eq!(event.request_delta, 4);
        assert!(event
            .evidence
            .join(" ")
            .contains("10000 before to 7500 after"));
    }

    #[test]
    fn headroom_stats_snapshot_skips_measured_attribution_without_real_counts() {
        let mut tracker = make_tracker();
        let stats = HeadroomDashboardStats {
            session_requests: Some(0),
            session_estimated_savings_usd: Some(0.0),
            session_estimated_tokens_saved: Some(2_500),
            session_savings_pct: Some(25.0),
            session_actual_cost_usd: Some(0.0),
            session_total_tokens_sent: Some(7_500),
            savings_history: Vec::new(),
            output_reduction: None,
        };

        super::maybe_append_measured_headroom_attribution(&mut tracker, &stats)
            .expect("skip measured event");
        assert!(tracker.attribution_events().is_empty());
    }
}
