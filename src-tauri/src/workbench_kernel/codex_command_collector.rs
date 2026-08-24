//! Native-only metadata collection for the fixed Codex command catalog.
//!
//! Collection observes only the catalogued filesystem locations. It does not
//! search shell lookup locations, start a child, read credentials, inspect a
//! workspace, persist a path, or expose renderer authority.

use std::fs;
use std::path::{Component, Path, PathBuf};

use super::codex_command_catalog::{
    codex_command_catalog, CodexCandidateObservation, CodexCommandCatalogEntry,
    CodexCommandSnapshot, CodexResolvedCandidateKind, CATALOG_SCHEMA_VERSION,
};
use super::codex_command_identity::{
    account_home_directory, hash_bounded_file, identity_digest, metadata_identity,
    metadata_is_executable, open_without_following, HashError, MetadataIdentity,
};

pub(super) const MAX_CODEX_IDENTITY_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) fn collect_codex_command_snapshot() -> CodexCommandSnapshot {
    let home = account_home_directory().ok();
    collect_codex_command_snapshot_with_roots(home.as_deref(), Path::new("/"))
}

pub(super) fn collect_codex_command_snapshot_with_roots(
    home: Option<&Path>,
    filesystem_root: &Path,
) -> CodexCommandSnapshot {
    let canonical_root = fs::canonicalize(filesystem_root).ok();
    let observations = codex_command_catalog()
        .iter()
        .map(|entry| {
            match (
                resolve_candidate_path(entry, home, filesystem_root),
                canonical_root.as_deref(),
            ) {
                (Ok(path), Some(root)) => {
                    observe_candidate(entry.candidate_id, &path, filesystem_root, root)
                }
                _ => CodexCandidateObservation::ObservationFailed {
                    candidate_id: entry.candidate_id.into(),
                },
            }
        })
        .collect();
    CodexCommandSnapshot {
        schema_version: CATALOG_SCHEMA_VERSION,
        observations,
    }
}

pub(super) fn resolve_candidate_path(
    entry: &CodexCommandCatalogEntry,
    home: Option<&Path>,
    filesystem_root: &Path,
) -> Result<PathBuf, ()> {
    if !safe_absolute_path(filesystem_root) {
        return Err(());
    }
    if let Some(relative) = entry.location_template.strip_prefix("$HOME/") {
        let home = home.ok_or(())?;
        let scoped_home = home.strip_prefix(filesystem_root).map_err(|_| ())?;
        if scoped_home.as_os_str().is_empty()
            || !safe_component_path(scoped_home)
            || !safe_relative_path(relative)
        {
            return Err(());
        }
        return Ok(filesystem_root.join(scoped_home).join(relative));
    }
    let relative = entry.location_template.strip_prefix('/').ok_or(())?;
    if !safe_relative_path(relative) {
        return Err(());
    }
    Ok(filesystem_root.join(relative))
}

fn safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::Normal(_)
            )
        })
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty() && safe_component_path(Path::new(value))
}

fn safe_component_path(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CollectorHookPoint {
    AfterLeafMetadata,
    AfterResolvedMetadata,
    AfterHash,
}

fn observe_candidate(
    candidate_id: &str,
    requested_path: &Path,
    filesystem_root: &Path,
    canonical_root: &Path,
) -> CodexCandidateObservation {
    observe_candidate_with_hook(
        candidate_id,
        requested_path,
        filesystem_root,
        canonical_root,
        &mut |_, _| {},
    )
}

pub(super) fn observe_candidate_with_test_hook(
    entry: &CodexCommandCatalogEntry,
    home: Option<&Path>,
    filesystem_root: &Path,
    hook: &mut impl FnMut(CollectorHookPoint, &Path),
) -> CodexCandidateObservation {
    let Some(canonical_root) = fs::canonicalize(filesystem_root).ok() else {
        return observation_failed(entry.candidate_id);
    };
    let Ok(path) = resolve_candidate_path(entry, home, filesystem_root) else {
        return observation_failed(entry.candidate_id);
    };
    observe_candidate_with_hook(
        entry.candidate_id,
        &path,
        filesystem_root,
        &canonical_root,
        hook,
    )
}

fn observe_candidate_with_hook(
    candidate_id: &str,
    requested_path: &Path,
    filesystem_root: &Path,
    canonical_root: &Path,
    hook: &mut impl FnMut(CollectorHookPoint, &Path),
) -> CodexCandidateObservation {
    let safe_leaf = match scope_candidate_leaf(requested_path, filesystem_root, canonical_root) {
        ScopeResult::Ready(path) => path,
        ScopeResult::ConfirmedAbsent => return confirmed_absent(candidate_id),
        ScopeResult::Unsafe => return unsafe_resolution(candidate_id),
        ScopeResult::Failed => return observation_failed(candidate_id),
    };
    let candidate_metadata = match fs::symlink_metadata(&safe_leaf) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return confirmed_absent(candidate_id);
        }
        Err(_) => {
            return observation_failed(candidate_id);
        }
    };

    let candidate_identity = metadata_identity(&candidate_metadata);
    let leaf_is_symlink = candidate_metadata.file_type().is_symlink();
    hook(CollectorHookPoint::AfterLeafMetadata, &safe_leaf);
    let resolved_path = match fs::canonicalize(&safe_leaf) {
        Ok(path) if path.is_absolute() && path.starts_with(canonical_root) => path,
        Ok(_) => return unsafe_resolution(candidate_id),
        Err(error)
            if leaf_is_symlink
                && (error.kind() == std::io::ErrorKind::NotFound
                    || error.raw_os_error() == Some(libc::ELOOP)) =>
        {
            return match resolution_stability(
                requested_path,
                filesystem_root,
                canonical_root,
                &safe_leaf,
                &candidate_identity,
                None,
            ) {
                Stability::Stable => present(
                    candidate_id,
                    CodexResolvedCandidateKind::UnresolvedSymlink,
                    false,
                    None,
                ),
                Stability::Unsafe => unsafe_resolution(candidate_id),
                Stability::Failed => observation_failed(candidate_id),
            };
        }
        Err(error) if is_drift_error(&error) => return unsafe_resolution(candidate_id),
        Err(_) => return observation_failed(candidate_id),
    };
    match resolution_stability(
        requested_path,
        filesystem_root,
        canonical_root,
        &safe_leaf,
        &candidate_identity,
        None,
    ) {
        Stability::Stable => {}
        Stability::Unsafe => return unsafe_resolution(candidate_id),
        Stability::Failed => return observation_failed(candidate_id),
    }

    let resolved_metadata = match fs::symlink_metadata(&resolved_path) {
        Ok(metadata) => metadata,
        Err(error) if is_drift_error(&error) => return unsafe_resolution(candidate_id),
        Err(_) => return observation_failed(candidate_id),
    };
    if resolved_metadata.file_type().is_symlink() {
        return unsafe_resolution(candidate_id);
    }
    hook(CollectorHookPoint::AfterResolvedMetadata, &resolved_path);
    let executable = metadata_is_executable(&resolved_metadata);
    let kind = if resolved_metadata.is_file() {
        CodexResolvedCandidateKind::RegularFile
    } else if resolved_metadata.is_dir() {
        CodexResolvedCandidateKind::Directory
    } else {
        CodexResolvedCandidateKind::SpecialFile
    };
    if kind != CodexResolvedCandidateKind::RegularFile {
        return match resolution_stability(
            requested_path,
            filesystem_root,
            canonical_root,
            &safe_leaf,
            &candidate_identity,
            Some((&resolved_path, &metadata_identity(&resolved_metadata))),
        ) {
            Stability::Stable => present(candidate_id, kind, executable, None),
            Stability::Unsafe => unsafe_resolution(candidate_id),
            Stability::Failed => observation_failed(candidate_id),
        };
    }
    if resolved_metadata.len() > MAX_CODEX_IDENTITY_BYTES {
        return observation_failed(candidate_id);
    }

    let file = match open_without_following(&resolved_path) {
        Ok(file) => file,
        Err(error) if is_drift_error(&error) => return unsafe_resolution(candidate_id),
        Err(_) => return observation_failed(candidate_id),
    };
    let opened_before = match file.metadata() {
        Ok(metadata) => metadata,
        Err(_) => return unsafe_resolution(candidate_id),
    };
    if metadata_identity(&opened_before) != metadata_identity(&resolved_metadata)
        || !opened_before.is_file()
        || opened_before.len() > MAX_CODEX_IDENTITY_BYTES
    {
        return unsafe_resolution(candidate_id);
    }
    let digest = match hash_bounded_file(file, MAX_CODEX_IDENTITY_BYTES) {
        Ok((content_digest, opened_after)) => {
            hook(CollectorHookPoint::AfterHash, &resolved_path);
            if metadata_identity(&opened_before) != metadata_identity(&opened_after) {
                return unsafe_resolution(candidate_id);
            }
            match resolution_stability(
                requested_path,
                filesystem_root,
                canonical_root,
                &safe_leaf,
                &candidate_identity,
                Some((&resolved_path, &metadata_identity(&opened_after))),
            ) {
                Stability::Stable => {}
                Stability::Unsafe => return unsafe_resolution(candidate_id),
                Stability::Failed => return observation_failed(candidate_id),
            }
            identity_digest(
                candidate_id,
                &candidate_identity,
                &metadata_identity(&opened_after),
                content_digest,
            )
        }
        Err(HashError::GrewPastLimit) => return unsafe_resolution(candidate_id),
        Err(HashError::ReadFailed) => return observation_failed(candidate_id),
    };
    present(candidate_id, kind, executable, Some(digest))
}

enum ScopeResult {
    Ready(PathBuf),
    ConfirmedAbsent,
    Unsafe,
    Failed,
}

fn scope_candidate_leaf(path: &Path, root: &Path, canonical_root: &Path) -> ScopeResult {
    let Ok(relative) = path.strip_prefix(root) else {
        return ScopeResult::Unsafe;
    };
    if !safe_component_path(relative) {
        return ScopeResult::Unsafe;
    }
    let parts = relative.components().collect::<Vec<_>>();
    let Some((leaf, parents)) = parts.split_last() else {
        return ScopeResult::Unsafe;
    };
    let Component::Normal(leaf) = leaf else {
        return ScopeResult::Unsafe;
    };
    let mut current = canonical_root.to_path_buf();
    for part in parents {
        let Component::Normal(part) = part else {
            return ScopeResult::Unsafe;
        };
        let next = current.join(part);
        let before = match fs::symlink_metadata(&next) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return ScopeResult::ConfirmedAbsent;
            }
            Err(error) if is_drift_error(&error) => return ScopeResult::Unsafe,
            Err(_) => return ScopeResult::Failed,
        };
        let resolved = match fs::canonicalize(&next) {
            Ok(value) if value.starts_with(canonical_root) => value,
            Ok(_) => return ScopeResult::Unsafe,
            Err(error) if is_drift_error(&error) => return ScopeResult::Unsafe,
            Err(_) => return ScopeResult::Failed,
        };
        let after = match fs::symlink_metadata(&next) {
            Ok(value) => value,
            Err(error) if is_drift_error(&error) => return ScopeResult::Unsafe,
            Err(_) => return ScopeResult::Failed,
        };
        let target = match fs::symlink_metadata(&resolved) {
            Ok(value) => value,
            Err(error) if is_drift_error(&error) => return ScopeResult::Unsafe,
            Err(_) => return ScopeResult::Failed,
        };
        if metadata_identity(&before) != metadata_identity(&after)
            || target.file_type().is_symlink()
            || !target.is_dir()
        {
            return ScopeResult::Unsafe;
        }
        current = resolved;
    }
    ScopeResult::Ready(current.join(leaf))
}

enum Stability {
    Stable,
    Unsafe,
    Failed,
}

fn resolution_stability(
    requested: &Path,
    root: &Path,
    canonical_root: &Path,
    expected_leaf_path: &Path,
    expected_leaf: &MetadataIdentity,
    target: Option<(&Path, &MetadataIdentity)>,
) -> Stability {
    let leaf = match scope_candidate_leaf(requested, root, canonical_root) {
        ScopeResult::Ready(path) if path == expected_leaf_path => path,
        ScopeResult::Failed => return Stability::Failed,
        _ => return Stability::Unsafe,
    };
    match fs::symlink_metadata(&leaf) {
        Ok(metadata) if metadata_identity(&metadata) == *expected_leaf => {}
        Ok(_) => return Stability::Unsafe,
        Err(error) if is_drift_error(&error) => return Stability::Unsafe,
        Err(_) => return Stability::Failed,
    }
    let Some((expected_path, expected_target)) = target else {
        return Stability::Stable;
    };
    match fs::canonicalize(&leaf) {
        Ok(path) if path == expected_path => {}
        Ok(_) => return Stability::Unsafe,
        Err(error) if is_drift_error(&error) => return Stability::Unsafe,
        Err(_) => return Stability::Failed,
    }
    match fs::symlink_metadata(expected_path) {
        Ok(metadata)
            if !metadata.file_type().is_symlink()
                && metadata_identity(&metadata) == *expected_target =>
        {
            Stability::Stable
        }
        Ok(_) => Stability::Unsafe,
        Err(error) if is_drift_error(&error) => Stability::Unsafe,
        Err(_) => Stability::Failed,
    }
}

fn is_drift_error(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound
        || matches!(error.raw_os_error(), Some(libc::ELOOP | libc::ENOTDIR))
}

fn present(
    candidate_id: &str,
    resolved_kind: CodexResolvedCandidateKind,
    executable: bool,
    identity_digest: Option<String>,
) -> CodexCandidateObservation {
    CodexCandidateObservation::Present {
        candidate_id: candidate_id.into(),
        resolved_kind,
        executable,
        identity_digest,
    }
}

fn confirmed_absent(candidate_id: &str) -> CodexCandidateObservation {
    CodexCandidateObservation::ConfirmedAbsent {
        candidate_id: candidate_id.into(),
    }
}

fn unsafe_resolution(candidate_id: &str) -> CodexCandidateObservation {
    present(
        candidate_id,
        CodexResolvedCandidateKind::UnsafeResolution,
        false,
        None,
    )
}

fn observation_failed(candidate_id: &str) -> CodexCandidateObservation {
    CodexCandidateObservation::ObservationFailed {
        candidate_id: candidate_id.into(),
    }
}
