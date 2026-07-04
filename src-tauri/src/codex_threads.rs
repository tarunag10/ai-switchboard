use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;

use crate::client_adapters::codex_home;
use crate::models::{
    CodexDbRestoreResult, CodexThreadRetaggingMode, CodexThreadRetaggingReport,
    CodexThreadRetaggingRunReport, CodexThreadRetaggingSettings,
};
use crate::storage::{app_data_dir, config_file};

const CODEX_HEADROOM_PROVIDER: &str = "headroom";
const CODEX_NATIVE_PROVIDER: &str = "openai";
const CODEX_RETAGGING_SETTINGS_FILE: &str = "codex-retagging.json";

/// Codex store-schema versions this build has been verified against. Discovered
/// stores with a version outside this set are skipped until verified, so a Codex
/// store bump is visible before Switchboard writes an unknown private DB.
const KNOWN_CODEX_STORE_VERSIONS: &[u32] = &[5];

/// Directories Codex is known to keep its state store in: the v148 GUI uses
/// `<codex_home>/sqlite/`, the CLI/TUI uses `<codex_home>/`.
pub(crate) fn codex_state_dirs() -> Vec<PathBuf> {
    let codex = codex_home();
    vec![codex.join("sqlite"), codex]
}

/// True when Codex keeps (or kept) a sqlite-backed thread store on this machine,
/// so a *missing* recognized `state_<N>.sqlite` means the store moved/renamed --
/// the case worth a signal. Two store shapes exist: the GUI uses the
/// `<codex_home>/sqlite/` dir, the CLI/TUI drops `state_<N>.sqlite` loose in
/// `<codex_home>/`. Evidence of either: the `sqlite/` dir, or any
/// `state_*.sqlite`-shaped file (incl. a renamed one whose version no longer
/// parses, which is exactly the relocation we want to catch). CLI-only or
/// pre-sqlite installs with just `config.toml`/`sessions/` match neither and
/// stay silent -- they have no thread store to split.
pub(crate) fn codex_sqlite_store_expected() -> bool {
    if codex_home().join("sqlite").is_dir() {
        return true;
    }
    codex_state_dirs().iter().any(|dir| {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries.flatten().any(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|n| n.starts_with("state_") && n.ends_with(".sqlite"))
                })
            })
            .unwrap_or(false)
    })
}

/// Parse `N` from a `state_<N>.sqlite` filename (`state_5.sqlite` -> `Some(5)`).
/// Anything else -> `None`.
pub(crate) fn codex_store_version(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    name.strip_prefix("state_")?
        .strip_suffix(".sqlite")?
        .parse()
        .ok()
}

/// Discover every `state_<N>.sqlite` store under the known Codex dirs, with the
/// version parsed from its name. Scanning the directories (rather than probing a
/// hardcoded `state_5.sqlite`) means a future Codex store-version bump keeps
/// working without a release instead of silently no-opping for every user at
/// once. A missing dir (`read_dir` error) is skipped. Paths are deduped in case
/// the two dirs ever resolve to the same place.
pub(crate) fn discover_codex_state_dbs() -> Vec<(PathBuf, u32)> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for dir in codex_state_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(version) = codex_store_version(&path) else {
                continue;
            };
            if seen.insert(path.clone()) {
                out.push((path, version));
            }
        }
    }
    out
}

impl Default for CodexThreadRetaggingSettings {
    fn default() -> Self {
        Self {
            codex_thread_retagging: CodexThreadRetaggingMode::Ask,
        }
    }
}

pub(crate) fn codex_retagging_settings_path() -> PathBuf {
    config_file(&app_data_dir(), CODEX_RETAGGING_SETTINGS_FILE)
}

pub fn get_codex_thread_retagging_settings() -> CodexThreadRetaggingSettings {
    let path = codex_retagging_settings_path();
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return CodexThreadRetaggingSettings::default();
    };
    serde_json::from_str::<CodexThreadRetaggingSettings>(&raw).unwrap_or_else(|err| {
        log::warn!(
            "codex retag: reading {} failed, falling back to ask mode: {err}",
            path.display()
        );
        CodexThreadRetaggingSettings::default()
    })
}

pub fn set_codex_thread_retagging_settings(
    settings: CodexThreadRetaggingSettings,
) -> Result<CodexThreadRetaggingSettings> {
    let path = codex_retagging_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&settings).context("serializing Codex retagging settings")?,
    )
    .with_context(|| format!("writing {}", path.display()))?;
    Ok(settings)
}

fn codex_retagging_backup_path(path: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state.sqlite");
    path.with_file_name(format!("{name}.switchboard-backup-{timestamp}"))
}

fn backup_codex_db(path: &Path) -> Result<PathBuf> {
    let backup = codex_retagging_backup_path(path);
    std::fs::copy(path, &backup)
        .with_context(|| format!("creating Codex thread DB backup {}", backup.display()))?;
    Ok(backup)
}

fn codex_skip_report(
    path: &Path,
    from: &str,
    to: &str,
    reason: impl Into<String>,
) -> CodexThreadRetaggingReport {
    CodexThreadRetaggingReport {
        path: path.display().to_string(),
        from_provider: from.to_string(),
        to_provider: to.to_string(),
        rows_changed: 0,
        backup_path: None,
        skipped_reason: Some(reason.into()),
    }
}

/// Best-effort retag of Codex thread provider tags so the history menu stays
/// whole across the Headroom proxy boundary. Never fails the caller: a missing
/// store, a missing `threads` table, or a DB locked by a running Codex is logged
/// and skipped. Only rows whose `model_provider` equals `from` are touched, so
/// third-party providers are left alone.
pub(crate) fn retag_codex_thread_providers(from: &str, to: &str) -> CodexThreadRetaggingRunReport {
    let settings = get_codex_thread_retagging_settings();
    let mode = settings.codex_thread_retagging.clone();
    if mode != CodexThreadRetaggingMode::Enabled {
        log::info!("codex retag {from}->{to}: skipped because mode is {mode:?}");
        return CodexThreadRetaggingRunReport {
            mode,
            reports: Vec::new(),
        };
    }

    let stores = discover_codex_state_dbs();
    if stores.is_empty() {
        // Only a signal when a sqlite thread store is actually expected: the
        // launch/quit lifecycle hooks call this for every user, so a clean
        // machine -- or a CLI-only / pre-sqlite Codex with config but no
        // state_<N>.sqlite -- must stay silent. A present sqlite/ dir with no
        // recognized store is the genuine moved/renamed case worth flagging.
        if codex_sqlite_store_expected() {
            log::warn!(
                "codex retag {from}->{to}: Codex is present but no state_<N>.sqlite \
                 store was found under {dirs:?}; the history menu may split. Codex \
                 may have moved or renamed its store.",
                dirs = codex_state_dirs(),
            );
        }
        return CodexThreadRetaggingRunReport {
            mode,
            reports: Vec::new(),
        };
    }
    let mut reports = Vec::new();
    for (path, version) in stores {
        if !KNOWN_CODEX_STORE_VERSIONS.contains(&version) {
            log::warn!(
                "codex retag: store version {version} at {} is outside the known \
                 set {KNOWN_CODEX_STORE_VERSIONS:?}; skipping until the user \
                 explicitly restores compatibility or this version is verified.",
                path.display(),
            );
            reports.push(codex_skip_report(
                &path,
                from,
                to,
                format!("unknown Codex store version {version}"),
            ));
            continue;
        }
        let backup = match backup_codex_db(&path) {
            Ok(path) => path,
            Err(e) => {
                log::warn!(
                    "codex retag {from}->{to} skipped for {}: backup failed: {e}",
                    path.display()
                );
                reports.push(codex_skip_report(
                    &path,
                    from,
                    to,
                    format!("backup failed: {e}"),
                ));
                continue;
            }
        };
        match retag_one_codex_db(&path, from, to) {
            Ok(n) => {
                if n > 0 {
                    log::info!(
                        "codex retag {from}->{to}: {n} thread(s) in {}",
                        path.display()
                    );
                }
                reports.push(CodexThreadRetaggingReport {
                    path: path.display().to_string(),
                    from_provider: from.to_string(),
                    to_provider: to.to_string(),
                    rows_changed: n,
                    backup_path: Some(backup.display().to_string()),
                    skipped_reason: None,
                });
            }
            Err(e) => {
                log::warn!(
                    "codex retag {from}->{to} skipped for {}: {e}",
                    path.display()
                );
                reports.push(codex_skip_report(&path, from, to, e.to_string()));
            }
        }
    }
    CodexThreadRetaggingRunReport { mode, reports }
}

pub(crate) fn retag_one_codex_db(path: &Path, from: &str, to: &str) -> rusqlite::Result<usize> {
    use rusqlite::OptionalExtension;

    let mut conn = rusqlite::Connection::open(path)?;
    conn.busy_timeout(Duration::from_millis(750))?;
    // No-op (without erroring) on builds whose store lacks the threads table.
    let has_table = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'threads'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !has_table {
        return Ok(0);
    }
    let has_column = conn
        .query_row("PRAGMA table_info(threads)", [], |_| Ok(()))
        .optional()?
        .is_some()
        && {
            let mut stmt = conn.prepare("PRAGMA table_info(threads)")?;
            let mut rows = stmt.query([])?;
            let mut found = false;
            while let Some(row) = rows.next()? {
                let name: String = row.get(1)?;
                if name == "model_provider" {
                    found = true;
                    break;
                }
            }
            found
        };
    if !has_column {
        return Ok(0);
    }
    let tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE threads SET model_provider = ?2 WHERE model_provider = ?1",
        rusqlite::params![from, to],
    )?;
    tx.commit()?;
    Ok(changed)
}

pub fn restore_codex_thread_db_backup(path: &str) -> Result<CodexDbRestoreResult> {
    let backup = PathBuf::from(path);
    if !backup.exists() {
        return Err(anyhow!(
            "Codex thread DB backup does not exist: {}",
            backup.display()
        ));
    }
    let file_name = backup
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Invalid Codex backup path: {}", backup.display()))?;
    let Some(original_name) = file_name.split(".switchboard-backup-").next() else {
        return Err(anyhow!("Invalid Switchboard backup name: {file_name}"));
    };
    if original_name == file_name {
        return Err(anyhow!("Not a Switchboard Codex DB backup: {file_name}"));
    }
    let original = backup.with_file_name(original_name);
    std::fs::copy(&backup, &original).with_context(|| {
        format!(
            "restoring Codex thread DB from {} to {}",
            backup.display(),
            original.display()
        )
    })?;
    Ok(CodexDbRestoreResult {
        restored_path: original.display().to_string(),
        backup_path: backup.display().to_string(),
    })
}

/// Retag Codex threads back to the native provider. Exposed for the app-quit
/// hook in `lib.rs`, which covers exit paths (Cmd-Q, dock quit, signals) that
/// bypass `clear_client_setups` and therefore the disconnect retag.
pub fn retag_codex_threads_to_native() {
    retag_codex_thread_providers(CODEX_HEADROOM_PROVIDER, CODEX_NATIVE_PROVIDER);
}

/// Pull Codex threads into the headroom provider menu. Exposed for the
/// app-launch hook in `lib.rs`, which must undo the quit-time native retag on
/// the exit paths (Cmd-Q, dock quit, app-update restart) that never populate
/// `remembered_clients` and are therefore skipped by `restore_client_setups`.
pub fn retag_codex_threads_to_headroom() {
    retag_codex_thread_providers(CODEX_NATIVE_PROVIDER, CODEX_HEADROOM_PROVIDER);
}
