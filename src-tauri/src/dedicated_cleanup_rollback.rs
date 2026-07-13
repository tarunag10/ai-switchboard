use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;

use crate::client_cleanup;
use crate::client_footprint;
use crate::models::{
    ManagedRollbackExecutionResult, ManagedRollbackExecutionStatus, ManagedRollbackPreview,
};
use crate::repo_intelligence;
use crate::state::AppState;

const REPO_INTELLIGENCE_ROLLBACK_RECORD_ID: &str = "repo-intelligence";
const REPO_INTELLIGENCE_ROLLBACK_OWNER: &str = "Repo Intelligence";
const REPO_INTELLIGENCE_ROLLBACK_MARKER: &str = "repo-intelligence-latest.json";
const REPO_INTELLIGENCE_ROLLBACK_CONFIRMATION: &str =
    "Clear repo-intelligence-latest.json for Repo Intelligence";
const LOGIN_ITEM_ROLLBACK_RECORD_ID: &str = "login-item";
const LOGIN_ITEM_ROLLBACK_OWNER: &str = "Launch at login";
const LOGIN_ITEM_ROLLBACK_MARKER: &str = "com.tarunagarwal.mac-ai-switchboard";
const LOGIN_ITEM_ROLLBACK_CONFIRMATION: &str =
    "Remove com.tarunagarwal.mac-ai-switchboard LaunchAgent for Launch at login";
const PLUGINS_ROLLBACK_RECORD_ID: &str = "plugins-backups";
const PLUGINS_ROLLBACK_OWNER: &str = "Add-ons";
const PLUGINS_ROLLBACK_MARKER: &str = "headroom:addon";
const PLUGINS_ROLLBACK_CONFIRMATION: &str = "Remove headroom:addon for Add-ons";
const MANAGED_STORAGE_ROLLBACK_RECORD_ID: &str = "managed-storage";
const MANAGED_STORAGE_ROLLBACK_OWNER: &str = "AI Switchboard runtime";
const MANAGED_STORAGE_ROLLBACK_MARKER: &str = "managed storage path";
const MANAGED_STORAGE_ROLLBACK_CONFIRMATION: &str =
    "Delete managed storage for AI Switchboard runtime";
const APP_STATE_ROLLBACK_RECORD_ID: &str = "app-state";
const APP_STATE_ROLLBACK_OWNER: &str = "AI Switchboard app state";
const APP_STATE_ROLLBACK_MARKER: &str = "com.tarunagarwal.mac-ai-switchboard";
const APP_STATE_ROLLBACK_CONFIRMATION: &str =
    "Delete com.tarunagarwal.mac-ai-switchboard app state";

fn display_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn any_path_exists(paths: &[PathBuf]) -> bool {
    paths.iter().any(|path| path.exists())
}

fn repo_intelligence_summary_path_string() -> String {
    repo_intelligence::latest_summary_path()
        .display()
        .to_string()
}

fn launch_agent_cleanup_paths() -> Vec<PathBuf> {
    let launch_agents_dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("Library")
        .join("LaunchAgents");
    vec![
        launch_agents_dir.join("com.tarunagarwal.mac-ai-switchboard.plist"),
        launch_agents_dir.join("Headroom.plist"),
    ]
}

fn launch_agent_target_summary(paths: &[PathBuf]) -> String {
    display_paths(paths)
}

fn remove_launch_agent_files(paths: &[PathBuf]) -> Result<Vec<String>, String> {
    let mut removed = Vec::new();
    for path in paths {
        if !path.exists() {
            continue;
        }
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("launchctl")
                .args(["unload", "-w"])
                .arg(path)
                .output();
        }
        std::fs::remove_file(path)
            .map_err(|err| format!("removing {} failed: {err}", path.display()))?;
        removed.push(path.display().to_string());
    }
    Ok(removed)
}

pub(crate) fn preview_dedicated_cleanup_rollback_inner(
    state: Option<&AppState>,
    record_id: String,
) -> Result<ManagedRollbackPreview, String> {
    match record_id.as_str() {
        REPO_INTELLIGENCE_ROLLBACK_RECORD_ID => {
            let target_path = repo_intelligence::latest_summary_path();
            let marker_present = target_path.exists();
            Ok(ManagedRollbackPreview {
                record_id,
                owner: REPO_INTELLIGENCE_ROLLBACK_OWNER.to_string(),
                target_path: target_path.display().to_string(),
                marker: REPO_INTELLIGENCE_ROLLBACK_MARKER.to_string(),
                backup_path: None,
                marker_present,
                backup_exists: true,
                status: if marker_present {
                    ManagedRollbackExecutionStatus::Ready
                } else {
                    ManagedRollbackExecutionStatus::Blocked
                },
                confirmation_phrase: REPO_INTELLIGENCE_ROLLBACK_CONFIRMATION.to_string(),
                proposed_action:
                    "Remove only the Switchboard-managed Repo Intelligence latest-summary metadata."
                        .to_string(),
                blocked_reason: if marker_present {
                    None
                } else {
                    Some("No saved Repo Intelligence summary exists in managed storage.".to_string())
                },
                evidence: vec![
                    "Dedicated cleanup row: repo-intelligence.".to_string(),
                    "Cleanup calls the existing Clear index path used by Doctor and the Repo Intelligence add-on card.".to_string(),
                    "User repositories are not modified; only Switchboard managed summary metadata is removed.".to_string(),
                ],
            })
        }
        LOGIN_ITEM_ROLLBACK_RECORD_ID => {
            let paths = launch_agent_cleanup_paths();
            let marker_present = paths.iter().any(|path| path.exists());
            Ok(ManagedRollbackPreview {
                record_id,
                owner: LOGIN_ITEM_ROLLBACK_OWNER.to_string(),
                target_path: launch_agent_target_summary(&paths),
                marker: LOGIN_ITEM_ROLLBACK_MARKER.to_string(),
                backup_path: None,
                marker_present,
                backup_exists: true,
                status: if marker_present {
                    ManagedRollbackExecutionStatus::Ready
                } else {
                    ManagedRollbackExecutionStatus::Blocked
                },
                confirmation_phrase: LOGIN_ITEM_ROLLBACK_CONFIRMATION.to_string(),
                proposed_action:
                    "Disable app autostart and remove only Switchboard-managed LaunchAgent plist files."
                        .to_string(),
                blocked_reason: if marker_present {
                    None
                } else {
                    Some("No app-managed LaunchAgent plist exists in managed locations.".to_string())
                },
                evidence: vec![
                    "Dedicated cleanup row: login-item.".to_string(),
                    "Cleanup targets only com.tarunagarwal.mac-ai-switchboard.plist and legacy Headroom.plist in ~/Library/LaunchAgents.".to_string(),
                    "launchctl unload is best-effort on macOS before file removal.".to_string(),
                ],
            })
        }
        PLUGINS_ROLLBACK_RECORD_ID => {
            let receipt_present = state
                .map(|state| state.tool_manager.ponytail_receipt_exists())
                .unwrap_or(false);
            let registered_hosts = state
                .map(|state| state.tool_manager.ponytail_registered_hosts())
                .unwrap_or_default();
            Ok(ManagedRollbackPreview {
                record_id,
                owner: PLUGINS_ROLLBACK_OWNER.to_string(),
                target_path: "Ponytail plugin receipt and app-managed host registrations"
                    .to_string(),
                marker: PLUGINS_ROLLBACK_MARKER.to_string(),
                backup_path: None,
                marker_present: receipt_present,
                backup_exists: true,
                status: if receipt_present {
                    ManagedRollbackExecutionStatus::Ready
                } else {
                    ManagedRollbackExecutionStatus::Blocked
                },
                confirmation_phrase: PLUGINS_ROLLBACK_CONFIRMATION.to_string(),
                proposed_action:
                    "Remove only the Switchboard-receipted Ponytail plugin registration."
                        .to_string(),
                blocked_reason: if receipt_present {
                    None
                } else {
                    Some(
                        "No Switchboard Ponytail receipt exists; plugin config may be user-owned."
                            .to_string(),
                    )
                },
                evidence: vec![
                    "Dedicated cleanup row: plugins-backups.".to_string(),
                    "Cleanup calls the existing Ponytail uninstall path, which is no-op without the app receipt.".to_string(),
                    format!(
                        "Currently registered hosts: {}.",
                        if registered_hosts.is_empty() {
                            "none detected".to_string()
                        } else {
                            registered_hosts.join(", ")
                        }
                    ),
                    "Managed backup-file sweeping remains manual until a stricter backup allowlist exists.".to_string(),
                ],
            })
        }
        MANAGED_STORAGE_ROLLBACK_RECORD_ID => {
            let paths = client_footprint::managed_runtime_storage_paths();
            let marker_present = any_path_exists(&paths);
            Ok(ManagedRollbackPreview {
                record_id,
                owner: MANAGED_STORAGE_ROLLBACK_OWNER.to_string(),
                target_path: display_paths(&paths),
                marker: MANAGED_STORAGE_ROLLBACK_MARKER.to_string(),
                backup_path: None,
                marker_present,
                backup_exists: true,
                status: if marker_present {
                    ManagedRollbackExecutionStatus::Ready
                } else {
                    ManagedRollbackExecutionStatus::Blocked
                },
                confirmation_phrase: MANAGED_STORAGE_ROLLBACK_CONFIRMATION.to_string(),
                proposed_action:
                    "Delete only Switchboard-managed runtime storage and legacy runtime folders."
                        .to_string(),
                blocked_reason: if marker_present {
                    None
                } else {
                    Some("No managed runtime storage paths exist.".to_string())
                },
                evidence: vec![
                    "Dedicated cleanup row: managed-storage.".to_string(),
                    "Cleanup targets app support storage, legacy Headroom app support storage, and ~/.headroom runtime files.".to_string(),
                    "User repositories, provider credentials, shell profiles, LaunchAgents, and app preferences are not modified by this row.".to_string(),
                ],
            })
        }
        APP_STATE_ROLLBACK_RECORD_ID => {
            let paths = client_footprint::macos_app_state_paths();
            let keychain_labels = client_footprint::known_keychain_entry_labels();
            let marker_present = any_path_exists(&paths);
            Ok(ManagedRollbackPreview {
                record_id,
                owner: APP_STATE_ROLLBACK_OWNER.to_string(),
                target_path: if keychain_labels.is_empty() {
                    display_paths(&paths)
                } else {
                    format!(
                        "{}; {}",
                        display_paths(&paths),
                        keychain_labels.join(", ")
                    )
                },
                marker: APP_STATE_ROLLBACK_MARKER.to_string(),
                backup_path: None,
                marker_present,
                backup_exists: true,
                status: if marker_present {
                    ManagedRollbackExecutionStatus::Ready
                } else {
                    ManagedRollbackExecutionStatus::Blocked
                },
                confirmation_phrase: APP_STATE_ROLLBACK_CONFIRMATION.to_string(),
                proposed_action:
                    "Delete only Switchboard app preferences, caches, logs, WebKit/HTTP storage, saved state, and known keychain entries."
                        .to_string(),
                blocked_reason: if marker_present {
                    None
                } else {
                    Some("No Switchboard app-state files exist in managed macOS locations.".to_string())
                },
                evidence: vec![
                    "Dedicated cleanup row: app-state.".to_string(),
                    "Cleanup targets app bundle-id preferences, caches, logs, WebKit/HTTP storage, saved state, and known app keychain labels.".to_string(),
                    "Runtime storage, user repositories, shell profiles, provider configs, and LaunchAgents are not modified by this row.".to_string(),
                ],
            })
        }
        _ => Err(format!(
            "Dedicated cleanup rollback is currently enabled only for {REPO_INTELLIGENCE_ROLLBACK_RECORD_ID}, {LOGIN_ITEM_ROLLBACK_RECORD_ID}, {PLUGINS_ROLLBACK_RECORD_ID}, {MANAGED_STORAGE_ROLLBACK_RECORD_ID}, and {APP_STATE_ROLLBACK_RECORD_ID}."
        )),
    }
}

pub(crate) fn execute_dedicated_cleanup_rollback_inner(
    state: Option<&AppState>,
    record_id: String,
    confirmation_phrase: String,
) -> Result<ManagedRollbackExecutionResult, String> {
    let preview = preview_dedicated_cleanup_rollback_inner(state, record_id.clone())?;
    if confirmation_phrase != preview.confirmation_phrase {
        return Err("Rollback confirmation phrase does not match.".to_string());
    }
    if preview.status != ManagedRollbackExecutionStatus::Ready {
        return Err(preview
            .blocked_reason
            .unwrap_or_else(|| "Dedicated cleanup rollback is not ready.".to_string()));
    }

    match record_id.as_str() {
        REPO_INTELLIGENCE_ROLLBACK_RECORD_ID => {
            repo_intelligence::clear_latest_summary().map(|_| ()).map_err(|err| err.to_string())?;
            let still_present = repo_intelligence::latest_summary_path().exists();
            if still_present {
                return Err("Repo Intelligence summary is still present after cleanup.".to_string());
            }

            Ok(ManagedRollbackExecutionResult {
                record_id,
                owner: REPO_INTELLIGENCE_ROLLBACK_OWNER.to_string(),
                target_path: repo_intelligence_summary_path_string(),
                restored_from:
                    "Switchboard-managed Repo Intelligence latest-summary metadata removed."
                        .to_string(),
                safety_backup_path: None,
                marker: REPO_INTELLIGENCE_ROLLBACK_MARKER.to_string(),
                verification: vec![
                    "Exact cleanup confirmation phrase matched.".to_string(),
                    "Existing Clear index cleanup path completed.".to_string(),
                    "Repo Intelligence latest-summary file was absent after cleanup.".to_string(),
                    "User repositories were not modified by this cleanup.".to_string(),
                ],
            })
        }
        LOGIN_ITEM_ROLLBACK_RECORD_ID => {
            let paths = launch_agent_cleanup_paths();
            let removed = remove_launch_agent_files(&paths)?;
            if paths.iter().any(|path| path.exists()) {
                return Err("A managed LaunchAgent plist is still present after cleanup.".to_string());
            }
            Ok(ManagedRollbackExecutionResult {
                record_id,
                owner: LOGIN_ITEM_ROLLBACK_OWNER.to_string(),
                target_path: launch_agent_target_summary(&paths),
                restored_from: if removed.is_empty() {
                    "No managed LaunchAgent plist was present.".to_string()
                } else {
                    format!("Removed managed LaunchAgent plist files: {}", removed.join(", "))
                },
                safety_backup_path: None,
                marker: LOGIN_ITEM_ROLLBACK_MARKER.to_string(),
                verification: vec![
                    "Exact cleanup confirmation phrase matched.".to_string(),
                    "Tauri autostart disable was requested before file cleanup.".to_string(),
                    "Managed LaunchAgent plist files were absent after cleanup.".to_string(),
                    "No shell, client, repo, Keychain, or runtime storage files were modified by this cleanup.".to_string(),
                ],
            })
        }
        PLUGINS_ROLLBACK_RECORD_ID => {
            let state = state.ok_or_else(|| {
                "Plugin cleanup requires app state to access the managed Ponytail receipt."
                    .to_string()
            })?;
            let before_hosts = state.tool_manager.ponytail_registered_hosts();
            state
                .tool_manager
                .uninstall_ponytail()
                .map_err(|err| err.to_string())?;
            if state.tool_manager.ponytail_receipt_exists() {
                return Err("Ponytail receipt is still present after cleanup.".to_string());
            }
            Ok(ManagedRollbackExecutionResult {
                record_id,
                owner: PLUGINS_ROLLBACK_OWNER.to_string(),
                target_path: "Ponytail plugin receipt and app-managed host registrations"
                    .to_string(),
                restored_from: "Switchboard-receipted Ponytail plugin registration removed."
                    .to_string(),
                safety_backup_path: None,
                marker: PLUGINS_ROLLBACK_MARKER.to_string(),
                verification: vec![
                    "Exact cleanup confirmation phrase matched.".to_string(),
                    "Existing Ponytail uninstall path completed.".to_string(),
                    "Ponytail receipt was absent after cleanup.".to_string(),
                    format!(
                        "Previously registered hosts: {}.",
                        if before_hosts.is_empty() {
                            "none detected".to_string()
                        } else {
                            before_hosts.join(", ")
                        }
                    ),
                    "No add-on backup files were swept without a stricter allowlist.".to_string(),
                ],
            })
        }
        MANAGED_STORAGE_ROLLBACK_RECORD_ID => {
            let paths = client_footprint::managed_runtime_storage_paths();
            let removed = client_cleanup::remove_managed_runtime_storage();
            if any_path_exists(&paths) {
                return Err("A managed runtime storage path is still present after cleanup.".to_string());
            }
            Ok(ManagedRollbackExecutionResult {
                record_id,
                owner: MANAGED_STORAGE_ROLLBACK_OWNER.to_string(),
                target_path: display_paths(&paths),
                restored_from: if removed.is_empty() {
                    "No managed runtime storage path was present.".to_string()
                } else {
                    format!("Removed managed runtime storage paths: {}", removed.join(", "))
                },
                safety_backup_path: None,
                marker: MANAGED_STORAGE_ROLLBACK_MARKER.to_string(),
                verification: vec![
                    "Exact cleanup confirmation phrase matched.".to_string(),
                    "Managed runtime storage cleanup completed.".to_string(),
                    "App support storage, legacy storage, and ~/.headroom were absent after cleanup.".to_string(),
                    "App preferences, keychain entries, LaunchAgents, shell profiles, provider configs, and user repositories were not modified by this row.".to_string(),
                ],
            })
        }
        APP_STATE_ROLLBACK_RECORD_ID => {
            let paths = client_footprint::macos_app_state_paths();
            let removed = client_cleanup::remove_macos_app_state();
            if any_path_exists(&paths) {
                return Err("A managed app-state path is still present after cleanup.".to_string());
            }
            Ok(ManagedRollbackExecutionResult {
                record_id,
                owner: APP_STATE_ROLLBACK_OWNER.to_string(),
                target_path: display_paths(&paths),
                restored_from: if removed.is_empty() {
                    "No managed app-state file was present.".to_string()
                } else {
                    format!("Removed managed app-state paths: {}", removed.join(", "))
                },
                safety_backup_path: None,
                marker: APP_STATE_ROLLBACK_MARKER.to_string(),
                verification: vec![
                    "Exact cleanup confirmation phrase matched.".to_string(),
                    "App-state cleanup completed.".to_string(),
                    "Managed app preferences, caches, logs, WebKit/HTTP storage, and saved state were absent after cleanup.".to_string(),
                    "Known app keychain labels were requested for deletion without reading secret values.".to_string(),
                    "Runtime storage, LaunchAgents, shell profiles, provider configs, and user repositories were not modified by this row.".to_string(),
                ],
            })
        }
        _ => Err(format!(
            "Dedicated cleanup rollback is currently enabled only for {REPO_INTELLIGENCE_ROLLBACK_RECORD_ID}, {LOGIN_ITEM_ROLLBACK_RECORD_ID}, {PLUGINS_ROLLBACK_RECORD_ID}, {MANAGED_STORAGE_ROLLBACK_RECORD_ID}, and {APP_STATE_ROLLBACK_RECORD_ID}."
        )),
    }
}

pub(crate) fn is_login_item_record(record_id: &str) -> bool {
    record_id == LOGIN_ITEM_ROLLBACK_RECORD_ID
}
