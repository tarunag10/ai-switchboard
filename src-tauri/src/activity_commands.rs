use std::collections::HashMap;

use chrono::Utc;
use tauri::{AppHandle, Manager, State};

use crate::models::{
    ActivityFeedResponse, AppliedSection, ClaudeAccountProfile, ClaudeCodeProject, ClaudeUsage,
    TransformationFeedEvent, TransformationFeedResponse,
};
use crate::state::AppState;
use crate::{learning_commands, message_logging, pricing};

#[tauri::command]
pub async fn get_headroom_logs(
    state: State<'_, AppState>,
    max_lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let limit = max_lines.unwrap_or(120).clamp(20, 500);
    state
        .tool_manager
        .read_headroom_log_tail(limit)
        .map_err(|err| err.to_string())
}

/// Read-only snapshot of the activity feed. Observation writes happen on the
/// background observer, so this IPC path stays read-only and quick.
#[tauri::command]
pub fn get_activity_feed(state: State<'_, AppState>) -> ActivityFeedResponse {
    ActivityFeedResponse {
        tiles: state.activity_feed_snapshot(),
        proxy_reachable: crate::state::headroom_proxy_reachable(),
    }
}

#[tauri::command]
pub async fn get_transformations_feed(limit: Option<u32>) -> TransformationFeedResponse {
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

/// Observation cadence for background activity milestones. A modest delay is
/// fine here; foreground Activity still polls separately, and the
/// memory-export path is intentionally kept away from tight loops.
const ACTIVITY_OBSERVER_INTERVAL: std::time::Duration = std::time::Duration::from_secs(20);
/// Matches the frontend's `ACTIVITY_FEED_WINDOW` in App.tsx so the observer
/// sees the same transformations the UI will display.
const ACTIVITY_OBSERVER_LIMIT: u32 = 150;

pub(crate) fn spawn_activity_observer(app: AppHandle) {
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
    let memory_path = crate::headroom_memory_db_path();
    let patterns_today = if memory_path.exists() {
        learning_commands::memory_export_cached(&state, &memory_path)
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
            let applied = learning_commands::read_applied_patterns_for_project(&p.project_path);
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

    // No point nudging the user to run Train if the claude CLI isn't installed.
    // They'd just hit an install prompt. The Optimize tab surfaces the install
    // UI in that case; let them fix prereqs first.
    if state.headroom_learn_prereq_status().claude_cli_available {
        let _ = state.observe_train_suggestions(&projects);
    }
}

fn flatten_applied_bullets(sections: &[AppliedSection]) -> Vec<String> {
    sections
        .iter()
        .flat_map(|sec| sec.bullets.iter().cloned())
        .collect()
}

/// Count entries in a `headroom memory export` JSON payload whose `created_at`
/// parses into the same UTC day as `now`. The export writes `created_at` as an
/// RFC3339-ish string without a timezone suffix (`2026-04-21T10:00:00`); we
/// treat those as UTC, matching the rest of the activity pipeline.
pub(crate) fn count_memories_created_today(
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
    transformations: Vec<TransformationFeedEvent>,
}

pub(crate) fn fetch_transformations_feed_from(
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

fn redact_transformation_feed_event(mut event: TransformationFeedEvent) -> TransformationFeedEvent {
    event.request_messages = event
        .request_messages
        .map(crate::message_logging::redact_value);
    event.compressed_messages = event
        .compressed_messages
        .map(crate::message_logging::redact_value);
    event
}

/// Authoritative "did the proxy receive a request" signal for the connector
/// verification UI. Reads `/stats` on the live Rust front proxy and returns
/// `requests.total`. The earlier verification path scanned the python proxy
/// log for /v1/messages lines, but Claude Code traffic flows through the
/// Rust proxy on 6767 - the python log only ever sees background/internal
/// activity, so the regex match never fired even when the user's calls were
/// being optimized normally.
///
/// `None` means the proxy is unreachable or `/stats` failed; the frontend
/// must distinguish that from `Some(0)` ("up but no traffic yet"), otherwise
/// a transient unreachable -> reachable transition would look like a counter
/// jump from 0 -> N and falsely flip the badge to healthy.
#[tauri::command]
pub async fn get_headroom_request_count() -> Option<u64> {
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
pub async fn get_headroom_request_counts_by_agent() -> Option<HashMap<String, u64>> {
    parse_request_counts_by_agent(&fetch_proxy_stats_body()?)
}

pub(crate) fn parse_request_counts_by_agent(body: &str) -> Option<HashMap<String, u64>> {
    let root = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let mut counts = HashMap::new();
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
pub async fn get_rtk_activity(
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
pub async fn get_tool_logs(
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
pub async fn get_claude_code_projects(
    state: State<'_, AppState>,
) -> Result<Vec<ClaudeCodeProject>, String> {
    state
        .list_claude_code_projects()
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_claude_usage(state: State<'_, AppState>) -> Result<ClaudeUsage, String> {
    pricing::fetch_claude_usage(&state)
}

#[tauri::command]
pub fn get_claude_profile(state: State<'_, AppState>) -> ClaudeAccountProfile {
    pricing::detect_claude_profile(&state)
}
