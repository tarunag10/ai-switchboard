use chrono::{Duration, TimeZone, Utc};

use super::adapter_readiness::command_readiness_for;
use super::capability_grant::{
    issue_process_start_grant, process_start_confirmation_phrase, WorkbenchProcessGrantStore,
    WorkbenchProcessStartGrant,
};
use super::codex_command_catalog::CodexProbePlan;
use super::codex_probe_preflight::{CodexManualProbePreflight, CodexProbeContainmentObservation};
use super::codex_probe_preflight_test_support::{
    containment, digest, direct_target, evaluate, evaluate_npm, npm_receipt,
    npm_receipt_without_signature, probe_plan,
};
use super::codex_restricted_helper_preparation::{
    prepare_codex_helper_launch_contract, CodexHelperEvidenceKind, CodexHelperLaunchContractError,
    CodexHelperLaunchPreparationReceipt, CodexHelperLaunchRequest,
};
use super::events::WorkbenchSessionAction;
use super::process_run_spec::{process_run_spec_for, ProcessRunSpec};
use super::process_supervisor::{
    admit_process, WorkbenchProcessAdmission, WorkbenchProcessAdmissionStore,
};
use super::run_contract::{workbench_run_plan_identity, workbench_run_plan_snapshot_digest};
use super::session::CreateWorkbenchSessionInput;
use super::{CapabilityRequest, RouterDecisionReference, WorkbenchRunPlan, WorkbenchSession};
use crate::client_adapter_contract::{
    coding_client_adapter_for_version, ConfigPlanAction, CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
};
use crate::models::SwitchboardMode;

pub(super) struct Fixture {
    pub(super) session: WorkbenchSession,
    pub(super) plan: WorkbenchRunPlan,
    pub(super) process: ProcessRunSpec,
    pub(super) grant: WorkbenchProcessStartGrant,
    pub(super) admission: WorkbenchProcessAdmission,
    pub(super) probe_plan: CodexProbePlan,
    pub(super) containment: CodexProbeContainmentObservation,
    pub(super) preflight: CodexManualProbePreflight,
    pub(super) now: chrono::DateTime<Utc>,
    pub(super) grant_store: WorkbenchProcessGrantStore,
    pub(super) admission_store: WorkbenchProcessAdmissionStore,
    pub(super) directory: tempfile::TempDir,
}

pub(super) fn fixture() -> Fixture {
    let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
        workspace_digest: digest('w'),
        task_class: "coding".into(),
    })
    .expect("create Workbench session");
    let adapter =
        coding_client_adapter_for_version("codex", CODING_CLIENT_ADAPTER_CONTRACT_VERSION)
            .expect("canonical Codex adapter");
    let adapter_plan = adapter
        .plan(SwitchboardMode::Off)
        .expect("canonical Codex adapter plan");
    let adapter_plan_id = adapter_plan.plan_id.clone();
    let process = process_run_spec_for(
        &session.session_id,
        &adapter_plan_id,
        "codex",
        &session.workspace_digest,
    )
    .expect("create process spec");
    let mut plan = WorkbenchRunPlan {
        schema_version: 1,
        plan_id: "run-plan:pending".into(),
        session_id: session.session_id.clone(),
        adapter_id: "codex".into(),
        workspace_digest: session.workspace_digest.clone(),
        context_pack_digest: None,
        router_decision: RouterDecisionReference {
            decision_id: "routing-decision-test".into(),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: digest('r'),
        },
        replay_reference: None,
        preset: None,
        requested_mode: SwitchboardMode::Off,
        adapter_plan_id: adapter_plan_id.clone(),
        adapter_action: match adapter_plan.action {
            ConfigPlanAction::ApplyManagedRouting => "apply_managed_routing".into(),
            ConfigPlanAction::CleanupManagedRouting => "cleanup_managed_routing".into(),
        },
        adapter_reversible: adapter_plan.reversible,
        command_readiness: Some(
            command_readiness_for("codex", &adapter_plan_id).expect("canonical command readiness"),
        ),
        process_containment: Some(process.clone()),
        capability_requests: [
            "router_observe",
            "client_adapter_plan",
            "adapter_command_readiness",
        ]
        .into_iter()
        .map(|capability_id| CapabilityRequest {
            capability_id: capability_id.into(),
            scope: "session".into(),
            approval_state: "pending".into(),
            execution_enabled: false,
        })
        .collect(),
        execution_mode: "plan_only".into(),
        provider_traffic: "none".into(),
        writes_enabled: false,
    };
    plan.plan_id = workbench_run_plan_identity(&plan).expect("canonical run plan identity");
    let now = Utc.with_ymd_and_hms(2026, 8, 24, 0, 0, 0).unwrap();
    let grant = issue_process_start_grant(
        &session,
        &plan,
        &process_start_confirmation_phrase(&plan),
        now,
    )
    .expect("issue process grant");
    let admission =
        admit_process(&session, &plan, &process, &grant, now).expect("admit prepared process");
    let directory = tempfile::tempdir().expect("temporary authority directory");
    let grant_store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
    let admission_store =
        WorkbenchProcessAdmissionStore::at(directory.path().join("admissions.json"));
    grant_store
        .issue(grant.clone(), now)
        .expect("persist process grant");
    admission_store
        .issue(admission.clone())
        .expect("persist process admission");
    let probe_plan = probe_plan();
    let containment = containment();
    let preflight = evaluate_npm(&process, &npm_receipt(), &containment)
        .expect("evaluate collected Codex preflight");
    Fixture {
        session,
        plan,
        process,
        grant,
        admission,
        probe_plan,
        containment,
        preflight,
        now,
        grant_store,
        admission_store,
        directory,
    }
}

pub(super) fn prepare(
    value: &Fixture,
) -> Result<
    (
        CodexHelperLaunchRequest,
        CodexHelperLaunchPreparationReceipt,
    ),
    CodexHelperLaunchContractError,
> {
    prepare_with(value, &value.preflight, &value.containment)
}

pub(super) fn prepare_with(
    value: &Fixture,
    preflight: &CodexManualProbePreflight,
    containment: &CodexProbeContainmentObservation,
) -> Result<
    (
        CodexHelperLaunchRequest,
        CodexHelperLaunchPreparationReceipt,
    ),
    CodexHelperLaunchContractError,
> {
    prepare_codex_helper_launch_contract(
        &value.session,
        &value.plan,
        &value.process,
        &value.grant_store,
        &value.grant.grant_id,
        &value.admission_store,
        &value.admission.admission_id,
        &value.probe_plan,
        preflight,
        containment,
        value.now,
    )
}

#[test]
fn collected_preflight_creates_deterministic_no_process_contract() {
    let value = fixture();
    let first = prepare(&value).expect("prepare helper launch contract");
    let second = prepare(&value).expect("repeat helper launch contract");
    assert_eq!(first, second);
    let (request, receipt) = first;
    assert_eq!(request.binding.session_id, value.session.session_id);
    assert_eq!(request.binding.plan_id, value.plan.plan_id);
    assert_eq!(
        request.binding.plan_snapshot_digest,
        workbench_run_plan_snapshot_digest(&value.plan).expect("plan snapshot digest")
    );
    assert_eq!(request.binding.process_run_id, value.process.run_id);
    assert_eq!(request.binding.grant_id, value.grant.grant_id);
    assert_eq!(request.binding.admission_id, value.admission.admission_id);
    assert_eq!(request.binding.attempt_id, value.containment.attempt_id);
    assert_eq!(
        request.binding.helper_transport,
        "separately-signed-nested-helper-required"
    );
    assert_eq!(request.binding.target_provenance, "collected-npm-schema-v2");
    assert!(request.manual_opt_in_required);
    assert!(!request.runnable);
    assert!(!request.supported);
    assert!(!request.process_start_enabled);
    assert!(!receipt.helper_invoked);
    assert!(!receipt.process_started);
    assert!(!receipt.execution_reserved);
    request.validate().expect("validate request");
    receipt.validate_for(&request).expect("validate receipt");
}

#[test]
fn direct_preflight_cannot_impersonate_collected_npm_provenance() {
    let value = fixture();
    let mut direct = evaluate(&value.process, &direct_target(), &value.containment)
        .expect("evaluate direct preflight");
    assert_eq!(
        prepare_with(&value, &direct, &value.containment),
        Err(CodexHelperLaunchContractError::CollectedNpmPreflightRequired)
    );

    direct.state = "collected_target_shape_complete_non_executing".into();
    direct.reason_code = "restricted_helper_and_manual_harness_still_required".into();
    assert_eq!(
        prepare_with(&value, &direct, &value.containment),
        Err(CodexHelperLaunchContractError::CollectedNpmPreflightRequired)
    );
}

#[test]
fn tampered_preflight_fields_fail_closed() {
    let value = fixture();
    let mut variants = Vec::new();
    let mut changed = value.preflight.clone();
    changed.preflight_identity_digest = digest('x');
    variants.push(changed);
    let mut changed = value.preflight.clone();
    changed.process_run_id = "process-run:different".into();
    variants.push(changed);
    let mut changed = value.preflight.clone();
    changed.attempt_id = "codex-probe:attempt-different".into();
    variants.push(changed);
    let mut changed = value.preflight.clone();
    changed.state = "supplied_evidence_shape_complete_non_executing".into();
    variants.push(changed);
    let mut changed = value.preflight.clone();
    changed.reason_code = "native_collection_and_manual_harness_still_required".into();
    variants.push(changed);
    let mut changed = value.preflight.clone();
    changed.runnable = true;
    variants.push(changed);

    for changed in variants {
        assert_eq!(
            prepare_with(&value, &changed, &value.containment),
            Err(CodexHelperLaunchContractError::PreflightTampered)
        );
    }
}

#[test]
fn inactive_expired_revoked_and_clock_rollback_authority_is_rejected() {
    let mut paused = fixture();
    paused
        .session
        .transition(WorkbenchSessionAction::Pause)
        .expect("pause session");
    assert_eq!(
        prepare(&paused),
        Err(CodexHelperLaunchContractError::SessionNotActive)
    );

    let mut expired = fixture();
    expired.now += Duration::seconds(15 * 60);
    assert_eq!(
        prepare(&expired),
        Err(CodexHelperLaunchContractError::GrantNotActive)
    );

    let mut revoked = fixture();
    revoked
        .grant_store
        .revoke(&revoked.grant.grant_id, revoked.now + Duration::seconds(1))
        .expect("revoke durable grant");
    revoked.now += Duration::seconds(2);
    assert_eq!(
        prepare(&revoked),
        Err(CodexHelperLaunchContractError::GrantNotActive)
    );

    let mut rollback = fixture();
    rollback.now -= Duration::seconds(1);
    assert_eq!(
        prepare(&rollback),
        Err(CodexHelperLaunchContractError::ClockRollback)
    );
}

#[test]
fn never_issued_authority_is_rejected_by_the_durable_ledgers() {
    let value = fixture();
    let directory = tempfile::tempdir().expect("empty authority directory");
    let empty_grants = WorkbenchProcessGrantStore::at(directory.path().join("empty-grants.json"));
    let empty_admissions =
        WorkbenchProcessAdmissionStore::at(directory.path().join("empty-admissions.json"));

    assert_eq!(
        prepare_codex_helper_launch_contract(
            &value.session,
            &value.plan,
            &value.process,
            &empty_grants,
            &value.grant.grant_id,
            &value.admission_store,
            &value.admission.admission_id,
            &value.probe_plan,
            &value.preflight,
            &value.containment,
            value.now,
        ),
        Err(CodexHelperLaunchContractError::InvalidEvidence(
            CodexHelperEvidenceKind::Grant
        ))
    );

    assert_eq!(
        prepare_codex_helper_launch_contract(
            &value.session,
            &value.plan,
            &value.process,
            &value.grant_store,
            &value.grant.grant_id,
            &empty_admissions,
            &value.admission.admission_id,
            &value.probe_plan,
            &value.preflight,
            &value.containment,
            value.now,
        ),
        Err(CodexHelperLaunchContractError::InvalidEvidence(
            CodexHelperEvidenceKind::Admission
        ))
    );
}

#[test]
fn changed_plan_process_grant_and_admission_bindings_are_rejected() {
    let base = fixture();
    let other = fixture();

    let mut executable_capability = base.plan.clone();
    executable_capability.capability_requests[0].execution_enabled = true;
    assert_eq!(
        prepare_codex_helper_launch_contract(
            &base.session,
            &executable_capability,
            &base.process,
            &base.grant_store,
            &base.grant.grant_id,
            &base.admission_store,
            &base.admission.admission_id,
            &base.probe_plan,
            &base.preflight,
            &base.containment,
            base.now,
        ),
        Err(CodexHelperLaunchContractError::InvalidEvidence(
            CodexHelperEvidenceKind::CurrentPlan
        ))
    );

    let mut changed_router = base.plan.clone();
    changed_router.router_decision.evidence_digest = digest('v');
    assert_eq!(
        prepare_codex_helper_launch_contract(
            &base.session,
            &changed_router,
            &base.process,
            &base.grant_store,
            &base.grant.grant_id,
            &base.admission_store,
            &base.admission.admission_id,
            &base.probe_plan,
            &base.preflight,
            &base.containment,
            base.now,
        ),
        Err(CodexHelperLaunchContractError::InvalidEvidence(
            CodexHelperEvidenceKind::CurrentPlan
        ))
    );

    let mut changed_plan = base.plan.clone();
    changed_plan.process_containment = None;
    assert_eq!(
        prepare_codex_helper_launch_contract(
            &base.session,
            &changed_plan,
            &base.process,
            &base.grant_store,
            &base.grant.grant_id,
            &base.admission_store,
            &base.admission.admission_id,
            &base.probe_plan,
            &base.preflight,
            &base.containment,
            base.now,
        ),
        Err(CodexHelperLaunchContractError::InvalidEvidence(
            CodexHelperEvidenceKind::CurrentPlan
        ))
    );

    let mut changed_readiness = base.plan.clone();
    changed_readiness
        .command_readiness
        .as_mut()
        .expect("command readiness")
        .process_start_enabled = true;
    assert_eq!(
        prepare_codex_helper_launch_contract(
            &base.session,
            &changed_readiness,
            &base.process,
            &base.grant_store,
            &base.grant.grant_id,
            &base.admission_store,
            &base.admission.admission_id,
            &base.probe_plan,
            &base.preflight,
            &base.containment,
            base.now,
        ),
        Err(CodexHelperLaunchContractError::InvalidEvidence(
            CodexHelperEvidenceKind::CurrentPlan
        ))
    );

    assert_eq!(
        prepare_codex_helper_launch_contract(
            &base.session,
            &base.plan,
            &base.process,
            &other.grant_store,
            &other.grant.grant_id,
            &base.admission_store,
            &base.admission.admission_id,
            &base.probe_plan,
            &base.preflight,
            &base.containment,
            base.now,
        ),
        Err(CodexHelperLaunchContractError::InvalidEvidence(
            CodexHelperEvidenceKind::Grant
        ))
    );
    assert_eq!(
        prepare_codex_helper_launch_contract(
            &base.session,
            &base.plan,
            &base.process,
            &base.grant_store,
            &base.grant.grant_id,
            &other.admission_store,
            &other.admission.admission_id,
            &base.probe_plan,
            &base.preflight,
            &base.containment,
            base.now,
        ),
        Err(CodexHelperLaunchContractError::InvalidEvidence(
            CodexHelperEvidenceKind::Admission
        ))
    );
}

#[test]
fn containment_identities_and_signature_presence_change_request_identity() {
    let base = fixture();
    let (base_request, _) = prepare(&base).expect("prepare baseline request");
    let mutations: [fn(&mut CodexProbeContainmentObservation); 5] = [
        |value| value.host_instance_identity_digest = digest('u'),
        |value| value.boot_session_identity_digest = digest('b'),
        |value| value.helper_code_identity_digest = digest('k'),
        |value| value.helper_entitlements_identity_digest = digest('l'),
        |value| value.enforcement_policy_identity_digest = digest('p'),
    ];
    for (index, mutate) in mutations.into_iter().enumerate() {
        let mut changed_containment = base.containment.clone();
        mutate(&mut changed_containment);
        let changed_preflight = evaluate_npm(&base.process, &npm_receipt(), &changed_containment)
            .expect("evaluate changed containment");
        let (changed_request, _) = prepare_with(&base, &changed_preflight, &changed_containment)
            .expect("prepare changed request");
        assert_ne!(
            base_request.request_digest, changed_request.request_digest,
            "identity mutation {index} did not change request"
        );
    }

    let unsigned_preflight = evaluate_npm(
        &base.process,
        &npm_receipt_without_signature(),
        &base.containment,
    )
    .expect("evaluate unsigned collected receipt");
    let (unsigned_request, unsigned_receipt) =
        prepare_with(&base, &unsigned_preflight, &base.containment)
            .expect("prepare unsigned request");
    assert_ne!(base_request.request_digest, unsigned_request.request_digest);
    assert!(!unsigned_request.runnable);
    assert!(!unsigned_request.supported);
    assert!(!unsigned_receipt.process_started);
}

#[test]
fn request_and_receipt_are_tamper_evident_and_content_free() {
    let value = fixture();
    let (request, receipt) = prepare(&value).expect("prepare helper launch contract");

    let mut changed_request = request.clone();
    changed_request.process_start_enabled = true;
    assert!(changed_request.validate().is_err());
    let mut changed_request_id = request.clone();
    changed_request_id.request_id = "codex-helper-request:forged".into();
    assert!(changed_request_id.validate().is_err());
    let mut changed_binding = request.clone();
    changed_binding.binding.host_instance_identity_digest = digest('x');
    assert!(changed_binding.validate().is_err());
    let mut changed_receipt = receipt.clone();
    changed_receipt.helper_invoked = true;
    assert!(changed_receipt.validate_for(&request).is_err());
    let mut changed_receipt_id = receipt.clone();
    changed_receipt_id.receipt_id = "codex-helper-receipt:forged".into();
    assert!(changed_receipt_id.validate_for(&request).is_err());

    let serialized = serde_json::json!({
        "request": request,
        "receipt": receipt,
    });
    assert_forbidden_keys_absent(&serialized);
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

#[test]
fn preparation_module_has_no_io_execution_or_deserialization_surface() {
    let source = include_str!("codex_restricted_helper_preparation.rs");
    for forbidden in [
        "Deserialize",
        "std::env",
        "std::fs",
        "std::process",
        "tokio::process",
        "Command::new",
        "tauri::",
        "TcpStream",
        "reqwest::",
        "PathBuf",
        "unsafe {",
        "libc::",
        "nix::",
    ] {
        assert!(
            !source.contains(forbidden),
            "preparation module unexpectedly contains {forbidden}"
        );
    }
}
