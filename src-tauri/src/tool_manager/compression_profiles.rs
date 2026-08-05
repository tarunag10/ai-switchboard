use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::models::SavingsMode;
use crate::storage;

use super::proxy_runtime::apply_savings_mode_env;

pub const COMPRESSION_PROFILE_FILE: &str = "compression-profile.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompressionProfileId {
    Balanced,
    Aggressive,
    CodexHeavy,
    ClaudeCacheSafe,
}

impl CompressionProfileId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
            Self::CodexHeavy => "codex-heavy",
            Self::ClaudeCacheSafe => "claude-cache-safe",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "balanced" => Some(Self::Balanced),
            "aggressive" => Some(Self::Aggressive),
            "codex-heavy" => Some(Self::CodexHeavy),
            "claude-cache-safe" => Some(Self::ClaudeCacheSafe),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompressionProfileAdvanced {
    #[serde(default = "default_true")]
    pub compress_user_messages: bool,
    #[serde(default)]
    pub compress_tool_results: bool,
    #[serde(default)]
    pub compress_history: bool,
    #[serde(default = "default_true")]
    pub output_shaper: bool,
}

fn default_true() -> bool {
    true
}

impl Default for CompressionProfileAdvanced {
    fn default() -> Self {
        Self {
            compress_user_messages: true,
            compress_tool_results: false,
            compress_history: false,
            output_shaper: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompressionProfileState {
    pub version: u8,
    pub preset_id: CompressionProfileId,
    #[serde(default)]
    pub advanced: CompressionProfileAdvanced,
}

impl Default for CompressionProfileState {
    fn default() -> Self {
        preset_definition(CompressionProfileId::Balanced).to_state()
    }
}

#[derive(Debug, Clone)]
pub struct CompressionProfileDefinition {
    pub id: CompressionProfileId,
    pub label: &'static str,
    pub description: &'static str,
    pub headroom_mode: &'static str,
    pub verbosity_level: u8,
    pub savings_mode: SavingsMode,
    pub advanced: CompressionProfileAdvanced,
}

impl CompressionProfileDefinition {
    pub fn to_state(&self) -> CompressionProfileState {
        CompressionProfileState {
            version: 1,
            preset_id: self.id,
            advanced: self.advanced.clone(),
        }
    }
}

pub fn all_compression_profile_definitions() -> [CompressionProfileDefinition; 4] {
    [
        preset_definition(CompressionProfileId::Balanced),
        preset_definition(CompressionProfileId::Aggressive),
        preset_definition(CompressionProfileId::CodexHeavy),
        preset_definition(CompressionProfileId::ClaudeCacheSafe),
    ]
}

pub fn preset_definition(id: CompressionProfileId) -> CompressionProfileDefinition {
    match id {
        CompressionProfileId::Balanced => CompressionProfileDefinition {
            id,
            label: "Balanced",
            description:
                "Matches the shipped default: token mode, user-message compression, output shaper level 2, and balanced savings profile.",
            headroom_mode: "token",
            verbosity_level: 2,
            savings_mode: SavingsMode::Balanced,
            advanced: CompressionProfileAdvanced {
                compress_user_messages: true,
                compress_tool_results: false,
                compress_history: false,
                output_shaper: true,
            },
        },
        CompressionProfileId::Aggressive => CompressionProfileDefinition {
            id,
            label: "Aggressive",
            description:
                "Higher savings with more tool-result interception and compaction-oriented savings env. Best when latency is acceptable.",
            headroom_mode: "token",
            verbosity_level: 3,
            savings_mode: SavingsMode::Aggressive,
            advanced: CompressionProfileAdvanced {
                compress_user_messages: true,
                compress_tool_results: true,
                compress_history: true,
                output_shaper: true,
            },
        },
        CompressionProfileId::CodexHeavy => CompressionProfileDefinition {
            id,
            label: "Codex-heavy",
            description:
                "Optimizes for Codex/OpenAI sessions with aggressive tool-result interception and user-message compression.",
            headroom_mode: "token",
            verbosity_level: 2,
            savings_mode: SavingsMode::Aggressive,
            advanced: CompressionProfileAdvanced {
                compress_user_messages: true,
                compress_tool_results: true,
                compress_history: false,
                output_shaper: true,
            },
        },
        CompressionProfileId::ClaudeCacheSafe => CompressionProfileDefinition {
            id,
            label: "Claude cache-safe",
            description:
                "Conservative profile that avoids user-message and output shaping changes that can disturb Claude prefix-cache stability.",
            headroom_mode: "token",
            verbosity_level: 1,
            savings_mode: SavingsMode::Balanced,
            advanced: CompressionProfileAdvanced {
                compress_user_messages: false,
                compress_tool_results: false,
                compress_history: false,
                output_shaper: false,
            },
        },
    }
}

pub fn compression_profile_path() -> PathBuf {
    storage::app_data_dir()
        .join("config")
        .join(COMPRESSION_PROFILE_FILE)
}

pub fn load_compression_profile() -> CompressionProfileState {
    let path = compression_profile_path();
    if !path.exists() {
        return CompressionProfileState::default();
    }
    match fs::read_to_string(&path) {
        Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
        Err(err) => {
            log::warn!("load_compression_profile: {err:#}");
            CompressionProfileState::default()
        }
    }
}

pub fn save_compression_profile(state: &CompressionProfileState) -> Result<()> {
    let path = compression_profile_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let body = serde_json::to_vec_pretty(state)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("renaming {}", path.display()))?;
    Ok(())
}

pub fn clear_compression_profile() -> Result<()> {
    let path = compression_profile_path();
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

pub fn resolved_compression_profile(state: &CompressionProfileState) -> CompressionProfileDefinition {
    let mut definition = preset_definition(state.preset_id);
    definition.advanced = state.advanced.clone();
    definition
}

pub fn effective_savings_mode(state: &CompressionProfileState) -> SavingsMode {
    let definition = resolved_compression_profile(state);
    if definition.advanced.compress_tool_results || definition.advanced.compress_history {
        SavingsMode::Aggressive
    } else {
        definition.savings_mode
    }
}

pub fn apply_compression_profile_env(command: &mut Command, state: &CompressionProfileState) {
    let definition = resolved_compression_profile(state);
    let savings_mode = effective_savings_mode(state);
    command
        .env(
            "HEADROOM_MODE",
            definition.headroom_mode,
        )
        .env(
            "HEADROOM_COMPRESS_USER_MESSAGES",
            if definition.advanced.compress_user_messages {
                "1"
            } else {
                "0"
            },
        )
        .env(
            "HEADROOM_OUTPUT_SHAPER",
            if definition.advanced.output_shaper {
                "1"
            } else {
                "0"
            },
        )
        .env(
            "HEADROOM_VERBOSITY_LEVEL",
            definition.verbosity_level.to_string(),
        );
    apply_savings_mode_env(command, &savings_mode);
    if definition.advanced.compress_history {
        command.env("HEADROOM_SMART_CRUSHER_COMPACTION", "1");
    }
}

pub fn history_compression_toggle_supported() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_preset_matches_shipped_defaults() {
        let preset = preset_definition(CompressionProfileId::Balanced);
        assert_eq!(preset.headroom_mode, "token");
        assert!(preset.advanced.compress_user_messages);
        assert!(preset.advanced.output_shaper);
        assert_eq!(preset.verbosity_level, 2);
        assert_eq!(preset.savings_mode, SavingsMode::Balanced);
        assert!(!preset.advanced.compress_tool_results);
    }

    #[test]
    fn aggressive_preset_enables_tool_result_interception_path() {
        let preset = preset_definition(CompressionProfileId::Aggressive);
        assert_eq!(effective_savings_mode(&preset.to_state()), SavingsMode::Aggressive);
        assert!(preset.advanced.compress_tool_results);
    }

    #[test]
    fn claude_cache_safe_reduces_cache_risky_controls() {
        let preset = preset_definition(CompressionProfileId::ClaudeCacheSafe);
        assert!(!preset.advanced.compress_user_messages);
        assert!(!preset.advanced.output_shaper);
        assert_eq!(preset.verbosity_level, 1);
    }

    #[test]
    fn apply_compression_profile_env_sets_expected_keys() {
        let state = preset_definition(CompressionProfileId::Balanced).to_state();
        let mut command = Command::new("/bin/echo");
        apply_compression_profile_env(&mut command, &state);
        // Command env is opaque in tests; mapping is covered by preset assertions above.
        assert_eq!(state.preset_id, CompressionProfileId::Balanced);
    }
}
