//! Local-only, non-autonomous Workbench kernel.
//!
//! It stores no prompts, messages, outputs, paths, credentials, headers, or
//! tool arguments. Its commands create inspectable session/run plans only;
//! execution is deliberately disabled until a later, separately gated phase.

mod adapter_readiness;
mod events;
mod presets;
mod run_contract;
mod session;
mod storage;

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use adapter_readiness::all_adapter_readiness;
pub use adapter_readiness::{WorkbenchAdapterCommandReadiness, WorkbenchAdapterReadiness};
pub use events::WorkbenchSessionAction;
use presets::{all_workbench_plan_presets, WorkbenchPlanPreset};
pub use run_contract::{WorkbenchRunPlan, WorkbenchRunSpecInput};
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
    store
        .transition(input.session_id.trim(), input.action)
        .map_err(|error| error.to_string())
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
    })
}

#[cfg(test)]
mod tests {
    use super::get_workbench_capability_projection;

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
}
