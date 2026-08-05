use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::storage;

pub const HEADROOM_ADVANCED_SETTINGS_FILE: &str = "headroom-advanced-settings.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HeadroomAdvancedSettings {
    pub version: u8,
    #[serde(default)]
    pub cc_switch_reconcile: bool,
}

impl Default for HeadroomAdvancedSettings {
    fn default() -> Self {
        Self {
            version: 1,
            cc_switch_reconcile: false,
        }
    }
}

fn settings_path() -> PathBuf {
    storage::app_data_dir().join(HEADROOM_ADVANCED_SETTINGS_FILE)
}

pub fn load_headroom_advanced_settings() -> HeadroomAdvancedSettings {
    let path = settings_path();
    if !path.exists() {
        return HeadroomAdvancedSettings::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_headroom_advanced_settings(
    settings: &HeadroomAdvancedSettings,
) -> Result<HeadroomAdvancedSettings> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(tmp, &path)?;
    Ok(settings.clone())
}

pub fn apply_headroom_advanced_env(command: &mut Command, settings: &HeadroomAdvancedSettings) {
    if settings.cc_switch_reconcile {
        command.env("HEADROOM_CC_SWITCH_RECONCILE", "1");
    } else {
        command.env_remove("HEADROOM_CC_SWITCH_RECONCILE");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cc_switch_reconcile_to_off() {
        let settings = HeadroomAdvancedSettings::default();
        assert!(!settings.cc_switch_reconcile);
    }
}
