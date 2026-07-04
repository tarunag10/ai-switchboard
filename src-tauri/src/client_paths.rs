use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::client_connectors::planned_sidecar_spec;

pub(crate) const SWITCHBOARD_ROUTING_FILE: &str = "mac-ai-switchboard-routing.md";

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
