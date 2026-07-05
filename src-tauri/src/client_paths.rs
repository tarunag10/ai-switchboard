use anyhow::{anyhow, Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::client_connectors::planned_sidecar_spec;

pub(crate) const SWITCHBOARD_ROUTING_FILE: &str = "mac-ai-switchboard-routing.md";
pub(crate) const OPENCODE_CONFIG_FILE: &str = "opencode.json";
pub(crate) const WINDSURF_CONFIG_FILE: &str = "settings.json";
pub(crate) const ZED_CONFIG_FILE: &str = "settings.json";
pub(crate) const ZSH_PROFILE_FILE: &str = ".zprofile";
pub(crate) const ZSH_RC_FILE: &str = ".zshrc";
pub(crate) const BASH_PROFILE_FILE: &str = ".bash_profile";
pub(crate) const BASH_LOGIN_FILE: &str = ".bash_login";
pub(crate) const POSIX_PROFILE_FILE: &str = ".profile";
pub(crate) const BASH_RC_FILE: &str = ".bashrc";
pub(crate) const ALL_SHELL_FILES: [&str; 6] = [
    ZSH_PROFILE_FILE,
    ZSH_RC_FILE,
    BASH_PROFILE_FILE,
    BASH_LOGIN_FILE,
    POSIX_PROFILE_FILE,
    BASH_RC_FILE,
];

pub(crate) fn home_dir() -> PathBuf {
    dirs::home_dir()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
}

pub(crate) fn planned_sidecar_routing_path(client_id: &str) -> Result<PathBuf> {
    let spec = planned_sidecar_spec(client_id)
        .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
    let mut path = home_dir();
    for part in spec.config_dir {
        path = path.join(part);
    }
    Ok(path.join(SWITCHBOARD_ROUTING_FILE))
}

pub(crate) fn shell_path(name: &str) -> PathBuf {
    home_dir().join(name)
}

pub(crate) fn all_shell_paths() -> Vec<PathBuf> {
    ALL_SHELL_FILES.into_iter().map(shell_path).collect()
}

pub(crate) fn claude_settings_path() -> PathBuf {
    home_dir().join(".claude").join("settings.json")
}

pub(crate) fn claude_settings_candidates() -> Vec<PathBuf> {
    let claude_dir = home_dir().join(".claude");
    vec![
        claude_dir.join("settings.json"),
        claude_dir.join("settings.local.json"),
    ]
}

pub(crate) fn headroom_rtk_hook_path() -> PathBuf {
    home_dir()
        .join(".claude")
        .join("hooks")
        .join("headroom-rtk-rewrite.sh")
}

pub(crate) fn headroom_markitdown_hook_path() -> PathBuf {
    home_dir()
        .join(".claude")
        .join("hooks")
        .join("headroom-markitdown-read.sh")
}

pub(crate) fn codex_config_toml_path() -> PathBuf {
    home_dir().join(".codex").join("config.toml")
}

pub(crate) fn rtk_codex_agents_path() -> PathBuf {
    home_dir().join(".codex").join("AGENTS.md")
}

pub(crate) fn opencode_config_path() -> PathBuf {
    home_dir()
        .join(".config")
        .join("opencode")
        .join(OPENCODE_CONFIG_FILE)
}

pub(crate) fn windsurf_config_path() -> PathBuf {
    home_dir()
        .join("Library")
        .join("Application Support")
        .join("Windsurf")
        .join("User")
        .join(WINDSURF_CONFIG_FILE)
}

pub(crate) fn zed_config_path() -> PathBuf {
    home_dir().join(".config").join("zed").join(ZED_CONFIG_FILE)
}

pub(crate) enum ShellFamily {
    Zsh,
    Bash,
    Posix,
}

pub(crate) fn detect_shell_family() -> ShellFamily {
    if let Some(shell_name) = std::env::var_os("SHELL")
        .as_deref()
        .and_then(|value| Path::new(value).file_name())
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
    {
        if shell_name.contains("zsh") {
            return ShellFamily::Zsh;
        }
        if shell_name.contains("bash") {
            return ShellFamily::Bash;
        }
        if shell_name == "sh" {
            return ShellFamily::Posix;
        }
    }

    let has_zsh_files = [ZSH_PROFILE_FILE, ZSH_RC_FILE]
        .into_iter()
        .map(shell_path)
        .any(|path| path.exists());
    let has_bash_files = [
        BASH_PROFILE_FILE,
        BASH_LOGIN_FILE,
        POSIX_PROFILE_FILE,
        BASH_RC_FILE,
    ]
    .into_iter()
    .map(shell_path)
    .any(|path| path.exists());

    match (has_zsh_files, has_bash_files) {
        (true, false) => ShellFamily::Zsh,
        (false, true) => ShellFamily::Bash,
        #[cfg(target_os = "macos")]
        _ => ShellFamily::Zsh,
        #[cfg(not(target_os = "macos"))]
        _ => ShellFamily::Bash,
    }
}

pub(crate) fn resolve_default_shell_targets() -> Vec<PathBuf> {
    let mut targets =
        discover_managed_shell_targets(&["managed_rtk", "claude_code"]).unwrap_or_default();
    if targets.is_empty() {
        targets = default_shell_targets_for_family(detect_shell_family());
    }
    dedupe_paths(targets)
}

pub(crate) fn default_shell_targets_for_family(shell_family: ShellFamily) -> Vec<PathBuf> {
    match shell_family {
        ShellFamily::Zsh => {
            dedupe_paths(vec![shell_path(ZSH_PROFILE_FILE), shell_path(ZSH_RC_FILE)])
        }
        ShellFamily::Bash => dedupe_paths(vec![
            preferred_bash_profile_path(),
            shell_path(BASH_RC_FILE),
        ]),
        ShellFamily::Posix => vec![shell_path(POSIX_PROFILE_FILE)],
    }
}

pub(crate) fn preferred_bash_profile_path() -> PathBuf {
    [BASH_PROFILE_FILE, BASH_LOGIN_FILE, POSIX_PROFILE_FILE]
        .into_iter()
        .map(shell_path)
        .find(|path| path.exists())
        .unwrap_or_else(|| shell_path(BASH_PROFILE_FILE))
}

pub(crate) fn discover_managed_shell_targets(block_ids: &[&str]) -> Result<Vec<PathBuf>> {
    let mut discovered = Vec::new();
    for file in all_shell_paths() {
        for block_id in block_ids {
            if file_has_managed_block(&file, block_id)? {
                discovered.push(file.clone());
                break;
            }
        }
    }
    Ok(dedupe_paths(discovered))
}

pub(crate) fn shell_targets_from_state(serialized_paths: Option<&Vec<String>>) -> Vec<PathBuf> {
    serialized_paths
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .collect::<Vec<_>>()
}

pub(crate) fn serialize_paths(paths: &[PathBuf]) -> Vec<String> {
    let mut serialized = paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    dedupe_strings(&mut serialized);
    serialized
}

pub(crate) fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for path in paths {
        let key = path.display().to_string();
        if seen.insert(key) {
            deduped.push(path);
        }
    }
    deduped
}

pub(crate) fn dedupe_strings(values: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

pub(crate) fn is_profile_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(ZSH_PROFILE_FILE | BASH_PROFILE_FILE | BASH_LOGIN_FILE | POSIX_PROFILE_FILE)
    )
}

pub(crate) fn file_has_managed_block(file_path: &Path, block_id: &str) -> Result<bool> {
    if !file_path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;
    let start = format!("# >>> headroom:{block_id} >>>");
    let end = format!("# <<< headroom:{block_id} <<<");
    Ok(content.contains(&start) && content.contains(&end))
}
