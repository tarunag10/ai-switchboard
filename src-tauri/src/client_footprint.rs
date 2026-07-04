use std::path::PathBuf;

use crate::client_paths::home_dir;
use crate::storage::{app_data_dir, LEGACY_STORAGE_DIR_NAME};

pub(crate) const APP_BUNDLE_ID: &str = "com.tarunagarwal.mac-ai-switchboard";

pub(crate) fn managed_runtime_storage_paths() -> Vec<PathBuf> {
    let app_dir = app_data_dir();
    let legacy_dir = app_dir
        .parent()
        .map(|parent| parent.join(LEGACY_STORAGE_DIR_NAME))
        .unwrap_or_else(|| {
            home_dir()
                .join("Library")
                .join("Application Support")
                .join(LEGACY_STORAGE_DIR_NAME)
        });
    vec![app_dir, legacy_dir, home_dir().join(".headroom")]
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn macos_app_state_paths() -> Vec<PathBuf> {
    let lib = home_dir().join("Library");
    let mut paths = vec![
        lib.join("Caches").join(APP_BUNDLE_ID),
        lib.join("WebKit").join(APP_BUNDLE_ID),
        lib.join("HTTPStorages").join(APP_BUNDLE_ID),
        lib.join("HTTPStorages")
            .join(format!("{APP_BUNDLE_ID}.binarycookies")),
        lib.join("Saved Application State")
            .join(format!("{APP_BUNDLE_ID}.savedState")),
    ];
    for log_dir in ["Headroom", "Mac AI Switchboard"] {
        paths.push(lib.join("Logs").join(log_dir));
    }
    let prefs_dir = lib.join("Preferences");
    if let Ok(entries) = std::fs::read_dir(&prefs_dir) {
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with(APP_BUNDLE_ID) {
                paths.push(entry.path());
            }
        }
    } else {
        paths.push(prefs_dir.join(format!("{APP_BUNDLE_ID}.plist")));
    }
    paths
}

#[cfg(all(not(target_os = "macos"), not(test)))]
pub(crate) fn macos_app_state_paths() -> Vec<PathBuf> {
    Vec::new()
}

pub(crate) fn known_keychain_entry_labels() -> Vec<String> {
    known_keychain_entries()
        .iter()
        .map(|(service, account)| format!("keychain://{service}/{account}"))
        .collect()
}

/// Every keychain entry Mac AI Switchboard is known to write. Accounts are
/// captured alongside services because macOS keychain queries require both.
pub(crate) fn known_keychain_entries() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "com.tarunagarwal.mac-ai-switchboard.account",
            "session-token",
        ),
        (
            "com.tarunagarwal.mac-ai-switchboard.device",
            "machine-id-digest",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_runtime_storage_paths_include_current_legacy_and_dot_headroom() {
        let paths = managed_runtime_storage_paths();
        let rendered = paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Mac AI Switchboard"));
        assert!(rendered.contains("Headroom"));
        assert!(rendered.contains(".headroom"));
    }

    #[test]
    fn app_state_and_keychain_inventory_uses_bundle_id_without_secret_values() {
        let app_state = macos_app_state_paths()
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let labels = known_keychain_entry_labels();

        assert!(app_state.contains(APP_BUNDLE_ID));
        assert!(labels.iter().all(|label| label.starts_with("keychain://")));
        assert!(labels.iter().all(|label| !label.contains("Bearer")));
    }
}
