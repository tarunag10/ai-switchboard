//! Isolated persistence for content-free daily analytics snapshots.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Duration, Local};

use crate::analytics_models::DailyUsageBriefingV1;

pub(crate) const RETENTION_DAYS: i64 = 365;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageAnalyticsClearPreviewV1 {
    pub snapshot_count: u64,
    pub day_keys: Vec<String>,
    pub scope: String,
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
    Ok(UsageAnalyticsClearPreviewV1 {
        snapshot_count: snapshots.len() as u64,
        day_keys: snapshots
            .into_iter()
            .map(|snapshot| snapshot.day_key)
            .collect(),
        scope: "daily_usage_briefing_snapshots_only".into(),
    })
}

pub(crate) fn clear(root: &Path) -> Result<UsageAnalyticsClearPreviewV1> {
    let preview = preview_clear(root)?;
    let directory = daily_directory(root);
    if directory.exists() {
        fs::remove_dir_all(&directory)
            .with_context(|| format!("clearing analytics directory {}", directory.display()))?;
    }
    Ok(preview)
}

fn prune_expired(root: &Path) -> Result<()> {
    let cutoff = (Local::now() - Duration::days(RETENTION_DAYS))
        .format("%Y-%m-%d")
        .to_string();
    for snapshot in list_daily_snapshots(root)? {
        if snapshot.day_key < cutoff {
            let path = snapshot_path(root, &snapshot.day_key)?;
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

fn daily_directory(root: &Path) -> PathBuf {
    root.join("analytics").join("daily-briefings")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics_models::{
        BriefingCompletenessV1, DailyUsageTotalsV1, EvidenceCoverageV1, TokenMetricV1,
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
        assert_eq!(preview.snapshot_count, 1);
        assert!(list_daily_snapshots(temp.path()).unwrap().is_empty());
    }

    #[test]
    fn rejects_path_like_day_keys() {
        assert!(snapshot_path(Path::new("/tmp"), "../../secret").is_err());
    }
}
