//! Local-only, non-autonomous Workbench kernel.
//!
//! It stores no prompts, messages, outputs, paths, credentials, headers, or
//! tool arguments. Its commands create inspectable session/run plans only;
//! execution is deliberately disabled until a later, separately gated phase.

mod adapter_readiness;
mod capability_grant;
mod events;
mod presets;
pub(crate) mod process_controller;
mod process_eligibility;
mod process_run_spec;
mod process_supervisor;
mod run_contract;
mod session;
mod storage;

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use adapter_readiness::all_adapter_readiness;
pub use adapter_readiness::{WorkbenchAdapterCommandReadiness, WorkbenchAdapterReadiness};
use capability_grant::{
    issue_process_start_grant, process_start_grant_policy, WorkbenchProcessGrantStore,
};
pub use capability_grant::{WorkbenchProcessStartGrantPolicy, WorkbenchProcessStartGrantView};
pub use events::WorkbenchSessionAction;
use presets::{all_workbench_plan_presets, WorkbenchPlanPreset};
use process_eligibility::derive_admission_eligibility;
pub use process_eligibility::{
    WorkbenchAdmissionEligibilityInput, WorkbenchAdmissionEligibilitySnapshot,
};
pub use process_run_spec::ProcessRunSpec;
pub use process_supervisor::WorkbenchProcessAdmission;
use process_supervisor::{admit_process, WorkbenchProcessAdmissionStore};
pub use run_contract::{
    CapabilityRequest, RouterDecisionReference, WorkbenchRunPlan, WorkbenchRunSpecInput,
};
pub use session::{CreateWorkbenchSessionInput, WorkbenchSession};
use storage::WorkbenchStore;

static WORKBENCH_STORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchTransitionInput {
    pub session_id: String,
    pub action: WorkbenchSessionAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchForkInput {
    pub session_id: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchProcessStartGrantInput {
    pub run_spec: WorkbenchRunSpecInput,
    pub expected_plan_id: String,
    pub expected_process_run_id: String,
    pub confirmation_phrase: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchProcessAdmissionInput {
    pub run_spec: WorkbenchRunSpecInput,
    pub expected_plan_id: String,
    pub expected_process_run_id: String,
    pub grant_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCapabilityProjection {
    pub schema_version: u32,
    pub execution_mode: String,
    pub writes_enabled: bool,
    pub provider_traffic: String,
    pub registry: crate::oss_capabilities::OssCapabilityRegistry,
    pub presets: Vec<WorkbenchPlanPreset>,
    pub adapter_readiness: Vec<WorkbenchAdapterReadiness>,
    pub process_start_grant_policy: WorkbenchProcessStartGrantPolicy,
}

fn locked_store() -> Result<(std::sync::MutexGuard<'static, ()>, WorkbenchStore), String> {
    let guard = WORKBENCH_STORE_LOCK
        .lock()
        .map_err(|_| "Workbench session ledger lock is unavailable".to_string())?;
    Ok((guard, WorkbenchStore::in_app_storage()))
}

#[tauri::command]
pub fn create_workbench_session(
    input: CreateWorkbenchSessionInput,
) -> Result<WorkbenchSession, String> {
    let (_guard, store) = locked_store()?;
    store.create(input).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_workbench_sessions() -> Result<Vec<WorkbenchSession>, String> {
    let (_guard, store) = locked_store()?;
    store.list().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_workbench_session(session_id: String) -> Result<WorkbenchSession, String> {
    let (_guard, store) = locked_store()?;
    store
        .get(session_id.trim())
        .map_err(|error| error.to_string())
}

/// Exports only the validated content-free session ledger. This is an alias of
/// inspection rather than a file write so callers decide where data goes.
#[tauri::command]
pub fn export_workbench_session(session_id: String) -> Result<WorkbenchSession, String> {
    let (_guard, store) = locked_store()?;
    store
        .get(session_id.trim())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn transition_workbench_session(
    input: WorkbenchTransitionInput,
) -> Result<WorkbenchSession, String> {
    let (_guard, store) = locked_store()?;
    transition_workbench_session_with_cleanup(&store, input, |session_id| {
        WorkbenchProcessGrantStore::in_app_storage()
            .revoke_for_terminal_session(session_id, chrono::Utc::now())
            .map_err(|error| error.to_string())
    })
}

fn transition_workbench_session_with_cleanup<F>(
    store: &WorkbenchStore,
    input: WorkbenchTransitionInput,
    cleanup: F,
) -> Result<WorkbenchSession, String>
where
    F: FnOnce(&str) -> Result<(), String>,
{
    let session = store
        .transition(input.session_id.trim(), input.action)
        .map_err(|error| error.to_string())?;
    if matches!(
        input.action,
        WorkbenchSessionAction::Cancel | WorkbenchSessionAction::Complete
    ) {
        if let Err(error) = cleanup(&session.session_id) {
            log::warn!("Workbench terminal session persisted but grant cleanup failed: {error}");
        }
    }
    Ok(session)
}

#[tauri::command]
pub fn fork_workbench_session(input: WorkbenchForkInput) -> Result<WorkbenchSession, String> {
    let (_guard, store) = locked_store()?;
    store
        .fork(input.session_id.trim(), input.event_id.trim())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn prepare_workbench_run_plan(
    input: WorkbenchRunSpecInput,
) -> Result<WorkbenchRunPlan, String> {
    let (_guard, store) = locked_store()?;
    let session = store
        .get(input.session_id.trim())
        .map_err(|error| error.to_string())?;
    run_contract::prepare_run_plan(&session, input).map_err(|error| error.to_string())
}

/// Stores an explicit, expiry-bound authorization receipt for one previously
/// prepared native process-containment plan. This does not start a process or
/// enable execution; a later executor must separately validate the receipt.
#[tauri::command]
pub fn issue_workbench_process_start_grant(
    input: WorkbenchProcessStartGrantInput,
) -> Result<WorkbenchProcessStartGrantView, String> {
    let (_guard, store) = locked_store()?;
    let session = store
        .get(input.run_spec.session_id.trim())
        .map_err(|error| error.to_string())?;
    let plan = run_contract::prepare_run_plan(&session, input.run_spec)
        .map_err(|error| error.to_string())?;
    events::validate_identifier(&input.expected_plan_id, "plan ID")
        .map_err(|error| error.to_string())?;
    events::validate_identifier(&input.expected_process_run_id, "process run ID")
        .map_err(|error| error.to_string())?;
    let process = plan
        .process_containment
        .as_ref()
        .ok_or_else(|| "Workbench process grant requires native containment".to_string())?;
    if input.expected_plan_id != plan.plan_id || input.expected_process_run_id != process.run_id {
        return Err("Workbench process grant no longer matches the prepared native plan".into());
    }
    let now = chrono::Utc::now();
    let grant = issue_process_start_grant(&session, &plan, &input.confirmation_phrase, now)
        .map_err(|error| error.to_string())?;
    WorkbenchProcessGrantStore::in_app_storage()
        .issue(grant, now)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_workbench_process_start_grants(
    session_id: String,
) -> Result<Vec<WorkbenchProcessStartGrantView>, String> {
    let (_guard, _) = locked_store()?;
    WorkbenchProcessGrantStore::in_app_storage()
        .list_for_session(session_id.trim(), chrono::Utc::now())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn revoke_workbench_process_start_grant(
    grant_id: String,
) -> Result<WorkbenchProcessStartGrantView, String> {
    let (_guard, _) = locked_store()?;
    WorkbenchProcessGrantStore::in_app_storage()
        .revoke(grant_id.trim(), chrono::Utc::now())
        .map_err(|error| error.to_string())
}

/// Records an executor-admission receipt for a verified Codex adapter. This
/// deliberately does not resolve or launch a binary, create a child process,
/// apply configuration, or enable execution.
#[tauri::command]
pub fn admit_workbench_process(
    input: WorkbenchProcessAdmissionInput,
) -> Result<WorkbenchProcessAdmission, String> {
    let (_guard, store) = locked_store()?;
    let session = store
        .get(input.run_spec.session_id.trim())
        .map_err(|error| error.to_string())?;
    if session.status != events::WorkbenchSessionStatus::Active {
        return Err("Workbench process admission requires an active session".into());
    }
    let plan = run_contract::prepare_run_plan(&session, input.run_spec)
        .map_err(|error| error.to_string())?;
    events::validate_identifier(&input.expected_plan_id, "plan ID")
        .map_err(|error| error.to_string())?;
    events::validate_identifier(&input.expected_process_run_id, "process run ID")
        .map_err(|error| error.to_string())?;
    events::validate_identifier(&input.grant_id, "process grant ID")
        .map_err(|error| error.to_string())?;
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
    let now = chrono::Utc::now();
    let grant = WorkbenchProcessGrantStore::in_app_storage()
        .require_active_for(
            &input.grant_id,
            &session.session_id,
            &plan.plan_id,
            &process.run_id,
            now,
        )
        .map_err(|error| error.to_string())?;
    let adapter = crate::client_adapter_contract::coding_client_adapter_for_version(
        "codex",
        process.adapter_contract_version,
    )
    .map_err(|error| error.to_string())?;
    let verification = adapter.verify().map_err(|error| error.to_string())?;
    if !verification.verified {
        return Err("Workbench process admission requires verified existing Codex routing".into());
    }
    let admission =
        admit_process(&session, &plan, process, &grant, now).map_err(|error| error.to_string())?;
    WorkbenchProcessAdmissionStore::in_app_storage()
        .issue(admission)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_workbench_process_admissions(
    session_id: String,
) -> Result<Vec<WorkbenchProcessAdmission>, String> {
    let (_guard, _) = locked_store()?;
    WorkbenchProcessAdmissionStore::in_app_storage()
        .list_for_session(session_id.trim())
        .map_err(|error| error.to_string())
}

/// Re-evaluates historical admissions against the current session, a freshly
/// prepared native plan, and the validated grant ledger. The snapshot is
/// ephemeral and remains non-executable.
#[tauri::command]
pub fn derive_workbench_process_admission_eligibility(
    input: WorkbenchAdmissionEligibilityInput,
) -> Result<WorkbenchAdmissionEligibilitySnapshot, String> {
    let (_guard, store) = locked_store()?;
    let session = store
        .get(input.run_spec.session_id.trim())
        .map_err(|error| error.to_string())?;
    let now = chrono::Utc::now();
    let admissions = WorkbenchProcessAdmissionStore::in_app_storage()
        .list_for_session(&session.session_id)
        .map_err(|error| error.to_string())?;
    if session.status == events::WorkbenchSessionStatus::Active {
        let plan = run_contract::prepare_run_plan(&session, input.run_spec)
            .map_err(|error| error.to_string())?;
        let grants = WorkbenchProcessGrantStore::in_app_storage()
            .snapshot(now)
            .map_err(|error| error.to_string())?;
        derive_admission_eligibility(&session, Some(&plan), admissions, &grants, now)
            .map_err(|error| error.to_string())
    } else {
        derive_admission_eligibility(&session, None, admissions, &[], now)
            .map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub fn get_workbench_capability_projection() -> Result<WorkbenchCapabilityProjection, String> {
    Ok(WorkbenchCapabilityProjection {
        schema_version: 1,
        execution_mode: "plan_only".into(),
        writes_enabled: false,
        provider_traffic: "none".into(),
        registry: crate::oss_capabilities::registry(),
        presets: all_workbench_plan_presets(),
        adapter_readiness: all_adapter_readiness(),
        process_start_grant_policy: process_start_grant_policy(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        get_workbench_capability_projection, transition_workbench_session_with_cleanup,
        CreateWorkbenchSessionInput, WorkbenchSessionAction, WorkbenchTransitionInput,
    };
    use crate::workbench_kernel::events::WorkbenchSessionStatus;
    use crate::workbench_kernel::storage::WorkbenchStore;
    use std::cell::Cell;

    #[test]
    fn capability_projection_reuses_the_native_oss_registry_exactly() {
        let projection = get_workbench_capability_projection().expect("capability projection");
        assert_eq!(projection.registry, crate::oss_capabilities::registry());
        assert_eq!(projection.execution_mode, "plan_only");
        assert_eq!(projection.provider_traffic, "none");
        assert!(!projection.writes_enabled);
        assert!(projection.presets.iter().all(|preset| {
            crate::workbench_kernel::presets::validate_workbench_plan_preset(preset).is_ok()
        }));
        assert!(projection.adapter_readiness.iter().all(|readiness| {
            readiness.cli_version_probe_state == "not_probed"
                && !readiness.process_start_enabled
                && readiness.provider_traffic == "none"
                && !readiness.writes_enabled
        }));
    }

    #[test]
    fn terminal_transition_remains_authoritative_when_grant_cleanup_fails() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = WorkbenchStore::at(directory.path().join("sessions.json"));
        let session = store
            .create(CreateWorkbenchSessionInput {
                workspace_digest: format!("sha256:{}", "c".repeat(64)),
                task_class: "coding".into(),
            })
            .expect("create session");
        let cleanup_called = Cell::new(false);
        let terminal = transition_workbench_session_with_cleanup(
            &store,
            WorkbenchTransitionInput {
                session_id: session.session_id.clone(),
                action: WorkbenchSessionAction::Cancel,
            },
            |_| {
                cleanup_called.set(true);
                Err("simulated cleanup failure".into())
            },
        )
        .expect("persisted terminal transition wins over cleanup failure");
        assert!(cleanup_called.get());
        assert_eq!(terminal.status, WorkbenchSessionStatus::Cancelled);
        assert_eq!(
            store
                .get(&session.session_id)
                .expect("reload terminal session")
                .status,
            WorkbenchSessionStatus::Cancelled
        );
    }
}
