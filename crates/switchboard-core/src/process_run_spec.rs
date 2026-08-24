//! Provider-neutral, content-free process containment contracts.
//!
//! This module owns the serialized process-run intent, deterministic identity,
//! and digest rules. Adapter allowlists, exact adapter contract-version policy,
//! command resolution, persistence, process launch, and supervision remain
//! platform-adapter concerns.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::workbench::{validate_digest, validate_identifier};

pub const PROCESS_RUN_SPEC_SCHEMA_VERSION: u32 = 1;
pub const WORKBENCH_NATIVE_OWNER: &str = "workbench_native";
pub const NOT_STARTED: &str = "not_started";
pub const START_NOT_GRANTED: &str = "not_granted";
pub const NATIVE_ADAPTER_ONLY: &str = "native_adapter_only";
pub const PROCESS_GROUP_REQUIRED_ON_UNIX: &str = "required_on_unix";
pub const NULL_STDIN: &str = "null";
pub const BOUNDED_REDACTED_OUTPUT: &str = "piped_bounded_redacted";
pub const FIXED_TIMEOUT_POLICY: &str = "native_fixed_policy_required";
pub const GROUP_TERMINATE_THEN_KILL: &str = "group_sigterm_then_sigkill";

/// Content-free containment intent for a future Workbench-owned process.
///
/// This deliberately contains no resolved executable, arguments, shell,
/// environment, working directory, prompt, credential, PID, or process-group
/// ID. Serde remains permissive toward unknown fields for compatibility with
/// the existing Tauri nested wire contract; changing that requires a separate
/// schema migration.
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
    /// Validates provider-neutral shape and fail-closed containment policy.
    ///
    /// Platform adapters must additionally enforce their adapter allowlist and
    /// exact supported adapter contract version.
    pub fn validate(&self) -> Result<()> {
        validate_process_run_spec(self)
    }
}

pub fn validate_process_run_spec(spec: &ProcessRunSpec) -> Result<()> {
    if spec.schema_version != PROCESS_RUN_SPEC_SCHEMA_VERSION {
        bail!("Workbench process run spec schema is unsupported");
    }
    validate_identifier(&spec.run_id, "process run ID")?;
    validate_identifier(&spec.session_id, "session ID")?;
    validate_identifier(&spec.adapter_plan_id, "adapter plan ID")?;
    validate_identifier(&spec.adapter_id, "adapter ID")?;
    validate_digest(&spec.workspace_digest, "workspace digest")?;
    if spec.adapter_contract_version == 0
        || spec.owner != WORKBENCH_NATIVE_OWNER
        || spec.state != NOT_STARTED
        || spec.start_authorization != START_NOT_GRANTED
        || spec.launch_mode != NATIVE_ADAPTER_ONLY
        || spec.process_group != PROCESS_GROUP_REQUIRED_ON_UNIX
        || spec.stdin != NULL_STDIN
        || spec.output != BOUNDED_REDACTED_OUTPUT
        || spec.timeout_policy != FIXED_TIMEOUT_POLICY
        || spec.cancellation != GROUP_TERMINATE_THEN_KILL
        || spec.provider_traffic != "none"
        || spec.writes_enabled
    {
        bail!("Workbench process run spec violates the non-executing containment boundary");
    }
    Ok(())
}

/// Derives the compatibility identity used by the existing Tauri contract.
pub fn process_run_id_for(
    session_id: &str,
    adapter_plan_id: &str,
    adapter_id: &str,
    adapter_contract_version: u32,
    workspace_digest: &str,
) -> Result<String> {
    validate_identifier(session_id, "session ID")?;
    validate_identifier(adapter_plan_id, "adapter plan ID")?;
    validate_identifier(adapter_id, "adapter ID")?;
    validate_digest(workspace_digest, "workspace digest")?;
    if adapter_contract_version == 0 {
        bail!("Workbench process run spec adapter contract version must be non-zero");
    }
    let canonical = serde_json::json!({
        "sessionId": session_id,
        "adapterPlanId": adapter_plan_id,
        "adapterId": adapter_id,
        "adapterContractVersion": adapter_contract_version,
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
    Ok(format!("process-run:{digest:x}")[..41].to_string())
}

/// Creates the provider-neutral spec. Adapter policy must be checked by the
/// caller before this intent can participate in a platform-owned run plan.
pub fn process_run_spec_for(
    session_id: &str,
    adapter_plan_id: &str,
    adapter_id: &str,
    adapter_contract_version: u32,
    workspace_digest: &str,
) -> Result<ProcessRunSpec> {
    let run_id = process_run_id_for(
        session_id,
        adapter_plan_id,
        adapter_id,
        adapter_contract_version,
        workspace_digest,
    )?;
    let spec = ProcessRunSpec {
        schema_version: PROCESS_RUN_SPEC_SCHEMA_VERSION,
        run_id,
        session_id: session_id.to_string(),
        adapter_plan_id: adapter_plan_id.to_string(),
        adapter_id: adapter_id.to_string(),
        adapter_contract_version,
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

/// Computes the exact digest used by the current Tauri process-run contract.
pub fn process_run_spec_digest(spec: &ProcessRunSpec) -> Result<String> {
    spec.validate()?;
    let bytes = serde_json::to_vec(spec).context("canonicalizing Workbench process run spec")?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn tauri_compatible_spec() -> ProcessRunSpec {
        process_run_spec_for(
            "workbench:test",
            "codex-1234567890ab",
            "codex",
            1,
            &digest('a'),
        )
        .expect("create Tauri-compatible process run spec")
    }

    #[test]
    fn tauri_wire_identity_and_digest_are_exactly_compatible() {
        let spec = tauri_compatible_spec();
        assert_eq!(spec.run_id, "process-run:2b6f2f14743aa219493611b83ddf5");
        assert_eq!(
            process_run_spec_digest(&spec).expect("digest process run spec"),
            "sha256:e04b8963658173bca19125ca8728341e92a6bad8d9c346989b33b4e22fb92a40"
        );

        let value = serde_json::to_value(&spec).expect("serialize process run spec");
        let object = value.as_object().expect("process run spec object");
        assert_eq!(object.len(), 18);
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["adapterPlanId"], "codex-1234567890ab");
        assert_eq!(value["adapterContractVersion"], 1);
        assert_eq!(value["startAuthorization"], "not_granted");
        assert_eq!(value["providerTraffic"], "none");
        assert_eq!(value["writesEnabled"], false);
    }

    #[test]
    fn identity_binds_every_dynamic_input() {
        let baseline = tauri_compatible_spec();
        let variants = [
            process_run_spec_for(
                "workbench:changed",
                "codex-1234567890ab",
                "codex",
                1,
                &digest('a'),
            ),
            process_run_spec_for("workbench:test", "codex-changed", "codex", 1, &digest('a')),
            process_run_spec_for(
                "workbench:test",
                "codex-1234567890ab",
                "future_adapter",
                1,
                &digest('a'),
            ),
            process_run_spec_for(
                "workbench:test",
                "codex-1234567890ab",
                "codex",
                2,
                &digest('a'),
            ),
            process_run_spec_for(
                "workbench:test",
                "codex-1234567890ab",
                "codex",
                1,
                &digest('b'),
            ),
        ];
        for variant in variants {
            let variant = variant.expect("create identity variant");
            assert_ne!(variant.run_id, baseline.run_id);
            assert_ne!(
                process_run_spec_digest(&variant).expect("digest variant"),
                process_run_spec_digest(&baseline).expect("digest baseline")
            );
        }
    }

    #[test]
    fn core_stays_provider_neutral_while_references_remain_bounded() {
        let future = process_run_spec_for(
            "workbench:test",
            "future-plan:1",
            "future_adapter",
            2,
            &digest('c'),
        )
        .expect("opaque future adapter is structurally valid");
        future.validate().expect("provider-neutral validation");

        assert!(process_run_spec_for(
            "workbench:test",
            "future-plan:1",
            "future adapter",
            2,
            &digest('c'),
        )
        .is_err());
        assert!(process_run_spec_for(
            "workbench:test",
            "future-plan:1",
            "future_adapter",
            0,
            &digest('c'),
        )
        .is_err());
        assert!(process_run_spec_for(
            "workbench:test",
            "future-plan:1",
            "future_adapter",
            2,
            "workspace-path",
        )
        .is_err());
    }

    #[test]
    fn every_static_containment_boundary_fails_closed_on_tampering() {
        let baseline = tauri_compatible_spec();
        let mut tampered = Vec::new();

        let mut value = baseline.clone();
        value.schema_version += 1;
        tampered.push(value);
        let mut value = baseline.clone();
        value.adapter_contract_version = 0;
        tampered.push(value);
        let mut value = baseline.clone();
        value.owner = "external".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.state = "started".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.start_authorization = "granted".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.launch_mode = "shell".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.process_group = "optional".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.stdin = "inherit".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.output = "unbounded".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.timeout_policy = "caller_controlled".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.cancellation = "none".into();
        tampered.push(value);
        let mut value = baseline.clone();
        value.provider_traffic = "enabled".into();
        tampered.push(value);
        let mut value = baseline;
        value.writes_enabled = true;
        tampered.push(value);

        assert!(tampered.iter().all(|spec| spec.validate().is_err()));
    }

    #[test]
    fn command_and_sensitive_fields_remain_absent() {
        let value = serde_json::to_value(tauri_compatible_spec()).expect("serialize spec");
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
                value.get(forbidden).is_none(),
                "unexpected {forbidden} field"
            );
        }
    }

    #[test]
    fn legacy_unknown_fields_remain_ignored_until_a_schema_migration() {
        let spec = tauri_compatible_spec();
        let mut value = serde_json::to_value(&spec).expect("serialize spec");
        value["legacyExtension"] = serde_json::json!("ignored");
        let decoded: ProcessRunSpec =
            serde_json::from_value(value).expect("preserve permissive nested Tauri behavior");
        assert_eq!(decoded, spec);
    }
}
