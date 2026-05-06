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
    fn write_record(&self, record: &Record) {
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
            level = record.level(),
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

// Drop transient transport errors (offline laptop, flaky wifi, upstream blip)
// from Sentry. They still hit the local log file via write_record.
fn skip_sentry(target: &str, msg: &str) -> bool {
    if target.starts_with("tauri_plugin_updater") {
        return is_transient_transport_error(msg);
    }
    // proxy_intercept bypass forwarder: when CC is bypassing the local Python
    // proxy and we re-issue directly to api.anthropic.com, transient network
    // failures aren't actionable — client already gets a 502 and CC retries.
    if target.starts_with("headroom_desktop_lib::proxy_intercept")
        && msg.starts_with("proxy_intercept bypass forward failed")
    {
        return is_transient_transport_error(msg);
    }
    false
}

impl Log for FileLogger {
    fn enabled(&self, _meta: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if record.level() <= Level::Warn {
            self.rotate_if_needed();
        }
        self.write_record(record);

        if record.level() <= Level::Warn {
            let msg = format!("{}", record.args());
            if skip_sentry(record.target(), &msg) {
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
fn log_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join("Library/Logs/Headroom/headroom-desktop.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/headroom-desktop.log"))
}

#[cfg(not(target_os = "macos"))]
fn log_path() -> PathBuf {
    dirs::data_local_dir()
        .map(|d| d.join("headroom/headroom-desktop.log"))
        .unwrap_or_else(|| std::env::temp_dir().join("headroom-desktop.log"))
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
            "headroom_desktop_lib::pricing",
            "error sending request: timeout"
        ));
        assert!(!skip_sentry("reqwest", "error sending request"));
    }

    #[test]
    fn skips_proxy_intercept_bypass_transport_errors() {
        assert!(skip_sentry(
            "headroom_desktop_lib::proxy_intercept",
            "proxy_intercept bypass forward failed: error sending request for url (https://api.anthropic.com/v1/messages?beta=true)"
        ));
        assert!(skip_sentry(
            "headroom_desktop_lib::proxy_intercept",
            "proxy_intercept bypass forward failed: dns error: failed to lookup address"
        ));
    }

    #[test]
    fn keeps_proxy_intercept_non_transport_errors() {
        assert!(!skip_sentry(
            "headroom_desktop_lib::proxy_intercept",
            "proxy_intercept bypass forward failed: invalid header value"
        ));
        assert!(!skip_sentry(
            "headroom_desktop_lib::proxy_intercept",
            "some other proxy_intercept warning"
        ));
    }
}
