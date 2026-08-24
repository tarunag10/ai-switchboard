use std::cell::Cell;
use std::fs;

use crate::models::SwitchboardMode;

use super::capability_grant::WorkbenchProcessGrantStore;
use super::events::WorkbenchSessionAction;
use super::run_contract::prepare_run_plan_with_reference;
use super::run_plan_publication::prepare_and_publish_workbench_run_plan;
use super::session::CreateWorkbenchSessionInput;
use super::storage::run_plan_head::{WorkbenchPlanHead, WorkbenchPlanHeadStore};
use super::storage::WorkbenchStore;
use super::{RouterDecisionReference, WorkbenchRunPlan, WorkbenchRunSpecInput, WorkbenchSession};

const PLAN_HEAD_LEDGER_FILE: &str = "workbench-current-plan-heads.json";

struct Fixture {
    directory: tempfile::TempDir,
    session_store: WorkbenchStore,
    grant_store: WorkbenchProcessGrantStore,
    plan_head_store: WorkbenchPlanHeadStore,
    session: WorkbenchSession,
    input: WorkbenchRunSpecInput,
}

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().expect("temporary authority directory");
    let session_store = WorkbenchStore::at(directory.path().join("workbench-sessions.json"));
    let session = session_store
        .create(CreateWorkbenchSessionInput {
            workspace_digest: digest('a'),
            task_class: "coding".into(),
        })
        .expect("create Workbench session");
    let grant_store =
        WorkbenchProcessGrantStore::at(directory.path().join("workbench-process-grants.json"));
    let plan_head_store = WorkbenchPlanHeadStore::for_authority_directory(directory.path());
    let input = WorkbenchRunSpecInput {
        session_id: session.session_id.clone(),
        adapter_id: "codex".into(),
        workspace_digest: session.workspace_digest.clone(),
        context_pack_digest: Some(digest('b')),
        router_decision_id: "routing-decision-publication".into(),
        replay_reference_id: None,
        preset_id: None,
        required_capability_ids: vec![
            "repo_context".into(),
            "router_observe".into(),
            "client_adapter_plan".into(),
        ],
        requested_mode: SwitchboardMode::Headroom,
    };
    Fixture {
        directory,
        session_store,
        grant_store,
        plan_head_store,
        session,
        input,
    }
}

fn publish(value: &Fixture, evidence_character: char) -> Result<WorkbenchRunPlan, String> {
    let router = RouterDecisionReference {
        decision_id: value.input.router_decision_id.clone(),
        decision_stage: "observe".into(),
        routing_mode: "observe_only".into(),
        evidence_digest: digest(evidence_character),
    };
    prepare_and_publish_workbench_run_plan(
        &value.session_store,
        &value.grant_store,
        &value.plan_head_store,
        value.input.clone(),
        move |session, input| {
            prepare_run_plan_with_reference(session, input, router, None, None)
                .map_err(|error| error.to_string())
        },
    )
}

fn current_head(value: &Fixture, plan: &WorkbenchRunPlan) -> WorkbenchPlanHead {
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("current-plan lookup transaction");
    value
        .plan_head_store
        .require_current_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            plan,
        )
        .expect("durable current plan head")
}

#[test]
fn production_publication_is_idempotent_and_remains_plan_only() {
    let value = fixture();
    let first = publish(&value, 'c').expect("publish first production plan");
    let first_head = current_head(&value, &first);
    let repeated = publish(&value, 'c').expect("repeat identical production plan");
    let repeated_head = current_head(&value, &repeated);

    assert_eq!(first, repeated);
    assert_eq!(first_head, repeated_head);
    assert_eq!(first_head.plan_id, first.plan_id);
    assert_eq!(first_head.session_id, value.session.session_id);
    assert!(!first_head.execution_enabled);
    assert_eq!(first_head.provider_traffic, "none");
    assert!(!first_head.writes_enabled);
    assert_eq!(first.execution_mode, "plan_only");
    assert_eq!(first.provider_traffic, "none");
    assert!(!first.writes_enabled);

    let public_payload = serde_json::to_value(&first).expect("serialize public plan payload");
    let object = public_payload
        .as_object()
        .expect("public plan payload remains an object");
    for private_field in [
        "headId",
        "headGeneration",
        "recordDigest",
        "planSnapshotJson",
    ] {
        assert!(
            !object.contains_key(private_field),
            "private current-plan field {private_field} crossed the command boundary"
        );
    }
}

#[test]
fn a_changed_production_plan_supersedes_the_returned_plan() {
    let value = fixture();
    let first = publish(&value, 'c').expect("publish first production plan");
    let first_head = current_head(&value, &first);
    let changed = publish(&value, 'd').expect("publish changed production plan");
    let changed_head = current_head(&value, &changed);

    assert_ne!(first.plan_id, changed.plan_id);
    assert_ne!(first_head.head_id, changed_head.head_id);
    assert!(value
        .plan_head_store
        .require_current_for_authority_transaction(
            &value
                .grant_store
                .begin_authority_transaction()
                .expect("stale lookup transaction"),
            &value.session_store,
            &value.session,
            &first,
        )
        .is_err());
}

#[test]
fn cross_directory_publication_fails_before_plan_preparation() {
    let value = fixture();
    let other = tempfile::tempdir().expect("other authority directory");
    let other_grant_store =
        WorkbenchProcessGrantStore::at(other.path().join("workbench-process-grants.json"));
    let other_plan_head_store = WorkbenchPlanHeadStore::for_authority_directory(other.path());
    let prepare_called = Cell::new(false);

    let result = prepare_and_publish_workbench_run_plan(
        &value.session_store,
        &other_grant_store,
        &other_plan_head_store,
        value.input.clone(),
        |_, _| {
            prepare_called.set(true);
            Err("plan preparation must not run".into())
        },
    );

    assert!(result
        .expect_err("cross-directory publication must fail")
        .contains("another storage directory"));
    assert!(!prepare_called.get());
    assert!(!other.path().join(PLAN_HEAD_LEDGER_FILE).exists());
}

#[test]
fn corrupt_plan_head_bytes_are_preserved_and_no_plan_is_returned() {
    let value = fixture();
    publish(&value, 'c').expect("publish initial plan");
    let path = value.directory.path().join(PLAN_HEAD_LEDGER_FILE);
    let corrupt = b"{\"schemaVersion\":1,\"forbidden\":true}";
    fs::write(&path, corrupt).expect("replace ledger with corrupt bytes");

    assert!(publish(&value, 'd').is_err());
    assert_eq!(fs::read(path).expect("preserved corrupt bytes"), corrupt);
}

#[test]
fn unavailable_plan_head_target_returns_no_plan_and_preserves_the_target() {
    let value = fixture();
    let path = value.directory.path().join(PLAN_HEAD_LEDGER_FILE);
    fs::create_dir(&path).expect("occupy current-plan target with a directory");

    assert!(publish(&value, 'c').is_err());
    assert!(path.is_dir());
}

#[test]
fn paused_session_rejects_publication_without_replacing_the_old_head() {
    let value = fixture();
    publish(&value, 'c').expect("publish initial plan");
    let path = value.directory.path().join(PLAN_HEAD_LEDGER_FILE);
    let before = fs::read(&path).expect("initial current-plan bytes");
    value
        .session_store
        .transition(&value.session.session_id, WorkbenchSessionAction::Pause)
        .expect("pause durable session");

    assert!(publish(&value, 'c').is_err());
    assert_eq!(
        fs::read(path).expect("preserved current-plan bytes"),
        before
    );
}
