//! Attempt-bound preparation for a future restricted Codex probe helper.
//!
//! This module creates only content-free, non-executing request and receipt
//! contracts. It does not resolve a helper, create a command, start a process,
//! persist state, inspect the filesystem, or grant execution. A separately
//! packaged and separately signed nested helper plus launch-time revalidation
//! remain open.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::capability_grant::{WorkbenchProcessGrantStore, WorkbenchProcessStartGrant};
use super::codex_command_catalog::CodexProbePlan;
use super::codex_probe_preflight::{CodexManualProbePreflight, CodexProbeContainmentObservation};
use super::codex_probe_preflight_digest::{bounded_digest, containment_digest, probe_plan_digest};
use super::events::{validate_identifier, WorkbenchSessionStatus};
use super::process_run_spec::{process_run_spec_digest, ProcessRunSpec};
use super::process_supervisor::{
    admit_process, WorkbenchProcessAdmission, WorkbenchProcessAdmissionStore,
};
use super::run_contract::{validate_workbench_run_plan, workbench_run_plan_snapshot_digest};
use super::session::validate_digest;
use super::{WorkbenchRunPlan, WorkbenchSession};

const CONTRACT_SCHEMA_VERSION: u32 = 1;
const CODEX_ADAPTER_ID: &str = "codex";
const HELPER_PROFILE_ID: &str = "macos-restricted-helper-v1";
const HELPER_ACTION_ID: &str = "codex-version-probe-v1";
const HELPER_TRANSPORT: &str = "separately-signed-nested-helper-required";
const TARGET_PROVENANCE: &str = "collected-npm-schema-v2";
const COLLECTION_RECEIPT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexHelperEvidenceKind {
    Session,
    CurrentPlan,
    ProcessRunSpec,
    Grant,
    Admission,
    Containment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexHelperBindingKind {
    SessionPlan,
    PlanProcess,
    PlanGrant,
    PlanAdmission,
    ProcessPreflight,
    AttemptContainment,
    ContainmentPreflight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexHelperLaunchContractError {
    InvalidEvidence(CodexHelperEvidenceKind),
    SessionNotActive,
    GrantNotActive,
    ClockRollback,
    CollectedNpmPreflightRequired,
    PreflightTampered,
    BindingMismatch(CodexHelperBindingKind),
    NoProcessBoundaryViolation,
    DigestFailure,
}

impl fmt::Display for CodexHelperLaunchContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Codex helper launch preparation failed: {self:?}"
        )
    }
}

impl std::error::Error for CodexHelperLaunchContractError {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct CodexHelperLaunchBinding {
    pub session_id: String,
    pub session_snapshot_digest: String,
    pub workspace_digest: String,
    pub plan_id: String,
    pub plan_snapshot_digest: String,
    pub process_run_id: String,
    pub process_run_spec_digest: String,
    pub grant_id: String,
    pub grant_receipt_digest: String,
    pub admission_id: String,
    pub admission_receipt_digest: String,
    pub attempt_id: String,
    pub host_instance_identity_digest: String,
    pub boot_session_identity_digest: String,
    pub os_build_identity_digest: String,
    pub containment_profile_id: String,
    pub helper_code_identity_digest: String,
    pub helper_entitlements_identity_digest: String,
    pub enforcement_policy_identity_digest: String,
    pub helper_action_id: String,
    pub helper_transport: String,
    pub target_provenance: String,
    pub collection_receipt_schema_version: u32,
    pub probe_plan_digest: String,
    pub candidate_id: String,
    pub launcher_chain_identity_digest: String,
    pub containment_identity_digest: String,
    pub preflight_identity_digest: String,
    pub binding_digest: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum CodexHelperLaunchRequestState {
    PreparedNoProcess,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct CodexHelperLaunchRequest {
    pub schema_version: u32,
    pub request_id: String,
    pub binding: CodexHelperLaunchBinding,
    pub state: CodexHelperLaunchRequestState,
    pub manual_opt_in_required: bool,
    pub runnable: bool,
    pub supported: bool,
    pub process_start_enabled: bool,
    pub provider_traffic: String,
    pub user_workspace_writes_enabled: bool,
    pub request_digest: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum CodexHelperLaunchPreparationState {
    ValidatedNoProcess,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct CodexHelperLaunchPreparationReceipt {
    pub schema_version: u32,
    pub receipt_id: String,
    pub request_id: String,
    pub request_digest: String,
    pub prepared_at: String,
    pub state: CodexHelperLaunchPreparationState,
    pub helper_invoked: bool,
    pub process_started: bool,
    pub execution_reserved: bool,
    pub runnable: bool,
    pub supported: bool,
    pub provider_traffic: String,
    pub user_workspace_writes_enabled: bool,
    pub receipt_digest: String,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_codex_helper_launch_contract(
    session: &WorkbenchSession,
    current_plan: &WorkbenchRunPlan,
    process: &ProcessRunSpec,
    grant_store: &WorkbenchProcessGrantStore,
    grant_id: &str,
    admission_store: &WorkbenchProcessAdmissionStore,
    admission_id: &str,
    probe_plan: &CodexProbePlan,
    preflight: &CodexManualProbePreflight,
    containment: &CodexProbeContainmentObservation,
    now: DateTime<Utc>,
) -> Result<
    (
        CodexHelperLaunchRequest,
        CodexHelperLaunchPreparationReceipt,
    ),
    CodexHelperLaunchContractError,
> {
    session
        .validate()
        .map_err(|_| invalid(CodexHelperEvidenceKind::Session))?;
    validate_workbench_run_plan(current_plan)
        .map_err(|_| invalid(CodexHelperEvidenceKind::CurrentPlan))?;
    process
        .validate()
        .map_err(|_| invalid(CodexHelperEvidenceKind::ProcessRunSpec))?;
    let grant = grant_store
        .require_current_for(
            grant_id,
            &session.session_id,
            &current_plan.plan_id,
            &process.run_id,
        )
        .map_err(|_| invalid(CodexHelperEvidenceKind::Grant))?;
    let admission = admission_store
        .require_exact_for(
            admission_id,
            &session.session_id,
            &current_plan.plan_id,
            &process.run_id,
            &grant.grant_id,
        )
        .map_err(|_| invalid(CodexHelperEvidenceKind::Admission))?;
    validate_authority(
        session,
        current_plan,
        process,
        &grant,
        &admission,
        probe_plan,
        preflight,
        containment,
        now,
    )?;

    let session_snapshot = serde_json::to_string(session)
        .map_err(|_| CodexHelperLaunchContractError::DigestFailure)?;
    let session_snapshot_digest = bounded_digest(
        b"ai-switchboard-codex-helper-session-snapshot-v1\0",
        &[session_snapshot.as_str()],
    );
    let process_digest = process_run_spec_digest(process)
        .map_err(|_| invalid(CodexHelperEvidenceKind::ProcessRunSpec))?;
    let plan_snapshot_digest = workbench_run_plan_snapshot_digest(current_plan)
        .map_err(|_| invalid(CodexHelperEvidenceKind::CurrentPlan))?;
    let mut binding = CodexHelperLaunchBinding {
        session_id: session.session_id.clone(),
        session_snapshot_digest,
        workspace_digest: session.workspace_digest.clone(),
        plan_id: current_plan.plan_id.clone(),
        plan_snapshot_digest,
        process_run_id: process.run_id.clone(),
        process_run_spec_digest: process_digest,
        grant_id: grant.grant_id.clone(),
        grant_receipt_digest: grant.receipt_digest.clone(),
        admission_id: admission.admission_id.clone(),
        admission_receipt_digest: admission.receipt_digest.clone(),
        attempt_id: containment.attempt_id.clone(),
        host_instance_identity_digest: containment.host_instance_identity_digest.clone(),
        boot_session_identity_digest: containment.boot_session_identity_digest.clone(),
        os_build_identity_digest: containment.os_build_identity_digest.clone(),
        containment_profile_id: containment.profile_id.clone(),
        helper_code_identity_digest: containment.helper_code_identity_digest.clone(),
        helper_entitlements_identity_digest: containment
            .helper_entitlements_identity_digest
            .clone(),
        enforcement_policy_identity_digest: containment.enforcement_policy_identity_digest.clone(),
        helper_action_id: HELPER_ACTION_ID.into(),
        helper_transport: HELPER_TRANSPORT.into(),
        target_provenance: TARGET_PROVENANCE.into(),
        collection_receipt_schema_version: COLLECTION_RECEIPT_SCHEMA_VERSION,
        probe_plan_digest: probe_plan_digest(probe_plan),
        candidate_id: preflight.candidate_id.clone(),
        launcher_chain_identity_digest: preflight.launcher_chain_identity_digest.clone(),
        containment_identity_digest: containment_digest(containment),
        preflight_identity_digest: preflight.preflight_identity_digest.clone(),
        binding_digest: String::new(),
    };
    binding.binding_digest = launch_binding_digest(&binding);
    binding.validate()?;

    let mut request = CodexHelperLaunchRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        request_id: format!(
            "codex-helper-request:{}",
            binding.binding_digest.trim_start_matches("sha256:")
        ),
        binding,
        state: CodexHelperLaunchRequestState::PreparedNoProcess,
        manual_opt_in_required: true,
        runnable: false,
        supported: false,
        process_start_enabled: false,
        provider_traffic: "none".into(),
        user_workspace_writes_enabled: false,
        request_digest: String::new(),
    };
    request.request_digest = launch_request_digest(&request);
    request.validate()?;

    let prepared_at = now.to_rfc3339();
    let receipt_identity = bounded_digest(
        b"ai-switchboard-codex-helper-launch-preparation-receipt-id-v1\0",
        &[
            request.request_id.as_str(),
            request.request_digest.as_str(),
            prepared_at.as_str(),
        ],
    );
    let mut receipt = CodexHelperLaunchPreparationReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        receipt_id: format!(
            "codex-helper-receipt:{}",
            receipt_identity.trim_start_matches("sha256:")
        ),
        request_id: request.request_id.clone(),
        request_digest: request.request_digest.clone(),
        prepared_at,
        state: CodexHelperLaunchPreparationState::ValidatedNoProcess,
        helper_invoked: false,
        process_started: false,
        execution_reserved: false,
        runnable: false,
        supported: false,
        provider_traffic: "none".into(),
        user_workspace_writes_enabled: false,
        receipt_digest: String::new(),
    };
    receipt.receipt_digest = launch_preparation_receipt_digest(&receipt);
    receipt.validate_for(&request)?;
    Ok((request, receipt))
}

#[allow(clippy::too_many_arguments)]
fn validate_authority(
    session: &WorkbenchSession,
    current_plan: &WorkbenchRunPlan,
    process: &ProcessRunSpec,
    grant: &WorkbenchProcessStartGrant,
    admission: &WorkbenchProcessAdmission,
    probe_plan: &CodexProbePlan,
    preflight: &CodexManualProbePreflight,
    containment: &CodexProbeContainmentObservation,
    now: DateTime<Utc>,
) -> Result<(), CodexHelperLaunchContractError> {
    session
        .validate()
        .map_err(|_| invalid(CodexHelperEvidenceKind::Session))?;
    if session.status != WorkbenchSessionStatus::Active {
        return Err(CodexHelperLaunchContractError::SessionNotActive);
    }
    validate_workbench_run_plan(current_plan)
        .map_err(|_| invalid(CodexHelperEvidenceKind::CurrentPlan))?;
    process
        .validate()
        .map_err(|_| invalid(CodexHelperEvidenceKind::ProcessRunSpec))?;
    if current_plan.session_id != session.session_id
        || current_plan.workspace_digest != session.workspace_digest
    {
        return Err(binding(CodexHelperBindingKind::SessionPlan));
    }
    if current_plan.schema_version != 1
        || current_plan.adapter_id != CODEX_ADAPTER_ID
        || process.adapter_id != CODEX_ADAPTER_ID
        || current_plan.execution_mode != "plan_only"
        || current_plan.provider_traffic != "none"
        || current_plan.writes_enabled
        || process.provider_traffic != "none"
        || process.writes_enabled
    {
        return Err(CodexHelperLaunchContractError::NoProcessBoundaryViolation);
    }
    let readiness = current_plan
        .command_readiness
        .as_ref()
        .ok_or(CodexHelperLaunchContractError::NoProcessBoundaryViolation)?;
    if readiness.adapter_id != CODEX_ADAPTER_ID
        || readiness.adapter_plan_id != current_plan.adapter_plan_id
        || readiness.process_start_enabled
        || readiness.provider_traffic != "none"
        || readiness.writes_enabled
        || !current_plan.capability_requests.iter().any(|request| {
            request.capability_id == "adapter_command_readiness" && !request.execution_enabled
        })
    {
        return Err(CodexHelperLaunchContractError::NoProcessBoundaryViolation);
    }
    if current_plan.process_containment.as_ref() != Some(process)
        || process.session_id != session.session_id
        || process.adapter_plan_id != current_plan.adapter_plan_id
    {
        return Err(binding(CodexHelperBindingKind::PlanProcess));
    }

    grant
        .validate()
        .map_err(|_| invalid(CodexHelperEvidenceKind::Grant))?;
    admission
        .validate()
        .map_err(|_| invalid(CodexHelperEvidenceKind::Admission))?;
    let grant_issued_at = parse_time(&grant.issued_at, CodexHelperEvidenceKind::Grant)?;
    let admitted_at = parse_time(&admission.admitted_at, CodexHelperEvidenceKind::Admission)?;
    if now < grant_issued_at || now < admitted_at {
        return Err(CodexHelperLaunchContractError::ClockRollback);
    }
    grant
        .require_active_at(now)
        .map_err(|_| CodexHelperLaunchContractError::GrantNotActive)?;
    if grant.session_id != session.session_id
        || grant.plan_id != current_plan.plan_id
        || grant.process_run_id != process.run_id
    {
        return Err(binding(CodexHelperBindingKind::PlanGrant));
    }
    if admission.session_id != session.session_id
        || admission.plan_id != current_plan.plan_id
        || admission.process_run_id != process.run_id
        || admission.grant_id != grant.grant_id
    {
        return Err(binding(CodexHelperBindingKind::PlanAdmission));
    }
    let expected_admission = admit_process(session, current_plan, process, grant, admitted_at)
        .map_err(|_| invalid(CodexHelperEvidenceKind::Admission))?;
    if &expected_admission != admission {
        return Err(binding(CodexHelperBindingKind::PlanAdmission));
    }

    if !preflight.has_collected_npm_origin() {
        return Err(CodexHelperLaunchContractError::CollectedNpmPreflightRequired);
    }
    preflight
        .validate_for_collected_helper(process, probe_plan, containment)
        .map_err(|_| CodexHelperLaunchContractError::PreflightTampered)?;
    if preflight.process_run_id != process.run_id {
        return Err(binding(CodexHelperBindingKind::ProcessPreflight));
    }
    if preflight.attempt_id != containment.attempt_id {
        return Err(binding(CodexHelperBindingKind::AttemptContainment));
    }
    if preflight.containment_identity_digest != containment_digest(containment) {
        return Err(binding(CodexHelperBindingKind::ContainmentPreflight));
    }
    Ok(())
}

impl CodexHelperLaunchBinding {
    pub(super) fn validate(&self) -> Result<(), CodexHelperLaunchContractError> {
        for value in [
            &self.session_id,
            &self.plan_id,
            &self.process_run_id,
            &self.grant_id,
            &self.admission_id,
            &self.attempt_id,
            &self.candidate_id,
        ] {
            validate_identifier(value, "Codex helper binding identifier")
                .map_err(|_| CodexHelperLaunchContractError::DigestFailure)?;
        }
        for value in [
            &self.session_snapshot_digest,
            &self.workspace_digest,
            &self.plan_snapshot_digest,
            &self.process_run_spec_digest,
            &self.grant_receipt_digest,
            &self.admission_receipt_digest,
            &self.host_instance_identity_digest,
            &self.boot_session_identity_digest,
            &self.os_build_identity_digest,
            &self.helper_code_identity_digest,
            &self.helper_entitlements_identity_digest,
            &self.enforcement_policy_identity_digest,
            &self.probe_plan_digest,
            &self.launcher_chain_identity_digest,
            &self.containment_identity_digest,
            &self.preflight_identity_digest,
            &self.binding_digest,
        ] {
            validate_digest(value, "Codex helper binding digest")
                .map_err(|_| CodexHelperLaunchContractError::DigestFailure)?;
        }
        if self.containment_profile_id != HELPER_PROFILE_ID
            || self.helper_action_id != HELPER_ACTION_ID
            || self.helper_transport != HELPER_TRANSPORT
            || self.target_provenance != TARGET_PROVENANCE
            || self.collection_receipt_schema_version != COLLECTION_RECEIPT_SCHEMA_VERSION
            || self.binding_digest != launch_binding_digest(self)
        {
            return Err(CodexHelperLaunchContractError::NoProcessBoundaryViolation);
        }
        Ok(())
    }
}

impl CodexHelperLaunchRequest {
    pub(super) fn validate(&self) -> Result<(), CodexHelperLaunchContractError> {
        self.binding.validate()?;
        validate_identifier(&self.request_id, "Codex helper request ID")
            .map_err(|_| CodexHelperLaunchContractError::DigestFailure)?;
        validate_digest(&self.request_digest, "Codex helper request digest")
            .map_err(|_| CodexHelperLaunchContractError::DigestFailure)?;
        let expected_request_id = format!(
            "codex-helper-request:{}",
            self.binding.binding_digest.trim_start_matches("sha256:")
        );
        if self.schema_version != CONTRACT_SCHEMA_VERSION
            || self.request_id != expected_request_id
            || self.state != CodexHelperLaunchRequestState::PreparedNoProcess
            || !self.manual_opt_in_required
            || self.runnable
            || self.supported
            || self.process_start_enabled
            || self.provider_traffic != "none"
            || self.user_workspace_writes_enabled
            || self.request_digest != launch_request_digest(self)
        {
            return Err(CodexHelperLaunchContractError::NoProcessBoundaryViolation);
        }
        Ok(())
    }
}

impl CodexHelperLaunchPreparationReceipt {
    pub(super) fn validate_for(
        &self,
        request: &CodexHelperLaunchRequest,
    ) -> Result<(), CodexHelperLaunchContractError> {
        request.validate()?;
        validate_identifier(&self.receipt_id, "Codex helper preparation receipt ID")
            .map_err(|_| CodexHelperLaunchContractError::DigestFailure)?;
        validate_digest(
            &self.receipt_digest,
            "Codex helper preparation receipt digest",
        )
        .map_err(|_| CodexHelperLaunchContractError::DigestFailure)?;
        parse_time(&self.prepared_at, CodexHelperEvidenceKind::Containment)?;
        let expected_receipt_identity = bounded_digest(
            b"ai-switchboard-codex-helper-launch-preparation-receipt-id-v1\0",
            &[
                request.request_id.as_str(),
                request.request_digest.as_str(),
                self.prepared_at.as_str(),
            ],
        );
        let expected_receipt_id = format!(
            "codex-helper-receipt:{}",
            expected_receipt_identity.trim_start_matches("sha256:")
        );
        if self.schema_version != CONTRACT_SCHEMA_VERSION
            || self.receipt_id != expected_receipt_id
            || self.request_id != request.request_id
            || self.request_digest != request.request_digest
            || self.state != CodexHelperLaunchPreparationState::ValidatedNoProcess
            || self.helper_invoked
            || self.process_started
            || self.execution_reserved
            || self.runnable
            || self.supported
            || self.provider_traffic != "none"
            || self.user_workspace_writes_enabled
            || self.receipt_digest != launch_preparation_receipt_digest(self)
        {
            return Err(CodexHelperLaunchContractError::NoProcessBoundaryViolation);
        }
        Ok(())
    }
}

fn launch_binding_digest(value: &CodexHelperLaunchBinding) -> String {
    let collection_schema_version = value.collection_receipt_schema_version.to_string();
    bounded_digest(
        b"ai-switchboard-codex-helper-launch-binding-v1\0",
        &[
            value.session_id.as_str(),
            value.session_snapshot_digest.as_str(),
            value.workspace_digest.as_str(),
            value.plan_id.as_str(),
            value.plan_snapshot_digest.as_str(),
            value.process_run_id.as_str(),
            value.process_run_spec_digest.as_str(),
            value.grant_id.as_str(),
            value.grant_receipt_digest.as_str(),
            value.admission_id.as_str(),
            value.admission_receipt_digest.as_str(),
            value.attempt_id.as_str(),
            value.host_instance_identity_digest.as_str(),
            value.boot_session_identity_digest.as_str(),
            value.os_build_identity_digest.as_str(),
            value.containment_profile_id.as_str(),
            value.helper_code_identity_digest.as_str(),
            value.helper_entitlements_identity_digest.as_str(),
            value.enforcement_policy_identity_digest.as_str(),
            value.helper_action_id.as_str(),
            value.helper_transport.as_str(),
            value.target_provenance.as_str(),
            collection_schema_version.as_str(),
            value.probe_plan_digest.as_str(),
            value.candidate_id.as_str(),
            value.launcher_chain_identity_digest.as_str(),
            value.containment_identity_digest.as_str(),
            value.preflight_identity_digest.as_str(),
        ],
    )
}

fn launch_request_digest(value: &CodexHelperLaunchRequest) -> String {
    let schema_version = value.schema_version.to_string();
    let flags = format!(
        "{}{}{}{}{}",
        value.manual_opt_in_required as u8,
        value.runnable as u8,
        value.supported as u8,
        value.process_start_enabled as u8,
        value.user_workspace_writes_enabled as u8,
    );
    bounded_digest(
        b"ai-switchboard-codex-helper-launch-request-v1\0",
        &[
            schema_version.as_str(),
            value.request_id.as_str(),
            value.binding.binding_digest.as_str(),
            "prepared_no_process",
            flags.as_str(),
            value.provider_traffic.as_str(),
        ],
    )
}

fn launch_preparation_receipt_digest(value: &CodexHelperLaunchPreparationReceipt) -> String {
    let schema_version = value.schema_version.to_string();
    let flags = format!(
        "{}{}{}{}{}{}",
        value.helper_invoked as u8,
        value.process_started as u8,
        value.execution_reserved as u8,
        value.runnable as u8,
        value.supported as u8,
        value.user_workspace_writes_enabled as u8,
    );
    bounded_digest(
        b"ai-switchboard-codex-helper-launch-preparation-receipt-v1\0",
        &[
            schema_version.as_str(),
            value.receipt_id.as_str(),
            value.request_id.as_str(),
            value.request_digest.as_str(),
            value.prepared_at.as_str(),
            "validated_no_process",
            flags.as_str(),
            value.provider_traffic.as_str(),
        ],
    )
}

fn parse_time(
    value: &str,
    evidence: CodexHelperEvidenceKind,
) -> Result<DateTime<Utc>, CodexHelperLaunchContractError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| invalid(evidence))
}

fn invalid(kind: CodexHelperEvidenceKind) -> CodexHelperLaunchContractError {
    CodexHelperLaunchContractError::InvalidEvidence(kind)
}

fn binding(kind: CodexHelperBindingKind) -> CodexHelperLaunchContractError {
    CodexHelperLaunchContractError::BindingMismatch(kind)
}
