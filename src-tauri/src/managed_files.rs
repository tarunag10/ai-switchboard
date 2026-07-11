use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

const LEGACY_MARKER_PREFIX: &str = "headroom";
const MARKER_PREFIX: &str = "headroom";
const SWITCHBOARD_MARKER_PREFIX: &str = "mac-ai-switchboard";

pub(crate) fn upsert_managed_block(
    file_path: &Path,
    block_id: &str,
    block_body: &str,
) -> Result<(bool, Option<PathBuf>)> {
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let existing = if file_path.exists() {
        std::fs::read_to_string(file_path)
            .with_context(|| format!("reading {}", file_path.display()))?
    } else {
        String::new()
    };

    let updated = managed_block_updated_content(&existing, block_id, block_body);

    if updated == existing {
        return Ok((false, None));
    }

    let backup = backup_if_exists(file_path)?;
    std::fs::write(file_path, updated)
        .with_context(|| format!("writing {}", file_path.display()))?;
    Ok((true, backup))
}

/// Computes an isolated Switchboard marker edit without touching the filesystem.
/// Config previews use this to make their before/after diff exactly match apply.
pub(crate) fn managed_block_updated_content(
    existing: &str,
    block_id: &str,
    block_body: &str,
) -> String {
    let start = managed_marker_start(MARKER_PREFIX, block_id);
    let end = managed_marker_end(MARKER_PREFIX, block_id);
    let legacy_start = managed_marker_start(LEGACY_MARKER_PREFIX, block_id);
    let legacy_end = managed_marker_end(LEGACY_MARKER_PREFIX, block_id);
    let block = format!("{start}\n{block_body}\n{end}\n");
    if let (Some(start_idx), Some(end_idx)) = (existing.find(&start), existing.find(&end)) {
        replace_marker_block(existing, start_idx, end_idx + end.len(), &block)
    } else if let (Some(start_idx), Some(end_idx)) =
        (existing.find(&legacy_start), existing.find(&legacy_end))
    {
        replace_marker_block(existing, start_idx, end_idx + legacy_end.len(), &block)
    } else if existing.trim().is_empty() {
        block
    } else {
        format!("{}\n{}", existing.trim_end(), block)
    }
}

fn replace_marker_block(existing: &str, start_idx: usize, end_idx: usize, block: &str) -> String {
    let mut rebuilt = String::with_capacity(existing.len() + block.len());
    rebuilt.push_str(&existing[..start_idx]);
    rebuilt.push_str(block);
    if end_idx < existing.len() {
        // `block` already ends in `\n`; avoid accumulating blank padding.
        rebuilt.push_str(
            existing[end_idx..]
                .strip_prefix('\n')
                .unwrap_or(&existing[end_idx..]),
        );
    }
    rebuilt
}

pub(crate) fn write_file_if_changed(
    file_path: &Path,
    content: &str,
    executable: bool,
) -> Result<(bool, Option<PathBuf>)> {
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let existing = if file_path.exists() {
        Some(
            std::fs::read_to_string(file_path)
                .with_context(|| format!("reading {}", file_path.display()))?,
        )
    } else {
        None
    };

    if existing.as_deref() == Some(content) {
        #[cfg(unix)]
        if executable {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = std::fs::metadata(file_path)
                .with_context(|| format!("reading {}", file_path.display()))?
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(file_path, permissions)
                .with_context(|| format!("chmod {}", file_path.display()))?;
        }
        return Ok((false, None));
    }

    let backup = backup_if_exists(file_path)?;
    std::fs::write(file_path, content)
        .with_context(|| format!("writing {}", file_path.display()))?;

    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(file_path)
            .with_context(|| format!("reading {}", file_path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(file_path, permissions)
            .with_context(|| format!("chmod {}", file_path.display()))?;
    }

    Ok((true, backup))
}

pub(crate) fn remove_shell_block(shell_targets: &[PathBuf], block_id: &str) -> Result<()> {
    for file in shell_targets {
        remove_managed_block(file, block_id)?;
    }
    Ok(())
}

pub(crate) fn remove_managed_block(file_path: &Path, block_id: &str) -> Result<bool> {
    remove_managed_block_with_backup(file_path, block_id).map(|(removed, _backup)| removed)
}

pub(crate) fn remove_managed_block_with_backup(
    file_path: &Path,
    block_id: &str,
) -> Result<(bool, Option<PathBuf>)> {
    if !file_path.exists() {
        return Ok((false, None));
    }

    let existing = std::fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;
    let new_start = managed_marker_start(SWITCHBOARD_MARKER_PREFIX, block_id);
    let new_end = managed_marker_end(SWITCHBOARD_MARKER_PREFIX, block_id);
    let legacy_start = managed_marker_start(LEGACY_MARKER_PREFIX, block_id);
    let legacy_end = managed_marker_end(LEGACY_MARKER_PREFIX, block_id);

    let (_start, end, start_idx, end_idx) = if let (Some(start_idx), Some(end_idx)) =
        (existing.find(&new_start), existing.find(&new_end))
    {
        (new_start, new_end, start_idx, end_idx)
    } else if let (Some(start_idx), Some(end_idx)) =
        (existing.find(&legacy_start), existing.find(&legacy_end))
    {
        (legacy_start, legacy_end, start_idx, end_idx)
    } else {
        return Ok((false, None));
    };

    if start_idx >= end_idx {
        return Ok((false, None));
    }

    let end_with_marker = end_idx + end.len();
    let tail = existing[end_with_marker..].trim_start_matches('\n');
    let mut rebuilt = String::with_capacity(existing.len());
    rebuilt.push_str(existing[..start_idx].trim_end());
    if !rebuilt.is_empty() && !tail.is_empty() {
        rebuilt.push('\n');
    }
    rebuilt.push_str(tail);
    if !rebuilt.is_empty() && !rebuilt.ends_with('\n') {
        rebuilt.push('\n');
    }

    let backup = backup_if_exists(file_path)?;
    std::fs::write(file_path, rebuilt)
        .with_context(|| format!("writing {}", file_path.display()))?;
    Ok((true, backup))
}

pub(crate) fn backup_if_exists(path: &Path) -> Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    let stamp = Utc::now().format("%Y%m%d%H%M%S");
    let mut backup_path = PathBuf::from(format!("{}.headroom-backup-{}", path.display(), stamp));
    if backup_path.exists() {
        backup_path = PathBuf::from(format!(
            "{}.headroom-backup-{}-{}",
            path.display(),
            stamp,
            Uuid::new_v4()
        ));
    }
    std::fs::copy(path, &backup_path)
        .with_context(|| format!("creating backup {}", backup_path.display()))?;

    // Prune old backups - keep only the 3 most recent for this base path.
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let headroom_prefix = format!("{}.headroom-backup-", file_name);
    let nommer_prefix = format!("{}.nommer-backup-", file_name);
    if let Some(dir) = path.parent() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut backups: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(&headroom_prefix) || n.starts_with(&nommer_prefix))
                        .unwrap_or(false)
                })
                .collect();
            backups.sort();
            if backups.len() > 3 {
                for old in &backups[..backups.len() - 3] {
                    let _ = std::fs::remove_file(old);
                }
            }
        }
    }

    Ok(Some(backup_path))
}

pub(crate) fn managed_marker_start(prefix: &str, block_id: &str) -> String {
    format!("# >>> {prefix}:{block_id} >>>")
}

pub(crate) fn managed_marker_end(prefix: &str, block_id: &str) -> String {
    format!("# <<< {prefix}:{block_id} <<<")
}

pub(crate) fn strip_marker_block(content: &str, block_id: &str) -> String {
    strip_marker_block_with_prefix(
        &strip_marker_block_with_prefix(content, block_id, SWITCHBOARD_MARKER_PREFIX),
        block_id,
        LEGACY_MARKER_PREFIX,
    )
}

pub(crate) fn strip_marker_block_with_prefix(
    content: &str,
    block_id: &str,
    prefix: &str,
) -> String {
    let start = managed_marker_start(prefix, block_id);
    let end = managed_marker_end(prefix, block_id);
    let (Some(start_idx), Some(end_idx)) = (content.find(&start), content.find(&end)) else {
        return content.to_string();
    };
    let tail = content[end_idx + end.len()..].trim_start_matches('\n');
    let head = content[..start_idx].trim_end();
    let mut rebuilt = String::with_capacity(content.len());
    rebuilt.push_str(head);
    if !rebuilt.is_empty() && !tail.is_empty() {
        rebuilt.push('\n');
    }
    rebuilt.push_str(tail);
    rebuilt
}

pub(crate) fn marker_block_contains(content: &str, block_id: &str, needle: &str) -> bool {
    marker_block_contains_with_prefix(content, block_id, needle, MARKER_PREFIX)
}

pub(crate) fn marker_block_contains_with_prefix(
    content: &str,
    block_id: &str,
    needle: &str,
    prefix: &str,
) -> bool {
    let start = managed_marker_start(prefix, block_id);
    let end = managed_marker_end(prefix, block_id);
    match (content.find(&start), content.find(&end)) {
        (Some(start_idx), Some(end_idx)) if start_idx < end_idx => {
            content[start_idx..end_idx].contains(needle)
        }
        _ => false,
    }
}

pub(crate) fn parse_json_object(raw: &str, path: &Path) -> Result<serde_json::Map<String, Value>> {
    let value: Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(_) => json5::from_str(raw).with_context(|| {
            format!(
                "parsing {} failed (JSON/JSON5); refusing to overwrite potentially valid user settings",
                path.display()
            )
        })?,
    };
    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("{} must contain a top-level JSON object", path.display()))
}
