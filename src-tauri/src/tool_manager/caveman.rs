use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::{
    ToolManager, CAVEMAN_LEVEL_AGGRESSIVE, CAVEMAN_LEVEL_COMPACT_CHINESE, CAVEMAN_LEVEL_SCOPED,
};

pub(super) const CAVEMAN_DISPLAY_VERSION: &str = "1";

impl ToolManager {
    pub fn caveman_receipt_exists(&self) -> bool {
        self.runtime.tools_dir.join("caveman.json").exists()
    }

    /// Persisted guidance level for the managed nudge body. Defaults to scoped.
    pub fn caveman_level(&self) -> String {
        self.read_tool_receipt("caveman")
            .and_then(|receipt| {
                receipt
                    .get("level")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| CAVEMAN_LEVEL_SCOPED.to_string())
    }

    /// Caveman has no external runtime: "install" just records the receipt.
    /// The managed guidance blocks are written by `client_adapters` from lib.rs.
    pub fn install_caveman(&self) -> Result<()> {
        self.write_tool_receipt(
            "caveman",
            json!({
                "version": CAVEMAN_DISPLAY_VERSION,
                "enabled": true,
                "level": CAVEMAN_LEVEL_SCOPED,
            }),
        )?;
        Ok(())
    }

    pub fn set_caveman_enabled(&self, enabled: bool) -> Result<()> {
        if !self.caveman_receipt_exists() {
            bail!("caveman is not installed");
        }
        let level = self.caveman_level();
        self.write_tool_receipt(
            "caveman",
            json!({
                "version": CAVEMAN_DISPLAY_VERSION,
                "enabled": enabled,
                "level": level,
            }),
        )?;
        Ok(())
    }

    pub fn set_caveman_level(&self, level: &str) -> Result<()> {
        if !self.caveman_receipt_exists() {
            bail!("caveman is not installed");
        }
        let normalized = match level {
            CAVEMAN_LEVEL_AGGRESSIVE => CAVEMAN_LEVEL_AGGRESSIVE,
            CAVEMAN_LEVEL_COMPACT_CHINESE => CAVEMAN_LEVEL_COMPACT_CHINESE,
            _ => CAVEMAN_LEVEL_SCOPED,
        };
        let enabled = self.tool_enabled("caveman");
        self.write_tool_receipt(
            "caveman",
            json!({
                "version": CAVEMAN_DISPLAY_VERSION,
                "enabled": enabled,
                "level": normalized,
            }),
        )?;
        Ok(())
    }

    pub fn uninstall_caveman(&self) -> Result<()> {
        let receipt = self.runtime.tools_dir.join("caveman.json");
        if receipt.exists() {
            std::fs::remove_file(&receipt)
                .with_context(|| format!("removing {}", receipt.display()))?;
        }
        Ok(())
    }
}
