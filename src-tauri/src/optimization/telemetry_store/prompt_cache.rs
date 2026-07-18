use chrono::{DateTime, NaiveDateTime, Utc};

use super::super::cache_metrics::{CacheTokenMetrics, CacheTokenMetricsEvidence};
use super::open_connection;

pub(crate) fn prompt_cache_totals() -> CacheTokenMetrics {
    prompt_cache_totals_result().unwrap_or_else(|error| {
        log::warn!("optimization telemetry read failed: {error}");
        CacheTokenMetrics::default()
    })
}

/// Reads aggregate provider cache metrics without converting storage failures
/// into a fabricated zero. Consumers that need provenance (such as Token
/// X-Ray) should use this result directly and surface `unavailable` on error.
pub(crate) fn prompt_cache_totals_result() -> rusqlite::Result<CacheTokenMetrics> {
    Ok(prompt_cache_totals_evidence_result()?
        .map(|evidence| evidence.metrics)
        .unwrap_or_default())
}

/// Reads aggregate cache counters with the newest local observation time.
/// `None` is materially different from zero: it means no provider usage has
/// been recorded yet. The table intentionally does not retain provider names,
/// so consumers must label these as cross-provider aggregates.
pub(crate) fn prompt_cache_totals_evidence_result(
) -> rusqlite::Result<Option<CacheTokenMetricsEvidence>> {
    let conn = open_connection()?;
    conn.query_row(
        "SELECT
            COUNT(*),
            COALESCE(SUM(prompt_tokens), 0),
            COALESCE(SUM(completion_tokens), 0),
            COALESCE(SUM(cache_read_tokens), 0),
            COALESCE(SUM(cache_creation_tokens), 0),
            MAX(recorded_at)
        FROM prompt_cache_events",
        [],
        |row| {
            let count = row.get::<_, i64>(0)?.max(0);
            if count == 0 {
                return Ok(None);
            }
            let raw_observed_at: String = row.get(5)?;
            let observed_at = parse_sqlite_timestamp(&raw_observed_at).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid prompt cache recorded_at",
                    )),
                )
            })?;
            Ok(Some(CacheTokenMetricsEvidence {
                metrics: CacheTokenMetrics {
                    prompt_tokens: row.get::<_, i64>(1)?.max(0) as u64,
                    completion_tokens: row.get::<_, i64>(2)?.max(0) as u64,
                    cache_read_tokens: row.get::<_, i64>(3)?.max(0) as u64,
                    cache_creation_tokens: row.get::<_, i64>(4)?.max(0) as u64,
                },
                observed_at,
            }))
        },
    )
}

fn parse_sqlite_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|value| DateTime::from_naive_utc_and_offset(value, Utc))
}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    if let Ok(conn) = open_connection() {
        let _ = conn.execute("DELETE FROM prompt_cache_events", []);
        let _ = conn.execute("DELETE FROM compaction_decisions", []);
        let _ = conn.execute("DELETE FROM routing_decisions", []);
        let _ = conn.execute("DELETE FROM token_xray_bucket_events", []);
        let _ = conn.execute("DELETE FROM redundancy_hash_events", []);
        let _ = conn.execute("DELETE FROM rtk_preset_metadata_events", []);
    }
}
