use tauri::{AppHandle, Manager, State};

use crate::analytics_models::{DailyUsageBriefingV1, TokenXrayLiveUpdateV1, TokenXraySnapshotV1};
use crate::analytics_store::UsageAnalyticsClearPreviewV1;
use crate::daily_briefing::{self, DailyUsageBriefingExportV1};
use crate::models::{DashboardState, SavingsAttributionEvent};
use crate::optimization::cache_metrics::CacheTokenMetricsEvidence;
use crate::state::AppState;
use crate::token_xray;

/// Returns a content-free current-process Token X-Ray snapshot. Missing provider
/// telemetry is represented as `unavailable`, never as a fabricated zero.
#[tauri::command]
pub async fn get_token_xray_snapshot(app: AppHandle) -> Result<TokenXraySnapshotV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        xray_snapshot(
            &state.dashboard(),
            state.savings_attribution_events(),
            crate::optimization::telemetry_store::prompt_cache_totals_evidence_result()
                .ok()
                .flatten(),
        )
    })
    .await
    .map_err(|error| error.to_string())
}

/// Returns a compact content-free update only after the caller's revision is
/// stale and the material projection changed. This is a local polling state,
/// not provider telemetry, so it never includes prompts or response bodies.
#[tauri::command]
pub async fn get_token_xray_live_update(
    app: AppHandle,
    since_revision: Option<u64>,
) -> Result<Option<TokenXrayLiveUpdateV1>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        Ok::<_, String>(state.token_xray_live_update(since_revision))
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Command-side evidence boundary: a cache storage failure remains absent all
/// the way through IPC instead of becoming a value of zero.
fn xray_snapshot(
    dashboard: &DashboardState,
    attribution: Vec<SavingsAttributionEvent>,
    cache_metrics: Option<CacheTokenMetricsEvidence>,
) -> TokenXraySnapshotV1 {
    token_xray::build_snapshot_with_cache_metrics(dashboard, attribution, cache_metrics)
}

/// Builds the local current-day briefing from the existing in-memory usage and
/// durable attribution ledger. It does not make provider or analytics requests.
#[tauri::command]
pub async fn get_daily_usage_briefing(app: AppHandle) -> Result<DailyUsageBriefingV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        state.daily_usage_briefing()
    })
    .await
    .map_err(|error| error.to_string())
}

/// Produces an allowlisted JSON-ready briefing and a copyable Markdown version;
/// callers choose where to save it so this command never writes user files.
#[tauri::command]
pub async fn export_daily_usage_briefing(
    app: AppHandle,
) -> Result<DailyUsageBriefingExportV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        daily_briefing::export(state.daily_usage_briefing())
    })
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_daily_usage_briefings(
    app: AppHandle,
) -> Result<Vec<DailyUsageBriefingV1>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        state
            .list_daily_usage_briefings()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Preview only: this targets daily briefing snapshots and never touches the
/// existing savings attribution ledger or provider history.
#[tauri::command]
pub async fn preview_clear_usage_analytics(
    app: AppHandle,
) -> Result<UsageAnalyticsClearPreviewV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        state
            .preview_clear_usage_analytics()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn clear_usage_analytics(app: AppHandle) -> Result<UsageAnalyticsClearPreviewV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, AppState> = app.state();
        state
            .clear_usage_analytics()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics_models::AnalyticsEvidenceConfidence;
    use crate::models::LaunchExperience;

    fn empty_dashboard() -> DashboardState {
        DashboardState {
            app_version: String::new(),
            launch_experience: LaunchExperience::Dashboard,
            bootstrap_complete: false,
            python_runtime_installed: false,
            lifetime_requests: 0,
            lifetime_estimated_savings_usd: 0.0,
            lifetime_estimated_tokens_saved: 0,
            session_requests: 0,
            session_estimated_savings_usd: 0.0,
            session_estimated_tokens_saved: 0,
            session_savings_pct: 0.0,
            output_reduction: None,
            daily_savings: vec![],
            hourly_savings: vec![],
            savings_history_loaded: false,
            tools: vec![],
            clients: vec![],
            recent_usage: vec![],
            insights: vec![],
            required_terms_version: 0,
            accepted_terms_version: 0,
            terms_url: String::new(),
        }
    }

    #[test]
    fn command_mapper_keeps_failed_cache_telemetry_unavailable() {
        let snapshot = xray_snapshot(&empty_dashboard(), vec![], None);
        assert!(snapshot.metrics.cache_read_tokens.value.is_none());
        assert!(matches!(
            snapshot.metrics.cache_read_tokens.confidence,
            AnalyticsEvidenceConfidence::Unavailable
        ));
    }
}
