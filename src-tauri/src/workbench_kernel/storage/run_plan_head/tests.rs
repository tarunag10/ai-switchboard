use std::fs;

use super::{
    plan_head_id_for, plan_head_record_digest, plan_head_tombstone_digest, WorkbenchPlanHeadStore,
    MAX_PLAN_HEAD_LEDGER_BYTES,
};
use crate::workbench_kernel::codex_restricted_helper_preparation_tests::fixture;
use crate::workbench_kernel::events::WorkbenchSessionAction;
use crate::workbench_kernel::run_contract::workbench_run_plan_identity;

fn changed_plan(
    value: &crate::workbench_kernel::codex_restricted_helper_preparation_tests::Fixture,
) -> crate::workbench_kernel::WorkbenchRunPlan {
    let mut plan = value.plan.clone();
    plan.router_decision.evidence_digest = format!("sha256:{}", "9".repeat(64));
    plan.plan_id = workbench_run_plan_identity(&plan).expect("changed plan identity");
    plan
}

#[test]
fn publish_is_idempotent_and_a_b_a_supersession_has_unique_heads() {
    let value = fixture();
    let store = WorkbenchPlanHeadStore::at(value.directory.path().join("test-plan-heads.json"));
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head transaction");
    let first = store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("publish first head");
    let repeated = store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("repeat first head");
    assert_eq!(first, repeated);
    let second_plan = changed_plan(&value);
    let second = store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &second_plan,
        )
        .expect("supersede with second head");
    let third = store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("return to original plan as new head");
    assert_eq!(
        (first.generation, second.generation, third.generation),
        (1, 2, 3)
    );
    assert_ne!(first.head_id, second.head_id);
    assert_ne!(first.head_id, third.head_id);
    assert_eq!(
        third.predecessor_head_id.as_deref(),
        Some(second.head_id.as_str())
    );
    assert!(store
        .require_current_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &second_plan,
        )
        .is_err());
    assert_eq!(
        store
            .require_current_for_authority_transaction(
                &transaction,
                &value.session_store,
                &value.session,
                &value.plan,
            )
            .expect("require third head"),
        third
    );
}

#[test]
fn missing_head_fails_closed_without_creating_storage() {
    let value = fixture();
    let path = value.directory.path().join("missing-plan-heads.json");
    let store = WorkbenchPlanHeadStore::at(path.clone());
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head transaction");
    assert!(store
        .require_current_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .is_err());
    assert!(!path.exists());
}

#[test]
fn session_mutation_makes_the_old_head_inert_after_pause_and_resume() {
    let value = fixture();
    let path = value.directory.path().join("session-bound-plan-heads.json");
    let store = WorkbenchPlanHeadStore::at(path);
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("publish transaction");
    let first = store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("publish first head");
    drop(transaction);
    value
        .session_store
        .transition(&value.session.session_id, WorkbenchSessionAction::Pause)
        .expect("pause session");
    let paused = value
        .session_store
        .get(&value.session.session_id)
        .expect("paused session");
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("paused transaction");
    assert!(store
        .require_current_for_authority_transaction(
            &transaction,
            &value.session_store,
            &paused,
            &value.plan,
        )
        .is_err());
    drop(transaction);
    value
        .session_store
        .transition(&value.session.session_id, WorkbenchSessionAction::Resume)
        .expect("resume session");
    let resumed = value
        .session_store
        .get(&value.session.session_id)
        .expect("resumed session");
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("resumed transaction");
    assert!(store
        .require_current_for_authority_transaction(
            &transaction,
            &value.session_store,
            &resumed,
            &value.plan,
        )
        .is_err());
    let republished = store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &resumed,
            &value.plan,
        )
        .expect("republish after resume");
    assert_ne!(first.head_id, republished.head_id);
}

#[test]
fn transaction_from_another_directory_is_rejected_without_writes() {
    let value = fixture();
    let other = tempfile::tempdir().expect("other plan-head directory");
    let path = other.path().join("plan-heads.json");
    let store = WorkbenchPlanHeadStore::at(path.clone());
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("foreign transaction");
    assert!(store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .is_err());
    assert!(!path.exists());
}

#[test]
fn unknown_or_corrupt_ledger_is_rejected_and_preserved() {
    let value = fixture();
    let path = value.directory.path().join("tampered-plan-heads.json");
    let store = WorkbenchPlanHeadStore::at(path.clone());
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head transaction");
    store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("publish head");
    drop(transaction);
    let original = fs::read(&path).expect("plan-head bytes");
    let mut document: serde_json::Value =
        serde_json::from_slice(&original).expect("decode plan-head ledger");
    document.as_object_mut().expect("plan-head object").insert(
        "workspacePath".into(),
        serde_json::Value::String("forbidden".into()),
    );
    let unknown = serde_json::to_vec_pretty(&document).expect("encode unknown ledger");
    fs::write(&path, &unknown).expect("write unknown ledger");
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("unknown transaction");
    assert!(store
        .require_current_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .is_err());
    assert_eq!(fs::read(&path).expect("preserve unknown ledger"), unknown);
    drop(transaction);
    fs::write(&path, b"{not json").expect("write corrupt ledger");
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("corrupt transaction");
    assert!(store
        .require_current_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .is_err());
    assert_eq!(
        fs::read(&path).expect("preserve corrupt ledger"),
        b"{not json"
    );
}

#[test]
fn stale_complete_byte_cas_preserves_external_replacement() {
    let value = fixture();
    let path = value.directory.path().join("cas-plan-heads.json");
    let store = WorkbenchPlanHeadStore::at(path.clone());
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head transaction");
    store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("publish head");
    let (ledger, expected) = store.load().expect("load expected ledger");
    let mut replacement = expected.expect("persisted ledger bytes");
    replacement.push(b'\n');
    fs::write(&path, &replacement).expect("replace ledger bytes");
    assert!(store
        .save(&ledger, Some(&replacement[..replacement.len() - 1]))
        .is_err());
    assert_eq!(fs::read(&path).expect("preserve replacement"), replacement);
}

#[test]
fn self_consistent_one_way_tombstone_lineage_is_rejected() {
    let value = fixture();
    let path = value.directory.path().join("lineage-plan-heads.json");
    let store = WorkbenchPlanHeadStore::at(path.clone());
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head transaction");
    store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("publish first head");
    let second_plan = changed_plan(&value);
    store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &second_plan,
        )
        .expect("publish second head");
    let (mut ledger, _) = store.load().expect("load lineage ledger");
    let ledger_id = ledger.ledger_id.clone();
    let new_head_id = {
        let head = ledger
            .current_heads
            .get_mut(&value.session.session_id)
            .expect("current lineage head");
        head.predecessor_head_id = None;
        head.predecessor_record_digest = None;
        head.head_id = plan_head_id_for(
            &ledger_id,
            head.generation,
            &head.session_id,
            &head.session_snapshot_digest,
            &head.plan_snapshot_digest,
            None,
            None,
        );
        head.record_digest = plan_head_record_digest(head).expect("refresh head digest");
        head.head_id.clone()
    };
    let tombstone = ledger
        .retired_heads
        .values_mut()
        .next()
        .expect("retired predecessor");
    tombstone.superseded_by_head_id = new_head_id;
    tombstone.tombstone_digest =
        plan_head_tombstone_digest(tombstone).expect("refresh tombstone digest");
    ledger.refresh_digest().expect("refresh ledger digest");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&ledger).expect("encode one-way lineage"),
    )
    .expect("write one-way lineage");
    let tampered = fs::read(&path).expect("tampered lineage bytes");
    assert!(store.load().is_err());
    assert_eq!(fs::read(path).expect("preserve tampered lineage"), tampered);
}

#[test]
fn oversized_ledger_is_rejected_before_read_allocation() {
    let value = fixture();
    let path = value.directory.path().join("oversized-plan-heads.json");
    let file = fs::File::create(&path).expect("create oversized ledger");
    file.set_len((MAX_PLAN_HEAD_LEDGER_BYTES + 1) as u64)
        .expect("size oversized ledger");
    let store = WorkbenchPlanHeadStore::at(path);
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head transaction");
    assert!(store
        .require_current_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_plan_head_ledger_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;

    let value = fixture();
    let path = value.directory.path().join("symlink-plan-heads.json");
    let target = value.directory.path().join("plan-head-target.json");
    fs::write(&target, b"target-sentinel").expect("write target");
    symlink(&target, &path).expect("symlink plan-head ledger");
    let store = WorkbenchPlanHeadStore::at(path);
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head transaction");
    assert!(store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .is_err());
    assert_eq!(
        fs::read(target).expect("preserve target"),
        b"target-sentinel"
    );
}

#[test]
fn plan_head_snapshot_contains_no_forbidden_runtime_fields() {
    let value = fixture();
    let path = value.directory.path().join("pure-plan-heads.json");
    let store = WorkbenchPlanHeadStore::at(path.clone());
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head transaction");
    store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("publish head");
    let raw = String::from_utf8(fs::read(path).expect("plan-head bytes")).expect("utf8 ledger");
    for forbidden in [
        "workspacePath",
        "prompt",
        "credential",
        "environment",
        "argv",
        "processId",
        "transport",
    ] {
        assert!(
            !raw.contains(&format!("\"{forbidden}\""))
                && !raw.contains(&format!("\\\"{forbidden}\\\"")),
            "plan-head ledger contains a forbidden {forbidden} field"
        );
    }
    assert!(raw.contains("piped_bounded_redacted"));
}
