use serde_json::json;
use tauri::{AppHandle, State};

use crate::analytics;
use crate::models::{
    BillingPeriod, HeadroomAuthCodeRequest, HeadroomPricingStatus, HeadroomSubscriptionTier,
};
use crate::pricing;
use crate::state::AppState;

#[tauri::command]
pub async fn get_headroom_pricing_status(
    state: State<'_, AppState>,
) -> Result<HeadroomPricingStatus, String> {
    let status = pricing::get_pricing_status(&state)?;
    // Reconcile the runtime with the freshly evaluated status. Bridges the
    // gap between "user just upgraded" (subscription_active flips on) and
    // "Headroom optimization actually resumes" - without this, the pricing
    // gate's bypass flag would stay set and Python would stay down until
    // the next app launch.
    state.apply_pricing_gate_status(&status);
    state.apply_codex_pricing_gate_status(status.codex.as_ref());
    Ok(status)
}

#[tauri::command]
pub async fn request_headroom_auth_code(
    app: AppHandle,
    state: State<'_, AppState>,
    email: String,
) -> Result<HeadroomAuthCodeRequest, String> {
    let request = pricing::request_auth_code(&state, &email)?;
    analytics::track_event(&app, "auth_code_requested", None);
    Ok(request)
}

#[tauri::command]
pub async fn verify_headroom_auth_code(
    app: AppHandle,
    state: State<'_, AppState>,
    email: String,
    code: String,
    invite_code: Option<String>,
) -> Result<HeadroomPricingStatus, String> {
    let used_invite_code = invite_code
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    let status = pricing::verify_auth_code(&state, &email, &code, invite_code.as_deref())?;
    // Reconcile the runtime with the freshly evaluated status. Mirrors
    // `get_headroom_pricing_status` so a user who signs up after grace
    // expiry doesn't have to wait for the next 60s pricing poll for
    // Python to come back online.
    state.apply_pricing_gate_status(&status);
    state.apply_codex_pricing_gate_status(status.codex.as_ref());
    analytics::track_event(
        &app,
        "auth_verified",
        Some(json!({ "invite_code_used": used_invite_code })),
    );
    Ok(status)
}

#[tauri::command]
pub async fn sign_out_headroom_account() -> Result<(), String> {
    pricing::sign_out()
}

#[tauri::command]
pub async fn activate_headroom_account(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<HeadroomPricingStatus, String> {
    let lifetime_tokens_saved = state.dashboard().lifetime_estimated_tokens_saved;
    let status = pricing::activate_account(&state, lifetime_tokens_saved)?;
    analytics::track_event(&app, "account_activated", None);
    Ok(status)
}

#[tauri::command]
pub async fn create_headroom_checkout_session(
    app: AppHandle,
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
) -> Result<String, String> {
    let url = pricing::create_checkout_session(subscription_tier.clone(), billing_period)?;
    analytics::track_event(
        &app,
        "checkout_started",
        Some(json!({
            "subscription_tier": subscription_tier_label(&subscription_tier)
        })),
    );
    Ok(url)
}

#[tauri::command]
pub async fn change_headroom_subscription_plan(
    app: AppHandle,
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
) -> Result<(), String> {
    pricing::change_subscription_plan(subscription_tier.clone(), billing_period)?;
    analytics::track_event(
        &app,
        "subscription_plan_changed",
        Some(json!({
            "subscription_tier": subscription_tier_label(&subscription_tier)
        })),
    );
    Ok(())
}

#[tauri::command]
pub async fn reactivate_headroom_subscription(app: AppHandle) -> Result<(), String> {
    pricing::reactivate_subscription()?;
    analytics::track_event(&app, "subscription_reactivated", None);
    Ok(())
}

#[tauri::command]
pub async fn get_headroom_billing_portal_url(target: Option<String>) -> Result<String, String> {
    pricing::get_billing_portal_url(target)
}

fn subscription_tier_label(tier: &HeadroomSubscriptionTier) -> &'static str {
    match tier {
        HeadroomSubscriptionTier::Pro => "pro",
        HeadroomSubscriptionTier::Max5x => "max5x",
        HeadroomSubscriptionTier::Max20x => "max20x",
    }
}
