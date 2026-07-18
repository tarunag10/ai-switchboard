//! Local-only exact response cache primitives.
//!
//! The application uses this module for an opt-in exact replay path without
//! exposing prompt text as a SQLite key: only a SHA-256 key and response
//! metadata are persisted.

use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CacheNamespace {
    pub provider: String,
    pub model: String,
    pub account: String,
    pub workspace: String,
    pub policy: String,
    /// Stable hash of non-credential request headers that can affect a
    /// provider response. Credential headers are represented by `account`.
    #[serde(default)]
    pub request_variant: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheRequest<'a> {
    pub namespace: CacheNamespace,
    pub prompt: &'a str,
    pub streaming: bool,
    pub has_tools_or_mcp: bool,
    pub sensitive_data: bool,
    pub temperature: f32,
    pub repo_state_changed_rapidly: bool,
    pub open_tool_calls: bool,
    pub no_cache: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BypassReason {
    Streaming,
    ToolsOrMcp,
    SensitiveData,
    HighTemperature,
    RapidRepoState,
    OpenToolCalls,
    NoCacheMarker,
}

pub const DEFAULT_HIGH_TEMPERATURE: f32 = 0.7;

impl<'a> CacheRequest<'a> {
    pub fn bypass_reasons(&self) -> Vec<BypassReason> {
        let mut reasons = Vec::new();
        if self.streaming {
            reasons.push(BypassReason::Streaming);
        }
        if self.has_tools_or_mcp {
            reasons.push(BypassReason::ToolsOrMcp);
        }
        if self.sensitive_data {
            reasons.push(BypassReason::SensitiveData);
        }
        if self.temperature > DEFAULT_HIGH_TEMPERATURE {
            reasons.push(BypassReason::HighTemperature);
        }
        if self.repo_state_changed_rapidly {
            reasons.push(BypassReason::RapidRepoState);
        }
        if self.open_tool_calls {
            reasons.push(BypassReason::OpenToolCalls);
        }
        if self.no_cache {
            reasons.push(BypassReason::NoCacheMarker);
        }
        reasons
    }

    pub fn cacheable(&self) -> bool {
        self.bypass_reasons().is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheResponse {
    pub body: String,
    pub metadata: serde_json::Value,
    pub ttl_seconds: u64,
    pub no_cache: bool,
    pub status_code: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheHit {
    pub body: String,
    pub metadata_json: String,
    pub key_hash: String,
    pub status_code: u16,
}

pub struct SemanticCache {
    connection: Connection,
}

impl SemanticCache {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        let cache = Self { connection };
        cache.initialize()?;
        Ok(cache)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::open(":memory:")
    }

    fn initialize(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA secure_delete=ON;
            CREATE TABLE IF NOT EXISTS semantic_cache (
              key_hash TEXT PRIMARY KEY, provider TEXT NOT NULL, model TEXT NOT NULL,
              account TEXT NOT NULL, workspace TEXT NOT NULL, policy TEXT NOT NULL,
              response TEXT NOT NULL, metadata TEXT NOT NULL, status_code INTEGER NOT NULL DEFAULT 200, expires_at INTEGER NOT NULL,
              no_cache INTEGER NOT NULL DEFAULT 0, invalidated INTEGER NOT NULL DEFAULT 0
            );")?;
        // Older preview builds created the table without status_code. Keep
        // upgrades non-destructive and ignore the expected duplicate-column
        // error on fresh/current databases.
        let _ = self.connection.execute(
            "ALTER TABLE semantic_cache ADD COLUMN status_code INTEGER NOT NULL DEFAULT 200",
            [],
        );
        self.purge_expired()?;
        Ok(())
    }

    fn purge_expired(&self) -> rusqlite::Result<usize> {
        self.connection.execute(
            "DELETE FROM semantic_cache WHERE expires_at <= ?1 OR invalidated != 0",
            params![unix_seconds()],
        )
    }

    pub fn key_hash(request: &CacheRequest<'_>) -> String {
        let namespace =
            serde_json::to_string(&request.namespace).expect("namespace is serializable");
        let mut hasher = Sha256::new();
        hasher.update((namespace.len() as u64).to_be_bytes());
        hasher.update(namespace.as_bytes());
        hasher.update((request.prompt.len() as u64).to_be_bytes());
        hasher.update(request.prompt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn get(&self, request: &CacheRequest<'_>) -> rusqlite::Result<Option<CacheHit>> {
        if !request.cacheable() {
            return Ok(None);
        }
        self.purge_expired()?;
        let key = Self::key_hash(request);
        let now = unix_seconds();
        let hit = self.connection.query_row(
            "SELECT response, metadata, key_hash, status_code FROM semantic_cache WHERE key_hash=?1 AND expires_at>?2 AND no_cache=0 AND invalidated=0",
            params![key, now], |row| Ok(CacheHit { body: row.get(0)?, metadata_json: row.get(1)?, key_hash: row.get(2)?, status_code: row.get::<_, u16>(3)? }))
            .optional()?;
        Ok(hit)
    }

    pub fn put(
        &self,
        request: &CacheRequest<'_>,
        response: &CacheResponse,
    ) -> rusqlite::Result<bool> {
        if !request.cacheable() || response.no_cache || response.ttl_seconds == 0 {
            return Ok(false);
        }
        let key = Self::key_hash(request);
        let metadata = serde_json::to_string(&response.metadata)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        self.connection.execute("INSERT OR REPLACE INTO semantic_cache
            (key_hash,provider,model,account,workspace,policy,response,metadata,status_code,expires_at,no_cache,invalidated)
            VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,0,0)", params![key, request.namespace.provider, request.namespace.model, request.namespace.account, request.namespace.workspace, request.namespace.policy, response.body, metadata, response.status_code, unix_seconds().saturating_add(response.ttl_seconds as i64)])?;
        Ok(true)
    }

    pub fn invalidate(&self, request: &CacheRequest<'_>) -> rusqlite::Result<bool> {
        let changed = self.connection.execute(
            "DELETE FROM semantic_cache WHERE key_hash=?1",
            params![Self::key_hash(request)],
        )?;
        Ok(changed != 0)
    }

    pub fn invalidate_namespace(&self, namespace: &CacheNamespace) -> rusqlite::Result<usize> {
        Ok(self.connection.execute("DELETE FROM semantic_cache WHERE provider=?1 AND model=?2 AND account=?3 AND workspace=?4 AND policy=?5", params![namespace.provider, namespace.model, namespace.account, namespace.workspace, namespace.policy])?)
    }

    pub fn clear(&self) -> rusqlite::Result<usize> {
        Ok(self.connection.execute("DELETE FROM semantic_cache", [])?)
    }

    pub fn entry_count(&self) -> rusqlite::Result<u64> {
        self.purge_expired()?;
        self.connection.query_row(
            "SELECT COUNT(*) FROM semantic_cache WHERE invalidated=0 AND expires_at>?1",
            params![unix_seconds()],
            |row| row.get(0),
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticCacheStatus {
    pub enabled: bool,
    pub entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub database_path: String,
    pub policy: &'static str,
    pub disclosure: &'static str,
}

/// App-owned exact cache service. It is deliberately opt-in and local-only;
/// the proxy uses it only for safe, non-streaming requests. The prompt is
/// hashed before SQLite sees it, and all cache failures are fail-open.
pub struct SemanticCacheService {
    cache: Mutex<SemanticCache>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
    enabled: AtomicBool,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl SemanticCacheService {
    pub fn open(
        database_path: impl AsRef<Path>,
        state_path: impl AsRef<Path>,
    ) -> rusqlite::Result<Self> {
        if let Some(parent) = database_path.as_ref().parent() {
            let _ = fs::create_dir_all(parent);
        }
        let enabled = fs::read_to_string(state_path.as_ref())
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|value| value.get("enabled").and_then(|value| value.as_bool()))
            .unwrap_or(false);
        Ok(Self {
            cache: Mutex::new(SemanticCache::open(database_path.as_ref())?),
            database_path: database_path.as_ref().to_path_buf(),
            state_path: state_path.as_ref().to_path_buf(),
            enabled: AtomicBool::new(enabled),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn set_enabled(&self, enabled: bool) -> std::io::Result<()> {
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = serde_json::json!({"version": 1, "enabled": enabled});
        let tmp = self.state_path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec(&payload).unwrap_or_default())?;
        fs::rename(tmp, &self.state_path)?;
        self.enabled.store(enabled, Ordering::Release);
        Ok(())
    }

    pub fn get(&self, request: &CacheRequest<'_>) -> Option<CacheHit> {
        if !self.enabled() || !request.cacheable() {
            return None;
        }
        match self.cache.lock().get(request) {
            Ok(Some(hit)) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(hit)
            }
            Ok(None) => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
            Err(err) => {
                log::warn!("semantic cache read failed: {err}");
                None
            }
        }
    }

    pub fn put(&self, request: &CacheRequest<'_>, response: &CacheResponse) {
        if !self.enabled() || !request.cacheable() {
            return;
        }
        if let Err(err) = self.cache.lock().put(request, response) {
            log::warn!("semantic cache write failed: {err}");
        }
    }

    pub fn clear(&self) -> rusqlite::Result<usize> {
        self.cache.lock().clear()
    }

    pub fn status(&self) -> SemanticCacheStatus {
        let entries = self.cache.lock().entry_count().unwrap_or(0);
        SemanticCacheStatus {
            enabled: self.enabled(),
            entries,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            database_path: self.database_path.display().to_string(),
            policy: "exact-v1",
            disclosure: "Local exact replay only; response bodies remain in local app storage until TTL or clear. Streaming, tools/MCP, image content, sensitive markers in requests or responses, high temperature, and no-cache requests bypass. Marker screening is conservative, not a guarantee of secrecy.",
        }
    }
}

#[tauri::command]
pub fn get_semantic_cache_status(
    state: tauri::State<'_, crate::state::AppState>,
) -> SemanticCacheStatus {
    state.semantic_cache.status()
}

#[tauri::command]
pub fn set_semantic_cache_enabled(
    state: tauri::State<'_, crate::state::AppState>,
    enabled: bool,
) -> Result<SemanticCacheStatus, String> {
    if enabled
        && matches!(
            crate::client_adapters::load_switchboard_mode(),
            Some(crate::models::SwitchboardMode::Off | crate::models::SwitchboardMode::Rtk)
        )
    {
        return Err(
            "Semantic Cache requires Full or Headroom mode; Off and RTK-only modes do not serve cached provider responses."
                .into(),
        );
    }
    state
        .semantic_cache
        .set_enabled(enabled)
        .map_err(|err| err.to_string())?;
    Ok(state.semantic_cache.status())
}

#[tauri::command]
pub fn clear_semantic_cache(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<SemanticCacheStatus, String> {
    state
        .semantic_cache
        .clear()
        .map_err(|err| err.to_string())?;
    Ok(state.semantic_cache.status())
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> CacheRequest<'static> {
        CacheRequest {
            namespace: CacheNamespace {
                provider: "local".into(),
                model: "m".into(),
                account: "a".into(),
                workspace: "w".into(),
                policy: "p".into(),
                request_variant: String::new(),
            },
            prompt: "hello",
            ..Default::default()
        }
    }
    fn response() -> CacheResponse {
        CacheResponse {
            body: "world".into(),
            metadata: serde_json::json!({"tokens": 2}),
            ttl_seconds: 60,
            no_cache: false,
            status_code: 200,
        }
    }

    #[test]
    fn round_trip_stores_only_hashed_key() {
        let c = SemanticCache::open_in_memory().unwrap();
        let r = request();
        assert!(c.put(&r, &response()).unwrap());
        assert_eq!(c.get(&r).unwrap().unwrap().body, "world");
        let stored: String = c
            .connection
            .query_row("SELECT key_hash FROM semantic_cache", [], |row| row.get(0))
            .unwrap();
        assert!(!stored.contains("hello"));
    }
    #[test]
    fn ttl_and_invalidation_are_enforced() {
        let c = SemanticCache::open_in_memory().unwrap();
        let r = request();
        let mut v = response();
        v.ttl_seconds = 0;
        assert!(!c.put(&r, &v).unwrap());
        assert!(c.put(&r, &response()).unwrap());
        assert!(c.invalidate(&r).unwrap());
        assert!(c.get(&r).unwrap().is_none());
    }

    #[test]
    fn expired_rows_are_purged_instead_of_retaining_response_bodies() {
        let c = SemanticCache::open_in_memory().unwrap();
        let r = request();
        let mut v = response();
        v.ttl_seconds = 1;
        assert!(c.put(&r, &v).unwrap());
        c.connection
            .execute(
                "UPDATE semantic_cache SET expires_at=?1",
                params![unix_seconds().saturating_sub(1)],
            )
            .unwrap();
        assert_eq!(c.entry_count().unwrap(), 0);
        let rows: u64 = c
            .connection
            .query_row("SELECT COUNT(*) FROM semantic_cache", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }
    #[test]
    fn unsafe_requests_bypass_without_db_access() {
        let mut r = request();
        r.streaming = true;
        r.temperature = 0.9;
        r.no_cache = true;
        assert_eq!(
            r.bypass_reasons(),
            vec![
                BypassReason::Streaming,
                BypassReason::HighTemperature,
                BypassReason::NoCacheMarker
            ]
        );
    }

    #[test]
    fn service_is_opt_in_and_reports_hit_miss_and_clear_evidence() {
        let state_path = std::env::temp_dir().join(format!(
            "ai-switchboard-semantic-cache-test-{}-{}.json",
            std::process::id(),
            unix_seconds()
        ));
        let service = SemanticCacheService::open(":memory:", &state_path).unwrap();
        let request = request();
        assert!(!service.enabled());
        assert!(service.get(&request).is_none());
        service.set_enabled(true).unwrap();
        service.put(&request, &response());
        assert_eq!(service.get(&request).unwrap().body, "world");
        let status = service.status();
        assert_eq!(status.entries, 1);
        assert_eq!(status.hits, 1);
        service.clear().unwrap();
        assert_eq!(service.status().entries, 0);
        service.set_enabled(false).unwrap();
        let _ = std::fs::remove_file(state_path);
    }
}
