use serde_json::Value;
use tauri::AppHandle;

use crate::analytics;
use crate::external_open;
use crate::local_mode;

const HEADROOM_DASHBOARD_URL: &str = "http://127.0.0.1:6767/dashboard";

#[tauri::command]
pub async fn open_headroom_dashboard() -> Result<(), String> {
    external_open::open_external_link(HEADROOM_DASHBOARD_URL)
        .map_err(|err| format!("Failed to open Headroom dashboard: {err}"))
}

#[tauri::command]
pub async fn open_external_link(url: String) -> Result<(), String> {
    external_open::open_external_link(&url)
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
