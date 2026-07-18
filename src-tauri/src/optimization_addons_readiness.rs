//! Redacted, advisory readiness checks for optional optimization sidecars.
//!
//! This module only inspects environment variables. It never launches an
//! executable or sends application data. An explicit preflight may open a TCP
//! connection only to loopback addresses for the two URL-bearing profiles.

use serde::Serialize;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationAddonReadinessReport {
    pub profile_id: String,
    pub configuration: Vec<OptimizationAddonReadinessItem>,
    pub executable_present: bool,
    pub path_present: bool,
    pub connectivity: OptimizationAddonConnectivityResult,
    pub live: bool,
    pub guidance: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationAddonReadinessItem {
    pub label: String,
    pub environment_variable: String,
    pub present: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationAddonConnectivityResult {
    pub attempted: bool,
    pub status: String,
    pub detail: String,
}

struct ProfileContract {
    configuration: &'static [(&'static str, &'static str)],
    executable: &'static str,
    path: &'static str,
    base_url: Option<&'static str>,
}

fn profile_contract(profile_id: &str) -> Option<ProfileContract> {
    let contract = match profile_id {
        "leanctx" => ProfileContract {
            configuration: &[("Base URL", "LEANCTX_BASE_URL")],
            executable: "LEANCTX_EXECUTABLE",
            path: "LEANCTX_PATH",
            base_url: Some("LEANCTX_BASE_URL"),
        },
        "llmlingua-2" => ProfileContract {
            configuration: &[],
            executable: "LLMLINGUA_2_EXECUTABLE",
            path: "LLMLINGUA_2_PATH",
            base_url: None,
        },
        "chonkify" => ProfileContract {
            configuration: &[],
            executable: "CHONKIFY_EXECUTABLE",
            path: "CHONKIFY_PATH",
            base_url: None,
        },
        "semantic-cache" => ProfileContract {
            configuration: &[],
            executable: "",
            path: "",
            base_url: None,
        },
        "pxpipe-text-image" => ProfileContract {
            configuration: &[],
            executable: "PXPIPE_TEXT_IMAGE_EXECUTABLE",
            path: "PXPIPE_TEXT_IMAGE_PATH",
            base_url: None,
        },
        _ => return None,
    };
    Some(contract)
}

fn env_presence(names: &[(&str, &str)]) -> Vec<OptimizationAddonReadinessItem> {
    names
        .iter()
        .map(|(label, name)| OptimizationAddonReadinessItem {
            label: (*label).into(),
            environment_variable: (*name).into(),
            present: std::env::var_os(name).is_some_and(|value| !value.is_empty()),
        })
        .collect()
}

fn loopback_socket(name: &str) -> Result<SocketAddr, String> {
    let raw = std::env::var(name).map_err(|_| format!("{name} is not present."))?;
    let authority = raw
        .trim()
        .strip_prefix("http://")
        .or_else(|| raw.trim().strip_prefix("https://"))
        .unwrap_or(raw.trim())
        .split('/')
        .next()
        .unwrap_or_default();
    let address: SocketAddr = authority
        .parse()
        .or_else(|_| format!("{authority}:80").parse())
        .map_err(|_| format!("{name} is not a loopback host:port value."))?;
    if !address.ip().is_loopback() {
        return Err("Only 127.0.0.1 and ::1 endpoints can be preflighted; remote endpoints are never contacted.".into());
    }
    Ok(address)
}

/// Returns advisory, redacted readiness. Connectivity is opt-in and loopback-only.
#[tauri::command]
pub fn get_optimization_addon_readiness(
    profile_id: String,
    run_local_connectivity: Option<bool>,
) -> Result<OptimizationAddonReadinessReport, String> {
    let contract = profile_contract(&profile_id)
        .ok_or_else(|| "Unknown optimization addon profile.".to_string())?;
    let is_semantic_cache = profile_id == "semantic-cache";
    let connectivity = if run_local_connectivity.unwrap_or(false) && contract.base_url.is_some() {
        match loopback_socket(contract.base_url.unwrap()) {
            Ok(address) => match TcpStream::connect_timeout(&address, Duration::from_millis(800)) {
                Ok(_) => OptimizationAddonConnectivityResult { attempted: true, status: "reachable".into(), detail: "Loopback TCP endpoint accepted a connection; this does not prove sidecar health.".into() },
                Err(_) => OptimizationAddonConnectivityResult { attempted: true, status: "unreachable".into(), detail: "Loopback TCP endpoint did not accept a connection. No request content was sent.".into() },
            },
            Err(detail) => OptimizationAddonConnectivityResult { attempted: true, status: "not-eligible".into(), detail },
        }
    } else {
        OptimizationAddonConnectivityResult {
            attempted: false,
            status: "not-run".into(),
            detail: "No connectivity preflight was run; remote endpoints are never contacted."
                .into(),
        }
    };
    Ok(OptimizationAddonReadinessReport {
        profile_id,
        configuration: env_presence(contract.configuration),
        executable_present: !contract.executable.is_empty()
            && std::env::var_os(contract.executable).is_some_and(|value| !value.is_empty()),
        path_present: !contract.path.is_empty()
            && std::env::var_os(contract.path).is_some_and(|value| !value.is_empty()),
        connectivity,
        live: false,
        guidance: if is_semantic_cache {
            "Built-in local cache readiness is exposed separately in Addons; this command performs no external connectivity check.".into()
        } else {
            "Advisory presence only. It does not prove installation, authentication, routing, or sidecar health.".into()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_profile_errors() {
        assert!(get_optimization_addon_readiness("unknown".into(), None).is_err());
    }
    #[test]
    fn reports_are_redacted_and_advisory() {
        let report = get_optimization_addon_readiness("pxpipe-text-image".into(), None).unwrap();
        assert!(!report.live);
        assert!(!report.connectivity.attempted);
    }
    #[test]
    fn remote_url_is_not_eligible() {
        std::env::set_var("LEANCTX_BASE_URL", "https://example.com:443");
        let report = get_optimization_addon_readiness("leanctx".into(), Some(true)).unwrap();
        assert_eq!(report.connectivity.status, "not-eligible");
        std::env::remove_var("LEANCTX_BASE_URL");
    }
}
