//! Credential-safe, opt-in readiness checks for manually managed gateways.
//!
//! This module intentionally never persists, returns, or logs environment
//! values. Connectivity is limited to an explicitly requested localhost TCP
//! probe; remote gateway URLs are never contacted by Switchboard.

use serde::Serialize;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayReadinessReport {
    pub profile_id: String,
    pub configuration: Vec<GatewayReadinessItem>,
    pub credentials: Vec<GatewayReadinessItem>,
    pub connectivity: GatewayConnectivityResult,
    pub live: bool,
    pub guidance: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayReadinessItem {
    pub label: String,
    pub environment_variable: String,
    pub present: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConnectivityResult {
    pub attempted: bool,
    pub status: String,
    pub detail: String,
}

fn env_presence(names: &[(&str, &str)]) -> Vec<GatewayReadinessItem> {
    names
        .iter()
        .map(|(label, name)| GatewayReadinessItem {
            label: (*label).to_owned(),
            environment_variable: (*name).to_owned(),
            present: std::env::var_os(name).is_some_and(|value| !value.is_empty()),
        })
        .collect()
}

fn profile_contract(
    profile_id: &str,
) -> Option<(
    &'static [(&'static str, &'static str)],
    &'static [(&'static str, &'static str)],
)> {
    match profile_id {
        "litellm-local-cache" => Some((
            &[("Local proxy URL", "LITELLM_BASE_URL")],
            &[("LiteLLM API key", "LITELLM_API_KEY")],
        )),
        "langfuse-export" => Some((
            &[("Self-hosted endpoint", "LANGFUSE_HOST")],
            &[
                ("Public key", "LANGFUSE_PUBLIC_KEY"),
                ("Secret key", "LANGFUSE_SECRET_KEY"),
            ],
        )),
        "cloudflare-ai-gateway" => Some((
            &[("Gateway URL", "CLOUDFLARE_AI_GATEWAY_BASE_URL")],
            &[("Gateway token", "CLOUDFLARE_AI_GATEWAY_TOKEN")],
        )),
        "kong-enterprise-gateway" => Some((
            &[("Gateway URL", "KONG_GATEWAY_BASE_URL")],
            &[("Gateway credential", "KONG_GATEWAY_TOKEN")],
        )),
        _ => None,
    }
}

fn localhost_socket_from_env() -> Result<SocketAddr, String> {
    let raw = std::env::var("LITELLM_BASE_URL")
        .map_err(|_| "LITELLM_BASE_URL is not present.".to_string())?;
    let authority = raw
        .trim()
        .strip_prefix("http://")
        .or_else(|| raw.trim().strip_prefix("https://"))
        .unwrap_or(raw.trim())
        .split('/')
        .next()
        .unwrap_or_default();
    let addr: SocketAddr = authority
        .parse()
        .or_else(|_| format!("{authority}:80").parse())
        .map_err(|_| "LITELLM_BASE_URL is not a localhost host:port value.".to_string())?;
    if !addr.ip().is_loopback() {
        return Err(
            "Only loopback endpoints can be preflighted. Remote gateways are never contacted."
                .to_string(),
        );
    }
    Ok(addr)
}

/// Environment variables are inspected only for presence. `run_local_connectivity`
/// defaults to false and can only probe LiteLLM's loopback address.
#[tauri::command]
pub fn get_gateway_readiness(
    profile_id: String,
    run_local_connectivity: Option<bool>,
) -> Result<GatewayReadinessReport, String> {
    let (configuration_contract, credential_contract) =
        profile_contract(&profile_id).ok_or_else(|| "Unknown gateway profile.".to_string())?;
    let should_probe =
        run_local_connectivity.unwrap_or(false) && profile_id == "litellm-local-cache";
    let connectivity = if should_probe {
        match localhost_socket_from_env() {
            Ok(address) => match TcpStream::connect_timeout(&address, Duration::from_millis(800)) {
                Ok(_) => GatewayConnectivityResult { attempted: true, status: "reachable".into(), detail: "Loopback TCP endpoint accepted a connection. This does not prove cache or provider health.".into() },
                Err(_) => GatewayConnectivityResult { attempted: true, status: "unreachable".into(), detail: "Loopback TCP endpoint did not accept a connection. No request content was sent.".into() },
            },
            Err(detail) => GatewayConnectivityResult { attempted: true, status: "not-eligible".into(), detail },
        }
    } else {
        GatewayConnectivityResult { attempted: false, status: "not-run".into(), detail: "No connectivity preflight was run. Remote profiles are never contacted; local probing requires an explicit action.".into() }
    };
    Ok(GatewayReadinessReport {
        profile_id,
        configuration: env_presence(configuration_contract),
        credentials: env_presence(credential_contract),
        connectivity,
        live: false,
        guidance: "Environment presence is redacted and advisory only. It does not prove authentication, routing, ownership, cache hits, trace delivery, or live service health.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_never_claims_a_profile_is_live_or_contacts_by_default() {
        let report = get_gateway_readiness("cloudflare-ai-gateway".into(), None).unwrap();
        assert!(!report.live);
        assert!(!report.connectivity.attempted);
        assert_eq!(report.connectivity.status, "not-run");
        assert!(report
            .credentials
            .iter()
            .all(|item| !item.environment_variable.is_empty()));
    }

    #[test]
    fn remote_profiles_cannot_enable_connectivity_probing() {
        let report = get_gateway_readiness("langfuse-export".into(), Some(true)).unwrap();
        assert!(!report.connectivity.attempted);
    }
}
