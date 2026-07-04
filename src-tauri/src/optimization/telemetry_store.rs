use std::path::PathBuf;

use rusqlite::{params, Connection};

use super::cache_metrics::CacheTokenMetrics;
use super::telemetry::{CompactionDecisionRecord, RoutingDecisionRecord};

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

pub(crate) fn prompt_cache_totals() -> CacheTokenMetrics {
    try_prompt_cache_totals().unwrap_or_else(|error| {
        log::warn!("optimization telemetry read failed: {error}");
        CacheTokenMetrics::default()
    })
}

fn try_prompt_cache_totals() -> rusqlite::Result<CacheTokenMetrics> {
    let conn = open_connection()?;
    conn.query_row(
        "SELECT
            COALESCE(SUM(prompt_tokens), 0),
            COALESCE(SUM(completion_tokens), 0),
            COALESCE(SUM(cache_read_tokens), 0),
            COALESCE(SUM(cache_creation_tokens), 0)
        FROM prompt_cache_events",
        [],
        |row| {
            Ok(CacheTokenMetrics {
                prompt_tokens: row.get::<_, i64>(0)?.max(0) as u64,
                completion_tokens: row.get::<_, i64>(1)?.max(0) as u64,
                cache_read_tokens: row.get::<_, i64>(2)?.max(0) as u64,
                cache_creation_tokens: row.get::<_, i64>(3)?.max(0) as u64,
            })
        },
    )
}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    if let Ok(conn) = open_connection() {
        let _ = conn.execute("DELETE FROM prompt_cache_events", []);
        let _ = conn.execute("DELETE FROM compaction_decisions", []);
        let _ = conn.execute("DELETE FROM routing_decisions", []);
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn prompt_cache_metrics_round_trip_through_sqlite() {
        let home = tempdir().expect("temp home");
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());

        let first = CacheTokenMetrics {
            prompt_tokens: 100,
            completion_tokens: 20,
            cache_read_tokens: 40,
            cache_creation_tokens: 10,
        };
        let second = CacheTokenMetrics {
            prompt_tokens: 50,
            completion_tokens: 10,
            cache_read_tokens: 25,
            cache_creation_tokens: 5,
        };

        record_prompt_cache_metrics(&first);
        record_prompt_cache_metrics(&second);

        assert_eq!(
            prompt_cache_totals(),
            CacheTokenMetrics {
                prompt_tokens: 150,
                completion_tokens: 30,
                cache_read_tokens: 65,
                cache_creation_tokens: 15,
            }
        );

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }
    #[test]
    fn routing_decisions_round_trip_through_sqlite() {
        let home = tempdir().expect("temp home");
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());

        reset_for_tests();
        record_routing_decision(&RoutingDecisionRecord {
            task: "commit message".to_string(),
            current_model: "gpt-5".to_string(),
            selected_model: "gpt-5-mini".to_string(),
            fallback_model: "gpt-5".to_string(),
            reason: "trivial task".to_string(),
            estimated_savings_percent: 42,
        });

        let decisions = recent_routing_decisions(8);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].selected_model, "gpt-5-mini");
        assert_eq!(decisions[0].estimated_savings_percent, 42);

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }
}
