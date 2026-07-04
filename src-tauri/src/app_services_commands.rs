use std::process::Command;

use serde_json::Value;
use tauri::AppHandle;

use crate::analytics;
use crate::local_mode;

const HEADROOM_DASHBOARD_URL: &str = "http://127.0.0.1:6767/dashboard";

#[tauri::command]
pub async fn open_headroom_dashboard() -> Result<(), String> {
    open_external_link_impl(HEADROOM_DASHBOARD_URL)
        .map_err(|err| format!("Failed to open Headroom dashboard: {err}"))
}

fn open_external_link_impl(url: &str) -> Result<(), String> {
    let trimmed = validate_external_link_url(url)?;

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(&trimmed);
        command
    };

    #[cfg(target_os = "linux")]
    {
        for opener in ["xdg-open", "gio", "kde-open5", "wslview"] {
            let mut command = Command::new(opener);
            if opener == "gio" {
                command.args(["open", &trimmed]);
            } else {
                command.arg(&trimmed);
            }
            match command.status() {
                Ok(status) if status.success() => return Ok(()),
                Ok(_) => continue,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => {
                    return Err(format!(
                        "Could not launch external link with {opener}: {err}"
                    ))
                }
            }
        }
        return Err(
            "No URL opener found. Install xdg-utils (provides xdg-open) to open links.".into(),
        );
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", trimmed.as_str()]);
        command
    };

    #[cfg(not(target_os = "linux"))]
    {
        let status = command
            .status()
            .map_err(|err| format!("Could not launch external link: {err}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("External link opener exited with {status}."))
        }
    }
}

pub(crate) fn validate_external_link_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("External link is empty.".into());
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("External links cannot contain line breaks.".into());
    }

    if trimmed.starts_with("mailto:") {
        let address = trimmed.trim_start_matches("mailto:");
        if address.is_empty() || address.contains('?') || address.contains('/') {
            return Err("Only simple mailto links are supported.".into());
        }
        return Ok(trimmed.to_string());
    }

    let parsed =
        reqwest::Url::parse(trimmed).map_err(|_| "External link URL is invalid.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only http, https, and mailto links are supported.".into());
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Err("External links cannot include embedded credentials.".into());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "External link must include a host.".to_string())?;
    if is_blocked_external_link_host(host) {
        return Err("External link host is not allowed.".into());
    }

    Ok(trimmed.to_string())
}

fn is_blocked_external_link_host(host: &str) -> bool {
    let normalized = host
        .trim_matches(|ch| ch == '[' || ch == ']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if matches!(normalized.as_str(), "localhost" | "localhost.localdomain") {
        return true;
    }
    if normalized.ends_with(".localhost") || normalized.ends_with(".local") {
        return true;
    }
    match normalized.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified()
        }
        Ok(std::net::IpAddr::V6(ip)) => {
            ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local()
        }
        Err(_) => false,
    }
}

#[tauri::command]
pub async fn open_external_link(url: String) -> Result<(), String> {
    open_external_link_impl(&url)
}

#[tauri::command]
pub fn track_analytics_event(app: AppHandle, name: String, properties: Option<Value>) {
    analytics::track_event(&app, &name, properties);
}

#[tauri::command]
pub async fn submit_contact_request(
    url: String,
    email: String,
    message: Option<String>,
) -> Result<(), String> {
    reject_contact_request_in_local_only()?;
    let trimmed = email.trim();
    if trimmed.is_empty() || !trimmed.contains('@') {
        return Err("Enter a valid email address.".to_string());
    }

    let target = validate_contact_request_url(&url)
        .ok_or_else(|| "Could not reach the contact form.".to_string())?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| err.to_string())?;
    let message_owned = message
        .map(|m| m.trim().chars().take(2000).collect::<String>())
        .unwrap_or_default();
    let response = client
        .post(target)
        .form(&[
            ("contact_request[email]", trimmed),
            ("contact_request[message]", message_owned.as_str()),
        ])
        .send()
        .await
        .map_err(|err| err.to_string())?;

    // Rails answers a successful POST with a 302 to /#pricing. Redirect policy
    // is none for SSRF defense, so accept 3xx as success here. 422 and 503 are
    // the controller's explicit error renders.
    match response.status().as_u16() {
        200..=399 => Ok(()),
        422 => Err("Enter a valid email address.".to_string()),
        503 => Err("Email delivery still needs to be configured.".to_string()),
        status => Err(format!("Contact request failed with status {status}.")),
    }
}

pub(crate) fn reject_contact_request_in_local_only() -> Result<(), String> {
    if local_mode::enabled() {
        Err("Support/contact requests are disabled in local-only mode.".to_string())
    } else {
        Ok(())
    }
}

// Scheme + host allowlist for the contact form endpoint. The URL reaches this
// Tauri command from the webview, so we must not assume it is trustworthy. An
// SSRF primitive here would let a compromised frame POST to arbitrary hosts,
// including loopback services.
pub(crate) fn validate_contact_request_url(raw: &str) -> Option<reqwest::Url> {
    const ALLOWED_HOSTS: &[&str] = &["github.com"];
    if raw.contains('\n') || raw.contains('\r') {
        return None;
    }
    let parsed = reqwest::Url::parse(raw).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return None;
    }
    let host = parsed.host_str()?;
    if !ALLOWED_HOSTS.contains(&host) {
        return None;
    }
    Some(parsed)
}
