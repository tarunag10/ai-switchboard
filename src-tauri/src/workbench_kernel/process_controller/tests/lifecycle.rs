use super::super::receipt::{FakeProcessState, FakeTerminalOutcome};
use super::super::stream::MAX_CLASSIFIED_STREAM_BYTES;
use super::helpers::{assert_no_sensitive_keys, fixture, grant_store, open_controller};
use crate::workbench_kernel::events::WorkbenchSessionAction;
use chrono::Duration;

#[test]
fn happy_path_is_explicit_deterministic_and_idempotent() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let (session, process, admission, grant, now) = fixture(1);
    let grants = grant_store(&directory.path().join("grants.json"), &[&grant]);
    let mut controller = open_controller(&path);

    let authorized = controller
        .register(&session, &process, &admission)
        .expect("register fake run");
    assert_eq!(authorized.state, FakeProcessState::Authorized);
    assert_eq!(
        controller
            .register(&session, &process, &admission)
            .expect("idempotent registration"),
        authorized
    );

    let starting = controller
        .start(&session, &process, &admission, &grants, now)
        .expect("start fake run");
    assert_eq!(starting.state, FakeProcessState::Starting);
    assert_eq!(
        controller
            .start(&session, &process, &admission, &grants, now)
            .expect("idempotent start"),
        starting
    );

    let running = controller
        .mark_running(&process.run_id)
        .expect("mark fake run running");
    assert_eq!(running.state, FakeProcessState::Running);
    assert_eq!(
        controller
            .mark_running(&process.run_id)
            .expect("idempotent running transition"),
        running
    );

    let stopping = controller.stop(&process.run_id).expect("stop fake run");
    assert_eq!(stopping.state, FakeProcessState::Stopping);
    assert_eq!(
        controller.stop(&process.run_id).expect("idempotent stop"),
        stopping
    );

    let terminal = controller
        .finalize(&process.run_id, FakeTerminalOutcome::Cancelled)
        .expect("finalize fake run");
    assert_eq!(terminal.state, FakeProcessState::Cancelled);
    assert_eq!(
        controller
            .finalize(&process.run_id, FakeTerminalOutcome::Cancelled)
            .expect("idempotent finalize"),
        terminal
    );

    let reopened = open_controller(&path);
    assert_eq!(reopened.reconciled_orphan_count(), 0);
    assert_eq!(
        reopened.receipt(&process.run_id).expect("reload receipt"),
        terminal
    );
}

#[test]
fn invalid_transitions_and_binding_drift_fail_closed() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let (session, process, admission, grant, now) = fixture(2);
    let grants = grant_store(&directory.path().join("grants.json"), &[&grant]);
    let mut controller = open_controller(&path);
    controller
        .register(&session, &process, &admission)
        .expect("register fake run");

    assert!(controller.mark_running(&process.run_id).is_err());
    assert!(controller
        .finalize(&process.run_id, FakeTerminalOutcome::Succeeded)
        .is_err());
    let (missing_session, missing_process, missing_admission, missing_grant, missing_now) =
        fixture(200);
    let missing_grants = grant_store(
        &directory.path().join("missing-grants.json"),
        &[&missing_grant],
    );
    assert!(controller
        .start(
            &missing_session,
            &missing_process,
            &missing_admission,
            &missing_grants,
            missing_now,
        )
        .is_err());

    controller
        .start(&session, &process, &admission, &grants, now)
        .expect("start fake run");
    controller.stop(&process.run_id).expect("stop fake run");
    assert!(controller
        .start(&session, &process, &admission, &grants, now)
        .is_err());
    let failed = controller
        .finalize(&process.run_id, FakeTerminalOutcome::Failed)
        .expect("finalize failed run");
    assert_eq!(failed.state, FakeProcessState::Failed);
    assert!(controller.mark_running(&process.run_id).is_err());
    assert!(controller
        .finalize(&process.run_id, FakeTerminalOutcome::Succeeded)
        .is_err());
    assert!(controller
        .finalize(&process.run_id, FakeTerminalOutcome::Orphaned)
        .is_err());

    let (other_session, _, _, _, _) = fixture(3);
    assert!(controller
        .register(&other_session, &process, &admission)
        .is_err());
}

#[test]
fn start_revalidates_live_session_state() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let (session, process, admission, grant, now) = fixture(30);
    let grants = grant_store(&directory.path().join("grants.json"), &[&grant]);
    let mut controller = open_controller(&path);
    controller
        .register(&session, &process, &admission)
        .expect("register fake run");

    let mut paused = session.clone();
    paused
        .transition(WorkbenchSessionAction::Pause)
        .expect("pause session");
    assert!(controller
        .start(&paused, &process, &admission, &grants, now)
        .is_err());

    let mut completed = session.clone();
    completed
        .transition(WorkbenchSessionAction::Complete)
        .expect("complete session");
    assert!(controller
        .start(&completed, &process, &admission, &grants, now)
        .is_err());
    assert_eq!(
        controller
            .receipt(&process.run_id)
            .expect("authorized receipt")
            .state,
        FakeProcessState::Authorized
    );
}

#[test]
fn start_revalidates_current_grant_expiry_and_revocation() {
    let directory = tempfile::tempdir().expect("temporary directory");

    let (expired_session, expired_process, expired_admission, expired_grant, expired_now) =
        fixture(31);
    let expired_store = grant_store(
        &directory.path().join("expired-grants.json"),
        &[&expired_grant],
    );
    let mut expired_controller = open_controller(&directory.path().join("expired-processes.json"));
    expired_controller
        .register(&expired_session, &expired_process, &expired_admission)
        .expect("register expiry run");
    assert!(expired_controller
        .start(
            &expired_session,
            &expired_process,
            &expired_admission,
            &expired_store,
            expired_now + Duration::minutes(15),
        )
        .is_err());

    let (revoked_session, revoked_process, revoked_admission, revoked_grant, revoked_now) =
        fixture(32);
    let revoked_store = grant_store(
        &directory.path().join("revoked-grants.json"),
        &[&revoked_grant],
    );
    let mut revoked_controller = open_controller(&directory.path().join("revoked-processes.json"));
    revoked_controller
        .register(&revoked_session, &revoked_process, &revoked_admission)
        .expect("register revoked run");
    revoked_store
        .revoke(&revoked_grant.grant_id, revoked_now + Duration::seconds(1))
        .expect("revoke process grant");
    assert!(revoked_controller
        .start(
            &revoked_session,
            &revoked_process,
            &revoked_admission,
            &revoked_store,
            revoked_now + Duration::seconds(1),
        )
        .is_err());
}

#[test]
fn stream_observation_is_bounded_redacted_and_content_free() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let (session, process, admission, grant, now) = fixture(4);
    let grants = grant_store(&directory.path().join("grants.json"), &[&grant]);
    let mut controller = open_controller(&path);
    controller
        .register(&session, &process, &admission)
        .expect("register fake run");
    controller
        .start(&session, &process, &admission, &grants, now)
        .expect("start fake run");
    controller
        .mark_running(&process.run_id)
        .expect("mark running");

    let secret = b"Authorization: Bearer sk-never-persist-this";
    controller
        .observe_stream_bytes(&process.run_id, secret)
        .expect("classify sensitive bytes");
    controller
        .observe_stream_bytes(&process.run_id, &[0xff, 0x00, b'x'])
        .expect("classify invalid bytes");
    controller
        .observe_stream_bytes(
            &process.run_id,
            &vec![b'x'; MAX_CLASSIFIED_STREAM_BYTES as usize + 17],
        )
        .expect("bound stream accounting");

    let receipt = controller.receipt(&process.run_id).expect("receipt");
    assert_eq!(
        receipt.stream_metadata.classified_bytes,
        MAX_CLASSIFIED_STREAM_BYTES
    );
    assert!(receipt.stream_metadata.dropped_bytes >= 17);
    assert_eq!(receipt.stream_metadata.invalid_utf8_chunks, 1);
    assert!(receipt.stream_metadata.redacted_chunks >= 2);

    let serialized = serde_json::to_value(&receipt).expect("serialize receipt");
    assert_no_sensitive_keys(&serialized);
    let text = serde_json::to_string(&receipt).expect("serialize receipt text");
    assert!(!text.contains("never-persist-this"));
    assert!(!text.contains("Authorization"));
    assert!(!text.contains("Bearer"));
}
