use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{Datelike, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::is_headroom_proxy_reachable;
use crate::activity_facts::WeeklyTotals;
use crate::insights::generate_daily_insights;

use crate::models::{
    ClientStatus, DailyInsight, DailySavingsPoint, HourlySavingsPoint,
    SavingsAttributionConfidence, SavingsAttributionCounter, SavingsAttributionEvent,
    SavingsAttributionScope, SavingsAttributionSource, UsageEvent,
};
use crate::storage::{config_file, telemetry_file};
use crate::tool_manager::RtkGainSummary;

const CAVEMAN_TEMPLATE_BASELINE_TOKENS: u64 = 480;
const CAVEMAN_TEMPLATE_OPTIMIZED_TOKENS: u64 = 180;
const PONYTAIL_TEMPLATE_BASELINE_TOKENS: u64 = 1_400;
const PONYTAIL_TEMPLATE_OPTIMIZED_TOKENS: u64 = 520;
const MARKITDOWN_TEMPLATE_BASELINE_TOKENS: u64 = 3_200;
const MARKITDOWN_TEMPLATE_OPTIMIZED_TOKENS: u64 = 900;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SavingsTotalsSnapshot {
    pub(super) session_requests: usize,
    pub(super) session_estimated_savings_usd: f64,
    pub(super) session_estimated_tokens_saved: u64,
    pub(super) session_savings_pct: f64,
    pub(super) lifetime_requests: usize,
    pub(super) lifetime_estimated_savings_usd: f64,
    pub(super) lifetime_estimated_tokens_saved: u64,
}

const FIRST_LIFETIME_TOKEN_MILESTONES: [u64; 3] = [100_000, 1_000_000, 5_000_000];
const REPEATING_LIFETIME_TOKEN_MILESTONE_STEP: u64 = 10_000_000;

const FIRST_LIFETIME_USD_MILESTONES: [u64; 3] = [10, 50, 100];
const REPEATING_LIFETIME_USD_MILESTONE_STEP: u64 = 100;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub(super) struct SavingsRecord {
    /// Schema version for forward-compatibility and migration detection.
    /// v0 = legacy (USD derived from tokens/10000)
    /// v2 = day-scoped deltas
    /// v3 = session-scoped deltas matching Headroom /stats
    /// v4 = session-scoped deltas plus actual usage totals
    /// v5 = v4 plus hour-scoped bucket keys
    /// v6 = v5 plus spend metrics sourced from /stats actual-input fields only
    /// v7 = v6 plus spend backfills distributed across session history
    pub(super) schema_version: u8,
    pub(super) id: String,
    pub(super) observed_at: chrono::DateTime<Utc>,
    pub(super) day_key: String,
    pub(super) hour_key: String,
    pub(super) session_requests: usize,
    pub(super) session_estimated_savings_usd: f64,
    pub(super) session_estimated_tokens_saved: u64,
    pub(super) session_actual_cost_usd: f64,
    pub(super) session_total_tokens_sent: u64,
    pub(super) delta_requests: usize,
    pub(super) delta_estimated_savings_usd: f64,
    pub(super) delta_estimated_tokens_saved: u64,
    pub(super) delta_actual_cost_usd: f64,
    pub(super) delta_total_tokens_sent: u64,
    pub(super) source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SavingsObservation {
    pub(super) observed_at: chrono::DateTime<Utc>,
    pub(super) last_activity_at: Option<chrono::DateTime<Utc>>,
    pub(super) session_requests: usize,
    pub(super) session_estimated_savings_usd: f64,
    pub(super) session_estimated_tokens_saved: u64,
    pub(super) session_actual_cost_usd: f64,
    pub(super) session_total_tokens_sent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RtkSavingsObservation {
    pub(super) observed_at: chrono::DateTime<Utc>,
    pub(super) total_commands: u64,
    pub(super) total_input: u64,
    pub(super) total_output: u64,
    pub(super) total_saved: u64,
    pub(super) total_time_ms: u64,
}

pub(super) fn build_repo_intelligence_attribution_event(
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

pub(super) fn build_addon_attribution_event(
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
pub(super) struct DailySavingsBucket {
    pub(super) estimated_savings_usd: f64,
    pub(super) estimated_tokens_saved: u64,
    pub(super) actual_cost_usd: f64,
    pub(super) total_tokens_sent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedSavingsState {
    pub(super) schema_version: u8,
    pub(super) session_requests: usize,
    pub(super) session_estimated_savings_usd: f64,
    pub(super) session_estimated_tokens_saved: u64,
    pub(super) session_savings_pct: f64,
    pub(super) lifetime_requests: usize,
    pub(super) lifetime_estimated_savings_usd: f64,
    pub(super) lifetime_estimated_tokens_saved: u64,
    pub(super) last_observation: Option<SavingsObservation>,
    pub(super) last_rtk_observation: Option<RtkSavingsObservation>,
    pub(super) display_session_baseline: Option<SavingsObservation>,
    pub(super) session_savings_history: Vec<HeadroomSavingsHistoryPoint>,
    pub(super) session_hourly_buckets: BTreeMap<String, DailySavingsBucket>,
    pub(super) daily_savings: BTreeMap<String, DailySavingsBucket>,
    pub(super) hourly_savings: BTreeMap<String, DailySavingsBucket>,
}

pub(super) fn maybe_append_measured_headroom_attribution(
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

pub(super) struct SavingsTracker {
    pub(super) records_path: std::path::PathBuf,
    pub(super) attribution_events_path: std::path::PathBuf,
    pub(super) state_path: std::path::PathBuf,
    pub(super) session_requests: usize,
    pub(super) session_estimated_savings_usd: f64,
    pub(super) session_estimated_tokens_saved: u64,
    pub(super) session_savings_pct: f64,
    pub(super) lifetime_requests: usize,
    pub(super) lifetime_estimated_savings_usd: f64,
    pub(super) lifetime_estimated_tokens_saved: u64,
    pub(super) last_observation: Option<SavingsObservation>,
    pub(super) last_rtk_observation: Option<RtkSavingsObservation>,
    pub(super) display_session_baseline: Option<SavingsObservation>,
    pub(super) session_savings_history: Vec<HeadroomSavingsHistoryPoint>,
    pub(super) session_hourly_buckets: BTreeMap<String, DailySavingsBucket>,
    pub(super) daily_savings: BTreeMap<String, DailySavingsBucket>,
    pub(super) hourly_savings: BTreeMap<String, DailySavingsBucket>,
    pub(super) pending_lifetime_token_milestones: Vec<u64>,
    pub(super) pending_lifetime_usd_milestones: Vec<u64>,
    // Write throttle — only flush to disk at most once per minute
    pub(super) last_written_at: Option<std::time::Instant>,
}

impl SavingsTracker {
    pub(super) fn load_or_create(base_dir: &Path) -> Result<Self> {
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

    pub(super) fn snapshot(&self) -> SavingsTotalsSnapshot {
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

    pub(super) fn daily_savings(&self) -> Vec<DailySavingsPoint> {
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

    pub(super) fn hourly_savings(&self) -> Vec<HourlySavingsPoint> {
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

    pub(super) fn attribution_events(&self) -> Vec<SavingsAttributionEvent> {
        let Ok(text) = std::fs::read_to_string(&self.attribution_events_path) else {
            return Vec::new();
        };

        text.lines()
            .filter_map(|line| serde_json::from_str::<SavingsAttributionEvent>(line).ok())
            .collect()
    }

    pub(super) fn observe_rtk_gain_summary(&mut self, stats: &RtkGainSummary) {
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
    pub(super) fn ingest_native_rollups(
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

    pub(super) fn take_pending_lifetime_token_milestones(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_lifetime_token_milestones)
    }

    #[cfg(test)]
    pub(super) fn take_pending_lifetime_usd_milestones(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_lifetime_usd_milestones)
    }

    pub(super) fn observe(
        &mut self,
        stats: &HeadroomDashboardStats,
    ) -> Option<SavingsTotalsSnapshot> {
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

    pub(super) fn ingest_hourly_buckets(&mut self, buckets: &[(String, DailySavingsBucket)]) {
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

    pub(super) fn append_attribution_event(&self, event: &SavingsAttributionEvent) -> Result<()> {
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

    pub(super) fn persisted_state(&self) -> PersistedSavingsState {
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

    pub(super) fn persist_state(&mut self) -> Result<()> {
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
pub(super) fn aggregate_savings_attribution_counters(
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

pub(super) fn most_recent_monday(d: chrono::NaiveDate) -> chrono::NaiveDate {
    let days_past = d.weekday().num_days_from_monday() as u64;
    d.checked_sub_days(chrono::Days::new(days_past))
        .unwrap_or(d)
}

pub(super) fn aggregate_weekly_totals(
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

pub(super) fn lifetime_usd_milestones_crossed(previous_usd: f64, current_usd: f64) -> Vec<u64> {
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

pub(super) fn lifetime_token_milestones_crossed(
    previous_total: u64,
    current_total: u64,
) -> Vec<u64> {
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

pub(super) fn load_persisted_savings_state(path: &Path) -> Result<Option<PersistedSavingsState>> {
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

pub(super) fn build_insights(
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
pub(super) struct HeadroomSavingsHistoryPoint {
    pub(super) timestamp: chrono::DateTime<Utc>,
    pub(super) total_tokens_saved: u64,
}

#[derive(Debug, Default, Clone)]
pub(super) struct HeadroomDashboardStats {
    pub(super) session_requests: Option<usize>,
    pub(super) session_estimated_savings_usd: Option<f64>,
    pub(super) session_estimated_tokens_saved: Option<u64>,
    pub(super) session_savings_pct: Option<f64>,
    pub(super) session_actual_cost_usd: Option<f64>,
    pub(super) session_total_tokens_sent: Option<u64>,
    pub(super) savings_history: Vec<HeadroomSavingsHistoryPoint>,
    pub(super) output_reduction: Option<OutputReduction>,
}

/// Counterfactual output-token reduction from the proxy's output shaper,
/// parsed from `/stats` (`savings.by_layer.output_shaping`). `method` is
/// "estimated" (synthetic control vs a learned baseline) or "measured" (A/B
/// holdout); the percentage always carries a 95% confidence band. Only
/// populated when the proxy reports `available: true` (i.e. a baseline exists).
#[derive(Debug, Clone)]
pub(super) struct OutputReduction {
    pub(super) method: String,
    pub(super) reduction_percent: f64,
    pub(super) ci_low_percent: f64,
    pub(super) ci_high_percent: f64,
    pub(super) requests: u64,
}

/// One provider's slice of a rollup bucket's delta, parsed from the upstream
/// `by_provider` map (`anthropic` / `openai` / `unknown`). Field names mirror the
/// bucket total; `hourly_savings` maps these to the display `ProviderSavingsPoint`.
#[derive(Debug, Default, Clone)]
pub(super) struct ProviderRollupDelta {
    pub(super) provider: String,
    pub(super) tokens_saved: u64,
    pub(super) compression_savings_usd_delta: f64,
    pub(super) total_input_tokens_delta: u64,
    pub(super) total_input_cost_usd_delta: f64,
}

#[derive(Debug, Default, Clone)]
pub(super) struct HeadroomSavingsRollupPoint {
    pub(super) timestamp: chrono::DateTime<Utc>,
    pub(super) tokens_saved: u64,
    pub(super) compression_savings_usd_delta: f64,
    pub(super) total_input_tokens_delta: u64,
    pub(super) total_input_cost_usd_delta: f64,
    pub(super) by_provider: Vec<ProviderRollupDelta>,
}

#[derive(Debug, Default, Clone)]
pub(super) struct HeadroomSavingsHistoryResponse {
    pub(super) lifetime_estimated_savings_usd: Option<f64>,
    pub(super) lifetime_estimated_tokens_saved: Option<u64>,
    pub(super) hourly: Vec<HeadroomSavingsRollupPoint>,
    pub(super) daily: Vec<HeadroomSavingsRollupPoint>,
}

impl HeadroomSavingsHistoryResponse {
    pub(super) fn daily_savings(&self) -> Vec<DailySavingsPoint> {
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

    pub(super) fn hourly_savings(&self) -> Vec<HourlySavingsPoint> {
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

pub(super) fn fetch_headroom_dashboard_stats() -> Option<HeadroomDashboardStats> {
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

pub(super) fn fetch_headroom_savings_history() -> Option<HeadroomSavingsHistoryResponse> {
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
pub(super) fn parse_output_reduction(root: &Value) -> Option<OutputReduction> {
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

pub(super) fn parse_headroom_stats_from_json(body: &str) -> Option<HeadroomDashboardStats> {
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
pub(super) fn upstream_history_trimmed(root: &Value) -> bool {
    let stored = value_at_path_u64(root, &["history_summary", "stored_points"]);
    let cap = value_at_path_u64(root, &["retention", "max_history_points"]);
    matches!((stored, cap), (Some(stored), Some(cap)) if cap > 0 && stored >= cap)
}

/// Remove the oldest bucket (smallest timestamp) from a rollup series.
pub(super) fn drop_oldest_rollup_bucket(series: &mut Vec<HeadroomSavingsRollupPoint>) {
    if let Some((idx, _)) = series
        .iter()
        .enumerate()
        .min_by_key(|(_, point)| point.timestamp)
    {
        series.remove(idx);
    }
}

pub(super) fn parse_headroom_stats_history_from_json(
    body: &str,
) -> Option<HeadroomSavingsHistoryResponse> {
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

pub(super) fn value_at_path_u64(root: &Value, path: &[&str]) -> Option<u64> {
    let value = value_at_path(root, path)?;
    parse_u64_value(value)
}

pub(super) fn value_at_path_f64(root: &Value, path: &[&str]) -> Option<f64> {
    let value = value_at_path(root, path)?;
    parse_f64_value(value)
}

pub(super) fn value_at_path<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
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

pub(super) fn parse_savings_history(value: &Value) -> Option<Vec<HeadroomSavingsHistoryPoint>> {
    let Value::Array(items) = value else {
        return None;
    };
    let points = items
        .iter()
        .filter_map(parse_savings_history_point)
        .collect::<Vec<_>>();
    Some(points)
}

pub(super) fn parse_savings_rollup_series(
    value: &Value,
) -> Option<Vec<HeadroomSavingsRollupPoint>> {
    let Value::Array(items) = value else {
        return None;
    };
    let points = items
        .iter()
        .filter_map(parse_savings_rollup_point)
        .collect::<Vec<_>>();
    Some(points)
}

pub(super) fn parse_savings_history_point(value: &Value) -> Option<HeadroomSavingsHistoryPoint> {
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

pub(super) fn parse_savings_rollup_point(value: &Value) -> Option<HeadroomSavingsRollupPoint> {
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
pub(super) fn parse_rollup_by_provider(value: Option<&Value>) -> Vec<ProviderRollupDelta> {
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

pub(super) fn parse_history_timestamp(text: &str) -> Option<chrono::DateTime<Utc>> {
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

pub(super) fn local_day_key(timestamp: chrono::DateTime<Local>) -> String {
    timestamp.format("%Y-%m-%d").to_string()
}

// Boundary between local tracker (pre-cutoff, authoritative) and /stats-history
// (cutoff and later, authoritative). Release builds pin to the date the schema
// stabilized; debug builds track "today" so dev sessions never fall behind the
// history source while iterating.
pub(super) fn savings_history_cutoff_date() -> String {
    if cfg!(debug_assertions) {
        local_day_key(Local::now())
    } else {
        "2026-06-02".to_string()
    }
}

pub(super) fn local_hour_key(timestamp: chrono::DateTime<Local>) -> String {
    timestamp.format("%Y-%m-%dT%H:00").to_string()
}

pub(super) fn day_key_from_hour_key(hour_key: &str) -> String {
    hour_key.split('T').next().unwrap_or(hour_key).to_string()
}

pub(super) fn should_rollover_display_session(
    last_activity_at: chrono::DateTime<Utc>,
    now: chrono::DateTime<Utc>,
) -> bool {
    let last_local = last_activity_at.with_timezone(&Local);
    let now_local = now.with_timezone(&Local);
    now_local.date_naive() > last_local.date_naive()
        && now.signed_duration_since(last_activity_at) >= chrono::Duration::hours(1)
}

pub(super) fn derive_session_buckets_with_key<F>(
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

pub(super) fn merge_session_savings_history(
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

pub(super) fn derive_session_hourly_buckets(
    stats: &HeadroomDashboardStats,
    history: &[HeadroomSavingsHistoryPoint],
) -> Vec<(String, DailySavingsBucket)> {
    derive_session_buckets_with_key(stats, history, local_hour_key)
}

pub(super) fn diff_hourly_buckets(
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

pub(super) fn build_hourly_backfill_records(
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

pub(super) fn build_hourly_delta_records(
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

pub(super) fn find_u64_key_recursive(value: &Value, keys: &[&str]) -> Option<u64> {
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

pub(super) fn find_f64_key_recursive(value: &Value, keys: &[&str]) -> Option<f64> {
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

pub(super) fn parse_u64_value(value: &Value) -> Option<u64> {
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

pub(super) fn parse_f64_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(num) => num.as_f64(),
        Value::String(text) => parse_f64_from_text(text),
        _ => None,
    }
}

pub(super) fn parse_u64_from_text(text: &str) -> Option<u64> {
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

pub(super) fn parse_f64_from_text(text: &str) -> Option<f64> {
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

/// Merge daily savings from tracker (pre-cutoff) and native headroom history (post-cutoff).
/// For days before `cutoff_date` (exclusive), the tracker is preferred.
/// For days on/after `cutoff_date`, native history is preferred.
/// Falls back to whichever source has data when the preferred one is absent.
pub(super) fn merge_daily_savings(
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
pub(super) fn merge_hourly_savings(
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
