use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::client_connectors::planned_sidecar_spec;

pub(crate) const SWITCHBOARD_ROUTING_FILE: &str = "mac-ai-switchboard-routing.md";
pub(crate) const OPENCODE_CONFIG_FILE: &str = "opencode.json";
pub(crate) const WINDSURF_CONFIG_FILE: &str = "settings.json";
pub(crate) const ZED_CONFIG_FILE: &str = "settings.json";

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
