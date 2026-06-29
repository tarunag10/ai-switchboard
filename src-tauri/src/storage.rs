use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

pub const LEGACY_STORAGE_DIR_NAME: &str = "Headroom";
pub const APP_STORAGE_DIR_NAME: &str = "Mac AI Switchboard";
const MIGRATIONS_FILE_NAME: &str = "migrations.json";

pub fn app_data_dir() -> PathBuf {
    let base = app_data_base_dir();
    let app_dir = base.join(APP_STORAGE_DIR_NAME);
    let legacy_dir = base.join(LEGACY_STORAGE_DIR_NAME);

    if let Err(error) = migrate_legacy_storage_if_needed(&legacy_dir, &app_dir) {
        log::warn!("storage migration skipped: {error:#}");
    }

    app_dir
}

fn app_data_base_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .unwrap_or_else(std::env::temp_dir);
    base
}

fn migrate_legacy_storage_if_needed(legacy_dir: &Path, app_dir: &Path) -> Result<()> {
    if app_dir.exists() || !legacy_dir.exists() {
        return Ok(());
    }

    let parent = app_dir.parent().context("app storage path has no parent")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("creating app storage parent {}", parent.display()))?;

    let temp_dir = parent.join(format!(
        ".{}.migration-{}",
        APP_STORAGE_DIR_NAME.replace(' ', "-"),
        Utc::now().timestamp_millis()
    ));

    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)
            .with_context(|| format!("removing stale migration dir {}", temp_dir.display()))?;
    }

    copy_dir_recursive(legacy_dir, &temp_dir).with_context(|| {
        format!(
            "copying legacy storage {} to {}",
            legacy_dir.display(),
            temp_dir.display()
        )
    })?;

    verify_storage_copy(legacy_dir, &temp_dir)?;
    std::fs::rename(&temp_dir, app_dir).with_context(|| {
        format!(
            "moving migrated storage {} to {}",
            temp_dir.display(),
            app_dir.display()
        )
    })?;

    write_migration_record(app_dir, legacy_dir)?;
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)
        .with_context(|| format!("creating {}", destination.display()))?;

    for entry in
        std::fs::read_dir(source).with_context(|| format!("reading {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "copying file {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn verify_storage_copy(legacy_dir: &Path, copied_dir: &Path) -> Result<()> {
    for relative in ["config", "telemetry", "memory.db"] {
        let legacy_path = legacy_dir.join(relative);
        if legacy_path.exists() && !copied_dir.join(relative).exists() {
            anyhow::bail!("migration copy missing {}", relative);
        }
    }
    Ok(())
}

fn write_migration_record(app_dir: &Path, legacy_dir: &Path) -> Result<()> {
    let config_dir = app_dir.join("config");
    std::fs::create_dir_all(&config_dir)
        .with_context(|| format!("creating config dir under {}", app_dir.display()))?;
    let body = serde_json::json!({
        "migrations": [
            {
                "id": "legacy-headroom-storage-copy",
                "migratedAt": Utc::now().to_rfc3339(),
                "from": legacy_dir.display().to_string(),
                "to": app_dir.display().to_string(),
                "mode": "copy-preserve-legacy"
            }
        ]
    });
    std::fs::write(
        config_dir.join(MIGRATIONS_FILE_NAME),
        serde_json::to_vec_pretty(&body)?,
    )
    .with_context(|| format!("writing migration record under {}", config_dir.display()))?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_copies_legacy_storage_and_preserves_legacy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let legacy = temp.path().join(LEGACY_STORAGE_DIR_NAME);
        let app = temp.path().join(APP_STORAGE_DIR_NAME);
        std::fs::create_dir_all(legacy.join("config")).expect("legacy config");
        std::fs::write(legacy.join("config/runtime.json"), "{}").expect("runtime");
        std::fs::write(legacy.join("memory.db"), "legacy").expect("memory");

        migrate_legacy_storage_if_needed(&legacy, &app).expect("migration");

        assert!(legacy.join("config/runtime.json").exists());
        assert!(app.join("config/runtime.json").exists());
        assert!(app.join("memory.db").exists());
        assert!(app.join("config/migrations.json").exists());
    }

    #[test]
    fn migration_skips_when_new_storage_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let legacy = temp.path().join(LEGACY_STORAGE_DIR_NAME);
        let app = temp.path().join(APP_STORAGE_DIR_NAME);
        std::fs::create_dir_all(legacy.join("config")).expect("legacy config");
        std::fs::create_dir_all(app.join("config")).expect("app config");
        std::fs::write(app.join("config/runtime.json"), "new").expect("runtime");

        migrate_legacy_storage_if_needed(&legacy, &app).expect("migration skip");

        assert_eq!(
            std::fs::read_to_string(app.join("config/runtime.json")).expect("runtime"),
            "new"
        );
        assert!(!app.join("config/migrations.json").exists());
    }

    #[test]
    fn migration_failure_leaves_legacy_storage_intact() {
        let temp = tempfile::tempdir().expect("tempdir");
        let legacy = temp.path().join(LEGACY_STORAGE_DIR_NAME);
        let app = temp.path().join("blocked").join(APP_STORAGE_DIR_NAME);
        std::fs::create_dir_all(legacy.join("config")).expect("legacy config");
        std::fs::write(legacy.join("config/runtime.json"), "{}").expect("runtime");
        std::fs::write(temp.path().join("blocked"), "not a directory").expect("blocker");

        assert!(migrate_legacy_storage_if_needed(&legacy, &app).is_err());
        assert!(legacy.join("config/runtime.json").exists());
    }
}
