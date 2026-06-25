use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Runtime storage intentionally remains under the upstream Headroom directory
/// while Mac AI Switchboard is productized. This preserves existing managed
/// Python runtimes, logs, receipts, backups, and cleanup paths until a dedicated
/// migration can copy/verify state safely.
pub const APP_STORAGE_DIR_NAME: &str = "Headroom";

pub fn app_data_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .unwrap_or_else(std::env::temp_dir);
    base.join(APP_STORAGE_DIR_NAME)
}

pub fn ensure_data_dirs(base_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(base_dir)
        .with_context(|| format!("creating app data dir {}", base_dir.display()))?;
    std::fs::create_dir_all(base_dir.join("telemetry"))
        .with_context(|| format!("creating telemetry dir under {}", base_dir.display()))?;
    std::fs::create_dir_all(base_dir.join("config"))
        .with_context(|| format!("creating config dir under {}", base_dir.display()))?;
    Ok(())
}

pub fn config_file(base_dir: &Path, name: &str) -> PathBuf {
    base_dir.join("config").join(name)
}

pub fn memory_db_path(base_dir: &Path) -> PathBuf {
    base_dir.join("memory.db")
}

pub fn telemetry_file(base_dir: &Path, name: &str) -> PathBuf {
    base_dir.join("telemetry").join(name)
}
