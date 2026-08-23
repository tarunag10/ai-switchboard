//! Isolated persistence for content-free daily analytics snapshots.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Duration, Local, Utc};

use crate::analytics_models::DailyUsageBriefingV1;
use crate::analytics_models::NormalizedAnalyticsEventV1;

pub(crate) const RETENTION_DAYS: i64 = 365;
pub(crate) const EVENT_RETENTION_DAYS: i64 = 30;
const MAX_PERSISTED_EVENTS: usize = 20_000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageAnalyticsClearPreviewV1 {
    /// Number of persisted daily briefing snapshots targeted by this action.
    ///
    /// The public read model calls these `briefings`; the field accepts the
    /// legacy `snapshotCount` spelling when decoding older fixtures, but emits
    /// only the stable frontend contract (`briefingCount`).
    #[serde(rename = "briefingCount", alias = "snapshotCount")]
    pub briefing_count: u64,
    /// Number of persisted normalized event projections targeted by this
    /// action. These are bounded, content-free facts rather than raw requests.
    pub event_count: u64,
    pub day_keys: Vec<String>,
    pub scope: String,
    pub detail: String,
}

pub(crate) fn save_daily_snapshot(root: &Path, briefing: &DailyUsageBriefingV1) -> Result<()> {
    let directory = daily_directory(root);
    fs::create_dir_all(&directory)
        .with_context(|| format!("creating analytics directory {}", directory.display()))?;
    let path = snapshot_path(root, &briefing.day_key)?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(briefing)?;
    fs::write(&temporary, bytes)
        .with_context(|| format!("writing analytics snapshot {}", temporary.display()))?;
    fs::rename(&temporary, &path)
        .with_context(|| format!("committing analytics snapshot {}", path.display()))?;
    prune_expired(root)?;
    Ok(())
}

pub(crate) fn save_events(root: &Path, events: &[NormalizedAnalyticsEventV1]) -> Result<()> {
    if events.is_empty() {
        prune_expired(root)?;
        return Ok(());
    }
    let directory = events_directory(root);
    fs::create_dir_all(&directory)
        .with_context(|| format!("creating analytics event directory {}", directory.display()))?;
    for event in events.iter().take(MAX_PERSISTED_EVENTS) {
        let path = event_path(root, event)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating analytics event day {}", parent.display()))?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(event)?)
            .with_context(|| format!("writing analytics event {}", temporary.display()))?;
        fs::rename(&temporary, &path)
            .with_context(|| format!("committing analytics event {}", path.display()))?;
    }
    prune_expired(root)?;
    Ok(())
}

pub(crate) fn list_events(root: &Path) -> Result<Vec<NormalizedAnalyticsEventV1>> {
    let directory = events_directory(root);
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut events = Vec::new();
    for day_entry in fs::read_dir(&directory)
        .with_context(|| format!("reading analytics event directory {}", directory.display()))?
    {
        let day_path = day_entry?.path();
        let Some(day_key) = day_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !day_path.is_dir() || !safe_day_key(day_key) {
            continue;
        }
        for entry in fs::read_dir(&day_path)
            .with_context(|| format!("reading analytics event day {}", day_path.display()))?
        {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            match fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<NormalizedAnalyticsEventV1>(&bytes).ok())
            {
                Some(event)
                    if event.schema_version == 1
                        && safe_event_id(&event.id)
                        && event.occurred_at.date_naive().to_string() == day_key =>
                {
                    events.push(event)
                }
                _ => log::warn!("ignoring unreadable analytics event {}", path.display()),
            }
        }
    }
    events.sort_by(|left, right| {
        right
            .occurred_at
            .cmp(&left.occurred_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    events.truncate(MAX_PERSISTED_EVENTS);
    Ok(events)
}

pub(crate) fn list_daily_snapshots(root: &Path) -> Result<Vec<DailyUsageBriefingV1>> {
    let directory = daily_directory(root);
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut snapshots = Vec::new();
    for entry in fs::read_dir(&directory)
        .with_context(|| format!("reading analytics directory {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        match fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<DailyUsageBriefingV1>(&bytes).ok())
        {
            Some(snapshot) if safe_day_key(&snapshot.day_key) => snapshots.push(snapshot),
            _ => log::warn!("ignoring unreadable analytics snapshot {}", path.display()),
        }
    }
    snapshots.sort_by(|left, right| right.day_key.cmp(&left.day_key));
    Ok(snapshots)
}

pub(crate) fn preview_clear(root: &Path) -> Result<UsageAnalyticsClearPreviewV1> {
    let snapshots = list_daily_snapshots(root)?;
    let events = list_events(root)?;
    Ok(UsageAnalyticsClearPreviewV1 {
        briefing_count: snapshots.len() as u64,
        event_count: events.len() as u64,
        day_keys: snapshots
            .into_iter()
            .map(|snapshot| snapshot.day_key)
            .collect(),
        scope: "daily_usage_briefing_snapshots_and_normalized_events".into(),
        detail: "Deletes content-free daily briefing snapshots and normalized analytics events only. Prompts, responses, credentials, and the savings attribution ledger are never included in this scope.".into(),
    })
}

pub(crate) fn clear(root: &Path) -> Result<UsageAnalyticsClearPreviewV1> {
    let preview = preview_clear(root)?;
    let directory = daily_directory(root);
    if directory.exists() {
        fs::remove_dir_all(&directory)
            .with_context(|| format!("clearing analytics directory {}", directory.display()))?;
    }
    let events = events_directory(root);
    if events.exists() {
        fs::remove_dir_all(&events)
            .with_context(|| format!("clearing analytics event directory {}", events.display()))?;
    }
    Ok(preview)
}

fn prune_expired(root: &Path) -> Result<()> {
    let daily_cutoff = (Local::now() - Duration::days(RETENTION_DAYS))
        .format("%Y-%m-%d")
        .to_string();
    for snapshot in list_daily_snapshots(root)? {
        if snapshot.day_key < daily_cutoff {
            let path = snapshot_path(root, &snapshot.day_key)?;
            let _ = fs::remove_file(path);
        }
    }
    let event_cutoff = (Utc::now() - Duration::days(EVENT_RETENTION_DAYS))
        .date_naive()
        .to_string();
    let directory = events_directory(root);
    if directory.exists() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            let Some(day_key) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if path.is_dir() && safe_day_key(day_key) && day_key < event_cutoff.as_str() {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
    Ok(())
}

fn daily_directory(root: &Path) -> PathBuf {
    root.join("analytics").join("daily-briefings")
}

fn events_directory(root: &Path) -> PathBuf {
    root.join("analytics").join("events")
}

fn event_path(root: &Path, event: &NormalizedAnalyticsEventV1) -> Result<PathBuf> {
    anyhow::ensure!(safe_event_id(&event.id), "invalid analytics event ID");
    let day_key = event.occurred_at.date_naive().to_string();
    Ok(events_directory(root)
        .join(day_key)
        .join(format!("{}.json", event.id)))
}

fn snapshot_path(root: &Path, day_key: &str) -> Result<PathBuf> {
    anyhow::ensure!(safe_day_key(day_key), "invalid analytics day key");
    Ok(daily_directory(root).join(format!("{day_key}.json")))
}

fn safe_day_key(day_key: &str) -> bool {
    day_key.len() == 10
        && day_key.as_bytes().get(4) == Some(&b'-')
        && day_key.as_bytes().get(7) == Some(&b'-')
        && day_key
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn safe_event_id(event_id: &str) -> bool {
    event_id.len() <= 128
        && !event_id.is_empty()
        && event_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics_models::{
        AnalyticsEvidenceConfidence, BriefingCompletenessV1, DailyUsageTotalsV1,
        EvidenceCoverageV1, NormalizedAnalyticsEventV1, TokenMetricV1, TokenXrayEventKindV1,
    };
    use chrono::Utc;

    fn fixture(day_key: &str) -> DailyUsageBriefingV1 {
        let unavailable = || TokenMetricV1::unavailable("fixture", "fixture");
        DailyUsageBriefingV1 {
            schema_version: 1,
            day_key: day_key.into(),
            timezone: "UTC".into(),
            generated_at: Utc::now(),
            completeness: BriefingCompletenessV1::InsufficientData,
            totals: DailyUsageTotalsV1 {
                requests: 0,
                active_agents: 0,
                input_tokens: unavailable(),
                output_tokens: unavailable(),
                saved_tokens: unavailable(),
                avoided_tokens: unavailable(),
                estimated_cost_usd: unavailable(),
                estimated_savings_usd: unavailable(),
            },
            agents: vec![],
            providers: vec![],
            attention_items: vec![],
            recommendations: vec![],
            evidence_coverage: EvidenceCoverageV1 {
                measured_sources: 0,
                estimated_sources: 0,
                inferred_sources: 0,
                unavailable_metrics: 8,
                notes: vec![],
            },
        }
    }

    #[test]
    fn snapshots_round_trip_and_clear_is_scoped() {
        let temp = tempfile::tempdir().unwrap();
        save_daily_snapshot(temp.path(), &fixture("2026-07-11")).unwrap();
        assert_eq!(list_daily_snapshots(temp.path()).unwrap().len(), 1);
        let preview = clear(temp.path()).unwrap();
        assert_eq!(preview.briefing_count, 1);
        assert_eq!(preview.event_count, 0);
        assert!(preview.detail.contains("normalized analytics events"));
        assert!(list_daily_snapshots(temp.path()).unwrap().is_empty());
    }

    #[test]
    fn clear_preview_serializes_the_frontend_contract_without_claiming_events() {
        let temp = tempfile::tempdir().unwrap();
        save_daily_snapshot(temp.path(), &fixture("2026-07-10")).unwrap();
        save_daily_snapshot(temp.path(), &fixture("2026-07-11")).unwrap();

        let preview = preview_clear(temp.path()).unwrap();
        let value = serde_json::to_value(preview).unwrap();
        assert_eq!(value["briefingCount"], 2);
        assert_eq!(value["eventCount"], 0);
        assert!(value.get("snapshotCount").is_none());
        assert_eq!(
            value["scope"],
            "daily_usage_briefing_snapshots_and_normalized_events"
        );
        assert!(value["detail"]
            .as_str()
            .unwrap()
            .contains("normalized analytics events"));
    }

    #[test]
    fn normalized_events_round_trip_and_clear_counts_them() {
        let temp = tempfile::tempdir().unwrap();
        let occurred_at = Utc::now();
        let event = NormalizedAnalyticsEventV1 {
            schema_version: 1,
            id: "usage-0123456789abcdef".into(),
            occurred_at,
            kind: TokenXrayEventKindV1::Usage,
            label: "Agent request".into(),
            confidence: AnalyticsEvidenceConfidence::Estimated,
            input_tokens: 12,
            output_tokens: 4,
            saved_tokens: 2,
            avoided_tokens: 0,
            request_count: 1,
            latency_ms: Some(30),
            outcome: Some("success".into()),
            source: "recent_usage".into(),
        };
        save_events(temp.path(), &[event.clone()]).unwrap();
        let events = list_events(temp.path()).unwrap();
        assert_eq!(events, vec![event]);
        let preview = preview_clear(temp.path()).unwrap();
        assert_eq!(preview.event_count, 1);
        clear(temp.path()).unwrap();
        assert!(list_events(temp.path()).unwrap().is_empty());
    }

    #[test]
    fn rejects_path_like_day_keys() {
        assert!(snapshot_path(Path::new("/tmp"), "../../secret").is_err());
    }
}
