use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use tauri::{AppHandle, Manager, State};

use crate::analytics;
use crate::models::{
    DailySavingsPoint, DashboardState, SavingsAttributionCounter, SavingsAttributionEvent,
};
use crate::pricing;
use crate::state::AppState;

static ZERO_SPEND_ALERT_FIRED: AtomicBool = AtomicBool::new(false);

// Spend fields (actual_cost_usd, total_tokens_sent) were added to SavingsRecord in
// schema v6, shipped in 0.2.40 on 2026-04-13. Records written before that date
// deserialize those fields as 0 via #[serde(default)], producing false positives.
const SPEND_SCHEMA_CUTOFF_DATE: &str = "2026-04-13";

pub(crate) fn zero_spend_affected_days(daily_savings: &[DailySavingsPoint]) -> Vec<&str> {
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
pub async fn get_dashboard_state(app: AppHandle) -> Result<DashboardState, String> {
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
pub async fn get_savings_attribution_events(
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
pub async fn get_savings_attribution_counters(
    app: AppHandle,
) -> Result<Vec<SavingsAttributionCounter>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        state.savings_attribution_counters()
    })
    .await
    .map_err(|err| err.to_string())
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasuredSavingsAttributionRequest {
    source: crate::models::SavingsAttributionSource,
    label: String,
    baseline_tokens: u64,
    optimized_tokens: u64,
    #[serde(default = "default_request_delta")]
    request_delta: usize,
    #[serde(default)]
    detail: String,
}

fn default_request_delta() -> usize {
    1
}

#[tauri::command]
pub async fn record_measured_savings_attribution(
    app: AppHandle,
    request: MeasuredSavingsAttributionRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        state
            .record_measured_addon_attribution(
                request.source,
                &request.label,
                request.baseline_tokens,
                request.optimized_tokens,
                request.request_delta,
                request.detail,
            )
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

pub(crate) fn lifetime_token_milestone_kind(milestone_tokens_saved: u64) -> &'static str {
    match milestone_tokens_saved {
        1_000_000 => "first_1m",
        5_000_000 => "first_5m",
        10_000_000 => "first_10m",
        _ => "repeating_10m",
    }
}
