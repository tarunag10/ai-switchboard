//! Atomic production publication for declarative Workbench run plans.
//!
//! Publication returns the existing content-free plan projection, but first
//! commits its complete private current-plan head while the canonical authority
//! transaction is held. It grants no capability and performs no process,
//! network, provider, workspace, or user-file operation.

use super::capability_grant::WorkbenchProcessGrantStore;
use super::storage::run_plan_head::WorkbenchPlanHeadStore;
use super::storage::WorkbenchStore;
use super::{WorkbenchRunPlan, WorkbenchRunSpecInput, WorkbenchSession};

pub(super) fn prepare_and_publish_workbench_run_plan<PreparePlan>(
    session_store: &WorkbenchStore,
    grant_store: &WorkbenchProcessGrantStore,
    plan_head_store: &WorkbenchPlanHeadStore,
    input: WorkbenchRunSpecInput,
    prepare_plan: PreparePlan,
) -> Result<WorkbenchRunPlan, String>
where
    PreparePlan:
        FnOnce(&WorkbenchSession, WorkbenchRunSpecInput) -> Result<WorkbenchRunPlan, String>,
{
    let transaction = grant_store
        .begin_authority_transaction()
        .map_err(|error| error.to_string())?;
    let session = session_store
        .get_for_authority_transaction(&transaction, input.session_id.trim())
        .map_err(|error| error.to_string())?;
    let plan = prepare_plan(&session, input)?;
    plan_head_store
        .publish_for_authority_transaction(&transaction, session_store, &session, &plan)
        .map_err(|error| error.to_string())?;
    Ok(plan)
}
