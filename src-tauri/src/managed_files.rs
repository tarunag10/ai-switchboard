use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;

use serde_json::Value;
use uuid::Uuid;

use crate::switchboard_identity::{primary_marker_prefix, SwitchboardIdentitySlug};

/// Writes through a same-directory temporary file so an interrupted or
/// disk-full write cannot truncate the current managed file. Existing
/// permissions are copied to the replacement before the atomic rename.
pub(crate) fn atomic_write_bytes(file_path: &Path, content: &[u8]) -> Result<()> {
    atomic_write_bytes_with_commit(file_path, content, |temporary, destination| {
        std::fs::rename(temporary, destination).with_context(|| {
            format!(
                "atomically replacing {} from {}",
                destination.display(),
                temporary.display()
            )
        })
    })
}

/// Publishes a fully synced file only if the destination is still absent.
/// The hard-link operation is the no-clobber commit point: a file created by
/// another writer wins and is never replaced.
pub(crate) fn atomic_write_bytes_if_absent(file_path: &Path, content: &[u8]) -> Result<()> {
    atomic_write_bytes_with_commit(file_path, content, |temporary, destination| {
        std::fs::hard_link(temporary, destination).with_context(|| {
            format!(
                "publishing {} only while it remains absent",
                destination.display()
            )
        })?;
        std::fs::remove_file(temporary)
            .with_context(|| format!("removing temporary link {}", temporary.display()))
    })
}

/// Atomically replaces a file only when its complete current bytes still
/// match the caller's snapshot. The final comparison happens after the
/// replacement has been fully written and synced, immediately before rename.
pub(crate) fn atomic_write_bytes_if_unchanged(
    file_path: &Path,
    expected: &[u8],
    replacement: &[u8],
) -> Result<()> {
    atomic_write_bytes_with_commit(file_path, replacement, |temporary, destination| {
        let current = std::fs::read(destination).with_context(|| {
            format!("revalidating {} before replacement", destination.display())
        })?;
        if current != expected {
            anyhow::bail!(
                "{} changed before its managed update; current content was preserved",
                destination.display()
            );
        }
        std::fs::rename(temporary, destination).with_context(|| {
            format!(
                "atomically replacing {} from {}",
                destination.display(),
                temporary.display()
            )
        })
    })
}

/// Removes a managed file only while its complete bytes still match the
/// caller's snapshot. Comparison and unlink share the directory write lock.
pub(crate) fn atomic_remove_file_if_unchanged(file_path: &Path, expected: &[u8]) -> Result<()> {
    let parent = file_path
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent directory", file_path.display()))?;
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let _directory_lock = DirectoryWriteLock::acquire(parent)?;
    match std::fs::symlink_metadata(file_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            anyhow::bail!(
                "refusing to remove symlinked managed file {}; its target was preserved",
                file_path.display()
            );
        }
        Ok(_) => {}
        Err(error) => {
            return Err(error).with_context(|| format!("inspecting {}", file_path.display()));
        }
    }
    let current = std::fs::read(file_path)
        .with_context(|| format!("revalidating {} before removal", file_path.display()))?;
    if current != expected {
        anyhow::bail!(
            "{} changed before its managed removal; current content was preserved",
            file_path.display()
        );
    }
    std::fs::remove_file(file_path)
        .with_context(|| format!("atomically removing {}", file_path.display()))?;
    #[cfg(unix)]
    if let Ok(directory) = std::fs::File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(())
}

fn atomic_write_bytes_with_commit<F>(file_path: &Path, content: &[u8], commit: F) -> Result<()>
where
    F: FnOnce(&Path, &Path) -> Result<()>,
{
    let parent = file_path
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent directory", file_path.display()))?;
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    // Every Switchboard managed-file writer takes the same directory lock, so
    // its compare and commit form one serialized operation. The no-clobber
    // helper below additionally protects absent destinations from any writer.
    let _directory_lock = DirectoryWriteLock::acquire(parent)?;
    match std::fs::symlink_metadata(file_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            anyhow::bail!(
                "refusing to replace symlinked managed file {}; its target was preserved",
                file_path.display()
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("inspecting {}", file_path.display()));
        }
    }
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("managed-file");
    let temporary = parent.join(format!(
        ".{file_name}.ai-switchboard-{}.tmp",
        Uuid::new_v4()
    ));

    let write_result = (|| -> Result<()> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = options
            .open(&temporary)
            .with_context(|| format!("creating temporary file {}", temporary.display()))?;
        if let Ok(metadata) = std::fs::metadata(file_path) {
            file.set_permissions(metadata.permissions())
                .with_context(|| format!("preserving permissions for {}", file_path.display()))?;
        }
        file.write_all(content)
            .with_context(|| format!("writing temporary file {}", temporary.display()))?;
        file.sync_all()
            .with_context(|| format!("syncing temporary file {}", temporary.display()))?;
        drop(file);
        commit(&temporary, file_path)?;
        #[cfg(unix)]
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    write_result
}

#[cfg(unix)]
struct DirectoryWriteLock(std::fs::File);

#[cfg(unix)]
impl DirectoryWriteLock {
    fn acquire(parent: &Path) -> Result<Self> {
        use std::os::fd::AsRawFd;

        let directory = std::fs::File::open(parent)
            .with_context(|| format!("opening {} for managed-write locking", parent.display()))?;
        let result = unsafe { libc::flock(directory.as_raw_fd(), libc::LOCK_EX) };
        if result != 0 {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("locking {} for managed write", parent.display()));
        }
        Ok(Self(directory))
    }
}

#[cfg(unix)]
impl Drop for DirectoryWriteLock {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;

        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[cfg(not(unix))]
struct DirectoryWriteLock;

#[cfg(not(unix))]
impl DirectoryWriteLock {
    fn acquire(_parent: &Path) -> Result<Self> {
        Ok(Self)
    }
}

pub(crate) fn find_managed_block_range(content: &str, block_id: &str) -> Option<(usize, usize)> {
    for slug in SwitchboardIdentitySlug::marker_prefixes() {
        let start = managed_marker_start(slug.as_str(), block_id);
        let end = managed_marker_end(slug.as_str(), block_id);
        if let (Some(start_idx), Some(end_idx)) = (content.find(&start), content.find(&end)) {
            if start_idx < end_idx {
                return Some((start_idx, end_idx));
            }
        }
    }
    None
}

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
    atomic_write_bytes(file_path, updated.as_bytes())?;
    Ok((true, backup))
}

/// Computes an isolated Switchboard marker edit without touching the filesystem.
/// Config previews use this to make their before/after diff exactly match apply.
pub(crate) fn managed_block_updated_content(
    existing: &str,
    block_id: &str,
    block_body: &str,
) -> String {
    let start = managed_marker_start(primary_marker_prefix(), block_id);
    let end = managed_marker_end(primary_marker_prefix(), block_id);
    let block = format!("{start}\n{block_body}\n{end}\n");
    for slug in SwitchboardIdentitySlug::marker_prefixes() {
        let legacy_start = managed_marker_start(slug.as_str(), block_id);
        let legacy_end = managed_marker_end(slug.as_str(), block_id);
        if let (Some(start_idx), Some(end_idx)) =
            (existing.find(&legacy_start), existing.find(&legacy_end))
        {
            return replace_marker_block(existing, start_idx, end_idx + legacy_end.len(), &block);
        }
    }
    if existing.trim().is_empty() {
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
    atomic_write_bytes(file_path, content.as_bytes())?;

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

    for slug in SwitchboardIdentitySlug::marker_prefixes() {
        let start = managed_marker_start(slug.as_str(), block_id);
        let end = managed_marker_end(slug.as_str(), block_id);
        if let (Some(start_idx), Some(end_idx)) = (existing.find(&start), existing.find(&end)) {
            return remove_marker_range(file_path, &existing, start, end, start_idx, end_idx);
        }
    }
    Ok((false, None))
}

fn remove_marker_range(
    file_path: &Path,
    existing: &str,
    _start: String,
    end: String,
    start_idx: usize,
    end_idx: usize,
) -> Result<(bool, Option<PathBuf>)> {
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
    atomic_write_bytes(file_path, rebuilt.as_bytes())?;
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
    // Backups can contain provider endpoints, account identifiers, or other
    // user configuration. Never let a permissive source mode make the backup
    // group/world-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(&backup_path)
            .with_context(|| format!("reading backup permissions {}", backup_path.display()))?
            .permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(&backup_path, permissions)
            .with_context(|| format!("restricting backup permissions {}", backup_path.display()))?;
    }

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

#[cfg(test)]
mod security_tests {
    use super::{
        atomic_remove_file_if_unchanged, atomic_write_bytes, atomic_write_bytes_if_absent,
        atomic_write_bytes_if_unchanged, atomic_write_bytes_with_commit, backup_if_exists,
    };

    #[test]
    fn atomic_write_replaces_content_without_leaving_a_temporary_file() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let path = temp.path().join("receipt.json");
        std::fs::write(&path, b"before").expect("seed file");

        atomic_write_bytes(&path, b"after").expect("atomic write");

        assert_eq!(std::fs::read(&path).expect("replacement"), b"after");
        assert_eq!(
            std::fs::read_dir(temp.path()).expect("directory").count(),
            1
        );
    }

    #[test]
    fn failed_atomic_commit_preserves_current_file_and_cleans_temporary_file() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let path = temp.path().join("receipt.json");
        std::fs::write(&path, b"owned-before").expect("seed file");

        let error = atomic_write_bytes_with_commit(&path, b"new-value", |_temporary, _path| {
            anyhow::bail!("injected commit failure")
        })
        .expect_err("commit must fail");

        assert!(error.to_string().contains("injected commit failure"));
        assert_eq!(
            std::fs::read(&path).expect("preserved current file"),
            b"owned-before"
        );
        assert_eq!(
            std::fs::read_dir(temp.path()).expect("directory").count(),
            1
        );
    }

    #[test]
    fn compare_and_replace_preserves_a_concurrently_changed_file() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let path = temp.path().join("instructions.md");
        std::fs::write(&path, b"new-editor-content").expect("seed current file");

        let error =
            atomic_write_bytes_if_unchanged(&path, b"stale-snapshot", b"switchboard-replacement")
                .expect_err("stale compare must fail");

        assert!(error
            .to_string()
            .contains("changed before its managed update"));
        assert_eq!(
            std::fs::read(&path).expect("preserved editor content"),
            b"new-editor-content"
        );
        assert_eq!(
            std::fs::read_dir(temp.path()).expect("directory").count(),
            1
        );
    }

    #[test]
    fn no_clobber_publish_preserves_a_concurrently_created_file() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let path = temp.path().join("AGENTS.md");
        std::fs::write(&path, b"editor-created").expect("seed competing file");

        let error = atomic_write_bytes_if_absent(&path, b"switchboard-created")
            .expect_err("existing destination must win");

        assert!(error.to_string().contains("only while it remains absent"));
        assert_eq!(
            std::fs::read(&path).expect("preserved competing file"),
            b"editor-created"
        );
        assert_eq!(
            std::fs::read_dir(temp.path()).expect("directory").count(),
            1
        );
    }

    #[test]
    fn compare_and_remove_preserves_a_concurrently_changed_file() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let path = temp.path().join("ponytail.json");
        std::fs::write(&path, b"new-receipt").expect("seed changed receipt");

        let error = atomic_remove_file_if_unchanged(&path, b"stale-receipt")
            .expect_err("stale removal must fail");

        assert!(error
            .to_string()
            .contains("changed before its managed removal"));
        assert_eq!(
            std::fs::read(&path).expect("preserved receipt"),
            b"new-receipt"
        );
    }

    #[test]
    fn compare_and_remove_unlinks_the_exact_owned_file() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let path = temp.path().join("ponytail.json");
        std::fs::write(&path, b"owned-receipt").expect("seed owned receipt");

        atomic_remove_file_if_unchanged(&path, b"owned-receipt").expect("exact removal");

        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_rejects_symlinks_without_replacing_link_or_target() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::TempDir::new().expect("temp dir");
        let target = temp.path().join("shared-instructions.md");
        let link = temp.path().join("AGENTS.md");
        std::fs::write(&target, b"shared-before").expect("seed target");
        symlink(&target, &link).expect("create symlink");

        let error = atomic_write_bytes(&link, b"replacement").expect_err("symlink must fail");

        assert!(error.to_string().contains("refusing to replace symlinked"));
        assert!(std::fs::symlink_metadata(&link)
            .expect("link metadata")
            .file_type()
            .is_symlink());
        assert_eq!(
            std::fs::read(&target).expect("preserved target"),
            b"shared-before"
        );
    }

    #[cfg(unix)]
    #[test]
    fn config_backups_are_owner_read_write_only() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::TempDir::new().expect("temp dir");
        let config = temp.path().join("provider-config.json");
        std::fs::write(&config, r#"{"token":"fixture-only"}"#).expect("seed config");
        std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o644))
            .expect("make source deliberately permissive");

        let backup = backup_if_exists(&config)
            .expect("create backup")
            .expect("backup path");
        let mode = std::fs::metadata(&backup)
            .expect("backup metadata")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(mode, 0o600);
        assert_eq!(
            std::fs::read_to_string(backup).expect("backup content"),
            r#"{"token":"fixture-only"}"#
        );
    }
}

pub(crate) fn managed_marker_start(prefix: &str, block_id: &str) -> String {
    format!("# >>> {prefix}:{block_id} >>>")
}

pub(crate) fn managed_marker_end(prefix: &str, block_id: &str) -> String {
    format!("# <<< {prefix}:{block_id} <<<")
}

pub(crate) fn strip_marker_block(content: &str, block_id: &str) -> String {
    let mut stripped = content.to_string();
    for prefix in SwitchboardIdentitySlug::marker_prefixes() {
        stripped = strip_marker_block_with_prefix(stripped.as_str(), block_id, prefix.as_str());
    }
    stripped
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
    SwitchboardIdentitySlug::marker_prefixes()
        .iter()
        .any(|slug| marker_block_contains_with_prefix(content, block_id, needle, slug.as_str()))
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
