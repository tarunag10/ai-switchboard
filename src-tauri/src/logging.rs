// Panic-safe file logger.
//
// Background: macOS LaunchServices does not guarantee stderr is connected
// to a valid fd when it spawns the app to handle a URL scheme, file
// association, or login item. Rust's `eprintln!`/`println!` macros panic
// on write failure, and a panic that crosses an ObjC -> Rust callback
// (e.g. the deep-link handler) aborts the whole process.
//
// This logger writes to a file under the platform's log directory and
// forwards Warn/Error records to Sentry. All write failures are swallowed
// so a logging failure can never crash the app.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use log::{Level, Log, Metadata, Record, SetLoggerError};

const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const SENTRY_MESSAGE_CHAR_CAP: usize = 400;

struct FileLogger {
    file: Mutex<Option<File>>,
    path: PathBuf,
}

impl FileLogger {
    fn write_record(&self, record: &Record, display_level: Level) {
        let Ok(mut guard) = self.file.lock() else {
            return;
        };
        let Some(file) = guard.as_mut() else {
            return;
        };
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(
            file,
            "{ts} {level:<5} {target}: {msg}",
            level = display_level,
            target = record.target(),
            msg = record.args(),
        );
        let _ = file.flush();
    }

    fn rotate_if_needed(&self) {
        let metadata = match fs::metadata(&self.path) {
            Ok(m) => m,
            Err(_) => return,
        };
        if metadata.len() < MAX_LOG_BYTES {
            return;
        }
        let Ok(mut guard) = self.file.lock() else {
            return;
        };
        // Drop the current handle before renaming so Windows can't hold it open;
        // also necessary on macOS for log inspection while the app runs.
        *guard = None;
        let backup = self.path.with_extension("log.old");
        let _ = fs::remove_file(&backup);
        let _ = fs::rename(&self.path, &backup);
        if let Ok(f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            *guard = Some(f);
        }
    }
}

fn is_transient_transport_error(msg: &str) -> bool {
    msg.contains("error sending request")
        || msg.contains("dns error")
        || msg.contains("connection refused")
        || msg.contains("connection reset")
        || msg.contains("operation timed out")
        || msg.contains("network is unreachable")
        || msg.contains("os error 50") // macOS: Network is down
        || msg.contains("os error 51") // macOS: Network is unreachable
        || msg.contains("os error 65") // macOS: No route to host
}

// Non-2xx response from the update endpoint. Most commonly a transient 5xx
// from GitHub releases or a 404 during a tag-publish race — not actionable.
fn is_updater_endpoint_error(msg: &str) -> bool {
    msg.contains("update endpoint did not respond with a successful status code")
}

// Drop transient transport errors (offline laptop, flaky wifi, upstream blip)
// from Sentry. They still hit the local log file via write_record.
fn skip_sentry(target: &str, msg: &str) -> bool {
    if target.starts_with("tauri_plugin_updater") {
        return is_transient_transport_error(msg) || is_updater_endpoint_error(msg);
    }
    // proxy_intercept bypass forwarder: when CC is bypassing the local Python
    // proxy and we re-issue directly to api.anthropic.com, transient network
    // failures aren't actionable — client already gets a 502 and CC retries.
    if target.starts_with("mac_ai_switchboard_lib::proxy_intercept")
        && msg.starts_with("proxy_intercept bypass forward failed")
    {
        return is_transient_transport_error(msg);
    }
    // The accept loop self-heals: it backs off and keeps accepting. A transient
    // EMFILE (or similar) under load isn't actionable as a Sentry event.
    if target.starts_with("mac_ai_switchboard_lib::proxy_intercept")
        && msg.starts_with("[proxy_intercept] accept error")
    {
        return true;
    }
    // Kompress prefetch is best-effort; the proxy lazy-loads the model on first
    // request if this fails. These two variants carry no actionable detail (the
    // spawn error is rare and the restart self-heals on next request), so they
    // are pure noise. The "download error" variant is NOT suppressed — it
    // carries a classified cause and is the systemic signal worth tracking.
    if target.starts_with("mac_ai_switchboard_lib::state")
        && (msg.starts_with("kompress prefetch failed")
            || msg.starts_with("kompress prefetch: restart after download failed"))
    {
        return true;
    }
    // Uninstall cleanup is best-effort and races a still-exiting backend/proxy
    // that may re-create a file mid-walk ("Directory not empty"). The removal
    // is retried; a residual failure during teardown isn't actionable.
    if target.starts_with("mac_ai_switchboard_lib::client_adapters")
        && msg.starts_with("cleanup: removing")
    {
        return true;
    }
    false
}

impl Log for FileLogger {
    fn enabled(&self, _meta: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let msg = format!("{}", record.args());
        let demote = record.level() <= Level::Warn && skip_sentry(record.target(), &msg);
        let display_level = if demote && record.level() == Level::Error {
            Level::Warn
        } else {
            record.level()
        };

        if display_level <= Level::Warn {
            self.rotate_if_needed();
        }
        self.write_record(record, display_level);

        if record.level() <= Level::Warn {
            if demote {
                return;
            }
            let level = match record.level() {
                Level::Error => sentry::Level::Error,
                _ => sentry::Level::Warning,
            };
            let truncated: String = msg.chars().take(SENTRY_MESSAGE_CHAR_CAP).collect();
            sentry::capture_message(&truncated, level);
        }
    }

    fn flush(&self) {
        if let Ok(mut g) = self.file.lock() {
            if let Some(f) = g.as_mut() {
                let _ = f.flush();
            }
        }
    }
}

/// Initialize the global logger. Safe to call once at startup. Subsequent
/// calls return Err but do not panic.
pub fn init() -> Result<PathBuf, SetLoggerError> {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok();
    let logger = FileLogger {
        file: Mutex::new(file),
        path: path.clone(),
    };
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(log::LevelFilter::Debug);
    Ok(path)
}

#[cfg(target_os = "macos")]
pub(crate) fn log_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join("Library/Logs/Headroom/mac-ai-switchboard.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/mac-ai-switchboard.log"))
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn log_path() -> PathBuf {
    dirs::data_local_dir()
        .map(|d| d.join("headroom/mac-ai-switchboard.log"))
        .unwrap_or_else(|| std::env::temp_dir().join("mac-ai-switchboard.log"))
}

#[cfg(test)]
mod tests {
    use super::skip_sentry;

    #[test]
    fn skips_updater_transport_errors() {
        assert!(skip_sentry(
            "tauri_plugin_updater::updater",
            "failed to check for updates: error sending request for url (https://github.com/...)"
        ));
        assert!(skip_sentry(
            "tauri_plugin_updater",
            "dns error: failed to lookup address"
        ));
        assert!(skip_sentry(
            "tauri_plugin_updater::updater",
            "operation timed out"
        ));
    }

    #[test]
    fn skips_updater_endpoint_status_errors() {
        assert!(skip_sentry(
            "tauri_plugin_updater::updater",
            "update endpoint did not respond with a successful status code"
        ));
    }

    #[test]
    fn keeps_updater_non_transport_errors() {
        assert!(!skip_sentry(
            "tauri_plugin_updater::updater",
            "signature verification failed"
        ));
        assert!(!skip_sentry(
            "tauri_plugin_updater",
            "invalid release manifest"
        ));
    }

    #[test]
    fn keeps_other_targets() {
        assert!(!skip_sentry(
            "mac_ai_switchboard_lib::pricing",
            "error sending request: timeout"
        ));
        assert!(!skip_sentry("reqwest", "error sending request"));
    }

    #[test]
    fn skips_proxy_intercept_bypass_transport_errors() {
        assert!(skip_sentry(
            "mac_ai_switchboard_lib::proxy_intercept",
            "proxy_intercept bypass forward failed: error sending request for url (https://api.anthropic.com/v1/messages?beta=true)"
        ));
        assert!(skip_sentry(
            "mac_ai_switchboard_lib::proxy_intercept",
            "proxy_intercept bypass forward failed: dns error: failed to lookup address"
        ));
    }

    #[test]
    fn keeps_proxy_intercept_non_transport_errors() {
        assert!(!skip_sentry(
            "mac_ai_switchboard_lib::proxy_intercept",
            "proxy_intercept bypass forward failed: invalid header value"
        ));
        assert!(!skip_sentry(
            "mac_ai_switchboard_lib::proxy_intercept",
            "some other proxy_intercept warning"
        ));
    }

    #[test]
    fn skips_kompress_prefetch_best_effort_warnings() {
        assert!(skip_sentry(
            "mac_ai_switchboard_lib::state",
            "kompress prefetch failed: some error"
        ));
        assert!(skip_sentry(
            "mac_ai_switchboard_lib::state",
            "kompress prefetch: restart after download failed: boom"
        ));
    }

    #[test]
    fn skips_uninstall_cleanup_removal_warnings() {
        assert!(skip_sentry(
            "mac_ai_switchboard_lib::client_adapters",
            "cleanup: removing /Users/x/Library/Application Support/Headroom failed: Directory not empty (os error 66)"
        ));
    }

    #[test]
    fn keeps_kompress_prefetch_download_error() {
        // The classified-cause variant carries the systemic signal and must
        // reach Sentry.
        assert!(!skip_sentry(
            "mac_ai_switchboard_lib::state",
            "kompress prefetch download error: [network] Max retries exceeded"
        ));
    }

    #[test]
    fn keeps_other_state_warnings() {
        assert!(!skip_sentry(
            "mac_ai_switchboard_lib::state",
            "some other state warning"
        ));
    }
}
