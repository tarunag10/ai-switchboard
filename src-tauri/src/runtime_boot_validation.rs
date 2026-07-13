use std::path::Path;
use std::time::{Duration, SystemTime};

pub(crate) fn log_mtime_advanced(prev: Option<SystemTime>, current: Option<SystemTime>) -> bool {
    current.is_some() && current != prev
}

pub(crate) fn boot_validation_stalled(
    elapsed: Duration,
    activity_age: Duration,
    grace: Duration,
    silence: Duration,
) -> bool {
    elapsed > grace && activity_age > silence
}

pub(crate) fn newest_proxy_log_mtime(logs_dir: &Path) -> Option<SystemTime> {
    let entries = std::fs::read_dir(logs_dir).ok()?;
    let mut newest: Option<SystemTime> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("headroom-proxy") || !name_str.ends_with(".log") {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                newest = Some(match newest {
                    Some(prev) if prev > mtime => prev,
                    _ => mtime,
                });
            }
        }
    }
    newest
}

/// User-facing message shown during boot validation. Evolves with elapsed
/// time and whether the proxy log is actively being written to. Cycles
/// through a rotating set of sub-messages per phase so the UI never looks
/// frozen even when all phases last a while.
pub(crate) fn boot_validation_message(elapsed_secs: u64, active: bool) -> String {
    let prefix = if elapsed_secs < 10 {
        "Launching AI Switchboard".to_string()
    } else if elapsed_secs < 30 {
        if active {
            "Warming up Headroom's runtime".to_string()
        } else {
            "Launching AI Switchboard".to_string()
        }
    } else if elapsed_secs < 90 {
        // Rotate across a few descriptive phrasings so the line changes
        // every ~10 seconds instead of repeating identically.
        let rotation = (elapsed_secs / 10) % 3;
        match rotation {
            0 => "Preparing Headroom's ML subsystems".to_string(),
            1 => "Loading optimization pipeline".to_string(),
            _ => "Initializing caches and request handlers".to_string(),
        }
    } else if elapsed_secs < 240 {
        let rotation = (elapsed_secs / 15) % 3;
        match rotation {
            0 => "Downloading Headroom's ML models (first-run only)".to_string(),
            1 => "Fetching model weights from Hugging Face".to_string(),
            _ => "Preparing model caches for first-time use".to_string(),
        }
    } else {
        "Finishing up the first-run download — slower connections may take several more minutes"
            .to_string()
    };

    let hint = if active {
        " · activity detected"
    } else if elapsed_secs > 60 {
        " · this is normal for a first-time upgrade"
    } else {
        ""
    };

    format!("{prefix}… ({}s elapsed{})", elapsed_secs, hint)
}

/// Outcome of the boot-validation loop.
#[derive(Debug)]
pub enum BootValidationOutcome {
    /// Proxy reachable via /livez within the max timeout.
    Reachable,
    /// Proxy process exited before becoming reachable.
    ProcessExited,
    /// No log activity for long enough that we consider the proxy stalled.
    Stalled,
    /// Hit the absolute max without reachability or obvious failure.
    TimedOut,
    /// `ensure_headroom_running` short-circuited or errored — there is no
    /// tracked child to wait on AND no externally-reachable proxy on :6768.
    /// Reported instead of `Stalled` so we don't burn ~120s waiting for a
    /// process that was never going to start.
    NotStarted,
}

impl BootValidationOutcome {
    pub fn is_ok(&self) -> bool {
        matches!(self, BootValidationOutcome::Reachable)
    }

    pub fn label(&self) -> &'static str {
        match self {
            BootValidationOutcome::Reachable => "reachable",
            BootValidationOutcome::ProcessExited => "process_exited",
            BootValidationOutcome::Stalled => "stalled",
            BootValidationOutcome::TimedOut => "timed_out",
            BootValidationOutcome::NotStarted => "not_started",
        }
    }
}
