use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::client_adapter_contract::{
    coding_client_adapter_for_version, ConfigPlanAction, CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
};
use crate::models::SwitchboardMode;
use crate::oss_harness_replay::{
    resolve_oss_harness_replay_reference_for_workbench, validate_replay_reference,
    OssHarnessReplayReference,
};

use super::events::validate_identifier;
use super::presets::{
    resolve_workbench_plan_preset, validate_workbench_plan_preset, WorkbenchPlanPreset,
};
use super::process_run_spec::process_run_spec_for;
use super::session::{validate_digest, WorkbenchSession};
use super::{
    adapter_readiness::{
        command_readiness_for, validate_adapter_command_readiness,
        validate_adapter_command_readiness_adapter_id, ADAPTER_COMMAND_READINESS_CAPABILITY_ID,
    },
    WorkbenchAdapterCommandReadiness,
};

const RUN_SPEC_SCHEMA_VERSION: u32 = 1;
const PLAN_ONLY: &str = "plan_only";
const OBSERVE_ONLY: &str = "observe_only";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchRunSpecInput {
    pub session_id: String,
    pub adapter_id: String,
    pub workspace_digest: String,
    pub context_pack_digest: Option<String>,
    /// The only caller-supplied Router field. It is resolved against the
    /// durable native completion ledger before a plan is created.
    pub router_decision_id: String,
    /// Optional only when the redacted replay capability is requested. The
    /// native receipt is resolved again before a plan is created.
    pub replay_reference_id: Option<String>,
    /// Optional native-owned template. When set, its capability composition
    /// must match exactly; callers cannot supply preset metadata.
    pub preset_id: Option<String>,
    pub required_capability_ids: Vec<String>,
    pub requested_mode: SwitchboardMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterDecisionReference {
    pub decision_id: String,
    pub decision_stage: String,
    pub routing_mode: String,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRequest {
    pub capability_id: String,
    pub scope: String,
    pub approval_state: String,
    pub execution_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchRunPlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub session_id: String,
    pub adapter_id: String,
    pub workspace_digest: String,
    pub context_pack_digest: Option<String>,
    pub router_decision: RouterDecisionReference,
    pub replay_reference: Option<OssHarnessReplayReference>,
    pub preset: Option<WorkbenchPlanPreset>,
    pub requested_mode: SwitchboardMode,
    pub adapter_plan_id: String,
    pub adapter_action: String,
    pub adapter_reversible: bool,
    pub command_readiness: Option<WorkbenchAdapterCommandReadiness>,
    pub process_containment: Option<super::ProcessRunSpec>,
    pub capability_requests: Vec<CapabilityRequest>,
    pub execution_mode: String,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

pub(crate) fn workbench_run_plan_identity(plan: &WorkbenchRunPlan) -> Result<String> {
    validate_workbench_run_plan_body(plan)?;
    let capability_ids = plan
        .capability_requests
        .iter()
        .map(|request| request.capability_id.as_str())
        .collect::<Vec<_>>();
    let canonical = serde_json::json!({
        "sessionId": &plan.session_id,
        "adapterId": &plan.adapter_id,
        "workspaceDigest": &plan.workspace_digest,
        "contextPackDigest": &plan.context_pack_digest,
        "routerDecision": &plan.router_decision,
        "replayReference": &plan.replay_reference,
        "preset": &plan.preset,
        "capabilityIds": capability_ids,
        "requestedMode": &plan.requested_mode,
        "adapterPlanId": &plan.adapter_plan_id,
        "commandReadiness": &plan.command_readiness,
        "processContainment": &plan.process_containment,
    });
    let digest = Sha256::digest(
        serde_json::to_vec(&canonical).context("canonicalizing Workbench run plan identity")?,
    );
    Ok(format!("run-plan:{digest:x}")[..41].to_string())
}

pub(crate) fn validate_workbench_run_plan(plan: &WorkbenchRunPlan) -> Result<()> {
    validate_identifier(&plan.plan_id, "plan ID")?;
    if plan.plan_id != workbench_run_plan_identity(plan)? {
        bail!("Workbench run plan identity does not match its complete native plan");
    }
    Ok(())
}

pub(crate) fn workbench_run_plan_snapshot_digest(plan: &WorkbenchRunPlan) -> Result<String> {
    validate_workbench_run_plan(plan)?;
    let bytes = serde_json::to_vec(plan).context("canonicalizing Workbench run plan snapshot")?;
    let mut hasher = Sha256::new();
    hasher.update(b"ai-switchboard-workbench-run-plan-snapshot-v1\0");
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn validate_workbench_run_plan_body(plan: &WorkbenchRunPlan) -> Result<()> {
    if plan.schema_version != RUN_SPEC_SCHEMA_VERSION
        || plan.execution_mode != PLAN_ONLY
        || plan.provider_traffic != "none"
        || plan.writes_enabled
    {
        bail!("Workbench run plan violates the plan-only boundary");
    }
    for (value, label) in [
        (&plan.session_id, "session ID"),
        (&plan.adapter_id, "adapter ID"),
        (&plan.adapter_plan_id, "adapter plan ID"),
    ] {
        validate_identifier(value, label)?;
    }
    validate_digest(&plan.workspace_digest, "workspace digest")?;
    if let Some(context_pack_digest) = &plan.context_pack_digest {
        validate_digest(context_pack_digest, "context pack digest")?;
    }
    validate_router_reference(&plan.router_decision)?;

    if plan.capability_requests.iter().any(|request| {
        request.scope != "session"
            || request.approval_state != "pending"
            || request.execution_enabled
    }) {
        bail!("Workbench run plan capability requests are not plan-only");
    }
    let capability_ids = plan
        .capability_requests
        .iter()
        .map(|request| request.capability_id.clone())
        .collect::<Vec<_>>();
    validate_capability_ids(&capability_ids)?;
    for required in ["router_observe", "client_adapter_plan"] {
        if !capability_ids.iter().any(|value| value == required) {
            bail!("Workbench run plan is missing required capability {required}");
        }
    }
    let has_capability = |value: &str| capability_ids.iter().any(|candidate| candidate == value);
    match (
        has_capability("repo_context"),
        plan.context_pack_digest.as_ref(),
    ) {
        (true, Some(_)) | (false, None) => {}
        _ => bail!("Workbench run plan context binding is invalid"),
    }
    match (
        has_capability("redacted_replay"),
        plan.replay_reference.as_ref(),
    ) {
        (true, Some(reference)) => validate_replay_reference(reference)?,
        (false, None) => {}
        _ => bail!("Workbench run plan replay binding is invalid"),
    }
    if let Some(preset) = &plan.preset {
        validate_workbench_plan_preset(preset)?;
        if preset.required_capability_ids != capability_ids {
            bail!("Workbench run plan preset capabilities have drifted");
        }
    }

    let has_command_readiness = has_capability(ADAPTER_COMMAND_READINESS_CAPABILITY_ID);
    match (has_command_readiness, plan.command_readiness.as_ref()) {
        (true, Some(readiness)) => {
            validate_adapter_command_readiness(readiness)?;
            if readiness.adapter_id != plan.adapter_id
                || readiness.adapter_plan_id != plan.adapter_plan_id
            {
                bail!("Workbench run plan command readiness is misbound");
            }
        }
        (false, None) => {}
        _ => bail!("Workbench run plan command readiness binding is invalid"),
    }
    match (has_command_readiness, plan.process_containment.as_ref()) {
        (true, Some(process)) => {
            process.validate()?;
            let expected = process_run_spec_for(
                &plan.session_id,
                &plan.adapter_plan_id,
                &plan.adapter_id,
                &plan.workspace_digest,
            )?;
            if process != &expected {
                bail!("Workbench run plan process containment has drifted");
            }
        }
        (false, None) => {}
        _ => bail!("Workbench run plan process containment binding is invalid"),
    }

    let adapter = coding_client_adapter_for_version(
        &plan.adapter_id,
        CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
    )?;
    let adapter_plan = adapter.plan(plan.requested_mode.clone())?;
    let expected_action = match adapter_plan.action {
        ConfigPlanAction::ApplyManagedRouting => "apply_managed_routing",
        ConfigPlanAction::CleanupManagedRouting => "cleanup_managed_routing",
    };
    if adapter_plan.plan_id != plan.adapter_plan_id
        || expected_action != plan.adapter_action
        || adapter_plan.reversible != plan.adapter_reversible
    {
        bail!("Workbench run plan adapter output has drifted");
    }
    Ok(())
}

fn validate_router_reference(reference: &RouterDecisionReference) -> Result<()> {
    validate_identifier(&reference.decision_id, "router decision ID")?;
    validate_digest(&reference.evidence_digest, "router evidence digest")?;
    if !matches!(
        reference.decision_stage.as_str(),
        "observe" | "userApproved" | "automaticAllowlisted"
    ) {
        bail!("Workbench plan requires a Router decision with a known policy stage");
    }
    if reference.routing_mode != OBSERVE_ONLY {
        bail!("Workbench plan requires an observe-only Router decision reference");
    }
    Ok(())
}

fn resolved_router_reference(decision_id: &str) -> Result<RouterDecisionReference> {
    validate_identifier(decision_id, "router decision ID")?;
    let reference =
        crate::optimization::telemetry_store::resolve_model_routing_decision_reference(decision_id)
            .map_err(|error| anyhow!("Workbench Router decision could not be resolved: {error}"))?;
    let resolved = RouterDecisionReference {
        decision_id: reference.decision_id,
        decision_stage: reference.decision_stage,
        routing_mode: reference.routing_mode,
        evidence_digest: reference.evidence_digest,
    };
    validate_router_reference(&resolved)?;
    Ok(resolved)
}

fn resolved_replay_reference(replay_id: &str) -> Result<OssHarnessReplayReference> {
    let reference = resolve_oss_harness_replay_reference_for_workbench(replay_id.trim())
        .map_err(|error| anyhow!("Workbench redacted replay could not be resolved: {error}"))?;
    validate_replay_reference(&reference)
        .map_err(|error| anyhow!("Workbench redacted replay could not be validated: {error}"))?;
    Ok(reference)
}

fn validate_capability_ids(ids: &[String]) -> Result<()> {
    if ids.len() > 10 {
        bail!("Workbench run plan supports at most ten capability requests");
    }
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        validate_identifier(id, "capability ID")?;
        if !matches!(
            id.as_str(),
            "repo_context"
                | "redacted_replay"
                | "router_observe"
                | "client_adapter_plan"
                | ADAPTER_COMMAND_READINESS_CAPABILITY_ID
        ) {
            bail!("Workbench capability is not available in plan-only mode: {id}");
        }
        if !seen.insert(id) {
            bail!("Workbench run plan contains a duplicate capability request");
        }
    }
    Ok(())
}

fn prepare_run_plan_with_reference(
    session: &WorkbenchSession,
    input: WorkbenchRunSpecInput,
    router_decision: RouterDecisionReference,
    replay_reference: Option<OssHarnessReplayReference>,
    preset: Option<WorkbenchPlanPreset>,
) -> Result<WorkbenchRunPlan> {
    if input.session_id != session.session_id {
        return Err(anyhow!("Workbench run spec belongs to another session"));
    }
    if input.workspace_digest != session.workspace_digest {
        return Err(anyhow!(
            "Workbench run spec workspace does not match its session"
        ));
    }
    validate_identifier(&input.adapter_id, "adapter ID")?;
    validate_digest(&input.workspace_digest, "workspace digest")?;
    if let Some(context_pack_digest) = &input.context_pack_digest {
        validate_digest(context_pack_digest, "context pack digest")?;
    }
    validate_router_reference(&router_decision)?;
    validate_capability_ids(&input.required_capability_ids)?;
    let requests_router_observe = input
        .required_capability_ids
        .iter()
        .any(|capability_id| capability_id == "router_observe");
    if !requests_router_observe {
        bail!(
            "Workbench plans require the router observe capability for their native Router receipt"
        );
    }
    let requests_adapter_plan = input
        .required_capability_ids
        .iter()
        .any(|capability_id| capability_id == "client_adapter_plan");
    if !requests_adapter_plan {
        bail!("Workbench plans require the client adapter plan capability before adapter planning");
    }
    let requests_adapter_command_readiness = input
        .required_capability_ids
        .iter()
        .any(|capability_id| capability_id == ADAPTER_COMMAND_READINESS_CAPABILITY_ID);
    if requests_adapter_command_readiness {
        validate_adapter_command_readiness_adapter_id(&input.adapter_id)?;
    }
    let requests_repo_context = input
        .required_capability_ids
        .iter()
        .any(|capability_id| capability_id == "repo_context");
    match (requests_repo_context, input.context_pack_digest.as_ref()) {
        (true, Some(_)) | (false, None) => {}
        (true, None) => bail!("Workbench repo context capability requires a context pack digest"),
        (false, Some(_)) => {
            bail!("Workbench context pack digest requires the repo context capability")
        }
    }
    match (input.preset_id.as_deref(), preset.as_ref()) {
        (Some(preset_id), Some(resolved)) if preset_id == resolved.preset_id => {
            validate_workbench_plan_preset(resolved)?;
            if resolved.required_capability_ids != input.required_capability_ids {
                bail!("Workbench preset capabilities must match the native preset exactly");
            }
        }
        (None, None) => {}
        _ => bail!("Workbench preset must be resolved from its native preset ID"),
    }
    let requests_redacted_replay = input
        .required_capability_ids
        .iter()
        .any(|capability_id| capability_id == "redacted_replay");
    match (requests_redacted_replay, replay_reference.as_ref()) {
        (true, Some(reference)) => validate_replay_reference(reference)?,
        (true, None) => {
            bail!("Workbench redacted replay capability requires a native replay receipt")
        }
        (false, Some(_)) => {
            bail!("Workbench replay receipt requires the redacted replay capability")
        }
        (false, None) => {}
    }
    let adapter = coding_client_adapter_for_version(
        &input.adapter_id,
        CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
    )?;
    let adapter_plan = adapter.plan(input.requested_mode.clone())?;
    let command_readiness = if requests_adapter_command_readiness {
        Some(command_readiness_for(
            &input.adapter_id,
            &adapter_plan.plan_id,
        )?)
    } else {
        None
    };
    let process_containment = if command_readiness.is_some() {
        Some(process_run_spec_for(
            &input.session_id,
            &adapter_plan.plan_id,
            &input.adapter_id,
            &input.workspace_digest,
        )?)
    } else {
        None
    };
    let canonical = serde_json::json!({
        "sessionId": &input.session_id,
        "adapterId": adapter.id(),
        "workspaceDigest": &input.workspace_digest,
        "contextPackDigest": &input.context_pack_digest,
        "routerDecision": &router_decision,
        "replayReference": &replay_reference,
        "preset": &preset,
        "capabilityIds": &input.required_capability_ids,
        "requestedMode": &input.requested_mode,
        "adapterPlanId": &adapter_plan.plan_id,
        "commandReadiness": &command_readiness,
        "processContainment": &process_containment,
    });
    let digest = Sha256::digest(
        serde_json::to_vec(&canonical).context("canonicalizing Workbench run plan")?,
    );
    Ok(WorkbenchRunPlan {
        schema_version: RUN_SPEC_SCHEMA_VERSION,
        plan_id: format!("run-plan:{:x}", digest)[..41].to_string(),
        session_id: input.session_id,
        adapter_id: adapter.id().to_string(),
        workspace_digest: input.workspace_digest,
        context_pack_digest: input.context_pack_digest,
        router_decision,
        replay_reference,
        preset,
        requested_mode: input.requested_mode,
        adapter_plan_id: adapter_plan.plan_id,
        adapter_action: match adapter_plan.action {
            ConfigPlanAction::ApplyManagedRouting => "apply_managed_routing".into(),
            ConfigPlanAction::CleanupManagedRouting => "cleanup_managed_routing".into(),
        },
        adapter_reversible: adapter_plan.reversible,
        command_readiness,
        process_containment,
        capability_requests: input
            .required_capability_ids
            .into_iter()
            .map(|capability_id| CapabilityRequest {
                capability_id,
                scope: "session".into(),
                approval_state: "pending".into(),
                execution_enabled: false,
            })
            .collect(),
        execution_mode: PLAN_ONLY.into(),
        provider_traffic: "none".into(),
        writes_enabled: false,
    })
}

pub(crate) fn prepare_run_plan(
    session: &WorkbenchSession,
    input: WorkbenchRunSpecInput,
) -> Result<WorkbenchRunPlan> {
    let router_decision = resolved_router_reference(&input.router_decision_id)?;
    let replay_reference = match input.replay_reference_id.as_deref() {
        Some(replay_id) => Some(resolved_replay_reference(replay_id)?),
        None => None,
    };
    let preset = match input.preset_id.as_deref() {
        Some(preset_id) => Some(resolve_workbench_plan_preset(preset_id)?),
        None => None,
    };
    prepare_run_plan_with_reference(session, input, router_decision, replay_reference, preset)
}

#[cfg(test)]
mod tests {
    use super::{
        prepare_run_plan, prepare_run_plan_with_reference, validate_workbench_run_plan,
        workbench_run_plan_snapshot_digest, RouterDecisionReference, WorkbenchRunSpecInput,
    };
    use crate::models::SwitchboardMode;
    use crate::workbench_kernel::session::{CreateWorkbenchSessionInput, WorkbenchSession};

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn replay_reference() -> crate::oss_harness_replay::OssHarnessReplayReference {
        let mut reference = crate::oss_harness_replay::OssHarnessReplayReference {
            schema_version: 1,
            replay_id: "replay-reference-00000000-0000-4000-8000-000000000001".into(),
            validated_at: "2026-08-23T00:00:00Z".into(),
            replay_mode: "redacted_observe_only".into(),
            automatic_promotion: "disabled".into(),
            provider_traffic: "none".into(),
            event_count: 2,
            replay_digest: digest('1'),
            receipt_digest: String::new(),
        };
        reference.receipt_digest = crate::oss_harness_replay::replay_reference_digest(&reference)
            .expect("create replay receipt digest");
        reference
    }

    #[test]
    fn run_plan_is_declarative_and_execution_disabled() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('c'),
            task_class: "coding".into(),
        })
        .expect("create session");
        let plan = prepare_run_plan_with_reference(
            &session,
            WorkbenchRunSpecInput {
                session_id: session.session_id.clone(),
                adapter_id: "codex".into(),
                workspace_digest: session.workspace_digest.clone(),
                context_pack_digest: Some(digest('d')),
                router_decision_id: "routing-decision-test-1".into(),
                replay_reference_id: None,
                preset_id: None,
                required_capability_ids: vec![
                    "repo_context".into(),
                    "router_observe".into(),
                    "client_adapter_plan".into(),
                ],
                requested_mode: SwitchboardMode::Headroom,
            },
            RouterDecisionReference {
                decision_id: "routing-decision-test-1".into(),
                decision_stage: "observe".into(),
                routing_mode: "observe_only".into(),
                evidence_digest: digest('e'),
            },
            None,
            None,
        )
        .expect("prepare plan");
        assert_eq!(plan.execution_mode, "plan_only");
        assert_eq!(plan.provider_traffic, "none");
        assert!(!plan.writes_enabled);
        assert_eq!(plan.command_readiness, None);
        assert_eq!(plan.process_containment, None);
        assert!(plan
            .capability_requests
            .iter()
            .all(|request| !request.execution_enabled));
        validate_workbench_run_plan(&plan).expect("validate complete native plan");
        assert!(workbench_run_plan_snapshot_digest(&plan)
            .expect("digest complete native plan")
            .starts_with("sha256:"));
        let mut changed = plan;
        changed.capability_requests[0].execution_enabled = true;
        assert!(validate_workbench_run_plan(&changed).is_err());
    }

    #[test]
    fn run_plan_rejects_non_observe_router_and_unknown_capability() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('f'),
            task_class: "planning".into(),
        })
        .expect("create session");
        let mut input = WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision_id: "routing-decision-test-2".into(),
            replay_reference_id: None,
            preset_id: None,
            required_capability_ids: vec!["router_observe".into()],
            requested_mode: SwitchboardMode::Off,
        };
        let mut reference = RouterDecisionReference {
            decision_id: "routing-decision-test-2".into(),
            decision_stage: "observe".into(),
            routing_mode: "automatic".into(),
            evidence_digest: digest('a'),
        };
        assert!(prepare_run_plan_with_reference(
            &session,
            input.clone(),
            reference.clone(),
            None,
            None
        )
        .is_err());
        reference.routing_mode = "observe_only".into();
        input.required_capability_ids = vec!["arbitrary_shell".into()];
        assert!(prepare_run_plan_with_reference(&session, input, reference, None, None).is_err());
    }

    #[test]
    fn run_plan_binds_a_verified_replay_only_when_requested() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('f'),
            task_class: "planning".into(),
        })
        .expect("create session");
        let input = WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision_id: "routing-decision-test-3".into(),
            replay_reference_id: Some(
                "replay-reference-00000000-0000-4000-8000-000000000001".into(),
            ),
            preset_id: None,
            required_capability_ids: vec![
                "router_observe".into(),
                "redacted_replay".into(),
                "client_adapter_plan".into(),
            ],
            requested_mode: SwitchboardMode::Off,
        };
        let router = RouterDecisionReference {
            decision_id: "routing-decision-test-3".into(),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: digest('2'),
        };
        let reference = replay_reference();
        let plan =
            prepare_run_plan_with_reference(&session, input, router, Some(reference.clone()), None)
                .expect("prepare plan with replay receipt");
        assert_eq!(plan.replay_reference, Some(reference));
        assert_eq!(plan.execution_mode, "plan_only");
        assert_eq!(plan.provider_traffic, "none");
        assert!(!plan.writes_enabled);
    }

    #[test]
    fn run_plan_rejects_replay_capability_and_receipt_mismatches() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('f'),
            task_class: "planning".into(),
        })
        .expect("create session");
        let router = RouterDecisionReference {
            decision_id: "routing-decision-test-4".into(),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: digest('2'),
        };
        let mut input = WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision_id: "routing-decision-test-4".into(),
            replay_reference_id: None,
            preset_id: None,
            required_capability_ids: vec![
                "router_observe".into(),
                "redacted_replay".into(),
                "client_adapter_plan".into(),
            ],
            requested_mode: SwitchboardMode::Off,
        };
        assert!(prepare_run_plan_with_reference(
            &session,
            input.clone(),
            router.clone(),
            None,
            None
        )
        .is_err());
        input.required_capability_ids = vec!["router_observe".into(), "client_adapter_plan".into()];
        assert!(prepare_run_plan_with_reference(
            &session,
            input,
            router,
            Some(replay_reference()),
            None
        )
        .is_err());
    }

    #[test]
    fn run_plan_requires_capabilities_for_each_existing_plan_input() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('f'),
            task_class: "planning".into(),
        })
        .expect("create session");
        let router = RouterDecisionReference {
            decision_id: "routing-decision-test-5".into(),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: digest('2'),
        };
        let mut input = WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision_id: "routing-decision-test-5".into(),
            replay_reference_id: None,
            preset_id: None,
            required_capability_ids: vec!["router_observe".into()],
            requested_mode: SwitchboardMode::Off,
        };
        assert!(prepare_run_plan_with_reference(
            &session,
            input.clone(),
            router.clone(),
            None,
            None
        )
        .is_err());
        input.required_capability_ids = vec!["client_adapter_plan".into()];
        assert!(prepare_run_plan_with_reference(
            &session,
            input.clone(),
            router.clone(),
            None,
            None
        )
        .is_err());
        input.required_capability_ids = vec![
            "router_observe".into(),
            "client_adapter_plan".into(),
            "repo_context".into(),
        ];
        assert!(prepare_run_plan_with_reference(
            &session,
            input.clone(),
            router.clone(),
            None,
            None
        )
        .is_err());
        input.required_capability_ids = vec!["router_observe".into(), "client_adapter_plan".into()];
        input.context_pack_digest = Some(digest('3'));
        assert!(prepare_run_plan_with_reference(&session, input, router, None, None).is_err());
    }

    #[test]
    fn command_readiness_is_bound_to_a_canonical_adapter_plan_only() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('f'),
            task_class: "planning".into(),
        })
        .expect("create session");
        let router = RouterDecisionReference {
            decision_id: "routing-decision-test-readiness".into(),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: digest('2'),
        };
        let mut input = WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision_id: "routing-decision-test-readiness".into(),
            replay_reference_id: None,
            preset_id: None,
            required_capability_ids: vec![
                "router_observe".into(),
                "client_adapter_plan".into(),
                "adapter_command_readiness".into(),
            ],
            requested_mode: SwitchboardMode::Off,
        };
        let plan =
            prepare_run_plan_with_reference(&session, input.clone(), router.clone(), None, None)
                .expect("prepare command readiness");
        let adapter_plan_id = plan.adapter_plan_id.clone();
        let readiness = plan.command_readiness.expect("readiness requested");
        assert_eq!(readiness.adapter_id, "codex");
        assert_eq!(readiness.adapter_plan_id, adapter_plan_id);
        assert_eq!(readiness.cli_version_probe_state, "not_probed");
        assert!(!readiness.process_start_enabled);
        let containment = plan.process_containment.expect("containment requested");
        assert_eq!(containment.adapter_plan_id, adapter_plan_id);
        assert_eq!(containment.state, "not_started");
        assert_eq!(containment.start_authorization, "not_granted");
        assert_eq!(containment.process_group, "required_on_unix");

        input.required_capability_ids = vec!["router_observe".into()];
        assert!(prepare_run_plan_with_reference(&session, input, router, None, None).is_err());
    }

    #[test]
    fn command_readiness_rejects_aliases_and_changes_the_plan_digest() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('f'),
            task_class: "planning".into(),
        })
        .expect("create session");
        let router = RouterDecisionReference {
            decision_id: "routing-decision-test-readiness-alias".into(),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: digest('2'),
        };
        let mut input = WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision_id: "routing-decision-test-readiness-alias".into(),
            replay_reference_id: None,
            preset_id: None,
            required_capability_ids: vec!["router_observe".into(), "client_adapter_plan".into()],
            requested_mode: SwitchboardMode::Off,
        };
        let without_readiness =
            prepare_run_plan_with_reference(&session, input.clone(), router.clone(), None, None)
                .expect("prepare baseline plan");
        input
            .required_capability_ids
            .push("adapter_command_readiness".into());
        let with_readiness =
            prepare_run_plan_with_reference(&session, input.clone(), router.clone(), None, None)
                .expect("prepare readiness plan");
        assert_ne!(without_readiness.plan_id, with_readiness.plan_id);

        for adapter_id in ["codex_cli", "gemini_cli", "deepseek_harness", "unknown"] {
            input.adapter_id = adapter_id.into();
            assert!(prepare_run_plan_with_reference(
                &session,
                input.clone(),
                router.clone(),
                None,
                None,
            )
            .is_err());
        }
    }

    #[test]
    fn run_plan_binds_only_exact_native_workbench_presets() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('f'),
            task_class: "planning".into(),
        })
        .expect("create session");
        let preset = super::resolve_workbench_plan_preset("adapter-plan-review")
            .expect("resolve native preset");
        let mut input = WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision_id: "routing-decision-test-6".into(),
            replay_reference_id: None,
            preset_id: Some(preset.preset_id.clone()),
            required_capability_ids: preset.required_capability_ids.clone(),
            requested_mode: SwitchboardMode::Off,
        };
        let router = RouterDecisionReference {
            decision_id: "routing-decision-test-6".into(),
            decision_stage: "observe".into(),
            routing_mode: "observe_only".into(),
            evidence_digest: digest('2'),
        };
        let plan = prepare_run_plan_with_reference(
            &session,
            input.clone(),
            router.clone(),
            None,
            Some(preset.clone()),
        )
        .expect("prepare native preset plan");
        assert_eq!(plan.preset, Some(preset.clone()));
        input.required_capability_ids.reverse();
        assert!(
            prepare_run_plan_with_reference(&session, input, router, None, Some(preset)).is_err()
        );
    }

    #[test]
    fn run_plan_requires_a_native_router_reference_resolution() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('f'),
            task_class: "planning".into(),
        })
        .expect("create session");
        let input = WorkbenchRunSpecInput {
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision_id: "routing-decision-unknown".into(),
            replay_reference_id: None,
            preset_id: None,
            required_capability_ids: vec!["router_observe".into()],
            requested_mode: SwitchboardMode::Off,
        };
        let error = prepare_run_plan(&session, input).expect_err("manual Router IDs must not plan");
        assert!(error.to_string().contains("could not be resolved"));
    }

    #[test]
    fn run_spec_rejects_prompt_and_tool_output_fields() {
        let payload = format!(
            r#"{{"sessionId":"workbench:test","adapterId":"codex","workspaceDigest":"sha256:{}","routerDecisionId":"routing-decision-test","requiredCapabilityIds":[],"requestedMode":"off","toolOutput":"private"}}"#,
            "a".repeat(64),
        );
        assert!(serde_json::from_str::<WorkbenchRunSpecInput>(&payload).is_err());
    }

    #[test]
    fn run_spec_rejects_manually_supplied_router_metadata() {
        let payload = format!(
            r#"{{"sessionId":"workbench:test","adapterId":"codex","workspaceDigest":"sha256:{}","routerDecisionId":"routing-decision-test","routerDecision":{{"decisionId":"routing-decision-test","routingMode":"observe_only","evidenceDigest":"sha256:{}"}},"requiredCapabilityIds":[],"requestedMode":"off"}}"#,
            "a".repeat(64),
            "b".repeat(64),
        );
        assert!(serde_json::from_str::<WorkbenchRunSpecInput>(&payload).is_err());
    }

    #[test]
    fn run_spec_rejects_manually_supplied_replay_data() {
        let payload = format!(
            r#"{{"sessionId":"workbench:test","adapterId":"codex","workspaceDigest":"sha256:{}","routerDecisionId":"routing-decision-test","replayDigest":"sha256:{}","requiredCapabilityIds":[],"requestedMode":"off"}}"#,
            "a".repeat(64),
            "b".repeat(64),
        );
        assert!(serde_json::from_str::<WorkbenchRunSpecInput>(&payload).is_err());
    }

    #[test]
    fn run_spec_rejects_manually_supplied_preset_metadata() {
        let payload = format!(
            r#"{{"sessionId":"workbench:test","adapterId":"codex","workspaceDigest":"sha256:{}","routerDecisionId":"routing-decision-test","preset":{{"presetId":"adapter-plan-review"}},"requiredCapabilityIds":[],"requestedMode":"off"}}"#,
            "a".repeat(64),
        );
        assert!(serde_json::from_str::<WorkbenchRunSpecInput>(&payload).is_err());
    }

    #[test]
    fn run_spec_rejects_caller_supplied_command_details() {
        let payload = format!(
            r#"{{"sessionId":"workbench:test","adapterId":"codex","workspaceDigest":"sha256:{}","routerDecisionId":"routing-decision-test","requiredCapabilityIds":[],"requestedMode":"off","command":["codex"],"environment":{{"TOKEN":"private"}},"workingDirectory":"/tmp"}}"#,
            "a".repeat(64),
        );
        assert!(serde_json::from_str::<WorkbenchRunSpecInput>(&payload).is_err());
    }
}
