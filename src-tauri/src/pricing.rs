use std::path::PathBuf;

use chrono::{DateTime, Duration, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::device;
use crate::keychain;
use crate::models::{
    headroom_tier_for_claude_plan, BillingPeriod, ClaudeAccountProfile, ClaudeAuthMethod,
    ClaudePlanTier, ClaudeUsage, ClaudeUsageWindow, HeadroomAccountProfile, HeadroomAuthCodeRequest,
    HeadroomPricingStatus, HeadroomSubscriptionTier, PricingGateReason, TierMismatch,
};
use crate::state::AppState;
use crate::storage::{app_data_dir, config_file};

const HEADROOM_ACCOUNT_KEYCHAIN_SERVICE: &str = "com.extraheadroom.headroom.account";
const HEADROOM_ACCOUNT_SESSION_ACCOUNT: &str = "session-token";
#[cfg(debug_assertions)]
const DEFAULT_ACCOUNT_API_BASE_URL: &str = "http://127.0.0.1:3000/api/v1";
#[cfg(not(debug_assertions))]
const DEFAULT_ACCOUNT_API_BASE_URL: &str = "https://extraheadroom.com/api/v1";
const LOCAL_GRACE_PERIOD_HOURS: i64 = 72;
const TIER_MISMATCH_GRACE_DAYS: i64 = 14;
// Set to true in dev builds to skip sign-in requirement (indefinite trial)
#[cfg(debug_assertions)]
const INDEFINITE_TRIAL: bool = true;
const AUTH_CODE_EXPIRY_SECONDS: u64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalPricingState {
    first_seen_at: DateTime<Utc>,
    #[serde(default)]
    reconcile_with_server: bool,
    #[serde(default)]
    mismatch_since: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityPayload {
    device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    chopratejas_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    claude_account_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    claude_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    claude_plan_tier: Option<ClaudePlanTier>,
    /// Raw OAuth fields, forwarded verbatim so the server can audit which
    /// taxonomy strings we haven't enumerated yet (especially when the
    /// classified `claude_plan_tier` ends up `unknown`).
    #[serde(skip_serializing_if = "Option::is_none")]
    claude_organization_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    claude_rate_limit_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    claude_billing_type: Option<String>,
}

/// Reqwest errors caused by the user's environment (offline, captive portal,
/// flaky DNS, slow network) rather than anything actionable on our side.
/// Filtering these out of Sentry keeps the activation alert signal-to-noise
/// high.
fn is_transient_transport_error(err: &reqwest::Error) -> bool {
    err.is_connect() || err.is_timeout() || err.is_request()
}

fn plan_tier_header_value(tier: &ClaudePlanTier) -> &'static str {
    match tier {
        ClaudePlanTier::Free => "free",
        ClaudePlanTier::Pro => "pro",
        ClaudePlanTier::Max5x => "max5x",
        ClaudePlanTier::Max20x => "max20x",
        ClaudePlanTier::Unknown => "unknown",
    }
}

impl IdentityPayload {
    fn for_state(state: &AppState) -> Self {
        let claude = state.cached_claude_profile();
        Self::build(Some(&claude))
    }

    fn device_only() -> Self {
        Self::build(None)
    }

    fn build(claude: Option<&ClaudeAccountProfile>) -> Self {
        let device = device::current();
        Self {
            device_id: device.machine_id_digest,
            chopratejas_instance_id: device.chopratejas_instance_id,
            claude_account_uuid: claude.and_then(|p| p.account_uuid.clone()),
            claude_email: claude.and_then(|p| p.email.clone()),
            claude_plan_tier: claude.map(|p| p.plan_tier.clone()),
            claude_organization_type: claude.and_then(|p| p.organization_type.clone()),
            claude_rate_limit_tier: claude.and_then(|p| p.rate_limit_tier.clone()),
            claude_billing_type: claude.and_then(|p| p.billing_type.clone()),
        }
    }

    fn apply_headers(
        &self,
        mut builder: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        builder = builder.header("X-Headroom-App-Version", env!("CARGO_PKG_VERSION"));
        builder = builder.header("X-Headroom-Device-Id", &self.device_id);
        if let Some(value) = self.chopratejas_instance_id.as_deref() {
            builder = builder.header("X-Headroom-Chopratejas-Id", value);
        }
        if let Some(value) = self.claude_account_uuid.as_deref() {
            builder = builder.header("X-Headroom-Claude-Uuid", value);
        }
        if let Some(value) = self.claude_email.as_deref() {
            builder = builder.header("X-Headroom-Claude-Email", value);
        }
        if let Some(tier) = self.claude_plan_tier.as_ref() {
            builder = builder.header("X-Headroom-Claude-Plan", plan_tier_header_value(tier));
        }
        if let Some(value) = self.claude_organization_type.as_deref() {
            builder = builder.header("X-Headroom-Claude-Organization-Type", value);
        }
        if let Some(value) = self.claude_rate_limit_tier.as_deref() {
            builder = builder.header("X-Headroom-Claude-Rate-Limit-Tier", value);
        }
        if let Some(value) = self.claude_billing_type.as_deref() {
            builder = builder.header("X-Headroom-Claude-Billing-Type", value);
        }
        builder
    }
}

/// Stable comparison key for an `IdentityPayload`'s Claude fields. Used to
/// skip redundant `desktop/grace/start` posts when the bearer-triggered
/// worker fires for an account whose fingerprint has not changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityFingerprint {
    claude_account_uuid: Option<String>,
    claude_email: Option<String>,
    claude_plan_tier: Option<ClaudePlanTier>,
    claude_organization_type: Option<String>,
    claude_rate_limit_tier: Option<String>,
    claude_billing_type: Option<String>,
}

impl IdentityFingerprint {
    fn from_payload(p: &IdentityPayload) -> Self {
        Self {
            claude_account_uuid: p.claude_account_uuid.clone(),
            claude_email: p.claude_email.clone(),
            claude_plan_tier: p.claude_plan_tier.clone(),
            claude_organization_type: p.claude_organization_type.clone(),
            claude_rate_limit_tier: p.claude_rate_limit_tier.clone(),
            claude_billing_type: p.claude_billing_type.clone(),
        }
    }

    /// True when there is nothing meaningful to report — no UUID and no real
    /// plan tier. This is the bearer-not-yet-captured shape.
    fn is_empty(&self) -> bool {
        self.claude_account_uuid.is_none()
            && matches!(
                self.claude_plan_tier,
                None | Some(ClaudePlanTier::Unknown)
            )
    }
}

/// True when a Claude profile carries every identity field we want headroom-web
/// to record: account UUID, email, and a classified plan tier (i.e. Anthropic's
/// OAuth profile fetch returned a populated payload, not a sparse one).
pub fn is_identity_complete(profile: &ClaudeAccountProfile) -> bool {
    profile.account_uuid.is_some()
        && profile.email.is_some()
        && !matches!(profile.plan_tier, ClaudePlanTier::Unknown)
}

/// Warm the cached Claude profile and, if it carries new identity fields,
/// push the populated `IdentityPayload` to `desktop/grace/start`.
///
/// Invoked by the bearer-pusher worker thread whenever the intercept proxy
/// captures a fresh bearer. The OAuth-profile fetch is throttled to once
/// per 24 h once we already know who the user is, so the per-hour bearer
/// rotations don't translate into per-hour calls to Anthropic's
/// `/api/oauth/profile`.
///
/// Throttle does NOT short-circuit the function: we still consult
/// `cached_claude_profile()` (which may have been refreshed by the pricing
/// UI) and push whatever fingerprint it yields if it differs from what we
/// last sent to headroom-web. That way an account switch picked up by
/// another path still propagates without waiting for the 24 h window to
/// expire.
///
/// Idempotent: if the resulting fingerprint matches the last successful
/// push in this session, this is a no-op. On HTTP failure the fingerprint
/// is not recorded, so the next bearer change retries.
pub fn warm_and_push_identity(state: &AppState) {
    const COMPLETE_FETCH_THROTTLE: std::time::Duration =
        std::time::Duration::from_secs(24 * 60 * 60);

    // When the throttle is active we skip the explicit cache warm — but we
    // still read whatever's currently cached and let the fingerprint memo
    // decide whether anything is worth pushing. `IdentityPayload::for_state`
    // calls `cached_claude_profile()`, which itself respects the 5-min TTL
    // and will only round-trip to Anthropic on a true cache miss.
    let throttled = state.complete_identity_fetched_within(COMPLETE_FETCH_THROTTLE);
    if !throttled {
        // Force-warm. Cheap when the bearer slot is empty (short-circuits
        // inside `detect_claude_profile_uncached`).
        let _ = state.cached_claude_profile();
    }

    let identity = IdentityPayload::for_state(state);
    let fp = IdentityFingerprint::from_payload(&identity);

    if fp.is_empty() {
        return;
    }

    if state.identity_fingerprint_already_pushed(&fp) {
        return;
    }

    // Fingerprint differs from last push but throttle is active: another
    // path (pricing UI poll, sign-in) must have refreshed the cache with
    // new identity fields. Push them now even though the worker would
    // otherwise have skipped the OAuth fetch — this is the account-switch
    // path.
    match fetch_grace_start(&identity) {
        Ok(_) => state.record_pushed_identity_fingerprint(fp),
        Err(_) => {
            // Silent — matches `reconcile_local_state_with_server`'s
            // pattern. `fetch_grace_start` failures are typically transient
            // (offline, captive portal, headroom-web blip) and the next
            // bearer change will retry. Sentry-capturing per failure would
            // pin every offline session.
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraceResponse {
    first_seen_at: DateTime<Utc>,
    #[allow(dead_code)]
    grace_ends_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    trial_started_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    trial_ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct ClaudeOauthProfile {
    account: ClaudeOauthProfileAccount,
    organization: Option<ClaudeOauthProfileOrganization>,
}

#[derive(Debug, Clone)]
struct ClaudeOauthProfileAccount {
    uuid: Option<String>,
    email: Option<String>,
    display_name: Option<String>,
    created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct ClaudeOauthProfileOrganization {
    uuid: Option<String>,
    billing_type: Option<String>,
    subscription_created_at: Option<DateTime<Utc>>,
    has_extra_usage_enabled: bool,
    /// e.g. "claude_pro", "claude_max", "claude_enterprise"
    organization_type: Option<String>,
    /// e.g. "default_claude_ai", "claude_max_5x", "claude_max_20x",
    /// "default_claude_max_x5", "default_claude_max_x20" (Anthropic ships both
    /// the `_5x`/`_20x` and `_x5`/`_x20` orderings in the wild)
    rate_limit_tier: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteAccountEnvelope {
    account: RemoteAccountResponse,
    #[serde(default)]
    launch_discount_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteAccountResponse {
    email: String,
    trial_started_at: Option<DateTime<Utc>>,
    trial_ends_at: Option<DateTime<Utc>>,
    trial_active: bool,
    subscription_active: bool,
    subscription_tier: Option<HeadroomSubscriptionTier>,
    #[serde(default)]
    subscription_started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    subscription_renews_at: Option<DateTime<Utc>>,
    #[serde(default)]
    subscription_amount_cents: Option<i64>,
    #[serde(default)]
    subscription_billing_period: Option<String>,
    #[serde(default)]
    subscription_discount_duration: Option<String>,
    #[serde(default)]
    subscription_discount_duration_in_months: Option<i64>,
    #[serde(default)]
    subscription_cancel_at_period_end: bool,
    #[serde(default)]
    subscription_ends_at: Option<DateTime<Utc>>,
    invite_code: Option<String>,
    accepted_invites_count: usize,
    invite_bonus_percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyCodeResponse {
    session_token: String,
    account: RemoteAccountResponse,
    #[serde(default)]
    launch_discount_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestCodeResponse {
    email: String,
    expires_in_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestCodePayload<'a> {
    email: &'a str,
    #[serde(flatten)]
    identity: IdentityPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifyCodePayload<'a> {
    email: &'a str,
    code: &'a str,
    invite_code: Option<&'a str>,
    #[serde(flatten)]
    identity: IdentityPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutSessionPayload {
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckoutSessionResponse {
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct BillingPortalResponse {
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiErrorResponse {
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum RemoteAccountSyncError {
    Unauthorized,
    Other,
}

pub fn get_pricing_status(state: &AppState) -> Result<HeadroomPricingStatus, String> {
    let local_state = reconcile_local_state_with_server(state)?;
    let local_grace_ends_at = local_state.first_seen_at + Duration::hours(LOCAL_GRACE_PERIOD_HOURS);
    let local_grace_active = Utc::now() < local_grace_ends_at;
    let session_token = read_session_token()?;
    let identity = IdentityPayload::for_state(state);
    let (authenticated, account, account_sync_error, launch_discount_active) =
        if let Some(token) = session_token.as_deref() {
            let envelope_result = fetch_remote_account(token, &identity);
            let launch_discount_active = envelope_result
                .as_ref()
                .map(|e| e.launch_discount_active)
                .unwrap_or(false);
            let account_result = envelope_result.map(|e| e.account);
            let (auth, acc, err) = merge_background_account_sync(Some(token), account_result);
            (auth, acc, err, launch_discount_active)
        } else {
            let launch_discount_active = fetch_public_config()
                .map(|c| c.launch_discount_active)
                .unwrap_or(false);
            (false, None, None, launch_discount_active)
        };

    let claude = detect_claude_profile(state);
    let last_known_good_plan_tier = state.last_known_good_plan_tier();
    let tier_mismatch = resolve_tier_mismatch(account.as_ref(), &claude);

    Ok(evaluate_pricing_status_with_mismatch(
        authenticated,
        local_state.first_seen_at,
        local_grace_ends_at,
        local_grace_active,
        account_sync_error,
        account,
        claude,
        launch_discount_active,
        last_known_good_plan_tier,
        tier_mismatch,
    ))
}

pub fn request_auth_code(state: &AppState, email: &str) -> Result<HeadroomAuthCodeRequest, String> {
    request_auth_code_with_base_url(state, email, &api_base_url())
}

/// Test-only seam: `request_auth_code` against a parameterized base URL so a
/// canned-response test server can stand in for headroom-web.
pub(crate) fn request_auth_code_with_base_url(
    state: &AppState,
    email: &str,
    base_url: &str,
) -> Result<HeadroomAuthCodeRequest, String> {
    let trimmed = email.trim().to_ascii_lowercase();
    if trimmed.is_empty() || !trimmed.contains('@') {
        return Err("Enter a valid email address.".into());
    }

    let response = http_client()?
        .post(join_url(base_url, "desktop/auth/request_code"))
        .json(&RequestCodePayload {
            email: &trimmed,
            identity: IdentityPayload::for_state(state),
        })
        .send()
        .map_err(|err| {
            let msg = format!("Could not request sign-in code: {err}");
            sentry::capture_message(&msg, sentry::Level::Info);
            msg
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let msg = format!("Could not request sign-in code (status {status}).");
        if status >= 500 {
            sentry::capture_message(&msg, sentry::Level::Error);
        }
        return Err(msg);
    }

    let body: RequestCodeResponse = response.json().map_err(|err| {
        let msg = format!("Could not parse sign-in response: {err}");
        sentry::capture_message(&msg, sentry::Level::Error);
        msg
    })?;

    Ok(HeadroomAuthCodeRequest {
        email: body.email,
        expires_in_seconds: body.expires_in_seconds.max(1).min(AUTH_CODE_EXPIRY_SECONDS),
    })
}

pub fn verify_auth_code(
    state: &AppState,
    email: &str,
    code: &str,
    invite_code: Option<&str>,
) -> Result<HeadroomPricingStatus, String> {
    verify_auth_code_with_base_url(state, email, code, invite_code, &api_base_url())
}

/// Test-only seam: `verify_auth_code` against a parameterized base URL so a
/// canned-response test server can stand in for headroom-web.
pub(crate) fn verify_auth_code_with_base_url(
    state: &AppState,
    email: &str,
    code: &str,
    invite_code: Option<&str>,
    base_url: &str,
) -> Result<HeadroomPricingStatus, String> {
    let trimmed_email = email.trim().to_ascii_lowercase();
    let trimmed_code = code.trim();
    if trimmed_email.is_empty() || !trimmed_email.contains('@') {
        return Err("Enter a valid email address.".into());
    }
    if trimmed_code.is_empty() {
        return Err("Enter the authentication code from your email.".into());
    }

    let response = http_client()?
        .post(join_url(base_url, "desktop/auth/verify_code"))
        .json(&VerifyCodePayload {
            email: &trimmed_email,
            code: trimmed_code,
            invite_code: invite_code.map(str::trim).filter(|value| !value.is_empty()),
            identity: IdentityPayload::for_state(state),
        })
        .send()
        .map_err(|err| format!("Could not verify sign-in code: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Could not verify sign-in code (status {}).",
            response.status().as_u16()
        ));
    }

    let body: VerifyCodeResponse = response
        .json()
        .map_err(|err| format!("Could not parse verification response: {err}"))?;

    write_session_token(&body.session_token)?;

    let local_state = reconcile_local_state_with_server(state)?;
    let local_grace_ends_at = local_state.first_seen_at + Duration::hours(LOCAL_GRACE_PERIOD_HOURS);
    let claude = detect_claude_profile(state);
    let last_known_good_plan_tier = state.last_known_good_plan_tier();
    let account = remote_account_to_profile(body.account);
    let tier_mismatch = resolve_tier_mismatch(Some(&account), &claude);

    Ok(evaluate_pricing_status_with_mismatch(
        true,
        local_state.first_seen_at,
        local_grace_ends_at,
        Utc::now() < local_grace_ends_at,
        None,
        Some(account),
        claude,
        body.launch_discount_active,
        last_known_good_plan_tier,
        tier_mismatch,
    ))
}

pub fn sign_out() -> Result<(), String> {
    clear_session_token()
}

pub fn activate_account(
    state: &AppState,
    lifetime_tokens_saved: u64,
) -> Result<HeadroomPricingStatus, String> {
    activate_account_with_base_url(state, lifetime_tokens_saved, &api_base_url())
}

/// Test-only seam: `activate_account` against a parameterized base URL so a
/// canned-response test server can stand in for headroom-web.
pub(crate) fn activate_account_with_base_url(
    state: &AppState,
    lifetime_tokens_saved: u64,
    base_url: &str,
) -> Result<HeadroomPricingStatus, String> {
    let token = read_session_token()?
        .ok_or_else(|| "Sign in to Headroom before activating desktop access.".to_string())?;
    let identity = IdentityPayload::for_state(state);
    let builder = http_client()?
        .post(join_url(base_url, "desktop/account/activate"))
        .header("Authorization", format!("Bearer {token}"));
    let response = identity
        .apply_headers(builder)
        .json(&serde_json::json!({ "lifetime_tokens_saved": lifetime_tokens_saved }))
        .send()
        .map_err(|err| {
            let msg = format!("Could not activate Headroom desktop access: {err}");
            if !is_transient_transport_error(&err) {
                sentry::capture_message(&msg, sentry::Level::Warning);
            }
            msg
        })?;

    if response.status().as_u16() == 401 {
        clear_session_token()?;
        return Err("Your Headroom session expired. Sign in again.".into());
    }

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let msg = format!("Could not activate Headroom desktop access (status {status}).");
        if status >= 500 {
            sentry::capture_message(&msg, sentry::Level::Error);
        }
        return Err(msg);
    }

    let body: RemoteAccountEnvelope = response.json().map_err(|err| {
        let msg = format!("Could not parse Headroom activation response: {err}");
        sentry::capture_message(&msg, sentry::Level::Error);
        msg
    })?;
    let local_state = reconcile_local_state_with_server(state)?;
    let local_grace_ends_at = local_state.first_seen_at + Duration::hours(LOCAL_GRACE_PERIOD_HOURS);
    let claude = detect_claude_profile(state);
    let last_known_good_plan_tier = state.last_known_good_plan_tier();
    let account = remote_account_to_profile(body.account);
    let tier_mismatch = resolve_tier_mismatch(Some(&account), &claude);

    Ok(evaluate_pricing_status_with_mismatch(
        true,
        local_state.first_seen_at,
        local_grace_ends_at,
        Utc::now() < local_grace_ends_at,
        None,
        Some(account),
        claude,
        body.launch_discount_active,
        last_known_good_plan_tier,
        tier_mismatch,
    ))
}

/// Fire-and-forget: reports a milestone to the server so it can trigger
/// the feedback email for users who were below the threshold at sign-up.
/// Silently no-ops if the user is not signed in or the request fails.
pub fn report_milestone(milestone_tokens_saved: u64) {
    let token = match read_session_token() {
        Ok(Some(t)) => t,
        _ => return,
    };
    let client = match http_client() {
        Ok(c) => c,
        Err(_) => return,
    };
    let identity = IdentityPayload::device_only();
    let builder = client
        .post(api_url("desktop/milestones"))
        .header("Authorization", format!("Bearer {token}"));
    let _ = identity
        .apply_headers(builder)
        .json(&serde_json::json!({ "milestone_tokens_saved": milestone_tokens_saved }))
        .send();
}

pub fn create_checkout_session(
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
) -> Result<String, String> {
    create_checkout_session_with_base_url(subscription_tier, billing_period, &api_base_url())
}

/// Test-only seam: `create_checkout_session` against a parameterized base URL.
pub(crate) fn create_checkout_session_with_base_url(
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
    base_url: &str,
) -> Result<String, String> {
    let token = read_session_token()?
        .ok_or_else(|| "Sign in to Headroom before starting checkout.".to_string())?;
    let response = http_client()?
        .post(join_url(base_url, "desktop/checkout"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&CheckoutSessionPayload {
            subscription_tier,
            billing_period,
        })
        .send()
        .map_err(|err| format!("Could not create checkout session: {err}"))?;

    if response.status().as_u16() == 401 {
        clear_session_token()?;
        return Err("Your Headroom session expired. Sign in again.".into());
    }

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let api_error = response
            .json::<ApiErrorResponse>()
            .ok()
            .and_then(|body| body.error)
            .filter(|value| !value.trim().is_empty());
        return Err(api_error
            .unwrap_or_else(|| format!("Could not create checkout session (status {status}).")));
    }

    response
        .json::<CheckoutSessionResponse>()
        .map(|body| body.url)
        .map_err(|err| format!("Could not parse checkout response: {err}"))
}

pub fn change_subscription_plan(
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
) -> Result<(), String> {
    change_subscription_plan_with_base_url(subscription_tier, billing_period, &api_base_url())
}

/// Test-only seam: `change_subscription_plan` against a parameterized base URL.
pub(crate) fn change_subscription_plan_with_base_url(
    subscription_tier: HeadroomSubscriptionTier,
    billing_period: BillingPeriod,
    base_url: &str,
) -> Result<(), String> {
    let token = read_session_token()?
        .ok_or_else(|| "Sign in to Headroom before changing your plan.".to_string())?;
    let response = http_client()?
        .post(join_url(base_url, "desktop/subscriptions/change_plan"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&CheckoutSessionPayload {
            subscription_tier,
            billing_period,
        })
        .send()
        .map_err(|err| format!("Could not change subscription plan: {err}"))?;

    if response.status().as_u16() == 401 {
        clear_session_token()?;
        return Err("Your Headroom session expired. Sign in again.".into());
    }

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let api_error = response
            .json::<ApiErrorResponse>()
            .ok()
            .and_then(|body| body.error)
            .filter(|value| !value.trim().is_empty());
        return Err(api_error
            .unwrap_or_else(|| format!("Could not change subscription plan (status {status}).")));
    }

    Ok(())
}

pub fn reactivate_subscription() -> Result<(), String> {
    reactivate_subscription_with_base_url(&api_base_url())
}

pub(crate) fn reactivate_subscription_with_base_url(base_url: &str) -> Result<(), String> {
    let token = read_session_token()?
        .ok_or_else(|| "Sign in to Headroom before reactivating your plan.".to_string())?;
    let response = http_client()?
        .post(join_url(base_url, "desktop/subscriptions/reactivate"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .map_err(|err| format!("Could not reactivate subscription: {err}"))?;

    if response.status().as_u16() == 401 {
        clear_session_token()?;
        return Err("Your Headroom session expired. Sign in again.".into());
    }

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let api_error = response
            .json::<ApiErrorResponse>()
            .ok()
            .and_then(|body| body.error)
            .filter(|value| !value.trim().is_empty());
        return Err(api_error
            .unwrap_or_else(|| format!("Could not reactivate subscription (status {status}).")));
    }

    Ok(())
}

pub fn get_billing_portal_url(target: Option<String>) -> Result<String, String> {
    get_billing_portal_url_with_base_url(&api_base_url(), target.as_deref())
}

/// Test-only seam: `get_billing_portal_url` against a parameterized base URL.
pub(crate) fn get_billing_portal_url_with_base_url(
    base_url: &str,
    target: Option<&str>,
) -> Result<String, String> {
    let token = read_session_token()?
        .ok_or_else(|| "Sign in to Headroom before accessing billing.".to_string())?;
    let mut request = http_client()?
        .get(join_url(base_url, "desktop/billing_portal"))
        .header("Authorization", format!("Bearer {token}"));
    if let Some(target_value) = target {
        request = request.query(&[("target", target_value)]);
    }
    let response = request
        .send()
        .map_err(|err| format!("Could not reach billing portal: {err}"))?;

    if response.status().as_u16() == 401 {
        clear_session_token()?;
        return Err("Your Headroom session expired. Sign in again.".into());
    }

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let api_error = response
            .json::<ApiErrorResponse>()
            .ok()
            .and_then(|body| body.error)
            .filter(|value| !value.trim().is_empty());
        return Err(api_error
            .unwrap_or_else(|| format!("Could not open billing portal (status {status}).")));
    }

    response
        .json::<BillingPortalResponse>()
        .map(|body| body.url)
        .map_err(|err| format!("Could not parse billing portal response: {err}"))
}

pub fn fetch_claude_usage(state: &AppState) -> Result<ClaudeUsage, String> {
    let access_token = state.current_bearer_token().ok_or_else(|| {
        "No Claude AI token captured yet — make sure Claude Code is running and authenticated via Claude AI (not an API key), then try again after the first request passes through the proxy.".to_string()
    })?;

    let resp = http_client()?
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Content-Type", "application/json")
        .send()
        .map_err(|e| format!("Request failed: {e}"))?;

    let body: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    parse_claude_usage_response(&body)
}

/// Pure parser for the Anthropic OAuth usage endpoint response. Extracted so
/// schema-drift tests don't need a live HTTP server.
fn parse_claude_usage_response(body: &serde_json::Value) -> Result<ClaudeUsage, String> {
    use chrono::DateTime;

    if let Some(err) = body.get("error") {
        return Err(format!(
            "API error: {}",
            err["message"].as_str().unwrap_or("unknown")
        ));
    }

    let parse_window = |v: &serde_json::Value| -> Option<ClaudeUsageWindow> {
        let utilization = v.get("utilization")?.as_f64()?;
        let resets_at_str = v.get("resets_at")?.as_str()?;
        let resets_at = DateTime::parse_from_rfc3339(resets_at_str).ok()?.to_utc();
        Some(ClaudeUsageWindow {
            utilization,
            resets_at,
        })
    };

    let five_hour = body.get("five_hour").and_then(parse_window);
    let seven_day = body.get("seven_day").and_then(parse_window);

    let extra_usage = body.get("extra_usage").and_then(|e| {
        Some(crate::models::ClaudeExtraUsage {
            is_enabled: e.get("is_enabled")?.as_bool()?,
            monthly_limit: e.get("monthly_limit").and_then(|v| v.as_f64()),
            used_credits: e.get("used_credits").and_then(|v| v.as_f64()),
            utilization: e.get("utilization").and_then(|v| v.as_f64()),
        })
    });

    Ok(ClaudeUsage {
        five_hour,
        seven_day,
        extra_usage,
    })
}

fn evaluate_pricing_status_with_mismatch(
    authenticated: bool,
    local_grace_started_at: DateTime<Utc>,
    local_grace_ends_at: DateTime<Utc>,
    local_grace_active: bool,
    account_sync_error: Option<String>,
    account: Option<HeadroomAccountProfile>,
    claude: ClaudeAccountProfile,
    launch_discount_active: bool,
    last_known_good_plan_tier: Option<ClaudePlanTier>,
    tier_mismatch: Option<TierMismatch>,
) -> HeadroomPricingStatus {
    #[cfg(debug_assertions)]
    let local_grace_active = if INDEFINITE_TRIAL {
        true
    } else {
        local_grace_active
    };
    let needs_authentication = !authenticated && !local_grace_active;
    let mut optimization_allowed = true;
    let mut should_nudge = false;
    let mut nudge_level: u8 = 0;
    let mut gate_reason = None;
    let gate_message: String;
    let mut nudge_threshold_percent = None;
    let mut effective_nudge_thresholds_percent: Option<Vec<f64>> = None;
    let mut disable_threshold_percent = None;
    let mut effective_disable_threshold_percent = None;
    let mut recommended_subscription_tier = None;

    if needs_authentication {
        optimization_allowed = false;
        gate_reason = Some(PricingGateReason::SignInRequired);
        gate_message =
            "Create a Headroom account to unlock your 14-day trial and keep optimization enabled."
                .into();
    } else if let Some(account) = account.as_ref() {
        if account.subscription_active {
            if tier_mismatch.as_ref().is_some_and(|m| m.clamped) {
                let gate = paid_plan_gate(
                    &claude.plan_tier,
                    claude.weekly_utilization_pct,
                    account.invite_bonus_percent,
                );
                optimization_allowed = gate.optimization_allowed;
                should_nudge = gate.should_nudge;
                nudge_level = gate.nudge_level;
                gate_reason = gate.gate_reason;
                nudge_threshold_percent = gate.nudge_threshold_percent;
                effective_nudge_thresholds_percent = gate.effective_nudge_thresholds_percent;
                disable_threshold_percent = gate.disable_threshold_percent;
                effective_disable_threshold_percent = gate.effective_disable_threshold_percent;
                recommended_subscription_tier = gate.recommended_subscription_tier;
                gate_message = gate.gate_message;
            } else {
                recommended_subscription_tier = tier_mismatch.as_ref().map(|m| m.recommended_tier);
                gate_message =
                    "Headroom subscription active. Optimization stays fully enabled.".into();
            }
        } else if account.trial_active {
            gate_message =
                "Your 14-day Headroom trial is active with unlimited optimization.".into();
        } else {
            match claude.plan_tier {
                ClaudePlanTier::Free => {
                    gate_message =
                        "Claude Free accounts can keep using Headroom without weekly usage gating."
                            .into();
                }
                ClaudePlanTier::Unknown => {
                    // Live classifier returned Unknown — Anthropic's OAuth
                    // profile came back sparse. Free usage requires a live
                    // Free signal; cached Free is not enough to grant it
                    // post-trial. Fall back to cached paid tier when present,
                    // otherwise Pro as the default upgrade path.
                    let effective_tier = match last_known_good_plan_tier.as_ref() {
                        Some(tier) if !matches!(tier, ClaudePlanTier::Free) => tier.clone(),
                        _ => ClaudePlanTier::Pro,
                    };
                    let gate = paid_plan_gate(
                        &effective_tier,
                        claude.weekly_utilization_pct,
                        account.invite_bonus_percent,
                    );
                    optimization_allowed = gate.optimization_allowed;
                    should_nudge = gate.should_nudge;
                    nudge_level = gate.nudge_level;
                    gate_reason = gate.gate_reason;
                    nudge_threshold_percent = gate.nudge_threshold_percent;
                    effective_nudge_thresholds_percent = gate.effective_nudge_thresholds_percent;
                    disable_threshold_percent = gate.disable_threshold_percent;
                    effective_disable_threshold_percent = gate.effective_disable_threshold_percent;
                    recommended_subscription_tier = gate.recommended_subscription_tier;
                    gate_message = format!(
                        "We couldn't refresh your Claude plan from Anthropic right now, so we're applying {} thresholds until the next sync. {}",
                        plan_tier_display(&effective_tier),
                        gate.gate_message
                    );
                }
                _ => {
                    let gate = paid_plan_gate(
                        &claude.plan_tier,
                        claude.weekly_utilization_pct,
                        account.invite_bonus_percent,
                    );
                    optimization_allowed = gate.optimization_allowed;
                    should_nudge = gate.should_nudge;
                    nudge_level = gate.nudge_level;
                    gate_reason = gate.gate_reason;
                    nudge_threshold_percent = gate.nudge_threshold_percent;
                    effective_nudge_thresholds_percent = gate.effective_nudge_thresholds_percent;
                    disable_threshold_percent = gate.disable_threshold_percent;
                    effective_disable_threshold_percent = gate.effective_disable_threshold_percent;
                    recommended_subscription_tier = gate.recommended_subscription_tier;
                    gate_message = gate.gate_message;
                }
            }
        }
    } else if authenticated {
        gate_message =
            "Headroom account connected, but pricing status could not be synced right now. Optimization stays enabled for now."
                .into();
    } else {
        gate_message =
            "Headroom is active during your first 72 hours. Create an account to unlock the 14-day trial before this grace period ends."
                .into();
    }

    HeadroomPricingStatus {
        authenticated,
        local_grace_started_at,
        local_grace_ends_at,
        local_grace_active,
        account_sync_error,
        needs_authentication,
        optimization_allowed,
        should_nudge,
        nudge_level,
        gate_reason,
        gate_message,
        nudge_threshold_percent,
        effective_nudge_thresholds_percent,
        disable_threshold_percent,
        effective_disable_threshold_percent,
        recommended_subscription_tier,
        tier_mismatch,
        claude,
        account,
        launch_discount_active,
    }
}

/// Pure comparison: returns `(paid_tier, recommended_tier)` when an active
/// subscriber's paid tier is below the tier their confidently-detected Claude
/// plan implies. Uses the live `claude.plan_tier` only — `Unknown`/`Free`
/// (and any cached fallback) yield no recommended tier, so no mismatch fires.
fn detect_tier_mismatch(
    account: &HeadroomAccountProfile,
    claude: &ClaudeAccountProfile,
) -> Option<(HeadroomSubscriptionTier, HeadroomSubscriptionTier)> {
    if !account.subscription_active {
        return None;
    }
    let paid = account.subscription_tier?;
    let recommended = headroom_tier_for_claude_plan(&claude.plan_tier)?;
    (recommended.rank() > paid.rank()).then_some((paid, recommended))
}

/// Detects the mismatch and manages the persisted grace clock. Sets
/// `mismatch_since` on first detection, clears it once resolved, and reports
/// `clamped` after the grace window elapses.
fn resolve_tier_mismatch(
    account: Option<&HeadroomAccountProfile>,
    claude: &ClaudeAccountProfile,
) -> Option<TierMismatch> {
    let (paid_tier, recommended_tier) = match account.and_then(|a| detect_tier_mismatch(a, claude)) {
        Some(pair) => pair,
        None => {
            if let Ok(mut local) = load_or_initialize_local_state() {
                if local.mismatch_since.is_some() {
                    local.mismatch_since = None;
                    let _ = write_local_state(&local);
                }
            }
            return None;
        }
    };

    let mut local = load_or_initialize_local_state().ok()?;
    let since = match local.mismatch_since {
        Some(since) => since,
        None => {
            let now = Utc::now();
            local.mismatch_since = Some(now);
            let _ = write_local_state(&local);
            now
        }
    };

    let grace_ends_at = since + Duration::days(TIER_MISMATCH_GRACE_DAYS);
    Some(TierMismatch {
        paid_tier,
        recommended_tier,
        grace_ends_at,
        clamped: Utc::now() > grace_ends_at,
    })
}

struct PaidPlanGate {
    optimization_allowed: bool,
    should_nudge: bool,
    nudge_level: u8,
    gate_reason: Option<PricingGateReason>,
    gate_message: String,
    nudge_threshold_percent: Option<f64>,
    effective_nudge_thresholds_percent: Option<Vec<f64>>,
    disable_threshold_percent: Option<f64>,
    effective_disable_threshold_percent: Option<f64>,
    recommended_subscription_tier: Option<HeadroomSubscriptionTier>,
}

fn paid_plan_gate(
    tier: &ClaudePlanTier,
    weekly_utilization_pct: Option<f64>,
    invite_bonus_percent: f64,
) -> PaidPlanGate {
    let pricing = pricing_policy_for_plan(tier);
    let bonus = invite_bonus_percent.clamp(0.0, 50.0);
    let nudge_threshold_percent = pricing
        .as_ref()
        .map(|policy| policy.nudge_thresholds_percent[0]);
    let effective_nudge_thresholds_percent: Option<Vec<f64>> =
        pricing.as_ref().map(|policy| {
            policy
                .nudge_thresholds_percent
                .iter()
                .map(|n| n + bonus)
                .collect()
        });
    let disable_threshold_percent = pricing
        .as_ref()
        .map(|policy| policy.disable_threshold_percent);
    let effective_disable_threshold_percent = pricing.as_ref().map(|policy| {
        (policy.disable_threshold_percent + invite_bonus_percent)
            .min(policy.disable_threshold_percent + 50.0)
    });
    let recommended_subscription_tier = pricing
        .as_ref()
        .map(|policy| policy.recommended_tier.clone());

    let mut optimization_allowed = true;
    let mut should_nudge = false;
    let mut nudge_level: u8 = 0;
    let mut gate_reason = None;
    let gate_message: String;

    if let (Some(weekly_usage), Some(nudges), Some(disable)) = (
        weekly_utilization_pct,
        effective_nudge_thresholds_percent.as_ref(),
        effective_disable_threshold_percent,
    ) {
        if weekly_usage >= disable {
            optimization_allowed = false;
            gate_reason = Some(PricingGateReason::WeeklyUsageLimitReached);
            gate_message = format!(
                "Headroom is paused because you've reached {:.1}% of weekly Claude usage. Upgrade to raise your limit.",
                weekly_usage
            );
        } else {
            nudge_level = nudges.iter().filter(|t| weekly_usage >= **t).count() as u8;
            should_nudge = nudge_level > 0;
            gate_message = if should_nudge {
                format_nudge_message(weekly_usage, disable, nudge_level)
            } else {
                format!(
                    "Headroom is active. It will start nudging at {:.1}% and pause at {:.1}% of weekly Claude usage for your detected plan.",
                    nudges[0], disable
                )
            };
        }
    } else {
        gate_message = "Headroom is active. Send a Claude Code message through Headroom to sync your current weekly usage and pricing threshold.".into();
    }

    PaidPlanGate {
        optimization_allowed,
        should_nudge,
        nudge_level,
        gate_reason,
        gate_message,
        nudge_threshold_percent,
        effective_nudge_thresholds_percent,
        disable_threshold_percent,
        effective_disable_threshold_percent,
        recommended_subscription_tier,
    }
}

fn plan_tier_display(tier: &ClaudePlanTier) -> &'static str {
    match tier {
        ClaudePlanTier::Free => "Free",
        ClaudePlanTier::Pro => "Pro",
        ClaudePlanTier::Max5x => "Max x5",
        ClaudePlanTier::Max20x => "Max x20",
        ClaudePlanTier::Unknown => "Unknown",
    }
}

fn format_nudge_message(weekly_usage: f64, disable: f64, level: u8) -> String {
    match level {
        1 => format!(
            "You're at {:.1}% of weekly Claude usage. Upgrade Headroom to keep optimization through {:.1}% — invites also raise your limit.",
            weekly_usage, disable
        ),
        2 => format!(
            "You're at {:.1}% of weekly Claude usage. Headroom pauses at {:.1}% on the free plan — upgrade now to keep going.",
            weekly_usage, disable
        ),
        _ => format!(
            "You're at {:.1}% of weekly Claude usage. Headroom will pause at {:.1}% — upgrade now to avoid losing optimization.",
            weekly_usage, disable
        ),
    }
}

pub fn detect_claude_profile(state: &AppState) -> ClaudeAccountProfile {
    state.cached_claude_profile()
}

pub fn detect_claude_profile_uncached(state: &AppState) -> ClaudeAccountProfile {
    let Some(token) = state.current_bearer_token() else {
        // No token yet — proxy hasn't seen a request through. Return a minimal
        // profile so the app can show "send a message first" messaging.
        return ClaudeAccountProfile {
            auth_method: ClaudeAuthMethod::Unknown,
            email: None,
            display_name: None,
            account_uuid: None,
            organization_uuid: None,
            billing_type: None,
            account_created_at: None,
            subscription_created_at: None,
            has_extra_usage_enabled: false,
            plan_tier: ClaudePlanTier::Unknown,
            plan_detection_source: None,
            organization_type: None,
            rate_limit_tier: None,
            weekly_utilization_pct: None,
            five_hour_utilization_pct: None,
            extra_usage_monthly_limit: None,
            profile_fetch_error: None,
        };
    };

    let (profile, profile_fetch_error) = match fetch_oauth_profile(&token) {
        Ok(p) => (Some(p), None),
        Err(msg) => (None, Some(msg)),
    };
    let usage = fetch_claude_usage(state).ok();

    let (plan_tier, plan_detection_source) = if let Some(ref p) = profile {
        detect_plan_tier_from_profile(p)
    } else {
        (ClaudePlanTier::Unknown, None)
    };

    // Persist the classifier output when it carries real signal so the
    // pricing gate can fall back to it next time Anthropic returns a sparse
    // profile and we'd otherwise classify as Unknown. The helper filters
    // Unknown internally.
    state.record_known_good_plan_tier(&plan_tier);

    ClaudeAccountProfile {
        auth_method: ClaudeAuthMethod::ClaudeAiOauth,
        email: profile.as_ref().and_then(|p| p.account.email.clone()),
        display_name: profile
            .as_ref()
            .and_then(|p| p.account.display_name.clone()),
        account_uuid: profile.as_ref().and_then(|p| p.account.uuid.clone()),
        organization_uuid: profile
            .as_ref()
            .and_then(|p| p.organization.as_ref().and_then(|o| o.uuid.clone())),
        billing_type: profile
            .as_ref()
            .and_then(|p| p.organization.as_ref().and_then(|o| o.billing_type.clone())),
        account_created_at: profile.as_ref().and_then(|p| p.account.created_at),
        subscription_created_at: profile.as_ref().and_then(|p| {
            p.organization
                .as_ref()
                .and_then(|o| o.subscription_created_at)
        }),
        has_extra_usage_enabled: profile
            .as_ref()
            .and_then(|p| p.organization.as_ref().map(|o| o.has_extra_usage_enabled))
            .unwrap_or(false),
        plan_tier,
        plan_detection_source,
        organization_type: profile
            .as_ref()
            .and_then(|p| p.organization.as_ref().and_then(|o| o.organization_type.clone())),
        rate_limit_tier: profile
            .as_ref()
            .and_then(|p| p.organization.as_ref().and_then(|o| o.rate_limit_tier.clone())),
        weekly_utilization_pct: usage
            .as_ref()
            .and_then(|u| u.seven_day.as_ref().map(|w| w.utilization)),
        five_hour_utilization_pct: usage
            .as_ref()
            .and_then(|u| u.five_hour.as_ref().map(|w| w.utilization)),
        extra_usage_monthly_limit: usage
            .as_ref()
            .and_then(|u| u.extra_usage.as_ref().and_then(|e| e.monthly_limit)),
        profile_fetch_error,
    }
}

fn fetch_oauth_profile(token: &str) -> Result<ClaudeOauthProfile, String> {
    let response = http_client()?
        .get("https://api.anthropic.com/api/oauth/profile")
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Content-Type", "application/json")
        .send()
        .map_err(|_| {
            "Couldn't reach Anthropic to refresh your Claude plan. Check your internet connection \
             and we'll try again shortly."
                .to_string()
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let user_msg = if status >= 500 {
            format!(
                "Anthropic is having trouble serving your Claude plan right now (HTTP {status}). \
                 We'll keep trying."
            )
        } else if status == 401 || status == 403 {
            "Anthropic rejected our request for your Claude plan. Try signing out of Claude Code \
             and back in."
                .to_string()
        } else {
            format!(
                "Anthropic returned an unexpected response for your Claude plan (HTTP {status}). \
                 We'll try again shortly."
            )
        };
        return Err(user_msg);
    }

    let body: serde_json::Value = response.json().map_err(|err| {
        sentry::capture_message(
            &format!("Could not parse Claude OAuth profile: {err}"),
            sentry::Level::Error,
        );
        "We couldn't read the response from Anthropic for your Claude plan. Please report this if \
         it keeps happening."
            .to_string()
    })?;

    parse_oauth_profile_value(&body).ok_or_else(|| {
        "Anthropic's response didn't include your Claude account details. Please report this if \
         it keeps happening."
            .to_string()
    })
}

fn parse_oauth_profile_value(value: &serde_json::Value) -> Option<ClaudeOauthProfile> {
    let root = value
        .get("profile")
        .or_else(|| value.get("data"))
        .unwrap_or(value);
    let account_value = root.get("account").unwrap_or(root);

    Some(ClaudeOauthProfile {
        account: ClaudeOauthProfileAccount {
            uuid: json_string(account_value, &["uuid", "account_uuid"]),
            email: json_string(account_value, &["email", "email_address"]),
            display_name: json_string(account_value, &["display_name", "displayName"]),
            created_at: json_datetime(account_value, &["created_at", "createdAt"]),
        },
        organization: root
            .get("organization")
            .and_then(parse_oauth_profile_organization),
    })
}

fn parse_oauth_profile_organization(
    value: &serde_json::Value,
) -> Option<ClaudeOauthProfileOrganization> {
    Some(ClaudeOauthProfileOrganization {
        uuid: json_string(value, &["uuid", "organization_uuid"]),
        billing_type: json_string(value, &["billing_type", "billingType"]),
        subscription_created_at: json_datetime(
            value,
            &["subscription_created_at", "subscriptionCreatedAt"],
        ),
        has_extra_usage_enabled: json_bool(
            value,
            &["has_extra_usage_enabled", "hasExtraUsageEnabled"],
        )
        .unwrap_or(false),
        organization_type: json_string(value, &["organization_type", "organizationType"]),
        rate_limit_tier: json_string(value, &["rate_limit_tier", "rateLimitTier"]),
    })
}

fn detect_plan_tier_from_profile(profile: &ClaudeOauthProfile) -> (ClaudePlanTier, Option<String>) {
    let Some(org) = profile.organization.as_ref() else {
        return (ClaudePlanTier::Free, Some("oauth_profile.account".into()));
    };

    if let Some(rate_limit_tier) = org.rate_limit_tier.as_deref() {
        let normalized = rate_limit_tier.trim().to_ascii_lowercase();
        // Anthropic ships both orderings in the wild: "claude_max_20x" and
        // "default_claude_max_x20" (same for 5x/x5). Match either.
        if normalized.contains("20x") || normalized.contains("x20") {
            return (
                ClaudePlanTier::Max20x,
                Some("oauth_profile.organization.rateLimitTier".into()),
            );
        }
        if normalized.contains("5x") || normalized.contains("x5") {
            return (
                ClaudePlanTier::Max5x,
                Some("oauth_profile.organization.rateLimitTier".into()),
            );
        }
        // Anthropic's internal label for Team-plan rate limits. Show Max20x
        // pricing rather than falling through to Pro.
        if normalized.contains("raven") {
            return (
                ClaudePlanTier::Max20x,
                Some("oauth_profile.organization.rateLimitTier".into()),
            );
        }
        if normalized == "default_claude_ai" {
            let organization_type = org.organization_type.as_deref().unwrap_or_default();
            if organization_type.eq_ignore_ascii_case("claude_max") {
                return (
                    ClaudePlanTier::Max5x,
                    Some("oauth_profile.organization.organizationType".into()),
                );
            }
            if organization_type.eq_ignore_ascii_case("claude_pro")
                || organization_type.eq_ignore_ascii_case("claude_enterprise")
            {
                return (
                    ClaudePlanTier::Pro,
                    Some("oauth_profile.organization.organizationType".into()),
                );
            }
        }
    }

    if let Some(organization_type) = org.organization_type.as_deref() {
        let normalized = organization_type.trim().to_ascii_lowercase();
        if normalized == "claude_max" {
            return (
                ClaudePlanTier::Max5x,
                Some("oauth_profile.organization.organizationType".into()),
            );
        }
        if normalized == "claude_pro" || normalized == "claude_enterprise" {
            return (
                ClaudePlanTier::Pro,
                Some("oauth_profile.organization.organizationType".into()),
            );
        }
        if normalized == "claude_free" || normalized == "free" {
            return (
                ClaudePlanTier::Free,
                Some("oauth_profile.organization.organizationType".into()),
            );
        }
    }

    if org.subscription_created_at.is_none() {
        return (
            ClaudePlanTier::Free,
            Some("oauth_profile.organization.subscriptionCreatedAt".into()),
        );
    }

    log_unknown_plan_tier_once(profile);
    (
        ClaudePlanTier::Unknown,
        Some("oauth_profile.organization".into()),
    )
}

/// Capture the raw classification fields whenever `detect_plan_tier_from_profile`
/// falls into the `Unknown` branch — i.e., the user has an Anthropic
/// organization with `subscription_created_at` set but neither
/// `organization_type` nor `rate_limit_tier` matches our enum. Almost
/// certainly Team/Workspace/Enterprise plans we haven't enumerated.
///
/// Currently those users bypass the pricing gate entirely, which means
/// paying Anthropic customers get Headroom for free. Goal of this telemetry
/// is to learn which taxonomy strings to add to the detection (or to a new
/// "treat as Pro" fallback) before changing the gate policy.
///
/// Deduped on content — Sentry sees one event per distinct
/// (organization_type, rate_limit_tier, has_subscription, billing_type)
/// combo across the lifetime of the desktop process.
fn log_unknown_plan_tier_once(profile: &ClaudeOauthProfile) {
    use std::collections::HashSet;
    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::sync::OnceLock;

    static SEEN: OnceLock<parking_lot::Mutex<HashSet<u64>>> = OnceLock::new();

    let org = profile.organization.as_ref();
    let organization_type = org
        .and_then(|o| o.organization_type.as_deref())
        .unwrap_or("");
    let rate_limit_tier = org.and_then(|o| o.rate_limit_tier.as_deref()).unwrap_or("");
    let billing_type = org.and_then(|o| o.billing_type.as_deref()).unwrap_or("");
    let has_subscription_created_at = org
        .and_then(|o| o.subscription_created_at.as_ref())
        .is_some();

    let mut hasher = DefaultHasher::new();
    organization_type.hash(&mut hasher);
    rate_limit_tier.hash(&mut hasher);
    billing_type.hash(&mut hasher);
    has_subscription_created_at.hash(&mut hasher);
    let key = hasher.finish();

    let seen = SEEN.get_or_init(|| parking_lot::Mutex::new(HashSet::new()));
    if !seen.lock().insert(key) {
        return;
    }

    let payload = serde_json::json!({
        "organization_type": organization_type,
        "rate_limit_tier": rate_limit_tier,
        "billing_type": billing_type,
        "has_subscription_created_at": has_subscription_created_at,
    });
    sentry::capture_message(
        &format!("plan_tier_unknown: {payload}"),
        sentry::Level::Warning,
    );
}

fn json_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|entry| entry.as_str()))
        .map(str::to_string)
}

fn json_bool(value: &serde_json::Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|entry| entry.as_bool()))
}

fn json_datetime(value: &serde_json::Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(|entry| entry.as_str())
            .and_then(|entry| DateTime::parse_from_rfc3339(entry).ok())
            .map(|entry| entry.to_utc())
    })
}

fn remote_account_to_profile(value: RemoteAccountResponse) -> HeadroomAccountProfile {
    HeadroomAccountProfile {
        email: value.email,
        trial_started_at: value.trial_started_at,
        trial_ends_at: value.trial_ends_at,
        trial_active: value.trial_active,
        subscription_active: value.subscription_active,
        subscription_tier: value.subscription_tier,
        subscription_started_at: value.subscription_started_at,
        subscription_renews_at: value.subscription_renews_at,
        subscription_amount_cents: value.subscription_amount_cents,
        subscription_billing_period: value.subscription_billing_period,
        subscription_discount_duration: value.subscription_discount_duration,
        subscription_discount_duration_in_months: value.subscription_discount_duration_in_months,
        subscription_cancel_at_period_end: value.subscription_cancel_at_period_end,
        subscription_ends_at: value.subscription_ends_at,
        invite_code: value.invite_code,
        accepted_invites_count: value.accepted_invites_count,
        invite_bonus_percent: value.invite_bonus_percent.min(50.0).max(0.0),
    }
}

fn merge_background_account_sync(
    session_token: Option<&str>,
    sync_result: Result<RemoteAccountResponse, RemoteAccountSyncError>,
) -> (bool, Option<HeadroomAccountProfile>, Option<String>) {
    if session_token.is_none() {
        return (false, None, None);
    }

    match sync_result {
        // Background polling should not silently drop the locally stored session.
        // Explicit auth-required actions still clear the token if the server says it
        // is expired, but passive refreshes keep the user signed in locally.
        Ok(account) => (true, Some(remote_account_to_profile(account)), None),
        Err(RemoteAccountSyncError::Unauthorized) => (
            true,
            None,
            Some("Headroom account connected, but your plan details could not be refreshed. Sign in again if this keeps happening.".into()),
        ),
        Err(RemoteAccountSyncError::Other) => (
            true,
            None,
            Some("Headroom account connected, but your plan details are unavailable right now.".into()),
        ),
    }
}

fn load_or_initialize_local_state() -> Result<LocalPricingState, String> {
    let path = local_state_path();
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(state) = serde_json::from_slice::<LocalPricingState>(&bytes) {
            return Ok(state);
        }
    }

    let state = LocalPricingState {
        first_seen_at: Utc::now(),
        reconcile_with_server: true,
        mismatch_since: None,
    };
    write_local_state(&state)?;
    Ok(state)
}

fn write_local_state(state: &LocalPricingState) -> Result<(), String> {
    let path = local_state_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create pricing config directory {}: {err}",
                parent.display()
            )
        })?;
    }
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(state)
            .map_err(|err| format!("Failed to serialize pricing state: {err}"))?,
    )
    .map_err(|err| format!("Failed to write pricing state {}: {err}", path.display()))
}

fn reconcile_local_state_with_server(state: &AppState) -> Result<LocalPricingState, String> {
    let mut local = load_or_initialize_local_state()?;
    let identity = IdentityPayload::for_state(state);
    match fetch_grace_start(&identity) {
        Ok(response) => {
            // Record the fingerprint we just successfully posted so the
            // bearer-pusher worker doesn't immediately repost the same data.
            state.record_pushed_identity_fingerprint(IdentityFingerprint::from_payload(
                &identity,
            ));
            let server_first_seen = response.first_seen_at;
            let new_first_seen = if local.reconcile_with_server {
                server_first_seen.min(local.first_seen_at)
            } else {
                server_first_seen
            };
            if new_first_seen != local.first_seen_at || local.reconcile_with_server {
                local.first_seen_at = new_first_seen;
                local.reconcile_with_server = false;
                if let Err(err) = write_local_state(&local) {
                    sentry::capture_message(
                        &format!("Could not persist reconciled grace state: {err}"),
                        sentry::Level::Warning,
                    );
                }
            }
        }
        Err(_) => {
            // Server unreachable; keep whatever we have locally. reconcile_with_server
            // stays set if this is a fresh install so the next successful call wins.
        }
    }
    Ok(local)
}

fn fetch_grace_start(identity: &IdentityPayload) -> Result<GraceResponse, String> {
    let builder = http_client()?.post(api_url("desktop/grace/start"));
    let response = identity
        .apply_headers(builder)
        .json(identity)
        .send()
        .map_err(|err| format!("grace/start request failed: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "grace/start returned {}",
            response.status().as_u16()
        ));
    }

    response
        .json::<GraceResponse>()
        .map_err(|err| format!("grace/start parse failed: {err}"))
}

fn local_state_path() -> PathBuf {
    config_file(&app_data_dir(), "headroom-pricing-state.json")
}

fn read_session_token() -> Result<Option<String>, String> {
    keychain::read_secret(
        HEADROOM_ACCOUNT_KEYCHAIN_SERVICE,
        HEADROOM_ACCOUNT_SESSION_ACCOUNT,
    )
    .map(|value| value.and_then(non_empty_string))
}

fn write_session_token(token: &str) -> Result<(), String> {
    keychain::write_secret(
        HEADROOM_ACCOUNT_KEYCHAIN_SERVICE,
        HEADROOM_ACCOUNT_SESSION_ACCOUNT,
        token.trim(),
    )
}

fn clear_session_token() -> Result<(), String> {
    keychain::delete_secret(
        HEADROOM_ACCOUNT_KEYCHAIN_SERVICE,
        HEADROOM_ACCOUNT_SESSION_ACCOUNT,
    )
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicConfig {
    #[serde(default)]
    launch_discount_active: bool,
}

fn fetch_public_config() -> Option<PublicConfig> {
    let response = http_client()
        .ok()?
        .get(api_url("desktop/config"))
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<PublicConfig>().ok()
}

fn fetch_remote_account(
    token: &str,
    identity: &IdentityPayload,
) -> Result<RemoteAccountEnvelope, RemoteAccountSyncError> {
    let builder = http_client()
        .map_err(|_| RemoteAccountSyncError::Other)?
        .get(api_url("desktop/account"))
        .header("Authorization", format!("Bearer {token}"));
    let response = identity
        .apply_headers(builder)
        .send()
        .map_err(|_| RemoteAccountSyncError::Other)?;

    if response.status().as_u16() == 401 {
        return Err(RemoteAccountSyncError::Unauthorized);
    }

    if !response.status().is_success() {
        return Err(RemoteAccountSyncError::Other);
    }

    response
        .json::<RemoteAccountEnvelope>()
        .map_err(|_| RemoteAccountSyncError::Other)
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|err| format!("Could not build HTTP client: {err}"))
}

fn api_url(path: &str) -> String {
    join_url(&api_base_url(), path)
}

fn api_base_url() -> String {
    // Runtime override is only honored in debug builds. In release builds an
    // attacker with persistence on the user's machine (e.g. a launchd plist)
    // could otherwise redirect every billing/auth call to a rogue host.
    #[cfg(debug_assertions)]
    let runtime_env = std::env::var("HEADROOM_ACCOUNT_API_BASE_URL").ok();
    #[cfg(not(debug_assertions))]
    let runtime_env: Option<String> = None;

    resolve_account_api_base_url(runtime_env, option_env!("HEADROOM_ACCOUNT_API_BASE_URL"))
}

fn join_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn resolve_account_api_base_url(
    runtime_env: Option<String>,
    compile_time_env: Option<&str>,
) -> String {
    runtime_env
        .and_then(non_empty_string)
        .or_else(|| compile_time_env.and_then(|value| non_empty_string(value.to_string())))
        .unwrap_or_else(|| DEFAULT_ACCOUNT_API_BASE_URL.to_string())
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

const NUDGE_THRESHOLDS_PERCENT: [f64; 3] = [25.0, 35.0, 45.0];

#[derive(Debug, Clone)]
struct PricingPolicy {
    nudge_thresholds_percent: [f64; 3],
    disable_threshold_percent: f64,
    recommended_tier: HeadroomSubscriptionTier,
}

fn pricing_policy_for_plan(plan: &ClaudePlanTier) -> Option<PricingPolicy> {
    match plan {
        ClaudePlanTier::Free => None,
        ClaudePlanTier::Pro => Some(PricingPolicy {
            nudge_thresholds_percent: NUDGE_THRESHOLDS_PERCENT,
            disable_threshold_percent: 50.0,
            recommended_tier: HeadroomSubscriptionTier::Pro,
        }),
        ClaudePlanTier::Max5x => Some(PricingPolicy {
            nudge_thresholds_percent: NUDGE_THRESHOLDS_PERCENT,
            disable_threshold_percent: 50.0,
            recommended_tier: HeadroomSubscriptionTier::Max5x,
        }),
        ClaudePlanTier::Max20x => Some(PricingPolicy {
            nudge_thresholds_percent: NUDGE_THRESHOLDS_PERCENT,
            disable_threshold_percent: 50.0,
            recommended_tier: HeadroomSubscriptionTier::Max20x,
        }),
        ClaudePlanTier::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::{
        detect_plan_tier_from_profile, detect_tier_mismatch, evaluate_pricing_status_with_mismatch,
        is_identity_complete, merge_background_account_sync, plan_tier_header_value,
        remote_account_to_profile, resolve_account_api_base_url, ClaudeOauthProfile,
        ClaudeOauthProfileAccount, ClaudeOauthProfileOrganization, HeadroomSubscriptionTier,
        IdentityFingerprint, IdentityPayload, LocalPricingState, RemoteAccountResponse,
        RemoteAccountSyncError, DEFAULT_ACCOUNT_API_BASE_URL,
    };
    use crate::models::{
        BillingPeriod, ClaudeAccountProfile, ClaudeAuthMethod, ClaudePlanTier,
        HeadroomAccountProfile, HeadroomPricingStatus, PricingGateReason, TierMismatch,
    };

    #[allow(clippy::too_many_arguments)]
    fn evaluate_pricing_status(
        authenticated: bool,
        local_grace_started_at: DateTime<Utc>,
        local_grace_ends_at: DateTime<Utc>,
        local_grace_active: bool,
        account_sync_error: Option<String>,
        account: Option<HeadroomAccountProfile>,
        claude: ClaudeAccountProfile,
        launch_discount_active: bool,
        last_known_good_plan_tier: Option<ClaudePlanTier>,
    ) -> HeadroomPricingStatus {
        evaluate_pricing_status_with_mismatch(
            authenticated,
            local_grace_started_at,
            local_grace_ends_at,
            local_grace_active,
            account_sync_error,
            account,
            claude,
            launch_discount_active,
            last_known_good_plan_tier,
            None,
        )
    }

    fn mismatch(recommended: HeadroomSubscriptionTier, clamped: bool) -> TierMismatch {
        TierMismatch {
            paid_tier: HeadroomSubscriptionTier::Pro,
            recommended_tier: recommended,
            grace_ends_at: Utc::now(),
            clamped,
        }
    }

    fn active_subscriber(tier: HeadroomSubscriptionTier) -> HeadroomAccountProfile {
        let mut account = trial_account();
        account.trial_active = false;
        account.subscription_active = true;
        account.subscription_tier = Some(tier);
        account
    }

    fn sample_remote_account() -> RemoteAccountResponse {
        RemoteAccountResponse {
            email: "user@example.com".into(),
            trial_started_at: Some(Utc::now()),
            trial_ends_at: Some(Utc::now()),
            trial_active: true,
            subscription_active: true,
            subscription_tier: Some(HeadroomSubscriptionTier::Pro),
            subscription_started_at: None,
            subscription_renews_at: None,
            subscription_amount_cents: None,
            subscription_billing_period: None,
            subscription_discount_duration: None,
            subscription_discount_duration_in_months: None,
            subscription_cancel_at_period_end: false,
            subscription_ends_at: None,
            invite_code: Some("invite-code".into()),
            accepted_invites_count: 2,
            invite_bonus_percent: 10.0,
        }
    }

    #[test]
    fn identity_payload_serializes_with_camelcase_keys_and_skips_nulls() {
        let identity = IdentityPayload {
            device_id: "abc123".into(),
            claude_account_uuid: Some("claude-uuid".into()),
            claude_plan_tier: Some(ClaudePlanTier::Pro),
            ..Default::default()
        };
        let json = serde_json::to_value(&identity).unwrap();
        assert_eq!(json["deviceId"], "abc123");
        assert_eq!(json["claudeAccountUuid"], "claude-uuid");
        assert_eq!(json["claudePlanTier"], "pro");
        assert!(json.get("chopratejasInstanceId").is_none());
        assert!(json.get("claudeEmail").is_none());
    }

    #[test]
    fn identity_payload_skips_plan_when_none() {
        let identity = IdentityPayload {
            device_id: "abc123".into(),
            ..Default::default()
        };
        let json = serde_json::to_value(&identity).unwrap();
        assert!(json.get("claudePlanTier").is_none());
    }

    #[test]
    fn plan_tier_header_value_covers_all_variants() {
        assert_eq!(plan_tier_header_value(&ClaudePlanTier::Free), "free");
        assert_eq!(plan_tier_header_value(&ClaudePlanTier::Pro), "pro");
        assert_eq!(plan_tier_header_value(&ClaudePlanTier::Max5x), "max5x");
        assert_eq!(plan_tier_header_value(&ClaudePlanTier::Max20x), "max20x");
        assert_eq!(plan_tier_header_value(&ClaudePlanTier::Unknown), "unknown");
    }

    #[test]
    fn apply_headers_sets_plan_when_present() {
        let identity = IdentityPayload {
            device_id: "abc123".into(),
            claude_plan_tier: Some(ClaudePlanTier::Max20x),
            ..Default::default()
        };
        let client = reqwest::blocking::Client::new();
        let req = identity
            .apply_headers(client.get("http://example.test"))
            .build()
            .unwrap();
        assert_eq!(
            req.headers().get("X-Headroom-Claude-Plan").unwrap(),
            "max20x"
        );
    }

    #[test]
    fn apply_headers_sets_app_version() {
        let identity = IdentityPayload {
            device_id: "abc123".into(),
            ..Default::default()
        };
        let client = reqwest::blocking::Client::new();
        let req = identity
            .apply_headers(client.get("http://example.test"))
            .build()
            .unwrap();
        assert_eq!(
            req.headers().get("X-Headroom-App-Version").unwrap(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn apply_headers_omits_plan_when_none() {
        let identity = IdentityPayload {
            device_id: "abc123".into(),
            ..Default::default()
        };
        let client = reqwest::blocking::Client::new();
        let req = identity
            .apply_headers(client.get("http://example.test"))
            .build()
            .unwrap();
        assert!(req.headers().get("X-Headroom-Claude-Plan").is_none());
    }

    fn complete_profile() -> ClaudeAccountProfile {
        ClaudeAccountProfile {
            auth_method: ClaudeAuthMethod::ClaudeAiOauth,
            email: Some("user@example.com".into()),
            display_name: Some("User".into()),
            account_uuid: Some("uuid-1".into()),
            organization_uuid: Some("org-1".into()),
            billing_type: Some("personal".into()),
            account_created_at: None,
            subscription_created_at: None,
            has_extra_usage_enabled: false,
            plan_tier: ClaudePlanTier::Pro,
            plan_detection_source: Some("oauth_profile.org.rate_limit_tier".into()),
            organization_type: Some("claude_pro".into()),
            rate_limit_tier: Some("default_claude_ai".into()),
            weekly_utilization_pct: None,
            five_hour_utilization_pct: None,
            extra_usage_monthly_limit: None,
            profile_fetch_error: None,
        }
    }

    #[test]
    fn is_identity_complete_requires_uuid_email_and_known_plan() {
        let mut profile = complete_profile();
        assert!(is_identity_complete(&profile));

        profile.account_uuid = None;
        assert!(!is_identity_complete(&profile));

        profile = complete_profile();
        profile.email = None;
        assert!(!is_identity_complete(&profile));

        profile = complete_profile();
        profile.plan_tier = ClaudePlanTier::Unknown;
        assert!(!is_identity_complete(&profile));

        profile = complete_profile();
        profile.plan_tier = ClaudePlanTier::Free;
        assert!(is_identity_complete(&profile));
    }

    #[test]
    fn identity_fingerprint_round_trips_payload_claude_fields() {
        let payload = IdentityPayload {
            device_id: "device-abc".into(),
            chopratejas_instance_id: Some("ignored".into()),
            claude_account_uuid: Some("uuid-1".into()),
            claude_email: Some("user@example.com".into()),
            claude_plan_tier: Some(ClaudePlanTier::Max20x),
            claude_organization_type: Some("claude_max".into()),
            claude_rate_limit_tier: Some("default_claude_max_x20".into()),
            claude_billing_type: Some("personal".into()),
        };
        let fp = IdentityFingerprint::from_payload(&payload);

        // Same payload produces equal fingerprint.
        assert_eq!(fp, IdentityFingerprint::from_payload(&payload));

        // Mutating any Claude field changes the fingerprint.
        let mut other = payload.clone();
        other.claude_plan_tier = Some(ClaudePlanTier::Pro);
        assert_ne!(fp, IdentityFingerprint::from_payload(&other));

        // device_id / chopratejas_instance_id are not part of the fingerprint.
        let mut device_only_diff = payload.clone();
        device_only_diff.device_id = "different-device".into();
        device_only_diff.chopratejas_instance_id = Some("different".into());
        assert_eq!(fp, IdentityFingerprint::from_payload(&device_only_diff));
    }

    #[test]
    fn identity_fingerprint_is_empty_when_no_claude_signal_captured() {
        // Bearer-not-yet-captured shape: no UUID, plan_tier defaulted to Unknown.
        let empty_unknown = IdentityFingerprint::from_payload(&IdentityPayload {
            device_id: "abc".into(),
            claude_plan_tier: Some(ClaudePlanTier::Unknown),
            ..Default::default()
        });
        assert!(empty_unknown.is_empty());

        // Device-only payload: no plan tier at all.
        let device_only = IdentityFingerprint::from_payload(&IdentityPayload {
            device_id: "abc".into(),
            ..Default::default()
        });
        assert!(device_only.is_empty());

        // Anything with a real plan tier OR a UUID is NOT empty.
        let with_plan = IdentityFingerprint::from_payload(&IdentityPayload {
            device_id: "abc".into(),
            claude_plan_tier: Some(ClaudePlanTier::Pro),
            ..Default::default()
        });
        assert!(!with_plan.is_empty());

        let with_uuid = IdentityFingerprint::from_payload(&IdentityPayload {
            device_id: "abc".into(),
            claude_account_uuid: Some("uuid".into()),
            claude_plan_tier: Some(ClaudePlanTier::Unknown),
            ..Default::default()
        });
        assert!(!with_uuid.is_empty());
    }

    #[test]
    fn local_pricing_state_back_compat_parses_old_payload_without_reconcile_flag() {
        let raw = r#"{"first_seen_at":"2026-04-10T00:00:00Z"}"#;
        let state: LocalPricingState = serde_json::from_str(raw).unwrap();
        assert!(!state.reconcile_with_server);
    }

    #[test]
    fn runtime_env_overrides_compile_time_env() {
        let resolved = resolve_account_api_base_url(
            Some("https://runtime.example/api/v1".into()),
            Some("https://compile.example/api/v1"),
        );

        assert_eq!(resolved, "https://runtime.example/api/v1");
    }

    #[test]
    fn compile_time_env_used_when_runtime_missing() {
        let resolved = resolve_account_api_base_url(None, Some("https://compile.example/api/v1"));

        assert_eq!(resolved, "https://compile.example/api/v1");
    }

    #[test]
    fn blank_values_fall_back_to_default() {
        let resolved = resolve_account_api_base_url(Some("   ".into()), Some(" "));

        assert_eq!(resolved, DEFAULT_ACCOUNT_API_BASE_URL);
    }

    #[test]
    fn unauthorized_background_sync_keeps_local_session_authenticated() {
        let (authenticated, account, error) = merge_background_account_sync(
            Some("session-token"),
            Err(RemoteAccountSyncError::Unauthorized),
        );

        assert!(authenticated);
        assert!(account.is_none());
        assert!(error.is_some());
    }

    #[test]
    fn transient_background_sync_error_keeps_local_session_authenticated() {
        let (authenticated, account, error) = merge_background_account_sync(
            Some("session-token"),
            Err(RemoteAccountSyncError::Other),
        );

        assert!(authenticated);
        assert!(account.is_none());
        assert!(error.is_some());
    }

    #[test]
    fn successful_background_sync_returns_remote_account_profile() {
        let (authenticated, account, error) =
            merge_background_account_sync(Some("session-token"), Ok(sample_remote_account()));

        assert!(authenticated);
        assert!(error.is_none());
        assert_eq!(
            account.as_ref().map(|value| value.email.as_str()),
            Some("user@example.com")
        );
        assert!(matches!(
            account
                .as_ref()
                .and_then(|value| value.subscription_tier.clone()),
            Some(HeadroomSubscriptionTier::Pro)
        ));
    }

    #[test]
    fn release_default_points_at_production_api() {
        #[cfg(not(debug_assertions))]
        assert_eq!(
            DEFAULT_ACCOUNT_API_BASE_URL,
            "https://extraheadroom.com/api/v1"
        );
    }

    fn empty_claude_profile(plan_tier: ClaudePlanTier) -> ClaudeAccountProfile {
        ClaudeAccountProfile {
            auth_method: ClaudeAuthMethod::ClaudeAiOauth,
            email: None,
            display_name: None,
            account_uuid: None,
            organization_uuid: None,
            billing_type: None,
            account_created_at: None,
            subscription_created_at: None,
            has_extra_usage_enabled: false,
            plan_tier,
            plan_detection_source: None,
            organization_type: None,
            rate_limit_tier: None,
            weekly_utilization_pct: None,
            five_hour_utilization_pct: None,
            extra_usage_monthly_limit: None,
            profile_fetch_error: None,
        }
    }

    fn pro_profile_with_weekly(weekly: f64) -> ClaudeAccountProfile {
        let mut p = empty_claude_profile(ClaudePlanTier::Pro);
        p.weekly_utilization_pct = Some(weekly);
        p
    }

    fn unknown_profile_with_weekly(weekly: f64) -> ClaudeAccountProfile {
        let mut p = empty_claude_profile(ClaudePlanTier::Unknown);
        p.weekly_utilization_pct = Some(weekly);
        p
    }

    fn trial_account() -> HeadroomAccountProfile {
        HeadroomAccountProfile {
            email: "user@example.com".into(),
            trial_started_at: Some(Utc::now()),
            trial_ends_at: Some(Utc::now()),
            trial_active: true,
            subscription_active: false,
            subscription_tier: None,
            subscription_started_at: None,
            subscription_renews_at: None,
            subscription_amount_cents: None,
            subscription_billing_period: None,
            subscription_discount_duration: None,
            subscription_discount_duration_in_months: None,
            subscription_cancel_at_period_end: false,
            subscription_ends_at: None,
            invite_code: None,
            accepted_invites_count: 0,
            invite_bonus_percent: 0.0,
        }
    }

    fn expired_account(invite_bonus: f64) -> HeadroomAccountProfile {
        HeadroomAccountProfile {
            email: "user@example.com".into(),
            trial_started_at: None,
            trial_ends_at: None,
            trial_active: false,
            subscription_active: false,
            subscription_tier: None,
            subscription_started_at: None,
            subscription_renews_at: None,
            subscription_amount_cents: None,
            subscription_billing_period: None,
            subscription_discount_duration: None,
            subscription_discount_duration_in_months: None,
            subscription_cancel_at_period_end: false,
            subscription_ends_at: None,
            invite_code: None,
            accepted_invites_count: 0,
            invite_bonus_percent: invite_bonus,
        }
    }

    fn grace() -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc::now();
        (now, now + chrono::Duration::hours(72))
    }

    #[test]
    fn trial_active_allows_optimization_without_weekly_gating() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            true,
            None,
            Some(trial_account()),
            pro_profile_with_weekly(95.0),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(!status.should_nudge);
        assert!(status.gate_reason.is_none());
    }

    #[test]
    fn active_subscription_allows_optimization_even_over_limit() {
        let (start, end) = grace();
        let mut account = trial_account();
        account.trial_active = false;
        account.subscription_active = true;
        account.subscription_tier = Some(HeadroomSubscriptionTier::Pro);
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            true,
            None,
            Some(account),
            pro_profile_with_weekly(99.0),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(status.gate_reason.is_none());
    }

    #[test]
    fn free_tier_is_never_gated_by_weekly_usage() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            empty_claude_profile(ClaudePlanTier::Free),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(!status.should_nudge);
        assert!(status.nudge_threshold_percent.is_none());
    }

    #[test]
    fn unknown_tier_without_cache_falls_back_to_pro_thresholds() {
        // No last-known-good cache, no weekly usage signal — fallback applies
        // Pro thresholds but the user keeps optimization on (no gating yet).
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            empty_claude_profile(ClaudePlanTier::Unknown),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(!status.should_nudge);
        // Pro pricing policy is exposed even though classifier returned Unknown.
        assert_eq!(status.disable_threshold_percent, Some(50.0));
        assert!(status.gate_message.contains("Pro"));
    }

    #[test]
    fn unknown_tier_without_cache_gates_at_pro_disable_threshold() {
        // Same fallback, but weekly usage is already over Pro's 50% disable
        // threshold — the user MUST be paused, not given a free pass.
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            unknown_profile_with_weekly(60.0),
            false,
            None,
        );
        assert!(!status.optimization_allowed);
        assert!(matches!(
            status.gate_reason,
            Some(PricingGateReason::WeeklyUsageLimitReached)
        ));
    }

    #[test]
    fn unknown_tier_with_cached_max5x_uses_max5x_thresholds() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            unknown_profile_with_weekly(60.0),
            false,
            Some(ClaudePlanTier::Max5x),
        );
        assert!(!status.optimization_allowed);
        assert!(matches!(
            status.gate_reason,
            Some(PricingGateReason::WeeklyUsageLimitReached)
        ));
        assert!(matches!(
            status.recommended_subscription_tier,
            Some(HeadroomSubscriptionTier::Max5x)
        ));
    }

    #[test]
    fn unknown_tier_with_cached_pro_below_threshold_nudges_at_pro_levels() {
        // Pro nudges at 25% — confirm fallback applies the nudge, not silence.
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            unknown_profile_with_weekly(30.0),
            false,
            Some(ClaudePlanTier::Pro),
        );
        assert!(status.optimization_allowed);
        assert!(status.should_nudge);
        assert_eq!(status.nudge_threshold_percent, Some(25.0));
    }

    #[test]
    fn live_pro_classification_ignores_cached_free() {
        // Cache says Free (a stale prior signal); classifier now says Pro.
        // Live wins — Pro thresholds apply.
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            pro_profile_with_weekly(50.0),
            false,
            Some(ClaudePlanTier::Free),
        );
        assert!(!status.optimization_allowed);
        assert!(matches!(
            status.gate_reason,
            Some(PricingGateReason::WeeklyUsageLimitReached)
        ));
    }

    #[test]
    fn unknown_tier_with_cached_free_falls_back_to_pro_not_free() {
        // Free usage is granted only when the live classifier currently
        // returns Free. A cached known-good Free is stale and must not
        // re-open the no-gating path once the live signal goes Unknown.
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            unknown_profile_with_weekly(60.0),
            false,
            Some(ClaudePlanTier::Free),
        );
        assert!(!status.optimization_allowed);
        assert!(matches!(
            status.gate_reason,
            Some(PricingGateReason::WeeklyUsageLimitReached)
        ));
        assert_eq!(status.disable_threshold_percent, Some(50.0));
        assert!(status.gate_message.contains("Pro"));
    }

    #[test]
    fn unknown_tier_gate_message_names_fallback_tier() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            empty_claude_profile(ClaudePlanTier::Unknown),
            false,
            Some(ClaudePlanTier::Max20x),
        );
        assert!(status.gate_message.contains("Max x20"));
    }

    #[test]
    fn pro_below_nudge_threshold_stays_silent() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            pro_profile_with_weekly(20.0),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(!status.should_nudge);
        assert_eq!(status.nudge_level, 0);
    }

    #[test]
    fn pro_at_first_nudge_threshold_fires_level_one() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            pro_profile_with_weekly(25.0),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(status.should_nudge);
        assert_eq!(status.nudge_level, 1);
    }

    #[test]
    fn pro_at_second_nudge_threshold_fires_level_two() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            pro_profile_with_weekly(36.0),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert_eq!(status.nudge_level, 2);
    }

    #[test]
    fn pro_at_third_nudge_threshold_fires_level_three() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            pro_profile_with_weekly(46.0),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert_eq!(status.nudge_level, 3);
        // Each level uses distinct copy.
        assert!(status.gate_message.contains("upgrade"));
    }

    #[test]
    fn invite_bonus_shifts_nudge_thresholds() {
        let (start, end) = grace();
        // Pro nudges = 25/35/45; with +10 bonus -> 35/45/55. Usage 30% should
        // be silent (below shifted level 1) and disable shifts to 60%.
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(10.0)),
            pro_profile_with_weekly(30.0),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert_eq!(status.nudge_level, 0);
        assert_eq!(
            status.effective_nudge_thresholds_percent,
            Some(vec![35.0, 45.0, 55.0])
        );
    }

    #[test]
    fn pro_at_disable_threshold_gates_optimization() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            pro_profile_with_weekly(50.0),
            false,
            None,
        );
        assert!(!status.optimization_allowed);
        assert!(matches!(
            status.gate_reason,
            Some(PricingGateReason::WeeklyUsageLimitReached)
        ));
    }

    #[test]
    fn invite_bonus_raises_disable_threshold() {
        let (start, end) = grace();
        // Pro disable=50; with +10 bonus -> 60. Usage=55 should not gate.
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(10.0)),
            pro_profile_with_weekly(55.0),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(status.should_nudge);
        assert_eq!(status.effective_disable_threshold_percent, Some(60.0));
    }

    #[test]
    fn invite_bonus_is_capped_at_50_percentage_points() {
        let (start, end) = grace();
        // Even if the backend sent 200, the effective cap is +50 (so 50 + 50 = 100).
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(200.0)),
            pro_profile_with_weekly(0.0),
            false,
            None,
        );
        assert_eq!(status.effective_disable_threshold_percent, Some(100.0));
    }

    #[test]
    fn missing_weekly_usage_keeps_optimization_on_for_paid_tier() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            None,
            Some(expired_account(0.0)),
            empty_claude_profile(ClaudePlanTier::Pro),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(!status.should_nudge);
    }

    #[test]
    fn authenticated_without_account_keeps_optimization_on() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            true,
            start,
            end,
            false,
            Some("transient".into()),
            None,
            empty_claude_profile(ClaudePlanTier::Pro),
            false,
            None,
        );
        assert!(status.optimization_allowed);
        assert!(!status.needs_authentication);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn unauthenticated_without_grace_requires_sign_in() {
        let (start, end) = grace();
        let status = evaluate_pricing_status(
            false,
            start,
            end,
            false,
            None,
            None,
            empty_claude_profile(ClaudePlanTier::Pro),
            false,
            None,
        );
        assert!(status.needs_authentication);
        assert!(!status.optimization_allowed);
        assert!(matches!(
            status.gate_reason,
            Some(PricingGateReason::SignInRequired)
        ));
    }

    fn oauth_profile(
        rate_limit_tier: Option<&str>,
        organization_type: Option<&str>,
        subscription_created_at: Option<DateTime<Utc>>,
    ) -> ClaudeOauthProfile {
        ClaudeOauthProfile {
            account: ClaudeOauthProfileAccount {
                uuid: None,
                email: None,
                display_name: None,
                created_at: None,
            },
            organization: Some(ClaudeOauthProfileOrganization {
                uuid: None,
                billing_type: None,
                subscription_created_at,
                has_extra_usage_enabled: false,
                organization_type: organization_type.map(str::to_string),
                rate_limit_tier: rate_limit_tier.map(str::to_string),
            }),
        }
    }

    #[test]
    fn detect_plan_tier_rate_limit_20x_wins() {
        let p = oauth_profile(Some("claude_max_20x"), Some("claude_pro"), Some(Utc::now()));
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Max20x
        ));
    }

    #[test]
    fn detect_plan_tier_rate_limit_5x_wins() {
        let p = oauth_profile(Some("claude_max_5x"), Some("claude_pro"), Some(Utc::now()));
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Max5x
        ));
    }

    #[test]
    fn detect_plan_tier_rate_limit_x5_variant_is_max5x() {
        let p = oauth_profile(
            Some("default_claude_max_x5"),
            Some("default_claude"),
            Some(Utc::now()),
        );
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Max5x
        ));
    }

    #[test]
    fn detect_plan_tier_rate_limit_x20_variant_is_max20x() {
        let p = oauth_profile(
            Some("default_claude_max_x20"),
            Some("default_claude"),
            Some(Utc::now()),
        );
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Max20x
        ));
    }

    #[test]
    fn detect_plan_tier_default_raven_is_max20x() {
        let p = oauth_profile(
            Some("default_raven"),
            Some("claude_team"),
            Some(Utc::now()),
        );
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Max20x
        ));
    }

    #[test]
    fn detect_plan_tier_raven_substring_is_max20x() {
        let p = oauth_profile(Some("default_raven_x"), Some("claude_team"), Some(Utc::now()));
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Max20x
        ));
    }

    #[test]
    fn detect_plan_tier_default_rate_limit_with_claude_max_is_max5x() {
        let p = oauth_profile(
            Some("default_claude_ai"),
            Some("claude_max"),
            Some(Utc::now()),
        );
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Max5x
        ));
    }

    #[test]
    fn detect_plan_tier_default_rate_limit_with_claude_pro_is_pro() {
        let p = oauth_profile(
            Some("default_claude_ai"),
            Some("claude_pro"),
            Some(Utc::now()),
        );
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Pro
        ));
    }

    #[test]
    fn detect_plan_tier_organization_type_claude_free_is_free() {
        let p = oauth_profile(None, Some("claude_free"), Some(Utc::now()));
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Free
        ));
    }

    #[test]
    fn detect_plan_tier_missing_organization_is_free() {
        let p = ClaudeOauthProfile {
            account: ClaudeOauthProfileAccount {
                uuid: None,
                email: None,
                display_name: None,
                created_at: None,
            },
            organization: None,
        };
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Free
        ));
    }

    #[test]
    fn detect_plan_tier_no_subscription_created_at_is_free() {
        let p = oauth_profile(None, None, None);
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Free
        ));
    }

    #[test]
    fn detect_plan_tier_with_subscription_but_no_identifying_fields_is_unknown() {
        let p = oauth_profile(None, None, Some(Utc::now()));
        assert!(matches!(
            detect_plan_tier_from_profile(&p).0,
            ClaudePlanTier::Unknown
        ));
    }

    #[test]
    fn remote_account_clamps_invite_bonus_to_50() {
        let raw = RemoteAccountResponse {
            email: "a@b".into(),
            trial_started_at: None,
            trial_ends_at: None,
            trial_active: false,
            subscription_active: false,
            subscription_tier: None,
            subscription_started_at: None,
            subscription_renews_at: None,
            subscription_amount_cents: None,
            subscription_billing_period: None,
            subscription_discount_duration: None,
            subscription_discount_duration_in_months: None,
            subscription_cancel_at_period_end: false,
            subscription_ends_at: None,
            invite_code: None,
            accepted_invites_count: 0,
            invite_bonus_percent: 999.0,
        };
        assert_eq!(remote_account_to_profile(raw).invite_bonus_percent, 50.0);
    }

    #[test]
    fn remote_account_clamps_negative_invite_bonus_to_zero() {
        let raw = RemoteAccountResponse {
            email: "a@b".into(),
            trial_started_at: None,
            trial_ends_at: None,
            trial_active: false,
            subscription_active: false,
            subscription_tier: None,
            subscription_started_at: None,
            subscription_renews_at: None,
            subscription_amount_cents: None,
            subscription_billing_period: None,
            subscription_discount_duration: None,
            subscription_discount_duration_in_months: None,
            subscription_cancel_at_period_end: false,
            subscription_ends_at: None,
            invite_code: None,
            accepted_invites_count: 0,
            invite_bonus_percent: -10.0,
        };
        assert_eq!(remote_account_to_profile(raw).invite_bonus_percent, 0.0);
    }

    // ── Anthropic OAuth usage parser ────────────────────────────────────────

    #[test]
    fn parse_claude_usage_response_decodes_full_payload() {
        let body = serde_json::json!({
            "five_hour": {
                "utilization": 42.5,
                "resets_at": "2026-04-25T15:00:00Z"
            },
            "seven_day": {
                "utilization": 18.75,
                "resets_at": "2026-04-30T00:00:00Z"
            },
            "extra_usage": {
                "is_enabled": true,
                "monthly_limit": 50.0,
                "used_credits": 12.5,
                "utilization": 25.0
            }
        });
        let usage = super::parse_claude_usage_response(&body).expect("parse usage");

        let five = usage.five_hour.expect("five-hour window");
        assert!((five.utilization - 42.5).abs() < f64::EPSILON);

        let seven = usage.seven_day.expect("seven-day window");
        assert!((seven.utilization - 18.75).abs() < f64::EPSILON);

        let extra = usage.extra_usage.expect("extra-usage block");
        assert!(extra.is_enabled);
        assert_eq!(extra.monthly_limit, Some(50.0));
        assert_eq!(extra.used_credits, Some(12.5));
        assert_eq!(extra.utilization, Some(25.0));
    }

    #[test]
    fn parse_claude_usage_response_returns_error_on_api_error_envelope() {
        let body = serde_json::json!({
            "error": { "message": "rate limit exceeded" }
        });
        let err = super::parse_claude_usage_response(&body).expect_err("api error");
        assert!(
            err.contains("rate limit exceeded"),
            "expected rate-limit message, got: {err}"
        );
    }

    #[test]
    fn parse_claude_usage_response_returns_error_with_unknown_message_when_message_missing() {
        let body = serde_json::json!({ "error": {} });
        let err = super::parse_claude_usage_response(&body).expect_err("api error");
        assert!(err.contains("unknown"));
    }

    #[test]
    fn parse_claude_usage_response_skips_windows_missing_required_fields() {
        // Schema-drift smoke: a window object missing `resets_at` should be
        // dropped rather than producing a panic.
        let body = serde_json::json!({
            "five_hour": { "utilization": 10.0 },
            "seven_day": { "resets_at": "2026-04-30T00:00:00Z" },
            "extra_usage": null
        });
        let usage = super::parse_claude_usage_response(&body).expect("parse usage");
        assert!(usage.five_hour.is_none(), "no resets_at → window dropped");
        assert!(usage.seven_day.is_none(), "no utilization → window dropped");
        assert!(usage.extra_usage.is_none());
    }

    #[test]
    fn parse_claude_usage_response_skips_extra_usage_missing_required_field() {
        let body = serde_json::json!({
            "extra_usage": { "monthly_limit": 50.0 }  // missing is_enabled
        });
        let usage = super::parse_claude_usage_response(&body).expect("parse");
        assert!(
            usage.extra_usage.is_none(),
            "extra_usage without is_enabled should be dropped"
        );
    }

    #[test]
    fn parse_claude_usage_response_skips_window_with_malformed_resets_at() {
        let body = serde_json::json!({
            "five_hour": { "utilization": 10.0, "resets_at": "not-a-date" }
        });
        let usage = super::parse_claude_usage_response(&body).expect("parse");
        assert!(usage.five_hour.is_none());
    }

    // ── headroom-web auth contract tests ────────────────────────────────────

    fn temp_app_state() -> (crate::state::AppState, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "headroom-pricing-test-{}",
            uuid::Uuid::new_v4()
        ));
        let state = crate::state::AppState::new_in(dir.clone()).expect("app state");
        (state, dir)
    }

    fn drop_state(dir: std::path::PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn spawn_canned_response_server(
        body: serde_json::Value,
        status_line: &'static str,
    ) -> (u16, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind canned server");
        let port = listener.local_addr().unwrap().port();
        let body_bytes = body.to_string();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let response = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body_bytes.len(),
                body_bytes
            );
            let _ = stream.write_all(response.as_bytes());
        });
        (port, handle)
    }

    #[test]
    fn request_auth_code_decodes_headroom_web_response() {
        let body = serde_json::json!({
            "email": "user@example.com",
            "expiresInSeconds": 600
        });
        let (port, server) = spawn_canned_response_server(body, "HTTP/1.1 200 OK");
        let (state, dir) = temp_app_state();

        let result = super::request_auth_code_with_base_url(
            &state,
            "user@example.com",
            &format!("http://127.0.0.1:{port}"),
        )
        .expect("request_auth_code succeeds");

        server.join().unwrap();
        assert_eq!(result.email, "user@example.com");
        assert_eq!(result.expires_in_seconds, 600);
        drop_state(dir);
    }

    #[test]
    fn request_auth_code_clamps_expiry_to_documented_maximum() {
        let body = serde_json::json!({
            "email": "user@example.com",
            "expiresInSeconds": 99999
        });
        let (port, server) = spawn_canned_response_server(body, "HTTP/1.1 200 OK");
        let (state, dir) = temp_app_state();

        let result = super::request_auth_code_with_base_url(
            &state,
            "user@example.com",
            &format!("http://127.0.0.1:{port}"),
        )
        .expect("request_auth_code succeeds");

        server.join().unwrap();
        assert_eq!(
            result.expires_in_seconds,
            super::AUTH_CODE_EXPIRY_SECONDS,
            "expiry clamped to documented maximum"
        );
        drop_state(dir);
    }

    #[test]
    fn request_auth_code_rejects_invalid_email_before_calling_server() {
        let (state, dir) = temp_app_state();
        let result = super::request_auth_code_with_base_url(
            &state,
            "  ",
            "http://127.0.0.1:1", // would fail if reached
        );
        assert!(matches!(result, Err(msg) if msg.contains("valid email")));
        drop_state(dir);
    }

    #[test]
    fn request_auth_code_returns_error_on_5xx_response() {
        let body = serde_json::json!({"error": "internal"});
        let (port, server) = spawn_canned_response_server(body, "HTTP/1.1 500 Internal Server Error");
        let (state, dir) = temp_app_state();

        let err = super::request_auth_code_with_base_url(
            &state,
            "user@example.com",
            &format!("http://127.0.0.1:{port}"),
        )
        .expect_err("5xx surfaces as error");

        server.join().unwrap();
        assert!(err.contains("status 500"));
        drop_state(dir);
    }

    #[test]
    #[serial_test::serial]
    fn verify_auth_code_decodes_and_writes_session_token() {
        // Override HOME / XDG_DATA_HOME so the keychain debug store and
        // app_data_dir live in a fresh tempdir, not the dev's real profile.
        let prev_home = std::env::var_os("HOME");
        let prev_xdg = std::env::var_os("XDG_DATA_HOME");
        let scratch = tempfile::tempdir().expect("scratch tempdir");
        std::env::set_var("HOME", scratch.path());
        std::env::set_var(
            "XDG_DATA_HOME",
            scratch.path().join(".local").join("share"),
        );
        crate::storage::ensure_data_dirs(&crate::storage::app_data_dir())
            .expect("ensure_data_dirs in scratch");

        let body = serde_json::json!({
            "sessionToken": "session-xyz",
            "account": {
                "email": "user@example.com",
                "trialStartedAt": "2026-04-01T00:00:00Z",
                "trialEndsAt": "2026-04-15T00:00:00Z",
                "trialActive": true,
                "subscriptionActive": false,
                "subscriptionTier": null,
                "inviteCode": null,
                "acceptedInvitesCount": 0,
                "inviteBonusPercent": 0
            },
            "launchDiscountActive": false
        });
        let (port, server) = spawn_canned_response_server(body, "HTTP/1.1 200 OK");
        let (state, dir) = temp_app_state();

        let result = super::verify_auth_code_with_base_url(
            &state,
            "user@example.com",
            "123456",
            None,
            &format!("http://127.0.0.1:{port}"),
        )
        .expect("verify_auth_code succeeds");

        server.join().unwrap();
        assert!(result.authenticated);
        let account = result.account.expect("account profile populated");
        assert_eq!(account.email, "user@example.com");
        assert!(account.trial_active);

        // Session token should have been written to the (debug) keychain.
        let stored = crate::keychain::read_secret(
            super::HEADROOM_ACCOUNT_KEYCHAIN_SERVICE,
            super::HEADROOM_ACCOUNT_SESSION_ACCOUNT,
        )
        .expect("read session token");
        assert_eq!(stored.as_deref(), Some("session-xyz"));

        drop_state(dir);
        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }

    #[test]
    fn verify_auth_code_rejects_blank_code_before_hitting_server() {
        let (state, dir) = temp_app_state();
        let err = super::verify_auth_code_with_base_url(
            &state,
            "user@example.com",
            "   ",
            None,
            "http://127.0.0.1:1",
        )
        .expect_err("blank code rejected");
        assert!(err.contains("authentication code"));
        drop_state(dir);
    }

    // ── activate_account / create_checkout_session / get_billing_portal_url ─

    /// Snapshot HOME / XDG_DATA_HOME, redirect them at a fresh tempdir,
    /// ensure_data_dirs, and seed a session token in the (debug) keychain
    /// so authenticated functions don't bail at the read_session_token step.
    /// Returns a guard whose Drop restores the original env vars.
    struct AuthedTestEnv {
        _scratch: tempfile::TempDir,
        prev_home: Option<std::ffi::OsString>,
        prev_xdg: Option<std::ffi::OsString>,
    }

    impl AuthedTestEnv {
        fn new(session_token: &str) -> Self {
            let scratch = tempfile::tempdir().expect("scratch tempdir");
            let prev_home = std::env::var_os("HOME");
            let prev_xdg = std::env::var_os("XDG_DATA_HOME");
            std::env::set_var("HOME", scratch.path());
            std::env::set_var(
                "XDG_DATA_HOME",
                scratch.path().join(".local").join("share"),
            );
            crate::storage::ensure_data_dirs(&crate::storage::app_data_dir())
                .expect("ensure_data_dirs in scratch");
            crate::keychain::write_secret(
                super::HEADROOM_ACCOUNT_KEYCHAIN_SERVICE,
                super::HEADROOM_ACCOUNT_SESSION_ACCOUNT,
                session_token,
            )
            .expect("seed session token");
            AuthedTestEnv {
                _scratch: scratch,
                prev_home,
                prev_xdg,
            }
        }
    }

    impl Drop for AuthedTestEnv {
        fn drop(&mut self) {
            match self.prev_home.take() {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match self.prev_xdg.take() {
                Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }
    }

    fn sample_account_envelope_body() -> serde_json::Value {
        serde_json::json!({
            "account": {
                "email": "user@example.com",
                "trialStartedAt": "2026-04-01T00:00:00Z",
                "trialEndsAt": "2026-04-15T00:00:00Z",
                "trialActive": true,
                "subscriptionActive": false,
                "subscriptionTier": null,
                "inviteCode": null,
                "acceptedInvitesCount": 0,
                "inviteBonusPercent": 0
            },
            "launchDiscountActive": false
        })
    }

    #[test]
    #[serial_test::serial]
    fn activate_account_decodes_remote_envelope_and_returns_pricing_status() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) = spawn_canned_response_server(
            sample_account_envelope_body(),
            "HTTP/1.1 200 OK",
        );
        let (state, dir) = temp_app_state();

        let result = super::activate_account_with_base_url(
            &state,
            42,
            &format!("http://127.0.0.1:{port}"),
        )
        .expect("activate_account succeeds");

        server.join().unwrap();
        assert!(result.authenticated);
        let account = result.account.expect("account profile populated");
        assert_eq!(account.email, "user@example.com");
        assert!(account.trial_active);
        drop_state(dir);
    }

    #[test]
    #[serial_test::serial]
    fn activate_account_clears_session_and_returns_expired_error_on_401() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) =
            spawn_canned_response_server(serde_json::json!({}), "HTTP/1.1 401 Unauthorized");
        let (state, dir) = temp_app_state();

        let err = super::activate_account_with_base_url(
            &state,
            0,
            &format!("http://127.0.0.1:{port}"),
        )
        .expect_err("401 surfaces as expired session");
        server.join().unwrap();
        assert!(err.contains("session expired"));

        // Session token should be cleared after 401.
        let stored = crate::keychain::read_secret(
            super::HEADROOM_ACCOUNT_KEYCHAIN_SERVICE,
            super::HEADROOM_ACCOUNT_SESSION_ACCOUNT,
        )
        .expect("read after 401");
        assert!(stored.is_none(), "session token cleared after 401");

        drop_state(dir);
    }

    #[test]
    #[serial_test::serial]
    fn activate_account_requires_session_token() {
        // No AuthedTestEnv → no token in keychain. Override HOME so any
        // keychain read still goes to a tempdir, not the dev profile.
        let scratch = tempfile::tempdir().expect("scratch");
        let prev_home = std::env::var_os("HOME");
        let prev_xdg = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("HOME", scratch.path());
        std::env::set_var(
            "XDG_DATA_HOME",
            scratch.path().join(".local").join("share"),
        );
        crate::storage::ensure_data_dirs(&crate::storage::app_data_dir()).unwrap();
        let (state, dir) = temp_app_state();

        let err = super::activate_account_with_base_url(&state, 0, "http://127.0.0.1:1")
            .expect_err("no session → error");
        assert!(err.contains("Sign in"));

        drop_state(dir);
        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn create_checkout_session_returns_url_from_response() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) = spawn_canned_response_server(
            serde_json::json!({ "url": "https://buy.polar.sh/abc123" }),
            "HTTP/1.1 200 OK",
        );

        let url = super::create_checkout_session_with_base_url(
            HeadroomSubscriptionTier::Pro,
            BillingPeriod::Annual,
            &format!("http://127.0.0.1:{port}"),
        )
        .expect("checkout session succeeds");
        server.join().unwrap();

        assert_eq!(url, "https://buy.polar.sh/abc123");
    }

    #[test]
    #[serial_test::serial]
    fn create_checkout_session_surfaces_api_error_message() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) = spawn_canned_response_server(
            serde_json::json!({ "error": "Plan unavailable in your region" }),
            "HTTP/1.1 400 Bad Request",
        );

        let err = super::create_checkout_session_with_base_url(
            HeadroomSubscriptionTier::Pro,
            BillingPeriod::Annual,
            &format!("http://127.0.0.1:{port}"),
        )
        .expect_err("4xx surfaces as error");
        server.join().unwrap();

        assert_eq!(err, "Plan unavailable in your region");
    }

    #[test]
    #[serial_test::serial]
    fn create_checkout_session_falls_back_to_status_message_when_no_api_error() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) =
            spawn_canned_response_server(serde_json::json!({}), "HTTP/1.1 502 Bad Gateway");

        let err = super::create_checkout_session_with_base_url(
            HeadroomSubscriptionTier::Pro,
            BillingPeriod::Annual,
            &format!("http://127.0.0.1:{port}"),
        )
        .expect_err("error");
        server.join().unwrap();

        assert!(err.contains("status 502"));
    }

    #[test]
    #[serial_test::serial]
    fn create_checkout_session_clears_session_on_401() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) =
            spawn_canned_response_server(serde_json::json!({}), "HTTP/1.1 401 Unauthorized");

        let err = super::create_checkout_session_with_base_url(
            HeadroomSubscriptionTier::Pro,
            BillingPeriod::Annual,
            &format!("http://127.0.0.1:{port}"),
        )
        .expect_err("401");
        server.join().unwrap();
        assert!(err.contains("session expired"));

        let stored = crate::keychain::read_secret(
            super::HEADROOM_ACCOUNT_KEYCHAIN_SERVICE,
            super::HEADROOM_ACCOUNT_SESSION_ACCOUNT,
        )
        .unwrap();
        assert!(stored.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn reactivate_subscription_succeeds_on_200() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) = spawn_canned_response_server(
            serde_json::json!({ "ok": true }),
            "HTTP/1.1 200 OK",
        );

        super::reactivate_subscription_with_base_url(&format!("http://127.0.0.1:{port}"))
            .expect("reactivate succeeds");
        server.join().unwrap();
    }

    #[test]
    #[serial_test::serial]
    fn reactivate_subscription_surfaces_api_error_message() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) = spawn_canned_response_server(
            serde_json::json!({ "error": "Subscription is not scheduled for cancellation." }),
            "HTTP/1.1 422 Unprocessable Entity",
        );

        let err = super::reactivate_subscription_with_base_url(&format!(
            "http://127.0.0.1:{port}"
        ))
        .expect_err("4xx surfaces as error");
        server.join().unwrap();
        assert_eq!(err, "Subscription is not scheduled for cancellation.");
    }

    #[test]
    #[serial_test::serial]
    fn reactivate_subscription_clears_session_on_401() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) =
            spawn_canned_response_server(serde_json::json!({}), "HTTP/1.1 401 Unauthorized");

        let err = super::reactivate_subscription_with_base_url(&format!(
            "http://127.0.0.1:{port}"
        ))
        .expect_err("401");
        server.join().unwrap();
        assert!(err.contains("session expired"));

        let stored = crate::keychain::read_secret(
            super::HEADROOM_ACCOUNT_KEYCHAIN_SERVICE,
            super::HEADROOM_ACCOUNT_SESSION_ACCOUNT,
        )
        .unwrap();
        assert!(stored.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn get_billing_portal_url_returns_url_from_response() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) = spawn_canned_response_server(
            serde_json::json!({ "url": "https://billing.polar.sh/customer/abc" }),
            "HTTP/1.1 200 OK",
        );

        let url = super::get_billing_portal_url_with_base_url(
            &format!("http://127.0.0.1:{port}"),
            None,
        )
        .expect("billing portal succeeds");
        server.join().unwrap();

        assert_eq!(url, "https://billing.polar.sh/customer/abc");
    }

    #[test]
    #[serial_test::serial]
    fn get_billing_portal_url_surfaces_api_error_message() {
        let _env = AuthedTestEnv::new("session-xyz");
        let (port, server) = spawn_canned_response_server(
            serde_json::json!({ "error": "Customer not found" }),
            "HTTP/1.1 404 Not Found",
        );

        let err = super::get_billing_portal_url_with_base_url(
            &format!("http://127.0.0.1:{port}"),
            None,
        )
        .expect_err("404 surfaces as error");
        server.join().unwrap();

        assert_eq!(err, "Customer not found");
    }

    #[test]
    fn detect_tier_mismatch_flags_under_subscribed_pro() {
        let account = active_subscriber(HeadroomSubscriptionTier::Pro);
        let claude = empty_claude_profile(ClaudePlanTier::Max20x);
        assert_eq!(
            detect_tier_mismatch(&account, &claude),
            Some((HeadroomSubscriptionTier::Pro, HeadroomSubscriptionTier::Max20x))
        );
    }

    #[test]
    fn detect_tier_mismatch_ignores_matching_or_higher_paid_tier() {
        let claude = empty_claude_profile(ClaudePlanTier::Pro);
        // Equal tiers.
        assert!(detect_tier_mismatch(&active_subscriber(HeadroomSubscriptionTier::Pro), &claude)
            .is_none());
        // Paid higher than Claude plan.
        assert!(detect_tier_mismatch(&active_subscriber(HeadroomSubscriptionTier::Max20x), &claude)
            .is_none());
    }

    #[test]
    fn detect_tier_mismatch_requires_confident_paid_claude_plan() {
        let account = active_subscriber(HeadroomSubscriptionTier::Pro);
        // Unknown and Free carry no recommended paid tier.
        assert!(
            detect_tier_mismatch(&account, &empty_claude_profile(ClaudePlanTier::Unknown)).is_none()
        );
        assert!(
            detect_tier_mismatch(&account, &empty_claude_profile(ClaudePlanTier::Free)).is_none()
        );
    }

    #[test]
    fn detect_tier_mismatch_ignores_inactive_subscription() {
        let mut account = active_subscriber(HeadroomSubscriptionTier::Pro);
        account.subscription_active = false;
        let claude = empty_claude_profile(ClaudePlanTier::Max20x);
        assert!(detect_tier_mismatch(&account, &claude).is_none());
    }

    #[test]
    fn within_grace_mismatch_keeps_optimization_unlimited() {
        let (start, end) = grace();
        let status = evaluate_pricing_status_with_mismatch(
            true,
            start,
            end,
            true,
            None,
            Some(active_subscriber(HeadroomSubscriptionTier::Pro)),
            pro_profile_with_weekly(99.0),
            false,
            None,
            Some(mismatch(HeadroomSubscriptionTier::Max20x, false)),
        );
        assert!(status.optimization_allowed);
        assert!(!status.should_nudge);
        assert!(status.tier_mismatch.is_some());
        assert_eq!(
            status.recommended_subscription_tier,
            Some(HeadroomSubscriptionTier::Max20x)
        );
    }

    #[test]
    fn clamped_mismatch_applies_standard_usage_gate() {
        let (start, end) = grace();
        let status = evaluate_pricing_status_with_mismatch(
            true,
            start,
            end,
            true,
            None,
            Some(active_subscriber(HeadroomSubscriptionTier::Pro)),
            pro_profile_with_weekly(99.0),
            false,
            None,
            Some(mismatch(HeadroomSubscriptionTier::Max20x, true)),
        );
        // Over the disable threshold, the standard paid gate pauses optimization.
        assert!(!status.optimization_allowed);
        assert!(matches!(
            status.gate_reason,
            Some(PricingGateReason::WeeklyUsageLimitReached)
        ));
        assert!(status.tier_mismatch.is_some_and(|m| m.clamped));
    }

}
