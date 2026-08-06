//! Canonical Switchboard product identity for on-disk managed artifacts.
//!
//! User-facing branding is "AI Switchboard" with slug `ai-switchboard`. Older
//! installs may still carry `mac-ai-switchboard` filenames/markers or legacy
//! `headroom:` managed blocks; readers must recognize all variants.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::client_connectors::planned_sidecar_spec;
use crate::managed_files::{managed_marker_end, managed_marker_start, remove_managed_block};

/// Product slug variants used in managed marker ids and dry-run previews.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchboardIdentitySlug {
    AiSwitchboard,
    LegacyMacAiSwitchboard,
    LegacyHeadroom,
}

impl SwitchboardIdentitySlug {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AiSwitchboard => "ai-switchboard",
            Self::LegacyMacAiSwitchboard => "mac-ai-switchboard",
            Self::LegacyHeadroom => "headroom",
        }
    }

    pub fn marker_prefixes() -> &'static [Self] {
        &[
            Self::AiSwitchboard,
            Self::LegacyMacAiSwitchboard,
            Self::LegacyHeadroom,
        ]
    }
}

pub const ROUTING_FILE: &str = "ai-switchboard-routing.md";
pub const LEGACY_ROUTING_FILE: &str = "mac-ai-switchboard-routing.md";

pub const DRY_RUN_BACKUP_SUFFIX: &str = ".ai-switchboard.bak";
pub const LEGACY_DRY_RUN_BACKUP_SUFFIX: &str = ".mac-ai-switchboard.bak";

pub const AGENT_MEMORY_START: &str = "<!-- ai-switchboard:agent-memory:start -->";
pub const AGENT_MEMORY_END: &str = "<!-- ai-switchboard:agent-memory:end -->";
pub const LEGACY_AGENT_MEMORY_START: &str = "<!-- mac-ai-switchboard:agent-memory:start -->";
pub const LEGACY_AGENT_MEMORY_END: &str = "<!-- mac-ai-switchboard:agent-memory:end -->";

pub fn primary_marker_prefix() -> &'static str {
    SwitchboardIdentitySlug::AiSwitchboard.as_str()
}

pub fn managed_marker_id(block_id: &str) -> String {
    format!("{}:{block_id}", primary_marker_prefix())
}

pub fn marker_id_variants(suffix: &str) -> Vec<String> {
    SwitchboardIdentitySlug::marker_prefixes()
        .iter()
        .map(|slug| format!("{}:{suffix}", slug.as_str()))
        .collect()
}

pub fn json_comment_marker_start(marker_id: &str) -> String {
    format!("// >>> {marker_id} >>>")
}

pub fn json_comment_marker_end(marker_id: &str) -> String {
    format!("// <<< {marker_id} <<<")
}

pub fn dry_run_marker(block_id: &str) -> String {
    managed_marker_id(block_id)
}

pub fn dry_run_backup_path(target: &str) -> String {
    format!("{target}{DRY_RUN_BACKUP_SUFFIX}")
}

pub fn routing_file_name(legacy: bool) -> &'static str {
    if legacy {
        LEGACY_ROUTING_FILE
    } else {
        ROUTING_FILE
    }
}

pub fn planned_sidecar_routing_path(client_id: &str) -> Result<PathBuf> {
    Ok(sidecar_root(client_id)?.join(ROUTING_FILE))
}

pub fn legacy_planned_sidecar_routing_path(client_id: &str) -> Result<PathBuf> {
    Ok(sidecar_root(client_id)?.join(LEGACY_ROUTING_FILE))
}

pub fn resolve_planned_sidecar_routing_path(client_id: &str) -> Result<PathBuf> {
    let canonical = planned_sidecar_routing_path(client_id)?;
    if canonical.exists() {
        return Ok(canonical);
    }
    let legacy = legacy_planned_sidecar_routing_path(client_id)?;
    if legacy.exists() {
        return Ok(legacy);
    }
    Ok(canonical)
}

pub fn sidecar_marker_present(content: &str, block_id: &str) -> bool {
    SwitchboardIdentitySlug::marker_prefixes()
        .iter()
        .any(|slug| sidecar_marker_present_with_prefix(content, block_id, slug.as_str()))
}

fn sidecar_marker_present_with_prefix(content: &str, block_id: &str, prefix: &str) -> bool {
    content.contains(&managed_marker_start(prefix, block_id))
        && content.contains(&managed_marker_end(prefix, block_id))
}

pub fn retire_legacy_planned_sidecar(client_id: &str) -> Result<()> {
    let canonical = planned_sidecar_routing_path(client_id)?;
    let legacy = legacy_planned_sidecar_routing_path(client_id)?;
    if legacy == canonical || !legacy.exists() {
        return Ok(());
    }

    if !canonical.exists() {
        std::fs::rename(&legacy, &canonical).with_context(|| {
            format!(
                "migrating legacy sidecar {} to {}",
                legacy.display(),
                canonical.display()
            )
        })?;
        return Ok(());
    }

    let _ = remove_managed_block(&legacy, client_id)?;
    if legacy.exists() {
        let remaining = std::fs::read_to_string(&legacy)
            .with_context(|| format!("reading {}", legacy.display()))?;
        if remaining.trim().is_empty() {
            std::fs::remove_file(&legacy)
                .with_context(|| format!("removing {}", legacy.display()))?;
        }
    }
    Ok(())
}

fn sidecar_root(client_id: &str) -> Result<PathBuf> {
    let spec = planned_sidecar_spec(client_id)
        .ok_or_else(|| anyhow::anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
    let mut path = crate::client_paths::home_dir();
    for part in spec.config_dir {
        path = path.join(part);
    }
    Ok(path)
}

pub fn routing_path_display_suffix() -> &'static str {
    ROUTING_FILE
}

pub fn legacy_routing_path_display_suffix() -> &'static str {
    LEGACY_ROUTING_FILE
}

pub fn footprint_marker_recognition_note() -> &'static str {
    "ai-switchboard:, mac-ai-switchboard:, and headroom: marker blocks are recognized."
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn primary_slug_and_routing_file_use_ai_switchboard() {
        assert_eq!(primary_marker_prefix(), "ai-switchboard");
        assert_eq!(ROUTING_FILE, "ai-switchboard-routing.md");
        assert_eq!(managed_marker_id("continue"), "ai-switchboard:continue");
    }

    #[test]
    fn recognizes_all_marker_prefix_variants() {
        for prefix in ["ai-switchboard", "mac-ai-switchboard", "headroom"] {
            let content = format!(
                "# >>> {prefix}:continue >>>\nproxy\n# <<< {prefix}:continue <<<\n"
            );
            assert!(sidecar_marker_present(&content, "continue"));
        }
    }

    #[test]
    fn migrates_legacy_sidecar_filename_on_retire() {
        let home = std::env::temp_dir().join(format!(
            "switchboard-identity-migrate-{}",
            std::process::id()
        ));
        let continue_dir = home.join(".continue");
        fs::create_dir_all(&continue_dir).expect("continue dir");
        let legacy = continue_dir.join(LEGACY_ROUTING_FILE);
        fs::write(&legacy, "# user note\n").expect("legacy sidecar");

        let prev_home = std::env::var_os("HOME");
        let prev_continue = std::env::var_os("CONTINUE_PATH_ROOT");
        std::env::set_var("HOME", &home);
        std::env::set_var("CONTINUE_PATH_ROOT", &continue_dir);

        retire_legacy_planned_sidecar("continue").expect("migrate legacy sidecar");
        let canonical = continue_dir.join(ROUTING_FILE);
        assert!(canonical.exists());
        assert!(!legacy.exists());

        match prev_continue {
            Some(value) => std::env::set_var("CONTINUE_PATH_ROOT", value),
            None => std::env::remove_var("CONTINUE_PATH_ROOT"),
        }
        match prev_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = fs::remove_dir_all(home);
    }
}
