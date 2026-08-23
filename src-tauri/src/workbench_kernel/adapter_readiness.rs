//! Metadata-only readiness for the first execution-adapter gate.
//!
//! This module never resolves a shell, reads a CLI version, starts a process,
//! reads credentials, or returns a local path. It deliberately reports only
//! fixed-candidate metadata for the two canonical adapters that Phase 4 is
//! preparing: Codex and Claude Code.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::client_adapter_contract::CODING_CLIENT_ADAPTER_CONTRACT_VERSION;

use super::events::validate_identifier;

pub(crate) const ADAPTER_COMMAND_READINESS_CAPABILITY_ID: &str = "adapter_command_readiness";
const READINESS_SCHEMA_VERSION: u32 = 1;
const METADATA_DISCOVERY_MODE: &str = "fixed_known_location_metadata_only";
const CLI_VERSION_NOT_PROBED: &str = "not_probed";
const VERSION_PROBE_DEFERRED_REASON: &str =
    "CLI version probing is deferred because it would start a process.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchAdapterReadiness {
    pub schema_version: u32,
    pub adapter_id: String,
    pub adapter_contract_version: u32,
    pub logical_binary: String,
    pub known_candidate_present: bool,
    pub discovery_mode: String,
    pub cli_version_probe_state: String,
    pub version_probe_reason: String,
    pub process_start_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchAdapterCommandReadiness {
    pub schema_version: u32,
    pub adapter_id: String,
    pub adapter_contract_version: u32,
    pub adapter_plan_id: String,
    pub logical_binary: String,
    pub known_candidate_present: bool,
    pub discovery_mode: String,
    pub cli_version_probe_state: String,
    pub version_probe_reason: String,
    pub process_start_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

fn logical_binary(adapter_id: &str) -> Result<&'static str> {
    match adapter_id {
        "codex" => Ok("codex"),
        "claude_code" => Ok("claude"),
        _ => bail!(
            "Workbench command readiness is available only for canonical Codex or Claude Code adapters"
        ),
    }
}

pub(crate) fn validate_adapter_command_readiness_adapter_id(adapter_id: &str) -> Result<()> {
    validate_identifier(adapter_id, "adapter ID")?;
    logical_binary(adapter_id).map(|_| ())
}

fn readiness_for_adapter(adapter_id: &str) -> Result<WorkbenchAdapterReadiness> {
    validate_adapter_command_readiness_adapter_id(adapter_id)?;
    let logical_binary = logical_binary(adapter_id)?;
    let known_candidate_present = match adapter_id {
        "claude_code" => {
            crate::cli_discovery::known_cli_candidate_present_without_start("claude")
                || crate::cli_discovery::known_cli_candidate_present_without_start("claude-code")
        }
        "codex" => crate::cli_discovery::known_cli_candidate_present_without_start("codex"),
        _ => unreachable!("canonical adapter validation must resolve the logical binary"),
    };
    Ok(WorkbenchAdapterReadiness {
        schema_version: READINESS_SCHEMA_VERSION,
        adapter_id: adapter_id.to_string(),
        adapter_contract_version: CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
        logical_binary: logical_binary.to_string(),
        known_candidate_present,
        discovery_mode: METADATA_DISCOVERY_MODE.into(),
        cli_version_probe_state: CLI_VERSION_NOT_PROBED.into(),
        version_probe_reason: VERSION_PROBE_DEFERRED_REASON.into(),
        process_start_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
    })
}

/// Native-owned metadata projection for the Phase 4 compatibility matrix.
/// The boolean candidate state is intentionally not installation proof.
pub(crate) fn all_adapter_readiness() -> Vec<WorkbenchAdapterReadiness> {
    ["codex", "claude_code"]
        .into_iter()
        .map(|adapter_id| {
            readiness_for_adapter(adapter_id)
                .expect("static adapter readiness catalog must contain canonical adapters")
        })
        .collect()
}

/// Creates a non-executable command-readiness descriptor after the existing
/// adapter has produced its dry-run configuration plan. It deliberately omits
/// executable paths, argv, environment, working directory, instructions,
/// prompts, timeouts, and credentials.
pub(crate) fn command_readiness_for(
    adapter_id: &str,
    adapter_plan_id: &str,
) -> Result<WorkbenchAdapterCommandReadiness> {
    validate_identifier(adapter_plan_id, "adapter plan ID")?;
    let readiness = readiness_for_adapter(adapter_id)?;
    Ok(WorkbenchAdapterCommandReadiness {
        schema_version: READINESS_SCHEMA_VERSION,
        adapter_id: readiness.adapter_id,
        adapter_contract_version: readiness.adapter_contract_version,
        adapter_plan_id: adapter_plan_id.to_string(),
        logical_binary: readiness.logical_binary,
        known_candidate_present: readiness.known_candidate_present,
        discovery_mode: readiness.discovery_mode,
        cli_version_probe_state: readiness.cli_version_probe_state,
        version_probe_reason: readiness.version_probe_reason,
        process_start_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        all_adapter_readiness, command_readiness_for,
        validate_adapter_command_readiness_adapter_id, ADAPTER_COMMAND_READINESS_CAPABILITY_ID,
    };
    use crate::client_adapter_contract::CODING_CLIENT_ADAPTER_CONTRACT_VERSION;

    #[test]
    fn readiness_matrix_is_limited_to_canonical_no_process_adapters() {
        let matrix = all_adapter_readiness();
        assert_eq!(matrix.len(), 2);
        assert_eq!(
            matrix
                .iter()
                .map(|readiness| readiness.adapter_id.as_str())
                .collect::<Vec<_>>(),
            vec!["codex", "claude_code"]
        );
        assert!(matrix.iter().all(|readiness| {
            readiness.adapter_contract_version == CODING_CLIENT_ADAPTER_CONTRACT_VERSION
                && readiness.cli_version_probe_state == "not_probed"
                && !readiness.process_start_enabled
                && readiness.provider_traffic == "none"
                && !readiness.writes_enabled
        }));
    }

    #[test]
    fn command_readiness_is_content_free_and_non_executable() {
        let readiness =
            command_readiness_for("codex", "codex-1234567890ab").expect("canonical readiness");
        assert_eq!(readiness.logical_binary, "codex");
        assert_eq!(readiness.adapter_plan_id, "codex-1234567890ab");
        assert!(!readiness.process_start_enabled);
        let object = serde_json::to_value(&readiness).expect("serialize readiness");
        for forbidden in [
            "path",
            "executable",
            "arguments",
            "environment",
            "workingDirectory",
            "timeout",
            "prompt",
            "credential",
            "token",
        ] {
            assert!(
                object.get(forbidden).is_none(),
                "unexpected {forbidden} field"
            );
        }

        let claude = command_readiness_for("claude_code", "claude_code-1234567890")
            .expect("canonical Claude Code readiness");
        assert_eq!(claude.logical_binary, "claude");
    }

    #[test]
    fn aliases_and_unprepared_adapters_fail_closed() {
        assert_eq!(
            ADAPTER_COMMAND_READINESS_CAPABILITY_ID,
            "adapter_command_readiness"
        );
        for adapter_id in [
            "codex_cli",
            "gemini_cli",
            "deepseek_harness",
            "unknown",
            " codex",
        ] {
            assert!(validate_adapter_command_readiness_adapter_id(adapter_id).is_err());
        }
    }
}
