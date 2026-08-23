use crate::models::SwitchboardMode;
use crate::workbench_kernel::adapter_readiness::WorkbenchAdapterCommandReadiness;
use crate::workbench_kernel::capability_grant::{
    issue_process_start_grant, process_start_confirmation_phrase, WorkbenchProcessGrantStore,
    WorkbenchProcessStartGrant,
};
use crate::workbench_kernel::process_controller::WorkbenchFakeProcessController;
use crate::workbench_kernel::process_run_spec::{process_run_spec_for, ProcessRunSpec};
use crate::workbench_kernel::process_supervisor::{admit_process, WorkbenchProcessAdmission};
use crate::workbench_kernel::session::{CreateWorkbenchSessionInput, WorkbenchSession};
use crate::workbench_kernel::{CapabilityRequest, RouterDecisionReference, WorkbenchRunPlan};
use chrono::{Duration, TimeZone, Utc};
use serde_json::Value;
use std::path::Path;

pub(super) const TEST_OWNER_EPOCH: &str = "fake-controller-epoch:test-launch";

pub(super) fn fixture(
    seed: usize,
) -> (
    WorkbenchSession,
    ProcessRunSpec,
    WorkbenchProcessAdmission,
    WorkbenchProcessStartGrant,
    chrono::DateTime<Utc>,
) {
    let workspace_digest = format!("sha256:{:064x}", seed + 1);
    let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
        workspace_digest: workspace_digest.clone(),
        task_class: "coding".into(),
    })
    .expect("create Workbench session");
    let adapter_plan_id = format!("codex-{seed:012x}");
    let process = process_run_spec_for(
        &session.session_id,
        &adapter_plan_id,
        "codex",
        &workspace_digest,
    )
    .expect("create process run spec");
    let plan = WorkbenchRunPlan {
        schema_version: 1,
        plan_id: format!("run-plan:{seed:032x}"),
        session_id: session.session_id.clone(),
        adapter_id: "codex".into(),
        workspace_digest: workspace_digest.clone(),
        context_pack_digest: None,
        router_decision: RouterDecisionReference {
            decision_id: format!("routing-decision-{seed}"),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: format!("sha256:{:064x}", seed + 10_000),
        },
        replay_reference: None,
        preset: None,
        requested_mode: SwitchboardMode::Off,
        adapter_plan_id: adapter_plan_id.clone(),
        adapter_action: "cleanup_managed_routing".into(),
        adapter_reversible: true,
        command_readiness: Some(WorkbenchAdapterCommandReadiness {
            schema_version: 1,
            adapter_id: "codex".into(),
            adapter_contract_version:
                crate::client_adapter_contract::CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
            adapter_plan_id,
            logical_binary: "codex".into(),
            known_candidate_present: false,
            discovery_mode: "fixed_known_location_metadata_only".into(),
            cli_version_probe_state: "not_probed".into(),
            version_probe_reason: "test fixture".into(),
            process_start_enabled: false,
            provider_traffic: "none".into(),
            writes_enabled: false,
        }),
        process_containment: Some(process.clone()),
        capability_requests: vec![CapabilityRequest {
            capability_id: "adapter_command_readiness".into(),
            scope: "session".into(),
            approval_state: "pending".into(),
            execution_enabled: false,
        }],
        execution_mode: "plan_only".into(),
        provider_traffic: "none".into(),
        writes_enabled: false,
    };
    let now = Utc.with_ymd_and_hms(2026, 8, 24, 0, 0, 0).unwrap() + Duration::seconds(seed as i64);
    let grant = issue_process_start_grant(
        &session,
        &plan,
        &process_start_confirmation_phrase(&plan),
        now,
    )
    .expect("issue process grant");
    let admission =
        admit_process(&session, &plan, &process, &grant, now).expect("create process admission");
    (session, process, admission, grant, now)
}

pub(super) fn open_controller(path: &Path) -> WorkbenchFakeProcessController {
    open_controller_for_epoch(path, TEST_OWNER_EPOCH)
}

pub(super) fn open_controller_for_epoch(
    path: &Path,
    owner_epoch: &str,
) -> WorkbenchFakeProcessController {
    WorkbenchFakeProcessController::open(path.to_path_buf(), owner_epoch)
        .expect("open fake controller")
}

pub(super) fn grant_store(
    path: &Path,
    grants: &[&WorkbenchProcessStartGrant],
) -> WorkbenchProcessGrantStore {
    let store = WorkbenchProcessGrantStore::at(path.to_path_buf());
    for grant in grants {
        let issued_at = chrono::DateTime::parse_from_rfc3339(&grant.issued_at)
            .expect("parse process grant issue time")
            .with_timezone(&Utc);
        store
            .issue((*grant).clone(), issued_at)
            .expect("persist process grant");
    }
    store
}

pub(super) fn assert_no_sensitive_keys(value: &Value) {
    const FORBIDDEN: [&str; 10] = [
        "prompt",
        "tool",
        "toolOutput",
        "command",
        "argv",
        "environment",
        "env",
        "credential",
        "headers",
        "output",
    ];
    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(!FORBIDDEN.contains(&key.as_str()), "forbidden key {key}");
            }
            for child in object.values() {
                assert_no_sensitive_keys(child);
            }
        }
        Value::Array(values) => {
            for child in values {
                assert_no_sensitive_keys(child);
            }
        }
        _ => {}
    }
}
