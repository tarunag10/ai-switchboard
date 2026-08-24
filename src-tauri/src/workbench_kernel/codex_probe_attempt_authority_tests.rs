use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::Duration as StdDuration;

use chrono::Duration;

use super::capability_grant::WorkbenchProcessGrantStore;
use super::codex_probe_attempt_authority::{
    codex_probe_attempt_confirmation_phrase, CodexProbeAttemptAuthorityStore,
    CodexProbeAttemptContext, CodexProbeAttemptState, MAX_CODEX_PROBE_ATTEMPT_AUTHORITIES,
};
use super::codex_probe_preflight_test_support::{digest, evaluate_npm, npm_receipt};
use super::codex_restricted_helper_preparation::{
    CodexHelperLaunchPreparationReceipt, CodexHelperLaunchRequest,
};
use super::codex_restricted_helper_preparation_tests::{fixture, prepare, prepare_with, Fixture};
use super::events::WorkbenchSessionAction;
use super::process_supervisor::WorkbenchProcessAdmissionStore;

fn context<'a>(
    value: &'a Fixture,
    request: &'a CodexHelperLaunchRequest,
    receipt: &'a CodexHelperLaunchPreparationReceipt,
) -> CodexProbeAttemptContext<'a> {
    CodexProbeAttemptContext {
        session: &value.session,
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
    assert!(first.manual_opt_in_confirmed);
    assert!(!first.helper_invoked);
    assert!(!first.process_started);
    assert!(!first.process_start_enabled);
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
fn claim_is_exactly_one_terminal_no_process_revision() {
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
    assert_eq!(claimed.state, CodexProbeAttemptState::ClaimedNoProcess);
    assert_eq!(claimed.revision, 1);
    assert!(claimed.transition_at.is_some());
    assert!(claimed.claim_id.is_some());
    assert!(!claimed.helper_invoked);
    assert!(!claimed.process_started);
    assert!(store
        .claim(
            &issued.authority_id,
            &claimed.record_digest,
            context(&value, &request, &receipt),
            value.now + Duration::seconds(2),
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
fn grant_mutation_and_claim_share_one_cross_process_transaction_lock() {
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
        let grant_store = WorkbenchProcessGrantStore::at(grant_path);
        let admission_store = WorkbenchProcessAdmissionStore::at(admission_path);
        let mut store = CodexProbeAttemptAuthorityStore::open(authority_path, "owner-1", claim_at)
            .expect("open threaded claim store");
        let claim_context = CodexProbeAttemptContext {
            session: &session,
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

    let mut paused = fixture();
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
        .session
        .transition(WorkbenchSessionAction::Pause)
        .expect("pause session");
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
fn claimed_authority_remains_terminal_across_restart() {
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
    assert_eq!(restarted.reconciled_abandoned_count(), 0);
    assert_eq!(
        restarted
            .get(&issued.authority_id)
            .expect("claimed record")
            .state,
        CodexProbeAttemptState::ClaimedNoProcess
    );
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
