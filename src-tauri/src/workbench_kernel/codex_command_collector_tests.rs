use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};

use super::codex_command_catalog::{
    codex_command_catalog, evaluate_codex_command_snapshot, CodexCandidateObservation,
    CodexCommandCatalogEntry, CodexResolvedCandidateKind,
};
use super::codex_command_collector::{
    collect_codex_command_snapshot_with_roots, resolve_candidate_path, MAX_CODEX_IDENTITY_BYTES,
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

    fn collect(&self) -> super::codex_command_catalog::CodexCommandSnapshot {
        collect_codex_command_snapshot_with_roots(Some(&self.home), &self.root)
    }
}

#[cfg(unix)]
fn write_executable(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("candidate parent")).expect("create parent");
    fs::write(path, bytes).expect("write candidate");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make executable");
}

#[test]
fn complete_fixed_catalog_can_confirm_absence_without_shell_lookup() {
    let fixture = Fixture::new();
    let snapshot = fixture.collect();
    assert_eq!(snapshot.observations.len(), codex_command_catalog().len());
    assert!(snapshot
        .observations
        .iter()
        .all(|value| matches!(value, CodexCandidateObservation::ConfirmedAbsent { .. })));
    let evaluation = evaluate_codex_command_snapshot(&snapshot).expect("evaluate absence");
    assert_eq!(evaluation.state, "confirmed_absent_from_fixed_catalog");
    assert!(!evaluation.runnable);
    assert!(!evaluation.process_start_enabled);
}

#[cfg(unix)]
#[test]
fn hashes_one_stable_executable_without_exposing_its_path() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    write_executable(
        &fixture.candidate(entry.location_template),
        b"fixed-codex-fixture",
    );
    let snapshot = fixture.collect();
    let evaluation = evaluate_codex_command_snapshot(&snapshot).expect("evaluate candidate");
    assert_eq!(evaluation.state, "present_unprobed");
    assert_eq!(evaluation.candidate_id.as_deref(), Some(entry.candidate_id));
    assert!(evaluation
        .binary_identity_digest
        .as_deref()
        .is_some_and(|digest| digest.starts_with("sha256:") && digest.len() == 71));
    assert!(!format!("{snapshot:?}").contains(fixture.home.to_string_lossy().as_ref()));
}

#[cfg(unix)]
#[test]
fn multiple_fixed_candidates_remain_ambiguous() {
    let fixture = Fixture::new();
    let target = fixture.root.join("artifacts/shared-codex");
    write_executable(&target, b"shared-codex-fixture");
    for entry in &codex_command_catalog()[..2] {
        let candidate = fixture.candidate(entry.location_template);
        fs::create_dir_all(candidate.parent().expect("candidate parent")).expect("create parent");
        symlink(&target, candidate).expect("candidate alias");
    }
    let evaluation =
        evaluate_codex_command_snapshot(&fixture.collect()).expect("evaluate ambiguity");
    assert_eq!(evaluation.state, "ambiguous");
    assert!(evaluation.candidate_id.is_none());
}

#[cfg(unix)]
#[test]
fn symlink_is_identity_checked_and_broken_link_is_rejected() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    let target = fixture.root.join("artifacts/codex");
    write_executable(&target, b"symlinked-codex-fixture");
    fs::create_dir_all(candidate.parent().expect("candidate parent")).expect("create parent");
    symlink(&target, &candidate).expect("candidate symlink");
    assert_eq!(
        evaluate_codex_command_snapshot(&fixture.collect())
            .expect("evaluate symlink")
            .state,
        "present_unprobed"
    );

    fs::remove_file(&target).expect("break symlink");
    let snapshot = fixture.collect();
    assert!(snapshot.observations.iter().any(|observation| matches!(
        observation,
        CodexCandidateObservation::Present {
            candidate_id,
            resolved_kind: CodexResolvedCandidateKind::UnresolvedSymlink,
            ..
        } if candidate_id == entry.candidate_id
    )));
    assert_eq!(
        evaluate_codex_command_snapshot(&snapshot)
            .expect("evaluate broken symlink")
            .state,
        "rejected"
    );
}

#[cfg(unix)]
#[test]
fn non_executable_and_directory_candidates_are_rejected() {
    let cases = ["non_executable", "directory"];
    for case in cases {
        let fixture = Fixture::new();
        let entry = &codex_command_catalog()[0];
        let candidate = fixture.candidate(entry.location_template);
        fs::create_dir_all(candidate.parent().expect("candidate parent")).expect("create parent");
        match case {
            "non_executable" => fs::write(&candidate, b"not executable").expect("write file"),
            "directory" => fs::create_dir(&candidate).expect("create directory"),
            _ => unreachable!(),
        }
        assert_eq!(
            evaluate_codex_command_snapshot(&fixture.collect())
                .expect("evaluate unsafe candidate")
                .state,
            "rejected",
            "case {case}"
        );
    }
}

#[cfg(unix)]
#[test]
fn oversized_candidate_is_an_observation_failure_without_hashing() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    fs::create_dir_all(candidate.parent().expect("candidate parent")).expect("create parent");
    let file = fs::File::create(&candidate).expect("create oversized file");
    file.set_len(MAX_CODEX_IDENTITY_BYTES + 1)
        .expect("size oversized file");
    fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755))
        .expect("make oversized candidate executable");
    assert_eq!(
        evaluate_codex_command_snapshot(&fixture.collect())
            .expect("evaluate oversized candidate")
            .state,
        "observation_failed"
    );
}

#[cfg(unix)]
#[test]
fn canonical_target_outside_injected_root_is_unsafe() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    let outside = fixture
        .root
        .parent()
        .expect("fixture parent")
        .join("outside-codex");
    write_executable(&outside, b"outside-root");
    fs::create_dir_all(candidate.parent().expect("candidate parent")).expect("create parent");
    symlink(outside, candidate).expect("outside-root symlink");
    let snapshot = fixture.collect();
    assert!(snapshot.observations.iter().any(|observation| matches!(
        observation,
        CodexCandidateObservation::Present {
            candidate_id,
            resolved_kind: CodexResolvedCandidateKind::UnsafeResolution,
            ..
        } if candidate_id == entry.candidate_id
    )));
}

#[cfg(unix)]
#[test]
fn identity_digest_binds_metadata_inode_and_content() {
    let fixture = Fixture::new();
    let entry = &codex_command_catalog()[0];
    let candidate = fixture.candidate(entry.location_template);
    write_executable(&candidate, b"identity-a");
    let digest = || {
        evaluate_codex_command_snapshot(&fixture.collect())
            .expect("evaluate identity")
            .binary_identity_digest
            .expect("identity digest")
    };
    let first = digest();
    assert_eq!(
        first,
        digest(),
        "stable metadata and bytes are deterministic"
    );

    fs::set_permissions(&candidate, fs::Permissions::from_mode(0o700)).expect("change mode");
    let mode_changed = digest();
    assert_ne!(first, mode_changed);

    fs::remove_file(&candidate).expect("replace inode");
    write_executable(&candidate, b"identity-a");
    let inode_changed = digest();
    assert_ne!(first, inode_changed);

    fs::write(&candidate, b"identity-b").expect("change bytes");
    let bytes_changed = digest();
    assert_ne!(inode_changed, bytes_changed);
}

#[test]
fn resolver_rejects_parent_traversal_tokens_and_unscoped_home() {
    let fixture = Fixture::new();
    let traversal = CodexCommandCatalogEntry {
        candidate_id: "test",
        location_template: "$HOME/../codex",
    };
    assert!(resolve_candidate_path(&traversal, Some(&fixture.home), &fixture.root).is_err());
    let outside_home = fixture
        .root
        .parent()
        .expect("fixture parent")
        .join("outside-home");
    let normal = CodexCommandCatalogEntry {
        candidate_id: "test",
        location_template: "$HOME/.local/bin/codex",
    };
    assert!(resolve_candidate_path(&normal, Some(&outside_home), &fixture.root).is_err());
    assert!(resolve_candidate_path(&normal, Some(Path::new("relative")), &fixture.root).is_err());
    assert!(resolve_candidate_path(
        &normal,
        Some(&fixture.root.join("../outside")),
        &fixture.root,
    )
    .is_err());
}

#[test]
fn missing_account_home_marks_only_home_templates_failed() {
    let fixture = Fixture::new();
    let snapshot = collect_codex_command_snapshot_with_roots(None, &fixture.root);
    let failed_ids = snapshot
        .observations
        .iter()
        .filter_map(|observation| match observation {
            CodexCandidateObservation::ObservationFailed { candidate_id } => Some(candidate_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(failed_ids.len(), 4);
    assert_eq!(
        evaluate_codex_command_snapshot(&snapshot)
            .expect("evaluate account failure")
            .state,
        "observation_failed"
    );
}

#[test]
fn collector_source_contains_no_process_environment_renderer_or_probe_authority() {
    let source = format!(
        "{}\n{}",
        include_str!("codex_command_collector.rs"),
        include_str!("codex_command_identity.rs")
    );
    let compact = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    for forbidden in [
        "std::process",
        "tokio::process",
        "async_process",
        "command::new",
        ".spawn(",
        ".output(",
        ".status(",
        "std::env",
        "var_os(",
        "dirs::",
        "home_dir(",
        "tauri::",
        "#[tauri",
        "--version",
        "reqwest",
        "ureq::",
        "hyper::",
        "tcpstream",
        "unixstream",
        "serde",
        "fs::write",
        "file::create",
        ".write(true)",
        ".create(true)",
        ".truncate(true)",
        ".append(true)",
    ] {
        assert!(
            !compact.contains(forbidden),
            "unexpected authority: {forbidden}"
        );
    }
}
