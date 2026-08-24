//! Content-free containment intent for a future Workbench-owned process.
//!
//! A `ProcessRunSpec` is not an executable command. It deliberately omits a
//! resolved executable, arguments, shell, environment, working directory,
//! prompt, credential, PID, and process-group ID. A later executor must honor
//! this contract before it can create and register an app-owned process group.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::client_adapter_contract::CODING_CLIENT_ADAPTER_CONTRACT_VERSION;

use super::adapter_readiness::validate_adapter_command_readiness_adapter_id;
use super::events::validate_identifier;
use super::session::validate_digest;

const PROCESS_RUN_SPEC_SCHEMA_VERSION: u32 = 1;
const WORKBENCH_NATIVE_OWNER: &str = "workbench_native";
const NOT_STARTED: &str = "not_started";
const START_NOT_GRANTED: &str = "not_granted";
const NATIVE_ADAPTER_ONLY: &str = "native_adapter_only";
const PROCESS_GROUP_REQUIRED_ON_UNIX: &str = "required_on_unix";
const NULL_STDIN: &str = "null";
const BOUNDED_REDACTED_OUTPUT: &str = "piped_bounded_redacted";
const FIXED_TIMEOUT_POLICY: &str = "native_fixed_policy_required";
const GROUP_TERMINATE_THEN_KILL: &str = "group_sigterm_then_sigkill";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRunSpec {
    pub schema_version: u32,
    pub run_id: String,
    pub session_id: String,
    /// Existing adapter dry-run plan only. This is not an executable command.
    pub adapter_plan_id: String,
    pub adapter_id: String,
    pub adapter_contract_version: u32,
    pub workspace_digest: String,
    pub owner: String,
    pub state: String,
    pub start_authorization: String,
    pub launch_mode: String,
    pub process_group: String,
    pub stdin: String,
    pub output: String,
    pub timeout_policy: String,
    pub cancellation: String,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

impl ProcessRunSpec {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.schema_version != PROCESS_RUN_SPEC_SCHEMA_VERSION {
            bail!("Workbench process run spec schema is unsupported");
        }
        validate_identifier(&self.run_id, "process run ID")?;
        validate_identifier(&self.session_id, "session ID")?;
        validate_identifier(&self.adapter_plan_id, "adapter plan ID")?;
        validate_adapter_command_readiness_adapter_id(&self.adapter_id)?;
        validate_digest(&self.workspace_digest, "workspace digest")?;
        if self.adapter_contract_version != CODING_CLIENT_ADAPTER_CONTRACT_VERSION
            || self.owner != WORKBENCH_NATIVE_OWNER
            || self.state != NOT_STARTED
            || self.start_authorization != START_NOT_GRANTED
            || self.launch_mode != NATIVE_ADAPTER_ONLY
            || self.process_group != PROCESS_GROUP_REQUIRED_ON_UNIX
            || self.stdin != NULL_STDIN
            || self.output != BOUNDED_REDACTED_OUTPUT
            || self.timeout_policy != FIXED_TIMEOUT_POLICY
            || self.cancellation != GROUP_TERMINATE_THEN_KILL
            || self.provider_traffic != "none"
            || self.writes_enabled
        {
            bail!("Workbench process run spec violates the non-executing containment boundary");
        }
        Ok(())
    }
}

pub(crate) fn process_run_spec_digest(spec: &ProcessRunSpec) -> Result<String> {
    spec.validate()?;
    let bytes = serde_json::to_vec(spec).context("canonicalizing Workbench process run spec")?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

pub(crate) fn process_run_spec_for(
    session_id: &str,
    adapter_plan_id: &str,
    adapter_id: &str,
    workspace_digest: &str,
) -> Result<ProcessRunSpec> {
    validate_identifier(session_id, "session ID")?;
    validate_identifier(adapter_plan_id, "adapter plan ID")?;
    validate_adapter_command_readiness_adapter_id(adapter_id)?;
    validate_digest(workspace_digest, "workspace digest")?;
    let canonical = serde_json::json!({
        "sessionId": session_id,
        "adapterPlanId": adapter_plan_id,
        "adapterId": adapter_id,
        "adapterContractVersion": CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
        "workspaceDigest": workspace_digest,
        "owner": WORKBENCH_NATIVE_OWNER,
        "launchMode": NATIVE_ADAPTER_ONLY,
        "processGroup": PROCESS_GROUP_REQUIRED_ON_UNIX,
        "stdin": NULL_STDIN,
        "output": BOUNDED_REDACTED_OUTPUT,
        "timeoutPolicy": FIXED_TIMEOUT_POLICY,
        "cancellation": GROUP_TERMINATE_THEN_KILL,
    });
    let digest = Sha256::digest(
        serde_json::to_vec(&canonical).context("canonicalizing Workbench process run spec")?,
    );
    let spec = ProcessRunSpec {
        schema_version: PROCESS_RUN_SPEC_SCHEMA_VERSION,
        run_id: format!("process-run:{:x}", digest)[..41].to_string(),
        session_id: session_id.to_string(),
        adapter_plan_id: adapter_plan_id.to_string(),
        adapter_id: adapter_id.to_string(),
        adapter_contract_version: CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
        workspace_digest: workspace_digest.to_string(),
        owner: WORKBENCH_NATIVE_OWNER.into(),
        state: NOT_STARTED.into(),
        start_authorization: START_NOT_GRANTED.into(),
        launch_mode: NATIVE_ADAPTER_ONLY.into(),
        process_group: PROCESS_GROUP_REQUIRED_ON_UNIX.into(),
        stdin: NULL_STDIN.into(),
        output: BOUNDED_REDACTED_OUTPUT.into(),
        timeout_policy: FIXED_TIMEOUT_POLICY.into(),
        cancellation: GROUP_TERMINATE_THEN_KILL.into(),
        provider_traffic: "none".into(),
        writes_enabled: false,
    };
    spec.validate()?;
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::{process_run_spec_digest, process_run_spec_for};

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    #[test]
    fn process_run_spec_is_deterministic_and_non_executing() {
        let first = process_run_spec_for(
            "workbench:test",
            "codex-1234567890ab",
            "codex",
            &digest('a'),
        )
        .expect("create process run spec");
        let second = process_run_spec_for(
            "workbench:test",
            "codex-1234567890ab",
            "codex",
            &digest('a'),
        )
        .expect("create process run spec");
        assert_eq!(first, second);
        assert_eq!(
            process_run_spec_digest(&first).expect("digest first spec"),
            process_run_spec_digest(&second).expect("digest second spec")
        );
        assert_eq!(first.state, "not_started");
        assert_eq!(first.start_authorization, "not_granted");
        assert_eq!(first.cancellation, "group_sigterm_then_sigkill");
        assert_eq!(first.provider_traffic, "none");
        assert!(!first.writes_enabled);
        let different_workspace = process_run_spec_for(
            "workbench:test",
            "codex-1234567890ab",
            "codex",
            &digest('b'),
        )
        .expect("create distinct process run spec");
        assert_ne!(first.run_id, different_workspace.run_id);
        assert_ne!(
            process_run_spec_digest(&first).expect("digest first spec"),
            process_run_spec_digest(&different_workspace).expect("digest distinct spec")
        );
    }

    #[test]
    fn process_run_spec_omits_command_and_sensitive_fields() {
        let spec = process_run_spec_for(
            "workbench:test",
            "claude_code-1234567890",
            "claude_code",
            &digest('b'),
        )
        .expect("create process run spec");
        let object = serde_json::to_value(&spec).expect("serialize process run spec");
        for forbidden in [
            "executable",
            "path",
            "arguments",
            "shell",
            "environment",
            "workingDirectory",
            "prompt",
            "credential",
            "pid",
            "pgid",
            "timeoutSeconds",
        ] {
            assert!(
                object.get(forbidden).is_none(),
                "unexpected {forbidden} field"
            );
        }
    }

    #[test]
    fn process_run_spec_rejects_aliases_and_invalid_references() {
        let workspace = digest('c');
        assert!(process_run_spec_for(
            "workbench:test",
            "codex-1234567890ab",
            "codex_cli",
            &workspace,
        )
        .is_err());
        assert!(process_run_spec_for("workbench:test", "not an ID", "codex", &workspace).is_err());
        assert!(process_run_spec_for(
            "workbench:test",
            "codex-1234567890ab",
            "codex",
            "workspace-path",
        )
        .is_err());
    }

    #[test]
    fn tampered_start_authorization_fails_closed() {
        let mut spec = process_run_spec_for(
            "workbench:test",
            "codex-1234567890ab",
            "codex",
            &digest('d'),
        )
        .expect("create process run spec");
        spec.start_authorization = "granted".into();
        assert!(spec.validate().is_err());
    }
}
