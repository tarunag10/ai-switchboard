use std::sync::atomic::{AtomicBool, Ordering};

use crate::{port_conflict, process_runner, tool_manager};

// Set after the first port-conflict start failure has been captured this
// session. Subsequent in-session port conflicts stay silent so the dashboard
// doesn't drown in the sleep/wake / kill -9 race noise.
static PORT_CONFLICT_CAPTURED: AtomicBool = AtomicBool::new(false);

pub(crate) fn is_port_conflict_failure(technical_err: &str) -> bool {
    port_conflict::is_port_conflict(technical_err)
        || technical_err.contains("headroom proxy already running on port")
}

/// Report a headroom proxy startup failure to Sentry. If the error chain
/// contains a `HeadroomStartupFailure`, its log tail, log path, and invocation
/// are sent as structured `extra` fields so we can see what Python printed
/// before failing to bind the port.
pub(crate) fn capture_headroom_start_failure(context: &str, err: &anyhow::Error) {
    let technical_err = format!("{err:#}");

    // Environmental failures: another process holds port 6768, or a stale
    // headroom proxy is still bound. The user gets an actionable hint via
    // `state::classify_startup_error` and the persistent-conflict case is
    // surfaced separately by `port_conflict::note_proxy_failed`. Capture once
    // per session at Warning level under a distinct fingerprint so the
    // dashboard sees real failures (stale child holding the port,
    // sleep/wake race) without drowning in non-actionable noise.
    let is_port_conflict = is_port_conflict_failure(&technical_err);

    let startup_failure = err
        .chain()
        .find_map(|e| e.downcast_ref::<tool_manager::HeadroomStartupFailure>());

    let headline = format!("{context}: {technical_err}");
    let truncated = headline.chars().take(400).collect::<String>();

    if is_port_conflict {
        if PORT_CONFLICT_CAPTURED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        sentry::with_scope(
            |scope| {
                let fp: &[&str] = &["proxy_start_port_conflict"];
                scope.set_fingerprint(Some(fp));
                if let Some(failure) = startup_failure {
                    scope.set_extra("program", failure.program.clone().into());
                    scope.set_extra("args", failure.args.join(" ").into());
                    scope.set_extra("log_path", failure.log_path.clone().into());
                    scope.set_extra("log_tail", failure.log_tail.clone().into());
                    scope.set_extra("reason", failure.reason.clone().into());
                }
                scope.set_extra("error_chain", technical_err.clone().into());
            },
            || {
                sentry::capture_message(&truncated, sentry::Level::Warning);
            },
        );
        return;
    }

    if let Some(failure) = startup_failure {
        sentry::with_scope(
            |scope| {
                scope.set_extra("program", failure.program.clone().into());
                scope.set_extra("args", failure.args.join(" ").into());
                scope.set_extra("log_path", failure.log_path.clone().into());
                scope.set_extra("log_tail", failure.log_tail.clone().into());
                scope.set_extra("reason", failure.reason.clone().into());
                scope.set_extra("error_chain", technical_err.clone().into());
            },
            || {
                sentry::capture_message(&truncated, sentry::Level::Error);
            },
        );
    } else {
        sentry::capture_message(&truncated, sentry::Level::Error);
    }
}

/// Diagnostic snapshot taken at the moment a boot-validation failure is
/// captured. Distinguishes "the new proxy never spawned" (tracked_child=false)
/// from "spawned but crashed before writing logs" (no new log) from "spawned
/// and bound but unreachable" (port_bound=true, log written, /livez never
/// answered). None for install-phase failures where no proxy launch happened.
///
/// When `tracked_child` is false, the secondary fields below identify which
/// `ensure_headroom_running` short-circuit fired or whether the spawn errored
/// outright — without these, every "Stalled" / "NotStarted" event looks
/// identical in Sentry.
#[derive(Default, Clone)]
pub(crate) struct UpgradeBootDiagnostics {
    pub tracked_child: bool,
    pub new_proxy_log_written: bool,
    pub proxy_port_bound: bool,
    pub python_installed: bool,
    pub proxy_bypass: bool,
    pub pricing_allows_optimization: bool,
    pub runtime_paused: bool,
    pub ensure_error: Option<String>,
    /// Last ~100 lines of pip stdout/stderr from the install pass that
    /// produced the venv we're now booting. Pip can return exit 0 while
    /// leaving the venv broken (skipped packages, ABI-mismatched native
    /// deps); this tail is the only forensic record of what pip actually
    /// did. Empty string when no pip ran (e.g. requirements-repair).
    pub pip_output_tail: String,
}

/// Report a runtime upgrade failure to Sentry. `phase` is "install" for
/// pip/smoke-test failures, "boot_validation" for "installed but didn't boot".
/// `outcome` is the BootValidationOutcome label when phase is boot_validation.
pub(crate) fn capture_upgrade_failure(
    err: &anyhow::Error,
    restored: bool,
    phase: &str,
    outcome: Option<&str>,
    duration_ms: Option<u64>,
    target_version: Option<&str>,
    fallback_version: Option<&str>,
    log_tail: Option<&str>,
    boot_diagnostics: Option<UpgradeBootDiagnostics>,
) {
    let technical_err = format!("{err:#}");
    let cmd_failure = err
        .chain()
        .find_map(|e| e.downcast_ref::<process_runner::CommandFailure>());

    // ENOSPC is environmental — the user can't fix it by retrying, and the
    // pip log dump bloats Sentry with thousands of "Requirement already
    // satisfied" lines per report. Drop the Sentry capture; the user still
    // sees the disk-full hint via `classify_upgrade_error`, and the local
    // failure is recorded by the caller's `record_upgrade_failure` +
    // analytics::track_event.
    let cmd_stderr = cmd_failure.map(|f| f.stderr.as_str()).unwrap_or("");
    if is_disk_full_signal(&technical_err) || is_disk_full_signal(cmd_stderr) {
        log::warn!(
            "skipping Sentry capture for runtime_upgrade_failed ({phase}): disk full (ENOSPC)"
        );
        return;
    }

    // Sentry drops extras larger than ~16KB. Cap the tail aggressively so the
    // tail's tail (where the panic/error usually lives) survives.
    let log_tail_capped = log_tail.map(|s| {
        if s.len() > 12_000 {
            let cut = s.len() - 12_000;
            format!("[truncated {cut} bytes]\n...{}", &s[cut..])
        } else {
            s.to_string()
        }
    });

    let outcome_for_fingerprint = outcome.unwrap_or("none");
    let fingerprint: [&str; 3] = ["runtime_upgrade", phase, outcome_for_fingerprint];

    // Bake diagnostic fields into the message so they appear in the issue
    // title/preview without requiring a drill-down into tags. The first ~400
    // chars of the err chain are usually enough to disambiguate.
    let mut summary = format!("runtime_upgrade_failed ({phase})");
    if let Some(o) = outcome {
        summary.push_str(&format!(" outcome={o}"));
    }
    if let Some(d) = duration_ms {
        summary.push_str(&format!(" duration_ms={d}"));
    }
    let err_capped: String = technical_err.chars().take(400).collect();
    summary.push_str(&format!(" err={err_capped}"));

    let endpoint_protection_suspected = is_endpoint_protection_signal(&technical_err);

    sentry::with_scope(
        |scope| {
            scope.set_tag("flow", "runtime_upgrade");
            scope.set_tag("upgrade_phase", phase);
            scope.set_tag(
                "endpoint_protection_suspected",
                if endpoint_protection_suspected {
                    "true"
                } else {
                    "false"
                },
            );
            if let Some(o) = outcome {
                scope.set_tag("outcome", o);
            }
            if let Some(t) = target_version {
                scope.set_tag("target_version", t);
            }
            if let Some(f) = fallback_version {
                scope.set_tag("fallback_version", f);
            }
            scope.set_extra("rollback_restored", restored.into());
            scope.set_extra("error_chain", technical_err.clone().into());
            if let Some(d) = duration_ms {
                scope.set_extra("duration_ms", d.into());
            }
            if let Some(tail) = log_tail_capped.as_deref() {
                scope.set_extra("log_tail", tail.into());
            }
            if let Some(diag) = boot_diagnostics.as_ref() {
                scope.set_tag(
                    "tracked_child",
                    if diag.tracked_child { "true" } else { "false" },
                );
                scope.set_tag(
                    "new_proxy_log_written",
                    if diag.new_proxy_log_written {
                        "true"
                    } else {
                        "false"
                    },
                );
                scope.set_tag(
                    "proxy_port_bound",
                    if diag.proxy_port_bound {
                        "true"
                    } else {
                        "false"
                    },
                );
                scope.set_extra("tracked_child", diag.tracked_child.into());
                scope.set_extra("new_proxy_log_written", diag.new_proxy_log_written.into());
                scope.set_extra("proxy_port_bound", diag.proxy_port_bound.into());
                scope.set_extra("python_installed", diag.python_installed.into());
                scope.set_extra("proxy_bypass", diag.proxy_bypass.into());
                scope.set_extra(
                    "pricing_allows_optimization",
                    diag.pricing_allows_optimization.into(),
                );
                scope.set_extra("runtime_paused", diag.runtime_paused.into());
                if let Some(err) = diag.ensure_error.as_deref() {
                    scope.set_extra("ensure_headroom_running_error", err.into());
                }
                if !diag.pip_output_tail.is_empty() {
                    // Cap aggressively — Sentry drops extras > ~16KB and the
                    // tail (where pip warnings/skips/successfully-installed
                    // lines live) is the most informative part.
                    let tail = if diag.pip_output_tail.len() > 12_000 {
                        let cut = diag.pip_output_tail.len() - 12_000;
                        format!(
                            "[truncated {cut} bytes]\n...{}",
                            &diag.pip_output_tail[cut..]
                        )
                    } else {
                        diag.pip_output_tail.clone()
                    };
                    scope.set_extra("pip_install_output", tail.into());
                }
            }
            if let Some(failure) = cmd_failure {
                scope.set_extra("program", failure.program.clone().into());
                scope.set_extra("args", failure.args.join(" ").into());
                scope.set_extra(
                    "exit_code",
                    failure
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "signal".into())
                        .into(),
                );
                scope.set_extra(
                    "signal",
                    failure
                        .signal
                        .map(|s| s.to_string().into())
                        .unwrap_or(serde_json::Value::Null),
                );
                scope.set_extra("stdout", failure.stdout.clone().into());
                scope.set_extra("stderr", failure.stderr.clone().into());
            }
            scope.set_fingerprint(Some(fingerprint.as_slice()));
        },
        || {
            // Build the anyhow chain as exception values. With at least one
            // exception present, the AttachStacktraceIntegration attaches the
            // stacktrace to the exception rather than emitting a synthetic
            // thread frame full of sentry/backtrace internals.
            let mut exception_values: Vec<sentry::protocol::Exception> = err
                .chain()
                .map(|e| sentry::protocol::Exception {
                    ty: "anyhow::Error".to_string(),
                    value: Some(e.to_string()),
                    ..Default::default()
                })
                .collect();
            // Sentry convention: innermost cause first.
            exception_values.reverse();

            let event = sentry::protocol::Event {
                message: Some(summary.clone()),
                level: sentry::protocol::Level::Error,
                exception: exception_values.into(),
                ..Default::default()
            };
            sentry::capture_event(event);
        },
    );
}

/// High-confidence signatures that an install/runtime failure was caused by
/// endpoint-protection software (antivirus or EDR) blocking the freshly
/// installed native code. Conservative on purpose — we only match patterns
/// that are unlikely to surface from anything else, so the user-facing hint
/// stays trustworthy. If the matcher grows past ~6 patterns we should split
/// it by failure surface (install vs runtime) and consider tightening.
///
/// Input is matched case-insensitively.
pub(crate) fn is_endpoint_protection_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    // Apple's loader rejecting a fresh signature (codesign tampered or not
    // recognized by the kernel — almost always EDR injecting/rewriting).
    if lower.contains("code signature invalid")
        || lower.contains("code signature could not be verified")
    {
        return true;
    }
    // `dlopen` reports the "tried: ... (operation not permitted)" suffix when
    // a sandbox/AV blocks a freshly-extracted .so/.dylib. The "library not
    // loaded" prefix alone is too noisy (covers ordinary missing-dep cases),
    // so require the "not permitted" companion.
    if (lower.contains("library not loaded") || lower.contains("dlopen"))
        && lower.contains("not permitted")
    {
        return true;
    }
    // SIGKILL with no app-side cause is the classic EDR signature — the
    // process is killed before it can write a useful error. Plain "killed"
    // is too noisy (covers OOM, user pkill), so require the explicit signal
    // marker. CommandFailure formats this as "signal=9" or "Killed: 9".
    if lower.contains("signal=9") || lower.contains("killed: 9") || lower.contains("exit code 137")
    {
        return true;
    }
    // `Operation not permitted` paired with a freshly-installed native
    // extension path strongly implicates AV that hooks open(2)/exec(2). The
    // bare phrase appears in too many unrelated permission errors, so we
    // gate it on "site-packages" (where pip just wrote the file) or ".so" /
    // ".dylib" appearing in the same chain.
    if lower.contains("operation not permitted")
        && (lower.contains("site-packages") || lower.contains(".so") || lower.contains(".dylib"))
    {
        return true;
    }
    false
}

/// True when an install/upgrade failure was caused by the user's disk
/// running out of space. ENOSPC is environmental — the user can't fix it
/// by retrying, only by freeing space — so we use this to drop noisy
/// pip-log Sentry reports and emit a single clear local log line instead.
/// The user-facing hint is produced separately by `classify_upgrade_error`.
pub(crate) fn is_disk_full_signal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("no space left on device")
        || lower.contains("errno 28")
        || lower.contains("enospc")
        || lower.contains("disk full")
}

/// Shared hint copy for endpoint-protection failures. Two variants because
/// the install-time and runtime surfaces want slightly different "what to
/// do" wording (retry the install vs allow the runtime dir + click Retry).
const ENDPOINT_PROTECTION_HINT_INSTALL: &str =
    "Looks like endpoint protection (antivirus or EDR) blocked the new native code. \
     Allow Headroom in your security software, then retry.";

const ENDPOINT_PROTECTION_HINT_RUNTIME: &str =
    "A Headroom component was killed at launch — usually endpoint protection (antivirus or EDR) \
     interfering with freshly-installed code. Allow `~/Library/Application Support/Headroom` \
     in your security software, then click Retry.";

pub(crate) fn endpoint_protection_hint_install() -> String {
    ENDPOINT_PROTECTION_HINT_INSTALL.to_string()
}

pub(crate) fn endpoint_protection_hint_runtime() -> String {
    ENDPOINT_PROTECTION_HINT_RUNTIME.to_string()
}

/// Map common runtime-upgrade failure modes to a short user-facing hint.
pub(crate) fn classify_upgrade_error(err: &anyhow::Error) -> Option<String> {
    let chain_raw = format!("{err:#}");
    // Endpoint protection check uses the raw chain (the matcher does its own
    // case-folding) so signal patterns like "signal=9" match exactly.
    if is_endpoint_protection_signal(&chain_raw) {
        return Some(endpoint_protection_hint_install());
    }
    let chain = chain_raw.to_ascii_lowercase();
    if chain.contains("network")
        || chain.contains("timed out")
        || chain.contains("dns")
        || chain.contains("connection refused")
        || chain.contains("could not resolve")
    {
        return Some("Couldn't reach PyPI. Check your network and retry.".into());
    }
    if chain.contains("no space") || chain.contains("disk full") || chain.contains("enospc") {
        return Some(
            "Not enough disk space to install the update. Free up space and retry.".into(),
        );
    }
    if chain.contains("sha256") || chain.contains("checksum") || chain.contains("digest") {
        return Some("The downloaded wheel's checksum didn't match. Retry to redownload.".into());
    }
    if chain.contains("import") && chain.contains("smoke test") {
        return Some(
            "The new Headroom version couldn't be imported. Try retrying or reinstalling.".into(),
        );
    }
    if chain.contains("resolution") || chain.contains("no matching distribution") {
        return Some(
            "Pip couldn't resolve dependencies for the new version. Please report this.".into(),
        );
    }
    None
}
