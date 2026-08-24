use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::client_paths::{
    all_shell_paths, dedupe_paths, dedupe_strings, default_shell_targets_for_family,
    detect_shell_family, discover_managed_shell_targets, is_profile_file, shell_targets_from_state,
};
use crate::models::{SavingsMode, SwitchboardMode};
use crate::storage::{app_data_dir, config_file};

pub fn is_claude_code_enabled() -> bool {
    is_configured(&load_setup_state(), "claude_code")
}

pub fn is_codex_enabled() -> bool {
    is_configured(&load_setup_state(), "codex_cli")
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientSetupState {
    pub(crate) configured_clients: BTreeMap<String, String>,
    /// Snapshot of configured_clients taken at last pause/quit, used to restore on next startup.
    #[serde(default)]
    pub(crate) remembered_clients: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) managed_shell_files: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub(crate) remembered_shell_files: BTreeMap<String, Vec<String>>,
    /// User opted RTK out via the tool status toggle. When true, bootstrap and
    /// client setup skip re-adding the RTK PATH export and Claude Code hook.
    #[serde(default)]
    pub(crate) rtk_disabled: bool,
    #[serde(default)]
    pub(crate) switchboard_mode: Option<SwitchboardMode>,
    #[serde(default)]
    pub(crate) savings_mode: Option<SavingsMode>,
}

pub fn load_switchboard_mode() -> Option<SwitchboardMode> {
    load_setup_state().switchboard_mode
}

pub fn write_switchboard_mode(mode: SwitchboardMode) -> Result<()> {
    let mut state = load_setup_state();
    state.switchboard_mode = Some(mode);
    write_setup_state(&state)
}

pub fn load_savings_mode() -> SavingsMode {
    load_setup_state()
        .savings_mode
        .unwrap_or(SavingsMode::Balanced)
}

pub fn write_savings_mode(mode: SavingsMode) -> Result<()> {
    let mut state = load_setup_state();
    state.savings_mode = Some(mode);
    write_setup_state(&state)
}

pub(crate) fn is_configured(state: &ClientSetupState, client_id: &str) -> bool {
    configured_timestamp(state, client_id).is_some()
}

pub(crate) fn configured_timestamp(state: &ClientSetupState, client_id: &str) -> Option<String> {
    let primary = normalized_setup_id(client_id);
    state.configured_clients.get(primary).cloned()
}

pub fn load_setup_state() -> ClientSetupState {
    let path = setup_state_path();
    if !path.exists() {
        return ClientSetupState::default();
    }

    // The on-disk file is rewritten by other code paths in this module
    // (apply_client_setup, disable_client_setup, clear_client_setups). Even
    // though `write_setup_state` now publishes via tmp+rename, retry once
    // before giving up: a parse failure on an existing file is almost always
    // a transient race or a partially-written file from an older build, and
    // returning the empty default flips `is_claude_code_enabled` to false,
    // which the tray reads as "Claude Code disconnected" and notifies on.
    match try_load_setup_state(&path) {
        Ok(state) => normalize_setup_state(state),
        Err(first_err) => {
            std::thread::sleep(std::time::Duration::from_millis(15));
            match try_load_setup_state(&path) {
                Ok(state) => normalize_setup_state(state),
                Err(second_err) => {
                    log::warn!(
                        "load_setup_state: failed to read/parse {} twice ({first_err:#}; {second_err:#}); returning default",
                        path.display()
                    );
                    ClientSetupState::default()
                }
            }
        }
    }
}

fn try_load_setup_state(path: &Path) -> Result<ClientSetupState> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice::<ClientSetupState>(&bytes)
        .with_context(|| format!("parsing {}", path.display()))
}

pub(crate) fn normalize_setup_state(mut state: ClientSetupState) -> ClientSetupState {
    state.configured_clients = normalize_setup_entries(state.configured_clients);
    state.remembered_clients = normalize_setup_entries(state.remembered_clients);
    state.managed_shell_files = normalize_shell_file_entries(state.managed_shell_files);
    state.remembered_shell_files = normalize_shell_file_entries(state.remembered_shell_files);
    state
}

fn normalize_setup_entries(mut entries: BTreeMap<String, String>) -> BTreeMap<String, String> {
    // codex_gui is a removed id; codex/codex_cli are live again, keep them.
    entries.remove("codex_gui");

    entries
}

fn normalize_shell_file_entries(
    mut entries: BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    entries.remove("codex_gui");

    for files in entries.values_mut() {
        dedupe_strings(files);
    }

    entries
}

pub fn write_setup_state(state: &ClientSetupState) -> Result<()> {
    let path = setup_state_path();
    let payload = serde_json::to_vec_pretty(state).context("serializing client setup state")?;

    // Publish atomically: write to a sibling tmp file then rename. POSIX
    // rename is atomic, so concurrent readers (e.g. the tray-icon thread
    // calling `is_claude_code_enabled` every 2s) see either the old file or
    // the new one — never a half-written truncate. The previous direct
    // `fs::write` opened a microsecond window where readers parsed an empty
    // file, concluded no clients were configured, and flipped the tray to
    // "Disconnected" with a spurious notification.
    let tmp_path = {
        let mut s = path.clone().into_os_string();
        s.push(".tmp");
        PathBuf::from(s)
    };
    std::fs::write(&tmp_path, &payload)
        .with_context(|| format!("writing {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path)
        .with_context(|| format!("renaming {} -> {}", tmp_path.display(), path.display()))
}

pub(crate) fn setup_state_path() -> PathBuf {
    config_file(&app_data_dir(), "client-setup.json")
}

pub(crate) fn default_headroom_root_dir() -> PathBuf {
    app_data_dir().join("headroom")
}

pub(crate) fn default_headroom_rtk_path() -> PathBuf {
    default_headroom_root_dir().join("bin").join("rtk")
}

pub(crate) fn default_headroom_managed_python_path() -> PathBuf {
    default_headroom_root_dir()
        .join("runtime")
        .join("venv")
        .join("bin")
        .join("python3")
}

pub(crate) fn resolve_client_shell_targets(
    state: &ClientSetupState,
    client_id: &str,
) -> Result<Vec<PathBuf>> {
    let state_id = normalized_setup_id(client_id);
    let mut targets = shell_targets_from_state(state.managed_shell_files.get(state_id));
    if targets.is_empty() {
        targets = shell_targets_from_state(state.remembered_shell_files.get(state_id));
    }
    targets.extend(discover_managed_shell_targets(&[
        "claude_code",
        "managed_rtk",
        "codex_cli",
    ])?);

    let default_targets = default_shell_targets_for_family(detect_shell_family());
    if targets.is_empty() {
        targets = default_targets;
    } else {
        for file in default_targets {
            if is_profile_file(&file) {
                targets.push(file);
            }
        }
    }

    Ok(dedupe_paths(targets))
}

pub(crate) fn resolve_client_shell_targets_for_cleanup(
    state: &ClientSetupState,
    client_id: &str,
) -> Result<Vec<PathBuf>> {
    let mut targets = resolve_client_shell_targets(state, client_id)?;
    targets.extend(all_shell_paths());
    Ok(dedupe_paths(targets))
}
pub(crate) fn normalized_setup_id(client_id: &str) -> &str {
    match client_id {
        "codex" | "codex_gui" => "codex_cli",
        "vscode" => "claude_code",
        other => other,
    }
}
