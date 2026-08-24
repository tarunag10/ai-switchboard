#![cfg(unix)]

use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

use super::codex_command_catalog::{
    codex_command_catalog, CodexCandidateObservation, CodexResolvedCandidateKind,
};
use super::codex_command_collector::{
    collect_codex_command_snapshot_with_roots, observe_candidate_with_test_hook, CollectorHookPoint,
};

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("collector fixture");
        let root = directory.path().join("root");
        let home = root.join("Users/researcher");
        fs::create_dir_all(&home).expect("fixture home");
        Self {
            _directory: directory,
            root,
            home,
        }
    }

    fn candidate(&self, template: &str) -> PathBuf {
        template
            .strip_prefix("$HOME/")
            .map(|relative| self.home.join(relative))
            .unwrap_or_else(|| {
                self.root.join(
                    template
                        .strip_prefix('/')
                        .expect("absolute catalog template"),
                )
            })
    }
}

fn write_executable(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("candidate parent")).expect("create parent");
    fs::write(path, bytes).expect("write candidate");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make executable");
}

fn assert_unsafe(observation: CodexCandidateObservation) {
    assert!(matches!(
        observation,
        CodexCandidateObservation::Present {
            resolved_kind: CodexResolvedCandidateKind::UnsafeResolution,
            ..
        }
    ));
}

#[test]
fn stable_symlink_rejects_target_inode_replacement_after_hash() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    let target = fixture.root.join("artifacts/codex");
    let replacement = fixture.root.join("artifacts/replacement");
    write_executable(&target, b"old-target");
    write_executable(&replacement, b"new-target");
    fs::create_dir_all(candidate.parent().expect("candidate parent")).expect("create parent");
    symlink(&target, &candidate).expect("candidate symlink");
    assert_unsafe(observe_candidate_with_test_hook(
        entry,
        Some(&fixture.home),
        &fixture.root,
        &mut |point, _| {
            if point == CollectorHookPoint::AfterHash {
                fs::rename(&replacement, &target).expect("replace target inode");
            }
        },
    ));
}

#[test]
fn leaf_retarget_between_lstat_and_canonicalization_is_unsafe() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    let first = fixture.root.join("artifacts/first");
    let second = fixture.root.join("artifacts/second");
    write_executable(&first, b"first");
    write_executable(&second, b"second");
    fs::create_dir_all(candidate.parent().expect("candidate parent")).expect("create parent");
    symlink(&first, &candidate).expect("initial symlink");
    assert_unsafe(observe_candidate_with_test_hook(
        entry,
        Some(&fixture.home),
        &fixture.root,
        &mut |point, _| {
            if point == CollectorHookPoint::AfterLeafMetadata {
                fs::remove_file(&candidate).expect("remove initial symlink");
                symlink(&second, &candidate).expect("retarget leaf");
            }
        },
    ));
}

#[test]
fn target_replaced_with_symlink_before_open_is_unsafe() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    let replacement = fixture.root.join("artifacts/replacement");
    write_executable(&candidate, b"initial-target");
    write_executable(&replacement, b"replacement-target");
    assert_unsafe(observe_candidate_with_test_hook(
        entry,
        Some(&fixture.home),
        &fixture.root,
        &mut |point, _| {
            if point == CollectorHookPoint::AfterResolvedMetadata {
                fs::remove_file(&candidate).expect("remove resolved target");
                symlink(&replacement, &candidate).expect("replace target with symlink");
            }
        },
    ));
}

#[test]
fn fifo_is_classified_without_opening_or_blocking() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    fs::create_dir_all(candidate.parent().expect("candidate parent")).expect("create parent");
    let candidate_c = CString::new(candidate.as_os_str().as_bytes()).expect("fifo path");
    assert_eq!(unsafe { libc::mkfifo(candidate_c.as_ptr(), 0o700) }, 0);
    assert!(matches!(
        observe_candidate_with_test_hook(entry, Some(&fixture.home), &fixture.root, &mut |_, _| {},),
        CodexCandidateObservation::Present {
            resolved_kind: CodexResolvedCandidateKind::SpecialFile,
            identity_digest: None,
            ..
        }
    ));
}

#[test]
fn unreadable_executable_is_an_observation_failure() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    write_executable(&candidate, b"unreadable");
    fs::set_permissions(&candidate, fs::Permissions::from_mode(0o111))
        .expect("remove read permission");
    assert!(matches!(
        observe_candidate_with_test_hook(entry, Some(&fixture.home), &fixture.root, &mut |_, _| {},),
        CodexCandidateObservation::ObservationFailed { .. }
    ));
}

#[test]
fn escaped_parent_with_missing_leaf_is_never_reported_absent() {
    let directory = tempfile::tempdir().expect("collector fixture");
    let root = directory.path().join("root");
    let outside = directory.path().join("outside");
    fs::create_dir_all(&root).expect("fixture root");
    fs::create_dir_all(outside.join("researcher")).expect("outside home");
    symlink(&outside, root.join("Users")).expect("escaped Users parent");
    let home = root.join("Users/researcher");
    let snapshot = collect_codex_command_snapshot_with_roots(Some(&home), &root);
    for entry in codex_command_catalog()
        .iter()
        .filter(|entry| entry.location_template.starts_with("$HOME/"))
    {
        assert!(snapshot.observations.iter().any(|observation| matches!(
            observation,
            CodexCandidateObservation::Present {
                candidate_id,
                resolved_kind: CodexResolvedCandidateKind::UnsafeResolution,
                ..
            } if candidate_id == entry.candidate_id
        )));
    }
}
