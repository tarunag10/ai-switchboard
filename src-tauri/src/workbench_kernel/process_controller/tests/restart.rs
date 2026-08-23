use super::super::receipt::{FakeProcessState, FakeTerminalOutcome};
use super::helpers::{fixture, grant_store, open_controller, open_controller_for_epoch};

#[test]
fn restart_reconciliation_orphans_only_active_fake_runs_once() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fake-processes.json");
    let mut controller = open_controller(&path);
    let mut fixtures = Vec::new();
    for seed in 10..15 {
        let fixture = fixture(seed);
        controller
            .register(&fixture.0, &fixture.1, &fixture.2)
            .expect("register fake run");
        fixtures.push(fixture);
    }
    let grant_refs = fixtures
        .iter()
        .map(|fixture| &fixture.3)
        .collect::<Vec<_>>();
    let grants = grant_store(&directory.path().join("grants.json"), &grant_refs);

    controller
        .start(
            &fixtures[1].0,
            &fixtures[1].1,
            &fixtures[1].2,
            &grants,
            fixtures[1].4,
        )
        .expect("starting run");
    controller
        .start(
            &fixtures[2].0,
            &fixtures[2].1,
            &fixtures[2].2,
            &grants,
            fixtures[2].4,
        )
        .expect("start running run");
    controller
        .mark_running(&fixtures[2].1.run_id)
        .expect("running run");
    controller
        .start(
            &fixtures[3].0,
            &fixtures[3].1,
            &fixtures[3].2,
            &grants,
            fixtures[3].4,
        )
        .expect("start stopping run");
    controller
        .mark_running(&fixtures[3].1.run_id)
        .expect("running before stop");
    controller
        .stop(&fixtures[3].1.run_id)
        .expect("stopping run");
    controller
        .start(
            &fixtures[4].0,
            &fixtures[4].1,
            &fixtures[4].2,
            &grants,
            fixtures[4].4,
        )
        .expect("start terminal run");
    controller
        .finalize(&fixtures[4].1.run_id, FakeTerminalOutcome::Succeeded)
        .expect("terminal run");

    let same_launch = open_controller(&path);
    assert_eq!(same_launch.reconciled_orphan_count(), 0);
    assert_eq!(
        same_launch
            .receipt(&fixtures[2].1.run_id)
            .expect("same-launch running receipt")
            .state,
        FakeProcessState::Running
    );
    drop(same_launch);
    drop(controller);

    let reopened = open_controller_for_epoch(&path, "fake-controller-epoch:next-launch");
    assert_eq!(reopened.reconciled_orphan_count(), 3);
    assert_eq!(
        reopened
            .receipt(&fixtures[0].1.run_id)
            .expect("authorized receipt")
            .state,
        FakeProcessState::Authorized
    );
    for fixture in &fixtures[1..4] {
        let receipt = reopened
            .receipt(&fixture.1.run_id)
            .expect("orphaned receipt");
        assert_eq!(receipt.state, FakeProcessState::Orphaned);
        assert_eq!(
            receipt.terminal_outcome,
            Some(FakeTerminalOutcome::Orphaned)
        );
    }
    assert_eq!(
        reopened
            .receipt(&fixtures[4].1.run_id)
            .expect("terminal receipt")
            .state,
        FakeProcessState::Succeeded
    );
    drop(reopened);

    let reopened_again = open_controller_for_epoch(&path, "fake-controller-epoch:next-launch");
    assert_eq!(reopened_again.reconciled_orphan_count(), 0);
    for fixture in &fixtures[1..4] {
        assert_eq!(
            reopened_again
                .receipt(&fixture.1.run_id)
                .expect("stable orphan receipt")
                .state,
            FakeProcessState::Orphaned
        );
    }
}
