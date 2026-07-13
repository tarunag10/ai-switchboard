/// Turn a raw `last_startup_error` string into a short user-friendly
/// explanation plus a suggested next step. Returns `None` for shapes we don't
/// recognize, so callers can fall back to a generic "open logs" prompt.
pub(crate) fn classify_startup_error(raw: &str) -> Option<String> {
    if crate::is_endpoint_protection_signal(raw) {
        return Some(crate::endpoint_protection_hint_runtime());
    }
    if raw.contains("is occupied by a non-headroom process") {
        return Some(
            "A port Headroom needs is held by another app on your machine. \
             Reboot to clear stuck listeners, then relaunch AI Switchboard."
                .into(),
        );
    }
    if raw.contains("headroom proxy already running on port") {
        return Some(
            "A previous Headroom proxy is still running in the background. \
             Quit and relaunch AI Switchboard to reset it."
                .into(),
        );
    }
    if raw.contains("client-facing proxy on 127.0.0.1:6767 is not ready") {
        return Some(
            "The Headroom backend is running, but the client-facing proxy on \
             127.0.0.1:6767 is not accepting traffic. Click Repair runtime \
             to respawn the local proxy front door."
                .into(),
        );
    }
    if raw.contains("never opened port") {
        return Some(
            "The Headroom runtime took too long to start. \
             On first launch, macOS Gatekeeper can scan the bundled Python runtime for ~1-2 minutes. \
             Wait a moment and click Retry. If it keeps failing, open engine logs from Settings."
                .into(),
        );
    }
    if raw.contains("exited with status") && raw.contains("before opening port") {
        return Some(
            "The Headroom Python runtime crashed at startup. \
             Open engine logs from Settings to see the traceback, \
             or reinstall the runtime from Settings > Advanced."
                .into(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::classify_startup_error;

    #[test]
    fn classify_startup_error_port_timeout() {
        let raw = "unable to keep headroom running in background (prior attempts: \
            /Users/x/venv/bin/headroom proxy --port 6768 never opened port 6768 within 60000ms): \
            /Users/x/venv/bin/python3 -m headroom.proxy.server --port 6768 --no-http2 never opened port 6768 within 60000ms";
        let hint = classify_startup_error(raw).expect("timeout should classify");
        assert!(hint.contains("Gatekeeper"), "got: {hint}");
        assert!(hint.contains("Retry"));
    }

    #[test]
    fn classify_startup_error_python_crash() {
        let raw = "unable to keep headroom running in background (prior attempts: \
            /home/h/venv/bin/headroom proxy --port 6768 exited with status exit status: 1 before opening port 6768): \
            /home/h/venv/bin/python3 -m headroom.proxy.server --port 6768 exited with status exit status: 1 before opening port 6768";
        let hint = classify_startup_error(raw).expect("crash should classify");
        assert!(hint.contains("crashed at startup"), "got: {hint}");
        assert!(hint.contains("logs"));
    }

    #[test]
    fn classify_startup_error_foreign_port() {
        let raw =
            "port 6768 is occupied by a non-headroom process (pid 1234 node); cannot start proxy.";
        let hint = classify_startup_error(raw).expect("foreign port should classify");
        assert!(hint.contains("Reboot"), "got: {hint}");
    }

    #[test]
    fn classify_startup_error_foreign_port_with_fallback_exhausted() {
        let raw =
            "port 6768 is occupied by a non-headroom process (rapportd pid 594) and fallback ports 6769-6790 are also unavailable; cannot start proxy. Reboot to clear stuck listeners, then relaunch Mac AI Switchboard.";
        let hint = classify_startup_error(raw).expect("all-foreign should classify");
        assert!(hint.contains("Reboot"), "got: {hint}");
    }

    #[test]
    fn classify_startup_error_endpoint_protection_signal_kill() {
        let raw = "unable to keep headroom running in background (prior attempts: \
                   /Users/x/venv/bin/headroom proxy --port 6768 exited with signal=9): \
                   /Users/x/venv/bin/python3 -m headroom.proxy.server exited with signal=9";
        let hint = classify_startup_error(raw).expect("SIGKILL should classify");
        assert!(
            hint.contains("endpoint protection"),
            "expected EDR hint, got: {hint}"
        );
        assert!(hint.contains("Retry"), "hint should be actionable: {hint}");
    }

    #[test]
    fn classify_startup_error_endpoint_protection_dlopen_blocked() {
        let raw = "ImportError: dlopen(/Users/x/Library/Application Support/Headroom/headroom/runtime/venv/\
                   lib/python3.12/site-packages/torch/lib/libtorch.dylib, 0x0006): tried: '...' \
                   (operation not permitted)";
        let hint = classify_startup_error(raw).expect("dlopen-blocked should classify");
        assert!(
            hint.contains("endpoint protection"),
            "expected EDR hint, got: {hint}"
        );
    }

    #[test]
    fn classify_startup_error_endpoint_protection_takes_priority_over_port_path() {
        let raw = "unable to keep headroom running in background (prior attempts: \
                   /venv/bin/headroom proxy --port 6768 never opened port 6768 within 60000ms: \
                   Killed: 9)";
        let hint = classify_startup_error(raw).expect("should classify");
        assert!(
            hint.contains("endpoint protection"),
            "expected EDR to win over port hint, got: {hint}"
        );
    }

    #[test]
    fn classify_startup_error_handles_every_tool_manager_bail_format() {
        let raw = "port 6768 is occupied by a non-headroom process (rapportd pid 594) and fallback ports 6769-6790 are also unavailable; cannot start proxy. \
                   Reboot to clear stuck listeners, then relaunch Mac AI Switchboard.";
        assert!(
            classify_startup_error(raw).is_some(),
            "all-foreign bail must classify"
        );

        let raw = "headroom proxy already running on port 6768 (likely a stale process from a prior session). \
                   Run `lsof -iTCP:6768 -sTCP:LISTEN` to find and kill it, then retry.";
        assert!(
            classify_startup_error(raw).is_some(),
            "stale proxy bail must classify"
        );

        let raw = "never opened port 6770 within 60000ms";
        assert!(
            classify_startup_error(raw).is_some(),
            "spawn timeout must classify on any port"
        );

        let raw = "exited with status 1 before opening port 6770";
        assert!(
            classify_startup_error(raw).is_some(),
            "python crash must classify on any port"
        );
    }

    #[test]
    fn classify_startup_error_stale_headroom() {
        let raw = "headroom proxy already running on port 6768 (likely a stale process from a prior session).";
        let hint = classify_startup_error(raw).expect("stale should classify");
        assert!(hint.contains("relaunch"), "got: {hint}");
    }

    #[test]
    fn classify_startup_error_unknown_returns_none() {
        assert!(classify_startup_error("some other error").is_none());
    }
}
