//! Testable orchestration for the non-executing process-admission command.
//!
//! The production command supplies native stores, plan preparation, and the
//! canonical adapter verifier. Tests inject deterministic equivalents so the
//! verified-routing prerequisite can be proven without a developer machine's
//! Codex installation or configuration.

use chrono::{DateTime, Utc};
use switchboard_runtime::RuntimeClock;

use crate::client_adapter_contract::VerificationReport;

use super::capability_grant::WorkbenchProcessStartGrant;
use super::events::{validate_identifier, WorkbenchSessionStatus};
use super::process_supervisor::admit_process;
use super::runtime_time::utc_from_runtime_clock;
use super::{
    ProcessRunSpec, WorkbenchProcessAdmission, WorkbenchProcessAdmissionInput, WorkbenchRunPlan,
    WorkbenchRunSpecInput, WorkbenchSession,
};

pub(super) fn admit_workbench_process_with_dependencies<
    PreparePlan,
    Now,
    RequireGrant,
    VerifyRouting,
    PersistAdmission,
>(
    session: &WorkbenchSession,
    input: WorkbenchProcessAdmissionInput,
    prepare_plan: PreparePlan,
    now: Now,
    require_grant: RequireGrant,
    verify_routing: VerifyRouting,
    persist_admission: PersistAdmission,
) -> Result<WorkbenchProcessAdmission, String>
where
    PreparePlan:
        FnOnce(&WorkbenchSession, WorkbenchRunSpecInput) -> Result<WorkbenchRunPlan, String>,
    Now: FnOnce() -> DateTime<Utc>,
    RequireGrant:
        FnOnce(&str, &str, &str, &str, DateTime<Utc>) -> Result<WorkbenchProcessStartGrant, String>,
    VerifyRouting: FnOnce(&ProcessRunSpec) -> Result<VerificationReport, String>,
    PersistAdmission:
        FnOnce(WorkbenchProcessAdmission) -> Result<WorkbenchProcessAdmission, String>,
{
    admit_workbench_process_with_fallible_now(
        session,
        input,
        prepare_plan,
        || Ok(now()),
        require_grant,
        verify_routing,
        persist_admission,
    )
}

pub(super) fn admit_workbench_process_with_clock<
    C,
    PreparePlan,
    RequireGrant,
    VerifyRouting,
    PersistAdmission,
>(
    clock: &C,
    session: &WorkbenchSession,
    input: WorkbenchProcessAdmissionInput,
    prepare_plan: PreparePlan,
    require_grant: RequireGrant,
    verify_routing: VerifyRouting,
    persist_admission: PersistAdmission,
) -> Result<WorkbenchProcessAdmission, String>
where
    C: RuntimeClock + ?Sized,
    PreparePlan:
        FnOnce(&WorkbenchSession, WorkbenchRunSpecInput) -> Result<WorkbenchRunPlan, String>,
    RequireGrant:
        FnOnce(&str, &str, &str, &str, DateTime<Utc>) -> Result<WorkbenchProcessStartGrant, String>,
    VerifyRouting: FnOnce(&ProcessRunSpec) -> Result<VerificationReport, String>,
    PersistAdmission:
        FnOnce(WorkbenchProcessAdmission) -> Result<WorkbenchProcessAdmission, String>,
{
    admit_workbench_process_with_fallible_now(
        session,
        input,
        prepare_plan,
        || utc_from_runtime_clock(clock).map_err(|error| error.to_string()),
        require_grant,
        verify_routing,
        persist_admission,
    )
}

fn admit_workbench_process_with_fallible_now<
    PreparePlan,
    Now,
    RequireGrant,
    VerifyRouting,
    PersistAdmission,
>(
    session: &WorkbenchSession,
    input: WorkbenchProcessAdmissionInput,
    prepare_plan: PreparePlan,
    now: Now,
    require_grant: RequireGrant,
    verify_routing: VerifyRouting,
    persist_admission: PersistAdmission,
) -> Result<WorkbenchProcessAdmission, String>
where
    PreparePlan:
        FnOnce(&WorkbenchSession, WorkbenchRunSpecInput) -> Result<WorkbenchRunPlan, String>,
    Now: FnOnce() -> Result<DateTime<Utc>, String>,
    RequireGrant:
        FnOnce(&str, &str, &str, &str, DateTime<Utc>) -> Result<WorkbenchProcessStartGrant, String>,
    VerifyRouting: FnOnce(&ProcessRunSpec) -> Result<VerificationReport, String>,
    PersistAdmission:
        FnOnce(WorkbenchProcessAdmission) -> Result<WorkbenchProcessAdmission, String>,
{
    if session.status != WorkbenchSessionStatus::Active {
        return Err("Workbench process admission requires an active session".into());
    }
    let plan = prepare_plan(session, input.run_spec)?;
    validate_identifier(&input.expected_plan_id, "plan ID").map_err(|error| error.to_string())?;
    validate_identifier(&input.expected_process_run_id, "process run ID")
        .map_err(|error| error.to_string())?;
    validate_identifier(&input.grant_id, "process grant ID").map_err(|error| error.to_string())?;
    let process = plan
        .process_containment
        .as_ref()
        .ok_or_else(|| "Workbench process admission requires native containment".to_string())?;
    if input.expected_plan_id != plan.plan_id || input.expected_process_run_id != process.run_id {
        return Err(
            "Workbench process admission no longer matches the prepared native plan".into(),
        );
    }
    if plan.adapter_id != "codex" {
        return Err("Workbench process admission is currently limited to canonical Codex".into());
    }
    let now = now()?;
    let grant = require_grant(
        &input.grant_id,
        &session.session_id,
        &plan.plan_id,
        &process.run_id,
        now,
    )?;
    let verification = verify_routing(process)?;
    if !verification.verified {
        return Err("Workbench process admission requires verified existing Codex routing".into());
    }
    let admission =
        admit_process(session, &plan, process, &grant, now).map_err(|error| error.to_string())?;
    persist_admission(admission)
}
