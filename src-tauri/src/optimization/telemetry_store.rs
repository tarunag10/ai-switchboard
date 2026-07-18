use std::path::PathBuf;

use rusqlite::{params, Connection};

use super::cache_metrics::CacheTokenMetrics;
use super::telemetry::{
    CompactionDecisionRecord, RedundancyHashRecord, RoutingDecisionRecord, RtkPresetMetadata,
    TokenBucketMetrics,
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
        );",
    )?;
    Ok(conn)
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
