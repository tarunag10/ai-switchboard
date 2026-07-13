use std::path::PathBuf;

use chrono::Utc;

use crate::client_connectors::PLANNED_SIDECAR_SPECS;
use crate::client_paths::{
    all_shell_paths, claude_settings_candidates, claude_settings_path, codex_config_toml_path,
    grok_config_path, headroom_markitdown_hook_path, headroom_rtk_hook_path, home_dir,
    planned_sidecar_routing_path, rtk_codex_agents_path, shell_path, ALL_SHELL_FILES,
};
use crate::models::{
    ManagedFootprintItem, ManagedFootprintReport, UninstallDryRunReport, UninstallTarget,
};
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

pub(crate) fn managed_backup_targets() -> Vec<PathBuf> {
    let mut backup_targets: Vec<PathBuf> = claude_settings_candidates();
    backup_targets.push(headroom_rtk_hook_path());
    backup_targets.push(headroom_markitdown_hook_path());
    backup_targets.push(codex_config_toml_path());
    backup_targets.push(grok_config_path());
    backup_targets.push(
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("Code")
            .join("User")
            .join("settings.json"),
    );
    backup_targets.extend(all_shell_paths());
    backup_targets
}

pub(crate) fn uninstall_dry_run_report() -> UninstallDryRunReport {
    let targets = uninstall_targets();
    UninstallDryRunReport {
        generated_at: Utc::now(),
        removed_on_uninstall: targets
            .iter()
            .filter(|target| target.managed)
            .map(|target| target.path.clone())
            .collect(),
        preserved: vec![
            "User repositories and source files are never deleted.".to_string(),
            "Provider credentials, AWS credentials, SSO cache, and user profiles are not modified."
                .to_string(),
            "Unmanaged shell/profile content outside Switchboard marker blocks is preserved."
                .to_string(),
            "Legacy Headroom storage is preserved during migration, but removed during explicit uninstall."
                .to_string(),
        ],
        targets,
    }
}

fn uninstall_targets() -> Vec<UninstallTarget> {
    let mut targets = Vec::new();

    let home = home_dir();
    for settings_path in claude_settings_candidates() {
        push_uninstall_target(
            &mut targets,
            "claude-settings-hooks",
            "client-config",
            settings_path,
            true,
            "Strip managed Claude Code hook entries and routing keys only.",
            false,
            vec!["User-owned Claude settings remain in place.".to_string()],
        );
    }
    push_uninstall_target(
        &mut targets,
        "codex-config",
        "client-config",
        codex_config_toml_path(),
        true,
        "Remove managed Codex provider/routing blocks only.",
        false,
        vec!["User-owned Codex config remains in place.".to_string()],
    );
    push_uninstall_target(
        &mut targets,
        "grok-config",
        "client-config",
        grok_config_path(),
        true,
        "Remove only the Switchboard-managed Grok endpoint block; preserve user-owned Grok settings and credentials.",
        false,
        vec![
            "XAI_API_KEY, auth.json, account state, and model selection are never read or deleted."
                .to_string(),
        ],
    );
    push_uninstall_target(
        &mut targets,
        "codex-agents-rules",
        "client-config",
        rtk_codex_agents_path(),
        true,
        "Remove managed RTK/Caveman instruction blocks only.",
        false,
        vec!["Both headroom: and mac-ai-switchboard: marker blocks are recognized.".to_string()],
    );
    for shell_path in all_shell_paths() {
        push_uninstall_target(
            &mut targets,
            "shell-routing-blocks",
            "shell-profile",
            shell_path,
            true,
            "Remove managed shell export blocks only.",
            false,
            vec!["Unmanaged shell profile content is preserved.".to_string()],
        );
    }
    push_uninstall_target(
        &mut targets,
        "rtk-hook",
        "managed-hook",
        headroom_rtk_hook_path(),
        true,
        "Delete the managed RTK hook script.",
        false,
        Vec::new(),
    );
    push_uninstall_target(
        &mut targets,
        "markitdown-hook",
        "managed-hook",
        headroom_markitdown_hook_path(),
        true,
        "Delete the managed MarkItDown hook script.",
        false,
        Vec::new(),
    );
    push_uninstall_target(
        &mut targets,
        "app-support-current",
        "app-storage",
        app_data_dir(),
        true,
        "Delete AI Switchboard app support storage after explicit uninstall confirmation.",
        true,
        vec![
            "Contains local runtime state, logs, memory DB, and Repo Intelligence cache."
                .to_string(),
        ],
    );
    let app_support = home.join("Library").join("Application Support");
    push_uninstall_target(
        &mut targets,
        "app-support-legacy",
        "app-storage",
        app_support.join(LEGACY_STORAGE_DIR_NAME),
        true,
        "Delete legacy Headroom app support storage after explicit uninstall confirmation.",
        true,
        vec!["Migration keeps this folder intact until uninstall.".to_string()],
    );
    push_uninstall_target(
        &mut targets,
        "dot-headroom-runtime",
        "runtime",
        home.join(".headroom"),
        true,
        "Delete managed local runtime files.",
        true,
        Vec::new(),
    );

    extend_macos_uninstall_targets(&mut targets);

    for target in managed_backup_targets() {
        let Some(parent) = target.parent() else {
            continue;
        };
        let Some(file_name) = target.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        for prefix in [
            format!("{file_name}.headroom-backup-*"),
            format!("{file_name}.nommer-backup-*"),
        ] {
            push_uninstall_target(
                &mut targets,
                "managed-backups",
                "backup",
                parent.join(prefix),
                true,
                "Delete managed backup siblings created by Switchboard/Headroom.",
                false,
                vec!["Only matching backup file names are removed.".to_string()],
            );
        }
    }

    targets
}

fn push_uninstall_target(
    targets: &mut Vec<UninstallTarget>,
    id: &str,
    category: &str,
    path: PathBuf,
    managed: bool,
    action: &str,
    requires_confirmation: bool,
    notes: Vec<String>,
) {
    targets.push(UninstallTarget {
        id: id.to_string(),
        category: category.to_string(),
        exists: path.exists(),
        path: path.display().to_string(),
        managed,
        action: action.to_string(),
        requires_confirmation,
        notes,
    });
}

fn extend_macos_uninstall_targets(targets: &mut Vec<UninstallTarget>) {
    let home = home_dir();
    let lib = home.join("Library");
    let launch_agents_dir = lib.join("LaunchAgents");
    for name in [
        format!("{APP_BUNDLE_ID}.plist"),
        "Headroom.plist".to_string(),
    ] {
        push_uninstall_target(
            targets,
            "launch-agent",
            "launch-agent",
            launch_agents_dir.join(name),
            true,
            "Unload and delete managed login item launch agent.",
            false,
            Vec::new(),
        );
    }

    for bundle_id in [APP_BUNDLE_ID] {
        push_uninstall_target(
            targets,
            "preferences",
            "macos-app-data",
            lib.join("Preferences").join(format!("{bundle_id}.plist")),
            true,
            "Delete managed app preferences for this bundle ID.",
            false,
            Vec::new(),
        );
        push_uninstall_target(
            targets,
            "caches",
            "macos-app-data",
            lib.join("Caches").join(bundle_id),
            true,
            "Delete managed app cache data.",
            false,
            Vec::new(),
        );
        push_uninstall_target(
            targets,
            "webkit-data",
            "macos-app-data",
            lib.join("WebKit").join(bundle_id),
            true,
            "Delete managed WebKit data for the app.",
            false,
            Vec::new(),
        );
        push_uninstall_target(
            targets,
            "http-storage",
            "macos-app-data",
            lib.join("HTTPStorages").join(bundle_id),
            true,
            "Delete managed HTTP storage for the app.",
            false,
            Vec::new(),
        );
        push_uninstall_target(
            targets,
            "http-cookies",
            "macos-app-data",
            lib.join("HTTPStorages")
                .join(format!("{bundle_id}.binarycookies")),
            true,
            "Delete managed HTTP cookie storage for the app.",
            false,
            Vec::new(),
        );
        push_uninstall_target(
            targets,
            "saved-state",
            "macos-app-data",
            lib.join("Saved Application State")
                .join(format!("{bundle_id}.savedState")),
            true,
            "Delete managed saved application state.",
            false,
            Vec::new(),
        );
    }
    for log_dir in ["Headroom", "Mac AI Switchboard"] {
        push_uninstall_target(
            targets,
            "logs",
            "macos-app-data",
            lib.join("Logs").join(log_dir),
            true,
            "Delete managed application logs.",
            false,
            vec!["Logs may include local paths but not provider secrets by design.".to_string()],
        );
    }
    for label in known_keychain_entry_labels() {
        push_uninstall_target(
            targets,
            "keychain",
            "credential-store",
            PathBuf::from(label),
            true,
            "Delete known app-owned keychain entries.",
            false,
            vec!["Keychain values are never read or exported.".to_string()],
        );
    }
}

pub(crate) fn get_managed_footprint() -> ManagedFootprintReport {
    let mut items = Vec::new();
    let app_dir = app_data_dir();
    let legacy_app_dir = app_dir
        .parent()
        .map(|parent| parent.join(crate::storage::LEGACY_STORAGE_DIR_NAME))
        .unwrap_or_else(|| home_dir().join("Library/Application Support/Headroom"));

    push_footprint_item(
        &mut items,
        "app-storage",
        "storage",
        app_dir,
        true,
        "Primary app support storage for runtimes, receipts, logs, backups, and indexes.",
        true,
        vec![],
        vec!["Contains local state, not secrets values in this report.".to_string()],
    );
    push_footprint_item(
        &mut items,
        "legacy-storage",
        "storage",
        legacy_app_dir,
        true,
        "Preserved legacy Headroom storage copied forward during migration.",
        true,
        vec![],
        vec!["Left intact for compatibility; not deleted by migration.".to_string()],
    );
    push_footprint_item(
        &mut items,
        "claude-settings",
        "client_config",
        claude_settings_path(),
        false,
        "Claude Code settings may contain managed env and hook references.",
        true,
        vec!["*.headroom-backup-* next to edited config".to_string()],
        vec!["Report does not read or include setting values.".to_string()],
    );
    push_footprint_item(
        &mut items,
        "claude-rtk-hook",
        "client_config",
        headroom_rtk_hook_path(),
        true,
        "Managed Claude Code RTK PreToolUse hook.",
        true,
        vec!["*.headroom-backup-* next to edited hook".to_string()],
        vec![],
    );
    push_footprint_item(
        &mut items,
        "claude-markitdown-hook",
        "client_config",
        headroom_markitdown_hook_path(),
        true,
        "Managed Claude Code MarkItDown PreToolUse hook.",
        true,
        vec!["*.headroom-backup-* next to edited hook".to_string()],
        vec![],
    );
    push_footprint_item(
        &mut items,
        "codex-config",
        "client_config",
        codex_config_toml_path(),
        false,
        "Codex config may contain managed provider and routing blocks.",
        true,
        vec!["*.headroom-backup-* next to edited config".to_string()],
        vec!["Report does not read or include provider values.".to_string()],
    );

    for shell in ALL_SHELL_FILES {
        push_footprint_item(
            &mut items,
            &format!("shell-{shell}"),
            "shell_profile",
            shell_path(shell),
            false,
            "Shell profile may contain Switchboard-managed routing or RTK blocks.",
            true,
            vec!["*.headroom-backup-* next to edited shell profile".to_string()],
            vec![],
        );
    }

    for spec in PLANNED_SIDECAR_SPECS {
        if let Ok(path) = planned_sidecar_routing_path(spec.id) {
            push_footprint_item(
                &mut items,
                &format!("{}-sidecar", spec.id),
                "connector_sidecar",
                path,
                true,
                &format!("Managed {} routing-intent sidecar.", spec.name),
                true,
                vec!["*.headroom-backup-* next to edited sidecar".to_string()],
                vec!["Sidecar contains no account secrets by design.".to_string()],
            );
        }
    }

    push_footprint_item(
        &mut items,
        "app-log",
        "logs",
        crate::logging::log_path(),
        true,
        "Desktop app log file.",
        true,
        vec![],
        vec!["Logs may include local paths; copy report excludes log contents.".to_string()],
    );
    push_footprint_item(
        &mut items,
        "memory-db",
        "local_database",
        crate::storage::memory_db_path(&app_data_dir()),
        true,
        "Local memory database.",
        true,
        vec![],
        vec!["Database contents are not included in this report.".to_string()],
    );
    push_footprint_item(
        &mut items,
        "launch-agent",
        "launch_agent",
        home_dir().join("Library/LaunchAgents/com.tarunagarwal.mac-ai-switchboard.plist"),
        false,
        "Launch at login agent if enabled.",
        true,
        vec![],
        vec![],
    );
    for service in ["mac-ai-switchboard", "headroom-desktop", "headroom"] {
        items.push(ManagedFootprintItem {
            id: format!("keychain-{service}"),
            category: "keychain".to_string(),
            path: format!("Keychain service: {service}"),
            exists: false,
            managed: true,
            action:
                "May store app/session secrets under this service name; values are never reported."
                    .to_string(),
            reversible: true,
            backup_paths: vec![],
            notes: vec!["Existence is not probed to avoid touching secret material.".to_string()],
        });
    }

    ManagedFootprintReport {
        generated_at: Utc::now(),
        items,
    }
}

fn push_footprint_item(
    items: &mut Vec<ManagedFootprintItem>,
    id: &str,
    category: &str,
    path: PathBuf,
    managed: bool,
    action: &str,
    reversible: bool,
    backup_paths: Vec<String>,
    notes: Vec<String>,
) {
    items.push(ManagedFootprintItem {
        id: id.to_string(),
        category: category.to_string(),
        exists: path.exists(),
        path: path.display().to_string(),
        managed,
        action: action.to_string(),
        reversible,
        backup_paths,
        notes,
    });
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
