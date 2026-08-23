use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::client_adapter_contract::{
    coding_client_adapter_for_version, ConfigPlanAction, CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
};
use crate::models::SwitchboardMode;

use super::events::validate_identifier;
use super::session::{validate_digest, WorkbenchSession};

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
    pub router_decision: RouterDecisionReference,
    pub required_capability_ids: Vec<String>,
    pub requested_mode: SwitchboardMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouterDecisionReference {
    pub decision_id: String,
    pub policy_stage: String,
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
    pub requested_mode: SwitchboardMode,
    pub adapter_plan_id: String,
    pub adapter_action: String,
    pub adapter_reversible: bool,
    pub capability_requests: Vec<CapabilityRequest>,
    pub execution_mode: String,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

fn validate_router_reference(reference: &RouterDecisionReference) -> Result<()> {
    validate_identifier(&reference.decision_id, "router decision ID")?;
    validate_digest(&reference.evidence_digest, "router evidence digest")?;
    if reference.policy_stage != OBSERVE_ONLY {
        bail!("Workbench plan requires an observe-only Router decision reference");
    }
    Ok(())
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
            "repo_context" | "redacted_replay" | "router_observe" | "client_adapter_plan"
        ) {
            bail!("Workbench capability is not available in plan-only mode: {id}");
        }
        if !seen.insert(id) {
            bail!("Workbench run plan contains a duplicate capability request");
        }
    }
    Ok(())
}

pub(crate) fn prepare_run_plan(
    session: &WorkbenchSession,
    input: WorkbenchRunSpecInput,
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
    validate_router_reference(&input.router_decision)?;
    validate_capability_ids(&input.required_capability_ids)?;
    let adapter = coding_client_adapter_for_version(
        &input.adapter_id,
        CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
    )?;
    let adapter_plan = adapter.plan(input.requested_mode.clone())?;
    let canonical = serde_json::json!({
        "sessionId": &input.session_id,
        "adapterId": adapter.id(),
        "workspaceDigest": &input.workspace_digest,
        "contextPackDigest": &input.context_pack_digest,
        "routerDecision": &input.router_decision,
        "capabilityIds": &input.required_capability_ids,
        "requestedMode": &input.requested_mode,
        "adapterPlanId": &adapter_plan.plan_id,
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
        router_decision: input.router_decision,
        requested_mode: input.requested_mode,
        adapter_plan_id: adapter_plan.plan_id,
        adapter_action: match adapter_plan.action {
            ConfigPlanAction::ApplyManagedRouting => "apply_managed_routing".into(),
            ConfigPlanAction::CleanupManagedRouting => "cleanup_managed_routing".into(),
        },
        adapter_reversible: adapter_plan.reversible,
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

#[cfg(test)]
mod tests {
    use super::{prepare_run_plan, RouterDecisionReference, WorkbenchRunSpecInput};
    use crate::models::SwitchboardMode;
    use crate::workbench_kernel::session::{CreateWorkbenchSessionInput, WorkbenchSession};

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    #[test]
    fn run_plan_is_declarative_and_execution_disabled() {
        let session = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: digest('c'),
            task_class: "coding".into(),
        })
        .expect("create session");
        let plan = prepare_run_plan(
            &session,
            WorkbenchRunSpecInput {
                session_id: session.session_id.clone(),
                adapter_id: "codex".into(),
                workspace_digest: session.workspace_digest.clone(),
                context_pack_digest: Some(digest('d')),
                router_decision: RouterDecisionReference {
                    decision_id: "route:local-1".into(),
                    policy_stage: "observe_only".into(),
                    evidence_digest: digest('e'),
                },
                required_capability_ids: vec![
                    "router_observe".into(),
                    "client_adapter_plan".into(),
                ],
                requested_mode: SwitchboardMode::Headroom,
            },
        )
        .expect("prepare plan");
        assert_eq!(plan.execution_mode, "plan_only");
        assert_eq!(plan.provider_traffic, "none");
        assert!(!plan.writes_enabled);
        assert!(plan
            .capability_requests
            .iter()
            .all(|request| !request.execution_enabled));
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
            router_decision: RouterDecisionReference {
                decision_id: "route:local-2".into(),
                policy_stage: "automatic".into(),
                evidence_digest: digest('a'),
            },
            required_capability_ids: vec!["router_observe".into()],
            requested_mode: SwitchboardMode::Off,
        };
        assert!(prepare_run_plan(&session, input.clone()).is_err());
        input.router_decision.policy_stage = "observe_only".into();
        input.required_capability_ids = vec!["arbitrary_shell".into()];
        assert!(prepare_run_plan(&session, input).is_err());
    }

    #[test]
    fn run_spec_rejects_prompt_and_tool_output_fields() {
        let payload = format!(
            r#"{{"sessionId":"workbench:test","adapterId":"codex","workspaceDigest":"sha256:{}","routerDecision":{{"decisionId":"route:test","policyStage":"observe_only","evidenceDigest":"sha256:{}"}},"requiredCapabilityIds":[],"requestedMode":"off","toolOutput":"private"}}"#,
            "a".repeat(64),
            "b".repeat(64)
        );
        assert!(serde_json::from_str::<WorkbenchRunSpecInput>(&payload).is_err());
    }
}
