//! Ephemeral current-eligibility snapshots for historical process admissions.
//!
//! This module never mutates a receipt or starts a process. It compares an
//! immutable admission with the current session, a freshly prepared native
//! plan, and the validated grant ledger. The result is deliberately transient.

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::capability_grant::WorkbenchProcessStartGrantView;
use super::events::WorkbenchSessionStatus;
use super::process_supervisor::WorkbenchProcessAdmission;
use super::{WorkbenchRunPlan, WorkbenchRunSpecInput, WorkbenchSession};

const ELIGIBILITY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchAdmissionEligibilityInput {
    pub run_spec: WorkbenchRunSpecInput,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchAdmissionEligibilityState {
    Active,
    Expired,
    Revoked,
    SessionPaused,
    SessionTerminal,
    Superseded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchAdmissionEligibilityReason {
    BoundAndCurrent,
    GrantExpired,
    ClockRollback,
    GrantRevoked,
    GrantMissing,
    SessionPaused,
    SessionCancelled,
    SessionCompleted,
    PlanChanged,
    ProcessContainmentChanged,
    ProcessContainmentRemoved,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchAdmissionEligibility {
    #[serde(flatten)]
    pub historical_admission: WorkbenchProcessAdmission,
    pub current_eligibility: WorkbenchAdmissionEligibilityState,
    pub reason: WorkbenchAdmissionEligibilityReason,
    pub grant_effective_state: Option<String>,
    pub evaluated_at: String,
    pub requires_start_revalidation: bool,
    pub execution_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchAdmissionEligibilitySnapshot {
    pub schema_version: u32,
    pub session_id: String,
    pub evaluated_at: String,
    pub current_plan_id: Option<String>,
    pub current_process_run_id: Option<String>,
    pub receipts: Vec<WorkbenchAdmissionEligibility>,
    pub execution_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

pub(crate) fn derive_admission_eligibility(
    session: &WorkbenchSession,
    current_plan: Option<&WorkbenchRunPlan>,
    admissions: Vec<WorkbenchProcessAdmission>,
    grants: &[WorkbenchProcessStartGrantView],
    now: DateTime<Utc>,
) -> Result<WorkbenchAdmissionEligibilitySnapshot> {
    session.validate()?;
    if session.status == WorkbenchSessionStatus::Active && current_plan.is_none() {
        bail!("Active Workbench admission eligibility requires a current native plan");
    }
    let current_process_run_id = current_plan
        .and_then(|plan| plan.process_containment.as_ref())
        .map(|process| process.run_id.clone());
    let evaluated_at = now.to_rfc3339();
    let mut receipts = Vec::with_capacity(admissions.len());
    for admission in admissions {
        admission.validate()?;
        if admission.session_id != session.session_id {
            bail!("Workbench process admission belongs to another session");
        }
        let admitted_at = DateTime::parse_from_rfc3339(&admission.admitted_at)
            .map_err(|_| anyhow!("Workbench process admission time is invalid"))?
            .with_timezone(&Utc);
        let (current_eligibility, reason, grant_effective_state) = match session.status {
            WorkbenchSessionStatus::Cancelled => (
                WorkbenchAdmissionEligibilityState::SessionTerminal,
                WorkbenchAdmissionEligibilityReason::SessionCancelled,
                None,
            ),
            WorkbenchSessionStatus::Completed => (
                WorkbenchAdmissionEligibilityState::SessionTerminal,
                WorkbenchAdmissionEligibilityReason::SessionCompleted,
                None,
            ),
            WorkbenchSessionStatus::Paused => (
                WorkbenchAdmissionEligibilityState::SessionPaused,
                WorkbenchAdmissionEligibilityReason::SessionPaused,
                None,
            ),
            WorkbenchSessionStatus::Active => {
                let plan = current_plan.expect("active session checked for current plan");
                if admission.plan_id != plan.plan_id {
                    (
                        WorkbenchAdmissionEligibilityState::Superseded,
                        WorkbenchAdmissionEligibilityReason::PlanChanged,
                        None,
                    )
                } else if plan.process_containment.is_none() {
                    (
                        WorkbenchAdmissionEligibilityState::Superseded,
                        WorkbenchAdmissionEligibilityReason::ProcessContainmentRemoved,
                        None,
                    )
                } else if Some(&admission.process_run_id)
                    != plan
                        .process_containment
                        .as_ref()
                        .map(|process| &process.run_id)
                {
                    (
                        WorkbenchAdmissionEligibilityState::Superseded,
                        WorkbenchAdmissionEligibilityReason::ProcessContainmentChanged,
                        None,
                    )
                } else {
                    let Some(grant) = grants
                        .iter()
                        .find(|grant| grant.grant_id == admission.grant_id)
                    else {
                        receipts.push(WorkbenchAdmissionEligibility {
                            historical_admission: admission,
                            current_eligibility: WorkbenchAdmissionEligibilityState::Unavailable,
                            reason: WorkbenchAdmissionEligibilityReason::GrantMissing,
                            grant_effective_state: None,
                            evaluated_at: evaluated_at.clone(),
                            requires_start_revalidation: true,
                            execution_enabled: false,
                            provider_traffic: "none".into(),
                            writes_enabled: false,
                        });
                        continue;
                    };
                    if grant.session_id != admission.session_id
                        || grant.plan_id != admission.plan_id
                        || grant.process_run_id != admission.process_run_id
                    {
                        bail!("Workbench process admission grant binding is corrupt");
                    }
                    let issued_at = DateTime::parse_from_rfc3339(&grant.issued_at)
                        .map_err(|_| anyhow!("Workbench process grant issue time is invalid"))?
                        .with_timezone(&Utc);
                    let grant_state = Some(grant.effective_state.clone());
                    if grant.effective_state == "revoked" {
                        (
                            WorkbenchAdmissionEligibilityState::Revoked,
                            WorkbenchAdmissionEligibilityReason::GrantRevoked,
                            grant_state,
                        )
                    } else if now < issued_at || now < admitted_at {
                        (
                            WorkbenchAdmissionEligibilityState::Expired,
                            WorkbenchAdmissionEligibilityReason::ClockRollback,
                            grant_state,
                        )
                    } else if grant.effective_state == "expired" {
                        (
                            WorkbenchAdmissionEligibilityState::Expired,
                            WorkbenchAdmissionEligibilityReason::GrantExpired,
                            grant_state,
                        )
                    } else if grant.effective_state == "active" {
                        (
                            WorkbenchAdmissionEligibilityState::Active,
                            WorkbenchAdmissionEligibilityReason::BoundAndCurrent,
                            grant_state,
                        )
                    } else {
                        bail!("Workbench process grant has an unknown effective state");
                    }
                }
            }
        };
        receipts.push(WorkbenchAdmissionEligibility {
            historical_admission: admission,
            current_eligibility,
            reason,
            grant_effective_state,
            evaluated_at: evaluated_at.clone(),
            requires_start_revalidation: true,
            execution_enabled: false,
            provider_traffic: "none".into(),
            writes_enabled: false,
        });
    }
    Ok(WorkbenchAdmissionEligibilitySnapshot {
        schema_version: ELIGIBILITY_SCHEMA_VERSION,
        session_id: session.session_id.clone(),
        evaluated_at,
        current_plan_id: current_plan.map(|plan| plan.plan_id.clone()),
        current_process_run_id,
        receipts,
        execution_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        derive_admission_eligibility, WorkbenchAdmissionEligibilityReason,
        WorkbenchAdmissionEligibilityState,
    };
    use crate::models::SwitchboardMode;
    use crate::workbench_kernel::capability_grant::{
        issue_process_start_grant, process_start_confirmation_phrase, WorkbenchProcessStartGrant,
        WorkbenchProcessStartGrantView,
    };
    use crate::workbench_kernel::events::WorkbenchSessionAction;
    use crate::workbench_kernel::process_run_spec::process_run_spec_for;
    use crate::workbench_kernel::process_supervisor::{admit_process, WorkbenchProcessAdmission};
    use crate::workbench_kernel::session::{CreateWorkbenchSessionInput, WorkbenchSession};
    use crate::workbench_kernel::{CapabilityRequest, RouterDecisionReference, WorkbenchRunPlan};
    use chrono::{DateTime, Duration, TimeZone, Utc};

    fn session() -> WorkbenchSession {
        WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: format!("sha256:{}", "a".repeat(64)),
            task_class: "coding".into(),
        })
        .expect("create session")
    }

    fn plan(session: &WorkbenchSession) -> WorkbenchRunPlan {
        let adapter_plan_id = "codex-1234567890ab".to_string();
        WorkbenchRunPlan {
            schema_version: 1,
            plan_id: "run-plan:1234567890abcdef1234567890abcdef".into(),
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision: RouterDecisionReference {
                decision_id: "routing-decision-test".into(),
                decision_stage: "observe".into(),
                routing_mode: "observe_only".into(),
                evidence_digest: format!("sha256:{}", "b".repeat(64)),
            },
            replay_reference: None,
            preset: None,
            requested_mode: SwitchboardMode::Off,
            adapter_plan_id: adapter_plan_id.clone(),
            adapter_action: "cleanup_managed_routing".into(),
            adapter_reversible: true,
            command_readiness: Some(crate::workbench_kernel::WorkbenchAdapterCommandReadiness {
                schema_version: 1,
                adapter_id: "codex".into(),
                adapter_contract_version: 1,
                adapter_plan_id: adapter_plan_id.clone(),
                logical_binary: "codex".into(),
                known_candidate_present: false,
                discovery_mode: "fixed_known_location_metadata_only".into(),
                cli_version_probe_state: "not_probed".into(),
                version_probe_reason: "deferred".into(),
                process_start_enabled: false,
                provider_traffic: "none".into(),
                writes_enabled: false,
            }),
            process_containment: Some(
                process_run_spec_for(
                    &session.session_id,
                    &adapter_plan_id,
                    "codex",
                    &session.workspace_digest,
                )
                .expect("process containment"),
            ),
            capability_requests: vec![CapabilityRequest {
                capability_id: "adapter_command_readiness".into(),
                scope: "session".into(),
                approval_state: "pending".into(),
                execution_enabled: false,
            }],
            execution_mode: "plan_only".into(),
            provider_traffic: "none".into(),
            writes_enabled: false,
        }
    }

    fn grant(
        session: &WorkbenchSession,
        plan: &WorkbenchRunPlan,
        now: DateTime<Utc>,
    ) -> WorkbenchProcessStartGrant {
        issue_process_start_grant(session, plan, &process_start_confirmation_phrase(plan), now)
            .expect("issue process start grant")
    }

    fn admission(
        session: &WorkbenchSession,
        plan: &WorkbenchRunPlan,
        grant: &WorkbenchProcessStartGrant,
        now: DateTime<Utc>,
    ) -> WorkbenchProcessAdmission {
        admit_process(
            session,
            plan,
            plan.process_containment
                .as_ref()
                .expect("process containment"),
            grant,
            now,
        )
        .expect("admit process")
    }

    fn grant_view(
        grant: &WorkbenchProcessStartGrant,
        effective_state: &str,
    ) -> WorkbenchProcessStartGrantView {
        WorkbenchProcessStartGrantView {
            schema_version: grant.schema_version,
            grant_id: grant.grant_id.clone(),
            session_id: grant.session_id.clone(),
            plan_id: grant.plan_id.clone(),
            process_run_id: grant.process_run_id.clone(),
            capability_id: grant.capability_id.clone(),
            issued_at: grant.issued_at.clone(),
            expires_at: grant.expires_at.clone(),
            effective_state: effective_state.into(),
            execution_enabled: false,
            provider_traffic: "none".into(),
            writes_enabled: false,
            receipt_digest: grant.receipt_digest.clone(),
        }
    }

    fn assert_state(
        snapshot: &super::WorkbenchAdmissionEligibilitySnapshot,
        state: WorkbenchAdmissionEligibilityState,
        reason: WorkbenchAdmissionEligibilityReason,
    ) {
        assert_eq!(snapshot.receipts.len(), 1);
        assert_eq!(snapshot.receipts[0].current_eligibility, state);
        assert_eq!(snapshot.receipts[0].reason, reason);
        assert!(snapshot.receipts[0].requires_start_revalidation);
        assert!(!snapshot.receipts[0].execution_enabled);
        assert_eq!(snapshot.receipts[0].provider_traffic, "none");
        assert!(!snapshot.receipts[0].writes_enabled);
        assert!(!snapshot.execution_enabled);
        assert_eq!(snapshot.provider_traffic, "none");
        assert!(!snapshot.writes_enabled);
    }

    #[test]
    fn active_expiry_boundary_and_clock_rollback_are_distinct() {
        let session = session();
        let plan = plan(&session);
        let issued_at = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let grant = grant(&session, &plan, issued_at);
        let admission = admission(&session, &plan, &grant, issued_at + Duration::seconds(1));

        let active = derive_admission_eligibility(
            &session,
            Some(&plan),
            vec![admission.clone()],
            &[grant_view(&grant, "active")],
            issued_at + Duration::seconds(2),
        )
        .expect("derive active eligibility");
        assert_state(
            &active,
            WorkbenchAdmissionEligibilityState::Active,
            WorkbenchAdmissionEligibilityReason::BoundAndCurrent,
        );

        let expires_at = DateTime::parse_from_rfc3339(&grant.expires_at)
            .expect("grant expiry")
            .with_timezone(&Utc);
        let expired = derive_admission_eligibility(
            &session,
            Some(&plan),
            vec![admission.clone()],
            &[grant_view(&grant, "expired")],
            expires_at,
        )
        .expect("derive expiry-boundary eligibility");
        assert_state(
            &expired,
            WorkbenchAdmissionEligibilityState::Expired,
            WorkbenchAdmissionEligibilityReason::GrantExpired,
        );

        let rollback = derive_admission_eligibility(
            &session,
            Some(&plan),
            vec![admission],
            &[grant_view(&grant, "expired")],
            issued_at - Duration::seconds(1),
        )
        .expect("derive rollback eligibility");
        assert_state(
            &rollback,
            WorkbenchAdmissionEligibilityState::Expired,
            WorkbenchAdmissionEligibilityReason::ClockRollback,
        );
    }

    #[test]
    fn superseded_and_missing_grant_states_remain_fail_closed() {
        let session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let grant = grant(&session, &plan, now);
        let admission = admission(&session, &plan, &grant, now);

        let mut changed_plan = plan.clone();
        changed_plan.plan_id = "run-plan:ffffffffffffffffffffffffffffffff".into();
        let superseded = derive_admission_eligibility(
            &session,
            Some(&changed_plan),
            vec![admission.clone()],
            &[grant_view(&grant, "active")],
            now,
        )
        .expect("derive superseded eligibility");
        assert_state(
            &superseded,
            WorkbenchAdmissionEligibilityState::Superseded,
            WorkbenchAdmissionEligibilityReason::PlanChanged,
        );

        let missing =
            derive_admission_eligibility(&session, Some(&plan), vec![admission], &[], now)
                .expect("derive missing grant eligibility");
        assert_state(
            &missing,
            WorkbenchAdmissionEligibilityState::Unavailable,
            WorkbenchAdmissionEligibilityReason::GrantMissing,
        );
    }

    #[test]
    fn revoked_paused_and_terminal_precedence_is_explicit() {
        let mut session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let grant = grant(&session, &plan, now);
        let admission = admission(&session, &plan, &grant, now);
        let revoked = derive_admission_eligibility(
            &session,
            Some(&plan),
            vec![admission.clone()],
            &[grant_view(&grant, "revoked")],
            now + Duration::seconds(901),
        )
        .expect("derive revoked eligibility");
        assert_state(
            &revoked,
            WorkbenchAdmissionEligibilityState::Revoked,
            WorkbenchAdmissionEligibilityReason::GrantRevoked,
        );

        session
            .transition(WorkbenchSessionAction::Pause)
            .expect("pause session");
        let paused =
            derive_admission_eligibility(&session, None, vec![admission.clone()], &[], now)
                .expect("derive paused eligibility");
        assert_state(
            &paused,
            WorkbenchAdmissionEligibilityState::SessionPaused,
            WorkbenchAdmissionEligibilityReason::SessionPaused,
        );

        session
            .transition(WorkbenchSessionAction::Resume)
            .expect("resume session");
        session
            .transition(WorkbenchSessionAction::Complete)
            .expect("complete session");
        let terminal = derive_admission_eligibility(&session, None, vec![admission], &[], now)
            .expect("derive terminal eligibility");
        assert_state(
            &terminal,
            WorkbenchAdmissionEligibilityState::SessionTerminal,
            WorkbenchAdmissionEligibilityReason::SessionCompleted,
        );
    }

    #[test]
    fn corrupt_cross_ledger_binding_rejects_the_entire_snapshot() {
        let session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let grant = grant(&session, &plan, now);
        let admission = admission(&session, &plan, &grant, now);
        let mut corrupt = grant_view(&grant, "active");
        corrupt.plan_id = "run-plan:ffffffffffffffffffffffffffffffff".into();
        let error =
            derive_admission_eligibility(&session, Some(&plan), vec![admission], &[corrupt], now)
                .expect_err("misbound grant must fail closed");
        assert!(error.to_string().contains("binding is corrupt"));
    }
}
