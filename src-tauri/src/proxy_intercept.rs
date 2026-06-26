/// Transparent HTTP proxy intercept layer.
///
/// Binds on 127.0.0.1:6767 (the address clients point at) and forwards every
/// request unchanged to 127.0.0.1:<backend_port>, where headroom actually
/// listens. The backend port is normally 6768 but is selected at proxy spawn
/// time and stored in `crate::backend_port`; it can shift to 6769..=6790 if
/// 6768 is held by a foreign process. We re-read the port per connection so
/// the intercept (which spawns before proxy startup runs the selection) picks
/// up the chosen value as soon as it's set.
///
/// As each request passes through, any `Authorization: Bearer …` header is
/// captured into `AppState::claude_bearer_token` so the usage-stats feature
/// can call the Anthropic OAuth usage endpoint without touching the keychain.
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use base64::Engine;

use crate::backend_port;
use crate::bearer::{BearerToken, BEARER_TOKEN_TTL};
use crate::models::{CodexPlanTier, CodexRateLimitSnapshot, CodexUsageWindow};

pub const INTERCEPT_PORT: u16 = 6767;

const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_HEADER_BYTES: usize = 64 * 1024;
const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(100);
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Dedicated Codex subscription-usage endpoint (ChatGPT OAuth/session auth).
/// Current Codex no longer ships `x-codex-*` on the `/responses` handshake, so
/// this is the only source left for the desktop gauge's rate-limit window.
const CODEX_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CODEX_USAGE_POLL_MIN_INTERVAL_SECS: u64 = 60;
const CODEX_USAGE_POLL_TIMEOUT: Duration = Duration::from_secs(10);
/// Epoch-seconds of the last usage-poll attempt; throttles the fire-and-forget
/// GET to at most one per `CODEX_USAGE_POLL_MIN_INTERVAL_SECS`.
static CODEX_USAGE_LAST_POLL: AtomicU64 = AtomicU64::new(0);

/// Shared state written by the intercept layer.
pub type SharedToken = Arc<Mutex<Option<BearerToken>>>;

/// Latest Codex rate-limit snapshot captured from `x-codex-*` response headers.
/// Shared with `AppState::codex_rate_limits`; read by `pricing::fetch_codex_usage`.
pub type CodexRateLimitSlot = Arc<Mutex<Option<CodexRateLimitSnapshot>>>;

/// When set to `true`, the intercept forwards traffic directly to
/// api.anthropic.com instead of the local Python proxy. Used to keep already-
/// running Claude Code sessions alive after the pricing gate has stopped the
/// Python proxy because the user crossed the free disable threshold.
pub type BypassFlag = Arc<AtomicBool>;

/// Shared with `AppState::codex_plan_tier`; populated from the Codex OAuth bearer
/// JWT and read by `pricing::fetch_codex_usage` to pick the recommended tier.
pub type CodexPlanSlot = Arc<Mutex<Option<crate::models::CodexPlanTier>>>;

/// Channel sender used to notify a background worker that the intercept just
/// captured a bearer token whose value differs from whatever was previously
/// in the slot. Empty payload — the worker reads the bearer from `AppState`
/// directly. Cloned per-connection in `run`.
pub type FreshBearerNotifier = mpsc::Sender<()>;

pub const ANTHROPIC_DIRECT_BASE: &str = "https://api.anthropic.com";
pub const OPENAI_DIRECT_BASE: &str = "https://api.openai.com";

/// Spawn the intercept proxy as a background Tokio task.
/// Returns immediately; the server runs until the process exits.
/// Uses a dedicated OS thread with its own Tokio runtime so it's safe to call
/// from Tauri's `.setup()` before the main async runtime has started.
pub fn spawn(
    token_slot: SharedToken,
    codex_slot: CodexRateLimitSlot,
    codex_plan_slot: CodexPlanSlot,
    bypass: BypassFlag,
    codex_bypass: BypassFlag,
    fresh_bearer_tx: FreshBearerNotifier,
) {
    let upstream_base = Arc::new(ANTHROPIC_DIRECT_BASE.to_string());
    std::thread::Builder::new()
        .name("proxy-intercept".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("proxy intercept runtime");
            rt.block_on(async move {
                let bind_addr: SocketAddr = ([127, 0, 0, 1], INTERCEPT_PORT).into();
                match run(
                    bind_addr,
                    token_slot,
                    codex_slot,
                    codex_plan_slot,
                    bypass,
                    codex_bypass,
                    fresh_bearer_tx,
                    upstream_base,
                )
                .await
                {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                        // Port is already bound. If /health responds over HTTP, an
                        // existing Headroom proxy owns the port (single-instance
                        // plugin should normally prevent this, but a crashed or
                        // still-exiting prior process can leave it held). Treat
                        // that as benign. Otherwise the port is foreign and we
                        // escalate to Sentry.
                        if probe_existing_intercept().await {
                            log::info!(
                                "[proxy_intercept] port {INTERCEPT_PORT} already owned by existing Headroom proxy; exiting thread"
                            );
                        } else {
                            log::debug!(
                                "[proxy_intercept] fatal: {e} (port {INTERCEPT_PORT} held by foreign process)"
                            );
                            sentry::capture_message(
                                &format!(
                                    "proxy_intercept fatal error: {e} (port {INTERCEPT_PORT} held by foreign process)"
                                ),
                                sentry::Level::Fatal,
                            );
                        }
                    }
                    Err(e) => {
                        log::debug!("[proxy_intercept] fatal: {e}");
                        sentry::capture_message(
                            &format!("proxy_intercept fatal error: {e}"),
                            sentry::Level::Fatal,
                        );
                    }
                }
            });
        })
        .expect("spawn proxy intercept thread");
}

async fn run(
    bind_addr: SocketAddr,
    token_slot: SharedToken,
    codex_slot: CodexRateLimitSlot,
    codex_plan_slot: CodexPlanSlot,
    bypass: BypassFlag,
    codex_bypass: BypassFlag,
    fresh_bearer_tx: FreshBearerNotifier,
    upstream_base: Arc<String>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;

    loop {
        match listener.accept().await {
            Ok((client, _)) => {
                let slot = token_slot.clone();
                let codex_slot = codex_slot.clone();
                let codex_plan_slot = codex_plan_slot.clone();
                let bypass = bypass.clone();
                let codex_bypass = codex_bypass.clone();
                let upstream_base = upstream_base.clone();
                let tx = fresh_bearer_tx.clone();
                tokio::spawn(handle(
                    client,
                    slot,
                    codex_slot,
                    codex_plan_slot,
                    bypass,
                    codex_bypass,
                    tx,
                    upstream_base,
                ));
            }
            Err(e) => {
                // EMFILE/ENFILE/ECONNABORTED are transient — log and keep serving
                // so the proxy self-heals once FDs free up, instead of dying.
                log::warn!("[proxy_intercept] accept error: {e}");
                tokio::time::sleep(ACCEPT_ERROR_BACKOFF).await;
            }
        }
    }
}

/// Returns `true` when `candidate` differs from whatever fresh bearer is
/// already in `slot`. An empty slot or a slot whose previous bearer has
/// aged out of `BEARER_TOKEN_TTL` both count as "changed" — the worker
/// should re-confirm identity in either case.
fn bearer_value_changed(slot: &SharedToken, candidate: &str) -> bool {
    let lock = slot.lock();
    lock.as_ref()
        .and_then(|t| t.value_if_fresh(BEARER_TOKEN_TTL))
        .map(|v| v != candidate)
        .unwrap_or(true)
}

#[allow(clippy::too_many_arguments)]
async fn handle(
    mut client: TcpStream,
    token_slot: SharedToken,
    codex_slot: CodexRateLimitSlot,
    codex_plan_slot: CodexPlanSlot,
    bypass: BypassFlag,
    codex_bypass: BypassFlag,
    fresh_bearer_tx: FreshBearerNotifier,
    upstream_base: Arc<String>,
) {
    // Re-read the backend port on each connection. `tool_manager` selects the
    // port (and may switch to a fallback) when the proxy spawn runs, which
    // happens after this thread is already accepting; reading per-connection
    // means existing clients pick up the chosen port without restarting.
    let backend_addr: SocketAddr = ([127, 0, 0, 1], backend_port::get()).into();
    // Read only through the end of the HTTP headers. We only need headers to
    // capture the bearer token, and forwarding early avoids deadlocks with
    // `Expect: 100-continue` request flows.
    let mut buf = Vec::with_capacity(4096);
    match tokio::time::timeout(
        HEADER_READ_TIMEOUT,
        read_http_headers(&mut client, &mut buf),
    )
    .await
    {
        Ok(Ok(())) => {}
        _ => return,
    }

    // Reject requests that didn't target the loopback listener or that carry
    // a browser Origin. This blocks DNS-rebinding attacks where an attacker
    // page resolves its hostname to 127.0.0.1 and drives the intercept from
    // a user's browser; CLI clients never set Origin and always send a
    // loopback Host.
    if !request_is_loopback_safe(&buf) {
        let _ = client
            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            .await;
        return;
    }

    // Whether this is a Codex (OpenAI-path) request. Parsed once here and
    // reused for the Codex plan capture, the Codex-only bypass, and the
    // response-head sniff below.
    let is_codex = find_header_end(&buf)
        .and_then(|end| parse_request_head(&buf[..end + 4]))
        .is_some_and(|head| is_openai_path(&head.path));

    // Scan headers for a Bearer token and capture it. When the token's
    // value differs from what was previously in the slot — or the slot was
    // empty / its previous token has aged out of the TTL — signal the
    // identity-pusher worker so it can re-confirm the user's Claude
    // identity with headroom-web. The send is non-blocking; the actual
    // OAuth-profile fetch happens off the request hot path.
    if let Some(token) = extract_bearer(&buf) {
        let changed = bearer_value_changed(&token_slot, &token);
        // For Codex requests the bearer is an OpenAI OAuth JWT carrying the
        // ChatGPT plan; decode it so the Codex gate can recommend a tier.
        if is_codex {
            if let Some(tier) = decode_codex_plan_tier(&token) {
                *codex_plan_slot.lock() = Some(tier);
            }
        }
        *token_slot.lock() = Some(BearerToken::new(token));
        if changed {
            let _ = fresh_bearer_tx.send(());
        }
    }

    // The current Codex WS handshake no longer carries `x-codex-*` response
    // headers, so `splice_with_codex_capture` below comes up empty. Fetch the
    // live subscription window from the dedicated usage endpoint instead.
    // Throttled and fire-and-forget, so the request hot path is untouched.
    if is_codex {
        maybe_spawn_codex_usage_poll(&buf, &codex_slot);
    }

    // When the pricing gate has bypassed Headroom, the Python proxy on
    // `backend_addr` is intentionally stopped. Forward direct to Anthropic so
    // already-running CC sessions stay alive while optimization is off.
    if bypass.load(Ordering::Acquire) {
        forward_direct_to_anthropic(client, buf, &upstream_base).await;
        return;
    }

    // Codex-only gate: when a free user has crossed the weekly Codex limit,
    // forward Codex traffic straight to OpenAI (unoptimized) while leaving the
    // Python backend up for Claude. `forward_direct_to_anthropic` routes
    // OpenAI paths to OPENAI_DIRECT_BASE, so it does the right thing here.
    if is_codex && codex_bypass.load(Ordering::Acquire) {
        forward_direct_to_anthropic(client, buf, &upstream_base).await;
        return;
    }

    // Forward to the headroom backend.
    let Ok(mut backend) = TcpStream::connect(backend_addr).await else {
        // headroom not up yet — send a 502 so the client gets a clean error.
        let _ = client
            .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n")
            .await;
        return;
    };

    // Codex GUI/IDE clients don't send a `codex-cli/` User-Agent, so the
    // backend's UA-based classifier can't tell they're Codex and treats a
    // compression timeout as a fail-closed HTTP 413 instead of taking the
    // codex fail-open path. Codex treats that 413 as a hard connection failure
    // and stops connecting. We already know by request path that this is Codex
    // traffic, so stamp `X-Client: codex` (which the backend honours over the
    // User-Agent) to keep Codex GUI and Codex CLI on the same backend path.
    if is_codex {
        stamp_codex_client_header(&mut buf);
    }

    if backend.write_all(&buf).await.is_err() {
        return;
    }

    // For Codex (OpenAI) requests, sniff the backend response head so we can
    // capture the `x-codex-*` rate-limit headers that feed the usage gauge.
    // Codex always streams, so the Python backend's own capture (non-streaming
    // only) never fires for it — this proxy is the only component left in the
    // response path that sees those headers. Every other client (Claude) keeps
    // the untouched zero-copy splice.
    if is_codex {
        splice_with_codex_capture(client, backend, &codex_slot, &codex_bypass).await;
    } else {
        let _ = tokio::io::copy_bidirectional(&mut client, &mut backend).await;
    }
}

/// Splice client <-> backend while sniffing the backend's response head for
/// `x-codex-*` rate-limit headers. Only the response head is read up-front (the
/// body/SSE bytes that follow are spliced through verbatim), so streaming
/// responses are neither buffered nor delayed beyond their header block. On any
/// read error before the head completes, whatever was read is still forwarded,
/// so the response is never corrupted.
async fn splice_with_codex_capture(
    mut client: TcpStream,
    mut backend: TcpStream,
    codex_slot: &CodexRateLimitSlot,
    codex_bypass: &BypassFlag,
) {
    let (mut client_rd, mut client_wr) = client.split();
    let (mut backend_rd, mut backend_wr) = backend.split();

    // client -> backend: opaque copy (request body / pipelined requests).
    let upstream = async {
        let _ = tokio::io::copy(&mut client_rd, &mut backend_wr).await;
        let _ = backend_wr.shutdown().await;
    };

    // backend -> client: capture the response head, then stream the remainder.
    let downstream = async {
        let mut head = Vec::with_capacity(4096);
        let read_head = tokio::time::timeout(
            HEADER_READ_TIMEOUT,
            read_http_headers(&mut backend_rd, &mut head),
        )
        .await;

        if matches!(read_head, Ok(Ok(()))) {
            if let Some(snapshot) = parse_codex_rate_limit_headers(&head) {
                *codex_slot.lock() = Some(snapshot);
            }
            if is_headroom_compression_refusal_response(&head) {
                log::warn!(
"Headroom refused Codex compression for an oversized request; enabling Codex direct bypass for subsequent requests"
);
                codex_bypass.store(true, Ordering::Release);
            }
        }

        // Forward the head bytes we read (full head on success, partial on
        // timeout/EOF — `read_http_headers` may also include leading body bytes
        // it over-read), then splice the rest of the response through.
        if client_wr.write_all(&head).await.is_err() {
            return;
        }
        let _ = tokio::io::copy(&mut backend_rd, &mut client_wr).await;
        let _ = client_wr.shutdown().await;
    };

    tokio::join!(upstream, downstream);
}

fn is_headroom_compression_refusal_response(head: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(head) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    lower.starts_with("http/1.1 413")
        && lower.contains("compression_refused")
        && lower.contains("headroom:")
        && lower.contains("compression timeout")
}

/// Parse the `x-codex-*` rate-limit headers out of a raw HTTP response head
/// (status line + headers up to the blank line). Mirrors the schema in upstream
/// `headroom/subscription/codex_rate_limits.py`. Returns `None` when there is no
/// usable signal (no windows and no credits balance).
fn parse_codex_rate_limit_headers(head: &[u8]) -> Option<CodexRateLimitSnapshot> {
    let text = std::str::from_utf8(head).ok()?;

    let mut headers: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for line in text.split("\r\n").skip(1) {
        if line.is_empty() {
            break; // end of header block
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let parse_window = |prefix: &str| -> Option<CodexUsageWindow> {
        let used_percent: f64 = headers
            .get(&format!("x-codex-{prefix}-used-percent"))?
            .parse()
            .ok()?;
        let window_minutes = headers
            .get(&format!("x-codex-{prefix}-window-minutes"))
            .and_then(|v| v.parse::<i64>().ok());
        let reset_at = headers
            .get(&format!("x-codex-{prefix}-reset-at"))
            .and_then(|v| v.parse::<i64>().ok());
        Some(CodexUsageWindow {
            used_percent,
            window_label: window_minutes.map(codex_window_label),
            window_minutes,
            seconds_until_reset: reset_at.map(|r| (r - now).max(0)),
        })
    };

    let primary = parse_window("primary");
    let secondary = parse_window("secondary");
    let credits_balance = headers
        .get("x-codex-credits-balance")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let credits_unlimited = headers
        .get("x-codex-credits-unlimited")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);
    let limit_name = headers
        .get("x-codex-limit-name")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if primary.is_none() && secondary.is_none() && credits_balance.is_none() {
        return None;
    }

    Some(CodexRateLimitSnapshot {
        limit_name,
        primary,
        secondary,
        credits_balance,
        credits_unlimited,
    })
}

fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Extract a single request header value (case-insensitive) from raw HTTP bytes.
fn extract_header_value(buf: &[u8], name: &str) -> Option<String> {
    let text = std::str::from_utf8(buf).ok()?;
    for line in text.lines() {
        if line.is_empty() {
            break; // end of header block
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case(name) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

// Subset of the `GET /wham/usage` JSON body we map onto a snapshot. Unknown
// fields are ignored by serde.
#[derive(serde::Deserialize)]
struct UsageWindowJson {
    used_percent: Option<f64>,
    limit_window_seconds: Option<i64>,
    reset_at: Option<i64>,
}

#[derive(serde::Deserialize)]
struct UsageRateLimitJson {
    primary_window: Option<UsageWindowJson>,
    secondary_window: Option<UsageWindowJson>,
}

#[derive(serde::Deserialize)]
struct UsageCreditsJson {
    has_credits: Option<bool>,
    unlimited: Option<bool>,
    balance: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct UsagePayloadJson {
    rate_limit: Option<UsageRateLimitJson>,
    credits: Option<UsageCreditsJson>,
    rate_limit_reached_type: Option<String>,
}

fn balance_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn codex_window_from_usage(win: &UsageWindowJson, now: i64) -> Option<CodexUsageWindow> {
    let used_percent = win.used_percent?;
    if used_percent.is_nan() {
        return None;
    }
    // Round window seconds up to whole minutes, matching codex-rs.
    let window_minutes = win
        .limit_window_seconds
        .filter(|s| *s > 0)
        .map(|s| (s + 59) / 60);
    Some(CodexUsageWindow {
        used_percent,
        window_label: window_minutes.map(codex_window_label),
        window_minutes,
        seconds_until_reset: win.reset_at.map(|r| (r - now).max(0)),
    })
}

/// Map a parsed `GET /wham/usage` body onto a [`CodexRateLimitSnapshot`].
/// Mirrors `parse_codex_usage_payload` in upstream `codex_rate_limits.py` and
/// the header parser above. Returns `None` when there is no usable signal.
fn codex_snapshot_from_usage_payload(payload: &UsagePayloadJson) -> Option<CodexRateLimitSnapshot> {
    let now = now_epoch_secs() as i64;
    let rate_limit = payload.rate_limit.as_ref();
    let primary = rate_limit
        .and_then(|r| r.primary_window.as_ref())
        .and_then(|w| codex_window_from_usage(w, now));
    let secondary = rate_limit
        .and_then(|r| r.secondary_window.as_ref())
        .and_then(|w| codex_window_from_usage(w, now));

    let (credits_balance, credits_unlimited) = match payload.credits.as_ref() {
        Some(c) => {
            let has_credits = c.has_credits.unwrap_or(false);
            // Only surface a balance when the account has credits; a "0"
            // balance on a no-credits plan is noise to the gauge.
            let balance = if has_credits {
                c.balance
                    .as_ref()
                    .and_then(balance_to_string)
                    .filter(|s| !s.is_empty())
            } else {
                None
            };
            (balance, c.unlimited.unwrap_or(false))
        }
        None => (None, false),
    };

    let limit_name = payload
        .rate_limit_reached_type
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if primary.is_none() && secondary.is_none() && credits_balance.is_none() {
        return None;
    }

    Some(CodexRateLimitSnapshot {
        limit_name,
        primary,
        secondary,
        credits_balance,
        credits_unlimited,
    })
}

/// GET the live Codex usage window (blocking; runs on a `spawn_blocking` thread).
fn fetch_codex_usage_snapshot(
    token: &str,
    account_id: &str,
    user_agent: &str,
) -> Option<CodexRateLimitSnapshot> {
    let client = reqwest::blocking::Client::builder()
        .timeout(CODEX_USAGE_POLL_TIMEOUT)
        .build()
        .ok()?;
    let resp = client
        .get(CODEX_USAGE_URL)
        .bearer_auth(token)
        .header("ChatGPT-Account-Id", account_id)
        .header("User-Agent", user_agent)
        .header("Accept", "application/json")
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let payload: UsagePayloadJson = resp.json().ok()?;
    codex_snapshot_from_usage_payload(&payload)
}

/// Fire-and-forget a throttled usage poll to refresh `codex_slot`.
///
/// Scoped to ChatGPT sessions by requiring both a bearer token and a
/// `ChatGPT-Account-Id` header (API-key Codex traffic carries neither), and
/// throttled to one live GET per `CODEX_USAGE_POLL_MIN_INTERVAL_SECS`.
fn maybe_spawn_codex_usage_poll(buf: &[u8], codex_slot: &CodexRateLimitSlot) {
    let Some(token) = extract_bearer(buf) else {
        return;
    };
    let Some(account_id) = extract_header_value(buf, "chatgpt-account-id") else {
        return;
    };

    let now = now_epoch_secs();
    let last = CODEX_USAGE_LAST_POLL.load(Ordering::Relaxed);
    if now.saturating_sub(last) < CODEX_USAGE_POLL_MIN_INTERVAL_SECS {
        return;
    }
    // Claim the slot; lose the race -> another connection is already polling.
    if CODEX_USAGE_LAST_POLL
        .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    let user_agent =
        extract_header_value(buf, "user-agent").unwrap_or_else(|| "headroom-desktop".to_string());
    let slot = codex_slot.clone();
    tokio::task::spawn_blocking(move || {
        if let Some(snapshot) = fetch_codex_usage_snapshot(&token, &account_id, &user_agent) {
            *slot.lock() = Some(snapshot);
        }
    });
}

/// Best-effort decode of the ChatGPT plan from a Codex OAuth bearer JWT. Reads
/// the `chatgpt_plan_type` claim from the `https://api.openai.com/auth` payload
/// object, mirroring the Python proxy's `_decode_openai_bearer_payload`. No
/// signature verification — this is a recommendation hint only.
fn decode_codex_plan_tier(token: &str) -> Option<CodexPlanTier> {
    let payload_b64 = token.split('.').nth(1)?;
    // JWT payloads are base64url without padding; tolerate either form.
    let trimmed = payload_b64.trim_end_matches('=');
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(trimmed)
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let plan = json
        .get("https://api.openai.com/auth")
        .and_then(|auth| auth.get("chatgpt_plan_type"))
        .and_then(|v| v.as_str())?;
    Some(CodexPlanTier::from_claim(plan))
}

/// Window label derived from a minute count, matching upstream's
/// `CodexRateLimitWindow.window_label` (`<60` -> "Nm", else "Hh" / "HhMMm").
fn codex_window_label(window_minutes: i64) -> String {
    if window_minutes < 60 {
        return format!("{window_minutes}m");
    }
    let hours = window_minutes / 60;
    let mins = window_minutes % 60;
    if mins == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h{mins:02}m")
    }
}

static UPSTREAM_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn upstream_client() -> &'static reqwest::Client {
    UPSTREAM_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .build()
            .expect("reqwest client for bypass forwarder")
    })
}

/// Forward the request that produced `header_buf` directly to api.anthropic.com.
///
/// Used when the pricing gate has stopped the local Python proxy. The CC
/// session keeps speaking HTTP/1.1 to 127.0.0.1:6767; we re-issue the same
/// request to the real Anthropic endpoint over TLS with `reqwest`, then stream
/// the response back as HTTP/1.1 chunked transfer.
async fn forward_direct_to_anthropic(
    mut client: TcpStream,
    header_buf: Vec<u8>,
    upstream_base: &str,
) {
    let header_end = match find_header_end(&header_buf) {
        Some(pos) => pos + 4,
        None => {
            let _ = client
                .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
                .await;
            return;
        }
    };
    let leftover_body = &header_buf[header_end..];

    let Some(parsed) = parse_request_head(&header_buf[..header_end]) else {
        let _ = client
            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            .await;
        return;
    };

    // These paths are served by the local Python proxy, not Anthropic. In
    // bypass mode the proxy is intentionally down, so reply 503 instead of
    // forwarding upstream (which would either fail noisily or, worse, hit a
    // real Anthropic endpoint that happens to share the path).
    // Denylist (not allowlist) so future Anthropic API versions like /v2/*
    // continue to forward automatically without requiring a desktop update.
    if is_local_proxy_path(&parsed.path) {
        let _ = client
            .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n")
            .await;
        return;
    }

    // Codex points OPENAI_BASE_URL at this intercept proxy, so in bypass mode
    // OpenAI traffic (e.g. /v1/responses) lands here too. Codex billing is
    // OpenAI's, separate from Headroom's Claude account gate, so don't break
    // Codex when the gate trips — forward OpenAI paths to OpenAI directly
    // rather than (wrongly) to api.anthropic.com.
    let effective_base: &str = if is_openai_path(&parsed.path) {
        OPENAI_DIRECT_BASE
    } else {
        upstream_base
    };

    let body = match parsed.content_length {
        Some(total) if total > leftover_body.len() => {
            let mut body = Vec::with_capacity(total);
            body.extend_from_slice(leftover_body);
            let mut remaining = vec![0u8; total - leftover_body.len()];
            if client.read_exact(&mut remaining).await.is_err() {
                return;
            }
            body.extend_from_slice(&remaining);
            body
        }
        Some(total) => leftover_body[..total.min(leftover_body.len())].to_vec(),
        None => leftover_body.to_vec(),
    };

    let url = format!("{}{}", effective_base, parsed.path);
    let method = match reqwest::Method::from_bytes(parsed.method.as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            let _ = client
                .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
                .await;
            return;
        }
    };

    let mut req = upstream_client().request(method, &url);
    for (name, value) in &parsed.headers {
        if is_hop_by_hop_request_header(name) {
            continue;
        }
        req = req.header(name, value);
    }
    if !body.is_empty() {
        req = req.body(body);
    }

    let mut resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("proxy_intercept bypass forward failed: {e}");
            let _ = client
                .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n")
                .await;
            return;
        }
    };

    let mut head = format!(
        "HTTP/1.1 {} {}\r\n",
        resp.status().as_u16(),
        resp.status().canonical_reason().unwrap_or("")
    );
    for (name, value) in resp.headers().iter() {
        if is_hop_by_hop_response_header(name.as_str()) {
            continue;
        }
        if let Ok(v) = value.to_str() {
            head.push_str(&format!("{}: {}\r\n", name.as_str(), v));
        }
    }
    head.push_str("Transfer-Encoding: chunked\r\nConnection: close\r\n\r\n");
    if client.write_all(head.as_bytes()).await.is_err() {
        return;
    }

    loop {
        match resp.chunk().await {
            Ok(Some(bytes)) if !bytes.is_empty() => {
                let header = format!("{:X}\r\n", bytes.len());
                if client.write_all(header.as_bytes()).await.is_err() {
                    return;
                }
                if client.write_all(&bytes).await.is_err() {
                    return;
                }
                if client.write_all(b"\r\n").await.is_err() {
                    return;
                }
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(e) => {
                log::debug!("[proxy_intercept] bypass body stream error: {e}");
                return;
            }
        }
    }
    let _ = client.write_all(b"0\r\n\r\n").await;
}

struct ParsedRequestHead {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    content_length: Option<usize>,
}

fn parse_request_head(buf: &[u8]) -> Option<ParsedRequestHead> {
    let text = std::str::from_utf8(buf).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();

    let mut headers = Vec::new();
    let mut content_length = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        let name = name.trim().to_string();
        let value = value.trim().to_string();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.parse().ok();
        }
        headers.push((name, value));
    }
    Some(ParsedRequestHead {
        method,
        path,
        headers,
        content_length,
    })
}

fn is_hop_by_hop_request_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "transfer-encoding"
            | "te"
            | "trailers"
            | "proxy-authorization"
            | "proxy-authenticate"
            | "upgrade"
            | "host"
            | "content-length"
            | "accept-encoding"
    )
}

fn is_hop_by_hop_response_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "transfer-encoding"
            | "te"
            | "trailers"
            | "proxy-authorization"
            | "proxy-authenticate"
            | "upgrade"
            | "content-length"
            | "content-encoding"
    )
}

/// Return true if something at 127.0.0.1:INTERCEPT_PORT answers /health with a
/// response that begins with `HTTP/` — that matches both our intercept (which
/// forwards to the python backend and may return 200 or 502) and no realistic
/// foreign process we expect to encounter on this port.
async fn probe_existing_intercept() -> bool {
    let connect = TcpStream::connect(("127.0.0.1", INTERCEPT_PORT));
    let Ok(Ok(mut stream)) = tokio::time::timeout(PROBE_TIMEOUT, connect).await else {
        return false;
    };
    let req = b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(req).await.is_err() {
        return false;
    }
    let mut buf = [0u8; 16];
    let Ok(Ok(n)) = tokio::time::timeout(PROBE_TIMEOUT, stream.read(&mut buf)).await else {
        return false;
    };
    buf.get(..n).is_some_and(|b| b.starts_with(b"HTTP/"))
}

/// Read through the end of the HTTP headers from `stream` into `buf`.
///
/// Forwarding immediately after the header block is enough for token capture
/// and avoids hanging on protocols that wait for a `100 Continue` response
/// before sending the request body.
async fn read_http_headers<R>(stream: &mut R, buf: &mut Vec<u8>) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut tmp = [0u8; 4096];

    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "client closed connection",
            ));
        }
        buf.extend_from_slice(&tmp[..n]);

        if find_header_end(buf).is_some() {
            return Ok(());
        }

        if buf.len() > MAX_HEADER_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "headers exceed maximum size",
            ));
        }
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Case-insensitive check for a header field name in an HTTP request head.
/// `buf` is the full request including the `\r\n\r\n` terminator; only field
/// names (the text before the first `:` on each header line) are matched.
fn request_has_header(buf: &[u8], name: &str) -> bool {
    let end = find_header_end(buf).unwrap_or(buf.len());
    let Ok(text) = std::str::from_utf8(&buf[..end]) else {
        return false;
    };
    text.split("\r\n")
        .skip(1) // request line
        .filter_map(|line| line.split_once(':'))
        .any(|(field, _)| field.trim().eq_ignore_ascii_case(name))
}

/// Insert `X-Client: codex` into a request head so the Python backend's
/// `classify_client` identifies Codex traffic even when the client's
/// User-Agent isn't `codex-cli/` (e.g. the Codex GUI/IDE). A client that
/// already self-identified via `X-Client` is left untouched. No-op if the
/// header terminator is missing.
fn stamp_codex_client_header(buf: &mut Vec<u8>) {
    if request_has_header(buf, "x-client") {
        return;
    }
    let Some(end) = find_header_end(buf) else {
        return;
    };
    // `end` points at the first `\r` of the `\r\n\r\n` terminator. Inserting at
    // `end + 2` (start of the blank line) appends a new last header line while
    // preserving the terminating CRLF.
    let insert_at = end + 2;
    buf.splice(insert_at..insert_at, *b"X-Client: codex\r\n");
}

/// Paths served by the local Python proxy (not Anthropic). Matches the prefix
/// so sub-paths (e.g. `/transformations/feed`) and query strings are covered,
/// while preventing partial matches (e.g. `/healthcheck` does not match
/// `/health`).
fn is_local_proxy_path(path: &str) -> bool {
    const LOCAL_PREFIXES: &[&str] = &[
        "/readyz",
        "/livez",
        "/health",
        "/stats",
        "/transformations",
        "/dashboard",
        "/debug",
        "/subscription-window",
        "/quota",
        "/metrics",
        "/cache",
    ];
    LOCAL_PREFIXES.iter().any(|prefix| {
        path.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/') || rest.starts_with('?'))
    })
}

/// OpenAI-specific API paths used by the Codex CLI. These have no Anthropic
/// counterpart (Claude uses `/v1/messages` / `/v1/complete`), so matching by
/// path is unambiguous and lets bypass-mode forward Codex traffic to OpenAI.
fn is_openai_path(path: &str) -> bool {
    const OPENAI_PREFIXES: &[&str] = &[
        "/v1/responses",
        "/v1/chat/completions",
        "/v1/completions",
        "/v1/embeddings",
    ];
    OPENAI_PREFIXES.iter().any(|prefix| {
        path.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/') || rest.starts_with('?'))
    })
}

/// Return true if the request's Host header targets the loopback listener
/// and no browser Origin header is present. Protects against DNS-rebinding
/// attacks that aim the user's browser at 127.0.0.1 via an attacker domain.
fn request_is_loopback_safe(buf: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(buf) else {
        return false;
    };
    let mut host: Option<&str> = None;
    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("origin:") {
            return false;
        }
        if host.is_none() && lower.starts_with("host:") {
            host = Some(line["host:".len()..].trim());
        }
    }
    match host {
        Some(value) => host_is_loopback(value),
        None => false,
    }
}

fn host_is_loopback(host: &str) -> bool {
    let name = host
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(host)
        .trim_start_matches('[')
        .trim_end_matches(']');
    matches!(name, "127.0.0.1" | "localhost" | "::1")
}

/// Extract the bearer token value from raw HTTP request bytes, if present.
fn extract_bearer(buf: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(buf).ok()?;
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("authorization:") {
            if let Some(_) = rest.trim().strip_prefix("bearer ") {
                // Find "bearer " in the original line (case-insensitive) and
                // return the token with its original casing intact.
                let bearer_pos = lower.find("bearer ").unwrap_or(0) + 7;
                return Some(line[bearer_pos..].trim().to_string());
            }
            // x-api-key style — not usable for the OAuth usage endpoint.
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        bearer_value_changed, codex_snapshot_from_usage_payload, codex_window_label,
        decode_codex_plan_tier, extract_bearer, extract_header_value, find_header_end,
        is_headroom_compression_refusal_response, is_hop_by_hop_request_header,
        is_hop_by_hop_response_header, is_local_proxy_path, is_openai_path,
        parse_codex_rate_limit_headers, parse_request_head, read_http_headers,
        request_is_loopback_safe, run, stamp_codex_client_header, BypassFlag, SharedToken,
    };
    use crate::backend_port;
    use crate::bearer::BearerToken;
    use crate::models::CodexPlanTier;
    use base64::Engine;
    use parking_lot::Mutex;
    use serial_test::serial;
    use std::net::SocketAddr;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::time::{timeout, Duration};

    #[test]
    fn finds_header_boundary() {
        let request = b"POST /v1/messages HTTP/1.1\r\nHost: localhost\r\n\r\n{\"x\":1}";
        assert_eq!(find_header_end(request), Some(43));
    }

    #[test]
    fn openai_paths_route_to_openai_in_bypass() {
        // Codex's Responses API and the OpenAI chat/completions family must be
        // recognized as OpenAI traffic so bypass mode forwards them to OpenAI,
        // not api.anthropic.com.
        assert!(is_openai_path("/v1/responses"));
        assert!(is_openai_path("/v1/responses/abc?stream=true"));
        assert!(is_openai_path("/v1/chat/completions"));
        assert!(is_openai_path("/v1/completions"));
        assert!(is_openai_path("/v1/embeddings"));
        // Anthropic paths must NOT be misrouted to OpenAI.
        assert!(!is_openai_path("/v1/messages"));
        assert!(!is_openai_path("/v1/complete"));
        assert!(!is_openai_path("/v1/models"));
        // Codex's own usage tracker endpoints stay local.
        assert!(is_local_proxy_path("/stats"));
        assert!(!is_openai_path("/stats"));
    }

    #[test]
    fn extracts_bearer_token_case_insensitively() {
        let request = b"POST / HTTP/1.1\r\nAuthorization: Bearer test-token\r\n\r\n";
        assert_eq!(extract_bearer(request).as_deref(), Some("test-token"));
    }

    #[test]
    fn bearer_value_changed_treats_empty_slot_as_changed() {
        let slot: SharedToken = Arc::new(Mutex::new(None));
        assert!(bearer_value_changed(&slot, "any-token"));
    }

    #[test]
    fn bearer_value_changed_skips_signal_when_value_matches() {
        let slot: SharedToken = Arc::new(Mutex::new(Some(BearerToken::new("token-A".into()))));
        assert!(!bearer_value_changed(&slot, "token-A"));
    }

    #[test]
    fn bearer_value_changed_signals_when_value_differs() {
        let slot: SharedToken = Arc::new(Mutex::new(Some(BearerToken::new("token-A".into()))));
        assert!(bearer_value_changed(&slot, "token-B"));
    }

    #[test]
    fn loopback_host_without_origin_is_accepted() {
        let req = b"POST / HTTP/1.1\r\nHost: 127.0.0.1:6767\r\n\r\n";
        assert!(request_is_loopback_safe(req));
        let req = b"POST / HTTP/1.1\r\nHost: localhost:6767\r\n\r\n";
        assert!(request_is_loopback_safe(req));
        let req = b"POST / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        assert!(request_is_loopback_safe(req));
    }

    #[test]
    fn non_loopback_host_is_rejected() {
        let req = b"POST / HTTP/1.1\r\nHost: evil.example.com\r\n\r\n";
        assert!(!request_is_loopback_safe(req));
        let req = b"POST / HTTP/1.1\r\nHost: 169.254.169.254\r\n\r\n";
        assert!(!request_is_loopback_safe(req));
    }

    #[test]
    fn origin_header_causes_rejection_even_on_loopback() {
        let req =
            b"POST / HTTP/1.1\r\nHost: 127.0.0.1:6767\r\nOrigin: https://evil.example.com\r\n\r\n";
        assert!(!request_is_loopback_safe(req));
    }

    #[test]
    fn missing_host_header_is_rejected() {
        let req = b"POST / HTTP/1.1\r\nContent-Length: 0\r\n\r\n";
        assert!(!request_is_loopback_safe(req));
    }

    #[tokio::test]
    async fn header_read_does_not_wait_for_continue_body() {
        let (mut client, mut server_stream) = duplex(1024);

        let writer = tokio::spawn(async move {
            client
                .write_all(
                    b"POST /v1/messages HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\nExpect: 100-continue\r\n\r\n",
                )
                .await
                .expect("write headers");
        });

        let mut buf = Vec::new();
        timeout(
            Duration::from_millis(250),
            read_http_headers(&mut server_stream, &mut buf),
        )
        .await
        .expect("headers should complete without waiting for body")
        .expect("header read succeeds");

        assert!(buf.windows(4).any(|window| window == b"\r\n\r\n"));
        writer.await.expect("writer task");
    }

    /// Bind a fresh `TcpListener` on an ephemeral port and return its address.
    async fn bind_ephemeral() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        (listener, addr)
    }

    /// Read header bytes from `stream` up through (and including) the `\r\n\r\n`
    /// boundary so the test can assert what the intercept forwarded.
    async fn read_until_header_end(stream: &mut TcpStream) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 1024];
        for _ in 0..32 {
            let n = stream.read(&mut tmp).await.unwrap_or(0);
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        buf
    }

    #[tokio::test]
    #[serial(backend_port)]
    async fn intercept_captures_bearer_and_forwards_headers_to_backend() {
        // Fake backend: accept one connection, read its header block, hold the
        // connection open long enough for the test to inspect what arrived.
        let (backend_listener, backend_addr) = bind_ephemeral().await;
        let backend_task = tokio::spawn(async move {
            let (mut sock, _) = backend_listener.accept().await.expect("backend accept");
            let received = read_until_header_end(&mut sock).await;
            // Send a stub response so the client side of copy_bidirectional has
            // something to consume.
            let _ = sock
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await;
            received
        });

        // Point the intercept's per-connection backend lookup at our fake
        // backend's ephemeral port. Serialized via #[serial(backend_port)] so
        // tests that mutate the global don't race.
        backend_port::set(backend_addr.port());

        // Run the intercept on its own ephemeral port.
        let token_slot: SharedToken = Arc::new(Mutex::new(None));
        let intercept_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("intercept bind");
        let intercept_addr = intercept_listener.local_addr().expect("intercept addr");
        drop(intercept_listener); // free the port; run() rebinds the same one
        let slot_for_run = token_slot.clone();
        let bypass_for_run: BypassFlag = Arc::new(AtomicBool::new(false));
        let upstream_base = Arc::new("https://api.anthropic.com".to_string());
        let (fresh_bearer_tx, _fresh_bearer_rx) = std::sync::mpsc::channel::<()>();
        let run_task = tokio::spawn(async move {
            // run() loops forever; the test cancels it via abort below.
            let _ = run(
                intercept_addr,
                slot_for_run,
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                bypass_for_run,
                Arc::new(AtomicBool::new(false)),
                fresh_bearer_tx,
                upstream_base,
            )
            .await;
        });

        // Give run() a moment to bind. A brief retry loop on connect is more
        // reliable than a fixed sleep, since CI can be slow.
        let mut client = None;
        for _ in 0..50 {
            if let Ok(c) = TcpStream::connect(intercept_addr).await {
                client = Some(c);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let mut client = client.expect("intercept reachable");

        let request = format!(
            "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer test-token-123\r\nContent-Length: 0\r\n\r\n",
            intercept_addr.port()
        );
        client
            .write_all(request.as_bytes())
            .await
            .expect("write request");

        let received = timeout(Duration::from_secs(2), backend_task)
            .await
            .expect("backend forwarded request in time")
            .expect("backend task ok");

        // Headers should have been forwarded verbatim — including the Bearer.
        let received_str = std::str::from_utf8(&received).expect("utf8");
        assert!(
            received_str.contains("POST /v1/messages HTTP/1.1"),
            "request line forwarded: {received_str:?}"
        );
        assert!(
            received_str.contains("Authorization: Bearer test-token-123"),
            "bearer header forwarded: {received_str:?}"
        );

        // The bearer token should have been captured into the shared slot.
        let captured = token_slot.lock().clone();
        let bearer = captured.expect("bearer captured");
        // BearerToken stores its value but doesn't expose it directly — verify
        // via value_if_fresh with a generous TTL.
        assert_eq!(
            bearer
                .value_if_fresh(Duration::from_secs(60))
                .map(|s| s.to_string()),
            Some("test-token-123".to_string())
        );

        run_task.abort();
        backend_port::reset_for_tests();
    }

    #[tokio::test]
    #[serial(backend_port)]
    async fn intercept_returns_502_when_backend_is_unreachable() {
        // Pick a backend port that nothing is listening on. Bind+immediately
        // drop a listener to grab a free port, then connect attempts will fail.
        let (probe, dead_backend_addr) = bind_ephemeral().await;
        drop(probe);
        backend_port::set(dead_backend_addr.port());

        let token_slot: SharedToken = Arc::new(Mutex::new(None));
        let intercept_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("intercept bind");
        let intercept_addr = intercept_listener.local_addr().expect("intercept addr");
        drop(intercept_listener);
        let slot_for_run = token_slot.clone();
        let bypass_for_run: BypassFlag = Arc::new(AtomicBool::new(false));
        let upstream_base = Arc::new("https://api.anthropic.com".to_string());
        let (fresh_bearer_tx, _fresh_bearer_rx) = std::sync::mpsc::channel::<()>();
        let run_task = tokio::spawn(async move {
            let _ = run(
                intercept_addr,
                slot_for_run,
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                bypass_for_run,
                Arc::new(AtomicBool::new(false)),
                fresh_bearer_tx,
                upstream_base,
            )
            .await;
        });

        let mut client = None;
        for _ in 0..50 {
            if let Ok(c) = TcpStream::connect(intercept_addr).await {
                client = Some(c);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let mut client = client.expect("intercept reachable");

        let request = format!(
            "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Length: 0\r\n\r\n",
            intercept_addr.port()
        );
        client
            .write_all(request.as_bytes())
            .await
            .expect("write request");

        let mut response = Vec::new();
        let mut tmp = [0u8; 256];
        let _ = timeout(Duration::from_secs(2), async {
            loop {
                let n = client.read(&mut tmp).await.unwrap_or(0);
                if n == 0 {
                    break;
                }
                response.extend_from_slice(&tmp[..n]);
                if response.len() >= 16 {
                    break;
                }
            }
        })
        .await;
        let response_str = std::str::from_utf8(&response).unwrap_or("");
        assert!(
            response_str.starts_with("HTTP/1.1 502"),
            "expected 502 Bad Gateway, got: {response_str:?}"
        );

        run_task.abort();
        backend_port::reset_for_tests();
    }

    #[test]
    fn parse_request_head_extracts_method_path_and_content_length() {
        let buf = b"POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1:6767\r\nAuthorization: Bearer abc\r\nContent-Length: 42\r\n\r\n";
        let parsed = parse_request_head(buf).expect("parsed");
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.path, "/v1/messages");
        assert_eq!(parsed.content_length, Some(42));
        assert!(parsed
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("authorization") && v == "Bearer abc"));
    }

    #[test]
    fn parse_request_head_handles_missing_content_length() {
        let buf = b"GET /v1/models HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let parsed = parse_request_head(buf).expect("parsed");
        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.path, "/v1/models");
        assert_eq!(parsed.content_length, None);
    }

    #[test]
    fn parse_request_head_returns_none_for_garbage() {
        // Only one token before \r\n -> no path -> None.
        let buf = b"NOTHTTP\r\n\r\n";
        assert!(parse_request_head(buf).is_none());
    }

    #[test]
    fn stamp_codex_client_header_inserts_last_header() {
        let mut buf =
            b"POST /v1/responses HTTP/1.1\r\nHost: 127.0.0.1:6767\r\nUser-Agent: codex_vscode/1.0\r\n\r\n"
                .to_vec();
        stamp_codex_client_header(&mut buf);
        let parsed = parse_request_head(&buf).expect("still a valid request head");
        assert_eq!(parsed.path, "/v1/responses");
        assert!(
            parsed
                .headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("x-client") && v == "codex"),
            "X-Client: codex should be present: {:?}",
            parsed.headers
        );
        // Header block stays well-formed (single blank-line terminator).
        assert!(buf.ends_with(b"X-Client: codex\r\n\r\n"));
        assert_eq!(buf.windows(4).filter(|w| *w == b"\r\n\r\n").count(), 1);
    }

    #[test]
    fn stamp_codex_client_header_preserves_body_bytes() {
        // The proxy only buffers the head, but a request may arrive with the
        // body already appended; the insertion must not corrupt it.
        let mut buf = b"POST /v1/responses HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello".to_vec();
        stamp_codex_client_header(&mut buf);
        assert!(buf.ends_with(b"\r\n\r\nhello"));
    }

    #[test]
    fn stamp_codex_client_header_respects_explicit_client() {
        let original = b"POST /v1/responses HTTP/1.1\r\nX-Client: aider\r\n\r\n".to_vec();
        let mut buf = original.clone();
        stamp_codex_client_header(&mut buf);
        assert_eq!(buf, original, "an explicit X-Client must be left untouched");
    }

    #[test]
    fn stamp_codex_client_header_noop_without_terminator() {
        let mut buf = b"POST /v1/responses HTTP/1.1\r\nHost: x".to_vec();
        let original = buf.clone();
        stamp_codex_client_header(&mut buf);
        assert_eq!(buf, original);
    }

    #[test]
    fn hop_by_hop_request_header_recognises_canonical_names() {
        for name in [
            "Connection",
            "keep-alive",
            "TRANSFER-ENCODING",
            "te",
            "trailers",
            "Proxy-Authorization",
            "Upgrade",
            "Host",
            "Content-Length",
            "Accept-Encoding",
        ] {
            assert!(
                is_hop_by_hop_request_header(name),
                "{name} should be hop-by-hop on the request side"
            );
        }
        // Headers we want to forward must NOT be flagged.
        for name in [
            "Authorization",
            "anthropic-version",
            "x-api-key",
            "Content-Type",
        ] {
            assert!(
                !is_hop_by_hop_request_header(name),
                "{name} must be forwarded"
            );
        }
    }

    #[test]
    fn hop_by_hop_response_header_recognises_canonical_names() {
        for name in [
            "Connection",
            "Keep-Alive",
            "transfer-encoding",
            "Content-Length",
            "Content-Encoding",
        ] {
            assert!(
                is_hop_by_hop_response_header(name),
                "{name} should be hop-by-hop on the response side"
            );
        }
        for name in [
            "Content-Type",
            "anthropic-ratelimit-requests-remaining",
            "x-request-id",
        ] {
            assert!(
                !is_hop_by_hop_response_header(name),
                "{name} must be forwarded"
            );
        }
    }

    /// Drive the bypass branch end-to-end: intercept on :6767 with bypass=true
    /// forwards a request to a fake upstream, then streams the upstream's
    /// response back to the client as HTTP/1.1 chunked transfer.
    #[tokio::test]
    #[serial(backend_port)]
    async fn bypass_forwards_request_to_upstream_and_streams_response_back() {
        let (upstream_listener, upstream_addr) = bind_ephemeral().await;
        let upstream_base = format!("http://127.0.0.1:{}", upstream_addr.port());

        let upstream_task = tokio::spawn(async move {
            let (mut sock, _) = upstream_listener.accept().await.expect("upstream accept");
            // Read until headers + content-length body have arrived.
            let mut received = Vec::new();
            let mut tmp = [0u8; 4096];
            let mut header_end: Option<usize> = None;
            let mut content_length: usize = 0;
            for _ in 0..256 {
                let n = sock.read(&mut tmp).await.unwrap_or(0);
                if n == 0 {
                    break;
                }
                received.extend_from_slice(&tmp[..n]);
                if header_end.is_none() {
                    if let Some(pos) = find_header_end(&received) {
                        header_end = Some(pos + 4);
                        let header_text = std::str::from_utf8(&received[..pos]).unwrap_or("");
                        for line in header_text.lines() {
                            let lower = line.to_ascii_lowercase();
                            if let Some(rest) = lower.strip_prefix("content-length:") {
                                content_length = rest.trim().parse().unwrap_or(0);
                            }
                        }
                    }
                }
                if let Some(end) = header_end {
                    if received.len() >= end + content_length {
                        break;
                    }
                }
            }
            // Reply with a small SSE-style payload over Content-Length so
            // reqwest can fully consume the response.
            let body = b"event: message\ndata: hi\n\n";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nx-request-id: req-test-1\r\n\r\n",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.write_all(body).await;
            let _ = sock.shutdown().await;
            received
        });

        let token_slot: SharedToken = Arc::new(Mutex::new(None));
        let intercept_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("intercept bind");
        let intercept_addr = intercept_listener.local_addr().expect("intercept addr");
        drop(intercept_listener);
        let bypass: BypassFlag = Arc::new(AtomicBool::new(true));
        // Bypass means we never actually contact the backend; pin to an
        // unused loopback port so any accidental connect would fail fast.
        backend_port::set(1);
        let upstream_base_arc = Arc::new(upstream_base);
        let token_for_run = token_slot.clone();
        let (fresh_bearer_tx, _fresh_bearer_rx) = std::sync::mpsc::channel::<()>();
        let run_task = tokio::spawn(async move {
            let _ = run(
                intercept_addr,
                token_for_run,
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                bypass,
                Arc::new(AtomicBool::new(false)),
                fresh_bearer_tx,
                upstream_base_arc,
            )
            .await;
        });

        let mut client = None;
        for _ in 0..50 {
            if let Ok(c) = TcpStream::connect(intercept_addr).await {
                client = Some(c);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let mut client = client.expect("intercept reachable");

        let req_body = br#"{"model":"claude"}"#;
        let request_head = format!(
            "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer test-bypass-token\r\nContent-Type: application/json\r\nAccept-Encoding: gzip\r\nContent-Length: {}\r\n\r\n",
            intercept_addr.port(),
            req_body.len()
        );
        client
            .write_all(request_head.as_bytes())
            .await
            .expect("write headers");
        client.write_all(req_body).await.expect("write body");

        let received = timeout(Duration::from_secs(5), upstream_task)
            .await
            .expect("upstream got request in time")
            .expect("upstream task ok");
        let received_str = std::str::from_utf8(&received).expect("utf8");

        assert!(
            received_str.starts_with("POST /v1/messages HTTP/1.1"),
            "request line forwarded verbatim: {received_str:?}"
        );
        let received_lower = received_str.to_ascii_lowercase();
        assert!(
            received_lower.contains("authorization: bearer test-bypass-token"),
            "Authorization forwarded: {received_str:?}"
        );
        assert!(
            received_lower.contains("content-type: application/json"),
            "Content-Type forwarded: {received_str:?}"
        );
        // Hop-by-hop request headers must be stripped before reaching upstream.
        assert!(
            !received_lower.contains("accept-encoding:"),
            "Accept-Encoding must be stripped: {received_str:?}"
        );
        // Body forwarded.
        assert!(
            received_str.contains(r#"{"model":"claude"}"#),
            "request body forwarded: {received_str:?}"
        );
        // Bearer captured into the shared slot.
        assert!(token_slot.lock().is_some(), "bearer was captured");

        // Now read the response the intercept relayed back to the client.
        let mut response = Vec::new();
        let mut tmp = [0u8; 4096];
        let _ = timeout(Duration::from_secs(5), async {
            for _ in 0..256 {
                let n = client.read(&mut tmp).await.unwrap_or(0);
                if n == 0 {
                    break;
                }
                response.extend_from_slice(&tmp[..n]);
                // Stop once the chunked terminator has arrived.
                if response.windows(5).any(|w| w == b"0\r\n\r\n") {
                    break;
                }
            }
        })
        .await;
        let response_str = std::str::from_utf8(&response).expect("utf8");

        assert!(
            response_str.starts_with("HTTP/1.1 200"),
            "response status forwarded: {response_str:?}"
        );
        let response_lower = response_str.to_ascii_lowercase();
        assert!(
            response_lower.contains("transfer-encoding: chunked"),
            "intercept rewrote response as chunked: {response_str:?}"
        );
        // Content-Length must have been stripped — replaced by chunked framing.
        assert!(
            !response_lower.contains("content-length:"),
            "Content-Length stripped on response: {response_str:?}"
        );
        // Forwarded response headers preserved.
        assert!(
            response_lower.contains("x-request-id: req-test-1"),
            "non-hop-by-hop response header forwarded: {response_str:?}"
        );
        // Body present somewhere in the chunked stream.
        assert!(
            response_str.contains("event: message"),
            "response body forwarded: {response_str:?}"
        );
        assert!(
            response_str.contains("data: hi"),
            "response body forwarded: {response_str:?}"
        );
        // Chunked terminator at the end.
        assert!(
            response_str.contains("0\r\n\r\n"),
            "chunked terminator written: {response_str:?}"
        );

        run_task.abort();
        backend_port::reset_for_tests();
    }

    #[tokio::test]
    #[serial(backend_port)]
    async fn bypass_returns_502_when_upstream_unreachable() {
        // Bind+drop to grab a free port nothing is listening on.
        let (probe, dead_addr) = bind_ephemeral().await;
        drop(probe);
        let upstream_base = format!("http://127.0.0.1:{}", dead_addr.port());

        let token_slot: SharedToken = Arc::new(Mutex::new(None));
        let intercept_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("intercept bind");
        let intercept_addr = intercept_listener.local_addr().expect("intercept addr");
        drop(intercept_listener);
        let bypass: BypassFlag = Arc::new(AtomicBool::new(true));
        backend_port::set(1);
        let upstream_base_arc = Arc::new(upstream_base);
        let (fresh_bearer_tx, _fresh_bearer_rx) = std::sync::mpsc::channel::<()>();
        let run_task = tokio::spawn(async move {
            let _ = run(
                intercept_addr,
                token_slot,
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                bypass,
                Arc::new(AtomicBool::new(false)),
                fresh_bearer_tx,
                upstream_base_arc,
            )
            .await;
        });

        let mut client = None;
        for _ in 0..50 {
            if let Ok(c) = TcpStream::connect(intercept_addr).await {
                client = Some(c);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let mut client = client.expect("intercept reachable");
        let request = format!(
            "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Length: 0\r\n\r\n",
            intercept_addr.port()
        );
        client
            .write_all(request.as_bytes())
            .await
            .expect("write request");

        let mut response = Vec::new();
        let mut tmp = [0u8; 256];
        let _ = timeout(Duration::from_secs(5), async {
            loop {
                let n = client.read(&mut tmp).await.unwrap_or(0);
                if n == 0 {
                    break;
                }
                response.extend_from_slice(&tmp[..n]);
                if response.len() >= 16 {
                    break;
                }
            }
        })
        .await;
        let response_str = std::str::from_utf8(&response).unwrap_or("");
        assert!(
            response_str.starts_with("HTTP/1.1 502"),
            "expected 502 when upstream unreachable, got: {response_str:?}"
        );

        run_task.abort();
        backend_port::reset_for_tests();
    }

    /// New: the intercept must read the backend port per connection so that
    /// when `tool_manager` selects a fallback port mid-launch, in-flight
    /// clients get routed to the new backend without a thread restart.
    #[tokio::test]
    #[serial(backend_port)]
    async fn intercept_picks_up_backend_port_changes_between_connections() {
        let (first_listener, first_addr) = bind_ephemeral().await;
        let (second_listener, second_addr) = bind_ephemeral().await;

        let first_task = tokio::spawn(async move {
            let (mut sock, _) = first_listener.accept().await.expect("first accept");
            let _ = read_until_header_end(&mut sock).await;
            let _ = sock
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await;
            "first"
        });
        let second_task = tokio::spawn(async move {
            let (mut sock, _) = second_listener.accept().await.expect("second accept");
            let _ = read_until_header_end(&mut sock).await;
            let _ = sock
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await;
            "second"
        });

        backend_port::set(first_addr.port());

        let token_slot: SharedToken = Arc::new(Mutex::new(None));
        let intercept_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("intercept bind");
        let intercept_addr = intercept_listener.local_addr().expect("intercept addr");
        drop(intercept_listener);
        let bypass_for_run: BypassFlag = Arc::new(AtomicBool::new(false));
        let upstream_base = Arc::new("https://api.anthropic.com".to_string());
        let token_for_run = token_slot.clone();
        let (fresh_bearer_tx, _fresh_bearer_rx) = std::sync::mpsc::channel::<()>();
        let run_task = tokio::spawn(async move {
            let _ = run(
                intercept_addr,
                token_for_run,
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                bypass_for_run,
                Arc::new(AtomicBool::new(false)),
                fresh_bearer_tx,
                upstream_base,
            )
            .await;
        });

        // Wait for the intercept to bind, then send the first request.
        let mut first_client = None;
        for _ in 0..50 {
            if let Ok(c) = TcpStream::connect(intercept_addr).await {
                first_client = Some(c);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let mut first_client = first_client.expect("intercept reachable");
        let req = format!(
            "POST / HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Length: 0\r\n\r\n",
            intercept_addr.port()
        );
        first_client
            .write_all(req.as_bytes())
            .await
            .expect("write first req");

        let routed_first = timeout(Duration::from_secs(2), first_task)
            .await
            .expect("first backend received request")
            .expect("first task ok");
        assert_eq!(routed_first, "first");

        // Switch the global to the second backend; next connection routes there.
        backend_port::set(second_addr.port());

        let mut second_client = TcpStream::connect(intercept_addr)
            .await
            .expect("connect second");
        second_client
            .write_all(req.as_bytes())
            .await
            .expect("write second req");

        let routed_second = timeout(Duration::from_secs(2), second_task)
            .await
            .expect("second backend received request")
            .expect("second task ok");
        assert_eq!(routed_second, "second");

        run_task.abort();
        backend_port::reset_for_tests();
    }

    // ── codex rate-limit header parsing ─────────────────────────────────────

    #[test]
    fn parse_codex_headers_decodes_primary_secondary_credits() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let head = format!(
            "HTTP/1.1 200 OK\r\n\
             content-type: text/event-stream\r\n\
             x-codex-limit-name: gpt-5.2-codex\r\n\
             x-codex-primary-used-percent: 42.5\r\n\
             x-codex-primary-window-minutes: 300\r\n\
             x-codex-primary-reset-at: {}\r\n\
             x-codex-secondary-used-percent: 12\r\n\
             x-codex-secondary-window-minutes: 10080\r\n\
             x-codex-secondary-reset-at: {}\r\n\
             x-codex-credits-balance: $5.00\r\n\
             x-codex-credits-unlimited: false\r\n\
             \r\n",
            now + 7200,
            now + 86400,
        );
        let snap = parse_codex_rate_limit_headers(head.as_bytes()).expect("snapshot");
        assert_eq!(snap.limit_name.as_deref(), Some("gpt-5.2-codex"));
        let primary = snap.primary.expect("primary");
        assert_eq!(primary.used_percent, 42.5);
        assert_eq!(primary.window_minutes, Some(300));
        assert_eq!(primary.window_label.as_deref(), Some("5h"));
        // Reset is ~7200s out; allow a couple seconds of clock slack.
        let secs = primary.seconds_until_reset.expect("reset");
        assert!((7195..=7200).contains(&secs), "got {secs}");
        let secondary = snap.secondary.expect("secondary");
        assert_eq!(secondary.window_label.as_deref(), Some("168h"));
        assert_eq!(snap.credits_balance.as_deref(), Some("$5.00"));
        assert!(!snap.credits_unlimited);
    }

    #[test]
    fn parse_codex_headers_case_insensitive_and_clamps_past_reset() {
        let head = "HTTP/1.1 429 Too Many Requests\r\n\
             X-Codex-Primary-Used-Percent: 99\r\n\
             X-Codex-Primary-Window-Minutes: 45\r\n\
             X-Codex-Primary-Reset-At: 100\r\n\
             \r\n";
        let snap = parse_codex_rate_limit_headers(head.as_bytes()).expect("snapshot");
        let primary = snap.primary.expect("primary");
        assert_eq!(primary.used_percent, 99.0);
        assert_eq!(primary.window_label.as_deref(), Some("45m"));
        // reset-at is in the distant past -> clamped to 0.
        assert_eq!(primary.seconds_until_reset, Some(0));
    }

    #[test]
    fn parse_codex_headers_absent_returns_none() {
        let head = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n";
        assert!(parse_codex_rate_limit_headers(head.as_bytes()).is_none());
    }

    #[test]
    fn parse_codex_headers_partial_head_returns_none() {
        // No header terminator / garbage — must not panic, no signal.
        let head = "HTTP/1.1 200 OK\r\nx-codex-limit-name: codex";
        assert!(parse_codex_rate_limit_headers(head.as_bytes()).is_none());
    }

    #[test]
    fn detects_headroom_compression_refusal_413() {
        let head = concat!(
            "HTTP/1.1 413 Payload Too Large\r\n",
            "content-type: application/json\r\n",
            "\r\n",
            r#"{"detail":{"error":{"type":"compression_refused","message":"headroom: compression timeout on a 1510408-byte request - please compact context and retry."}}}"#
        );
        assert!(is_headroom_compression_refusal_response(head.as_bytes()));
    }

    #[test]
    fn ignores_unrelated_payload_too_large() {
        let head = concat!(
            "HTTP/1.1 413 Payload Too Large\r\n",
            "content-type: application/json\r\n",
            "\r\n",
            r#"{"error":{"message":"request body too large"}}"#
        );
        assert!(!is_headroom_compression_refusal_response(head.as_bytes()));
    }

    // A faithful GET /wham/usage body (shape captured from a live Plus account).
    const USAGE_BODY: &str = r#"{
        "plan_type": "plus",
        "rate_limit": {
            "allowed": true,
            "limit_reached": false,
            "primary_window": {"used_percent": 23, "limit_window_seconds": 18000, "reset_at": 1781276043},
            "secondary_window": {"used_percent": 6, "limit_window_seconds": 604800, "reset_at": 1781622947}
        },
        "credits": {"has_credits": false, "unlimited": false, "balance": "0"},
        "rate_limit_reached_type": null,
        "promo": null
    }"#;

    #[test]
    fn usage_payload_maps_to_snapshot() {
        let payload = serde_json::from_str(USAGE_BODY).expect("json");
        let snap = codex_snapshot_from_usage_payload(&payload).expect("snapshot");
        let primary = snap.primary.expect("primary");
        assert_eq!(primary.used_percent, 23.0);
        assert_eq!(primary.window_minutes, Some(300)); // 18000s rounded up
        let secondary = snap.secondary.expect("secondary");
        assert_eq!(secondary.used_percent, 6.0);
        assert_eq!(secondary.window_minutes, Some(10080)); // 604800s
                                                           // has_credits=false -> "0" balance must not surface as noise.
        assert_eq!(snap.credits_balance, None);
        assert!(!snap.credits_unlimited);
    }

    #[test]
    fn usage_window_minutes_rounds_up() {
        let payload = serde_json::from_str(
            r#"{"rate_limit":{"primary_window":{"used_percent":1,"limit_window_seconds":61}}}"#,
        )
        .expect("json");
        let snap = codex_snapshot_from_usage_payload(&payload).expect("snapshot");
        assert_eq!(snap.primary.expect("primary").window_minutes, Some(2));
    }

    #[test]
    fn usage_credits_balance_kept_when_has_credits() {
        let payload = serde_json::from_str(
            r#"{"rate_limit":{"primary_window":{"used_percent":5}},"credits":{"has_credits":true,"unlimited":false,"balance":"$5.00"}}"#,
        )
        .expect("json");
        let snap = codex_snapshot_from_usage_payload(&payload).expect("snapshot");
        assert_eq!(snap.credits_balance.as_deref(), Some("$5.00"));
    }

    #[test]
    fn usage_empty_payload_returns_none() {
        let payload = serde_json::from_str("{}").expect("json");
        assert!(codex_snapshot_from_usage_payload(&payload).is_none());
        let payload = serde_json::from_str(r#"{"rate_limit":{}}"#).expect("json");
        assert!(codex_snapshot_from_usage_payload(&payload).is_none());
    }

    #[test]
    fn usage_window_missing_used_percent_skipped() {
        let payload = serde_json::from_str(
            r#"{"rate_limit":{"primary_window":{"limit_window_seconds":60}}}"#,
        )
        .expect("json");
        assert!(codex_snapshot_from_usage_payload(&payload).is_none());
    }

    #[test]
    fn extract_header_value_is_case_insensitive() {
        let req = b"GET /v1/responses HTTP/1.1\r\nHost: x\r\nChatGPT-Account-Id: acct_9\r\n\r\n";
        assert_eq!(
            extract_header_value(req, "chatgpt-account-id").as_deref(),
            Some("acct_9")
        );
        assert!(extract_header_value(req, "x-missing").is_none());
    }

    fn jwt_with_plan(plan: &str) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
        let payload_json = format!(
            "{{\"https://api.openai.com/auth\":{{\"chatgpt_plan_type\":\"{plan}\",\"chatgpt_account_id\":\"acct_1\"}}}}"
        );
        let payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        format!("{header}.{payload}.sig")
    }

    #[test]
    fn decode_codex_plan_tier_reads_chatgpt_plan_type() {
        assert_eq!(
            decode_codex_plan_tier(&jwt_with_plan("plus")),
            Some(CodexPlanTier::Plus)
        );
        assert_eq!(
            decode_codex_plan_tier(&jwt_with_plan("pro")),
            Some(CodexPlanTier::Pro)
        );
        // Unrecognized claim value still decodes, mapped to Unknown.
        assert_eq!(
            decode_codex_plan_tier(&jwt_with_plan("mystery")),
            Some(CodexPlanTier::Unknown)
        );
    }

    #[test]
    fn decode_codex_plan_tier_rejects_malformed_tokens() {
        assert!(decode_codex_plan_tier("not-a-jwt").is_none());
        assert!(decode_codex_plan_tier("only.two").is_none());
        // Valid JWT shape but no auth claim.
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{\"sub\":\"x\"}");
        assert!(decode_codex_plan_tier(&format!("h.{payload}.s")).is_none());
    }

    #[test]
    fn codex_window_label_formats() {
        assert_eq!(codex_window_label(45), "45m");
        assert_eq!(codex_window_label(300), "5h");
        assert_eq!(codex_window_label(10080), "168h");
        assert_eq!(codex_window_label(90), "1h30m");
    }
}
