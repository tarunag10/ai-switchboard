use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROXY_SESSION_HEADER: &str = "x-switchboard-proxy-session";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySessionAuthConfig {
    pub enforce: bool,
}

impl Default for ProxySessionAuthConfig {
    fn default() -> Self {
        Self { enforce: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySessionAuthStatus {
    pub available: bool,
    pub enforce: bool,
    pub fingerprint: String,
    pub status: String,
    pub detail: String,
    pub validated_request_count: u64,
    pub rejected_request_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySessionValidation {
    Valid,
    Missing,
    Invalid,
}

pub struct ProxySessionAuth {
    token: String,
    enforce: AtomicBool,
    validated_requests: AtomicU64,
    rejected_requests: AtomicU64,
    config_path: PathBuf,
}

impl ProxySessionAuth {
    pub fn open(base_dir: &Path) -> Arc<Self> {
        let config_path = base_dir.join("config").join("proxy-session-auth.json");
        let config = load_config(&config_path);
        let token = Uuid::new_v4().to_string();
        Arc::new(Self {
            token,
            enforce: AtomicBool::new(config.enforce),
            validated_requests: AtomicU64::new(0),
            rejected_requests: AtomicU64::new(0),
            config_path,
        })
    }

    pub fn fingerprint(&self) -> String {
        let bytes = self.token.as_bytes();
        if bytes.len() < 8 {
            return "redacted".to_string();
        }
        format!("{}…", &self.token[..8])
    }

    pub fn enforce(&self) -> bool {
        self.enforce.load(Ordering::Acquire)
    }

    pub fn set_enforce(&self, enforce: bool) -> Result<(), String> {
        self.enforce.store(enforce, Ordering::Release);
        save_config(&self.config_path, &ProxySessionAuthConfig { enforce })
    }

    pub fn validate_request_headers(&self, buf: &[u8]) -> ProxySessionValidation {
        match extract_header_value(buf, PROXY_SESSION_HEADER) {
            None => ProxySessionValidation::Missing,
            Some(value) if constant_time_eq(value.trim(), &self.token) => {
                self.validated_requests.fetch_add(1, Ordering::Relaxed);
                ProxySessionValidation::Valid
            }
            Some(_) => {
                self.rejected_requests.fetch_add(1, Ordering::Relaxed);
                ProxySessionValidation::Invalid
            }
        }
    }

    pub fn status(&self) -> ProxySessionAuthStatus {
        let enforce = self.enforce();
        let validated = self.validated_requests.load(Ordering::Relaxed);
        let rejected = self.rejected_requests.load(Ordering::Relaxed);
        let (status, detail) = if enforce {
            if validated > 0 {
                (
                    "authenticated".to_string(),
                    format!(
                        "Per-session proxy token is enforced on 127.0.0.1:6767. Clients must send `{PROXY_SESSION_HEADER}`. Validated {validated} request(s); rejected {rejected}."
                    ),
                )
            } else {
                (
                    "session_token_enforced".to_string(),
                    format!(
                        "Per-session proxy token is enforced, but no validated requests have arrived yet. Send `{PROXY_SESSION_HEADER}` from managed shims or compatible clients."
                    ),
                )
            }
        } else {
            (
                "session_token_available".to_string(),
                format!(
                    "Loopback/Origin checks remain active. A per-session proxy token is available for optional client shims via `{PROXY_SESSION_HEADER}` (fingerprint {}). Enforcement is off by default for managed-client compatibility.",
                    self.fingerprint()
                ),
            )
        };
        ProxySessionAuthStatus {
            available: true,
            enforce,
            fingerprint: self.fingerprint(),
            status,
            detail,
            validated_request_count: validated,
            rejected_request_count: rejected,
        }
    }
}

impl std::fmt::Debug for ProxySessionAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxySessionAuth")
            .field("fingerprint", &self.fingerprint())
            .field("enforce", &self.enforce())
            .finish()
    }
}

fn load_config(path: &Path) -> ProxySessionAuthConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_config(path: &Path, config: &ProxySessionAuthConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let payload = serde_json::to_string_pretty(config).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

fn extract_header_value(buf: &[u8], header_name: &str) -> Option<String> {
    let header_end = find_header_end(buf)?;
    let headers = std::str::from_utf8(&buf[..header_end]).ok()?;
    let needle = header_name.to_ascii_lowercase();
    for line in headers.lines().skip(1) {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case(&needle) {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|window| window == b"\r\n\r\n")
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[tauri::command]
pub fn get_proxy_session_auth_status(
    state: tauri::State<'_, crate::state::AppState>,
) -> ProxySessionAuthStatus {
    state.proxy_session_auth.status()
}

#[tauri::command]
pub fn set_proxy_session_auth_enforce(
    state: tauri::State<'_, crate::state::AppState>,
    enforce: bool,
) -> Result<ProxySessionAuthStatus, String> {
    state.proxy_session_auth.set_enforce(enforce)?;
    Ok(state.proxy_session_auth.status())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request(header_line: &str) -> Vec<u8> {
        format!("POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1:6767\r\n{header_line}\r\n\r\n")
            .into_bytes()
    }

    #[test]
    fn validates_matching_session_header() {
        let auth = ProxySessionAuth::open(std::env::temp_dir().as_path());
        let token = auth.token.clone();
        let req = sample_request(&format!("{PROXY_SESSION_HEADER}: {token}"));
        assert_eq!(
            auth.validate_request_headers(&req),
            ProxySessionValidation::Valid
        );
    }

    #[test]
    fn rejects_missing_and_invalid_headers() {
        let auth = ProxySessionAuth::open(std::env::temp_dir().as_path());
        let missing = sample_request("Content-Type: application/json");
        assert_eq!(
            auth.validate_request_headers(&missing),
            ProxySessionValidation::Missing
        );
        let invalid = sample_request(&format!("{PROXY_SESSION_HEADER}: not-the-token"));
        assert_eq!(
            auth.validate_request_headers(&invalid),
            ProxySessionValidation::Invalid
        );
    }

    #[test]
    fn debug_never_leaks_token() {
        let auth = ProxySessionAuth::open(std::env::temp_dir().as_path());
        let rendered = format!("{auth:?}");
        assert!(!rendered.contains(&auth.token));
        assert!(rendered.contains("fingerprint"));
    }

    #[test]
    fn enforce_status_labels_progress() {
        let auth = ProxySessionAuth::open(std::env::temp_dir().as_path());
        auth.set_enforce(true).expect("set enforce");
        assert_eq!(auth.status().status, "session_token_enforced");
        let token = auth.token.clone();
        let req = sample_request(&format!("{PROXY_SESSION_HEADER}: {token}"));
        assert_eq!(
            auth.validate_request_headers(&req),
            ProxySessionValidation::Valid
        );
        assert_eq!(auth.status().status, "authenticated");
    }
}
