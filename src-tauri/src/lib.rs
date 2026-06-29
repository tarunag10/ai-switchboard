mod activity_facts;
mod analytics;
mod backend_port;
mod bearer;
mod claude_cli;
mod client_adapters;
mod device;
mod insights;
mod keychain;
mod local_mode;
mod logging;
mod memory_scrubber;
mod message_logging;
mod models;
mod port_conflict;
mod pricing;
mod proxy_intercept;
mod repo_intelligence;
mod state;
mod storage;
mod tool_manager;

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;
use tauri::{
    AppHandle, PhysicalPosition, PhysicalSize, Position, Rect, State, Window, WindowEvent,
};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::models::{
    ActivityFeedResponse, BillingPeriod, BootstrapProgress, ClaudeAccountProfile,
    ClaudeCodeProject, ClaudeUsage, ClientConnectorStatus, ClientSetupResult,
    ClientSetupVerification, CodexDbRestoreResult, CodexThreadRetaggingSettings, DailySavingsPoint,
    DashboardState, HeadroomAuthCodeRequest, HeadroomLearnPrereqStatus, HeadroomLearnStatus,
    HeadroomPricingStatus, HeadroomSubscriptionTier, ManagedConfigApplyPreview,
    ManagedConfigApplyResult, ManagedFootprintReport, ManagedRollbackExecutionResult,
    ManagedRollbackPreview, ManagedRollbackUndoAllExecutionResult, ManagedRollbackUndoAllPreview,
    MessageLoggingSettings, PurgeResult, RepoAgentHandoffResponse, RepoContextPackResponse,
    RepoDependentsResponse, RepoIndexFreshnessResponse, RepoIntelligenceManifestResponse,
    RepoIntelligenceSummary, RepoSymbolSearchResponse, RuntimeStatus, RuntimeUpgradeProgress,
    SavingsAttributionEvent, SavingsMode, SwitchboardMode, SwitchboardState,
    TransformationFeedResponse, UninstallDryRunReport,
};
use crate::state::AppState;

const UPDATER_PUBLIC_KEY: Option<&str> = option_env!("HEADROOM_UPDATER_PUBLIC_KEY");
const UPDATER_ENDPOINTS: Option<&str> = option_env!("HEADROOM_UPDATER_ENDPOINTS");
const UPDATER_STAGING_ENDPOINTS: Option<&str> = option_env!("HEADROOM_UPDATER_STAGING_ENDPOINTS");
const SENTRY_DSN: Option<&str> = option_env!("HEADROOM_SENTRY_DSN");
const BETA_CHANNEL_ENV: &str = "HEADROOM_BETA_CHANNEL";
const BETA_CHANNEL_SENTINEL: &str = "beta_channel";
const AUTOSTART_LAUNCH_ARG: &str = "--autostart";
const HEADROOM_DASHBOARD_URL: &str = "http://127.0.0.1:6767/dashboard";
const MAIN_WINDOW_WIDTH: u32 = 760;
const MAIN_WINDOW_HEIGHT: u32 = 560;
const TRAY_WINDOW_VERTICAL_GAP: i32 = 10;
const MAIN_WINDOW_BLUR_HIDE_DELAY_MS: u64 = 150;

type InstallPendingUpdateFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuitSource {
    SettingsButton,
    TrayMenu,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectorSmokeTestResult {
    client_id: String,
    supported: bool,
    launched: bool,
    success: bool,
    summary: String,
    stdout_tail: String,
    stderr_tail: String,
}

impl QuitSource {
    fn label(self) -> &'static str {
        match self {
            Self::SettingsButton => "settings_button",
            Self::TrayMenu => "tray_menu",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "phase")]
enum AppUpdateProgress {
    #[serde(rename = "downloading")]
    Downloading { downloaded: u64, total: Option<u64> },
    #[serde(rename = "installing")]
    Installing,
}

const APP_UPDATE_PROGRESS_EVENT: &str = "app-update://progress";

type AppUpdateProgressEmitter = Arc<dyn Fn(AppUpdateProgress) + Send + Sync + 'static>;

#[cfg(test)]
fn noop_app_update_progress_emitter() -> AppUpdateProgressEmitter {
    Arc::new(|_| {})
}

trait InstallableAppUpdate: Send {
    fn metadata(&self) -> AvailableAppUpdate;
    fn install(self, progress: AppUpdateProgressEmitter) -> InstallPendingUpdateFuture;
}

struct TauriPendingUpdate(Update);

impl InstallableAppUpdate for TauriPendingUpdate {
    fn metadata(&self) -> AvailableAppUpdate {
        let published_at = self.0.date.as_ref().and_then(|date| {
            date.format(&time::format_description::well_known::Rfc3339)
                .ok()
        });

        AvailableAppUpdate {
            current_version: self.0.current_version.clone(),
            version: self.0.version.clone(),
            published_at,
            notes: self.0.body.clone(),
        }
    }

    fn install(self, progress: AppUpdateProgressEmitter) -> InstallPendingUpdateFuture {
        Box::pin(async move {
            let downloaded = Arc::new(AtomicU64::new(0));
            let on_chunk_downloaded = Arc::clone(&downloaded);
            let on_chunk_progress = Arc::clone(&progress);
            let on_finish_progress = Arc::clone(&progress);
            self.0
                .download_and_install(
                    move |chunk_len, content_length| {
                        let total = on_chunk_downloaded
                            .fetch_add(chunk_len as u64, Ordering::Relaxed)
                            + chunk_len as u64;
                        on_chunk_progress(AppUpdateProgress::Downloading {
                            downloaded: total,
                            total: content_length,
                        });
                    },
                    move || {
                        on_finish_progress(AppUpdateProgress::Installing);
                    },
                )
                .await
                .map_err(|err| err.to_string())
        })
    }
}

struct PendingAppUpdate(Mutex<Option<TauriPendingUpdate>>);

#[derive(Debug, Clone)]
struct ReleaseUpdaterConfig {
    pubkey: String,
    endpoints: Vec<reqwest::Url>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AppUpdateConfiguration {
    enabled: bool,
    current_version: String,
    endpoint_count: usize,
    configuration_error: Option<String>,
    beta_channel_enabled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AvailableAppUpdate {
    current_version: String,
    version: String,
    published_at: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ReleaseReadinessReportPayload {
    report_path: String,
    report: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ReleaseEvidenceCommandResult {
    command_id: String,
    label: String,
    command: String,
    summary_path: Option<String>,
    stdout: String,
    stderr: String,
}

struct ReleaseEvidenceCommandSpec {
    label: &'static str,
    command: &'static str,
    steps: &'static [(&'static str, &'static [&'static str])],
    summary_path: Option<&'static str>,
}

static ZERO_SPEND_ALERT_FIRED: AtomicBool = AtomicBool::new(false);

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

// Spend fields (actual_cost_usd, total_tokens_sent) were added to SavingsRecord in
// schema v6, shipped in 0.2.40 on 2026-04-13. Records written before that date
// deserialize those fields as 0 via #[serde(default)], producing false positives.
const SPEND_SCHEMA_CUTOFF_DATE: &str = "2026-04-13";

// Trigger on compression *dollar* savings, not the all-layers token total.
// `estimated_tokens_saved` folds in CLI context-tool filtering (RTK / lean-ctx),
// whose tokens are avoided before they ever reach a model request -- so they
// legitimately produce savings with zero tokens_sent and zero cost, tripping
// this probe on days dominated by that layer. `estimated_savings_usd` is
// proxy-compression-only (the proxy prices it at the model rate and excludes CLI
// filtering and prefix-cache discounts), so it is > 0 iff a real model request
// was compressed -- which implies tokens were sent and a cost incurred. Zero
// spend against it is the genuine pipeline anomaly.
fn zero_spend_affected_days(daily_savings: &[DailySavingsPoint]) -> Vec<&str> {
    daily_savings
        .iter()
        .filter(|p| {
            p.date.as_str() >= SPEND_SCHEMA_CUTOFF_DATE
                && p.estimated_savings_usd > 0.000_001
                && p.actual_cost_usd == 0.0
                && p.total_tokens_sent == 0
        })
        .map(|p| p.date.as_str())
        .collect()
}

fn check_zero_spend_anomaly(dashboard: &DashboardState) {
    if ZERO_SPEND_ALERT_FIRED.load(Ordering::Relaxed) {
        return;
    }
    let affected_days = zero_spend_affected_days(&dashboard.daily_savings);
    if affected_days.is_empty() {
        return;
    }
    ZERO_SPEND_ALERT_FIRED.store(true, Ordering::Relaxed);
    sentry::capture_message(
        &format!(
            "graph shows compression savings but zero tokens spent on days: {}",
            affected_days.join(", ")
        ),
        sentry::Level::Warning,
    );
}

#[tauri::command]
fn build_repo_intelligence_summary(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<RepoIntelligenceSummary, String> {
    let summary = repo_intelligence::summarize_repo(repo_path).map_err(|err| err.to_string())?;
    repo_intelligence::save_latest_summary(&summary).map_err(|err| err.to_string())?;
    if let Err(err) = state.record_repo_intelligence_attribution(&summary) {
        log::warn!("could not record Repo Intelligence attribution event: {err:#}");
    }
    Ok(summary)
}

#[tauri::command]
fn get_latest_repo_intelligence_summary() -> Result<Option<RepoIntelligenceSummary>, String> {
    repo_intelligence::load_latest_summary().map_err(|err| err.to_string())
}

#[tauri::command]
fn clear_repo_intelligence_summary() -> Result<bool, String> {
    repo_intelligence::clear_latest_summary().map_err(|err| err.to_string())
}

#[tauri::command]
fn get_repo_intelligence_context_pack(
    pack_id: Option<String>,
) -> Result<Option<RepoContextPackResponse>, String> {
    repo_intelligence::latest_context_pack(pack_id.as_deref()).map_err(|err| err.to_string())
}

#[tauri::command]
fn search_repo_intelligence_symbols(
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Option<RepoSymbolSearchResponse>, String> {
    repo_intelligence::latest_symbol_search(query.as_deref(), limit).map_err(|err| err.to_string())
}

#[tauri::command]
fn get_repo_intelligence_dependents(
    target: String,
    limit: Option<usize>,
) -> Result<Option<RepoDependentsResponse>, String> {
    repo_intelligence::latest_dependents_search(&target, limit).map_err(|err| err.to_string())
}

#[tauri::command]
fn get_repo_intelligence_manifest() -> Result<Option<RepoIntelligenceManifestResponse>, String> {
    repo_intelligence::latest_manifest().map_err(|err| err.to_string())
}

#[tauri::command]
fn get_repo_manifest() -> Result<Option<RepoIntelligenceManifestResponse>, String> {
    repo_intelligence::latest_manifest().map_err(|err| err.to_string())
}

#[tauri::command]
fn get_repo_pack(pack_id: Option<String>) -> Result<Option<RepoContextPackResponse>, String> {
    repo_intelligence::latest_context_pack(pack_id.as_deref()).map_err(|err| err.to_string())
}

#[tauri::command]
fn get_agent_handoff(
    agent_id: String,
    task_type: Option<String>,
) -> Result<Option<RepoAgentHandoffResponse>, String> {
    repo_intelligence::latest_agent_handoff(&agent_id, task_type.as_deref())
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn get_index_freshness() -> Result<RepoIndexFreshnessResponse, String> {
    repo_intelligence::latest_index_freshness().map_err(|err| err.to_string())
}

#[tauri::command]
fn clear_repo_index() -> Result<bool, String> {
    repo_intelligence::clear_latest_summary().map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_dashboard_state(app: AppHandle) -> Result<DashboardState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        let (dashboard, pending_milestones) = state.dashboard_with_pending_milestones();

        for milestone_tokens_saved in &pending_milestones.token {
            analytics::track_event(
                &app,
                "lifetime_tokens_saved_milestone_reached",
                Some(json!({
                    "milestone_tokens_saved": *milestone_tokens_saved,
                    "milestone_millions": milestone_tokens_saved / 1_000_000,
                    "milestone_kind": lifetime_token_milestone_kind(*milestone_tokens_saved),
                    "lifetime_tokens_saved": dashboard.lifetime_estimated_tokens_saved,
                    "lifetime_requests": dashboard.lifetime_requests,
                    "launch_count": state.launch_count(),
                    "launch_experience": state.launch_experience_label()
                })),
            );
            pricing::report_milestone(*milestone_tokens_saved);
        }

        check_zero_spend_anomaly(&dashboard);

        dashboard
    })
    .await
    .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_savings_attribution_events(
    app: AppHandle,
) -> Result<Vec<SavingsAttributionEvent>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        state.savings_attribution_events()
    })
    .await
    .map_err(|err| err.to_string())
}

#[tauri::command]
fn preview_managed_rollback(record_id: String) -> Result<ManagedRollbackPreview, String> {
    client_adapters::preview_managed_rollback(&record_id).map_err(|err| err.to_string())
}

#[tauri::command]
fn execute_managed_rollback(
    record_id: String,
    backup_path: String,
    confirmation_phrase: String,
) -> Result<ManagedRollbackExecutionResult, String> {
    client_adapters::execute_managed_rollback(&record_id, &backup_path, &confirmation_phrase)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn preview_managed_config_apply(record_id: String) -> Result<ManagedConfigApplyPreview, String> {
    client_adapters::preview_managed_config_apply(&record_id).map_err(|err| err.to_string())
}

#[tauri::command]
fn execute_managed_config_apply(
    record_id: String,
    confirmation_phrase: String,
) -> Result<ManagedConfigApplyResult, String> {
    client_adapters::execute_managed_config_apply(&record_id, &confirmation_phrase)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn preview_managed_rollback_undo_all() -> ManagedRollbackUndoAllPreview {
    client_adapters::preview_managed_rollback_undo_all()
}

#[tauri::command]
fn execute_managed_rollback_undo_all(
    confirmation_phrase: String,
) -> Result<ManagedRollbackUndoAllExecutionResult, String> {
    client_adapters::execute_managed_rollback_undo_all(&confirmation_phrase)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn get_managed_footprint() -> ManagedFootprintReport {
    client_adapters::get_managed_footprint()
}

#[tauri::command]
fn get_uninstall_dry_run_report() -> UninstallDryRunReport {
    client_adapters::uninstall_dry_run_report()
}

#[tauri::command]
fn get_app_update_configuration(app: AppHandle) -> AppUpdateConfiguration {
    let current_version = app.package_info().version.to_string();
    let beta_channel_enabled = beta_channel_enabled();
    match release_updater_config(&current_version, beta_channel_enabled) {
        Ok(Some(config)) => AppUpdateConfiguration {
            enabled: true,
            current_version,
            endpoint_count: config.endpoints.len(),
            configuration_error: None,
            beta_channel_enabled,
        },
        Ok(None) => AppUpdateConfiguration {
            enabled: false,
            current_version,
            endpoint_count: 0,
            configuration_error: None,
            beta_channel_enabled,
        },
        Err(ref err) => {
            sentry::capture_message(
                &format!("app update configuration error: {err}"),
                sentry::Level::Error,
            );
            AppUpdateConfiguration {
                enabled: false,
                current_version,
                endpoint_count: 0,
                configuration_error: Some(err.clone()),
                beta_channel_enabled,
            }
        }
    }
}

fn load_release_readiness_report_from(
    path: &Path,
) -> Result<ReleaseReadinessReportPayload, String> {
    let report_path = path.to_string_lossy().into_owned();
    match std::fs::read_to_string(path) {
        Ok(raw) => {
            let report = serde_json::from_str(&raw)
                .map_err(|err| format!("release readiness report is invalid JSON: {err}"))?;
            Ok(ReleaseReadinessReportPayload {
                report_path,
                report: Some(report),
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok(ReleaseReadinessReportPayload {
                report_path,
                report: None,
            })
        }
        Err(err) => Err(format!("failed to read release readiness report: {err}")),
    }
}

#[tauri::command]
fn load_release_readiness_report() -> Result<ReleaseReadinessReportPayload, String> {
    let path = std::env::current_dir()
        .map_err(|err| err.to_string())?
        .join("dist/release-readiness-report.json");
    load_release_readiness_report_from(&path)
}

#[tauri::command]
fn refresh_release_readiness_report() -> Result<ReleaseReadinessReportPayload, String> {
    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    let output = Command::new("npm")
        .args(["run", "release:ready", "--", "--json"])
        .current_dir(&cwd)
        .output()
        .map_err(|err| format!("failed to run npm run release:ready: {err}"))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = [stdout, stderr]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(if detail.is_empty() {
            format!("npm run release:ready failed with status {}", output.status)
        } else {
            format!(
                "npm run release:ready failed with status {}:\n{}",
                output.status, detail
            )
        });
    }

    load_release_readiness_report_from(&cwd.join("dist/release-readiness-report.json"))
}

#[tauri::command]
fn run_release_evidence_command(
    command_id: String,
) -> Result<ReleaseEvidenceCommandResult, String> {
    const STATIC_PREFLIGHT_STEPS: &[(&str, &[&str])] = &[("npm", &["run", "smoke:preflight"])];
    const DESKTOP_VALIDATION_STEPS: &[(&str, &[&str])] = &[
        ("npm", &["run", "fmt:desktop"]),
        ("npm", &["run", "test:desktop"]),
    ];
    const LOCAL_INSTALLED_SMOKE_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:installed:local"])];
    const LOCAL_MODE_RELAUNCH_SMOKE_STEPS: &[(&str, &[&str])] = &[(
        "npm",
        &["run", "smoke:mode-relaunch:local", "--", "--confirm"],
    )];
    const ROLLBACK_CENTER_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:rollback:local"])];
    const DOCTOR_REPAIR_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:doctor-repair:local"])];
    const UNINSTALL_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:uninstall:local"])];
    const REPO_INTELLIGENCE_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:repo-intelligence:local"])];
    const LOCAL_DMG_BUILD_INSTALL_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "build:mac:local-install"])];
    const RELEASE_REPORT_STEPS: &[(&str, &[&str])] = &[("npm", &["run", "release:report"])];

    let spec = match command_id.as_str() {
        "static-preflight" => ReleaseEvidenceCommandSpec {
            label: "Static smoke preflight",
            command: "npm run smoke:preflight",
            steps: STATIC_PREFLIGHT_STEPS,
            summary_path: Some("dist/smoke-preflight-summary.md"),
        },
        "desktop-validation" => ReleaseEvidenceCommandSpec {
            label: "Desktop validation",
            command: "npm run fmt:desktop && npm run test:desktop",
            steps: DESKTOP_VALIDATION_STEPS,
            summary_path: None,
        },
        "local-installed-smoke" => ReleaseEvidenceCommandSpec {
            label: "Local installed smoke",
            command: "npm run smoke:installed:local",
            steps: LOCAL_INSTALLED_SMOKE_STEPS,
            summary_path: Some("dist/local-installed-smoke-summary.md"),
        },
        "local-mode-relaunch-smoke" => ReleaseEvidenceCommandSpec {
            label: "Local mode relaunch smoke",
            command: "npm run smoke:mode-relaunch:local -- --confirm",
            steps: LOCAL_MODE_RELAUNCH_SMOKE_STEPS,
            summary_path: Some("dist/local-mode-relaunch-smoke-summary.md"),
        },
        "rollback-center-validation" => ReleaseEvidenceCommandSpec {
            label: "Rollback Center validation",
            command: "npm run smoke:rollback:local",
            steps: ROLLBACK_CENTER_VALIDATION_STEPS,
            summary_path: Some("dist/local-rollback-validation-summary.md"),
        },
        "doctor-repair-validation" => ReleaseEvidenceCommandSpec {
            label: "Doctor repair validation",
            command: "npm run smoke:doctor-repair:local",
            steps: DOCTOR_REPAIR_VALIDATION_STEPS,
            summary_path: Some("dist/local-doctor-repair-validation-summary.md"),
        },
        "uninstall-validation" => ReleaseEvidenceCommandSpec {
            label: "Uninstall dry-run validation",
            command: "npm run smoke:uninstall:local",
            steps: UNINSTALL_VALIDATION_STEPS,
            summary_path: Some("dist/local-uninstall-validation-summary.md"),
        },
        "repo-intelligence-validation" => ReleaseEvidenceCommandSpec {
            label: "Repo Intelligence validation",
            command: "npm run smoke:repo-intelligence:local",
            steps: REPO_INTELLIGENCE_VALIDATION_STEPS,
            summary_path: Some("dist/local-repo-intelligence-validation-summary.md"),
        },
        "local-dmg-build-install" => ReleaseEvidenceCommandSpec {
            label: "Local DMG build/install",
            command: "npm run build:mac:local-install",
            steps: LOCAL_DMG_BUILD_INSTALL_STEPS,
            summary_path: Some("dist/local-installed-smoke-summary.md"),
        },
        "release-report" => ReleaseEvidenceCommandSpec {
            label: "Release readiness report",
            command: "npm run release:report",
            steps: RELEASE_REPORT_STEPS,
            summary_path: Some("dist/release-readiness-report.md"),
        },
        _ => {
            return Err(
                "Release evidence execution is currently enabled only for static-preflight, desktop-validation, local-dmg-build-install, local-installed-smoke, local-mode-relaunch-smoke, rollback-center-validation, doctor-repair-validation, uninstall-validation, repo-intelligence-validation, and release-report."
                    .to_string(),
            )
        }
    };

    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    let mut combined_stdout = Vec::new();
    let mut combined_stderr = Vec::new();
    for (program, args) in spec.steps {
        let step_label = format!("{} {}", program, args.join(" "));
        let output = Command::new(program)
            .args(*args)
            .current_dir(&cwd)
            .output()
            .map_err(|err| format!("failed to run {step_label}: {err}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        combined_stdout.push(format!("$ {step_label}\n{stdout}"));
        if !stderr.trim().is_empty() {
            combined_stderr.push(format!("$ {step_label}\n{stderr}"));
        }
        if !output.status.success() {
            let detail = [stdout.trim(), stderr.trim()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(if detail.is_empty() {
                format!("{step_label} failed with status {}", output.status)
            } else {
                format!(
                    "{step_label} failed with status {}:\n{}",
                    output.status, detail
                )
            });
        }
    }

    Ok(ReleaseEvidenceCommandResult {
        command_id,
        label: spec.label.to_string(),
        command: spec.command.to_string(),
        summary_path: spec.summary_path.map(str::to_string),
        stdout: combined_stdout.join("\n"),
        stderr: combined_stderr.join("\n"),
    })
}

#[tauri::command]
async fn check_for_app_update(
    app: AppHandle,
    pending_update: State<'_, PendingAppUpdate>,
) -> Result<Option<AvailableAppUpdate>, String> {
    let current_version = app.package_info().version.to_string();
    let config = release_updater_config(&current_version, beta_channel_enabled())?
        .ok_or_else(|| "Update checks are not configured in this build.".to_string())?;

    let updater = app
        .updater_builder()
        .pubkey(config.pubkey)
        .endpoints(config.endpoints)
        .map_err(|err| err.to_string())?
        .build()
        .map_err(|err| err.to_string())?;

    let checked_update = updater
        .check()
        .await
        .map(|update| update.map(TauriPendingUpdate))
        .map_err(|err| err.to_string());

    store_checked_update(checked_update, &pending_update.0)
}

#[tauri::command]
async fn install_app_update(
    app: AppHandle,
    pending_update: State<'_, PendingAppUpdate>,
) -> Result<(), String> {
    let emitter_app = app.clone();
    let emitter: AppUpdateProgressEmitter = Arc::new(move |event| {
        let _ = emitter_app.emit(APP_UPDATE_PROGRESS_EVENT, &event);
    });
    install_pending_update(&pending_update.0, emitter).await
}

fn store_checked_update<U>(
    checked_update: Result<Option<U>, String>,
    pending_update: &Mutex<Option<U>>,
) -> Result<Option<AvailableAppUpdate>, String>
where
    U: InstallableAppUpdate,
{
    let update = checked_update?;
    let mut pending = pending_update.lock();

    if let Some(update) = update {
        let metadata = update.metadata();
        *pending = Some(update);
        Ok(Some(metadata))
    } else {
        *pending = None;
        Ok(None)
    }
}

async fn install_pending_update<U>(
    pending_update: &Mutex<Option<U>>,
    progress: AppUpdateProgressEmitter,
) -> Result<(), String>
where
    U: InstallableAppUpdate,
{
    let update = {
        let mut pending = pending_update.lock();
        pending
            .take()
            .ok_or_else(|| "No downloaded update is ready to install.".to_string())?
    };

    update.install(progress).await
}

#[tauri::command]
fn restart_app(app: AppHandle) {
    // Tauri 2.x has an open bug on macOS (tauri-apps/tauri#13923, #11392)
    // where `request_restart()` and `restart()` exit the process but never
    // relaunch — especially with `tauri-plugin-single-instance` loaded.
    // Workaround: spawn a detached relauncher via `open -n` against this
    // app's .app bundle (which is in-place updated by the updater).
    //
    // The relauncher is armed BEFORE the teardown below, because that teardown
    // can block the main thread for a long time (stop_headroom() does a
    // `child.wait()` with no timeout on the Python backend, and
    // analytics::shutdown() joins a worker whose last act is a network flush) —
    // observed as "Headroom is not responding". A previous version used a blind
    // `sleep 1` before `open -n`, which raced that teardown: if we hadn't
    // exited (and released the single-instance lock) within 1s, the new
    // instance saw the lock held, focused the dying old window, and bailed —
    // so the app was killed but never came back.
    //
    // Instead the relauncher waits for THIS pid to actually die (lock released)
    // before launching, and force-kills us after a deadline if teardown truly
    // deadlocks, so the lock is always freed and the new instance can boot.
    #[cfg(target_os = "macos")]
    {
        match current_app_bundle_path() {
            Some(bundle) => {
                let pid = std::process::id();
                let quoted = shell_quote_path(&bundle);
                // The relauncher runs AFTER this process exits, so the Rust
                // logger is gone by the time `open` runs. Have the script append
                // its own outcome (open's exit code) to the desktop log so a
                // field failure is diagnosable instead of silent. A non-zero rc
                // points at the launch itself (Gatekeeper, App Translocation,
                // a missing/stale bundle); rc=0 with no relaunch points at the
                // freshly-installed build crashing on its own startup.
                let log_quoted = shell_quote_path(&logging::log_path());
                log::info!("restart_app: relaunching via `open -n` against bundle {bundle:?}");
                let cmd = format!(
                    "alive=1; \
                     for i in $(seq 1 100); do \
                       if ! kill -0 {pid} 2>/dev/null; then alive=0; break; fi; \
                       sleep 0.1; \
                     done; \
                     if [ \"$alive\" = 1 ]; then kill -9 {pid} 2>/dev/null; sleep 0.5; fi; \
                     /usr/bin/open -n {quoted}; rc=$?; \
                     echo \"$(date '+%Y-%m-%d %H:%M:%S') relauncher: open -n {quoted} exited rc=$rc (alive=$alive)\" >> {log_quoted}",
                    pid = pid,
                    quoted = quoted,
                    log_quoted = log_quoted,
                );
                match Command::new("/bin/sh").arg("-c").arg(cmd).spawn() {
                    Ok(_) => log::info!("restart_app: relauncher spawned"),
                    Err(err) => log::error!("restart_app: failed to spawn relauncher: {err}"),
                }
            }
            None => {
                // No enclosing .app bundle (dev build, or an app launched from a
                // path with no `.app` ancestor). `open -n` has nothing to target;
                // the app will quit without relaunching.
                log::error!(
                    "restart_app: current_app_bundle_path() returned None (current_exe={:?}); cannot relaunch",
                    std::env::current_exe()
                );
            }
        }
    }

    // Stop the proxy before relaunching so the new build starts a fresh proxy
    // with current args (otherwise the orphan keeps serving traffic and the
    // new desktop reuses it via the reachability check). Without this, any
    // proxy-arg change shipped by an upgrade silently never takes effect.
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.stop_headroom();
    }
    analytics::shutdown(&app);

    #[cfg(target_os = "macos")]
    {
        app.exit(0);
        return;
    }

    #[cfg(not(target_os = "macos"))]
    {
        app.request_restart();
    }
}

/// Walks up from `current_exe` to find the enclosing `.app` bundle path.
#[cfg(target_os = "macos")]
fn current_app_bundle_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .find(|p| p.extension().is_some_and(|ext| ext == "app"))
        .map(|p| p.to_path_buf())
}

#[cfg(target_os = "macos")]
fn shell_quote_path(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    // POSIX single-quote escaping: anything inside '...' is literal except
    // ', which we close-escape-open. Safe against spaces / special chars in
    // the bundle path (e.g. `/Applications/Headroom RC.app`).
    format!("'{}'", s.replace('\'', "'\\''"))
}

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
    let bundle = current_app_bundle_path()?;

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
    let quoted = shell_quote_path(&bundle);
    let log_quoted = shell_quote_path(&logging::log_path());
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
fn show_app_update_notification(app: AppHandle, version: String) -> Result<(), String> {
    show_app_update_notification_impl(&app, &version)
}

fn app_update_notification_body(version: &str) -> String {
    let trimmed = version.trim();
    let lead = if trimmed.is_empty() {
        "A Mac AI Switchboard update is ready to install.".to_string()
    } else {
        format!("Mac AI Switchboard {trimmed} is ready to install.")
    };

    format!("{lead} Open Mac AI Switchboard to review the release and install it.")
}

fn show_app_update_notification_impl(app: &AppHandle, version: &str) -> Result<(), String> {
    let body = app_update_notification_body(version);
    show_notification_impl(
        app,
        "Mac AI Switchboard Update Available",
        &body,
        Some("update".into()),
    )
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
fn show_notification_impl(
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
fn show_notification_impl(
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

#[tauri::command]
async fn install_addon(state: State<'_, AppState>, id: String) -> Result<DashboardState, String> {
    match id.as_str() {
        "markitdown" => {
            state
                .tool_manager
                .install_markitdown()
                .map_err(|err| err.to_string())?;
            let (changed_files, backup_files) = client_adapters::enable_markitdown_integration(
                &state.tool_manager.markitdown_entrypoint(),
                &state.tool_manager.markitdown_shim_path(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| {
                format!("markitdown installed but enabling integration failed: {err:#}")
            })?;
            let _ = state.record_markitdown_attribution(&changed_files, &backup_files);
            Ok(state.dashboard())
        }
        "rtk" => {
            state
                .tool_manager
                .install_rtk()
                .map_err(|err| err.to_string())?;
            client_adapters::set_rtk_enabled(
                true,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| format!("rtk installed but enabling integration failed: {err:#}"))?;
            Ok(state.dashboard())
        }
        "ponytail" => {
            state
                .tool_manager
                .install_ponytail()
                .map_err(|err| err.to_string())?;
            let hosts = state.tool_manager.ponytail_registered_hosts();
            let _ = state.record_ponytail_attribution(&hosts);
            Ok(state.dashboard())
        }
        "caveman" => {
            state
                .tool_manager
                .install_caveman()
                .map_err(|err| err.to_string())?;
            let level = state.tool_manager.caveman_level();
            let (changed_files, backup_files) = client_adapters::enable_caveman_integration(&level)
                .map_err(|err| {
                    format!("caveman installed but enabling guidance failed: {err:#}")
                })?;
            let _ = state.record_caveman_attribution(&level, &changed_files, &backup_files);
            Ok(state.dashboard())
        }
        other => Err(format!("unknown addon: {other}")),
    }
}

#[tauri::command]
async fn set_addon_enabled(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<DashboardState, String> {
    match id.as_str() {
        "markitdown" => {
            state
                .tool_manager
                .set_markitdown_enabled(enabled)
                .map_err(|err| err.to_string())?;
            if enabled {
                let (changed_files, backup_files) = client_adapters::enable_markitdown_integration(
                    &state.tool_manager.markitdown_entrypoint(),
                    &state.tool_manager.markitdown_shim_path(),
                    &state.tool_manager.managed_python(),
                )
                .map_err(|err| err.to_string())?;
                let _ = state.record_markitdown_attribution(&changed_files, &backup_files);
            } else {
                client_adapters::disable_markitdown_integration(
                    &state.tool_manager.markitdown_shim_path(),
                )
                .map_err(|err| err.to_string())?;
            }
            Ok(state.dashboard())
        }
        "ponytail" => {
            state
                .tool_manager
                .set_ponytail_enabled(enabled)
                .map_err(|err| err.to_string())?;
            if enabled {
                let hosts = state.tool_manager.ponytail_registered_hosts();
                let _ = state.record_ponytail_attribution(&hosts);
            }
            Ok(state.dashboard())
        }
        "caveman" => {
            state
                .tool_manager
                .set_caveman_enabled(enabled)
                .map_err(|err| err.to_string())?;
            if enabled {
                let level = state.tool_manager.caveman_level();
                let (changed_files, backup_files) =
                    client_adapters::enable_caveman_integration(&level)
                        .map_err(|err| err.to_string())?;
                let _ = state.record_caveman_attribution(&level, &changed_files, &backup_files);
            } else {
                client_adapters::disable_caveman_integration().map_err(|err| err.to_string())?;
            }
            Ok(state.dashboard())
        }
        other => Err(format!("unknown addon: {other}")),
    }
}

#[tauri::command]
async fn uninstall_addon(state: State<'_, AppState>, id: String) -> Result<DashboardState, String> {
    match id.as_str() {
        "markitdown" => {
            let _ = client_adapters::disable_markitdown_integration(
                &state.tool_manager.markitdown_shim_path(),
            );
            state
                .tool_manager
                .uninstall_markitdown()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "rtk" => {
            client_adapters::set_rtk_enabled(
                false,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state
                .tool_manager
                .uninstall_rtk()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "ponytail" => {
            state
                .tool_manager
                .uninstall_ponytail()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        "caveman" => {
            let _ = client_adapters::disable_caveman_integration();
            state
                .tool_manager
                .uninstall_caveman()
                .map_err(|err| err.to_string())?;
            Ok(state.dashboard())
        }
        other => Err(format!("unknown addon: {other}")),
    }
}

#[tauri::command]
async fn set_caveman_level(
    state: State<'_, AppState>,
    level: String,
) -> Result<DashboardState, String> {
    state
        .tool_manager
        .set_caveman_level(&level)
        .map_err(|err| err.to_string())?;
    // Rewrite the managed blocks with the new level body when enabled.
    if state.tool_manager.caveman_receipt_exists()
        && state
            .tool_manager
            .list_tools()
            .iter()
            .any(|tool| tool.id == "caveman" && tool.enabled)
    {
        let level = state.tool_manager.caveman_level();
        let (changed_files, backup_files) =
            client_adapters::enable_caveman_integration(&level).map_err(|err| err.to_string())?;
        let _ = state.record_caveman_attribution(&level, &changed_files, &backup_files);
    }
    Ok(state.dashboard())
}

#[tauri::command]
fn install_repo_memory_mcp(state: State<'_, AppState>) -> Result<DashboardState, String> {
    state
        .tool_manager
        .install_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    Ok(state.dashboard())
}

#[tauri::command]
fn start_repo_memory_mcp(state: State<'_, AppState>) -> Result<DashboardState, String> {
    state
        .start_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    Ok(state.dashboard())
}

#[tauri::command]
fn stop_repo_memory_mcp(state: State<'_, AppState>) -> Result<DashboardState, String> {
    state
        .stop_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    Ok(state.dashboard())
}

#[tauri::command]
fn bootstrap_runtime(state: State<'_, AppState>) -> Result<DashboardState, String> {
    state
        .tool_manager
        .bootstrap_all()
        .map_err(|err| err.to_string())?;

    if saved_switchboard_mode_wants_rtk() {
        if let Err(err) = client_adapters::ensure_rtk_integrations(
            &state.tool_manager.rtk_entrypoint(),
            &state.tool_manager.managed_python(),
        ) {
            log::warn!("RTK integrations failed after bootstrap_runtime: {err:#}");
        }
    }

    if !saved_switchboard_mode_wants_headroom() {
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

fn emit_bootstrap_progress(app: &AppHandle, state: &AppState) {
    let _ = app.emit("bootstrap_progress", state.bootstrap_progress());
}

#[tauri::command]
fn start_bootstrap(app: AppHandle) -> Result<(), String> {
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
        let wants_headroom = saved_switchboard_mode_wants_headroom();

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

            if saved_switchboard_mode_wants_rtk() {
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
            let handled = port_conflict::note_proxy_failed(&app_handle, &err, true);
            if !handled {
                capture_headroom_start_failure("headroom auto-start failed after bootstrap", &err);
            }
            // Fall through so the user is not stuck on the install loader
            // indefinitely. The test screen will show a retry option.
        } else {
            port_conflict::note_proxy_started(&app_handle);
            // The intercept layer on 6767 is always bound by the Rust app, so
            // reachability really means "headroom's backend on 6768 is up".
            // We probe it by hitting 6767/health — the intercept forwards to
            // 6768 and returns 502 until the backend actually responds, so a
            // 2xx confirms the full chain is live. Gatekeeper's first-launch
            // scan of the bundled venv can take 30-60s, so we wait up to 60s
            // to match the ETA shown to the user.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while std::time::Instant::now() < deadline {
                if state::headroom_proxy_reachable() {
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

#[derive(Copy, Clone, Debug)]
enum BootstrapFailureKind {
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
    /// user's environment — it's self-recoverable, so we frame it softly and
    /// the user just needs to click Try again.
    NetworkDownload,
    Other,
}

impl BootstrapFailureKind {
    fn as_str(self) -> &'static str {
        match self {
            BootstrapFailureKind::SslInterception => "ssl_interception",
            BootstrapFailureKind::NoUsableTempDir => "no_usable_tempdir",
            BootstrapFailureKind::NetworkDownload => "network_download",
            BootstrapFailureKind::Other => "other",
        }
    }
}

fn classify_bootstrap_failure(err: &anyhow::Error) -> BootstrapFailureKind {
    // pip/venv failures surface as CommandFailure, where stdout/stderr carry the
    // real signal. Our own reqwest downloads (Python runtime, rtk binary) have no
    // CommandFailure, so fall back to the formatted error chain for those.
    let cmd_failure = err
        .chain()
        .find_map(|e| e.downcast_ref::<tool_manager::CommandFailure>());
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
fn is_network_download_signal(text: &str) -> bool {
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

fn user_message_for(kind: BootstrapFailureKind) -> &'static str {
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
fn capture_bootstrap_failure(err: &anyhow::Error, kind: BootstrapFailureKind) {
    let technical_err = format!("{err:#}");
    let cmd_failure = err
        .chain()
        .find_map(|e| e.downcast_ref::<tool_manager::CommandFailure>());

    // Match against stderr (where the real signal lives for CommandFailure)
    // in addition to the error chain. For non-CommandFailure paths the
    // chain is all we have.
    let endpoint_protection_suspected = is_endpoint_protection_signal(&technical_err)
        || cmd_failure
            .map(|f| is_endpoint_protection_signal(&f.stderr))
            .unwrap_or(false);

    // ENOSPC is environmental; skip the Sentry capture (see notes on
    // `capture_upgrade_failure`).
    let disk_full = is_disk_full_signal(&technical_err)
        || cmd_failure
            .map(|f| is_disk_full_signal(&f.stderr))
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

/// Pure payload for `capture_watchdog_give_up`. Built before any Sentry side
/// effects so it can be unit-tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchdogGiveUpReport {
    pub message: String,
    pub tracked_child_exit_status: String,
    pub bypass_active: bool,
    pub runtime_upgrade_in_progress: bool,
    pub consecutive_failures: u32,
    pub log_tail: Option<String>,
    /// Last error returned by `ensure_headroom_running` during this down
    /// episode, if any. Distinguishes "spawn keeps erroring" (Some) from
    /// "spawn returned Ok but `/readyz` never came back" (None) — the two
    /// failure modes look identical without this field.
    pub last_startup_error: Option<String>,
    /// PID of the tracked Python child at give-up time, if we own a Child
    /// handle. Useful for ad-hoc correlation with external `ps`/Activity
    /// Monitor snapshots the user can attach to a bug report.
    pub tracked_pid: Option<u32>,
    /// Whether the backend loopback port still accepts a TCP connection.
    /// Distinguishes "process gone, port closed" (false) from "process
    /// alive but event loop wedged" (true) — the kernel completes
    /// `accept()` even when uvicorn can't service HTTP. See
    /// `state::tcp_port_accepts_connection` for full semantics.
    pub port_accepts_tcp: bool,
    /// Accumulated CPU seconds for the tracked PID at give-up time.
    /// None when no tracked child or `ps` failed. Combined with
    /// `log_silent_secs`, lets us see whether the child was burning CPU
    /// silently (sync compute) vs idle/blocked (deadlock, await never
    /// resolving).
    pub process_cpu_secs: Option<u64>,
    /// Seconds since the newest `headroom-proxy*.log` file was last
    /// modified. None when there is no proxy log on disk yet, or the
    /// mtime is in the future (clock skew).
    pub log_silent_secs: Option<u64>,
    /// Outcome of probing `/readyz` directly on the backend port at
    /// give-up time. Disambiguates intercept-layer failures (intercept
    /// fails, backend `ok`) from Python-layer failures (both fail).
    /// One of: `ok`, `timeout`, `refused`, `http_<status>`, `error: <msg>`.
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

/// Probe `/readyz` on the backend port directly (bypassing the Rust
/// intercept on 6767) and classify the outcome for inclusion in a
/// give-up Sentry event. 1.5s timeout matches `is_headroom_proxy_reachable`
/// so a `timeout` here corresponds to the same wait the watchdog already
/// experienced.
fn probe_backend_readyz_outcome() -> String {
    probe_backend_readyz_outcome_with_timeout(std::time::Duration::from_millis(1500))
}

/// Same probe as [`probe_backend_readyz_outcome`] but with a caller-chosen
/// timeout. The watchdog uses a longer (5s) budget to confirm a failure before
/// counting a strike, so a niced backend that's merely slow under heavy
/// compression load isn't mistaken for a dead one.
fn probe_backend_readyz_outcome_with_timeout(timeout: std::time::Duration) -> String {
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
                // 503 = readiness failure: the process is alive and answering,
                // but a component check is false. Parse the body's per-check
                // breakdown so the watchdog can tell a transient upstream blip
                // (`http_503:upstream`) apart from a wedged core component and
                // route them differently. Falls back to bare "http_503" when the
                // body can't be read or parsed.
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
/// payload — those whose `checks.<name>.ready` is `false`. Empty when the body
/// has no `checks` object or every check is ready.
fn readyz_failed_checks_csv(body: &serde_json::Value) -> String {
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

/// Failing-check names parsed out of a `http_503:<a>,<b>` outcome string.
/// `None` for any other outcome (including a bare `http_503` whose body
/// couldn't be parsed), so callers treat unknown 503s as the conservative
/// give-up default.
fn parse_readyz_failed_checks(outcome: &str) -> Option<Vec<&str>> {
    outcome
        .strip_prefix("http_503:")
        .map(|rest| rest.split(',').filter(|s| !s.is_empty()).collect())
}

/// True when `/readyz` returned 503 and the *only* unhealthy component is the
/// upstream-connectivity probe. The proxy process is healthy; this is a
/// transient network/upstream blip (the upstream check is cached 30s) that
/// self-heals on the next refresh. Tearing Python down and bypassing routes to
/// the same unreachable upstream, so it buys nothing.
fn readyz_failure_is_upstream_only(outcome: &str) -> bool {
    matches!(parse_readyz_failed_checks(outcome), Some(checks) if checks == ["upstream"])
}

/// True when `/readyz` returned 503 with at least one *core* component
/// (startup, http_client, cache, rate_limiter, memory) unhealthy — a wedged
/// backend that a restart may clear, distinct from a pure upstream blip.
fn readyz_failure_has_core_unhealthy(outcome: &str) -> bool {
    parse_readyz_failed_checks(outcome)
        .map(|checks| checks.iter().any(|c| *c != "upstream"))
        .unwrap_or(false)
}

/// Whether two cumulative CPU samples (`ps -o time=`, whole seconds) taken
/// `elapsed_secs` apart represent a process actively burning CPU. Uses the
/// *rate*, not the delta: `ps` reports whole seconds, so a single incidental
/// tick at a second boundary reads as +1, which over a short window looks like
/// activity. Require >0.5 CPU-sec/sec so a real spin (~1.0) passes while a lone
/// boundary tick (~0.25 over a ~4s window) does not.
fn cpu_rate_indicates_burn(before: u64, after: u64, elapsed_secs: f64) -> bool {
    elapsed_secs > 0.0 && (after.saturating_sub(before) as f64) / elapsed_secs > 0.5
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
    let port_accepts_tcp = crate::state::proxy_port_accepts_connection();
    let process_cpu_secs = tracked_pid.and_then(crate::state::tracked_process_cpu_time_secs);
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
            crate::state::tracked_process_cpu_time_secs(pid)
                .map(|after| cpu_rate_indicates_burn(before, after, elapsed))
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

    let report = build_watchdog_give_up_report(
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
        .find_map(|e| e.downcast_ref::<tool_manager::CommandFailure>());

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

#[tauri::command]
fn get_bootstrap_progress(state: State<'_, AppState>) -> BootstrapProgress {
    state.bootstrap_progress()
}

#[tauri::command]
fn get_runtime_upgrade_progress(state: State<'_, AppState>) -> RuntimeUpgradeProgress {
    state.runtime_upgrade_progress()
}

#[tauri::command]
fn retry_runtime_upgrade(app: AppHandle) -> Result<(), String> {
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
fn retry_runtime_upgrade_with_rebuild(app: AppHandle) -> Result<(), String> {
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app_clone.state();
        state.retry_runtime_upgrade(&app_clone, true);
    });
    Ok(())
}

#[tauri::command]
fn dismiss_runtime_upgrade_failure(state: State<'_, AppState>) -> Result<(), String> {
    state.dismiss_upgrade_failure();
    Ok(())
}

#[tauri::command]
fn get_runtime_status(state: State<'_, AppState>) -> RuntimeStatus {
    state.runtime_status()
}

fn switchboard_mode_label(mode: &SwitchboardMode) -> &'static str {
    match mode {
        SwitchboardMode::Off => "Off",
        SwitchboardMode::Rtk => "RTK only",
        SwitchboardMode::Headroom => "Headroom only",
        SwitchboardMode::Full => "Full optimization",
    }
}

fn saved_switchboard_mode_wants_headroom() -> bool {
    switchboard_mode_wants_headroom(client_adapters::load_switchboard_mode().as_ref())
}

fn saved_switchboard_mode_wants_rtk() -> bool {
    switchboard_mode_wants_rtk(client_adapters::load_switchboard_mode().as_ref())
}

fn switchboard_mode_wants_headroom(mode: Option<&SwitchboardMode>) -> bool {
    matches!(
        mode,
        Some(SwitchboardMode::Headroom | SwitchboardMode::Full) | None
    )
}

fn switchboard_mode_wants_rtk(mode: Option<&SwitchboardMode>) -> bool {
    matches!(
        mode,
        Some(SwitchboardMode::Rtk | SwitchboardMode::Full) | None
    )
}

fn doctor_repair_action_restores_headroom(action: &str) -> bool {
    matches!(
        action,
        "repair_runtime" | "repair_client_setups" | "repair_codex_setup" | "repair_all"
    )
}

fn switchboard_mode_blocks_doctor_repair(mode: Option<&SwitchboardMode>, action: &str) -> bool {
    !switchboard_mode_wants_headroom(mode) && doctor_repair_action_restores_headroom(action)
}

fn infer_switchboard_mode(
    runtime: &RuntimeStatus,
    enabled_client_count: usize,
) -> (SwitchboardMode, bool, bool) {
    let rtk_enabled = runtime.rtk.installed && runtime.rtk.enabled;
    let headroom_enabled =
        runtime.running && runtime.proxy_reachable && !runtime.paused && enabled_client_count > 0;
    let mode = match (headroom_enabled, rtk_enabled) {
        (true, true) => SwitchboardMode::Full,
        (true, false) => SwitchboardMode::Headroom,
        (false, true) => SwitchboardMode::Rtk,
        (false, false) => SwitchboardMode::Off,
    };

    (mode, rtk_enabled, headroom_enabled)
}

fn off_mode_violations(runtime: &RuntimeStatus, enabled_client_count: usize) -> Vec<&'static str> {
    let mut violations = Vec::new();
    if runtime.running || runtime.proxy_reachable {
        violations.push("Headroom engine is still reachable");
    }
    if enabled_client_count > 0 {
        violations.push("managed clients are still routed");
    }
    if runtime.rtk.installed && runtime.rtk.enabled {
        violations.push("RTK is still enabled");
    }
    violations
}

fn push_off_mode_doctor_issue(
    issues: &mut Vec<crate::models::DoctorIssue>,
    runtime: &RuntimeStatus,
    enabled_client_count: usize,
) {
    let violations = off_mode_violations(runtime, enabled_client_count);
    if violations.is_empty() {
        return;
    }

    issues.push(crate::models::DoctorIssue {
        id: "off_mode_not_clean".to_string(),
        title: "Off mode still has active routing evidence".to_string(),
        body: format!(
            "Off mode requested, but {}. Disable routing or restart affected shells, then run Doctor again.",
            violations.join(", ")
        ),
        severity: crate::models::DoctorSeverity::Warning,
        repair_action: Some("verify_off_mode".to_string()),
    });
}

fn build_switchboard_state(state: &AppState) -> Result<SwitchboardState, String> {
    let runtime = state.runtime_status();
    let clients = client_adapters::list_client_connectors(&state.cached_clients())
        .map_err(|err| err.to_string())?;
    let enabled_clients: Vec<ClientConnectorStatus> = clients
        .iter()
        .filter(|client| client.enabled)
        .cloned()
        .collect();
    let (inferred_mode, rtk_enabled, headroom_enabled) =
        infer_switchboard_mode(&runtime, enabled_clients.len());
    let desired_mode = client_adapters::load_switchboard_mode().unwrap_or(inferred_mode.clone());
    let savings_mode = client_adapters::load_savings_mode();
    let effective_mode = inferred_mode;
    let needs_attention = desired_mode != effective_mode;
    let codex_direct_bypass = state
        .codex_bypass
        .load(std::sync::atomic::Ordering::Acquire);
    let summary = if needs_attention {
        format!(
"{} requested, but {} is currently active. Run Doctor to repair the missing local pieces.",
switchboard_mode_label(&desired_mode),
switchboard_mode_label(&effective_mode)
)
    } else if codex_direct_bypass
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
    {
        "Codex is temporarily bypassing Headroom after an oversized compression refusal. Compact context or switch to RTK only, then re-enable Headroom."
.to_string()
    } else {
        match desired_mode {
            SwitchboardMode::Full => {
                "Headroom proxy routing and RTK command compression are both active."
            }
            SwitchboardMode::Headroom => {
                "LLM traffic is routed through Headroom. RTK command compression is off."
            }
            SwitchboardMode::Rtk => {
                "RTK command compression is active. No coding client is routed through Headroom."
            }
            SwitchboardMode::Off => "No optimization layer is active right now.",
        }
        .to_string()
    };
    let local_only = local_mode::enabled();

    Ok(SwitchboardState {
        mode: desired_mode.clone(),
        desired_mode,
        effective_mode,
        savings_mode,
        needs_attention,
        local_only,
        remote_services_enabled: !local_only,
        runtime,
        clients,
        enabled_clients,
        rtk_enabled,
        headroom_enabled,
        summary,
    })
}

#[tauri::command]
async fn get_switchboard_state(state: State<'_, AppState>) -> Result<SwitchboardState, String> {
    build_switchboard_state(&state)
}

fn repo_intelligence_saved_paths_missing(summary: &crate::models::RepoIntelligenceSummary) -> bool {
    let Some(metadata) = summary.index_metadata.as_ref() else {
        return false;
    };
    if metadata.file_fingerprints.is_empty() {
        return false;
    }
    let repo_root = Path::new(&summary.repo_root);
    metadata
        .file_fingerprints
        .iter()
        .all(|entry| !repo_root.join(&entry.path).exists())
}

fn repo_intelligence_doctor_issue(
    summary: &crate::models::RepoIntelligenceSummary,
    now: DateTime<Utc>,
) -> Option<crate::models::DoctorIssue> {
    if !Path::new(&summary.repo_root).is_dir() {
        return Some(crate::models::DoctorIssue {
            id: "repo_intelligence_repo_missing".to_string(),
            title: "Repo Intelligence index points to a missing folder".to_string(),
            body: format!(
                "The last indexed repo path is no longer available: {}. Repair will clear this saved index; then re-index an available local repository from the Repo Intelligence add-on card.",
                summary.repo_root
            ),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        });
    }

    if repo_intelligence_saved_paths_missing(summary) {
        return Some(crate::models::DoctorIssue {
            id: "repo_intelligence_repo_moved".to_string(),
            title: "Repo Intelligence index no longer matches this folder".to_string(),
            body: format!(
                "The saved Repo Intelligence file map no longer matches files under {}. The repo may have moved, been replaced, or been cleaned. Repair will clear this saved index; then re-index the current local repository before copying packs or agent handoffs.",
                summary.repo_root
            ),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        });
    }

    let freshness = repo_intelligence::build_index_freshness_response(Some(summary));
    let indexer_health = if freshness.indexer_version.as_deref()
        == Some(repo_intelligence::current_indexer_version())
    {
        "current"
    } else {
        "version_mismatch"
    };
    if freshness.parser_health == "version_mismatch"
        || freshness.index_health == "metadata_missing"
        || indexer_health == "version_mismatch"
    {
        return Some(crate::models::DoctorIssue {
            id: "repo_intelligence_index_health".to_string(),
            title: "Repo Intelligence parser/index health needs refresh".to_string(),
            body: format!(
                "The saved Repo Intelligence index for {} reports index health '{}', parser health '{}', and indexer health '{}'. Repair will clear this saved index; then re-index the current local repository so Doctor and agent handoffs use the current parser/index contract.",
                summary.repo_root, freshness.index_health, freshness.parser_health, indexer_health
            ),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        });
    }

    let stale = DateTime::parse_from_rfc3339(&summary.indexed_at)
        .map(|indexed_at| {
            now.signed_duration_since(indexed_at.with_timezone(&Utc))
                .num_days()
                >= 7
        })
        .unwrap_or(false);
    if stale {
        return Some(crate::models::DoctorIssue {
            id: "repo_intelligence_stale".to_string(),
            title: "Repo Intelligence index is stale".to_string(),
            body: format!(
                "The last Repo Intelligence index for {} is more than 7 days old. Repair will clear the stale saved index; then re-index it before relying on context packs for agent handoff.",
                summary.repo_root
            ),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        });
    }

    None
}

fn repo_memory_mcp_doctor_issue(runtime: &RuntimeStatus) -> Option<crate::models::DoctorIssue> {
    if runtime.repo_memory_mcp_configured != Some(false) {
        return None;
    }

    Some(crate::models::DoctorIssue {
        id: "repo_memory_mcp_not_configured".to_string(),
        title: "Repo Memory MCP is not configured".to_string(),
        body: runtime
            .repo_memory_mcp_error
            .clone()
            .unwrap_or_else(|| {
                "Repo Memory MCP is required before supported agents can request read-only Repo Intelligence packs through MCP. Repair will install the app-managed read-only repo-memory server, then you can run npm run check:repo-memory-mcp to verify the tool contract.".to_string()
            }),
        severity: crate::models::DoctorSeverity::Warning,
        repair_action: Some("install_repo_memory_mcp".to_string()),
    })
}

fn build_doctor_report(state: &AppState) -> crate::models::DoctorReport {
    let runtime = state.runtime_status();
    let codex_direct_bypass = state
        .codex_bypass
        .load(std::sync::atomic::Ordering::Acquire);
    let mut issues = Vec::new();
    let connectors =
        client_adapters::list_client_connectors(&state.cached_clients()).unwrap_or_default();
    let managed_connectors = connectors.iter().filter(|client| {
        matches!(
            client.support_status,
            crate::models::ClientConnectorSupportStatus::Managed
        )
    });
    let enabled_clients = managed_connectors
        .clone()
        .filter(|client| client.enabled)
        .count();
    let installed_clients = managed_connectors.filter(|client| client.installed).count();
    let planned_installed = connectors
        .iter()
        .filter(|client| {
            client.installed
                && matches!(
                    client.support_status,
                    crate::models::ClientConnectorSupportStatus::Planned
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    let (inferred_mode, _rtk_ready, _headroom_ready) =
        infer_switchboard_mode(&runtime, enabled_clients);
    let desired_mode = client_adapters::load_switchboard_mode().unwrap_or(inferred_mode.clone());

    if matches!(desired_mode, SwitchboardMode::Off) {
        push_off_mode_doctor_issue(&mut issues, &runtime, enabled_clients);
    }

    if let Some(issue) = repo_memory_mcp_doctor_issue(&runtime) {
        issues.push(issue);
    }

    if desired_mode != inferred_mode {
        issues.push(crate::models::DoctorIssue {
            id: "switchboard_mode_degraded".to_string(),
            title: "Requested optimization is degraded".to_string(),
            body: format!(
                "{} is requested, but {} is active. Doctor lists missing local pieces below; repair managed connector items, then keep only remaining planned connector steps manual.",
                switchboard_mode_label(&desired_mode),
                switchboard_mode_label(&inferred_mode)
            ),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: None,
        });
    }

    if matches!(
        desired_mode,
        SwitchboardMode::Full | SwitchboardMode::Headroom
    ) && runtime.installed
        && (!runtime.running || !runtime.proxy_reachable || runtime.auto_paused)
    {
        issues.push(crate::models::DoctorIssue {
id: "headroom_runtime_unreachable".to_string(),
title: "Headroom runtime is not reachable".to_string(),
body: runtime
.startup_error_hint
.clone()
.or_else(|| runtime.startup_error.clone())
.unwrap_or_else(|| {
"The local proxy is not answering. Repair will restart the Headroom runtime and refresh switchboard status.".to_string()
}),
severity: crate::models::DoctorSeverity::Error,
repair_action: Some("repair_runtime".to_string()),
});
    }

    if codex_direct_bypass
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
    {
        issues.push(crate::models::DoctorIssue {
id: "codex_direct_bypass".to_string(),
title: "Codex is bypassing Headroom".to_string(),
body: "Headroom refused compression for an oversized Codex request, so Codex is temporarily going direct. Compact the conversation context, then reset this bypass to route Codex through Headroom again.".to_string(),
severity: crate::models::DoctorSeverity::Warning,
repair_action: Some("reset_codex_bypass".to_string()),
});
    }

    if runtime.proxy_reachable && runtime.proxy_auth_status != "authenticated" {
        issues.push(crate::models::DoctorIssue {
            id: "proxy_loopback_unauthenticated".to_string(),
            title: "Proxy is loopback-only, not authenticated".to_string(),
            body: format!(
                "The local proxy is bound to {} and rejects browser Origin/non-loopback Host requests, but managed clients do not yet send a per-session auth token. Treat localhost as local-process trust, not a security boundary.",
                runtime.proxy_bind_address
            ),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: None,
        });
    }

    let codex_connector_enabled = connectors
        .iter()
        .any(|client| client.client_id == "codex" && client.enabled);
    if codex_connector_enabled
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
        && !client_adapters::codex_provider_block_matches().unwrap_or(false)
    {
        issues.push(crate::models::DoctorIssue {
id: "codex_provider_mismatch".to_string(),
title: "Codex routing config needs repair".to_string(),
body: "Codex is marked as connected, but its model provider or proxy URL no longer matches the managed Headroom setup. This can cause empty or unsupported-model errors. Repair will re-apply the reversible Codex setup.".to_string(),
severity: crate::models::DoctorSeverity::Warning,
repair_action: Some("repair_codex_setup".to_string()),
        });
    }

    if codex_connector_enabled
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
    {
        let retagging = client_adapters::get_codex_thread_retagging_settings();
        if !matches!(
            retagging.codex_thread_retagging,
            crate::models::CodexThreadRetaggingMode::Enabled
        ) {
            issues.push(crate::models::DoctorIssue {
                id: "codex_thread_retagging_opt_in_required".to_string(),
                title: "Codex history retagging needs consent".to_string(),
                body: "Codex is routed through Headroom, but Switchboard will not edit Codex SQLite history until retagging is explicitly enabled. History may appear split between native and Headroom providers; enable retagging only after reviewing the backup and restore notes.".to_string(),
                severity: crate::models::DoctorSeverity::Warning,
                repair_action: None,
            });
        }
    }

    for connector in connectors.iter().filter(|client| {
        client.enabled
            && !client.verified
            && client.client_id != "codex"
            && matches!(
                client.support_status,
                crate::models::ClientConnectorSupportStatus::Managed
            )
    }) {
        issues.push(crate::models::DoctorIssue {
            id: format!("{}_routing_config_mismatch", connector.client_id),
            title: format!("{} routing config needs repair", connector.name),
            body: format!(
                "{} is marked as connected, but its managed routing config no longer verifies. Repair will re-apply the reversible managed setup and preserve user-owned config outside Switchboard markers.",
                connector.name
            ),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: Some("repair_client_setups".to_string()),
        });
    }

    if !planned_installed.is_empty() {
        issues.push(crate::models::DoctorIssue {
            id: "planned_connectors_detected".to_string(),
            title: "Planned coding tools detected".to_string(),
            body: planned_connector_doctor_body(&planned_installed),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: None,
        });
    }

    if matches!(
        desired_mode,
        SwitchboardMode::Full | SwitchboardMode::Headroom
    ) && enabled_clients == 0
    {
        let repair_action = if installed_clients > 0 {
            Some("repair_client_setups".to_string())
        } else {
            None
        };
        issues.push(crate::models::DoctorIssue {
id: "no_headroom_clients".to_string(),
title: "No clients are routed through Headroom".to_string(),
body: if installed_clients > 0 {
"Installed coding clients were found, but none are currently configured to use Headroom. Repair will re-apply reversible client setup.".to_string()
} else {
"No supported coding clients were detected yet. Install or open Codex, Claude Code, or a supported editor, then return to connect it.".to_string()
},
severity: crate::models::DoctorSeverity::Warning,
repair_action,
});
    }

    match repo_intelligence::load_latest_summary() {
        Ok(Some(summary)) => {
            if let Some(issue) = repo_intelligence_doctor_issue(&summary, Utc::now()) {
                issues.push(issue);
            }
        }
        Ok(None) => {}
        Err(err) => issues.push(crate::models::DoctorIssue {
            id: "repo_intelligence_storage_corrupt".to_string(),
            title: "Repo Intelligence index cannot be read".to_string(),
            body: format!(
                "The saved Repo Intelligence index could not be parsed or read: {err}. Repair will clear the saved index; then re-index a local repository before copying packs or agent handoffs."
            ),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        }),
    }

    if matches!(desired_mode, SwitchboardMode::Full | SwitchboardMode::Rtk)
        && (!runtime.rtk.installed || !runtime.rtk.enabled)
    {
        issues.push(crate::models::DoctorIssue {
id: "rtk_not_active".to_string(),
title: "RTK is not active".to_string(),
body: if runtime.rtk.installed {
"RTK is installed but turned off. Repair will enable local RTK shell compression.".to_string()
} else {
"RTK is required for the requested switchboard mode. Repair will install RTK into Headroom-managed storage and enable local shell compression.".to_string()
},
severity: crate::models::DoctorSeverity::Warning,
repair_action: Some("repair_rtk_runtime".to_string()),
});
    }

    if matches!(desired_mode, SwitchboardMode::Full | SwitchboardMode::Rtk)
        && runtime.rtk.installed
        && runtime.rtk.enabled
        && (!runtime.rtk.path_configured || !runtime.rtk.hook_configured)
    {
        issues.push(crate::models::DoctorIssue {
id: "rtk_integration_incomplete".to_string(),
title: "RTK integration is incomplete".to_string(),
body: "RTK is enabled, but its shell PATH export or Claude Code hook is missing. Repair will re-apply the local RTK integration.".to_string(),
severity: crate::models::DoctorSeverity::Warning,
repair_action: Some("repair_rtk_integrations".to_string()),
});
    }

    let tools = state.tool_manager.list_tools();
    let tool_needs_repair = |id: &str| {
        tools.iter().find(|tool| tool.id == id).is_some_and(|tool| {
            !tool.enabled || !matches!(tool.status, crate::models::ToolStatus::Healthy)
        })
    };
    let caveman_level = state.tool_manager.caveman_level();
    let caveman_guidance_drifted = state.tool_manager.caveman_receipt_exists()
        && tools
            .iter()
            .find(|tool| tool.id == "caveman")
            .is_some_and(|tool| tool.enabled)
        && !client_adapters::caveman_integration_matches_level(&caveman_level).unwrap_or(false);
    if tool_needs_repair("caveman") || caveman_guidance_drifted {
        issues.push(crate::models::DoctorIssue {
            id: "caveman_guidance_inactive".to_string(),
            title: "Caveman guidance is not active".to_string(),
            body: "Caveman should keep a managed guidance block in Claude Code and Codex instruction files. Repair will recreate its local receipt and rewrite the Switchboard-owned guidance block.".to_string(),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: Some("repair_caveman_guidance".to_string()),
        });
    }
    if tool_needs_repair("ponytail") {
        issues.push(crate::models::DoctorIssue {
            id: "ponytail_plugin_inactive".to_string(),
            title: "Ponytail plugin is not active".to_string(),
            body: "Ponytail should be registered with Claude Code or Codex when its add-on is enabled. Repair will re-run the plugin install for available local hosts.".to_string(),
            severity: crate::models::DoctorSeverity::Warning,
            repair_action: Some("repair_ponytail_plugin".to_string()),
        });
    }

    if runtime.paused
        && !runtime.auto_paused
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
    {
        issues.push(crate::models::DoctorIssue {
	id: "headroom_paused".to_string(),
	title: "Headroom engine is paused".to_string(),
	body: "The proxy is intentionally off. Use Full optimization or Headroom only to restart routing through the Headroom engine.".to_string(),
	severity: crate::models::DoctorSeverity::Warning,
	repair_action: None,
	});
    }

    let status = if issues
        .iter()
        .any(|issue| matches!(issue.severity, crate::models::DoctorSeverity::Error))
    {
        crate::models::DoctorSeverity::Error
    } else if issues.is_empty() {
        crate::models::DoctorSeverity::Ok
    } else {
        crate::models::DoctorSeverity::Warning
    };

    let summary = match status {
        crate::models::DoctorSeverity::Ok => {
            "No switchboard issues detected. Headroom and RTK look ready for normal use."
        }
        crate::models::DoctorSeverity::Warning => {
            "Doctor found switchboard items that may need attention."
        }
        crate::models::DoctorSeverity::Error => "Doctor found a blocking switchboard issue.",
    }
    .to_string();

    crate::models::DoctorReport {
        status,
        summary,
        issues,
    }
}

fn planned_connector_doctor_body(connectors: &[ClientConnectorStatus]) -> String {
    let names = connectors
        .iter()
        .map(|client| client.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let sources = connectors
        .iter()
        .flat_map(|client| client.detection_sources.iter().map(String::as_str))
        .take(4)
        .collect::<Vec<_>>();
    let locations = connectors
        .iter()
        .flat_map(|client| client.config_locations.iter().map(String::as_str))
        .take(3)
        .collect::<Vec<_>>();
    let evidence = connectors
        .iter()
        .flat_map(|client| client.detection_evidence.iter().map(String::as_str))
        .take(3)
        .collect::<Vec<_>>();
    let gates = connectors
        .iter()
        .flat_map(|client| client.automation_gates.iter().map(String::as_str))
        .take(3)
        .collect::<Vec<_>>();
    let manual_workflow = connectors
        .iter()
        .flat_map(|client| client.manual_workflow.iter().map(String::as_str))
        .take(3)
        .collect::<Vec<_>>();
    let config_steps = connectors
        .iter()
        .flat_map(|client| {
            if client.config_creation_step_details.is_empty() {
                client.config_creation_steps.clone()
            } else {
                client
                    .config_creation_step_details
                    .iter()
                    .map(|step| step.label.clone())
                    .collect()
            }
        })
        .take(7)
        .collect::<Vec<_>>();
    let dry_run_previews = connectors
        .iter()
        .filter_map(|client| {
            client.config_dry_run_preview.as_ref().map(|preview| {
                format!(
                    "{} target {} marker {} confirmation {}",
                    client.name, preview.target, preview.marker, preview.confirmation_phrase
                )
            })
        })
        .take(3)
        .collect::<Vec<_>>();

    let mut parts = vec![format!(
        "{names} detected. Mac AI Switchboard can identify these tools but keeps routing manual until backup, restore, and Off mode cleanup are implemented."
    )];

    if !sources.is_empty() {
        parts.push(format!("Backend checks: {}.", sources.join(", ")));
    }

    if !locations.is_empty() {
        parts.push(format!(
            "Config locations watched: {}.",
            locations.join(", ")
        ));
    }
    if !evidence.is_empty() {
        parts.push(format!("Detection evidence: {}.", evidence.join(" | ")));
    }
    if !gates.is_empty() {
        parts.push(format!("Automation gates: {}.", gates.join(" | ")));
    }
    if !manual_workflow.is_empty() {
        parts.push(format!("Manual workflow: {}.", manual_workflow.join(" | ")));
    }
    if !config_steps.is_empty() {
        parts.push(format!(
            "Config creation plan: {}.",
            config_steps.join(" -> ")
        ));
    }
    if !dry_run_previews.is_empty() {
        parts.push(format!(
            "Dry-run preview evidence: {}.",
            dry_run_previews.join(" | ")
        ));
    }
    parts.push(
        "Safe today: use RTK-only mode or Repo Intelligence packs; do not enable automatic provider routing yet."
            .to_string(),
    );

    parts.join(" ")
}

#[cfg(test)]
mod doctor_tests {
    use super::*;

    #[test]
    fn planned_connector_doctor_body_includes_backend_metadata() {
        let body = planned_connector_doctor_body(&[ClientConnectorStatus {
            client_id: "gemini_cli".to_string(),
            name: "Gemini CLI".to_string(),
            support_status: crate::models::ClientConnectorSupportStatus::Planned,
            setup_phase: "guide".to_string(),
            setup_hint: "Manual guide only.".to_string(),
            category: "cli".to_string(),
            detection_sources: vec!["PATH: gemini".to_string(), "~/.gemini".to_string()],
            detection_evidence: vec!["Detected at /opt/homebrew/bin/gemini".to_string()],
            config_locations: vec!["~/.gemini".to_string()],
            automation_gates: vec![
                "Back up provider settings before any routing change.".to_string()
            ],
            manual_workflow: vec!["Use RTK-only mode for noisy output.".to_string()],
            config_creation_steps: vec![
                "Detect config surface".to_string(),
                "Show dry-run diff".to_string(),
                "Create backup".to_string(),
                "Apply with consent".to_string(),
                "Verify in Doctor".to_string(),
                "Rollback safely".to_string(),
                "Clean up in Off mode".to_string(),
            ],
            config_creation_step_details: Vec::new(),
            config_dry_run_preview: Some(crate::models::ClientConnectorConfigDryRunPreview {
                target: "/Users/test/.gemini".to_string(),
                marker: "mac-ai-switchboard:gemini_cli".to_string(),
                backup_path: "/Users/test/.gemini.mac-ai-switchboard.bak".to_string(),
                current_state: "No Switchboard-managed Gemini provider routing detected."
                    .to_string(),
                proposed_state:
                    "Add Mac AI Switchboard local provider routing after explicit consent."
                        .to_string(),
                apply_blocked_reason:
                    "Gemini CLI automation is disabled until backup, verify, rollback, and Off cleanup gates pass."
                        .to_string(),
                rollback_preview:
                    "Restore the Gemini config backup or remove only the managed block.".to_string(),
                confirmation_phrase: "APPLY GEMINI CLI CONFIG".to_string(),
                writes: Vec::new(),
            }),
            automation_path: vec![
                crate::models::ClientConnectorAutomationStage {
                    id: "detect".to_string(),
                    label: "Detect config surface".to_string(),
                    status: "ready".to_string(),
                    evidence: "Gemini CLI has local detection evidence.".to_string(),
                },
                crate::models::ClientConnectorAutomationStage {
                    id: "dryRunDiff".to_string(),
                    label: "Show dry-run diff".to_string(),
                    status: "ready".to_string(),
                    evidence: "Blocked preview is ready.".to_string(),
                },
            ],
            installed: true,
            enabled: false,
            verified: false,
            setup_verification: None,
            last_configured_at: None,
        }]);

        assert!(body.contains("Gemini CLI detected"));
        assert!(body.contains("Backend checks: PATH: gemini, ~/.gemini."));
        assert!(body.contains("Config locations watched: ~/.gemini."));
        assert!(body.contains("Detection evidence: Detected at /opt/homebrew/bin/gemini."));
        assert!(body.contains("Automation gates: Back up provider settings"));
        assert!(body.contains("Manual workflow: Use RTK-only mode"));
        assert!(body.contains("Config creation plan: Detect config surface -> Show dry-run diff"));
        assert!(body.contains("Dry-run preview evidence: Gemini CLI target /Users/test/.gemini"));
        assert!(body.contains("marker mac-ai-switchboard:gemini_cli"));
        assert!(body.contains("confirmation APPLY GEMINI CLI CONFIG"));
        assert!(body.contains("Safe today: use RTK-only mode or Repo Intelligence packs"));
        assert!(body.contains("keeps routing manual"));
    }

    fn test_runtime_status(
        running: bool,
        proxy_reachable: bool,
        rtk_enabled: bool,
    ) -> RuntimeStatus {
        RuntimeStatus {
            platform: "macos".to_string(),
            support_tier: "full".to_string(),
            installed: true,
            running,
            starting: false,
            paused: false,
            auto_paused: false,
            proxy_reachable,
            proxy_bind_address: "127.0.0.1:6767".to_string(),
            proxy_auth_status: "loopback_validated_unauthenticated".to_string(),
            proxy_auth_detail: "Loopback-only test fixture.".to_string(),
            headroom_pid: if running { Some(42) } else { None },
            launch_agent_status: crate::models::LaunchAgentRuntimeStatus {
                installed: false,
                path: None,
                label: "com.tarunagarwal.mac-ai-switchboard".to_string(),
                loaded: Some(false),
                load_detail: Some(
                    "launchctl does not report test LaunchAgent as loaded.".to_string(),
                ),
                legacy_installed: false,
                legacy_path: None,
                legacy_label: "Headroom".to_string(),
                legacy_loaded: Some(false),
                legacy_load_detail: Some(
                    "launchctl does not report legacy test LaunchAgent as loaded.".to_string(),
                ),
            },
            backend_status: crate::models::BackendRuntimeStatus {
                reachable: running,
                bind_address: "127.0.0.1:6768".to_string(),
                port: 6768,
                default_port: 6768,
                fallback_range_start: 6769,
                fallback_range_end: 6790,
            },
            mcp_configured: None,
            mcp_error: None,
            repo_memory_mcp_configured: None,
            repo_memory_mcp_error: None,
            repo_memory_mcp_active: false,
            repo_memory_mcp_last_started_at: None,
            repo_memory_mcp_last_checked_at: None,
            repo_memory_mcp_supervision_status: "unknown".to_string(),
            repo_memory_mcp_service: None,
            ml_installed: None,
            kompress_enabled: None,
            headroom_learn_supported: true,
            headroom_learn_disabled_reason: None,
            startup_error: None,
            startup_error_hint: None,
            runtime_upgrade_failure: None,
            rtk: crate::models::RtkRuntimeStatus {
                installed: rtk_enabled,
                enabled: rtk_enabled,
                version: None,
                path_configured: false,
                hook_configured: false,
                total_commands: None,
                total_saved: None,
                avg_savings_pct: None,
            },
        }
    }

    #[test]
    fn off_mode_violations_empty_when_runtime_clients_and_rtk_are_off() {
        let runtime = test_runtime_status(false, false, false);
        assert!(off_mode_violations(&runtime, 0).is_empty());
    }

    #[test]
    fn switchboard_mode_intent_disables_everything_in_off_mode() {
        assert!(!switchboard_mode_wants_headroom(Some(
            &SwitchboardMode::Off
        )));
        assert!(!switchboard_mode_wants_rtk(Some(&SwitchboardMode::Off)));
    }

    #[test]
    fn switchboard_mode_intent_keeps_rtk_only_without_headroom() {
        assert!(!switchboard_mode_wants_headroom(Some(
            &SwitchboardMode::Rtk
        )));
        assert!(switchboard_mode_wants_rtk(Some(&SwitchboardMode::Rtk)));
    }

    #[test]
    fn switchboard_mode_intent_defaults_to_full_optimization() {
        assert!(switchboard_mode_wants_headroom(None));
        assert!(switchboard_mode_wants_rtk(None));
    }

    #[test]
    fn off_mode_blocks_doctor_repairs_that_restore_headroom() {
        for action in [
            "repair_runtime",
            "repair_client_setups",
            "repair_codex_setup",
            "repair_all",
        ] {
            assert!(
                switchboard_mode_blocks_doctor_repair(Some(&SwitchboardMode::Off), action),
                "{action} should be blocked in Off mode"
            );
        }
    }

    #[test]
    fn rtk_only_blocks_doctor_repairs_that_restore_headroom() {
        for action in [
            "repair_runtime",
            "repair_client_setups",
            "repair_codex_setup",
            "repair_all",
        ] {
            assert!(
                switchboard_mode_blocks_doctor_repair(Some(&SwitchboardMode::Rtk), action),
                "{action} should be blocked in RTK-only mode"
            );
        }
    }

    #[test]
    fn headroom_modes_allow_headroom_repair_actions() {
        for mode in [SwitchboardMode::Headroom, SwitchboardMode::Full] {
            for action in [
                "repair_runtime",
                "repair_client_setups",
                "repair_codex_setup",
                "repair_all",
            ] {
                assert!(
                    !switchboard_mode_blocks_doctor_repair(Some(&mode), action),
                    "{action} should be allowed in {}",
                    switchboard_mode_label(&mode)
                );
            }
        }
    }

    #[test]
    fn non_headroom_doctor_repairs_remain_available_in_off_and_rtk_modes() {
        for mode in [SwitchboardMode::Off, SwitchboardMode::Rtk] {
            for action in [
                "verify_off_mode",
                "reset_codex_bypass",
                "repair_rtk_integrations",
                "repair_rtk_runtime",
                "repair_caveman_guidance",
                "repair_ponytail_plugin",
                "clear_repo_intelligence_index",
                "install_repo_memory_mcp",
            ] {
                assert!(
                    !switchboard_mode_blocks_doctor_repair(Some(&mode), action),
                    "{action} should remain available in {}",
                    switchboard_mode_label(&mode)
                );
            }
        }
    }

    #[test]
    fn off_mode_doctor_issue_lists_active_routing_evidence() {
        let runtime = test_runtime_status(true, true, true);
        let mut issues = Vec::new();

        push_off_mode_doctor_issue(&mut issues, &runtime, 2);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "off_mode_not_clean");
        assert!(issues[0]
            .body
            .contains("Headroom engine is still reachable"));
        assert!(issues[0].body.contains("managed clients are still routed"));
        assert!(issues[0].body.contains("RTK is still enabled"));
        assert!(matches!(
            issues[0].severity,
            crate::models::DoctorSeverity::Warning
        ));
        assert_eq!(issues[0].repair_action.as_deref(), Some("verify_off_mode"));
    }

    #[test]
    fn repo_memory_mcp_doctor_issue_is_repairable_when_unconfigured() {
        let mut runtime = test_runtime_status(true, true, true);
        runtime.mcp_configured = Some(true);
        runtime.mcp_error = None;
        runtime.repo_memory_mcp_configured = Some(false);
        runtime.repo_memory_mcp_error =
            Some("repo-memory missing from Claude MCP config".to_string());

        let issue = repo_memory_mcp_doctor_issue(&runtime).expect("repo memory issue");

        assert_eq!(issue.id, "repo_memory_mcp_not_configured");
        assert_eq!(
            issue.repair_action.as_deref(),
            Some("install_repo_memory_mcp")
        );
        assert!(issue.body.contains("repo-memory missing"));

        runtime.repo_memory_mcp_configured = Some(true);
        assert!(repo_memory_mcp_doctor_issue(&runtime).is_none());
    }
}

#[tauri::command]
fn get_doctor_report(state: State<'_, AppState>) -> crate::models::DoctorReport {
    build_doctor_report(&state)
}

fn repair_runtime(state: &AppState) -> Result<(), String> {
    state.stop_headroom();
    state.set_runtime_auto_paused(false);
    state.resume_runtime().map_err(|err| err.to_string())?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

fn repair_client_setups(state: &AppState) -> Result<(), String> {
    state
        .codex_bypass
        .store(false, std::sync::atomic::Ordering::Release);
    state.resume_runtime().map_err(|err| err.to_string())?;
    let connectors = client_adapters::list_client_connectors(&state.cached_clients())
        .map_err(|err| err.to_string())?;
    let managed_installed = connectors.iter().filter(|connector| {
        connector.installed
            && matches!(
                connector.support_status,
                crate::models::ClientConnectorSupportStatus::Managed
            )
    });
    let mut repaired = 0usize;
    for connector in managed_installed {
        client_adapters::apply_client_setup(&connector.client_id).map_err(|err| err.to_string())?;
        repaired += 1;
    }
    if repaired == 0 {
        return Err("no installed supported clients found to repair".to_string());
    }
    state.invalidate_runtime_status_cache();
    Ok(())
}

fn repair_codex_setup(state: &AppState) -> Result<(), String> {
    state
        .codex_bypass
        .store(false, std::sync::atomic::Ordering::Release);
    state.resume_runtime().map_err(|err| err.to_string())?;
    client_adapters::apply_client_setup("codex").map_err(|err| err.to_string())?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

fn repair_rtk_integrations(state: &AppState) -> Result<(), String> {
    client_adapters::set_rtk_enabled(
        true,
        &state.tool_manager.rtk_entrypoint(),
        &state.tool_manager.managed_python(),
    )
    .map_err(|err| err.to_string())?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

fn repair_rtk_runtime(state: &AppState) -> Result<(), String> {
    if !state.tool_manager.rtk_installed() {
        state
            .tool_manager
            .install_rtk()
            .map_err(|err| err.to_string())?;
    }
    repair_rtk_integrations(state)
}

fn repair_caveman_guidance(state: &AppState) -> Result<(), String> {
    if !state.tool_manager.caveman_receipt_exists() {
        state
            .tool_manager
            .install_caveman()
            .map_err(|err| err.to_string())?;
    } else {
        state
            .tool_manager
            .set_caveman_enabled(true)
            .map_err(|err| err.to_string())?;
    }
    client_adapters::enable_caveman_integration(&state.tool_manager.caveman_level())
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn repair_ponytail_plugin(state: &AppState) -> Result<(), String> {
    if state.tool_manager.list_tools().iter().any(|tool| {
        tool.id == "ponytail" && !matches!(tool.status, crate::models::ToolStatus::Healthy)
    }) {
        state
            .tool_manager
            .install_ponytail()
            .map_err(|err| err.to_string())?;
    } else {
        state
            .tool_manager
            .set_ponytail_enabled(true)
            .map_err(|err| err.to_string())?;
    }
    let hosts = state.tool_manager.ponytail_registered_hosts();
    let _ = state.record_ponytail_attribution(&hosts);
    Ok(())
}

fn clear_repo_intelligence_index() -> Result<(), String> {
    repo_intelligence::clear_latest_summary()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn repair_repo_memory_mcp(state: &AppState) -> Result<(), String> {
    state
        .tool_manager
        .install_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    state
        .start_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

#[tauri::command]
async fn run_doctor_repair(
    state: State<'_, AppState>,
    action: String,
) -> Result<crate::models::DoctorReport, String> {
    let saved_mode = client_adapters::load_switchboard_mode();
    if switchboard_mode_blocks_doctor_repair(saved_mode.as_ref(), action.as_str()) {
        let mode_label = saved_mode
            .as_ref()
            .map(switchboard_mode_label)
            .unwrap_or("current mode");
        return Err(format!(
            "{mode_label} is requested, so Doctor will not run {action} because it can restore Headroom routing. Choose Headroom only or Full optimization first."
        ));
    }

    match action.as_str() {
        "verify_off_mode" => Ok(build_doctor_report(&state)),
        "reset_codex_bypass" => {
            state
                .codex_bypass
                .store(false, std::sync::atomic::Ordering::Release);
            state.invalidate_runtime_status_cache();
            Ok(build_doctor_report(&state))
        }
        "repair_runtime" => {
            repair_runtime(&state)?;
            Ok(build_doctor_report(&state))
        }
        "repair_client_setups" => {
            repair_client_setups(&state)?;
            Ok(build_doctor_report(&state))
        }
        "repair_codex_setup" => {
            repair_codex_setup(&state)?;
            Ok(build_doctor_report(&state))
        }
        "repair_rtk_integrations" => {
            repair_rtk_integrations(&state)?;
            Ok(build_doctor_report(&state))
        }
        "repair_rtk_runtime" => {
            repair_rtk_runtime(&state)?;
            Ok(build_doctor_report(&state))
        }
        "repair_caveman_guidance" => {
            repair_caveman_guidance(&state)?;
            Ok(build_doctor_report(&state))
        }
        "repair_ponytail_plugin" => {
            repair_ponytail_plugin(&state)?;
            Ok(build_doctor_report(&state))
        }
        "clear_repo_intelligence_index" => {
            clear_repo_intelligence_index()?;
            Ok(build_doctor_report(&state))
        }
        "install_repo_memory_mcp" => {
            repair_repo_memory_mcp(&state)?;
            Ok(build_doctor_report(&state))
        }
        "repair_all" => {
            let report = build_doctor_report(&state);
            for issue in report.issues {
                match issue.repair_action.as_deref() {
                    Some("reset_codex_bypass") => {
                        state
                            .codex_bypass
                            .store(false, std::sync::atomic::Ordering::Release);
                        state.invalidate_runtime_status_cache();
                    }
                    Some("repair_runtime") => repair_runtime(&state)?,
                    Some("repair_client_setups") => repair_client_setups(&state)?,
                    Some("repair_codex_setup") => repair_codex_setup(&state)?,
                    Some("repair_rtk_integrations") => repair_rtk_integrations(&state)?,
                    Some("repair_rtk_runtime") => repair_rtk_runtime(&state)?,
                    Some("repair_caveman_guidance") => repair_caveman_guidance(&state)?,
                    Some("repair_ponytail_plugin") => repair_ponytail_plugin(&state)?,
                    Some("clear_repo_intelligence_index") => clear_repo_intelligence_index()?,
                    Some("install_repo_memory_mcp") => repair_repo_memory_mcp(&state)?,
                    _ => {}
                }
            }
            Ok(build_doctor_report(&state))
        }
        other => Err(format!("unknown doctor repair action: {other}")),
    }
}

#[tauri::command]
async fn set_switchboard_mode(
    app: AppHandle,
    mode: SwitchboardMode,
) -> Result<SwitchboardState, String> {
    let state: tauri::State<'_, AppState> = app.state();
    client_adapters::write_switchboard_mode(mode.clone()).map_err(|err| err.to_string())?;

    match mode {
        SwitchboardMode::Off => {
            client_adapters::set_rtk_enabled(
                false,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state.set_runtime_paused(true);
            state.set_runtime_auto_paused(false);
            state
                .codex_bypass
                .store(true, std::sync::atomic::Ordering::Release);
            state.stop_headroom();
            client_adapters::clear_client_setups().map_err(|err| err.to_string())?;
            analytics::track_event(&app, "switchboard_mode_off", None);
        }
        SwitchboardMode::Rtk => {
            client_adapters::set_rtk_enabled(
                true,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state.set_runtime_paused(true);
            state.set_runtime_auto_paused(false);
            state
                .codex_bypass
                .store(true, std::sync::atomic::Ordering::Release);
            state.stop_headroom();
            client_adapters::clear_client_setups().map_err(|err| err.to_string())?;
            analytics::track_event(&app, "switchboard_mode_rtk", None);
        }
        SwitchboardMode::Headroom => {
            client_adapters::set_rtk_enabled(
                false,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state
                .codex_bypass
                .store(false, std::sync::atomic::Ordering::Release);
            state.resume_runtime().map_err(|err| err.to_string())?;
            client_adapters::restore_client_setups();
            analytics::track_event(&app, "switchboard_mode_headroom", None);
        }
        SwitchboardMode::Full => {
            client_adapters::set_rtk_enabled(
                true,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state
                .codex_bypass
                .store(false, std::sync::atomic::Ordering::Release);
            state.resume_runtime().map_err(|err| err.to_string())?;
            client_adapters::restore_client_setups();
            analytics::track_event(&app, "switchboard_mode_full", None);
        }
    }

    state.invalidate_runtime_status_cache();
    build_switchboard_state(&state)
}

#[tauri::command]
async fn set_savings_mode(app: AppHandle, mode: SavingsMode) -> Result<SwitchboardState, String> {
    let state: tauri::State<'_, AppState> = app.state();
    client_adapters::write_savings_mode(mode.clone()).map_err(|err| err.to_string())?;
    if !state.runtime_status().paused {
        repair_runtime(&state)?;
    }
    state.invalidate_runtime_status_cache();
    analytics::track_event(
        &app,
        "switchboard_savings_mode_changed",
        Some(json!({ "mode": format!("{mode:?}").to_ascii_lowercase() })),
    );
    build_switchboard_state(&state)
}

/// Debug-only: force the proxy intercept's bypass flag on/off so a developer
/// can manually exercise the gated path (Python proxy stopped, traffic routed
/// direct to api.anthropic.com) without crossing the real disable threshold.
/// Compiled out of release builds.
#[cfg(debug_assertions)]
#[tauri::command]
fn debug_force_proxy_bypass(state: State<'_, AppState>, on: bool) -> Result<bool, String> {
    log::debug!("[debug_force_proxy_bypass] requested on={on}");
    state
        .proxy_bypass
        .store(on, std::sync::atomic::Ordering::Release);
    log::debug!(
        "[debug_force_proxy_bypass] stored bypass={}",
        state
            .proxy_bypass
            .load(std::sync::atomic::Ordering::Acquire)
    );
    if on {
        state.stop_headroom();
        log::debug!("[debug_force_proxy_bypass] stop_headroom complete");
    } else {
        // Recover from any auto-pause / client teardown that may have run
        // while bypass was active (the watchdog's give-up path or the
        // pricing gate's `disable_client_setup` call).
        client_adapters::restore_client_setups();
        state.set_runtime_paused(false);
        state
            .ensure_headroom_running()
            .map_err(|err| err.to_string())?;
    }
    Ok(state
        .proxy_bypass
        .load(std::sync::atomic::Ordering::Acquire))
}

#[tauri::command]
async fn get_headroom_logs(
    state: State<'_, AppState>,
    max_lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let limit = max_lines.unwrap_or(120).clamp(20, 500);
    state
        .tool_manager
        .read_headroom_log_tail(limit)
        .map_err(|err| err.to_string())
}

/// Authoritative "did the proxy receive a request" signal for the connector
/// verification UI. Reads `/stats` on the live Rust front proxy and returns
/// `requests.total`. The earlier verification path scanned the python proxy
/// log for /v1/messages lines, but Claude Code traffic flows through the
/// Rust proxy on 6767 — the python log only ever sees background/internal
/// activity, so the regex match never fired even when the user's calls were
/// being optimized normally.
///
/// `None` means the proxy is unreachable or `/stats` failed; the frontend
/// must distinguish that from `Some(0)` ("up but no traffic yet"), otherwise
/// a transient unreachable → reachable transition would look like a counter
/// jump from 0 → N and falsely flip the badge to healthy.
#[tauri::command]
async fn get_headroom_request_count() -> Option<u64> {
    fetch_proxy_request_count_stats()
}

fn fetch_proxy_request_count_stats() -> Option<u64> {
    parse_request_count_from_stats_body(&fetch_proxy_stats_body()?)
}

fn fetch_proxy_stats_body() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()
        .ok()?;
    for host in ["127.0.0.1", "localhost"] {
        let url = format!("http://{host}:6767/stats");
        let Ok(response) = client.get(&url).send() else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        if let Ok(body) = response.text() {
            return Some(body);
        }
    }
    None
}

/// Per-agent request counts from `/stats` `agent_usage.agents[]`, keyed by the
/// proxy's agent id (`claude-code`, `codex`, ...). Used by setup verification
/// so a prompt sent to one client only flips that client's row, not all rows.
#[tauri::command]
async fn get_headroom_request_counts_by_agent() -> Option<std::collections::HashMap<String, u64>> {
    parse_request_counts_by_agent(&fetch_proxy_stats_body()?)
}

pub(crate) fn parse_request_counts_by_agent(
    body: &str,
) -> Option<std::collections::HashMap<String, u64>> {
    let root = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let mut counts = std::collections::HashMap::new();
    if let Some(agents) = root
        .get("agent_usage")
        .and_then(|v| v.get("agents"))
        .and_then(|v| v.as_array())
    {
        for agent in agents {
            if let (Some(key), Some(requests)) = (
                agent.get("agent").and_then(|v| v.as_str()),
                agent.get("requests").and_then(|v| v.as_u64()),
            ) {
                counts.insert(key.to_string(), requests);
            }
        }
    }
    Some(counts)
}

/// Pull `requests.total` (or any of the legacy spellings) out of a /stats
/// JSON body. Mirrors the lookup in `state::parse_headroom_stats_from_json`
/// but trimmed to just the counter we need for verification.
pub(crate) fn parse_request_count_from_stats_body(body: &str) -> Option<u64> {
    let root = serde_json::from_str::<serde_json::Value>(body).ok()?;
    if let Some(total) = root
        .get("requests")
        .and_then(|v| v.get("total"))
        .and_then(|v| v.as_u64())
    {
        return Some(total);
    }
    for key in ["total_requests", "totalRequests", "requests_total"] {
        if let Some(total) = find_u64_key_recursive_local(&root, key) {
            return Some(total);
        }
    }
    None
}

fn find_u64_key_recursive_local(value: &serde_json::Value, key: &str) -> Option<u64> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(found) = map.get(key).and_then(|v| v.as_u64()) {
                return Some(found);
            }
            for v in map.values() {
                if let Some(found) = find_u64_key_recursive_local(v, key) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(found) = find_u64_key_recursive_local(item, key) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

#[tauri::command]
async fn get_rtk_activity(
    state: State<'_, AppState>,
    max_lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let limit = max_lines.unwrap_or(120).clamp(20, 500);
    state
        .tool_manager
        .read_rtk_activity(limit)
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_tool_logs(
    state: State<'_, AppState>,
    tool_id: String,
    max_lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let limit = max_lines.unwrap_or(120).clamp(20, 500);
    state
        .tool_manager
        .read_tool_log_tail(&tool_id, limit)
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_claude_code_projects(
    state: State<'_, AppState>,
) -> Result<Vec<ClaudeCodeProject>, String> {
    state
        .list_claude_code_projects()
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_claude_usage(state: State<'_, AppState>) -> Result<ClaudeUsage, String> {
    pricing::fetch_claude_usage(&state)
}

#[tauri::command]
fn get_claude_profile(state: State<'_, AppState>) -> ClaudeAccountProfile {
    pricing::detect_claude_profile(&state)
}

#[tauri::command]
async fn get_headroom_pricing_status(
    state: State<'_, AppState>,
) -> Result<HeadroomPricingStatus, String> {
    let status = pricing::get_pricing_status(&state)?;
    // Reconcile the runtime with the freshly evaluated status. Bridges the
    // gap between "user just upgraded" (subscription_active flips on) and
    // "Headroom optimization actually resumes" — without this, the pricing
    // gate's bypass flag would stay set and Python would stay down until
    // the next app launch.
    state.apply_pricing_gate_status(&status);
    state.apply_codex_pricing_gate_status(status.codex.as_ref());
    Ok(status)
}

#[tauri::command]
async fn request_headroom_auth_code(
    app: AppHandle,
    state: State<'_, AppState>,
    email: String,
) -> Result<HeadroomAuthCodeRequest, String> {
    let request = pricing::request_auth_code(&state, &email)?;
    analytics::track_event(&app, "auth_code_requested", None);
    Ok(request)
}

#[tauri::command]
async fn verify_headroom_auth_code(
    app: AppHandle,
    state: State<'_, AppState>,
    email: String,
    code: String,
    invite_code: Option<String>,
) -> Result<HeadroomPricingStatus, String> {
    let used_invite_code = invite_code
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    let status = pricing::verify_auth_code(&state, &email, &code, invite_code.as_deref())?;
    // Reconcile the runtime with the freshly evaluated status. Mirrors
    // `get_headroom_pricing_status` so a user who signs up after grace
    // expiry doesn't have to wait for the next 60s pricing poll for
    // Python to come back online.
    state.apply_pricing_gate_status(&status);
    state.apply_codex_pricing_gate_status(status.codex.as_ref());
    analytics::track_event(
        &app,
        "auth_verified",
        Some(json!({ "invite_code_used": used_invite_code })),
    );
    Ok(status)
}

#[tauri::command]
async fn sign_out_headroom_account() -> Result<(), String> {
    pricing::sign_out()
}

#[tauri::command]
async fn activate_headroom_account(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<HeadroomPricingStatus, String> {
    let lifetime_tokens_saved = state.dashboard().lifetime_estimated_tokens_saved;
    let status = pricing::activate_account(&state, lifetime_tokens_saved)?;
    analytics::track_event(&app, "account_activated", None);
    Ok(status)
}

#[tauri::command]
async fn create_headroom_checkout_session(
    app: AppHandle,
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
) -> Result<String, String> {
    let url = pricing::create_checkout_session(subscription_tier.clone(), billing_period)?;
    analytics::track_event(
        &app,
        "checkout_started",
        Some(json!({
            "subscription_tier": subscription_tier_label(&subscription_tier)
        })),
    );
    Ok(url)
}

#[tauri::command]
async fn change_headroom_subscription_plan(
    app: AppHandle,
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
) -> Result<(), String> {
    pricing::change_subscription_plan(subscription_tier.clone(), billing_period)?;
    analytics::track_event(
        &app,
        "subscription_plan_changed",
        Some(json!({
            "subscription_tier": subscription_tier_label(&subscription_tier)
        })),
    );
    Ok(())
}

#[tauri::command]
async fn reactivate_headroom_subscription(app: AppHandle) -> Result<(), String> {
    pricing::reactivate_subscription()?;
    analytics::track_event(&app, "subscription_reactivated", None);
    Ok(())
}

#[tauri::command]
async fn get_headroom_billing_portal_url(target: Option<String>) -> Result<String, String> {
    pricing::get_billing_portal_url(target)
}

#[tauri::command]
fn get_headroom_learn_status(
    state: State<'_, AppState>,
    project_path: Option<String>,
) -> HeadroomLearnStatus {
    state.headroom_learn_status(project_path.as_deref())
}

#[tauri::command]
fn get_headroom_learn_prereq_status(
    state: State<'_, AppState>,
    force: Option<bool>,
) -> HeadroomLearnPrereqStatus {
    if force.unwrap_or(false) {
        state.invalidate_headroom_learn_prereq_cache();
    }
    state.headroom_learn_prereq_status()
}

#[tauri::command]
async fn get_transformations_feed(limit: Option<u32>) -> TransformationFeedResponse {
    let limit = limit.unwrap_or(50).min(100);
    let settings = message_logging::load_settings();
    fetch_transformations_feed(limit).unwrap_or_else(|_| TransformationFeedResponse {
        log_full_messages: false,
        full_message_logging_expires_at: settings.full_message_logging_expires_at,
        message_log_retention_hours: settings.message_log_retention_hours,
        transformations: Vec::new(),
        proxy_reachable: false,
    })
}

#[tauri::command]
fn get_message_logging_settings() -> MessageLoggingSettings {
    message_logging::load_settings()
}

#[tauri::command]
fn set_message_logging_settings(
    settings: MessageLoggingSettings,
) -> Result<MessageLoggingSettings, String> {
    message_logging::save_settings(&settings).map_err(|err| err.to_string())
}

#[tauri::command]
fn enable_full_message_logging(hours: u32) -> Result<MessageLoggingSettings, String> {
    let settings = MessageLoggingSettings::enabled_for(hours);
    message_logging::save_settings(&settings).map_err(|err| err.to_string())
}

#[tauri::command]
fn disable_full_message_logging() -> Result<MessageLoggingSettings, String> {
    let settings = MessageLoggingSettings {
        full_message_logging: false,
        full_message_logging_expires_at: None,
        message_log_retention_hours: 24,
    };
    message_logging::save_settings(&settings).map_err(|err| err.to_string())
}

#[tauri::command]
fn purge_message_logs(state: State<'_, AppState>) -> PurgeResult {
    state.purge_message_logs()
}

#[tauri::command]
fn get_codex_thread_retagging_settings() -> CodexThreadRetaggingSettings {
    client_adapters::get_codex_thread_retagging_settings()
}

#[tauri::command]
fn set_codex_thread_retagging_settings(
    settings: CodexThreadRetaggingSettings,
) -> Result<CodexThreadRetaggingSettings, String> {
    client_adapters::set_codex_thread_retagging_settings(settings).map_err(|err| err.to_string())
}

#[tauri::command]
fn restore_codex_thread_db_backup(path: String) -> Result<CodexDbRestoreResult, String> {
    client_adapters::restore_codex_thread_db_backup(&path).map_err(|err| err.to_string())
}

/// Read-only snapshot of the activity feed. Observation — fetching the proxy,
/// writing to ActivityFacts, persisting — happens on a dedicated background
/// timer (see `spawn_activity_observer`), so this command never mutates state.
/// That keeps the IPC hot path short: one in-memory lock + a cheap /readyz
/// ping to the local proxy.
#[tauri::command]
fn get_activity_feed(state: State<'_, AppState>) -> ActivityFeedResponse {
    ActivityFeedResponse {
        tiles: state.activity_feed_snapshot(),
        proxy_reachable: crate::state::headroom_proxy_reachable(),
    }
}

/// Observation cadence for background activity milestones. A modest delay is
/// fine here; foreground Activity still polls separately, and the
/// memory-export path is intentionally kept away from tight loops.
const ACTIVITY_OBSERVER_INTERVAL: std::time::Duration = std::time::Duration::from_secs(20);
/// Rescan cadence for the Claude projects cache. This keeps Optimize mostly
/// warm without doing filesystem-heavy project scans every minute forever.
const CLAUDE_PROJECTS_WARM_INTERVAL: std::time::Duration = std::time::Duration::from_secs(75);
/// Matches the frontend's `ACTIVITY_FEED_WINDOW` in App.tsx so the observer
/// sees the same transformations the UI will display.
const ACTIVITY_OBSERVER_LIMIT: u32 = 150;

fn spawn_activity_observer(app: AppHandle) {
    std::thread::spawn(move || {
        // Small warm-up so we don't race with runtime bring-up; the first
        // proxy fetch lands a few seconds after the proxy is actually up.
        std::thread::sleep(std::time::Duration::from_secs(3));
        loop {
            run_activity_observation(&app);
            std::thread::sleep(ACTIVITY_OBSERVER_INTERVAL);
        }
    });
}

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

fn run_activity_observation(app: &AppHandle) {
    let state: tauri::State<'_, AppState> = app.state();

    let _ = state.maybe_emit_weekly_recap();

    if let Ok(feed) = fetch_transformations_feed(ACTIVITY_OBSERVER_LIMIT) {
        let _ = state.observe_activity_from_transformations(&feed.transformations);
    }

    let projects = state.list_claude_code_projects().unwrap_or_default();

    // Memory.db "patterns today" comes from the export JSON's `created_at`
    // field. Everything else (reminders / learnings today) is derived from
    // per-project CLAUDE.md + MEMORY.md bullet diffs.
    let memory_path = headroom_memory_db_path();
    let patterns_today = if memory_path.exists() {
        memory_export_cached(&state, &memory_path)
            .ok()
            .and_then(|stdout| count_memories_created_today(&stdout, Utc::now()).ok())
            .unwrap_or(0) as u32
    } else {
        0
    };

    // Collect current bullet sets for every project the user has touched
    // today, so `observe_learnings_today` has a baseline regardless of which
    // one ends up being "most active".
    let project_inputs: Vec<crate::activity_facts::LearningsProjectInput> = projects
        .iter()
        .filter(|p| p.sessions_today > 0)
        .map(|p| {
            let applied = read_applied_patterns_for_project(&p.project_path);
            crate::activity_facts::LearningsProjectInput {
                project_path: p.project_path.clone(),
                project_display_name: p.display_name.clone(),
                claude_md_bullets: flatten_applied_bullets(&applied.claude_md),
                memory_md_bullets: flatten_applied_bullets(&applied.memory_md),
            }
        })
        .collect();

    // Most active = highest sessions_today; ties broken by most recent
    // last_worked_at so the chip tracks what the user is working on right now.
    let active_project_path = projects
        .iter()
        .filter(|p| p.sessions_today > 0)
        .max_by(|a, b| {
            a.sessions_today
                .cmp(&b.sessions_today)
                .then(a.last_worked_at.cmp(&b.last_worked_at))
        })
        .map(|p| p.project_path.clone());

    let _ = state.observe_learnings_today(
        patterns_today,
        project_inputs,
        active_project_path.as_deref(),
    );

    // No point nudging the user to run Train if the claude CLI isn't installed —
    // they'd just hit an install prompt. The Optimize tab surfaces the install
    // UI in that case; let them fix prereqs first.
    if state.headroom_learn_prereq_status().claude_cli_available {
        let _ = state.observe_train_suggestions(&projects);
    }
}

fn flatten_applied_bullets(sections: &[crate::models::AppliedSection]) -> Vec<String> {
    sections
        .iter()
        .flat_map(|sec| sec.bullets.iter().cloned())
        .collect()
}

#[tauri::command]
async fn list_live_learnings(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<Vec<crate::models::LiveLearning>, String> {
    let memory_path = headroom_memory_db_path();
    if !memory_path.exists() {
        return Ok(Vec::new());
    }
    let stdout = memory_export_cached(&state, &memory_path)?;
    parse_live_learnings(&stdout, &project_path)
}

#[tauri::command]
async fn list_live_learnings_for_projects(
    state: State<'_, AppState>,
    project_paths: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<crate::models::LiveLearning>>, String> {
    let memory_path = headroom_memory_db_path();
    if !memory_path.exists() {
        return Ok(empty_live_learnings_for_projects(&project_paths));
    }
    let stdout = memory_export_cached(&state, &memory_path)?;
    aggregate_live_learnings(&stdout, &project_paths)
}

fn empty_live_learnings_for_projects(
    project_paths: &[String],
) -> std::collections::HashMap<String, Vec<crate::models::LiveLearning>> {
    let mut out = std::collections::HashMap::with_capacity(project_paths.len());
    for p in project_paths {
        out.insert(p.clone(), Vec::new());
    }
    out
}

fn aggregate_live_learnings(
    stdout: &str,
    project_paths: &[String],
) -> Result<std::collections::HashMap<String, Vec<crate::models::LiveLearning>>, String> {
    let mut out = std::collections::HashMap::with_capacity(project_paths.len());
    for p in project_paths {
        let learnings = parse_live_learnings(stdout, p)?;
        out.insert(p.clone(), learnings);
    }
    Ok(out)
}

fn memory_export_cached(state: &State<'_, AppState>, memory_path: &Path) -> Result<String, String> {
    if let Some(cached) = state.cached_memory_export() {
        return Ok(cached);
    }
    let entrypoint = state.tool_manager.headroom_entrypoint();
    let stdout = run_memory_export(&entrypoint, memory_path)?;
    state.store_memory_export(stdout.clone());
    Ok(stdout)
}

#[tauri::command]
async fn delete_live_learning(state: State<'_, AppState>, memory_id: String) -> Result<(), String> {
    let memory_path = headroom_memory_db_path();
    if !memory_path.exists() {
        return Err("Memory database does not exist.".into());
    }
    let entrypoint = state.tool_manager.headroom_entrypoint();
    let output = Command::new(&entrypoint)
        .arg("memory")
        .arg("delete")
        .arg(&memory_id)
        .arg("--force")
        .arg("--db-path")
        .arg(&memory_path)
        .env("PYTHONNOUSERSITE", "1")
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "headroom memory delete failed ({}): {}",
            output.status,
            stderr.trim()
        ));
    }
    state.invalidate_memory_export_cache();
    Ok(())
}

#[tauri::command]
async fn list_applied_patterns(
    project_path: String,
) -> Result<crate::models::AppliedPatterns, String> {
    Ok(read_applied_patterns_for_project(&project_path))
}

#[tauri::command]
async fn list_applied_patterns_for_projects(
    project_paths: Vec<String>,
) -> Result<std::collections::HashMap<String, crate::models::AppliedPatterns>, String> {
    let mut out = std::collections::HashMap::with_capacity(project_paths.len());
    for p in project_paths {
        let patterns = read_applied_patterns_for_project(&p);
        out.insert(p, patterns);
    }
    Ok(out)
}

fn read_applied_patterns_for_project(project_path: &str) -> crate::models::AppliedPatterns {
    let claude_md = std::path::PathBuf::from(project_path).join("CLAUDE.md");
    let memory_md = crate::tool_manager::claude_project_memory_file(project_path);

    crate::models::AppliedPatterns {
        claude_md: read_applied_block(&claude_md),
        memory_md: read_applied_block(&memory_md),
    }
}

#[tauri::command]
async fn delete_applied_pattern(
    project_path: String,
    file_kind: String,
    section_title: String,
    bullet_text: String,
) -> Result<(), String> {
    let path = match file_kind.as_str() {
        "claude" => std::path::PathBuf::from(&project_path).join("CLAUDE.md"),
        "memory" => crate::tool_manager::claude_project_memory_file(&project_path),
        other => return Err(format!("Unknown file_kind: {other}")),
    };
    if !path.exists() {
        return Err(format!("{} does not exist.", path.display()));
    }
    let content =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let updated =
        crate::tool_manager::delete_applied_bullet(&content, &section_title, &bullet_text);
    if updated == content {
        return Ok(()); // no-op; nothing to write
    }
    std::fs::write(&path, updated).map_err(|err| format!("write {}: {err}", path.display()))?;
    Ok(())
}

fn read_applied_block(path: &std::path::Path) -> Vec<crate::models::AppliedSection> {
    match std::fs::read_to_string(path) {
        Ok(content) => crate::tool_manager::parse_headroom_learn_block(&content),
        Err(_) => Vec::new(),
    }
}

/// Shells `headroom memory export --db-path <db>` and returns raw JSON stdout.
fn run_memory_export(entrypoint: &Path, db_path: &Path) -> Result<String, String> {
    let output = Command::new(entrypoint)
        .arg("memory")
        .arg("export")
        .arg("--db-path")
        .arg(db_path)
        .env("PYTHONNOUSERSITE", "1")
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!("headroom memory export exited {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_live_learnings(
    json: &str,
    project_path: &str,
) -> Result<Vec<crate::models::LiveLearning>, String> {
    #[derive(serde::Deserialize)]
    struct Raw {
        id: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        created_at: Option<String>,
        #[serde(default)]
        importance: Option<f64>,
        #[serde(default)]
        metadata: serde_json::Value,
        #[serde(default)]
        entity_refs: Vec<String>,
    }

    let raws: Vec<Raw> = serde_json::from_str(json.trim()).map_err(|err| err.to_string())?;
    let mut out: Vec<crate::models::LiveLearning> = Vec::new();
    for r in raws {
        let source = r
            .metadata
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if source != "traffic_learner" {
            continue;
        }
        if !pattern_matches_project(&r.content, &r.entity_refs, project_path) {
            continue;
        }
        let category = r
            .metadata
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let evidence_count = r
            .metadata
            .get("evidence_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        out.push(crate::models::LiveLearning {
            id: r.id,
            content: r.content,
            category,
            importance: r.importance.unwrap_or(0.5),
            evidence_count,
            created_at: r.created_at.unwrap_or_default(),
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// True if any absolute path in `content` or `entity_refs` is under `project_path`.
fn pattern_matches_project(content: &str, entity_refs: &[String], project_path: &str) -> bool {
    let root = project_path.trim_end_matches('/');
    if root.is_empty() {
        return false;
    }
    let needle_slash = format!("{root}/");
    if content.contains(root) {
        // Guard against /x/ab matching /x/a — require either exact or followed by /
        if content.contains(&needle_slash)
            || content.contains(&format!("{root}\""))
            || content.contains(&format!("{root}`"))
        {
            return true;
        }
    }
    for r in entity_refs {
        if r == root || r.starts_with(&needle_slash) {
            return true;
        }
    }
    false
}

#[tauri::command]
async fn start_headroom_learn(
    app: AppHandle,
    agent: String,
    project_path: Option<String>,
) -> Result<(), String> {
    let agent = LearnAgent::parse(&agent)?;
    if matches!(agent, LearnAgent::Claude) && project_path.is_none() {
        return Err("A project path is required for Claude Headroom Learn.".into());
    }
    check_headroom_learn_prereqs(
        agent,
        crate::state::headroom_learn_platform_message().as_deref(),
        &detect_headroom_learn_prereq_status(),
    )?;

    // Codex isn't project-organized, so its run-status is keyed on a stable id.
    let run_key = match agent {
        LearnAgent::Claude => project_path.clone().unwrap_or_default(),
        LearnAgent::Codex => "codex".to_string(),
    };
    {
        let state: tauri::State<'_, AppState> = app.state();
        state.begin_headroom_learn_run(&run_key)?;
    }

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, AppState> = app_handle.state();
        let run = execute_headroom_learn_run(&state, agent, project_path.as_deref());
        state.complete_headroom_learn_run(run.success, run.summary, run.error, run.output_tail);
    });

    Ok(())
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
async fn open_headroom_dashboard() -> Result<(), String> {
    open_external_link_impl(HEADROOM_DASHBOARD_URL)
        .map_err(|err| format!("Failed to open Headroom dashboard: {err}"))
}

fn open_external_link_impl(url: &str) -> Result<(), String> {
    let trimmed = validate_external_link_url(url)?;

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(&trimmed);
        command
    };

    #[cfg(target_os = "linux")]
    {
        for opener in ["xdg-open", "gio", "kde-open5", "wslview"] {
            let mut command = Command::new(opener);
            if opener == "gio" {
                command.args(["open", &trimmed]);
            } else {
                command.arg(&trimmed);
            }
            match command.status() {
                Ok(status) if status.success() => return Ok(()),
                Ok(_) => continue,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => {
                    return Err(format!(
                        "Could not launch external link with {opener}: {err}"
                    ))
                }
            }
        }
        return Err(
            "No URL opener found. Install xdg-utils (provides xdg-open) to open links.".into(),
        );
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", trimmed.as_str()]);
        command
    };

    #[cfg(not(target_os = "linux"))]
    {
        let status = command
            .status()
            .map_err(|err| format!("Could not launch external link: {err}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("External link opener exited with {status}."))
        }
    }
}

fn validate_external_link_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("External link is empty.".into());
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("External links cannot contain line breaks.".into());
    }

    if trimmed.starts_with("mailto:") {
        let address = trimmed.trim_start_matches("mailto:");
        if address.is_empty() || address.contains('?') || address.contains('/') {
            return Err("Only simple mailto links are supported.".into());
        }
        return Ok(trimmed.to_string());
    }

    let parsed =
        reqwest::Url::parse(trimmed).map_err(|_| "External link URL is invalid.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only http, https, and mailto links are supported.".into());
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Err("External links cannot include embedded credentials.".into());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "External link must include a host.".to_string())?;
    if is_blocked_external_link_host(host) {
        return Err("External link host is not allowed.".into());
    }

    Ok(trimmed.to_string())
}

fn is_blocked_external_link_host(host: &str) -> bool {
    let normalized = host
        .trim_matches(|ch| ch == '[' || ch == ']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if matches!(normalized.as_str(), "localhost" | "localhost.localdomain") {
        return true;
    }
    if normalized.ends_with(".localhost") || normalized.ends_with(".local") {
        return true;
    }
    match normalized.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified()
        }
        Ok(std::net::IpAddr::V6(ip)) => {
            ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local()
        }
        Err(_) => false,
    }
}

#[tauri::command]
async fn open_external_link(url: String) -> Result<(), String> {
    open_external_link_impl(&url)
}

#[tauri::command]
fn track_analytics_event(app: AppHandle, name: String, properties: Option<Value>) {
    analytics::track_event(&app, &name, properties);
}

#[tauri::command]
async fn submit_contact_request(
    url: String,
    email: String,
    message: Option<String>,
) -> Result<(), String> {
    reject_contact_request_in_local_only()?;
    let trimmed = email.trim();
    if trimmed.is_empty() || !trimmed.contains('@') {
        return Err("Enter a valid email address.".to_string());
    }

    let target = validate_contact_request_url(&url)
        .ok_or_else(|| "Could not reach the contact form.".to_string())?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| err.to_string())?;
    let message_owned = message
        .map(|m| m.trim().chars().take(2000).collect::<String>())
        .unwrap_or_default();
    let response = client
        .post(target)
        .form(&[
            ("contact_request[email]", trimmed),
            ("contact_request[message]", message_owned.as_str()),
        ])
        .send()
        .await
        .map_err(|err| err.to_string())?;

    // Rails answers a successful POST with a 302 to /#pricing. Redirect policy
    // is none for SSRF defense, so accept 3xx as success here. 422 and 503 are
    // the controller's explicit error renders.
    match response.status().as_u16() {
        200..=399 => Ok(()),
        422 => Err("Enter a valid email address.".to_string()),
        503 => Err("Email delivery still needs to be configured.".to_string()),
        status => Err(format!("Contact request failed with status {status}.")),
    }
}

fn reject_contact_request_in_local_only() -> Result<(), String> {
    if local_mode::enabled() {
        Err("Support/contact requests are disabled in local-only mode.".to_string())
    } else {
        Ok(())
    }
}

// Scheme + host allowlist for the contact form endpoint. The URL reaches this
// Tauri command from the webview, so we must not assume it is trustworthy —
// an SSRF primitive here would let a compromised frame POST to arbitrary
// hosts, including loopback services.
fn validate_contact_request_url(raw: &str) -> Option<reqwest::Url> {
    const ALLOWED_HOSTS: &[&str] = &["github.com"];
    let parsed = reqwest::Url::parse(raw).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    let host = parsed.host_str()?;
    if !ALLOWED_HOSTS.contains(&host) {
        return None;
    }
    Some(parsed)
}

#[tauri::command]
async fn apply_client_setup(
    app: AppHandle,
    client_id: String,
) -> Result<ClientSetupResult, String> {
    // Two recovery paths land on the tray-banner "Re-enable" button:
    //   1. Watchdog give-up — pauses the runtime and clears client setups.
    //   2. Pricing gate (grace expiry, weekly cap) — sets `proxy_bypass` and
    //      calls `stop_headroom()` without flipping `runtime_paused`.
    // Both leave Python stopped, so re-enable has to clear bypass and bring
    // the runtime back. Without this, env vars get rewritten but the proxy
    // stays down and Claude Code traffic flows unoptimized until the next
    // pricing poll (or, in the watchdog case, until restart).
    let state: tauri::State<'_, AppState> = app.state();
    let bypassed = state
        .proxy_bypass
        .load(std::sync::atomic::Ordering::Acquire);
    if state.runtime_is_paused() || bypassed {
        if let Err(err) = state.resume_runtime() {
            log::warn!("apply_client_setup: resume_runtime failed: {err:#}");
        }
    }
    match client_adapters::apply_client_setup(&client_id) {
        Ok(result) => {
            analytics::track_event(
                &app,
                "client_setup_applied",
                Some(json!({
                    "client_id": result.client_id.clone(),
                    "already_configured": result.already_configured,
                    "verified": result.verification.verified,
                    "proxy_reachable": result.verification.proxy_reachable
                })),
            );
            // Setup returned Ok, but the post-write verification read the
            // files back and found the expected side effect missing. That's
            // the same class of bug as the MCP fallback silent-success —
            // subprocess/file-write succeeded yet the integration is not
            // actually in place. Capture to Sentry so we see it.
            if !result.verification.verified {
                sentry::with_scope(
                    |scope| {
                        scope.set_extra(
                            "proxy_reachable",
                            result.verification.proxy_reachable.into(),
                        );
                        scope.set_extra("checks", json!(result.verification.checks).into());
                        scope.set_extra("failures", json!(result.verification.failures).into());
                        scope.set_extra("already_configured", result.already_configured.into());
                    },
                    || {
                        sentry::capture_message(
                            &format!(
                                "client setup for {client_id} completed but verification failed",
                            ),
                            sentry::Level::Warning,
                        );
                    },
                );
            }
            Ok(result)
        }
        Err(err) => {
            let msg = err.to_string();
            if !msg.starts_with("Automatic setup is not supported yet") {
                sentry::capture_message(
                    &format!("client setup failed for {client_id}: {err:#}"),
                    sentry::Level::Error,
                );
            }
            Err(msg)
        }
    }
}

#[tauri::command]
async fn verify_client_setup(client_id: String) -> Result<ClientSetupVerification, String> {
    client_adapters::verify_client_setup(&client_id).map_err(|err| err.to_string())
}

fn connector_smoke_working_dir() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn tail_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }
    value
        .chars()
        .skip(char_count.saturating_sub(max_chars))
        .collect()
}

#[tauri::command]
async fn run_connector_smoke_test(client_id: String) -> Result<ConnectorSmokeTestResult, String> {
    let prompt = "Reply with exactly: switchboard verification ok";
    let mut command = match client_id.as_str() {
        "codex" => {
            let mut command = Command::new("codex");
            command.args([
                "exec",
                "--ephemeral",
                "--sandbox",
                "read-only",
                "--skip-git-repo-check",
                "--ignore-rules",
                prompt,
            ]);
            command
        }
        "claude_code" => {
            let mut command = Command::new("claude");
            command.args([
                "--print",
                "--no-session-persistence",
                "--permission-mode",
                "dontAsk",
                "--tools",
                "",
                "--output-format",
                "text",
                prompt,
            ]);
            command
        }
        _ => {
            return Ok(ConnectorSmokeTestResult {
                client_id,
                supported: false,
                launched: false,
                success: false,
                summary: "One-click test is available for Claude Code and Codex. For this connector, send a tiny prompt manually and watch this screen for verification.".into(),
                stdout_tail: String::new(),
                stderr_tail: String::new(),
            });
        }
    };

    command.current_dir(connector_smoke_working_dir());
    command.env("NO_COLOR", "1");
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| format!("Could not launch {client_id} smoke test: {err}"))?;

    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(250));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{client_id} smoke test timed out after 90 seconds."
                ));
            }
            Err(err) => return Err(format!("{client_id} smoke test failed to run: {err}")),
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Could not collect {client_id} smoke test output: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();
    Ok(ConnectorSmokeTestResult {
        client_id,
        supported: true,
        launched: true,
        success,
        summary: if success {
            "Test prompt sent. Waiting for the local proxy to confirm the request.".into()
        } else {
            format!(
                "Test prompt exited with status {}. Open the connector and send a tiny prompt manually.",
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "terminated".into())
            )
        },
        stdout_tail: tail_text(&stdout, 800),
        stderr_tail: tail_text(&stderr, 1200),
    })
}

#[tauri::command]
async fn get_client_connectors(
    state: State<'_, AppState>,
) -> Result<Vec<ClientConnectorStatus>, String> {
    client_adapters::list_client_connectors(&state.cached_clients()).map_err(|err| err.to_string())
}

#[tauri::command]
async fn disable_client_setup(app: AppHandle, client_id: String) -> Result<(), String> {
    client_adapters::disable_client_setup(&client_id).map_err(|err| err.to_string())?;
    analytics::track_event(
        &app,
        "client_setup_disabled",
        Some(json!({ "client_id": client_id })),
    );
    Ok(())
}

#[tauri::command]
async fn clear_client_setups() -> Result<(), String> {
    client_adapters::clear_client_setups().map_err(|err| err.to_string())
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
        match serde_json::to_string_pretty(&client_adapters::get_managed_footprint()) {
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
        match serde_json::to_string_pretty(&client_adapters::uninstall_dry_run_report()) {
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
        match client_adapters::restore_codex_thread_db_backup(path) {
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
            spawn_activity_observer(app.handle().clone());
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

            let wants_headroom = saved_switchboard_mode_wants_headroom();
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
                        client_adapters::retag_codex_threads_to_headroom();
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
        .manage(PendingAppUpdate(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            get_dashboard_state,
            get_savings_attribution_events,
            preview_managed_config_apply,
            execute_managed_config_apply,
            preview_managed_rollback,
            execute_managed_rollback,
            get_managed_footprint,
            get_uninstall_dry_run_report,
            preview_managed_rollback_undo_all,
            execute_managed_rollback_undo_all,
            build_repo_intelligence_summary,
            get_latest_repo_intelligence_summary,
            get_repo_intelligence_context_pack,
            search_repo_intelligence_symbols,
            get_repo_intelligence_dependents,
            get_repo_intelligence_manifest,
            clear_repo_intelligence_summary,
            get_repo_manifest,
            get_repo_pack,
            get_agent_handoff,
            get_index_freshness,
            clear_repo_index,
            get_app_update_configuration,
            load_release_readiness_report,
            refresh_release_readiness_report,
            run_release_evidence_command,
            check_for_app_update,
            install_app_update,
            restart_app,
            show_app_update_notification,
            show_notification,
            install_addon,
            set_addon_enabled,
            uninstall_addon,
            set_caveman_level,
            install_repo_memory_mcp,
            start_repo_memory_mcp,
            stop_repo_memory_mcp,
            bootstrap_runtime,
            start_bootstrap,
            get_bootstrap_progress,
            get_runtime_upgrade_progress,
            retry_runtime_upgrade,
            retry_runtime_upgrade_with_rebuild,
            dismiss_runtime_upgrade_failure,
            get_runtime_status,
            get_switchboard_state,
            get_doctor_report,
            run_doctor_repair,
            set_switchboard_mode,
            set_savings_mode,
            get_headroom_logs,
            get_headroom_request_count,
            get_headroom_request_counts_by_agent,
            get_rtk_activity,
            get_tool_logs,
            get_claude_code_projects,
            get_claude_usage,
            get_claude_profile,
            get_headroom_pricing_status,
            request_headroom_auth_code,
            verify_headroom_auth_code,
            sign_out_headroom_account,
            activate_headroom_account,
            create_headroom_checkout_session,
            change_headroom_subscription_plan,
            reactivate_headroom_subscription,
            get_headroom_billing_portal_url,
            get_activity_feed,
            get_message_logging_settings,
            set_message_logging_settings,
            enable_full_message_logging,
            disable_full_message_logging,
            purge_message_logs,
            get_codex_thread_retagging_settings,
            set_codex_thread_retagging_settings,
            restore_codex_thread_db_backup,
            list_live_learnings,
            list_live_learnings_for_projects,
            delete_live_learning,
            list_applied_patterns,
            list_applied_patterns_for_projects,
            delete_applied_pattern,
            get_headroom_learn_status,
            get_headroom_learn_prereq_status,
            get_transformations_feed,
            start_headroom_learn,
            apply_client_setup,
            verify_client_setup,
            run_connector_smoke_test,
            get_client_connectors,
            disable_client_setup,
            clear_client_setups,
            pause_headroom,
            start_headroom,
            force_restart_headroom,
            track_analytics_event,
            show_dashboard_window,
            open_headroom_dashboard,
            open_external_link,
            submit_contact_request,
            hide_launcher_animated,
            complete_setup_wizard,
            accept_terms,
            get_autostart_enabled,
            set_autostart_enabled,
            set_rtk_enabled,
            uninstall_and_quit,
            quit_headroom,
            #[cfg(debug_assertions)]
            debug_force_proxy_bypass
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
                client_adapters::retag_codex_threads_to_native();
            }
        });
}

fn subscription_tier_label(tier: &HeadroomSubscriptionTier) -> &'static str {
    match tier {
        HeadroomSubscriptionTier::Pro => "pro",
        HeadroomSubscriptionTier::Max5x => "max5x",
        HeadroomSubscriptionTier::Max20x => "max20x",
    }
}

fn lifetime_token_milestone_kind(milestone_tokens_saved: u64) -> &'static str {
    match milestone_tokens_saved {
        1_000_000 => "first_1m",
        5_000_000 => "first_5m",
        10_000_000 => "first_10m",
        _ => "repeating_10m",
    }
}

fn is_prerelease_version(version: &str) -> bool {
    version.contains('-')
}

fn beta_channel_enabled_from(env: Option<&str>, sentinel_exists: bool) -> bool {
    let env_yes = matches!(
        env.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("1") | Some("true") | Some("yes")
    );
    env_yes || sentinel_exists
}

fn beta_channel_enabled() -> bool {
    let env = std::env::var(BETA_CHANNEL_ENV).ok();
    let sentinel_exists = crate::storage::app_data_dir()
        .join(BETA_CHANNEL_SENTINEL)
        .exists();
    beta_channel_enabled_from(env.as_deref(), sentinel_exists)
}

fn select_updater_endpoints<'a>(
    configured_stable: Option<&'a str>,
    configured_staging: Option<&'a str>,
    prefer_staging: bool,
) -> Option<&'a str> {
    if prefer_staging {
        configured_staging.or(configured_stable)
    } else {
        configured_stable
    }
}

fn release_updater_config(
    current_version: &str,
    beta_channel_enabled: bool,
) -> Result<Option<ReleaseUpdaterConfig>, String> {
    resolve_release_updater_config(
        current_version,
        beta_channel_enabled,
        UPDATER_PUBLIC_KEY,
        UPDATER_ENDPOINTS,
        UPDATER_STAGING_ENDPOINTS,
        cfg!(debug_assertions),
    )
}

fn resolve_release_updater_config(
    current_version: &str,
    beta_channel_enabled: bool,
    configured_pubkey: Option<&str>,
    configured_stable: Option<&str>,
    configured_staging: Option<&str>,
    _debug_assertions: bool,
) -> Result<Option<ReleaseUpdaterConfig>, String> {
    let configured_pubkey = configured_pubkey
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let configured_stable = configured_stable
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let configured_staging = configured_staging
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let prefer_staging = is_prerelease_version(current_version) || beta_channel_enabled;
    let configured_endpoints =
        select_updater_endpoints(configured_stable, configured_staging, prefer_staging);

    match (configured_pubkey, configured_endpoints) {
        (Some(pubkey), Some(endpoint_spec)) => {
            build_release_updater_config(pubkey, endpoint_spec).map(Some)
        }
        (Some(_), None) => Err(
            "Updater public key is configured, but HEADROOM_UPDATER_ENDPOINTS is missing."
                .to_string(),
        ),
        (None, Some(_)) => Err(
            "HEADROOM_UPDATER_ENDPOINTS is configured, but HEADROOM_UPDATER_PUBLIC_KEY is missing."
                .to_string(),
        ),
        (None, None) => Ok(None),
    }
}

fn build_release_updater_config(
    pubkey: &str,
    endpoint_spec: &str,
) -> Result<ReleaseUpdaterConfig, String> {
    let endpoints = parse_updater_endpoint_list(endpoint_spec)?;

    if endpoints.is_empty() {
        return Err("HEADROOM_UPDATER_ENDPOINTS did not include any valid URLs.".into());
    }

    Ok(ReleaseUpdaterConfig {
        pubkey: pubkey.to_string(),
        endpoints,
    })
}

fn parse_updater_endpoint_list(raw: &str) -> Result<Vec<reqwest::Url>, String> {
    let values = if let Ok(json) = serde_json::from_str::<Vec<String>>(raw) {
        let values = json
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if !values.is_empty() {
            values
        } else {
            Vec::new()
        }
    } else {
        raw.split(|ch| ch == ',' || ch == '\n')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    };

    if values.is_empty() {
        return Err(
            "HEADROOM_UPDATER_ENDPOINTS must be a JSON array or comma-separated list of HTTPS URLs."
                .into(),
        );
    }

    values
        .into_iter()
        .map(|value| {
            let url = reqwest::Url::parse(&value)
                .map_err(|err| format!("Invalid updater URL {value}: {err}"))?;
            if url.scheme() != "https" {
                return Err(format!("Updater endpoint {} must use HTTPS.", url.as_str()));
            }
            Ok(url)
        })
        .collect()
}

pub fn headroom_memory_db_path() -> std::path::PathBuf {
    crate::storage::memory_db_path(&crate::storage::app_data_dir())
}

/// Which coding agent a Headroom Learn run targets. Determines the session
/// source, the analysis backend, and which context/memory files get written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LearnAgent {
    Claude,
    Codex,
}

impl LearnAgent {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "claude" => Ok(LearnAgent::Claude),
            "codex" => Ok(LearnAgent::Codex),
            other => Err(format!("Unknown Headroom Learn agent: {other}")),
        }
    }
}

pub(crate) fn detect_headroom_learn_prereq_status() -> HeadroomLearnPrereqStatus {
    let claude_path = claude_cli::detect_claude_cli();
    let codex_path = client_adapters::detect_codex_cli();
    HeadroomLearnPrereqStatus {
        claude_cli_available: claude_path.is_some(),
        claude_cli_path: claude_path.map(|p| p.display().to_string()),
        codex_cli_available: codex_path.is_some(),
        codex_cli_path: codex_path.map(|p| p.display().to_string()),
        codex_logged_in: client_adapters::codex_logged_in(),
    }
}

fn check_headroom_learn_prereqs(
    agent: LearnAgent,
    platform_disabled_reason: Option<&str>,
    prereq: &HeadroomLearnPrereqStatus,
) -> Result<(), String> {
    if let Some(reason) = platform_disabled_reason {
        return Err(reason.to_string());
    }
    match agent {
        LearnAgent::Claude => {
            if !prereq.claude_cli_available {
                return Err(
                    "Install the Claude Code CLI (`claude`) to enable Headroom Learn.".into(),
                );
            }
        }
        LearnAgent::Codex => {
            if !prereq.codex_cli_available {
                return Err(
                    "Install the Codex CLI (`codex`) to enable Headroom Learn for Codex.".into(),
                );
            }
            if !prereq.codex_logged_in {
                return Err("Sign in to the Codex CLI with your ChatGPT account to enable Headroom Learn for Codex.".into());
            }
        }
    }
    Ok(())
}

/// Count entries in a `headroom memory export` JSON payload whose `created_at`
/// parses into the same UTC day as `now`. The export writes `created_at` as an
/// RFC3339-ish string without a timezone suffix (`2026-04-21T10:00:00`); we
/// treat those as UTC, matching the rest of the activity pipeline.
fn count_memories_created_today(
    json: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<usize, String> {
    let raw: Vec<serde_json::Value> =
        serde_json::from_str(json.trim()).map_err(|err| err.to_string())?;
    let today = now.date_naive();
    Ok(raw
        .into_iter()
        .filter_map(|v| {
            v.get("created_at")
                .and_then(|c| c.as_str())
                .and_then(parse_memory_created_at)
        })
        .filter(|dt| dt.date_naive() == today)
        .count())
}

fn parse_memory_created_at(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if raw.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    // The export omits timezone info (`2026-04-21T10:00:00`); treat as UTC.
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
        return Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ));
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ));
    }
    None
}

fn fetch_transformations_feed(limit: u32) -> Result<TransformationFeedResponse, String> {
    fetch_transformations_feed_from("http://127.0.0.1:6767", limit)
}

#[derive(serde::Deserialize)]
struct RawTransformationsFeedResponse {
    log_full_messages: bool,
    transformations: Vec<crate::models::TransformationFeedEvent>,
}

fn fetch_transformations_feed_from(
    base_url: &str,
    limit: u32,
) -> Result<TransformationFeedResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(2000))
        .build()
        .map_err(|err| err.to_string())?;
    let url = format!("{base_url}/transformations/feed?limit={limit}");
    let response = client.get(url).send().map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("proxy returned HTTP {}", response.status()));
    }
    let raw: RawTransformationsFeedResponse = response.json().map_err(|err| err.to_string())?;
    let settings = message_logging::load_settings();
    let transformations = raw
        .transformations
        .into_iter()
        .map(redact_transformation_feed_event)
        .collect();
    Ok(TransformationFeedResponse {
        log_full_messages: raw.log_full_messages && settings.full_message_logging,
        full_message_logging_expires_at: settings.full_message_logging_expires_at,
        message_log_retention_hours: settings.message_log_retention_hours,
        transformations,
        proxy_reachable: true,
    })
}

fn redact_transformation_feed_event(
    mut event: crate::models::TransformationFeedEvent,
) -> crate::models::TransformationFeedEvent {
    event.request_messages = event
        .request_messages
        .map(crate::message_logging::redact_value);
    event.compressed_messages = event
        .compressed_messages
        .map(crate::message_logging::redact_value);
    event
}

struct HeadroomLearnRunResult {
    success: bool,
    summary: String,
    error: Option<String>,
    output_tail: Vec<String>,
}

/// Detect `headroom.learn.analyzer` warnings that mean the LLM never produced
/// recommendations even though the CLI exited 0. Returns a user-facing message
/// joining all such warnings, or None if the run was clean.
fn extract_llm_failure_warnings(stderr: &str) -> Option<String> {
    const MARKER: &str = "LLM analysis failed:";
    let messages: Vec<String> = stderr
        .lines()
        .filter_map(|line| {
            line.split_once(MARKER)
                .map(|(_, rest)| format!("{} {}", MARKER, rest.trim()))
        })
        .collect();
    if messages.is_empty() {
        None
    } else {
        Some(messages.join("\n"))
    }
}

fn execute_headroom_learn_run(
    state: &AppState,
    agent: LearnAgent,
    project_path: Option<&str>,
) -> HeadroomLearnRunResult {
    // `run_id` keys the run-status + log file; `project_name` is the user-facing
    // label. Codex isn't project-organized, so it uses a stable "codex" id.
    let (run_id, project_name): (&str, String) = match agent {
        LearnAgent::Claude => {
            let path = project_path.unwrap_or("");
            let name = Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path)
                .to_string();
            (path, name)
        }
        LearnAgent::Codex => ("codex", "Codex sessions".to_string()),
    };
    let entrypoint = state.tool_manager.headroom_entrypoint();
    if !entrypoint.exists() {
        return HeadroomLearnRunResult {
            success: false,
            summary: format!("headroom learn failed for {project_name}."),
            error: Some(format!(
                "Headroom entrypoint not found at {}",
                entrypoint.display()
            )),
            output_tail: Vec::new(),
        };
    }
    // Pre-flight: the Claude scan passes --project to the CLI, where Click's
    // Path(readable=True) rejects a missing/unreadable dir with exit 2. That's a
    // user-environment condition (project moved/deleted, or macOS TCC blocking
    // ~/Documents et al.), not an app bug, so short-circuit here instead of
    // spawning and reporting the failure to Sentry. read_dir mirrors Click's
    // readability check and surfaces both the missing-path and TCC-denied cases.
    if let LearnAgent::Claude = agent {
        let path = project_path.unwrap_or_default();
        if path.is_empty() || std::fs::read_dir(path).is_err() {
            return HeadroomLearnRunResult {
                success: false,
                summary: format!("headroom learn failed for {project_name}."),
                error: Some(format!(
                    "Project path is not readable: {path}\n\
                     It may have been moved or deleted, or Headroom needs \
                     Files & Folders / Full Disk Access to read it."
                )),
                output_tail: Vec::new(),
            };
        }
    }

    let cli_path = match agent {
        LearnAgent::Claude => claude_cli::detect_claude_cli(),
        LearnAgent::Codex => client_adapters::detect_codex_cli(),
    };

    let mut command = Command::new(&entrypoint);
    command.arg("learn").arg("--apply");
    match agent {
        LearnAgent::Claude => {
            // Per-project Claude scan; writes CLAUDE.md / MEMORY.md for the
            // passed --project.
            command
                .arg("--project")
                .arg(project_path.unwrap_or_default())
                .arg("--agent")
                .arg("claude")
                .env("HEADROOM_LEARN_CLI", "claude");
        }
        LearnAgent::Codex => {
            // Codex scans all of ~/.codex/sessions (no --project) and writes
            // ~/.codex/AGENTS.md + instructions.md. Force --model codex-cli so
            // analysis runs through `codex exec` on the user's ChatGPT
            // subscription rather than auto-detecting an API key or the claude CLI.
            command
                .arg("--agent")
                .arg("codex")
                .arg("--model")
                .arg("codex-cli")
                .env("HEADROOM_LEARN_CLI", "codex");
        }
    }
    command
        // Run from an app-owned directory. For Claude the project is passed
        // explicitly via --project, so CWD is irrelevant; running elsewhere also
        // avoids getcwd() EPERM in spawned CLI shells when the project lives in a
        // TCC-protected location. The entrypoint's parent (inside Application
        // Support) is always accessible.
        .current_dir(
            entrypoint
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| std::path::PathBuf::from("/")),
        )
        .env("PYTHONNOUSERSITE", "1")
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .env("PIP_NO_INPUT", "1")
        // Force the selected CLI backend: the analyzer picks LiteLLM over
        // HEADROOM_LEARN_CLI / --model codex-cli when any of these keys is set
        // in the parent env.
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("GEMINI_API_KEY")
        // Don't pin ANTHROPIC_MODEL here: it's a LiteLLM identifier that the
        // analyzer never reads on the CLI path. Worse, it's inherited by the
        // spawned `claude -p` subprocess, where Claude Code's CLI does honor it —
        // and "claude-sonnet-4-6" is not a valid Claude Code model alias,
        // routing the call to a slow/hung path past 120s.
        .env_remove("ANTHROPIC_MODEL");
    if let Some(dir) = cli_path.as_ref().and_then(|p| p.parent()) {
        let existing = std::env::var("PATH").unwrap_or_default();
        let augmented = if existing.is_empty() {
            dir.display().to_string()
        } else {
            format!("{}:{}", dir.display(), existing)
        };
        command.env("PATH", augmented);
    }
    let output = command.output();

    let (summary, success, error, output_tail, stdout, stderr, status_copy) = match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let merged = if stderr.trim().is_empty() {
                stdout.clone()
            } else if stdout.trim().is_empty() {
                stderr.clone()
            } else {
                format!("{stdout}\n{stderr}")
            };
            let output_tail = crate::state::tail_lines(&merged, 32);
            if output.status.success() {
                if let Some(warnings) = extract_llm_failure_warnings(&stderr) {
                    (
                        format!(
                            "headroom learn could not produce recommendations for {project_name}."
                        ),
                        false,
                        Some(warnings),
                        output_tail,
                        stdout,
                        stderr,
                        output.status.to_string(),
                    )
                } else {
                    (
                        format!("headroom learn completed for {project_name}."),
                        true,
                        None,
                        output_tail,
                        stdout,
                        stderr,
                        output.status.to_string(),
                    )
                }
            } else {
                let fail_tail = if output_tail.is_empty() {
                    "No output captured.".to_string()
                } else {
                    output_tail.join("\n")
                };
                let exit_code_str = output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into());
                let signal_num: Option<i32> = {
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        output.status.signal()
                    }
                    #[cfg(not(unix))]
                    {
                        None
                    }
                };
                // First non-empty line of stderr (or stdout if stderr empty),
                // truncated, used both in the message and the fingerprint so
                // events group by failure mode instead of the capture-site stack.
                let signature_source = if !stderr.trim().is_empty() {
                    stderr.as_str()
                } else {
                    stdout.as_str()
                };
                let signature: String = signature_source
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or("no output")
                    .chars()
                    .take(160)
                    .collect();
                let stderr_head: String = stderr.chars().take(2000).collect();
                let stdout_head: String = stdout.chars().take(2000).collect();
                let cli_path_str = cli_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "not_found".into());
                let summary_msg =
                    format!("headroom learn failed (exit={exit_code_str}) {signature}");
                let fingerprint: [&str; 3] =
                    ["headroom_learn", exit_code_str.as_str(), signature.as_str()];
                // Defense in depth against a TOCTOU race: the path can become
                // unreadable between the pre-flight read_dir check and the CLI
                // run. Click reports that as exit 2 with "is not readable" — a
                // user-environment condition, not an app bug, so don't report it.
                let user_env_condition = signature.contains("is not readable");
                if !user_env_condition {
                    sentry::with_scope(
                        |scope| {
                            scope.set_tag("flow", "headroom_learn");
                            scope.set_tag(
                                "learn_agent",
                                match agent {
                                    LearnAgent::Claude => "claude",
                                    LearnAgent::Codex => "codex",
                                },
                            );
                            scope.set_tag("exit_code", &exit_code_str);
                            scope.set_extra("exit_status", output.status.to_string().into());
                            scope.set_extra(
                                "signal",
                                signal_num
                                    .map(|s| s.to_string().into())
                                    .unwrap_or(serde_json::Value::Null),
                            );
                            scope.set_extra("output_tail", fail_tail.clone().into());
                            scope.set_extra("stderr_head", stderr_head.into());
                            scope.set_extra("stdout_head", stdout_head.into());
                            scope.set_extra("cli_path", cli_path_str.into());
                            scope.set_extra("project_name", project_name.to_string().into());
                            scope.set_fingerprint(Some(fingerprint.as_slice()));
                        },
                        || {
                            sentry::capture_message(&summary_msg, sentry::Level::Error);
                        },
                    );
                }
                (
                    format!("headroom learn failed for {project_name}."),
                    false,
                    Some(format!(
                        "headroom learn exited with {}.\n{}",
                        output.status, fail_tail
                    )),
                    output_tail,
                    stdout,
                    stderr,
                    output.status.to_string(),
                )
            }
        }
        Err(err) => {
            sentry::capture_message(
                &format!("headroom learn spawn failed: {err}"),
                sentry::Level::Error,
            );
            (
                format!("headroom learn failed for {project_name}."),
                false,
                Some(format!("Could not start headroom learn: {err}")),
                Vec::new(),
                String::new(),
                String::new(),
                "spawn_error".to_string(),
            )
        }
    };

    let log_path = state.tool_manager.headroom_learn_log_path(run_id);
    let log_content = format!(
        "[{}] headroom learn --agent {} (target={})\nstatus: {}\n\n--- stdout ---\n{}\n\n--- stderr ---\n{}\n",
        Utc::now().to_rfc3339(),
        match agent {
            LearnAgent::Claude => "claude",
            LearnAgent::Codex => "codex",
        },
        run_id,
        status_copy,
        stdout,
        stderr
    );
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(log_path, log_content);

    HeadroomLearnRunResult {
        success,
        summary,
        error,
        output_tail,
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show =
        tauri::menu::MenuItem::with_id(app, "show", "Show Mac AI Switchboard", true, None::<&str>)?;
    let quit =
        tauri::menu::MenuItem::with_id(app, "quit", "Quit Mac AI Switchboard", true, None::<&str>)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let menu = tauri::menu::Menu::with_items(app, &[&show, &separator, &quit])?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayRuntimeVisual {
    Off,
    Booting,
    Running,
    Paused,
    Unhealthy,
    Disconnected,
}

struct TrayRuntimeIcons {
    off: tauri::image::Image<'static>,
    paused: tauri::image::Image<'static>,
    running_rgba: Vec<u8>,
    running_dims: (u32, u32),
    booting_frames: Vec<tauri::image::Image<'static>>,
}

fn debounced_tray_runtime_visual(
    raw_visual: TrayRuntimeVisual,
    last_non_booting: Option<TrayRuntimeVisual>,
    unhealthy_streak: &mut u8,
) -> TrayRuntimeVisual {
    const UNHEALTHY_DEBOUNCE_TICKS: u8 = 8;

    if raw_visual == TrayRuntimeVisual::Unhealthy {
        *unhealthy_streak = unhealthy_streak.saturating_add(1);
        if *unhealthy_streak < UNHEALTHY_DEBOUNCE_TICKS {
            if matches!(
                last_non_booting,
                Some(TrayRuntimeVisual::Running) | Some(TrayRuntimeVisual::Disconnected)
            ) {
                return last_non_booting.expect("checked Some above");
            }
        }
        return TrayRuntimeVisual::Unhealthy;
    }

    *unhealthy_streak = 0;
    raw_visual
}

fn spawn_tray_runtime_icon_updater(app: AppHandle) {
    let icons = match build_tray_runtime_icons() {
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
        let mut last_non_booting: Option<TrayRuntimeVisual> = None;
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
                        TrayRuntimeVisual::Running
                    } else {
                        TrayRuntimeVisual::Disconnected
                    }
                } else if runtime.starting {
                    TrayRuntimeVisual::Booting
                } else if runtime.paused {
                    TrayRuntimeVisual::Paused
                } else if runtime.installed && !runtime.proxy_reachable {
                    // Runtime should be up (installed, not paused, not booting)
                    // but the proxy isn't answering. Treat as unhealthy so the
                    // user has a visible signal the watchdog is working on it.
                    TrayRuntimeVisual::Unhealthy
                } else {
                    TrayRuntimeVisual::Off
                }
            };
            let visual =
                debounced_tray_runtime_visual(raw_visual, last_non_booting, &mut unhealthy_streak);

            if let Some(tray) = app.tray_by_id("headroom-tray") {
                let tooltip = match visual {
                    TrayRuntimeVisual::Booting => "Mac AI Switchboard — starting engine",
                    TrayRuntimeVisual::Running => "Mac AI Switchboard — engine active",
                    TrayRuntimeVisual::Paused => {
                        "Mac AI Switchboard — engine paused (Claude Code or Codex running normally)"
                    }
                    TrayRuntimeVisual::Unhealthy => {
                        "Mac AI Switchboard — engine unreachable, attempting restart"
                    }
                    TrayRuntimeVisual::Disconnected => {
                        "Mac AI Switchboard — Claude Code or Codex not connected"
                    }
                    TrayRuntimeVisual::Off => "Mac AI Switchboard — off",
                };

                let mut icon_changed = false;
                match visual {
                    TrayRuntimeVisual::Booting => {
                        let icon =
                            icons.booting_frames[frame_index % icons.booting_frames.len()].clone();
                        let _ = tray.set_icon(Some(icon));
                        icon_changed = true;
                        frame_index = (frame_index + 1) % icons.booting_frames.len();
                        last_non_booting = Some(TrayRuntimeVisual::Booting);
                    }
                    TrayRuntimeVisual::Running => {
                        let dollars = {
                            let savings_state: tauri::State<'_, TraySessionSavings> = app.state();
                            let v = *savings_state.0.lock();
                            let d = v.floor() as u32;
                            #[cfg(debug_assertions)]
                            let d = d.max(1);
                            d
                        };
                        let changed_visual = last_non_booting != Some(TrayRuntimeVisual::Running);
                        let changed_dollars = last_displayed_dollars != Some(dollars);
                        if changed_visual || changed_dollars {
                            let (bw, bh) = icons.running_dims;
                            let (new_rgba, new_w, new_h) =
                                build_running_with_savings(&icons.running_rgba, bw, bh, dollars);
                            let _ = tray.set_icon(Some(tauri::image::Image::new_owned(
                                new_rgba, new_w, new_h,
                            )));
                            icon_changed = true;
                            last_non_booting = Some(TrayRuntimeVisual::Running);
                            last_displayed_dollars = Some(dollars);
                        }
                    }
                    TrayRuntimeVisual::Off => {
                        if last_non_booting != Some(TrayRuntimeVisual::Off) {
                            let _ = tray.set_icon(Some(icons.off.clone()));
                            icon_changed = true;
                            last_non_booting = Some(TrayRuntimeVisual::Off);
                        }
                    }
                    TrayRuntimeVisual::Paused => {
                        if last_non_booting != Some(TrayRuntimeVisual::Paused) {
                            let _ = tray.set_icon(Some(icons.paused.clone()));
                            icon_changed = true;
                            last_non_booting = Some(TrayRuntimeVisual::Paused);
                            last_displayed_dollars = None;
                        }
                    }
                    TrayRuntimeVisual::Unhealthy => {
                        if last_non_booting != Some(TrayRuntimeVisual::Unhealthy) {
                            let _ = tray.set_icon(Some(icons.off.clone()));
                            icon_changed = true;
                            last_non_booting = Some(TrayRuntimeVisual::Unhealthy);
                            last_displayed_dollars = None;
                        }
                    }
                    TrayRuntimeVisual::Disconnected => {
                        if last_non_booting != Some(TrayRuntimeVisual::Disconnected) {
                            let _ = tray.set_icon(Some(icons.off.clone()));
                            icon_changed = true;
                            // Only notify when transitioning from a healthy running
                            // state — not on first boot or from other non-running states.
                            if last_non_booting == Some(TrayRuntimeVisual::Running) {
                                let _ = show_notification_impl(
                                    &app,
                                    "Headroom",
                                    "Claude Code or Codex is disconnected — open Headroom to re-enable.",
                                    Some("connectors".into()),
                                );
                            }
                            last_non_booting = Some(TrayRuntimeVisual::Disconnected);
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
                TrayRuntimeVisual::Booting => std::time::Duration::from_millis(260),
                TrayRuntimeVisual::Unhealthy => std::time::Duration::from_millis(1500),
                _ => std::time::Duration::from_secs(5),
            };
            std::thread::sleep(sleep);
        }
    });
}

/// Should the watchdog expect the Python proxy to be reachable right now?
///
/// All five inputs are required to be in their "ready" state for the proxy
/// to be supposed-up. Pulled out as a pure function so the truth table is
/// trivially testable — every clause is load-bearing and removing one
/// silently turns the watchdog into a thrash loop. Specifically `bypass`
/// being false matters: when the pricing gate has flipped on `proxy_bypass`
/// the Rust intercept is routing direct to api.anthropic.com, so a missing
/// Python is intentional, not a failure.
fn watchdog_should_be_up(
    installed: bool,
    paused: bool,
    starting: bool,
    upgrading: bool,
    bypass: bool,
) -> bool {
    installed && !paused && !starting && !upgrading && !bypass
}

/// Backoff schedule for the self-heal auto-resume loop after the watchdog has
/// given up and auto-paused. Keyed by the number of failed resume attempts so
/// far: 30s, 1m, 2m, then a 5m cap for all later attempts. Retries continue
/// indefinitely at the cap so a transient outage (laptop slept on battery,
/// transient network) self-heals whenever it clears, without hammering restart.
fn auto_resume_backoff(failed_attempts: u32) -> std::time::Duration {
    let secs = match failed_attempts {
        0 => 30,
        1 => 60,
        2 => 120,
        _ => 300,
    };
    std::time::Duration::from_secs(secs)
}

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
            let should_be_up = watchdog_should_be_up(
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
            let tolerant_outcome =
                probe_backend_readyz_outcome_with_timeout(std::time::Duration::from_secs(5));
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
            if readyz_failure_is_upstream_only(&tolerant_outcome) {
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
            if state::proxy_port_accepts_connection() && !state::intercept_port_accepts_connection()
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
                let backend_readyz_outcome = probe_backend_readyz_outcome();
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
                if readyz_failure_is_upstream_only(&backend_readyz_outcome) {
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
                    || readyz_failure_has_core_unhealthy(&backend_readyz_outcome))
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
                auto_pause_next_retry =
                    Some(std::time::Instant::now() + auto_resume_backoff(auto_pause_failed));
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

fn build_tray_runtime_icons() -> anyhow::Result<TrayRuntimeIcons> {
    let decoded = image::load_from_memory_with_format(
        include_bytes!("../icons/32x32.png"),
        image::ImageFormat::Png,
    )?
    .to_rgba8();
    let width = decoded.width();
    let height = decoded.height();
    let rgba = decoded.into_vec();

    let off_rgba = add_red_badge_dot(to_grayscale_strength(&rgba, 1.0), width, height);
    // Paused intentionally has no badge — distinguishes "user chose off" from
    // "broken and needs attention" at a glance.
    let paused_rgba = to_grayscale_strength(&rgba, 1.0);
    let booting_base = to_grayscale_strength(&rgba, 0.5);
    let booting_90 = rotate_90_cw(&booting_base, width, height);
    let booting_180 = rotate_90_cw(&booting_90, width, height);
    let booting_270 = rotate_90_cw(&booting_180, width, height);

    Ok(TrayRuntimeIcons {
        off: tauri::image::Image::new_owned(off_rgba, width, height),
        paused: tauri::image::Image::new_owned(paused_rgba, width, height),
        running_rgba: rgba,
        running_dims: (width, height),
        booting_frames: vec![
            tauri::image::Image::new_owned(booting_base, width, height),
            tauri::image::Image::new_owned(booting_90, width, height),
            tauri::image::Image::new_owned(booting_180, width, height),
            tauri::image::Image::new_owned(booting_270, width, height),
        ],
    })
}

fn to_grayscale_strength(rgba: &[u8], strength: f32) -> Vec<u8> {
    let s = strength.clamp(0.0, 1.0);
    let mut out = rgba.to_vec();
    for pixel in out.chunks_exact_mut(4) {
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        let gray = 0.299 * r + 0.587 * g + 0.114 * b;
        pixel[0] = (r * (1.0 - s) + gray * s).round() as u8;
        pixel[1] = (g * (1.0 - s) + gray * s).round() as u8;
        pixel[2] = (b * (1.0 - s) + gray * s).round() as u8;
    }
    out
}

fn rotate_90_cw(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = vec![0u8; rgba.len()];
    let w = width as usize;
    let h = height as usize;

    for y in 0..h {
        for x in 0..w {
            let src_idx = (y * w + x) * 4;
            let dst_x = h - 1 - y;
            let dst_y = x;
            let dst_idx = (dst_y * w + dst_x) * 4;
            out[dst_idx..dst_idx + 4].copy_from_slice(&rgba[src_idx..src_idx + 4]);
        }
    }
    out
}

fn add_red_badge_dot(mut rgba: Vec<u8>, width: u32, height: u32) -> Vec<u8> {
    let w = width as i32;
    let h = height as i32;
    let cx = w - 5;
    let cy = 5;
    let radius = 3i32;

    for y in 0..h {
        for x in 0..w {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= radius * radius {
                let idx = ((y as usize * width as usize) + x as usize) * 4;
                rgba[idx] = 217;
                rgba[idx + 1] = 76;
                rgba[idx + 2] = 76;
                rgba[idx + 3] = 255;
            }
        }
    }

    rgba
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

// Returns a (possibly wider) RGBA image with whole-dollar savings stacked
// vertically to the right of the base icon. Returns the base unchanged when
// dollars == 0.
fn build_running_with_savings(
    base: &[u8],
    base_w: u32,
    base_h: u32,
    dollars: u32,
) -> (Vec<u8>, u32, u32) {
    if dollars == 0 {
        return (base.to_vec(), base_w, base_h);
    }

    const CHAR_W: usize = 3;
    const CHAR_H: usize = 5;
    const H_MARGIN: usize = 2; // pixel gap between icon and text column

    let text = if dollars >= 1000 {
        format!("{}K", dollars / 1000)
    } else {
        dollars.to_string()
    };
    let chars: Vec<u8> = text.bytes().collect();
    let n = chars.len();

    // 2-digit values get a slightly larger gap since there's room.
    let row_gap_px: usize = if n <= 2 { 2 } else { 1 };

    // Largest dot size that fits: n*CHAR_H*dot + (n-1)*row_gap_px <= base_h
    let available = (base_h as usize).saturating_sub(n.saturating_sub(1) * row_gap_px);
    let max_dot = if n <= 2 { 3 } else { 2 };
    let dot = (available / (n * CHAR_H)).clamp(1, max_dot);

    let col_px_w = CHAR_W * dot + H_MARGIN;
    let new_w = base_w + col_px_w as u32;
    let h = base_h as usize;
    let bw = base_w as usize;
    let nw = new_w as usize;

    let mut out = vec![0u8; nw * h * 4];

    // Copy base icon into left portion.
    for y in 0..h {
        let src = y * bw * 4;
        let dst = y * nw * 4;
        out[dst..dst + bw * 4].copy_from_slice(&base[src..src + bw * 4]);
    }

    // Stack digits vertically in the right column, centred on the icon height.
    let total_h = n * CHAR_H * dot + n.saturating_sub(1) * row_gap_px;
    let y0 = h.saturating_sub(total_h) / 2;
    let x0 = bw + H_MARGIN;

    for (ci, &c) in chars.iter().enumerate() {
        let glyph = pixel_char(c);
        let cy = y0 + ci * (CHAR_H * dot + row_gap_px);
        for (row, cols) in glyph.iter().enumerate() {
            for (col, &on) in cols.iter().enumerate() {
                if on == 0 {
                    continue;
                }
                for dy in 0..dot {
                    for dx in 0..dot {
                        let px = x0 + col * dot + dx;
                        let py = cy + row * dot + dy;
                        if px < nw && py < h {
                            let i = (py * nw + px) * 4;
                            out[i] = 80;
                            out[i + 1] = 210;
                            out[i + 2] = 100;
                            out[i + 3] = 240;
                        }
                    }
                }
            }
        }
    }

    (out, new_w, base_h)
}

// Each glyph is [[col0, col1, col2]; 5 rows], top to bottom.
fn pixel_char(c: u8) -> [[u8; 3]; 5] {
    match c {
        b'0' => [[1, 1, 1], [1, 0, 1], [1, 0, 1], [1, 0, 1], [1, 1, 1]],
        b'1' => [[0, 1, 0], [1, 1, 0], [0, 1, 0], [0, 1, 0], [1, 1, 1]],
        b'2' => [[1, 1, 1], [0, 0, 1], [1, 1, 1], [1, 0, 0], [1, 1, 1]],
        b'3' => [[1, 1, 1], [0, 0, 1], [1, 1, 1], [0, 0, 1], [1, 1, 1]],
        b'4' => [[1, 0, 1], [1, 0, 1], [1, 1, 1], [0, 0, 1], [0, 0, 1]],
        b'5' => [[1, 1, 1], [1, 0, 0], [1, 1, 1], [0, 0, 1], [1, 1, 1]],
        b'6' => [[1, 1, 1], [1, 0, 0], [1, 1, 1], [1, 0, 1], [1, 1, 1]],
        b'7' => [[1, 1, 1], [0, 0, 1], [0, 0, 1], [0, 0, 1], [0, 0, 1]],
        b'8' => [[1, 1, 1], [1, 0, 1], [1, 1, 1], [1, 0, 1], [1, 1, 1]],
        b'9' => [[1, 1, 1], [1, 0, 1], [1, 1, 1], [0, 0, 1], [1, 1, 1]],
        b'K' => [[1, 0, 1], [1, 1, 0], [1, 0, 0], [1, 1, 0], [1, 0, 1]],
        _ => [[0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0]],
    }
}

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
        position_tray_window(&window, rect)?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PhysicalRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MonitorBounds {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

fn position_tray_window(window: &tauri::WebviewWindow, rect: Rect) -> tauri::Result<()> {
    let scale_factor = window.scale_factor()?;
    let tray_rect = physical_rect_from_rect(rect, scale_factor);
    let window_size = window
        .outer_size()
        .unwrap_or_else(|_| PhysicalSize::new(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT));
    let monitor_bounds = resolve_monitor_bounds(window, tray_rect);
    let target = compute_tray_window_position(tray_rect, window_size, monitor_bounds);

    window.set_position(Position::Physical(target))
}

fn physical_rect_from_rect(rect: Rect, scale_factor: f64) -> PhysicalRect {
    let (x, y) = match rect.position {
        Position::Physical(position) => (position.x, position.y),
        Position::Logical(position) => (
            (position.x * scale_factor).round() as i32,
            (position.y * scale_factor).round() as i32,
        ),
    };
    let (width, height) = match rect.size {
        tauri::Size::Physical(size) => (
            i32::try_from(size.width).unwrap_or(i32::MAX),
            i32::try_from(size.height).unwrap_or(i32::MAX),
        ),
        tauri::Size::Logical(size) => (
            (size.width * scale_factor).round() as i32,
            (size.height * scale_factor).round() as i32,
        ),
    };

    PhysicalRect {
        x,
        y,
        width,
        height,
    }
}

fn resolve_monitor_bounds(
    window: &tauri::WebviewWindow,
    tray_rect: PhysicalRect,
) -> Option<MonitorBounds> {
    let anchor_x = tray_rect.x + (tray_rect.width / 2);
    let anchor_y = tray_rect.y + (tray_rect.height / 2);

    if let Ok(monitors) = window.available_monitors() {
        if let Some(bounds) = monitors
            .into_iter()
            .map(monitor_bounds_from_monitor)
            .find(|bounds| point_within_monitor(*bounds, anchor_x, anchor_y))
        {
            return Some(bounds);
        }
    }

    window
        .current_monitor()
        .ok()
        .flatten()
        .map(monitor_bounds_from_monitor)
}

fn monitor_bounds_from_monitor(monitor: tauri::Monitor) -> MonitorBounds {
    MonitorBounds {
        x: monitor.position().x,
        y: monitor.position().y,
        width: i32::try_from(monitor.size().width).unwrap_or(i32::MAX),
        height: i32::try_from(monitor.size().height).unwrap_or(i32::MAX),
    }
}

fn point_within_monitor(bounds: MonitorBounds, x: i32, y: i32) -> bool {
    let max_x = bounds.x.saturating_add(bounds.width);
    let max_y = bounds.y.saturating_add(bounds.height);
    x >= bounds.x && x < max_x && y >= bounds.y && y < max_y
}

fn compute_tray_window_position(
    tray_rect: PhysicalRect,
    window_size: PhysicalSize<u32>,
    monitor_bounds: Option<MonitorBounds>,
) -> PhysicalPosition<i32> {
    let window_width = i32::try_from(window_size.width).unwrap_or(i32::MAX);
    let window_height = i32::try_from(window_size.height).unwrap_or(i32::MAX);
    let centered_x = tray_rect
        .x
        .saturating_add(tray_rect.width / 2)
        .saturating_sub(window_width / 2);
    let below_y = tray_rect
        .y
        .saturating_add(tray_rect.height)
        .saturating_add(TRAY_WINDOW_VERTICAL_GAP);

    if let Some(bounds) = monitor_bounds {
        let max_x = bounds
            .x
            .saturating_add(bounds.width.saturating_sub(window_width).max(0));
        let clamped_x = centered_x.clamp(bounds.x, max_x);

        let max_y = bounds
            .y
            .saturating_add(bounds.height.saturating_sub(window_height).max(0));
        let above_y = tray_rect
            .y
            .saturating_sub(window_height)
            .saturating_sub(TRAY_WINDOW_VERTICAL_GAP);
        let target_y =
            if below_y.saturating_add(window_height) <= bounds.y.saturating_add(bounds.height) {
                below_y
            } else {
                above_y.clamp(bounds.y, max_y)
            };

        return PhysicalPosition::new(clamped_x, target_y);
    }

    PhysicalPosition::new(centered_x, below_y)
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_live_learnings, app_quit_requested_properties, app_update_notification_body,
        auto_resume_backoff, beta_channel_enabled_from, build_release_updater_config,
        build_watchdog_give_up_report, check_headroom_learn_prereqs, classify_bootstrap_failure,
        classify_upgrade_error, clear_repo_intelligence_index, compute_tray_window_position,
        count_memories_created_today, cpu_rate_indicates_burn, debounced_tray_runtime_visual,
        delete_applied_pattern, empty_live_learnings_for_projects, extract_llm_failure_warnings,
        fetch_transformations_feed_from, install_pending_update, is_disk_full_signal,
        is_endpoint_protection_signal, is_network_download_signal, is_port_conflict_failure,
        is_prerelease_version, lifetime_token_milestone_kind, load_release_readiness_report_from,
        noop_app_update_progress_emitter, parse_live_learnings,
        parse_request_count_from_stats_body, parse_request_counts_by_agent,
        parse_updater_endpoint_list, pattern_matches_project, physical_rect_from_rect,
        read_applied_patterns_for_project, readyz_failed_checks_csv,
        readyz_failure_has_core_unhealthy, readyz_failure_is_upstream_only,
        repo_intelligence_doctor_issue, resolve_release_updater_config, select_updater_endpoints,
        store_checked_update, watchdog_should_be_up, zero_spend_affected_days, AppUpdateProgress,
        AppUpdateProgressEmitter, AvailableAppUpdate, BootstrapFailureKind, DailySavingsPoint,
        HeadroomLearnPrereqStatus, InstallPendingUpdateFuture, InstallableAppUpdate, LearnAgent,
        MonitorBounds, PhysicalRect, QuitSource, TrayRuntimeVisual,
    };
    use crate::models::{RepoFileIndexEntry, RepoIndexMetadata, RepoIntelligenceSummary};
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
            indexer_version: Some("path-graph-v3".to_string()),
            total_files: 1,
            indexed_files: 1,
            skipped_files: 0,
            estimated_full_scan_tokens: 10,
            role_counts: BTreeMap::new(),
            index_metadata: Some(RepoIndexMetadata {
                schema_version: 1,
                indexer_version: "path-graph-v3".to_string(),
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
        let missing_issue = repo_intelligence_doctor_issue(&missing, now).expect("missing issue");
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
        let moved_issue = repo_intelligence_doctor_issue(&moved, now).expect("moved issue");
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
            repo_intelligence_doctor_issue(&moved, now).is_none(),
            "existing indexed file should keep the saved index healthy"
        );

        let mut missing_metadata = moved.clone();
        missing_metadata.index_metadata = None;
        let missing_metadata_issue =
            repo_intelligence_doctor_issue(&missing_metadata, now).expect("metadata issue");
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
            repo_intelligence_doctor_issue(&parser_mismatch, now).expect("parser issue");
        assert_eq!(parser_mismatch_issue.id, "repo_intelligence_index_health");
        assert!(parser_mismatch_issue.body.contains("version_mismatch"));

        let mut indexer_mismatch = moved.clone();
        indexer_mismatch.indexer_version = Some("path-graph-v2".to_string());
        let indexer_mismatch_issue =
            repo_intelligence_doctor_issue(&indexer_mismatch, now).expect("indexer issue");
        assert_eq!(indexer_mismatch_issue.id, "repo_intelligence_index_health");
        assert!(indexer_mismatch_issue.body.contains("indexer health"));
        assert!(indexer_mismatch_issue.body.contains("version_mismatch"));
    }

    #[test]
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

        clear_repo_intelligence_index().expect("clear corrupt repo index");

        assert!(!path.exists());
        assert!(crate::repo_intelligence::load_latest_summary()
            .expect("cleared summary should read as none")
            .is_none());
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
    fn external_link_validator_accepts_documented_public_links() {
        assert_eq!(
            super::validate_external_link_url(
                " https://github.com/tarunag10/mac-ai-switchboard/issues ",
            )
            .expect("repo issues link"),
            "https://github.com/tarunag10/mac-ai-switchboard/issues"
        );
        assert_eq!(
            super::validate_external_link_url("https://developers.openai.com/codex/cli")
                .expect("codex docs link"),
            "https://developers.openai.com/codex/cli"
        );
        assert_eq!(
            super::validate_external_link_url("mailto:hello@example.com").expect("simple mailto"),
            "mailto:hello@example.com"
        );
    }

    #[test]
    fn external_link_validator_rejects_ssrf_and_injection_shapes() {
        for raw in [
            "file:///etc/passwd",
            "http://127.0.0.1:6767/stats",
            "http://localhost:6767/stats",
            "https://10.0.0.4/admin",
            "https://172.16.0.2/admin",
            "https://192.168.1.2/admin",
            "https://[::1]/admin",
            "https://user:pass@example.com/path",
            "https://github.com/tarunag10/mac-ai-switchboard/issues\nhttps://evil.example",
            "mailto:hello@example.com?subject=Injected",
        ] {
            assert!(
                super::validate_external_link_url(raw).is_err(),
                "{raw} should be rejected"
            );
        }
    }

    #[test]
    #[serial_test::serial]
    fn local_only_blocks_contact_request_before_url_or_email_validation() {
        let _local_only = LocalOnlyEnvGuard::enabled();
        let err = super::reject_contact_request_in_local_only()
            .expect_err("local-only blocks contact requests");

        assert_eq!(
            err,
            "Support/contact requests are disabled in local-only mode."
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
            "Mac AI Switchboard 0.3.0 is ready to install. Open Mac AI Switchboard to review the release and install it."
        );
        assert_eq!(
            app_update_notification_body("   "),
            "A Mac AI Switchboard update is ready to install. Open Mac AI Switchboard to review the release and install it."
        );
    }

    #[test]
    fn macos_notifications_do_not_wait_for_clicks() {
        let source = include_str!("lib.rs");
        let start = source
            .find("#[cfg(target_os = \"macos\")]\nfn show_notification_impl")
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

    fn make_command_failure(stderr: &str) -> crate::tool_manager::CommandFailure {
        crate::tool_manager::CommandFailure {
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

        let payload = load_release_readiness_report_from(&path).unwrap();

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

        let payload = load_release_readiness_report_from(&path).unwrap();

        assert_eq!(payload.report_path, path.to_string_lossy());
        assert!(payload.report.is_none());
    }

    #[test]
    fn release_evidence_command_rejects_unallowlisted_commands() {
        let err = crate::run_release_evidence_command("build-mac-dmg".to_string()).unwrap_err();

        assert!(
            err.contains(
                "enabled only for static-preflight, desktop-validation, local-dmg-build-install, local-installed-smoke, local-mode-relaunch-smoke, rollback-center-validation, doctor-repair-validation, uninstall-validation, repo-intelligence-validation, and release-report"
            ),
            "unexpected error: {err}"
        );
    }
}
