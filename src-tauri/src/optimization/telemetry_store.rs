use std::path::PathBuf;

use rusqlite::{params, Connection};

use super::cache_metrics::CacheTokenMetrics;

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
    let _ = std::fs::remove_file(db_path());
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
}
