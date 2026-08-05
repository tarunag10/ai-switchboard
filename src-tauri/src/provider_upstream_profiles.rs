use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::storage;

pub const PROVIDER_UPSTREAM_PROFILE_FILE: &str = "provider-upstream-profiles.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpstreamOverride {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpstreamProfilesState {
    pub version: u8,
    #[serde(default)]
    pub openai: ProviderUpstreamOverride,
    #[serde(default)]
    pub anthropic: ProviderUpstreamOverride,
}

impl ProviderUpstreamProfilesState {
    pub fn new() -> Self {
        Self {
            version: 1,
            openai: ProviderUpstreamOverride::default(),
            anthropic: ProviderUpstreamOverride::default(),
        }
    }
}

pub fn provider_upstream_profiles_path() -> PathBuf {
    storage::app_data_dir()
        .join("config")
        .join(PROVIDER_UPSTREAM_PROFILE_FILE)
}

pub fn load_provider_upstream_profiles() -> ProviderUpstreamProfilesState {
    let path = provider_upstream_profiles_path();
    if !path.exists() {
        return ProviderUpstreamProfilesState::new();
    }
    match fs::read_to_string(&path) {
        Ok(body) => serde_json::from_str(&body).unwrap_or_else(|_| ProviderUpstreamProfilesState::new()),
        Err(err) => {
            log::warn!("load_provider_upstream_profiles: {err:#}");
            ProviderUpstreamProfilesState::new()
        }
    }
}

pub fn save_provider_upstream_profiles(state: &ProviderUpstreamProfilesState) -> Result<()> {
    validate_provider_upstream_profiles(state)?;
    let path = provider_upstream_profiles_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let body = serde_json::to_vec_pretty(state)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("renaming {}", path.display()))?;
    Ok(())
}

pub fn clear_provider_upstream_profiles() -> Result<()> {
    let path = provider_upstream_profiles_path();
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

pub fn validate_upstream_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("URL is required when an upstream override is enabled.");
    }
    let parsed = Url::parse(trimmed).context("URL must be valid")?;
    let scheme = parsed.scheme();
    let host = parsed.host_str().unwrap_or_default();
    let loopback = matches!(host, "127.0.0.1" | "localhost" | "::1");
    if scheme != "https" && !(scheme == "http" && loopback) {
        bail!("Only HTTPS upstream URLs are allowed, except loopback HTTP for local testing.");
    }
    Ok(trimmed.to_string())
}

pub fn validate_provider_upstream_profiles(state: &ProviderUpstreamProfilesState) -> Result<()> {
    if state.openai.enabled {
        validate_upstream_url(&state.openai.url)?;
    }
    if state.anthropic.enabled {
        validate_upstream_url(&state.anthropic.url)?;
    }
    Ok(())
}

pub fn apply_provider_upstream_env(command: &mut Command, state: &ProviderUpstreamProfilesState) {
    if state.openai.enabled {
        if let Ok(url) = validate_upstream_url(&state.openai.url) {
            command.env("OPENAI_TARGET_API_URL", url);
        }
    }
    if state.anthropic.enabled {
        if let Ok(url) = validate_upstream_url(&state.anthropic.url) {
            command.env("ANTHROPIC_TARGET_API_URL", url);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpstreamTestResult {
    pub provider: String,
    pub ok: bool,
    pub status_code: Option<u16>,
    pub detail: String,
}

pub fn test_provider_upstream_url(provider: &str, raw_url: &str) -> ProviderUpstreamTestResult {
    let url = match validate_upstream_url(raw_url) {
        Ok(url) => url,
        Err(err) => {
            return ProviderUpstreamTestResult {
                provider: provider.to_string(),
                ok: false,
                status_code: None,
                detail: err.to_string(),
            };
        }
    };
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return ProviderUpstreamTestResult {
                provider: provider.to_string(),
                ok: false,
                status_code: None,
                detail: format!("HTTP client unavailable: {err}"),
            };
        }
    };
    let response = client.get(&url).send();
    match response {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ok = status < 500;
            ProviderUpstreamTestResult {
                provider: provider.to_string(),
                ok,
                status_code: Some(status),
                detail: if ok {
                    "Upstream endpoint responded.".to_string()
                } else {
                    format!("Upstream endpoint returned HTTP {status}.")
                },
            }
        }
        Err(err) => ProviderUpstreamTestResult {
            provider: provider.to_string(),
            ok: false,
            status_code: None,
            detail: format!("Connection failed: {err}"),
        },
    }
}

pub fn doctor_upstream_issue(state: &ProviderUpstreamProfilesState) -> Option<(String, String)> {
    for (provider, override_state) in [
        ("openai", &state.openai),
        ("anthropic", &state.anthropic),
    ] {
        if !override_state.enabled {
            continue;
        }
        if let Err(err) = validate_upstream_url(&override_state.url) {
            return Some((
                format!("provider_upstream_{provider}_invalid"),
                format!(
                    "{provider} upstream override is enabled but invalid: {err}. Fix the URL in Settings before routing production traffic."
                ),
            ));
        }
    }
    None
}

pub fn doctor_byok_openai_compatible_issue(
    upstream: &ProviderUpstreamProfilesState,
    proxy_reachable: bool,
    enabled_managed_client_count: usize,
) -> Option<(String, String, String)> {
    if !upstream.openai.enabled {
        return None;
    }
    if !proxy_reachable {
        return Some((
            "byok_openai_proxy_unreachable".to_string(),
            "BYOK OpenAI-compatible routing needs a healthy loopback proxy".to_string(),
            "OpenAI-compatible upstream override is enabled, but the Headroom proxy on 127.0.0.1:6767 is not reachable. Start or repair Headroom before pointing clients at the local proxy.".to_string(),
        ));
    }
    if enabled_managed_client_count == 0 {
        return Some((
            "byok_openai_clients_not_routed".to_string(),
            "BYOK OpenAI-compatible routing has no managed clients on 6767".to_string(),
            "OpenAI-compatible upstream override is enabled, but no managed client is connected through the 127.0.0.1:6767 proxy. Repair or enable at least one managed client so traffic reaches Headroom before the upstream override.".to_string(),
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_https_remote_urls() {
        assert!(validate_upstream_url("http://api.example.com/v1").is_err());
        assert!(validate_upstream_url("https://api.example.com/v1").is_ok());
        assert!(validate_upstream_url("http://127.0.0.1:8080/v1").is_ok());
    }

    #[test]
    fn apply_env_only_for_enabled_valid_overrides() {
        let mut state = ProviderUpstreamProfilesState::new();
        state.openai.enabled = true;
        state.openai.url = "https://api.deepseek.com/v1".into();
        let mut command = Command::new("/bin/echo");
        apply_provider_upstream_env(&mut command, &state);
        assert!(validate_provider_upstream_profiles(&state).is_ok());
    }
}
