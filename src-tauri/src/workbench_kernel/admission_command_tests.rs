use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use chrono::{Duration, TimeZone, Utc};

use crate::client_adapter_contract::VerificationReport;
use crate::models::SwitchboardMode;

use super::adapter_readiness::WorkbenchAdapterCommandReadiness;
use super::admission_command::{
    admit_workbench_process_with_clock, admit_workbench_process_with_dependencies,
};
use super::capability_grant::{
    issue_process_start_grant, process_start_confirmation_phrase, WorkbenchProcessGrantStore,
    WorkbenchProcessStartGrant,
};
use super::events::WorkbenchSessionAction;
use super::process_run_spec::process_run_spec_for;
use super::process_supervisor::{WorkbenchProcessAdmission, WorkbenchProcessAdmissionStore};
use super::session::{CreateWorkbenchSessionInput, WorkbenchSession};
use super::{
    CapabilityRequest, RouterDecisionReference, WorkbenchProcessAdmissionInput, WorkbenchRunPlan,
    WorkbenchRunSpecInput,
};
use switchboard_runtime::{RuntimeClock, RuntimeClockError};

#[derive(Debug)]
struct AdmissionClock<'a> {
    unix_millis: i64,
    calls: &'a AtomicUsize,
    preparation_finished: &'a AtomicBool,
    fail: bool,
}

impl RuntimeClock for AdmissionClock<'_> {
    fn unix_millis(&self) -> i64 {
        self.unix_millis
    }

    fn try_unix_millis(&self) -> Result<i64, RuntimeClockError> {
        assert!(self.preparation_finished.load(Ordering::SeqCst));
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err(RuntimeClockError::Failed(
                "injected admission clock failure",
            ))
        } else {
            Ok(self.unix_millis)
        }
    }
}

struct Fixture {
    session: WorkbenchSession,
    plan: WorkbenchRunPlan,
    grant: WorkbenchProcessStartGrant,
    input: WorkbenchProcessAdmissionInput,
    now: chrono::DateTime<Utc>,
}

fn fixture(seed: usize) -> Fixture {
    let workspace_digest = format!("sha256:{:064x}", seed + 1);
    let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
        workspace_digest: workspace_digest.clone(),
        task_class: "coding".into(),
    })
    .expect("create Workbench session");
    let adapter_plan_id = format!("codex-{seed:012x}");
    let process = process_run_spec_for(
        &session.session_id,
        &adapter_plan_id,
        "codex",
        &workspace_digest,
    )
    .expect("create process run spec");
    let plan = WorkbenchRunPlan {
        schema_version: 1,
        plan_id: format!("run-plan:{seed:032x}"),
        session_id: session.session_id.clone(),
        adapter_id: "codex".into(),
        workspace_digest: workspace_digest.clone(),
        context_pack_digest: None,
        router_decision: RouterDecisionReference {
            decision_id: format!("routing-decision-{seed}"),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: format!("sha256:{:064x}", seed + 10_000),
        },
        replay_reference: None,
        preset: None,
        requested_mode: SwitchboardMode::Off,
        adapter_plan_id: adapter_plan_id.clone(),
        adapter_action: "cleanup_managed_routing".into(),
        adapter_reversible: true,
        command_readiness: Some(WorkbenchAdapterCommandReadiness {
            schema_version: 1,
            adapter_id: "codex".into(),
            adapter_contract_version:
                crate::client_adapter_contract::CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
            adapter_plan_id,
            logical_binary: "codex".into(),
            known_candidate_present: false,
            discovery_mode: "fixed_known_location_metadata_only".into(),
            cli_version_probe_state: "not_probed".into(),
            version_probe_reason: "fake verified-routing command test".into(),
            process_start_enabled: false,
            provider_traffic: "none".into(),
            writes_enabled: false,
        }),
        process_containment: Some(process.clone()),
        capability_requests: vec![CapabilityRequest {
            capability_id: "adapter_command_readiness".into(),
            scope: "session".into(),
            approval_state: "pending".into(),
            execution_enabled: false,
        }],
        execution_mode: "plan_only".into(),
        provider_traffic: "none".into(),
        writes_enabled: false,
    };
    let now = Utc.with_ymd_and_hms(2026, 8, 24, 6, 0, 0).unwrap() + Duration::seconds(seed as i64);
    let grant = issue_process_start_grant(
        &session,
        &plan,
        &process_start_confirmation_phrase(&plan),
        now,
    )
    .expect("issue process grant");
    let input = WorkbenchProcessAdmissionInput {
        run_spec: WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest,
            context_pack_digest: None,
            router_decision_id: plan.router_decision.decision_id.clone(),
            replay_reference_id: None,
            preset_id: None,
            required_capability_ids: vec!["adapter_command_readiness".into()],
            requested_mode: SwitchboardMode::Off,
        },
        expected_plan_id: plan.plan_id.clone(),
        expected_process_run_id: process.run_id.clone(),
        grant_id: grant.grant_id.clone(),
    };
    Fixture {
        session,
        plan,
        grant,
        input,
        now,
    }
}

fn verification(verified: bool, proxy_reachable: bool) -> VerificationReport {
    VerificationReport {
        client_id: "codex".into(),
        verified,
        proxy_reachable,
        checks: vec!["deterministic fake verifier".into()],
        failures: if verified {
            Vec::new()
        } else {
            vec!["routing not verified".into()]
        },
    }
}

fn persist_grant(store: &WorkbenchProcessGrantStore, fixture: &Fixture) {
    store
        .issue(fixture.grant.clone(), fixture.now)
        .expect("persist process grant");
}

fn admit_with_fake_verifier(
    fixture: &Fixture,
    grant_store: &WorkbenchProcessGrantStore,
    admission_store: &WorkbenchProcessAdmissionStore,
    report: Result<VerificationReport, String>,
    verifier_called: &Cell<usize>,
) -> Result<WorkbenchProcessAdmission, String> {
    let plan = fixture.plan.clone();
    admit_workbench_process_with_dependencies(
        &fixture.session,
        fixture.input.clone(),
        move |session, run_spec| {
            assert_eq!(run_spec.session_id, session.session_id);
            Ok(plan)
        },
        || fixture.now,
        |grant_id, session_id, plan_id, process_run_id, now| {
            grant_store
                .require_active_for(grant_id, session_id, plan_id, process_run_id, now)
                .map_err(|error| error.to_string())
        },
        |_| {
            verifier_called.set(verifier_called.get() + 1);
            report
        },
        |admission| {
            admission_store
                .issue(admission)
                .map_err(|error| error.to_string())
        },
    )
}

#[test]
fn verified_fake_routing_persists_one_idempotent_non_executing_admission() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(1);
    let grant_store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
    let admission_store =
        WorkbenchProcessAdmissionStore::at(directory.path().join("admissions.json"));
    persist_grant(&grant_store, &fixture);
    let verifier_called = Cell::new(0);

    let first = admit_with_fake_verifier(
        &fixture,
        &grant_store,
        &admission_store,
        Ok(verification(true, false)),
        &verifier_called,
    )
    .expect("admit with verified fake routing");
    let repeated = admit_with_fake_verifier(
        &fixture,
        &grant_store,
        &admission_store,
        Ok(verification(true, false)),
        &verifier_called,
    )
    .expect("idempotent repeated admission");

    assert_eq!(first, repeated);
    assert_eq!(verifier_called.get(), 2);
    assert_eq!(first.state, "authorized_not_started");
    assert!(!first.execution_enabled);
    assert_eq!(first.provider_traffic, "none");
    assert!(!first.writes_enabled);
    assert_eq!(
        admission_store
            .list_for_session(&fixture.session.session_id)
            .expect("list admissions"),
        vec![first]
    );
}

#[test]
fn runtime_clock_admission_samples_once_after_preparation_and_reuses_timestamp() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(40);
    let grant_store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
    let admission_store =
        WorkbenchProcessAdmissionStore::at(directory.path().join("admissions.json"));
    persist_grant(&grant_store, &fixture);
    let preparation_finished = AtomicBool::new(false);
    let clock_calls = AtomicUsize::new(0);
    let clock = AdmissionClock {
        unix_millis: fixture.now.timestamp_millis(),
        calls: &clock_calls,
        preparation_finished: &preparation_finished,
        fail: false,
    };
    let verifier_called = Cell::new(0);
    let plan = fixture.plan.clone();

    let admission = admit_workbench_process_with_clock(
        &clock,
        &fixture.session,
        fixture.input.clone(),
        |_, _| {
            preparation_finished.store(true, Ordering::SeqCst);
            Ok(plan)
        },
        |grant_id, session_id, plan_id, process_run_id, now| {
            assert_eq!(now, fixture.now);
            grant_store
                .require_active_for(grant_id, session_id, plan_id, process_run_id, now)
                .map_err(|error| error.to_string())
        },
        |_| {
            verifier_called.set(verifier_called.get() + 1);
            Ok(verification(true, false))
        },
        |admission| {
            admission_store
                .issue(admission)
                .map_err(|error| error.to_string())
        },
    )
    .expect("admit with runtime clock");

    assert_eq!(clock_calls.load(Ordering::SeqCst), 1);
    assert_eq!(verifier_called.get(), 1);
    assert_eq!(admission.admitted_at, fixture.now.to_rfc3339());
    assert_eq!(
        admission_store
            .list_for_session(&fixture.session.session_id)
            .expect("load persisted admission"),
        vec![admission]
    );
}

#[test]
fn runtime_clock_failure_preserves_ledgers_and_skips_grant_verifier_and_admission() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(41);
    let grant_path = directory.path().join("grants.json");
    let admission_path = directory.path().join("admissions.json");
    let grant_store = WorkbenchProcessGrantStore::at(grant_path.clone());
    let admission_store = WorkbenchProcessAdmissionStore::at(admission_path.clone());
    persist_grant(&grant_store, &fixture);
    let grant_bytes = std::fs::read(&grant_path).expect("read seeded grant ledger");
    let preparation_finished = AtomicBool::new(false);
    let clock_calls = AtomicUsize::new(0);
    let clock = AdmissionClock {
        unix_millis: fixture.now.timestamp_millis(),
        calls: &clock_calls,
        preparation_finished: &preparation_finished,
        fail: true,
    };
    let grant_called = Cell::new(0);
    let verifier_called = Cell::new(0);
    let persist_called = Cell::new(0);
    let plan = fixture.plan.clone();

    let error = admit_workbench_process_with_clock(
        &clock,
        &fixture.session,
        fixture.input.clone(),
        |_, _| {
            preparation_finished.store(true, Ordering::SeqCst);
            Ok(plan)
        },
        |grant_id, session_id, plan_id, process_run_id, now| {
            grant_called.set(grant_called.get() + 1);
            grant_store
                .require_active_for(grant_id, session_id, plan_id, process_run_id, now)
                .map_err(|error| error.to_string())
        },
        |_| {
            verifier_called.set(verifier_called.get() + 1);
            Ok(verification(true, false))
        },
        |admission| {
            persist_called.set(persist_called.get() + 1);
            admission_store
                .issue(admission)
                .map_err(|error| error.to_string())
        },
    )
    .expect_err("runtime clock failure must deny admission");

    assert!(error.contains("injected admission clock failure"));
    assert!(preparation_finished.load(Ordering::SeqCst));
    assert_eq!(clock_calls.load(Ordering::SeqCst), 1);
    assert_eq!(grant_called.get(), 0);
    assert_eq!(verifier_called.get(), 0);
    assert_eq!(persist_called.get(), 0);
    assert_eq!(
        std::fs::read(&grant_path).expect("read grant ledger after clock failure"),
        grant_bytes
    );
    assert!(!admission_path.exists());
}

#[test]
fn unverified_fake_routing_denies_without_persisting() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(2);
    let grant_store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
    let admission_store =
        WorkbenchProcessAdmissionStore::at(directory.path().join("admissions.json"));
    persist_grant(&grant_store, &fixture);
    let verifier_called = Cell::new(0);

    let error = admit_with_fake_verifier(
        &fixture,
        &grant_store,
        &admission_store,
        Ok(verification(false, true)),
        &verifier_called,
    )
    .expect_err("unverified routing must fail closed");

    assert!(error.contains("verified existing Codex routing"));
    assert_eq!(verifier_called.get(), 1);
    assert!(admission_store
        .list_for_session(&fixture.session.session_id)
        .expect("list admissions")
        .is_empty());
}

#[test]
fn verifier_error_propagates_without_persisting() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(3);
    let grant_store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
    let admission_store =
        WorkbenchProcessAdmissionStore::at(directory.path().join("admissions.json"));
    persist_grant(&grant_store, &fixture);
    let verifier_called = Cell::new(0);

    let error = admit_with_fake_verifier(
        &fixture,
        &grant_store,
        &admission_store,
        Err("fake verifier unavailable".into()),
        &verifier_called,
    )
    .expect_err("verifier error must propagate");

    assert_eq!(error, "fake verifier unavailable");
    assert_eq!(verifier_called.get(), 1);
    assert!(admission_store
        .list_for_session(&fixture.session.session_id)
        .expect("list admissions")
        .is_empty());
}

#[test]
fn grant_clock_is_evaluated_after_preparation_and_denies_exact_expiry() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(30);
    let grant_store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
    let admission_store =
        WorkbenchProcessAdmissionStore::at(directory.path().join("admissions.json"));
    persist_grant(&grant_store, &fixture);
    let preparation_finished = Cell::new(false);
    let verifier_called = Cell::new(false);

    let result = admit_workbench_process_with_dependencies(
        &fixture.session,
        fixture.input.clone(),
        |_, _| {
            preparation_finished.set(true);
            Ok(fixture.plan.clone())
        },
        || {
            assert!(preparation_finished.get());
            fixture.now + Duration::minutes(15)
        },
        |grant_id, session_id, plan_id, process_run_id, now| {
            grant_store
                .require_active_for(grant_id, session_id, plan_id, process_run_id, now)
                .map_err(|error| error.to_string())
        },
        |_| {
            verifier_called.set(true);
            Ok(verification(true, true))
        },
        |admission| {
            admission_store
                .issue(admission)
                .map_err(|error| error.to_string())
        },
    );

    assert!(result
        .expect_err("grant must be expired at the exact boundary")
        .contains("not active"));
    assert!(!verifier_called.get());
    assert!(admission_store
        .list_for_session(&fixture.session.session_id)
        .expect("list admissions")
        .is_empty());
}

#[test]
fn invalid_session_plan_and_grant_fail_before_verification() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(4);
    let grant_store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
    let admission_store =
        WorkbenchProcessAdmissionStore::at(directory.path().join("admissions.json"));
    persist_grant(&grant_store, &fixture);

    let mut paused = fixture.session.clone();
    paused
        .transition(WorkbenchSessionAction::Pause)
        .expect("pause session");
    let prepare_called = Cell::new(false);
    let verifier_called = Cell::new(false);
    let result = admit_workbench_process_with_dependencies(
        &paused,
        fixture.input.clone(),
        |_, _| {
            prepare_called.set(true);
            Ok(fixture.plan.clone())
        },
        || fixture.now,
        |_, _, _, _, _| Ok(fixture.grant.clone()),
        |_| {
            verifier_called.set(true);
            Ok(verification(true, true))
        },
        |admission| Ok(admission),
    );
    assert!(result.is_err());
    assert!(!prepare_called.get());
    assert!(!verifier_called.get());

    let mut drifted_input = fixture.input.clone();
    drifted_input.expected_plan_id = "run-plan:changed".into();
    let verifier_called = Cell::new(false);
    let result = admit_workbench_process_with_dependencies(
        &fixture.session,
        drifted_input,
        |_, _| Ok(fixture.plan.clone()),
        || fixture.now,
        |_, _, _, _, _| Ok(fixture.grant.clone()),
        |_| {
            verifier_called.set(true);
            Ok(verification(true, true))
        },
        |admission| Ok(admission),
    );
    assert!(result.is_err());
    assert!(!verifier_called.get());

    let verifier_called = Cell::new(false);
    let result = admit_workbench_process_with_dependencies(
        &fixture.session,
        fixture.input.clone(),
        |_, _| Ok(fixture.plan.clone()),
        || fixture.now,
        |_, _, _, _, _| Err("fake grant unavailable".into()),
        |_| {
            verifier_called.set(true);
            Ok(verification(true, true))
        },
        |admission| Ok(admission),
    );
    assert_eq!(
        result.expect_err("grant failure must propagate"),
        "fake grant unavailable"
    );
    assert!(!verifier_called.get());
    assert!(admission_store
        .list_for_session(&fixture.session.session_id)
        .expect("list admissions")
        .is_empty());
}
