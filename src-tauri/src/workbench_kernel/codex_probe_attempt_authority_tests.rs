use std::fs;
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::time::Duration as StdDuration;

use chrono::Duration;

use super::capability_grant::WorkbenchProcessGrantStore;
use super::codex_probe_attempt_authority::{
    codex_probe_attempt_confirmation_phrase, downgrade_codex_probe_authority_ledger_to_v1_for_test,
    rewrite_codex_probe_reservation_with_self_consistent_tamper_for_test,
    CodexProbeAttemptAuthorityStore, CodexProbeAttemptContext, CodexProbeAttemptState,
    MAX_CODEX_PROBE_ATTEMPT_AUTHORITIES,
};
use super::codex_probe_preflight_test_support::{digest, evaluate_npm, npm_receipt};
use super::codex_restricted_helper_preparation::{
    CodexHelperLaunchPreparationReceipt, CodexHelperLaunchRequest,
};
use super::codex_restricted_helper_preparation_tests::{fixture, prepare, prepare_with, Fixture};
use super::events::WorkbenchSessionAction;
use super::process_supervisor::WorkbenchProcessAdmissionStore;
use super::run_contract::workbench_run_plan_identity;
use super::session::CreateWorkbenchSessionInput;
use super::storage::run_plan_head::WorkbenchPlanHeadStore;
use super::storage::WorkbenchStore;

fn context<'a>(
    value: &'a Fixture,
    request: &'a CodexHelperLaunchRequest,
    receipt: &'a CodexHelperLaunchPreparationReceipt,
) -> CodexProbeAttemptContext<'a> {
    CodexProbeAttemptContext {
        session: &value.session,
        session_store: &value.session_store,
        plan_head_store: &value.plan_head_store,
        current_plan: &value.plan,
        process: &value.process,
        grant_store: &value.grant_store,
        admission_store: &value.admission_store,
        request,
        preparation_receipt: receipt,
    }
}

fn authority_path(value: &Fixture) -> std::path::PathBuf {
    value.directory.path().join("attempt-authorities.json")
}

#[test]
fn exact_preparation_issues_one_idempotent_content_free_authority() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open authority store");
    let phrase = codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id);
    assert!(store
        .issue(
            context(&value, &request, &receipt),
            "AUTHORIZE CODEX VERSION PROBE wrong-attempt",
            value.now,
        )
        .is_err());
    assert!(!path.exists());
    let first = store
        .issue(context(&value, &request, &receipt), &phrase, value.now)
        .expect("issue attempt authority");
    let first_bytes = fs::read(&path).expect("read authority ledger");
    let second = store
        .issue(context(&value, &request, &receipt), &phrase, value.now)
        .expect("repeat identical issue");
    assert_eq!(first, second);
    assert_eq!(fs::read(&path).expect("reread ledger"), first_bytes);
    assert_eq!(first.state, CodexProbeAttemptState::AvailableNoProcess);
    assert_eq!(first.revision, 0);
    assert!(first.claim_id.is_none());
    assert!(first.pre_reservation_record_digest.is_none());
    assert!(first.reservation_id.is_none());
    assert!(first.reservation_binding_digest.is_none());
    assert!(first.reservation_expires_at.is_none());
    assert!(first.manual_opt_in_confirmed);
    assert!(!first.helper_invoked);
    assert!(!first.process_started);
    assert!(!first.process_start_enabled);
    assert!(!first.launch_reserved);
    assert!(!first.execution_reserved);
    assert!(!first.execution_enabled);
    assert!(!first.runnable);
    assert!(!first.supported);
    assert!(!first.user_workspace_writes_enabled);
    assert_eq!(first.provider_traffic, "none");
    assert_forbidden_keys_absent(&serde_json::to_value(first).expect("serialize authority"));

    let mut changed_containment = value.containment.clone();
    changed_containment.helper_code_identity_digest = digest('x');
    let changed_preflight = evaluate_npm(&value.process, &npm_receipt(), &changed_containment)
        .expect("evaluate conflicting duplicate attempt");
    let (changed_request, changed_receipt) =
        prepare_with(&value, &changed_preflight, &changed_containment)
            .expect("prepare conflicting duplicate attempt");
    assert_eq!(
        changed_request.binding.attempt_id,
        request.binding.attempt_id
    );
    assert!(store
        .issue(
            context(&value, &changed_request, &changed_receipt),
            &codex_probe_attempt_confirmation_phrase(&changed_request.binding.attempt_id),
            value.now,
        )
        .is_err());
    assert_eq!(fs::read(path).expect("duplicate preserved"), first_bytes);
}

#[test]
fn issue_before_initialization_anchor_preserves_absent_storage() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let anchor_path = path.with_extension("anchor.json");
    let mut store = CodexProbeAttemptAuthorityStore::open(
        path.clone(),
        "owner-1",
        value.now + Duration::seconds(10),
    )
    .expect("open future-initialized authority store");

    assert!(store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .is_err());
    assert!(!path.exists());
    assert!(!anchor_path.exists());
}

#[test]
fn claim_atomically_creates_one_terminal_no_process_launch_reservation() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let mut store =
        CodexProbeAttemptAuthorityStore::open(authority_path(&value), "owner-1", value.now)
            .expect("open authority store");
    let issued = store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue attempt authority");
    let claimed = store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .expect("claim attempt authority");
    assert_eq!(claimed.state, CodexProbeAttemptState::ReservedNoProcess);
    assert_eq!(claimed.revision, 1);
    assert!(claimed.transition_at.is_some());
    assert!(claimed.claim_id.is_some());
    assert_eq!(
        claimed.pre_reservation_record_digest,
        Some(issued.record_digest.clone())
    );
    assert!(claimed.reservation_id.is_some());
    assert!(claimed.reservation_binding_digest.is_some());
    assert_eq!(
        claimed.reservation_expires_at,
        Some((value.now + Duration::seconds(11)).to_rfc3339())
    );
    assert!(claimed.launch_reserved);
    assert!(!claimed.execution_reserved);
    assert!(!claimed.helper_invoked);
    assert!(!claimed.process_started);
    assert!(!claimed.process_start_enabled);
    assert!(!claimed.execution_enabled);
    assert!(!claimed.runnable);
    assert!(!claimed.supported);
    assert!(!claimed.user_workspace_writes_enabled);
    assert_eq!(claimed.provider_traffic, "none");
    assert_forbidden_keys_absent(&serde_json::to_value(&claimed).expect("serialize reservation"));
    assert!(store
        .claim(
            &issued.authority_id,
            &claimed.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(11),
        )
        .is_err());
    assert_eq!(
        store
            .get(&issued.authority_id)
            .expect("reload claimed authority"),
        claimed
    );
}

#[test]
fn parent_grant_expiry_caps_the_attempt_window_and_is_exclusive() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let issue_time = value.now + Duration::seconds(15 * 60 - 1);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", issue_time)
        .expect("open authority store");
    let authority = store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            issue_time,
        )
        .expect("issue grant-capped authority");
    assert_eq!(authority.expires_at, value.grant.expires_at);
    let before = fs::read(&path).expect("authority bytes");
    assert!(store
        .claim(
            &authority.authority_id,
            &authority.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(15 * 60),
        )
        .is_err());
    assert_eq!(fs::read(path).expect("grant expiry preserved"), before);
}

#[test]
fn launch_reservation_window_is_capped_by_the_parent_grant() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let issue_time = value.now + Duration::seconds(15 * 60 - 5);
    let mut store =
        CodexProbeAttemptAuthorityStore::open(authority_path(&value), "owner-1", issue_time)
            .expect("open authority store");
    let authority = store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            issue_time,
        )
        .expect("issue grant-capped authority");
    let reserved = store
        .claim(
            &authority.authority_id,
            &authority.record_digest,
            context(&value, &request, &receipt),
            issue_time + Duration::seconds(1),
        )
        .expect("reserve before parent expiry");
    assert_eq!(
        reserved.reservation_expires_at,
        Some(value.grant.expires_at)
    );
    assert_eq!(reserved.state, CodexProbeAttemptState::ReservedNoProcess);
    assert!(reserved.launch_reserved);
    assert!(!reserved.execution_reserved);
}

#[test]
fn legacy_available_ledger_migrates_in_place_before_reservation() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut initial = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open initial authority store");
    let issued = initial
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue available authority");
    drop(initial);
    downgrade_codex_probe_authority_ledger_to_v1_for_test(&path)
        .expect("create valid legacy ledger fixture");
    let legacy: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read legacy ledger"))
            .expect("parse legacy ledger");
    assert_eq!(legacy["schemaVersion"], 1);

    let mut migrated = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("migrate legacy authority store");
    let migrated_document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read migrated ledger"))
            .expect("parse migrated ledger");
    assert_eq!(migrated_document["schemaVersion"], 2);
    let available = migrated
        .get(&issued.authority_id)
        .expect("load migrated authority");
    assert_eq!(available.state, CodexProbeAttemptState::AvailableNoProcess);
    let reserved = migrated
        .claim(
            &available.authority_id,
            &available.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .expect("reserve migrated authority");
    assert_eq!(reserved.state, CodexProbeAttemptState::ReservedNoProcess);
    assert_eq!(
        reserved.pre_reservation_record_digest,
        Some(available.record_digest)
    );
}

#[test]
fn legacy_claim_is_migrated_to_a_non_reservable_terminal_record() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut initial = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open initial authority store");
    let issued = initial
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue authority");
    initial
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .expect("create reservation before downgrade");
    drop(initial);
    downgrade_codex_probe_authority_ledger_to_v1_for_test(&path)
        .expect("create claimed legacy ledger fixture");

    let mut migrated =
        CodexProbeAttemptAuthorityStore::open(path, "owner-2", value.now + Duration::seconds(2))
            .expect("migrate claimed legacy authority");
    let terminal = migrated
        .get(&issued.authority_id)
        .expect("load migrated terminal authority");
    assert_eq!(
        terminal.state,
        CodexProbeAttemptState::LegacyClaimedNoReservation
    );
    assert!(!terminal.launch_reserved);
    assert!(terminal.reservation_id.is_none());
    assert!(migrated
        .claim(
            &terminal.authority_id,
            &terminal.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(3),
        )
        .is_err());
}

#[test]
fn corrupted_legacy_ledger_fails_without_replacement() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut initial = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open initial authority store");
    initial
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue authority");
    drop(initial);
    downgrade_codex_probe_authority_ledger_to_v1_for_test(&path)
        .expect("create legacy ledger fixture");
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read legacy ledger"))
            .expect("parse legacy ledger");
    document["authorities"]
        .as_object_mut()
        .expect("authority map")
        .values_mut()
        .next()
        .expect("authority")
        .as_object_mut()
        .expect("authority object")
        .insert(
            "scope".into(),
            serde_json::Value::String("launch_enabled".into()),
        );
    let corrupted = serde_json::to_vec_pretty(&document).expect("serialize corrupt legacy ledger");
    fs::write(&path, &corrupted).expect("write corrupt legacy ledger");
    assert!(CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now).is_err());
    assert_eq!(
        fs::read(path).expect("preserve corrupt legacy bytes"),
        corrupted
    );
}

#[test]
fn stale_store_and_byte_only_drift_fail_without_overwrite() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut first = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open first store");
    let issued = first
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue attempt authority");
    let mut stale = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open stale peer");
    first
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .expect("winning claim");
    let winning_bytes = fs::read(&path).expect("winning ledger bytes");
    assert!(stale
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .is_err());
    assert_eq!(fs::read(&path).expect("preserved winner"), winning_bytes);

    let parsed: serde_json::Value =
        serde_json::from_slice(&winning_bytes).expect("parse authority ledger");
    let compact = serde_json::to_vec(&parsed).expect("reformat ledger");
    assert_ne!(compact, winning_bytes);
    fs::write(&path, &compact).expect("simulate byte-only drift");
    assert!(first.get(&issued.authority_id).is_err());
    assert_eq!(fs::read(&path).expect("preserve drift"), compact);
}

#[test]
fn anchored_high_water_rejects_older_valid_v2_and_v1_ledger_snapshots() {
    let current = fixture();
    let (request, receipt) = prepare(&current).expect("prepare current helper contract");
    let current_path = authority_path(&current);
    let mut current_store =
        CodexProbeAttemptAuthorityStore::open(current_path.clone(), "owner-1", current.now)
            .expect("open current authority store");
    let authority = current_store
        .issue(
            context(&current, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            current.now,
        )
        .expect("issue current authority");
    let available_v2 = fs::read(&current_path).expect("capture available v2 ledger");
    current_store
        .claim(
            &authority.authority_id,
            &authority.record_digest,
            context(&current, &request, &receipt),
            current.now + Duration::seconds(1),
        )
        .expect("reserve current authority");
    drop(current_store);
    fs::write(&current_path, &available_v2).expect("restore older valid v2 ledger");
    assert!(CodexProbeAttemptAuthorityStore::open(
        current_path.clone(),
        "owner-1",
        current.now + Duration::seconds(2),
    )
    .is_err());
    assert_eq!(
        fs::read(&current_path).expect("preserve rejected v2 rollback"),
        available_v2
    );

    let legacy = fixture();
    let (request, receipt) = prepare(&legacy).expect("prepare legacy helper contract");
    let legacy_path = authority_path(&legacy);
    let mut initial =
        CodexProbeAttemptAuthorityStore::open(legacy_path.clone(), "owner-1", legacy.now)
            .expect("open legacy fixture store");
    let legacy_authority = initial
        .issue(
            context(&legacy, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            legacy.now,
        )
        .expect("issue legacy fixture authority");
    drop(initial);
    downgrade_codex_probe_authority_ledger_to_v1_for_test(&legacy_path)
        .expect("downgrade valid legacy fixture");
    let available_v1 = fs::read(&legacy_path).expect("capture available v1 ledger");
    let mut migrated =
        CodexProbeAttemptAuthorityStore::open(legacy_path.clone(), "owner-1", legacy.now)
            .expect("migrate legacy fixture");
    let migrated_authority = migrated
        .get(&legacy_authority.authority_id)
        .expect("load migrated legacy authority");
    migrated
        .claim(
            &migrated_authority.authority_id,
            &migrated_authority.record_digest,
            context(&legacy, &request, &receipt),
            legacy.now + Duration::seconds(1),
        )
        .expect("reserve migrated legacy authority");
    drop(migrated);
    fs::write(&legacy_path, &available_v1).expect("restore older valid v1 ledger");
    assert!(CodexProbeAttemptAuthorityStore::open(
        legacy_path.clone(),
        "owner-1",
        legacy.now + Duration::seconds(2),
    )
    .is_err());
    assert_eq!(
        fs::read(legacy_path).expect("preserve rejected v1 rollback"),
        available_v1
    );
}

#[test]
fn concurrent_double_claim_creates_exactly_one_launch_reservation() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut issuer = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open issuing store");
    let authority = issuer
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue authority");
    drop(issuer);

    let barrier = Arc::new(Barrier::new(2));
    let mut claims = Vec::new();
    for _ in 0..2 {
        let authority_path = path.clone();
        let session_store_path = value.directory.path().join("workbench-sessions.json");
        let plan_head_path = value
            .directory
            .path()
            .join("workbench-current-plan-heads.json");
        let grant_path = value.directory.path().join("grants.json");
        let admission_path = value.directory.path().join("admissions.json");
        let session = value.session.clone();
        let plan = value.plan.clone();
        let process = value.process.clone();
        let request = request.clone();
        let receipt = receipt.clone();
        let authority_id = authority.authority_id.clone();
        let expected_record_digest = authority.record_digest.clone();
        let claim_at = value.now + Duration::seconds(1);
        let barrier = Arc::clone(&barrier);
        claims.push(thread::spawn(move || {
            let session_store = WorkbenchStore::at(session_store_path);
            let plan_head_store = WorkbenchPlanHeadStore::at(plan_head_path);
            let grant_store = WorkbenchProcessGrantStore::at(grant_path);
            let admission_store = WorkbenchProcessAdmissionStore::at(admission_path);
            let mut store =
                CodexProbeAttemptAuthorityStore::open(authority_path, "owner-1", claim_at)
                    .expect("open concurrent claim store");
            barrier.wait();
            store
                .claim(
                    &authority_id,
                    &expected_record_digest,
                    CodexProbeAttemptContext {
                        session: &session,
                        session_store: &session_store,
                        plan_head_store: &plan_head_store,
                        current_plan: &plan,
                        process: &process,
                        grant_store: &grant_store,
                        admission_store: &admission_store,
                        request: &request,
                        preparation_receipt: &receipt,
                    },
                    claim_at,
                )
                .is_ok()
        }));
    }
    let successes = claims
        .into_iter()
        .map(|claim| claim.join().expect("join concurrent claim"))
        .filter(|succeeded| *succeeded)
        .count();
    assert_eq!(successes, 1);
    let winner =
        CodexProbeAttemptAuthorityStore::open(path, "owner-1", value.now + Duration::seconds(2))
            .expect("reopen winning reservation")
            .get(&authority.authority_id)
            .expect("load winning reservation");
    assert_eq!(winner.state, CodexProbeAttemptState::ReservedNoProcess);
    assert!(winner.launch_reserved);
}

#[test]
fn deleted_or_symlinked_ledger_is_never_recreated_or_followed() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open authority store");
    let issued = store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue attempt authority");
    fs::remove_file(&path).expect("delete ledger");
    assert!(store.get(&issued.authority_id).is_err());
    assert!(!path.exists());

    #[cfg(unix)]
    {
        let target = value.directory.path().join("symlink-target.json");
        fs::write(&target, b"sentinel-target").expect("write symlink target");
        std::os::unix::fs::symlink(&target, &path).expect("replace with symlink");
        assert!(CodexProbeAttemptAuthorityStore::open(path, "owner-1", value.now).is_err());
        assert_eq!(
            fs::read(target).expect("preserved target"),
            b"sentinel-target"
        );
    }
}

#[test]
fn durable_anchor_prevents_claimed_or_abandoned_resurrection_after_ledger_deletion() {
    let claimed = fixture();
    let (request, receipt) = prepare(&claimed).expect("prepare claimed contract");
    let claimed_path = authority_path(&claimed);
    let claimed_anchor_path = claimed_path.with_extension("anchor.json");
    let mut claimed_store =
        CodexProbeAttemptAuthorityStore::open(claimed_path.clone(), "owner-1", claimed.now)
            .expect("open claimed store");
    let authority = claimed_store
        .issue(
            context(&claimed, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            claimed.now,
        )
        .expect("issue claimed authority");
    claimed_store
        .claim(
            &authority.authority_id,
            &authority.record_digest,
            context(&claimed, &request, &receipt),
            claimed.now + Duration::seconds(1),
        )
        .expect("claim authority");
    drop(claimed_store);
    assert!(claimed_anchor_path.exists());
    fs::remove_file(&claimed_path).expect("delete claimed ledger only");
    assert!(CodexProbeAttemptAuthorityStore::open(
        claimed_path,
        "owner-2",
        claimed.now + Duration::seconds(2),
    )
    .is_err());

    let abandoned = fixture();
    let (request, receipt) = prepare(&abandoned).expect("prepare abandoned contract");
    let abandoned_path = authority_path(&abandoned);
    let abandoned_anchor_path = abandoned_path.with_extension("anchor.json");
    let mut first =
        CodexProbeAttemptAuthorityStore::open(abandoned_path.clone(), "owner-1", abandoned.now)
            .expect("open abandoned store");
    first
        .issue(
            context(&abandoned, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            abandoned.now,
        )
        .expect("issue abandoned authority");
    drop(first);
    let changed = CodexProbeAttemptAuthorityStore::open(
        abandoned_path.clone(),
        "owner-2",
        abandoned.now + Duration::seconds(1),
    )
    .expect("abandon on restart");
    assert_eq!(changed.reconciled_abandoned_count(), 1);
    drop(changed);
    assert!(abandoned_anchor_path.exists());
    fs::remove_file(&abandoned_path).expect("delete abandoned ledger only");
    assert!(CodexProbeAttemptAuthorityStore::open(
        abandoned_path,
        "owner-3",
        abandoned.now + Duration::seconds(2),
    )
    .is_err());
}

#[test]
fn session_grant_mutation_and_claim_share_one_cross_process_transaction_lock() {
    let session_mutation = fixture();
    let session_store_path = session_mutation
        .directory
        .path()
        .join("workbench-sessions.json");
    let transaction = session_mutation
        .grant_store
        .begin_authority_transaction()
        .expect("hold session authority transaction");
    let session_id = session_mutation.session.session_id.clone();
    let (started_tx, started_rx) = mpsc::channel();
    let (finished_tx, finished_rx) = mpsc::channel();
    let transition = thread::spawn(move || {
        let store = WorkbenchStore::at(session_store_path);
        started_tx.send(()).expect("signal transition start");
        let result = store
            .transition(&session_id, WorkbenchSessionAction::Pause)
            .map(|_| ());
        finished_tx.send(result).expect("signal transition finish");
    });
    started_rx.recv().expect("transition started");
    assert!(finished_rx
        .recv_timeout(StdDuration::from_millis(100))
        .is_err());
    drop(transaction);
    finished_rx
        .recv_timeout(StdDuration::from_secs(2))
        .expect("transition unblocked")
        .expect("transition succeeded");
    transition.join().expect("join transition thread");

    let value = fixture();
    let grant_path = value.directory.path().join("grants.json");
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("hold authority transaction");
    let grant_id = value.grant.grant_id.clone();
    let revoke_at = value.now + Duration::seconds(1);
    let (started_tx, started_rx) = mpsc::channel();
    let (finished_tx, finished_rx) = mpsc::channel();
    let revoke = thread::spawn(move || {
        let store = WorkbenchProcessGrantStore::at(grant_path);
        started_tx.send(()).expect("signal revoke start");
        let result = store.revoke(&grant_id, revoke_at).map(|_| ());
        finished_tx.send(result).expect("signal revoke finish");
    });
    started_rx.recv().expect("revoke started");
    assert!(finished_rx
        .recv_timeout(StdDuration::from_millis(100))
        .is_err());
    drop(transaction);
    finished_rx
        .recv_timeout(StdDuration::from_secs(2))
        .expect("revoke unblocked")
        .expect("revoke succeeded");
    revoke.join().expect("join revoke thread");

    let claimable = fixture();
    let (request, receipt) = prepare(&claimable).expect("prepare claim transaction");
    let authority_path = authority_path(&claimable);
    let grant_path = claimable.directory.path().join("grants.json");
    let session_store_path = claimable.directory.path().join("workbench-sessions.json");
    let plan_head_path = claimable
        .directory
        .path()
        .join("workbench-current-plan-heads.json");
    let admission_path = claimable.directory.path().join("admissions.json");
    let mut authority_store =
        CodexProbeAttemptAuthorityStore::open(authority_path.clone(), "owner-1", claimable.now)
            .expect("open claim store");
    let authority = authority_store
        .issue(
            context(&claimable, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            claimable.now,
        )
        .expect("issue claim authority");
    drop(authority_store);
    let transaction = claimable
        .grant_store
        .begin_authority_transaction()
        .expect("hold claim transaction");
    let session = claimable.session.clone();
    let plan = claimable.plan.clone();
    let process = claimable.process.clone();
    let claim_at = claimable.now + Duration::seconds(1);
    let (started_tx, started_rx) = mpsc::channel();
    let (finished_tx, finished_rx) = mpsc::channel();
    let claim = thread::spawn(move || {
        let session_store = WorkbenchStore::at(session_store_path);
        let plan_head_store = WorkbenchPlanHeadStore::at(plan_head_path);
        let grant_store = WorkbenchProcessGrantStore::at(grant_path);
        let admission_store = WorkbenchProcessAdmissionStore::at(admission_path);
        let mut store = CodexProbeAttemptAuthorityStore::open(authority_path, "owner-1", claim_at)
            .expect("open threaded claim store");
        let claim_context = CodexProbeAttemptContext {
            session: &session,
            session_store: &session_store,
            plan_head_store: &plan_head_store,
            current_plan: &plan,
            process: &process,
            grant_store: &grant_store,
            admission_store: &admission_store,
            request: &request,
            preparation_receipt: &receipt,
        };
        started_tx.send(()).expect("signal claim start");
        let result = store
            .claim(
                &authority.authority_id,
                &authority.record_digest,
                claim_context,
                claim_at,
            )
            .map(|_| ());
        finished_tx.send(result).expect("signal claim finish");
    });
    started_rx.recv().expect("claim started");
    assert!(finished_rx
        .recv_timeout(StdDuration::from_millis(100))
        .is_err());
    drop(transaction);
    finished_rx
        .recv_timeout(StdDuration::from_secs(2))
        .expect("claim unblocked")
        .expect("claim succeeded");
    claim.join().expect("join claim thread");
}

#[test]
fn session_create_and_fork_share_the_authority_transaction_lock() {
    let create_fixture = fixture();
    let create_path = create_fixture
        .directory
        .path()
        .join("workbench-sessions.json");
    let create_transaction = create_fixture
        .grant_store
        .begin_authority_transaction()
        .expect("hold create authority transaction");
    let (create_started_tx, create_started_rx) = mpsc::channel();
    let (create_finished_tx, create_finished_rx) = mpsc::channel();
    let create = thread::spawn(move || {
        let store = WorkbenchStore::at(create_path);
        create_started_tx.send(()).expect("signal create start");
        let result = store
            .create(CreateWorkbenchSessionInput {
                workspace_digest: digest('c'),
                task_class: "coding".into(),
            })
            .map(|_| ());
        create_finished_tx
            .send(result)
            .expect("signal create finish");
    });
    create_started_rx.recv().expect("create started");
    assert!(create_finished_rx
        .recv_timeout(StdDuration::from_millis(100))
        .is_err());
    drop(create_transaction);
    create_finished_rx
        .recv_timeout(StdDuration::from_secs(2))
        .expect("create unblocked")
        .expect("create succeeded");
    create.join().expect("join create thread");

    let fork_fixture = fixture();
    let fork_path = fork_fixture
        .directory
        .path()
        .join("workbench-sessions.json");
    let fork_transaction = fork_fixture
        .grant_store
        .begin_authority_transaction()
        .expect("hold fork authority transaction");
    let parent_session_id = fork_fixture.session.session_id.clone();
    let event_id = fork_fixture
        .session
        .events
        .first()
        .expect("created event")
        .event_id
        .clone();
    let (fork_started_tx, fork_started_rx) = mpsc::channel();
    let (fork_finished_tx, fork_finished_rx) = mpsc::channel();
    let fork = thread::spawn(move || {
        let store = WorkbenchStore::at(fork_path);
        fork_started_tx.send(()).expect("signal fork start");
        let result = store.fork(&parent_session_id, &event_id).map(|_| ());
        fork_finished_tx.send(result).expect("signal fork finish");
    });
    fork_started_rx.recv().expect("fork started");
    assert!(fork_finished_rx
        .recv_timeout(StdDuration::from_millis(100))
        .is_err());
    drop(fork_transaction);
    fork_finished_rx
        .recv_timeout(StdDuration::from_secs(2))
        .expect("fork unblocked")
        .expect("fork succeeded");
    fork.join().expect("join fork thread");
}

#[test]
fn authority_transaction_rejects_a_session_store_from_another_directory() {
    let owner = fixture();
    let other = fixture();
    let transaction = owner
        .grant_store
        .begin_authority_transaction()
        .expect("hold owner authority transaction");
    assert!(other
        .session_store
        .get_for_authority_transaction(&transaction, &other.session.session_id)
        .is_err());
}

#[test]
fn authority_issue_rejects_a_grant_transaction_from_another_directory() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let other = tempfile::tempdir().expect("other authority directory");
    let path = other.path().join("attempt-authorities.json");
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open other authority store");
    assert!(store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .is_err());
    assert!(!path.exists());
    assert!(!path.with_extension("anchor.json").exists());
}

#[cfg(unix)]
#[test]
fn authority_transaction_lock_refuses_symlink_substitution() {
    let value = fixture();
    let lock_path = value
        .directory
        .path()
        .join(".workbench-authority-transaction.lock");
    fs::remove_file(&lock_path).expect("remove original transaction lock");
    let target = value.directory.path().join("transaction-lock-target");
    fs::write(&target, b"transaction-lock-sentinel").expect("write lock target");
    std::os::unix::fs::symlink(&target, &lock_path).expect("substitute lock symlink");
    assert!(value
        .grant_store
        .revoke(&value.grant.grant_id, value.now + Duration::seconds(1),)
        .is_err());
    assert_eq!(
        fs::read(target).expect("preserved lock target"),
        b"transaction-lock-sentinel"
    );
}

#[test]
fn expiry_clock_rollback_revocation_and_inactive_session_preserve_bytes() {
    let exact_expiry = fixture();
    let (request, receipt) = prepare(&exact_expiry).expect("prepare helper contract");
    let path = authority_path(&exact_expiry);
    let mut store =
        CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", exact_expiry.now)
            .expect("open authority store");
    let issued = store
        .issue(
            context(&exact_expiry, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            exact_expiry.now,
        )
        .expect("issue attempt authority");
    let before = fs::read(&path).expect("authority bytes");
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&exact_expiry, &request, &receipt),
            exact_expiry.now + Duration::seconds(60),
        )
        .is_err());
    assert_eq!(fs::read(&path).expect("expiry bytes"), before);
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&exact_expiry, &request, &receipt),
            exact_expiry.now - Duration::seconds(1),
        )
        .is_err());
    assert_eq!(fs::read(&path).expect("rollback bytes"), before);

    let paused = fixture();
    let (paused_request, paused_receipt) = prepare(&paused).expect("prepare paused contract");
    let paused_path = authority_path(&paused);
    let mut paused_store =
        CodexProbeAttemptAuthorityStore::open(paused_path.clone(), "owner-1", paused.now)
            .expect("open paused store");
    let paused_authority = paused_store
        .issue(
            context(&paused, &paused_request, &paused_receipt),
            &codex_probe_attempt_confirmation_phrase(&paused_request.binding.attempt_id),
            paused.now,
        )
        .expect("issue paused authority");
    let paused_before = fs::read(&paused_path).expect("paused bytes");
    paused
        .session_store
        .transition(&paused.session.session_id, WorkbenchSessionAction::Pause)
        .expect("persist paused session");
    assert_eq!(
        paused.session.status,
        super::events::WorkbenchSessionStatus::Active,
        "supplied session snapshot remains stale and active"
    );
    assert!(paused_store
        .claim(
            &paused_authority.authority_id,
            &paused_authority.record_digest,
            context(&paused, &paused_request, &paused_receipt),
            paused.now + Duration::seconds(1),
        )
        .is_err());
    assert_eq!(
        fs::read(&paused_path).expect("paused preserved"),
        paused_before
    );

    let revoked = fixture();
    let (revoked_request, revoked_receipt) = prepare(&revoked).expect("prepare revoked contract");
    let revoked_path = authority_path(&revoked);
    let mut revoked_store =
        CodexProbeAttemptAuthorityStore::open(revoked_path.clone(), "owner-1", revoked.now)
            .expect("open revoked store");
    let revoked_authority = revoked_store
        .issue(
            context(&revoked, &revoked_request, &revoked_receipt),
            &codex_probe_attempt_confirmation_phrase(&revoked_request.binding.attempt_id),
            revoked.now,
        )
        .expect("issue revoked authority");
    let revoked_before = fs::read(&revoked_path).expect("revoked bytes");
    revoked
        .grant_store
        .revoke(&revoked.grant.grant_id, revoked.now + Duration::seconds(1))
        .expect("revoke grant");
    assert!(revoked_store
        .claim(
            &revoked_authority.authority_id,
            &revoked_authority.record_digest,
            context(&revoked, &revoked_request, &revoked_receipt),
            revoked.now + Duration::seconds(2),
        )
        .is_err());
    assert_eq!(
        fs::read(&revoked_path).expect("revoked preserved"),
        revoked_before
    );
}

#[test]
fn plan_process_grant_admission_request_receipt_and_digest_drift_fail_closed() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open authority store");
    let issued = store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue attempt authority");
    let before = fs::read(&path).expect("authority bytes");

    let mut changed_plan = value.plan.clone();
    changed_plan.router_decision.evidence_digest = digest('z');
    let changed_plan_context = CodexProbeAttemptContext {
        session: &value.session,
        session_store: &value.session_store,
        plan_head_store: &value.plan_head_store,
        current_plan: &changed_plan,
        process: &value.process,
        grant_store: &value.grant_store,
        admission_store: &value.admission_store,
        request: &request,
        preparation_receipt: &receipt,
    };
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            changed_plan_context,
            value.now + Duration::seconds(1),
        )
        .is_err());

    let mut changed_process = value.process.clone();
    changed_process.run_id = "process-run:different".into();
    let changed_process_context = CodexProbeAttemptContext {
        session: &value.session,
        session_store: &value.session_store,
        plan_head_store: &value.plan_head_store,
        current_plan: &value.plan,
        process: &changed_process,
        grant_store: &value.grant_store,
        admission_store: &value.admission_store,
        request: &request,
        preparation_receipt: &receipt,
    };
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            changed_process_context,
            value.now + Duration::seconds(1),
        )
        .is_err());

    let other = fixture();
    for (grant_store, admission_store) in [
        (&other.grant_store, &value.admission_store),
        (&value.grant_store, &other.admission_store),
    ] {
        let changed_ledger_context = CodexProbeAttemptContext {
            session: &value.session,
            session_store: &value.session_store,
            plan_head_store: &value.plan_head_store,
            current_plan: &value.plan,
            process: &value.process,
            grant_store,
            admission_store,
            request: &request,
            preparation_receipt: &receipt,
        };
        assert!(store
            .claim(
                &issued.authority_id,
                &issued.record_digest,
                changed_ledger_context,
                value.now + Duration::seconds(1),
            )
            .is_err());
    }

    let mut changed_request = request.clone();
    changed_request.binding.attempt_id = "codex-probe:attempt-different".into();
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &changed_request, &receipt),
            value.now + Duration::seconds(1),
        )
        .is_err());
    let mut changed_receipt = receipt.clone();
    changed_receipt.helper_invoked = true;
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &request, &changed_receipt),
            value.now + Duration::seconds(1),
        )
        .is_err());
    assert!(store
        .claim(
            &issued.authority_id,
            &digest('q'),
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .is_err());
    assert_eq!(fs::read(path).expect("all drift preserved"), before);
}

#[test]
fn missing_or_superseded_current_plan_head_preserves_unclaimed_authority() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open authority store");
    let issued = store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue authority");
    let before = fs::read(&path).expect("unclaimed authority bytes");

    let missing_head_store =
        WorkbenchPlanHeadStore::at(value.directory.path().join("missing-plan-heads.json"));
    let missing_context = CodexProbeAttemptContext {
        session: &value.session,
        session_store: &value.session_store,
        plan_head_store: &missing_head_store,
        current_plan: &value.plan,
        process: &value.process,
        grant_store: &value.grant_store,
        admission_store: &value.admission_store,
        request: &request,
        preparation_receipt: &receipt,
    };
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            missing_context,
            value.now + Duration::seconds(1),
        )
        .is_err());
    assert_eq!(fs::read(&path).expect("missing head preserved"), before);

    let mut next_plan = value.plan.clone();
    next_plan.router_decision.evidence_digest = digest('9');
    next_plan.plan_id = workbench_run_plan_identity(&next_plan).expect("next plan identity");
    let transaction = value
        .grant_store
        .begin_authority_transaction()
        .expect("plan-head supersession transaction");
    let next_head = value
        .plan_head_store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &next_plan,
        )
        .expect("supersede current plan head");
    let restored_head = value
        .plan_head_store
        .publish_for_authority_transaction(
            &transaction,
            &value.session_store,
            &value.session,
            &value.plan,
        )
        .expect("republish original plan as a new head");
    drop(transaction);
    assert_ne!(next_head.head_id, value.plan_head.head_id);
    assert_ne!(restored_head.head_id, value.plan_head.head_id);
    assert_eq!(restored_head.plan_id, value.plan_head.plan_id);
    assert_eq!(
        restored_head.plan_snapshot_digest,
        value.plan_head.plan_snapshot_digest
    );
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .is_err());
    assert_eq!(fs::read(&path).expect("A-B-A replay preserved"), before);

    let (fresh_request, fresh_receipt) = prepare(&value).expect("prepare A2 helper contract");
    assert_eq!(fresh_request.binding.plan_head_id, restored_head.head_id);
    assert_ne!(
        fresh_request.binding.binding_digest,
        request.binding.binding_digest
    );
    assert!(store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &fresh_request, &fresh_receipt),
            value.now + Duration::seconds(1),
        )
        .is_err());
    assert_eq!(fs::read(&path).expect("A1 authority preserved"), before);
}

#[test]
fn owner_epoch_restart_abandons_only_available_authorities_once() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut first = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open first owner");
    let issued = first
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue attempt authority");
    let same = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("reopen same owner");
    assert_eq!(same.reconciled_abandoned_count(), 0);
    assert_eq!(
        same.get(&issued.authority_id)
            .expect("available authority")
            .state,
        CodexProbeAttemptState::AvailableNoProcess
    );

    let before_rollback = fs::read(&path).expect("pre-restart bytes");
    assert!(CodexProbeAttemptAuthorityStore::open(
        path.clone(),
        "owner-2",
        value.now - Duration::seconds(1),
    )
    .is_err());
    assert_eq!(
        fs::read(&path).expect("restart rollback preserved"),
        before_rollback
    );

    let changed = CodexProbeAttemptAuthorityStore::open(
        path.clone(),
        "owner-2",
        value.now + Duration::seconds(1),
    )
    .expect("open changed owner");
    assert_eq!(changed.reconciled_abandoned_count(), 1);
    assert_eq!(
        changed
            .get(&issued.authority_id)
            .expect("abandoned authority")
            .state,
        CodexProbeAttemptState::AbandonedRestart
    );
    let mut repeated =
        CodexProbeAttemptAuthorityStore::open(path, "owner-2", value.now + Duration::seconds(2))
            .expect("repeat changed owner");
    assert_eq!(repeated.reconciled_abandoned_count(), 0);
    assert!(repeated
        .claim(
            &issued.authority_id,
            &repeated
                .get(&issued.authority_id)
                .expect("abandoned record")
                .record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(2),
        )
        .is_err());
}

#[test]
fn idempotent_issue_and_terminal_owner_changes_reject_clock_rollback() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare rollback contract");
    let path = authority_path(&value);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open rollback store");
    let issue_at = value.now + Duration::seconds(10);
    let authority = store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            issue_at,
        )
        .expect("issue future authority");
    let before = fs::read(&path).expect("rollback bytes");
    assert!(store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now + Duration::seconds(5),
        )
        .is_err());
    assert_eq!(
        fs::read(&path).expect("idempotent rollback preserved"),
        before
    );

    store
        .claim(
            &authority.authority_id,
            &authority.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(11),
        )
        .expect("claim terminal authority");
    let claimed_bytes = fs::read(&path).expect("claimed bytes");
    drop(store);
    assert!(CodexProbeAttemptAuthorityStore::open(
        path.clone(),
        "owner-2",
        value.now + Duration::seconds(10),
    )
    .is_err());
    assert_eq!(
        fs::read(path).expect("terminal rollback preserved"),
        claimed_bytes
    );
}

#[test]
fn reserved_authority_is_abandoned_on_owner_epoch_change() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut first = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open first owner");
    let issued = first
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue authority");
    first
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .expect("claim authority");
    let restarted =
        CodexProbeAttemptAuthorityStore::open(path, "owner-2", value.now + Duration::seconds(2))
            .expect("restart store");
    assert_eq!(restarted.reconciled_abandoned_count(), 1);
    let abandoned = restarted
        .get(&issued.authority_id)
        .expect("restart-abandoned reservation");
    assert_eq!(
        abandoned.state,
        CodexProbeAttemptState::AbandonedReservationRestart
    );
    assert!(!abandoned.launch_reserved);
    assert!(abandoned.reservation_closed_at.is_some());
}

#[test]
fn duplicate_attempt_and_capacity_overflow_preserve_existing_ledger() {
    let value = fixture();
    let path = authority_path(&value);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open authority store");
    for index in 0..MAX_CODEX_PROBE_ATTEMPT_AUTHORITIES {
        let mut containment = value.containment.clone();
        containment.attempt_id = format!("codex-probe:attempt-{index}");
        let preflight = evaluate_npm(&value.process, &npm_receipt(), &containment)
            .expect("evaluate distinct attempt");
        let (request, receipt) =
            prepare_with(&value, &preflight, &containment).expect("prepare distinct attempt");
        store
            .issue(
                context(&value, &request, &receipt),
                &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
                value.now,
            )
            .expect("fill authority ledger");
    }
    let before = fs::read(&path).expect("full ledger bytes");
    let mut overflow_containment = value.containment.clone();
    overflow_containment.attempt_id = "codex-probe:attempt-overflow".into();
    let overflow_preflight = evaluate_npm(&value.process, &npm_receipt(), &overflow_containment)
        .expect("evaluate overflow attempt");
    let (overflow_request, overflow_receipt) =
        prepare_with(&value, &overflow_preflight, &overflow_containment)
            .expect("prepare overflow attempt");
    assert!(store
        .issue(
            context(&value, &overflow_request, &overflow_receipt),
            &codex_probe_attempt_confirmation_phrase(&overflow_request.binding.attempt_id),
            value.now,
        )
        .is_err());
    assert_eq!(fs::read(path).expect("capacity preserved"), before);
}

#[test]
fn unknown_envelope_or_record_fields_fail_closed() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open authority store");
    store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue authority");
    let original = fs::read(&path).expect("authority bytes");

    let mut envelope: serde_json::Value =
        serde_json::from_slice(&original).expect("parse authority envelope");
    envelope
        .as_object_mut()
        .expect("authority envelope")
        .insert("path".into(), serde_json::Value::String("forbidden".into()));
    fs::write(
        &path,
        serde_json::to_vec_pretty(&envelope).expect("serialize tampered envelope"),
    )
    .expect("write tampered envelope");
    assert!(CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now).is_err());
    fs::write(&path, &original).expect("restore authority envelope");

    for field in [
        "path",
        "argv",
        "environment",
        "transport",
        "prompt",
        "credential",
        "pid",
        "output",
    ] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&original).expect("parse authority ledger");
        let record = document["authorities"]
            .as_object_mut()
            .expect("authority map")
            .values_mut()
            .next()
            .expect("authority record")
            .as_object_mut()
            .expect("authority object");
        record.insert(field.into(), serde_json::Value::String("forbidden".into()));
        fs::write(
            &path,
            serde_json::to_vec_pretty(&document).expect("serialize tampered ledger"),
        )
        .expect("write tampered ledger");
        assert!(CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now).is_err());
        fs::write(&path, &original).expect("restore authority ledger");
    }
}

#[test]
fn tampered_launch_reservation_fields_fail_closed() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper contract");
    let path = authority_path(&value);
    let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
        .expect("open authority store");
    let issued = store
        .issue(
            context(&value, &request, &receipt),
            &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
            value.now,
        )
        .expect("issue authority");
    store
        .claim(
            &issued.authority_id,
            &issued.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(1),
        )
        .expect("reserve authority");
    drop(store);
    let original = fs::read(&path).expect("reservation bytes");
    for (field, replacement) in [
        (
            "preReservationRecordDigest",
            serde_json::Value::String(digest('p')),
        ),
        (
            "reservationId",
            serde_json::Value::String("codex-probe-reservation:tampered".into()),
        ),
        (
            "reservationBindingDigest",
            serde_json::Value::String(digest('q')),
        ),
        (
            "reservationExpiresAt",
            serde_json::Value::String((value.now + Duration::hours(1)).to_rfc3339()),
        ),
        ("launchReserved", serde_json::Value::Bool(false)),
    ] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&original).expect("parse reservation ledger");
        document["authorities"]
            .as_object_mut()
            .expect("authority map")
            .values_mut()
            .next()
            .expect("authority record")
            .as_object_mut()
            .expect("authority object")
            .insert(field.into(), replacement);
        let tampered =
            serde_json::to_vec_pretty(&document).expect("serialize tampered reservation");
        fs::write(&path, &tampered).expect("write tampered reservation");
        assert!(CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now).is_err());
        assert_eq!(fs::read(&path).expect("preserve tampered bytes"), tampered);
        fs::write(&path, &original).expect("restore reservation ledger");
    }
}

#[test]
fn self_consistent_false_predecessor_and_oversized_reservation_ttl_fail_closed() {
    for tamper in ["predecessor", "ttl"] {
        let value = fixture();
        let (request, receipt) = prepare(&value).expect("prepare helper contract");
        let path = authority_path(&value);
        let mut store = CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now)
            .expect("open authority store");
        let issued = store
            .issue(
                context(&value, &request, &receipt),
                &codex_probe_attempt_confirmation_phrase(&request.binding.attempt_id),
                value.now,
            )
            .expect("issue authority");
        store
            .claim(
                &issued.authority_id,
                &issued.record_digest,
                context(&value, &request, &receipt),
                value.now + Duration::seconds(1),
            )
            .expect("reserve authority");
        drop(store);

        rewrite_codex_probe_reservation_with_self_consistent_tamper_for_test(&path, tamper)
            .expect("rewrite self-consistent tampered reservation");
        let tampered_ledger = fs::read(&path).expect("tampered ledger bytes");
        let tampered_anchor =
            fs::read(path.with_extension("anchor.json")).expect("tampered anchor bytes");
        assert!(CodexProbeAttemptAuthorityStore::open(path.clone(), "owner-1", value.now).is_err());
        assert_eq!(
            fs::read(&path).expect("preserved tampered ledger"),
            tampered_ledger
        );
        assert_eq!(
            fs::read(path.with_extension("anchor.json")).expect("preserved tampered anchor"),
            tampered_anchor
        );
    }
}

#[test]
fn source_has_no_helper_process_transport_renderer_or_command_surface() {
    let source = include_str!("codex_probe_attempt_authority.rs");
    for forbidden in [
        "std::process",
        "tokio::process",
        "Command::new",
        "tauri::command",
        "TcpStream",
        "TcpListener",
        "UnixStream",
        "reqwest::",
        "unsafe {",
        "libc::",
        "nix::",
    ] {
        assert!(
            !source.contains(forbidden),
            "attempt authority unexpectedly contains {forbidden}"
        );
    }
    let root_commands = include_str!("../lib.rs");
    let tauri_config = include_str!("../../tauri.conf.json");
    assert!(!root_commands.contains("CodexProbeAttemptAuthority"));
    assert!(!root_commands.contains("codex_probe_attempt_authority"));
    assert!(!tauri_config.contains("codex-probe-attempt"));
}

fn assert_forbidden_keys_absent(value: &serde_json::Value) {
    const FORBIDDEN: &[&str] = &[
        "path",
        "executable",
        "command",
        "argument",
        "arguments",
        "argv",
        "env",
        "environment",
        "stdin",
        "shell",
        "cwd",
        "workingDirectory",
        "prompt",
        "credential",
        "headers",
        "transport",
        "pid",
        "pgid",
        "stdout",
        "stderr",
        "output",
    ];
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                assert!(!FORBIDDEN.contains(&key.as_str()), "forbidden key {key}");
                assert_forbidden_keys_absent(nested);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                assert_forbidden_keys_absent(nested);
            }
        }
        _ => {}
    }
}
