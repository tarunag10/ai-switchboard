use super::super::receipt::FakeTerminalOutcome;
use super::super::registry::{WorkbenchFakeProcessRegistry, MAX_FAKE_RUNS};
use super::super::WorkbenchFakeProcessController;
use super::helpers::{fixture, grant_store, open_controller, TEST_OWNER_EPOCH};
use serde_json::Value;
use std::fs;

#[test]
fn registry_capacity_is_enforced_without_overwrite() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let mut controller = open_controller(&path);
    for seed in 100..100 + MAX_FAKE_RUNS {
        let (session, process, admission, _, _) = fixture(seed);
        controller
            .register(&session, &process, &admission)
            .expect("fill fake process registry");
    }
    let (session, process, admission, _, _) = fixture(10_000);
    assert!(controller.register(&session, &process, &admission).is_err());
    assert_eq!(controller.registry.runs.len(), MAX_FAKE_RUNS);
}

#[test]
fn registry_reclaims_oldest_terminal_receipt_deterministically() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let mut controller = open_controller(&path);
    let (first_session, first_process, first_admission, first_grant, first_now) = fixture(300);
    let first_store = grant_store(&directory.path().join("first-grant.json"), &[&first_grant]);
    controller
        .register(&first_session, &first_process, &first_admission)
        .expect("register first run");
    controller
        .start(
            &first_session,
            &first_process,
            &first_admission,
            &first_store,
            first_now,
        )
        .expect("start first run");
    controller
        .finalize(&first_process.run_id, FakeTerminalOutcome::Succeeded)
        .expect("finalize first run");

    for seed in 301..300 + MAX_FAKE_RUNS {
        let (session, process, admission, _, _) = fixture(seed);
        controller
            .register(&session, &process, &admission)
            .expect("fill registry after terminal run");
    }
    let (extra_session, extra_process, extra_admission, _, _) = fixture(30_000);
    let extra = controller
        .register(&extra_session, &extra_process, &extra_admission)
        .expect("reclaim oldest terminal receipt");
    assert_eq!(controller.registry.runs.len(), MAX_FAKE_RUNS);
    assert_eq!(extra.registered_sequence, MAX_FAKE_RUNS as u64);
    assert!(controller.receipt(&first_process.run_id).is_err());
    assert!(controller.receipt(&extra_process.run_id).is_ok());
    assert!(controller
        .registry
        .retired_runs
        .contains_key(&first_process.run_id));

    controller
        .stop(&extra_process.run_id)
        .expect("make a new terminal slot available");
    drop(controller);
    let mut controller = open_controller(&path);
    assert!(controller
        .registry
        .retired_runs
        .contains_key(&first_process.run_id));
    let error = controller
        .register(&first_session, &first_process, &first_admission)
        .expect_err("retired terminal run must never resurrect");
    assert!(error.to_string().contains("terminal"));
    assert_eq!(controller.registry.runs.len(), MAX_FAKE_RUNS);
}

#[test]
fn stale_controller_cannot_overwrite_a_newer_registry() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let mut first = open_controller(&path);
    let mut stale = open_controller(&path);
    let (first_session, first_process, first_admission, _, _) = fixture(20_000);
    let (stale_session, stale_process, stale_admission, _, _) = fixture(20_001);

    first
        .register(&first_session, &first_process, &first_admission)
        .expect("first controller commit");
    let error = stale
        .register(&stale_session, &stale_process, &stale_admission)
        .expect_err("stale controller must fail closed");
    assert!(error.to_string().contains("changed after it was opened"));

    drop(first);
    drop(stale);
    let reopened = open_controller(&path);
    assert!(reopened.receipt(&first_process.run_id).is_ok());
    assert!(reopened.receipt(&stale_process.run_id).is_err());
}

#[test]
fn byte_only_registry_reformatting_invalidates_the_open_snapshot() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let mut controller = open_controller(&path);
    let (first_session, first_process, first_admission, _, _) = fixture(40_000);
    controller
        .register(&first_session, &first_process, &first_admission)
        .expect("register first run");

    let value: Value =
        serde_json::from_slice(&fs::read(&path).expect("read registry")).expect("decode registry");
    fs::write(&path, serde_json::to_vec(&value).expect("compact registry"))
        .expect("reformat registry");

    let (second_session, second_process, second_admission, _, _) = fixture(40_001);
    assert!(controller
        .register(&second_session, &second_process, &second_admission)
        .is_err());
    let reopened = open_controller(&path);
    assert!(reopened.receipt(&first_process.run_id).is_ok());
    assert!(reopened.receipt(&second_process.run_id).is_err());
}

#[test]
fn deleted_registry_is_not_silently_recreated() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let (session, process, admission, _, _) = fixture(40_010);
    let mut controller = open_controller(&path);
    controller
        .register(&session, &process, &admission)
        .expect("register fake run");
    fs::remove_file(&path).expect("remove registry");

    assert!(controller.stop(&process.run_id).is_err());
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn symlink_substitution_is_rejected_at_the_atomic_commit_boundary() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let replacement_target = directory.path().join("replacement-target.json");
    let (session, process, admission, _, _) = fixture(40_020);
    let mut controller = open_controller(&path);
    controller
        .register(&session, &process, &admission)
        .expect("register fake run");
    let expected_target = fs::read(&path).expect("read registry snapshot");
    fs::write(&replacement_target, &expected_target).expect("write replacement target");
    fs::remove_file(&path).expect("remove registry path");
    symlink(&replacement_target, &path).expect("substitute symlink");

    let error = controller
        .stop(&process.run_id)
        .expect_err("symlink destination must fail closed");
    assert!(
        format!("{error:#}").contains("symlinked managed file"),
        "unexpected error: {error:#}"
    );
    assert!(path.is_symlink());
    assert_eq!(
        fs::read(&replacement_target).expect("read preserved symlink target"),
        expected_target
    );
}

#[test]
fn corrupt_and_sensitive_persistence_fields_are_rejected() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let (session, process, admission, _, _) = fixture(5);
    let mut controller = open_controller(&path);
    controller
        .register(&session, &process, &admission)
        .expect("register fake run");
    let valid = fs::read(&path).expect("read valid registry");

    fs::write(&path, b"{not-json").expect("write malformed registry");
    assert!(WorkbenchFakeProcessController::open(path.clone(), TEST_OWNER_EPOCH).is_err());

    let mut unknown_registry: Value =
        serde_json::from_slice(&valid).expect("decode valid registry");
    unknown_registry["environment"] = Value::String("forbidden".into());
    fs::write(
        &path,
        serde_json::to_vec(&unknown_registry).expect("encode unknown registry"),
    )
    .expect("write unknown registry");
    assert!(WorkbenchFakeProcessController::open(path.clone(), TEST_OWNER_EPOCH).is_err());

    let mut sensitive_receipt: Value =
        serde_json::from_slice(&valid).expect("decode valid registry");
    sensitive_receipt["runs"][&process.run_id]["prompt"] = Value::String("forbidden".into());
    fs::write(
        &path,
        serde_json::to_vec(&sensitive_receipt).expect("encode sensitive receipt"),
    )
    .expect("write sensitive receipt");
    assert!(WorkbenchFakeProcessController::open(path.clone(), TEST_OWNER_EPOCH).is_err());

    let mut corrupt_digest: Value = serde_json::from_slice(&valid).expect("decode valid registry");
    corrupt_digest["registryDigest"] = Value::String(format!("sha256:{}", "0".repeat(64)));
    fs::write(
        &path,
        serde_json::to_vec(&corrupt_digest).expect("encode corrupt registry"),
    )
    .expect("write corrupt registry");
    assert!(WorkbenchFakeProcessController::open(path, TEST_OWNER_EPOCH).is_err());
}

#[test]
fn persistence_types_reject_unknown_fields_directly() {
    let registry = WorkbenchFakeProcessRegistry::empty(TEST_OWNER_EPOCH).expect("empty registry");
    let mut value = serde_json::to_value(&registry).expect("serialize registry");
    value["command"] = Value::String("forbidden".into());
    assert!(serde_json::from_value::<WorkbenchFakeProcessRegistry>(value).is_err());
}
