use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::time::Duration;

use crate::client_footprint::{
    known_keychain_entries, managed_runtime_storage_paths, APP_BUNDLE_ID,
};
use crate::client_paths::home_dir;

/// `remove_dir_all`, retrying on transient `ENOTEMPTY`. A backend/proxy
/// process killed in `stop_headroom` may still flush a log line into the
/// directory tree mid-walk, re-creating an entry so the final `rmdir` fails
/// with "Directory not empty". A short backoff lets the writer finish.
fn remove_dir_all_retry(path: &Path) -> std::io::Result<()> {
    let mut last = Ok(());
    for attempt in 0..5 {
        match std::fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                last = Err(e);
                std::thread::sleep(Duration::from_millis(100 * (attempt + 1)));
            }
        }
    }
    last
}

pub(crate) fn remove_managed_runtime_storage() -> Vec<String> {
    let mut removed = Vec::new();
    for path in managed_runtime_storage_paths() {
        if !path.exists() {
            continue;
        }
        match remove_dir_all_retry(&path) {
            Ok(_) => removed.push(path.display().to_string()),
            Err(err) => log::warn!("cleanup: removing {} failed: {err}", path.display()),
        }
    }
    removed
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn remove_macos_app_state() -> Vec<String> {
    let mut removed = Vec::new();
    removed.extend(remove_macos_preferences());
    removed.extend(remove_macos_caches());
    removed.extend(remove_macos_logs());
    removed.extend(remove_macos_bundle_dirs());
    remove_known_keychain_entries();
    removed
}

#[cfg(all(not(target_os = "macos"), not(test)))]
pub(crate) fn remove_macos_app_state() -> Vec<String> {
    Vec::new()
}

/// Remove sibling backup files that `backup_if_exists` (or its predecessor
/// "nommer") created next to `target`. Filenames look like
/// `<basename>.headroom-backup-<timestamp>` and `<basename>.nommer-backup-<timestamp>`.
/// Returns the paths removed.
pub(crate) fn sweep_managed_backups(target: &Path) -> Vec<String> {
    let mut removed = Vec::new();
    let Some(parent) = target.parent() else {
        return removed;
    };
    let Some(file_name) = target.file_name().and_then(|n| n.to_str()) else {
        return removed;
    };
    let headroom_prefix = format!("{}.headroom-backup-", file_name);
    let nommer_prefix = format!("{}.nommer-backup-", file_name);

    let Ok(entries) = std::fs::read_dir(parent) else {
        return removed;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.starts_with(&headroom_prefix) && !name.starts_with(&nommer_prefix) {
            continue;
        }
        let path = entry.path();
        match std::fs::remove_file(&path) {
            Ok(_) => removed.push(path.display().to_string()),
            Err(err) => log::warn!("cleanup: removing {} failed: {err}", path.display()),
        }
    }
    removed
}

#[cfg(target_os = "macos")]
pub(crate) fn remove_macos_launch_agents() -> Vec<String> {
    let mut removed = Vec::new();
    let launch_agents_dir = home_dir().join("Library").join("LaunchAgents");

    // Bundle-id-style plist (tauri-plugin-autostart default) and the
    // "Headroom.plist" name some older local builds shipped. Either can exist.
    let candidates = [
        format!("{APP_BUNDLE_ID}.plist"),
        "Headroom.plist".to_string(),
    ];

    for name in candidates {
        let path = launch_agents_dir.join(name);
        if !path.exists() {
            continue;
        }
        // Best-effort unload before deletion so launchd forgets the job.
        let _ = Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&path)
            .output();
        match std::fs::remove_file(&path) {
            Ok(_) => removed.push(path.display().to_string()),
            Err(err) => log::warn!("cleanup: removing {} failed: {err}", path.display()),
        }
    }

    removed
}

#[cfg(any(target_os = "macos", test))]
fn remove_macos_preferences() -> Vec<String> {
    let mut removed = Vec::new();
    let prefs_dir = home_dir().join("Library").join("Preferences");
    let Ok(entries) = std::fs::read_dir(&prefs_dir) else {
        return removed;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.starts_with(APP_BUNDLE_ID) {
            continue;
        }
        let path = entry.path();
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        match result {
            Ok(_) => removed.push(path.display().to_string()),
            Err(err) => log::warn!("cleanup: removing {} failed: {err}", path.display()),
        }
    }
    removed
}

#[cfg(any(target_os = "macos", test))]
fn remove_macos_caches() -> Vec<String> {
    let mut removed = Vec::new();
    let caches_base = home_dir().join("Library").join("Caches");
    for bundle_id in [APP_BUNDLE_ID] {
        let caches_dir = caches_base.join(bundle_id);
        if caches_dir.exists() {
            match std::fs::remove_dir_all(&caches_dir) {
                Ok(_) => removed.push(caches_dir.display().to_string()),
                Err(err) => log::warn!("cleanup: removing {} failed: {err}", caches_dir.display()),
            }
        }
    }
    removed
}

#[cfg(any(target_os = "macos", test))]
fn remove_macos_logs() -> Vec<String> {
    let mut removed = Vec::new();
    let logs_base = home_dir().join("Library").join("Logs");
    for log_dir in ["Headroom", "Mac AI Switchboard"] {
        let logs_dir = logs_base.join(log_dir);
        if logs_dir.exists() {
            match std::fs::remove_dir_all(&logs_dir) {
                Ok(_) => removed.push(logs_dir.display().to_string()),
                Err(err) => log::warn!("cleanup: removing {} failed: {err}", logs_dir.display()),
            }
        }
    }
    removed
}

/// Sweep the per-bundle-id directories macOS creates for a GUI app outside the
/// Caches/Preferences locations already handled above: the WKWebView data
/// store, HTTP cookie/storage caches, and saved window state.
#[cfg(any(target_os = "macos", test))]
fn remove_macos_bundle_dirs() -> Vec<String> {
    let mut removed = Vec::new();
    let lib = home_dir().join("Library");
    for bundle_id in [APP_BUNDLE_ID] {
        let targets = [
            lib.join("WebKit").join(bundle_id),
            lib.join("HTTPStorages").join(bundle_id),
            lib.join("HTTPStorages")
                .join(format!("{bundle_id}.binarycookies")),
            lib.join("Saved Application State")
                .join(format!("{bundle_id}.savedState")),
        ];
        for path in targets {
            if !path.exists() {
                continue;
            }
            let result = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            match result {
                Ok(_) => removed.push(path.display().to_string()),
                Err(err) => log::warn!("cleanup: removing {} failed: {err}", path.display()),
            }
        }
    }
    removed
}

/// Delete every keychain entry Mac AI Switchboard is known to write. Accounts
/// are captured alongside services because macOS keychain queries require both.
pub(crate) fn remove_known_keychain_entries() {
    for (service, account) in known_keychain_entries() {
        if let Err(err) = crate::keychain::delete_secret(service, account) {
            log::warn!("cleanup: deleting keychain {service}/{account} failed: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    #[test]
    fn sweep_managed_backups_removes_headroom_and_nommer_siblings_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("settings.json");
        fs::write(&target, "{}").unwrap();

        let headroom_backup = tmp
            .path()
            .join("settings.json.headroom-backup-20260101000000");
        let nommer_backup = tmp
            .path()
            .join("settings.json.nommer-backup-20250101000000");
        let unrelated = tmp.path().join("settings.json.bak");
        let other_target_backup = tmp
            .path()
            .join("config.toml.headroom-backup-20260101000000");
        fs::write(&headroom_backup, "old").unwrap();
        fs::write(&nommer_backup, "older").unwrap();
        fs::write(&unrelated, "user-owned").unwrap();
        fs::write(&other_target_backup, "different file's backup").unwrap();

        let removed = super::sweep_managed_backups(&target);

        assert_eq!(removed.len(), 2, "removed: {removed:?}");
        assert!(!headroom_backup.exists(), "headroom backup should be gone");
        assert!(!nommer_backup.exists(), "nommer backup should be gone");
        assert!(unrelated.exists(), "unrelated .bak should survive");
        assert!(
            other_target_backup.exists(),
            "another file's backup should survive"
        );
        assert!(target.exists(), "target file itself should survive");
    }

    #[test]
    fn sweep_managed_backups_is_quiet_when_parent_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("does-not-exist").join("settings.json");
        let removed = super::sweep_managed_backups(&missing);
        assert!(removed.is_empty());
    }
}
