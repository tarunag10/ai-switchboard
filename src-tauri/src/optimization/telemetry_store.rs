use std::path::PathBuf;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use super::cache_metrics::CacheTokenMetrics;
use super::telemetry::{
    CompactionDecisionRecord, RedundancyHashRecord, RoutingDecisionRecord, RtkPresetMetadata,
    TokenBucketMetrics,
};
use super::model_routing::{
    aggregate_model_routing_evidence, ModelRoutingEvidenceArm,
    ModelRoutingEvidenceObservation,
};

const DB_FILE: &str = "optimization_telemetry.sqlite";

fn db_path() -> PathBuf {
    crate::storage::app_data_dir().join(DB_FILE)
}

fn open_connection() -> rusqlite::Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS prompt_cache_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            prompt_tokens INTEGER NOT NULL,
            completion_tokens INTEGER NOT NULL,
            cache_read_tokens INTEGER NOT NULL,
            cache_creation_tokens INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS compaction_decisions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            should_compact INTEGER NOT NULL,
            context_used_percent INTEGER NOT NULL,
            threshold_percent INTEGER NOT NULL,
            reason TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS routing_decisions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            task TEXT NOT NULL,
            current_model TEXT NOT NULL,
            selected_model TEXT NOT NULL,
            fallback_model TEXT NOT NULL,
            reason TEXT NOT NULL,
            estimated_savings_percent INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS token_xray_bucket_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            bucket TEXT NOT NULL,
            tokens INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS redundancy_hash_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            source_id TEXT NOT NULL,
            content_sha256 TEXT NOT NULL,
            estimated_tokens INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS rtk_preset_metadata_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            preset_id TEXT NOT NULL,
            label TEXT NOT NULL,
            command TEXT NOT NULL,
            focus TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS model_routing_evidence_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id TEXT NOT NULL,
            captured_at TEXT NOT NULL,
            task_class TEXT NOT NULL,
            arm TEXT NOT NULL,
            baseline_model TEXT NOT NULL,
            candidate_model TEXT NOT NULL,
            succeeded INTEGER NOT NULL,
            successful_task_cost_microunits INTEGER,
            quality_score_bps INTEGER NOT NULL,
            latency_ms INTEGER NOT NULL,
            follow_up_rework INTEGER NOT NULL
        );",
    )?;
    Ok(conn)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingEvidenceArtifact {
    pub(crate) schema_version: u32,
    pub(crate) evidence_class: &'static str,
    pub(crate) minimum_samples: u64,
    pub(crate) baseline: ModelRoutingEvidenceArmMetrics,
    pub(crate) candidate: ModelRoutingEvidenceArmMetrics,
    pub(crate) provenance: ModelRoutingEvidenceProvenance,
    pub(crate) promotion_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingEvidenceArmMetrics {
    pub(crate) sample_count: u64,
    pub(crate) success_rate_bps: u32,
    pub(crate) quality_score_bps: u32,
    pub(crate) p95_latency_ms: u64,
    pub(crate) successful_task_cost_micros: u64,
    pub(crate) follow_up_rework_rate_bps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingEvidenceProvenance {
    pub(crate) task_class: String,
    pub(crate) baseline_model: String,
    pub(crate) candidate_model: String,
    pub(crate) source: &'static str,
    pub(crate) run_id: String,
    pub(crate) captured_at: String,
}

const MODEL_ROUTING_EVIDENCE_MINIMUM_SAMPLES: u64 = 50;

/// Persist only redacted routing outcome metrics. Request and response bodies
/// are intentionally not represented by this schema.
pub(crate) fn record_model_routing_evidence(
    observation: &ModelRoutingEvidenceObservation,
) -> Result<(), String> {
    try_record_model_routing_evidence(observation)
        .map_err(|error| format!("model-routing evidence persist failed: {error}"))
}

fn try_record_model_routing_evidence(
    observation: &ModelRoutingEvidenceObservation,
) -> rusqlite::Result<()> {
    let valid_identifier = |value: &str| {
        let trimmed = value.trim();
        !trimmed.is_empty()
            && trimmed.len() <= 128
            && trimmed.chars().all(|character| !character.is_control())
    };
    let captured_at = DateTime::parse_from_rfc3339(observation.captured_at.trim())
        .ok()
        .map(|value| value.with_timezone(&Utc));
    let valid = !valid_identifier(&observation.run_id)
        || captured_at.is_none()
        || captured_at.is_some_and(|value| value > Utc::now() + chrono::Duration::minutes(5))
        || !valid_identifier(&observation.captured_at)
        || !valid_identifier(&observation.task_class)
        || !valid_identifier(&observation.baseline_model)
        || !valid_identifier(&observation.candidate_model)
        || observation.baseline_model.trim().eq_ignore_ascii_case(observation.candidate_model.trim())
        || observation.quality_score_bps > 10_000
        || observation.latency_ms > i64::MAX as u64
        || observation
            .successful_task_cost_microunits
            .is_some_and(|value| value > i64::MAX as u64);
    if valid {
        return Err(rusqlite::Error::InvalidParameterName(
            "invalid redacted model-routing evidence observation".to_string(),
        ));
    }
    if observation.succeeded && observation.successful_task_cost_microunits.is_none() {
        return Ok(());
    }
    let arm = match observation.arm {
        ModelRoutingEvidenceArm::Baseline => "baseline",
        ModelRoutingEvidenceArm::Candidate => "candidate",
    };
    let conn = open_connection()?;
    conn.execute(
        "INSERT INTO model_routing_evidence_events (
            run_id, captured_at, task_class, arm, baseline_model, candidate_model,
            succeeded, successful_task_cost_microunits, quality_score_bps, latency_ms,
            follow_up_rework
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            observation.run_id.trim(),
            observation.captured_at.trim(),
            observation.task_class.trim().to_ascii_lowercase(),
            arm,
            observation.baseline_model.trim(),
            observation.candidate_model.trim(),
            i64::from(observation.succeeded),
            observation.successful_task_cost_microunits.map(|value| value as i64),
            observation.quality_score_bps as i64,
            observation.latency_ms as i64,
            i64::from(observation.follow_up_rework),
        ],
    )?;
    Ok(())
}

/// Reconcile one complete run into the machine-readable evidence shape used
/// by the repository checker. Local runtime evidence is permanently
/// observe-only; it cannot turn on automatic routing by itself.
pub(crate) fn export_model_routing_evidence(
    run_id: &str,
    task_class: &str,
) -> Result<ModelRoutingEvidenceArtifact, String> {
    let run_id = run_id.trim();
    let task_class = task_class.trim().to_ascii_lowercase();
    if run_id.is_empty() || task_class.is_empty() {
        return Err("model-routing evidence export requires run_id and task_class".to_string());
    }
    let conn = open_connection().map_err(|error| format!("open model-routing evidence: {error}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT captured_at, arm, baseline_model, candidate_model, succeeded,
                    successful_task_cost_microunits, quality_score_bps, latency_ms,
                    follow_up_rework
             FROM model_routing_evidence_events
             WHERE run_id = ?1 AND task_class = ?2
             ORDER BY id ASC",
        )
        .map_err(|error| format!("prepare model-routing evidence export: {error}"))?;
    let rows = stmt
        .query_map(params![run_id, task_class], |row| {
            let arm: String = row.get(1)?;
            let arm = match arm.as_str() {
                "baseline" => ModelRoutingEvidenceArm::Baseline,
                "candidate" => ModelRoutingEvidenceArm::Candidate,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok(ModelRoutingEvidenceObservation {
                run_id: run_id.to_string(),
                captured_at: row.get(0)?,
                task_class: task_class.clone(),
                arm,
                baseline_model: row.get(2)?,
                candidate_model: row.get(3)?,
                succeeded: row.get::<_, i64>(4)? != 0,
                successful_task_cost_microunits: row.get::<_, Option<i64>>(5)?.map(|v| v.max(0) as u64),
                quality_score_bps: row.get::<_, i64>(6)?.max(0) as u32,
                latency_ms: row.get::<_, i64>(7)?.max(0) as u64,
                follow_up_rework: row.get::<_, i64>(8)? != 0,
            })
        })
        .map_err(|error| format!("query model-routing evidence export: {error}"))?;
    let observations = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| format!("read model-routing evidence export: {error}"))?;
    if observations.is_empty() {
        return Err("model-routing evidence export has no observations".to_string());
    }
    let baseline_model = observations[0].baseline_model.clone();
    let candidate_model = observations[0].candidate_model.clone();
    if observations.iter().any(|observation| {
        observation.baseline_model != baseline_model || observation.candidate_model != candidate_model
    }) {
        return Err("model-routing evidence export mixes model identities".to_string());
    }
    let captured_at = observations
        .iter()
        .map(|observation| observation.captured_at.as_str())
        .max()
        .unwrap_or_default()
        .to_string();
    let samples = observations.iter().map(|observation| observation.sample()).collect::<Vec<_>>();
    let evidence = aggregate_model_routing_evidence(&samples, &task_class)?;
    let rate = |successes: u64| -> u32 {
        ((successes.saturating_mul(10_000)) / evidence.sample_size.max(1)) as u32
    };
    let rework = |arm: ModelRoutingEvidenceArm| -> u32 {
        let selected = samples.iter().filter(|sample| sample.arm == arm).collect::<Vec<_>>();
        ((selected.iter().filter(|sample| sample.follow_up_rework).count() as u64 * 10_000)
            / selected.len().max(1) as u64) as u32
    };
    let quality = |arm: ModelRoutingEvidenceArm| -> u32 {
        let selected = samples.iter().filter(|sample| sample.arm == arm).collect::<Vec<_>>();
        (selected.iter().map(|sample| sample.quality_score_bps as u64).sum::<u64>()
            / selected.len().max(1) as u64) as u32
    };
    let baseline_count = samples.iter().filter(|sample| sample.arm == ModelRoutingEvidenceArm::Baseline).count() as u64;
    let candidate_count = samples.iter().filter(|sample| sample.arm == ModelRoutingEvidenceArm::Candidate).count() as u64;
    Ok(ModelRoutingEvidenceArtifact {
        schema_version: 1,
        evidence_class: "local_runtime_observation",
        minimum_samples: MODEL_ROUTING_EVIDENCE_MINIMUM_SAMPLES,
        baseline: ModelRoutingEvidenceArmMetrics {
            sample_count: baseline_count,
            success_rate_bps: rate(evidence.baseline_successes),
            quality_score_bps: quality(ModelRoutingEvidenceArm::Baseline),
            p95_latency_ms: evidence.baseline_p95_latency_ms,
            successful_task_cost_micros: evidence.baseline_average_success_cost_microunits,
            follow_up_rework_rate_bps: rework(ModelRoutingEvidenceArm::Baseline),
        },
        candidate: ModelRoutingEvidenceArmMetrics {
            sample_count: candidate_count,
            success_rate_bps: rate(evidence.candidate_successes),
            quality_score_bps: quality(ModelRoutingEvidenceArm::Candidate),
            p95_latency_ms: evidence.candidate_p95_latency_ms,
            successful_task_cost_micros: evidence.candidate_average_success_cost_microunits,
            follow_up_rework_rate_bps: evidence.follow_up_rework_rate_bps,
        },
        provenance: ModelRoutingEvidenceProvenance {
            task_class,
            baseline_model,
            candidate_model,
            source: "local_runtime_observation",
            run_id: run_id.to_string(),
            captured_at,
        },
        promotion_eligible: false,
    })
}

pub(crate) fn record_prompt_cache_metrics(metrics: &CacheTokenMetrics) {
    if metrics.total_tokens() == 0 {
        return;
    }
    if let Err(error) = try_record_prompt_cache_metrics(metrics) {
        log::warn!("optimization telemetry persist failed: {error}");
    }
}

fn try_record_prompt_cache_metrics(metrics: &CacheTokenMetrics) -> rusqlite::Result<()> {
    let conn = open_connection()?;
    conn.execute(
        "INSERT INTO prompt_cache_events (
            prompt_tokens,
            completion_tokens,
            cache_read_tokens,
            cache_creation_tokens
        ) VALUES (?1, ?2, ?3, ?4)",
        params![
            metrics.prompt_tokens as i64,
            metrics.completion_tokens as i64,
            metrics.cache_read_tokens as i64,
            metrics.cache_creation_tokens as i64
        ],
    )?;
    Ok(())
}

pub(crate) fn record_compaction_decision(decision: &CompactionDecisionRecord) {
    if let Err(error) = try_record_compaction_decision(decision) {
        log::warn!("optimization compaction telemetry persist failed: {error}");
    }
}

fn try_record_compaction_decision(decision: &CompactionDecisionRecord) -> rusqlite::Result<()> {
    let conn = open_connection()?;
    conn.execute(
        "INSERT INTO compaction_decisions (
            should_compact,
            context_used_percent,
            threshold_percent,
            reason
        ) VALUES (?1, ?2, ?3, ?4)",
        params![
            i64::from(decision.should_compact),
            decision.context_used_percent as i64,
            decision.threshold_percent as i64,
            decision.reason
        ],
    )?;
    Ok(())
}

pub(crate) fn latest_compaction_decision() -> Option<CompactionDecisionRecord> {
    try_latest_compaction_decision().unwrap_or_else(|error| {
        log::warn!("optimization compaction telemetry read failed: {error}");
        None
    })
}

fn try_latest_compaction_decision() -> rusqlite::Result<Option<CompactionDecisionRecord>> {
    let conn = open_connection()?;
    let mut stmt = conn.prepare(
        "SELECT should_compact, context_used_percent, threshold_percent, reason
        FROM compaction_decisions
        ORDER BY id DESC
        LIMIT 1",
    )?;
    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        return Ok(Some(CompactionDecisionRecord {
            should_compact: row.get::<_, i64>(0)? != 0,
            context_used_percent: row.get::<_, i64>(1)?.clamp(0, 100) as u8,
            threshold_percent: row.get::<_, i64>(2)?.clamp(0, 100) as u8,
            reason: row.get(3)?,
        }));
    }

    Ok(None)
}

pub(crate) fn record_routing_decision(decision: &RoutingDecisionRecord) {
    if let Err(error) = try_record_routing_decision(decision) {
        log::warn!("optimization routing telemetry persist failed: {error}");
    }
}

fn try_record_routing_decision(decision: &RoutingDecisionRecord) -> rusqlite::Result<()> {
    let conn = open_connection()?;
    conn.execute(
        "INSERT INTO routing_decisions (
            task,
            current_model,
            selected_model,
            fallback_model,
            reason,
            estimated_savings_percent
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            decision.task,
            decision.current_model,
            decision.selected_model,
            decision.fallback_model,
            decision.reason,
            decision.estimated_savings_percent as i64
        ],
    )?;
    Ok(())
}

pub(crate) fn recent_routing_decisions(limit: usize) -> Vec<RoutingDecisionRecord> {
    try_recent_routing_decisions(limit).unwrap_or_else(|error| {
        log::warn!("optimization routing telemetry read failed: {error}");
        Vec::new()
    })
}

fn try_recent_routing_decisions(limit: usize) -> rusqlite::Result<Vec<RoutingDecisionRecord>> {
    let conn = open_connection()?;
    let mut stmt = conn.prepare(
        "SELECT task, current_model, selected_model, fallback_model, reason, estimated_savings_percent
        FROM routing_decisions
        ORDER BY id DESC
        LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(RoutingDecisionRecord {
            task: row.get(0)?,
            current_model: row.get(1)?,
            selected_model: row.get(2)?,
            fallback_model: row.get(3)?,
            reason: row.get(4)?,
            estimated_savings_percent: row.get::<_, i64>(5)?.clamp(0, 100) as u8,
        })
    })?;

    let mut decisions = Vec::new();
    for row in rows {
        decisions.push(row?);
    }
    decisions.reverse();
    Ok(decisions)
}

pub(crate) fn record_token_xray_bucket(bucket: &str, tokens: u64) {
    if bucket.trim().is_empty() || tokens == 0 {
        return;
    }
    if let Err(error) = try_record_token_xray_bucket(bucket, tokens) {
        log::warn!("optimization token x-ray telemetry persist failed: {error}");
    }
}

fn try_record_token_xray_bucket(bucket: &str, tokens: u64) -> rusqlite::Result<()> {
    let conn = open_connection()?;
    conn.execute(
        "INSERT INTO token_xray_bucket_events (bucket, tokens) VALUES (?1, ?2)",
        params![bucket, tokens as i64],
    )?;
    Ok(())
}

pub(crate) fn token_xray_bucket_totals() -> Vec<TokenBucketMetrics> {
    try_token_xray_bucket_totals().unwrap_or_else(|error| {
        log::warn!("optimization token x-ray telemetry read failed: {error}");
        Vec::new()
    })
}

fn try_token_xray_bucket_totals() -> rusqlite::Result<Vec<TokenBucketMetrics>> {
    let conn = open_connection()?;
    let mut stmt = conn.prepare(
        "SELECT bucket, COALESCE(SUM(tokens), 0)
        FROM token_xray_bucket_events
        GROUP BY bucket
        ORDER BY bucket ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TokenBucketMetrics {
            bucket: row.get(0)?,
            tokens: row.get::<_, i64>(1)?.max(0) as u64,
        })
    })?;

    let mut buckets = Vec::new();
    for row in rows {
        buckets.push(row?);
    }
    Ok(buckets)
}

pub(crate) fn record_redundancy_hash(record: &RedundancyHashRecord) {
    if record.source_id.trim().is_empty() || record.content_sha256.trim().is_empty() {
        return;
    }
    if let Err(error) = try_record_redundancy_hash(record) {
        log::warn!("optimization redundancy telemetry persist failed: {error}");
    }
}

fn try_record_redundancy_hash(record: &RedundancyHashRecord) -> rusqlite::Result<()> {
    let conn = open_connection()?;
    conn.execute(
        "INSERT INTO redundancy_hash_events (
            source_id,
            content_sha256,
            estimated_tokens
        ) VALUES (?1, ?2, ?3)",
        params![
            record.source_id,
            record.content_sha256,
            record.estimated_tokens as i64
        ],
    )?;
    Ok(())
}

pub(crate) fn recent_redundancy_hashes(limit: usize) -> Vec<RedundancyHashRecord> {
    try_recent_redundancy_hashes(limit).unwrap_or_else(|error| {
        log::warn!("optimization redundancy telemetry read failed: {error}");
        Vec::new()
    })
}

fn try_recent_redundancy_hashes(limit: usize) -> rusqlite::Result<Vec<RedundancyHashRecord>> {
    let conn = open_connection()?;
    let mut stmt = conn.prepare(
        "SELECT source_id, content_sha256, estimated_tokens
        FROM redundancy_hash_events
        ORDER BY id DESC
        LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(RedundancyHashRecord {
            source_id: row.get(0)?,
            content_sha256: row.get(1)?,
            estimated_tokens: row.get::<_, i64>(2)?.max(0) as u64,
        })
    })?;

    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }
    records.reverse();
    Ok(records)
}

pub(crate) fn record_rtk_preset_metadata(metadata: &RtkPresetMetadata) {
    if metadata.id.trim().is_empty() {
        return;
    }
    if let Err(error) = try_record_rtk_preset_metadata(metadata) {
        log::warn!("optimization RTK preset telemetry persist failed: {error}");
    }
}

fn try_record_rtk_preset_metadata(metadata: &RtkPresetMetadata) -> rusqlite::Result<()> {
    let conn = open_connection()?;
    conn.execute(
        "INSERT INTO rtk_preset_metadata_events (
            preset_id,
            label,
            command,
            focus
        ) VALUES (?1, ?2, ?3, ?4)",
        params![
            metadata.id,
            metadata.label,
            metadata.command,
            metadata.focus
        ],
    )?;
    Ok(())
}

pub(crate) fn recent_rtk_preset_metadata(limit: usize) -> Vec<RtkPresetMetadata> {
    try_recent_rtk_preset_metadata(limit).unwrap_or_else(|error| {
        log::warn!("optimization RTK preset telemetry read failed: {error}");
        Vec::new()
    })
}

fn try_recent_rtk_preset_metadata(limit: usize) -> rusqlite::Result<Vec<RtkPresetMetadata>> {
    let conn = open_connection()?;
    let mut stmt = conn.prepare(
        "SELECT preset_id, label, command, focus
        FROM rtk_preset_metadata_events
        ORDER BY id DESC
        LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(RtkPresetMetadata {
            id: row.get(0)?,
            label: row.get(1)?,
            command: row.get(2)?,
            focus: row.get(3)?,
        })
    })?;

    let mut metadata = Vec::new();
    for row in rows {
        metadata.push(row?);
    }
    metadata.reverse();
    Ok(metadata)
}

mod prompt_cache;

pub(crate) use prompt_cache::{prompt_cache_totals, prompt_cache_totals_evidence_result};

#[cfg(test)]
pub(crate) use prompt_cache::reset_for_tests;
