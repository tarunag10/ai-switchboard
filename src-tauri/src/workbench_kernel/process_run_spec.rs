//! Content-free containment intent for a future Workbench-owned process.
//!
//! A `ProcessRunSpec` is not an executable command. It deliberately omits a
//! resolved executable, arguments, shell, environment, working directory,
//! prompt, credential, PID, and process-group ID. A later executor must honor
//! this contract before it can create and register an app-owned process group.
//!
//! The serialized schema, identity, and provider-neutral validation live in
//! `switchboard-core`. This module wraps the core contract and layers the
//! Tauri-only adapter allowlist and exact adapter contract-version pin on top.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::client_adapter_contract::CODING_CLIENT_ADAPTER_CONTRACT_VERSION;

use super::adapter_readiness::validate_adapter_command_readiness_adapter_id;
use super::events::validate_identifier;
use super::session::validate_digest;

/// Wire-compatible wrapper around the shared core contract.
///
/// Serde stays transparent so nested and persisted JSON remains identical to
/// the previous local struct, and `Deref`/`DerefMut` keep every field access
/// working unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ProcessRunSpec(switchboard_core::process_run_spec::ProcessRunSpec);

impl std::ops::Deref for ProcessRunSpec {
    type Target = switchboard_core::process_run_spec::ProcessRunSpec;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ProcessRunSpec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ProcessRunSpec {
    /// Provider-neutral validation plus the Tauri-owned adapter policy:
    /// the adapter must appear on the fixed allowlist with exactly the
    /// supported adapter contract version.
    pub(crate) fn validate(&self) -> Result<()> {
        self.0.validate()?;
        validate_adapter_command_readiness_adapter_id(&self.adapter_id)?;
        if self.adapter_contract_version != CODING_CLIENT_ADAPTER_CONTRACT_VERSION {
            bail!("Workbench process run spec violates the non-executing containment boundary");
        }
        Ok(())
    }
}

pub(crate) fn process_run_spec_digest(spec: &ProcessRunSpec) -> Result<String> {
    spec.validate()?;
    switchboard_core::process_run_spec::process_run_spec_digest(&spec.0)
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
    let spec = ProcessRunSpec(switchboard_core::process_run_spec::process_run_spec_for(
        session_id,
        adapter_plan_id,
        adapter_id,
        CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
        workspace_digest,
    )?);
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
        first
            .0
            .validate()
            .expect("validate shared core spec");
        assert_eq!(
            first.run_id,
            switchboard_core::process_run_spec::process_run_id_for(
                &first.session_id,
                &first.adapter_plan_id,
                &first.adapter_id,
                first.adapter_contract_version,
                &first.workspace_digest,
            )
            .expect("derive shared core process run ID")
        );
        assert_eq!(
            process_run_spec_digest(&first).expect("digest Tauri spec"),
            switchboard_core::process_run_spec::process_run_spec_digest(&first.0)
                .expect("digest shared core spec")
        );
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
