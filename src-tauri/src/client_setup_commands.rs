use serde_json::json;
use tauri::{AppHandle, Manager, State};

use crate::analytics;
use crate::client_adapters;
use crate::models::{ClientConnectorStatus, ClientSetupResult, ClientSetupVerification};
use crate::state::AppState;

#[tauri::command]
pub async fn apply_client_setup(
    app: AppHandle,
    client_id: String,
) -> Result<ClientSetupResult, String> {
    // Two recovery paths land on the tray-banner "Re-enable" button:
    //   1. Watchdog give-up - pauses the runtime and clears client setups.
    //   2. Pricing gate (grace expiry, weekly cap) - sets `proxy_bypass` and
    //      calls `stop_headroom()` without flipping `runtime_paused`.
    // Both leave Python stopped, so re-enable has to clear bypass and bring
    // the runtime back. Without this, env vars get rewritten but the proxy
    // stays down and Claude Code traffic flows unoptimized until the next
    // pricing poll (or, in the watchdog case, until restart).
    let state: tauri::State<'_, AppState> = app.state();
    let bypassed = state
        .proxy_bypass
        .load(std::sync::atomic::Ordering::Acquire);
    if state.runtime_is_paused() || bypassed {
        if let Err(err) = state.resume_runtime() {
            log::warn!("apply_client_setup: resume_runtime failed: {err:#}");
        }
    }
    match client_adapters::apply_client_setup(&client_id) {
        Ok(result) => {
            analytics::track_event(
                &app,
                "client_setup_applied",
                Some(json!({
                    "client_id": result.client_id.clone(),
                    "already_configured": result.already_configured,
                    "verified": result.verification.verified,
                    "proxy_reachable": result.verification.proxy_reachable
                })),
            );
            // Setup returned Ok, but the post-write verification read the
            // files back and found the expected side effect missing. That's
            // the same class of bug as the MCP fallback silent-success -
            // subprocess/file-write succeeded yet the integration is not
            // actually in place. Capture to Sentry so we see it.
            if !result.verification.verified {
                sentry::with_scope(
                    |scope| {
                        scope.set_extra(
                            "proxy_reachable",
                            result.verification.proxy_reachable.into(),
                        );
                        scope.set_extra("checks", json!(result.verification.checks).into());
                        scope.set_extra("failures", json!(result.verification.failures).into());
                        scope.set_extra("already_configured", result.already_configured.into());
                    },
                    || {
                        sentry::capture_message(
                            &format!(
                                "client setup for {client_id} completed but verification failed",
                            ),
                            sentry::Level::Warning,
                        );
                    },
                );
            }
            Ok(result)
        }
        Err(err) => {
            let msg = err.to_string();
            if !msg.starts_with("Automatic setup is not supported yet") {
                sentry::capture_message(
                    &format!("client setup failed for {client_id}: {err:#}"),
                    sentry::Level::Error,
                );
            }
            Err(msg)
        }
    }
}

#[tauri::command]
pub async fn verify_client_setup(client_id: String) -> Result<ClientSetupVerification, String> {
    client_adapters::verify_client_setup(&client_id).map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_client_connectors(
    state: State<'_, AppState>,
) -> Result<Vec<ClientConnectorStatus>, String> {
    client_adapters::list_client_connectors(&state.cached_clients()).map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn disable_client_setup(app: AppHandle, client_id: String) -> Result<(), String> {
    client_adapters::disable_client_setup(&client_id).map_err(|err| err.to_string())?;
    analytics::track_event(
        &app,
        "client_setup_disabled",
        Some(json!({ "client_id": client_id })),
    );
    Ok(())
}

#[tauri::command]
pub async fn clear_client_setups() -> Result<(), String> {
    client_adapters::clear_client_setups().map_err(|err| err.to_string())
}
