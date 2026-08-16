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
        self.bypass_reasons().is_empty() && self.namespace.is_scoped()
    }
}

impl CacheNamespace {
    /// A cache namespace must be explicit enough to prevent an anonymous or
    /// partially initialized caller from sharing responses with another
    /// scope. `request_variant` may be empty because it represents the
    /// absence of additional variant headers.
    pub fn is_scoped(&self) -> bool {
        !self.provider.trim().is_empty()
            && !self.model.trim().is_empty()
            && !self.account.trim().is_empty()
            && !self.workspace.trim().is_empty()
            && !self.policy.trim().is_empty()
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

pub struct ExactResponseCache {
    connection: Connection,
}

impl ExactResponseCache {
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
              account TEXT NOT NULL, workspace TEXT NOT NULL, policy TEXT NOT NULL, request_variant TEXT NOT NULL DEFAULT '',
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
        let _ = self.connection.execute(
            "ALTER TABLE semantic_cache ADD COLUMN request_variant TEXT NOT NULL DEFAULT ''",
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
            (key_hash,provider,model,account,workspace,policy,request_variant,response,metadata,status_code,expires_at,no_cache,invalidated)
            VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,0,0)", params![key, request.namespace.provider, request.namespace.model, request.namespace.account, request.namespace.workspace, request.namespace.policy, request.namespace.request_variant, response.body, metadata, response.status_code, unix_seconds().saturating_add(response.ttl_seconds as i64)])?;
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

    pub fn clear_with_receipt(&self) -> rusqlite::Result<ResponseCacheClearReceipt> {
        let entries_cleared = self.connection.execute("DELETE FROM semantic_cache", [])?;
        Ok(ResponseCacheClearReceipt {
            entries_cleared,
            restore_available: false,
        })
    }

    pub fn clear(&self) -> rusqlite::Result<usize> {
        Ok(self.clear_with_receipt()?.entries_cleared)
    }

    pub fn entry_count(&self) -> rusqlite::Result<u64> {
        self.purge_expired()?;
        self.connection.query_row(
            "SELECT COUNT(*) FROM semantic_cache WHERE invalidated=0 AND expires_at>?1",
            params![unix_seconds()],
            |row| row.get(0),
        )
    }

    fn ensure_namespace_stats_table(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS semantic_cache_namespace_stats (
              provider TEXT NOT NULL,
              model TEXT NOT NULL,
              account TEXT NOT NULL,
              workspace TEXT NOT NULL,
              policy TEXT NOT NULL,
              hits INTEGER NOT NULL DEFAULT 0,
              misses INTEGER NOT NULL DEFAULT 0,
              last_hit_at INTEGER,
              PRIMARY KEY (provider, model, account, workspace, policy)
            );",
        )?;
        Ok(())
    }

    pub fn record_namespace_hit(&self, namespace: &CacheNamespace) -> rusqlite::Result<()> {
        self.ensure_namespace_stats_table()?;
        let now = unix_seconds();
        self.connection.execute(
            "INSERT INTO semantic_cache_namespace_stats
              (provider, model, account, workspace, policy, hits, misses, last_hit_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, 0, ?6)
             ON CONFLICT(provider, model, account, workspace, policy) DO UPDATE SET
               hits = hits + 1,
               last_hit_at = excluded.last_hit_at",
            params![
                namespace.provider,
                namespace.model,
                namespace.account,
                namespace.workspace,
                namespace.policy,
                now
            ],
        )?;
        Ok(())
    }

    pub fn record_namespace_miss(&self, namespace: &CacheNamespace) -> rusqlite::Result<()> {
        self.ensure_namespace_stats_table()?;
        self.connection.execute(
            "INSERT INTO semantic_cache_namespace_stats
              (provider, model, account, workspace, policy, hits, misses, last_hit_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, NULL)
             ON CONFLICT(provider, model, account, workspace, policy) DO UPDATE SET
               misses = misses + 1",
            params![
                namespace.provider,
                namespace.model,
                namespace.account,
                namespace.workspace,
                namespace.policy
            ],
        )?;
        Ok(())
    }

    pub fn namespace_stats(&self) -> rusqlite::Result<Vec<SemanticCacheNamespaceStat>> {
        self.ensure_namespace_stats_table()?;
        self.purge_expired()?;
        let now = unix_seconds();
        let mut statement = self.connection.prepare(
            "SELECT s.provider, s.model, s.account, s.workspace, s.policy, s.hits, s.misses, s.last_hit_at,
                    COALESCE(e.entries, 0) AS entries
             FROM semantic_cache_namespace_stats s
             LEFT JOIN (
               SELECT provider, model, account, workspace, policy, COUNT(*) AS entries
               FROM semantic_cache
               WHERE invalidated=0 AND expires_at>?1
               GROUP BY provider, model, account, workspace, policy
             ) e ON e.provider=s.provider AND e.model=s.model AND e.account=s.account
                AND e.workspace=s.workspace AND e.policy=s.policy
             ORDER BY s.last_hit_at DESC, s.hits DESC, s.provider, s.model",
        )?;
        let rows = statement.query_map(params![now], |row| {
            Ok(SemanticCacheNamespaceStat {
                provider: row.get(0)?,
                model: row.get(1)?,
                account: row.get(2)?,
                workspace: row.get(3)?,
                policy: row.get(4)?,
                hits: row.get::<_, i64>(5)? as u64,
                misses: row.get::<_, i64>(6)? as u64,
                last_hit_at: row.get::<_, Option<i64>>(7)?,
                entries: row.get::<_, i64>(8)? as u64,
            })
        })?;
        rows.collect()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticCacheNamespaceStat {
    pub provider: String,
    pub model: String,
    pub account: String,
    pub workspace: String,
    pub policy: String,
    pub hits: u64,
    pub misses: u64,
    pub entries: u64,
    pub last_hit_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticCacheStats {
    pub namespaces: Vec<SemanticCacheNamespaceStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemanticCacheStateFile {
    version: u8,
    enabled: bool,
    #[serde(default)]
    semantic_v2_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticCacheStatus {
    pub enabled: bool,
    pub semantic_v2_enabled: bool,
    pub entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub storage_available: bool,
    pub read_failures: u64,
    pub write_failures: u64,
    pub evidence: &'static str,
    pub database_path: String,
    pub policy: &'static str,
    pub disclosure: &'static str,
}

/// Compatibility alias for the pre-taxonomy backend name. The SQLite table,
/// state filenames, command names, and serialized fields stay readable during
/// migration; new product contracts use Exact Response Cache terminology.
pub type SemanticCache = ExactResponseCache;
pub type SemanticCacheService = ExactResponseCacheService;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCacheClearReceipt {
    pub entries_cleared: usize,
    pub restore_available: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCacheClearResult {
    pub receipt: ResponseCacheClearReceipt,
    pub diagnostics: ResponseCacheDiagnostics,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCacheNamespaceDiagnostics {
    /// SHA-256 identifier over namespace fields. Account and workspace values
    /// are deliberately not exposed by the diagnostics API.
    pub namespace_id: String,
    pub entries: u64,
    pub hits: u64,
    pub misses: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCacheClearAction {
    pub command: &'static str,
    pub confirmation_phrase: &'static str,
    pub restore_command: Option<&'static str>,
    pub restore_available: bool,
    pub retains_local_restore_copy: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseCacheDiagnostics {
    pub product_name: &'static str,
    pub enabled: bool,
    pub exact_match_only: bool,
    pub matching_strategy: &'static str,
    pub entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub namespaces: Vec<ResponseCacheNamespaceDiagnostics>,
    pub storage_path: String,
    pub bypass_rules: Vec<&'static str>,
    pub safety_rules: Vec<&'static str>,
    pub clear_action: ResponseCacheClearAction,
}

/// App-owned exact cache service. It is deliberately opt-in and local-only;
/// the proxy uses it only for safe, non-streaming requests. The prompt is
/// hashed before SQLite sees it, and all cache failures are fail-open.
pub struct ExactResponseCacheService {
    cache: Mutex<ExactResponseCache>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
    enabled: AtomicBool,
    semantic_v2_enabled: AtomicBool,
    hits: AtomicU64,
    misses: AtomicU64,
    read_failures: AtomicU64,
    write_failures: AtomicU64,
    storage_available: AtomicBool,
}

impl ExactResponseCacheService {
    fn load_state_file(state_path: &Path) -> SemanticCacheStateFile {
        fs::read_to_string(state_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or(SemanticCacheStateFile {
                version: 1,
                enabled: false,
                semantic_v2_enabled: false,
            })
    }

    fn write_state_file_at(
        state_path: &Path,
        state: &SemanticCacheStateFile,
    ) -> std::io::Result<()> {
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = state_path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec(state).unwrap_or_default())?;
        fs::rename(tmp, state_path)?;
        Ok(())
    }

    fn write_state_file(&self, state: &SemanticCacheStateFile) -> std::io::Result<()> {
        Self::write_state_file_at(&self.state_path, state)
    }

    pub fn open(
        database_path: impl AsRef<Path>,
        state_path: impl AsRef<Path>,
    ) -> rusqlite::Result<Self> {
        if let Some(parent) = database_path.as_ref().parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut state = Self::load_state_file(state_path.as_ref());
        if state.semantic_v2_enabled {
            state.semantic_v2_enabled = false;
            if let Err(err) = Self::write_state_file_at(state_path.as_ref(), &state) {
                log::warn!(
                    "could not persist removal of legacy normalized-prefix v2 cache mode; it remains disabled in memory: {err}"
                );
            }
        }
        Ok(Self {
            cache: Mutex::new(ExactResponseCache::open(database_path.as_ref())?),
            database_path: database_path.as_ref().to_path_buf(),
            state_path: state_path.as_ref().to_path_buf(),
            enabled: AtomicBool::new(state.enabled),
            semantic_v2_enabled: AtomicBool::new(state.semantic_v2_enabled),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            read_failures: AtomicU64::new(0),
            write_failures: AtomicU64::new(0),
            storage_available: AtomicBool::new(true),
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn semantic_v2_enabled(&self) -> bool {
        self.semantic_v2_enabled.load(Ordering::Acquire)
    }

    pub fn set_enabled(&self, enabled: bool) -> std::io::Result<()> {
        let mut state = Self::load_state_file(&self.state_path);
        state.enabled = enabled;
        self.write_state_file(&state)?;
        self.enabled.store(enabled, Ordering::Release);
        Ok(())
    }

    pub fn set_semantic_v2_enabled(&self, enabled: bool) -> std::io::Result<()> {
        if enabled {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "legacy normalized-prefix v2 replay is unavailable; it is not semantic similarity",
            ));
        }
        let mut state = Self::load_state_file(&self.state_path);
        state.semantic_v2_enabled = false;
        self.write_state_file(&state)?;
        self.semantic_v2_enabled.store(false, Ordering::Release);
        Ok(())
    }

    pub fn get(&self, request: &CacheRequest<'_>) -> Option<CacheHit> {
        if !self.enabled() || !request.cacheable() {
            return None;
        }
        let lookup = {
            let cache = self.cache.lock();
            cache.get(request)
        };
        match lookup {
            Ok(Some(hit)) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                if let Err(err) = self.cache.lock().record_namespace_hit(&request.namespace) {
                    log::warn!("response cache namespace hit accounting failed: {err}");
                }
                Some(hit)
            }
            Ok(None) => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                if let Err(err) = self.cache.lock().record_namespace_miss(&request.namespace) {
                    log::warn!("response cache namespace miss accounting failed: {err}");
                }
                None
            }
            Err(err) => {
                self.read_failures.fetch_add(1, Ordering::Relaxed);
                self.storage_available.store(false, Ordering::Release);
                log::warn!("response cache read failed: {err}");
                None
            }
        }
    }

    pub fn put(&self, request: &CacheRequest<'_>, response: &CacheResponse) {
        if !self.enabled() || !request.cacheable() {
            return;
        }
        if let Err(err) = self.cache.lock().put(request, response) {
            self.write_failures.fetch_add(1, Ordering::Relaxed);
            self.storage_available.store(false, Ordering::Release);
            log::warn!("response cache write failed: {err}");
        }
    }

    pub fn clear(&self) -> rusqlite::Result<usize> {
        self.cache.lock().clear()
    }

    pub fn clear_with_receipt(&self) -> rusqlite::Result<ResponseCacheClearReceipt> {
        self.cache.lock().clear_with_receipt()
    }

    pub fn clear_namespace(&self, namespace: &CacheNamespace) -> rusqlite::Result<usize> {
        self.cache.lock().invalidate_namespace(namespace)
    }

    pub fn stats(&self) -> SemanticCacheStats {
        let namespaces = match self.cache.lock().namespace_stats() {
            Ok(namespaces) => namespaces,
            Err(err) => {
                self.read_failures.fetch_add(1, Ordering::Relaxed);
                self.storage_available.store(false, Ordering::Release);
                log::warn!("response cache stats read failed: {err}");
                Vec::new()
            }
        };
        SemanticCacheStats { namespaces }
    }

    pub fn status(&self) -> SemanticCacheStatus {
        let entries = match self.cache.lock().entry_count() {
            Ok(entries) => entries,
            Err(err) => {
                self.read_failures.fetch_add(1, Ordering::Relaxed);
                self.storage_available.store(false, Ordering::Release);
                log::warn!("response cache status read failed: {err}");
                0
            }
        };
        let storage_available = self.storage_available.load(Ordering::Acquire);
        let semantic_v2_enabled = self.semantic_v2_enabled();
        SemanticCacheStatus {
            enabled: self.enabled(),
            semantic_v2_enabled,
            entries,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            storage_available,
            read_failures: self.read_failures.load(Ordering::Relaxed),
            write_failures: self.write_failures.load(Ordering::Relaxed),
            evidence: if storage_available {
                "local-observed-exact-replay"
            } else {
                "unavailable-storage-error"
            },
            database_path: self.database_path.display().to_string(),
            policy: "exact-v1",
            disclosure: "Local exact replay only; response bodies remain in local app storage until TTL or clear. Streaming, tools/MCP, image content, sensitive markers in requests or responses, high temperature, and no-cache requests bypass. Marker screening is conservative, not a guarantee of secrecy.",
        }
    }

    /// Secret/content-free product diagnostics for the current response cache.
    /// Raw prompts, responses, keys, account names, and workspace paths are
    /// never serialized by this contract.
    pub fn diagnostics(&self) -> ResponseCacheDiagnostics {
        let legacy_status = self.status();
        let namespaces = self
            .stats()
            .namespaces
            .into_iter()
            .map(|namespace| ResponseCacheNamespaceDiagnostics {
                namespace_id: namespace_diagnostic_id(&namespace),
                entries: namespace.entries,
                hits: namespace.hits,
                misses: namespace.misses,
            })
            .collect();
        ResponseCacheDiagnostics {
            product_name: "Exact Response Cache",
            enabled: legacy_status.enabled,
            exact_match_only: true,
            matching_strategy: "sha256-exact-request-identity",
            entries: legacy_status.entries,
            hits: legacy_status.hits,
            misses: legacy_status.misses,
            namespaces,
            storage_path: legacy_status.database_path,
            bypass_rules: vec![
                "streaming",
                "tools_or_mcp",
                "sensitive_data_marker",
                "high_temperature",
                "rapid_repo_state",
                "open_tool_calls",
                "no_cache_marker",
                "unscoped_namespace",
            ],
            safety_rules: vec![
                "local_app_storage_only",
                "sha256_request_key",
                "provider_model_account_workspace_policy_isolation",
                "ttl_and_explicit_invalidation",
                "fail_open_on_storage_error",
                "marker_screening_is_not_a_secrecy_guarantee",
            ],
            clear_action: ResponseCacheClearAction {
                command: "clear_response_cache",
                confirmation_phrase: CLEAR_RESPONSE_CACHE_CONFIRMATION,
                restore_command: None,
                restore_available: false,
                retains_local_restore_copy: false,
            },
        }
    }
}

fn namespace_diagnostic_id(namespace: &SemanticCacheNamespaceStat) -> String {
    let mut hasher = Sha256::new();
    for value in [
        &namespace.provider,
        &namespace.model,
        &namespace.account,
        &namespace.workspace,
        &namespace.policy,
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

#[tauri::command]
pub fn get_semantic_cache_status(
    state: tauri::State<'_, crate::state::AppState>,
) -> SemanticCacheStatus {
    state.semantic_cache.status()
}

const CLEAR_RESPONSE_CACHE_CONFIRMATION: &str = "clear exact response cache";

#[tauri::command]
pub fn get_response_cache_diagnostics(
    state: tauri::State<'_, crate::state::AppState>,
) -> ResponseCacheDiagnostics {
    state.semantic_cache.diagnostics()
}

#[tauri::command]
pub fn clear_response_cache(
    state: tauri::State<'_, crate::state::AppState>,
    confirmation_phrase: String,
) -> Result<ResponseCacheClearResult, String> {
    if confirmation_phrase != CLEAR_RESPONSE_CACHE_CONFIRMATION {
        return Err(format!(
            "Type \"{CLEAR_RESPONSE_CACHE_CONFIRMATION}\" to clear the Exact Response Cache."
        ));
    }
    let receipt = state
        .semantic_cache
        .clear_with_receipt()
        .map_err(|err| err.to_string())?;
    Ok(ResponseCacheClearResult {
        receipt,
        diagnostics: state.semantic_cache.diagnostics(),
    })
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
            "Exact Response Cache requires Full or Headroom mode; Off and RTK-only modes do not serve cached provider responses."
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticCacheNamespaceClearRequest {
    pub provider: String,
    pub model: String,
    pub account: String,
    pub workspace: String,
    pub policy: String,
    pub confirmation_phrase: String,
}

const CLEAR_NAMESPACE_CONFIRMATION: &str = "clear cache namespace";

#[tauri::command]
pub fn get_semantic_cache_stats(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<SemanticCacheStats, String> {
    Ok(state.semantic_cache.stats())
}

#[tauri::command]
pub fn clear_semantic_cache_namespace(
    state: tauri::State<'_, crate::state::AppState>,
    request: SemanticCacheNamespaceClearRequest,
) -> Result<SemanticCacheStats, String> {
    if request.confirmation_phrase != CLEAR_NAMESPACE_CONFIRMATION {
        return Err(format!(
            "Type \"{CLEAR_NAMESPACE_CONFIRMATION}\" to clear this namespace."
        ));
    }
    let namespace = CacheNamespace {
        provider: request.provider,
        model: request.model,
        account: request.account,
        workspace: request.workspace,
        policy: request.policy,
        request_variant: String::new(),
    };
    state
        .semantic_cache
        .clear_namespace(&namespace)
        .map_err(|err| err.to_string())?;
    Ok(state.semantic_cache.stats())
}

#[tauri::command]
pub fn set_semantic_cache_v2_enabled(
    state: tauri::State<'_, crate::state::AppState>,
    enabled: bool,
) -> Result<SemanticCacheStatus, String> {
    if enabled {
        return Err(
            "Legacy normalized-prefix v2 replay is unavailable because it is not semantic similarity. A true semantic-cache experiment uses a separate gated contract.".into(),
        );
    }
    state
        .semantic_cache
        .set_semantic_v2_enabled(enabled)
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
    fn exact_identity_does_not_normalize_case_or_truncate_prompt_suffixes() {
        let mut upper = request();
        upper.prompt = "Hello";
        let mut lower = request();
        lower.prompt = "hello";
        assert_ne!(
            ExactResponseCache::key_hash(&upper),
            ExactResponseCache::key_hash(&lower)
        );

        let prefix = "x".repeat(240);
        let left_prompt = format!("{prefix}left");
        let right_prompt = format!("{prefix}right");
        let left = CacheRequest {
            namespace: request().namespace,
            prompt: &left_prompt,
            ..Default::default()
        };
        let right = CacheRequest {
            namespace: request().namespace,
            prompt: &right_prompt,
            ..Default::default()
        };
        assert_ne!(
            ExactResponseCache::key_hash(&left),
            ExactResponseCache::key_hash(&right)
        );
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
    fn namespace_invalidation_clears_all_variants_but_not_other_scopes() {
        let c = SemanticCache::open_in_memory().unwrap();
        let r = request();
        let mut variant = request();
        variant.namespace.request_variant = "sha256:variant".into();
        let mut other = request();
        other.namespace.workspace = "other-workspace".into();
        assert!(c.put(&r, &response()).unwrap());
        assert!(c.put(&variant, &response()).unwrap());
        assert!(c.put(&other, &response()).unwrap());

        assert_eq!(c.invalidate_namespace(&r.namespace).unwrap(), 2);
        assert!(c.get(&r).unwrap().is_none());
        assert!(c.get(&variant).unwrap().is_none());
        assert!(c.get(&other).unwrap().is_some());
    }

    #[test]
    fn unscoped_namespace_is_fail_closed() {
        let mut r = request();
        r.namespace.account.clear();
        assert!(!r.namespace.is_scoped());
        assert!(!r.cacheable());
        let c = SemanticCache::open_in_memory().unwrap();
        assert!(!c.put(&r, &response()).unwrap());
        assert!(c.get(&r).unwrap().is_none());
    }

    #[test]
    fn response_no_cache_is_never_stored() {
        let c = SemanticCache::open_in_memory().unwrap();
        let mut response = response();
        response.no_cache = true;
        let r = request();
        assert!(!c.put(&r, &response).unwrap());
        assert!(c.get(&r).unwrap().is_none());
        assert_eq!(c.entry_count().unwrap(), 0);
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
        assert!(status.storage_available);
        assert_eq!(status.evidence, "local-observed-exact-replay");
        assert_eq!(status.read_failures, 0);
        assert_eq!(status.write_failures, 0);
        service.clear().unwrap();
        assert_eq!(service.status().entries, 0);
        service.set_enabled(false).unwrap();
        let _ = std::fs::remove_file(state_path);
    }

    #[test]
    fn response_cache_diagnostics_are_content_and_secret_free() {
        let state_path = std::env::temp_dir().join(format!(
            "ai-switchboard-response-cache-diagnostics-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let service = ExactResponseCacheService::open(":memory:", &state_path).unwrap();
        service.set_enabled(true).unwrap();
        let mut request = request();
        request.namespace.account = "secret-account@example.test".into();
        request.namespace.workspace = "/private/customer/repository".into();
        request.prompt = "proprietary prompt content";
        let mut response = response();
        response.body = "private model response".into();
        service.put(&request, &response);
        assert!(service.get(&request).is_some());

        let diagnostics = service.diagnostics();
        let serialized = serde_json::to_string(&diagnostics).unwrap();
        assert_eq!(diagnostics.product_name, "Exact Response Cache");
        assert!(diagnostics.exact_match_only);
        assert_eq!(diagnostics.entries, 1);
        assert_eq!(diagnostics.hits, 1);
        assert_eq!(diagnostics.namespaces.len(), 1);
        assert!(diagnostics.namespaces[0]
            .namespace_id
            .starts_with("sha256:"));
        for forbidden in [
            "secret-account@example.test",
            "/private/customer/repository",
            "proprietary prompt content",
            "private model response",
        ] {
            assert!(!serialized.contains(forbidden));
        }
        assert!(diagnostics.bypass_rules.contains(&"tools_or_mcp"));
        assert_eq!(
            diagnostics.clear_action.confirmation_phrase,
            CLEAR_RESPONSE_CACHE_CONFIRMATION
        );
        let _ = std::fs::remove_file(state_path);
    }

    #[test]
    fn confirmed_clear_deletes_response_bodies_without_changing_enabled_state() {
        let state_path = std::env::temp_dir().join(format!(
            "ai-switchboard-response-cache-clear-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let service = ExactResponseCacheService::open(":memory:", &state_path).unwrap();
        service.set_enabled(true).unwrap();
        service.put(&request(), &response());

        let receipt = service.clear_with_receipt().unwrap();
        assert_eq!(receipt.entries_cleared, 1);
        assert!(!receipt.restore_available);
        assert_eq!(service.diagnostics().entries, 0);
        assert!(!service.diagnostics().clear_action.restore_available);
        assert!(
            !service
                .diagnostics()
                .clear_action
                .retains_local_restore_copy
        );
        assert_eq!(service.diagnostics().clear_action.restore_command, None);
        assert!(service.enabled());
        assert!(service.get(&request()).is_none());
        let _ = std::fs::remove_file(state_path);
    }

    #[test]
    fn legacy_v2_state_is_migrated_off_and_cannot_be_reenabled() {
        let state_path = std::env::temp_dir().join(format!(
            "ai-switchboard-response-cache-legacy-v2-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &state_path,
            r#"{"version":1,"enabled":true,"semanticV2Enabled":true}"#,
        )
        .unwrap();
        let service = ExactResponseCacheService::open(":memory:", &state_path).unwrap();
        assert!(!service.semantic_v2_enabled());
        assert!(service.set_semantic_v2_enabled(true).is_err());
        let diagnostics = service.diagnostics();
        assert!(diagnostics.exact_match_only);
        assert_eq!(
            diagnostics.matching_strategy,
            "sha256-exact-request-identity"
        );
        let migrated: SemanticCacheStateFile =
            serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
        assert!(!migrated.semantic_v2_enabled);
        let _ = std::fs::remove_file(state_path);
    }
}
