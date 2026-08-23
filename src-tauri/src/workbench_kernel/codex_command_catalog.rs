//! Workbench-owned, canonical Codex command catalog and pure probe evaluation.
//!
//! This module never reads the filesystem, performs shell lookup, starts a
//! process, reads credentials, or writes a workspace. A later native-only
//! collector may supply a complete metadata snapshot and a later opt-in manual
//! harness may supply bounded `codex --version` evidence. Both are evaluated
//! here without claiming that a binary is runnable or supported.

use super::session::validate_digest;
use std::collections::{BTreeMap, BTreeSet};
const CATALOG_SCHEMA_VERSION: u32 = 1;
const PROBE_SCHEMA_VERSION: u32 = 1;
const CODEX_ADAPTER_ID: &str = "codex";
const CODEX_VERSION_ARGUMENT: &str = "--version";
const CODEX_VERSION_PREFIX: &str = "codex-cli ";
const MAX_VERSION_OUTPUT_BYTES: usize = 128;
const VERSION_PROBE_TIMEOUT_MILLISECONDS: u64 = 2_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CodexCommandCatalogEntry {
    pub candidate_id: &'static str,
    pub location_template: &'static str,
}
const CODEX_COMMAND_CATALOG: [CodexCommandCatalogEntry; 7] = [
    CodexCommandCatalogEntry {
        candidate_id: "home-local-bin",
        location_template: "$HOME/.local/bin/codex",
    },
    CodexCommandCatalogEntry {
        candidate_id: "opt-homebrew-bin",
        location_template: "/opt/homebrew/bin/codex",
    },
    CodexCommandCatalogEntry {
        candidate_id: "usr-local-bin",
        location_template: "/usr/local/bin/codex",
    },
    CodexCommandCatalogEntry {
        candidate_id: "home-npm-global-bin",
        location_template: "$HOME/.npm-global/bin/codex",
    },
    CodexCommandCatalogEntry {
        candidate_id: "home-volta-bin",
        location_template: "$HOME/.volta/bin/codex",
    },
    CodexCommandCatalogEntry {
        candidate_id: "home-bun-bin",
        location_template: "$HOME/.bun/bin/codex",
    },
    CodexCommandCatalogEntry {
        candidate_id: "usr-bin",
        location_template: "/usr/bin/codex",
    },
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexResolvedCandidateKind {
    RegularFile,
    Directory,
    SpecialFile,
    UnresolvedSymlink,
    UnsafeResolution,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CodexCandidateObservation {
    Unobserved {
        candidate_id: String,
    },
    ConfirmedAbsent {
        candidate_id: String,
    },
    ObservationFailed {
        candidate_id: String,
    },
    Present {
        candidate_id: String,
        resolved_kind: CodexResolvedCandidateKind,
        executable: bool,
        identity_digest: Option<String>,
    },
}
impl CodexCandidateObservation {
    fn candidate_id(&self) -> &str {
        match self {
            Self::Unobserved { candidate_id }
            | Self::ConfirmedAbsent { candidate_id }
            | Self::ObservationFailed { candidate_id }
            | Self::Present { candidate_id, .. } => candidate_id,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexCommandSnapshot {
    pub schema_version: u32,
    pub observations: Vec<CodexCandidateObservation>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexCatalogEvaluation {
    pub schema_version: u32,
    pub adapter_id: String,
    pub state: String,
    pub reason_code: String,
    pub candidate_id: Option<String>,
    pub binary_identity_digest: Option<String>,
    pub version_state: String,
    pub runnable: bool,
    pub supported: bool,
    pub process_start_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexProbePlan {
    pub schema_version: u32,
    pub adapter_id: String,
    pub candidate_id: String,
    pub binary_identity_digest: String,
    pub argument: String,
    pub stdin_policy: String,
    pub output_policy: String,
    pub timeout_milliseconds: u64,
    pub max_output_bytes: usize,
    pub shell_enabled: bool,
    pub working_directory_enabled: bool,
    pub inherited_environment_enabled: bool,
    pub process_start_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CodexProbeOutcome {
    Completed {
        exit_success: bool,
        output_truncated: bool,
        /// Transient, bounded stdout. It is normalized and never returned.
        version_output: String,
    },
    TimedOut,
    SpawnFailed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexVersionProbeObservation {
    pub schema_version: u32,
    pub candidate_id: String,
    pub identity_digest_before: String,
    pub identity_digest_after: String,
    pub outcome: CodexProbeOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexVersionProbeEvaluation {
    pub schema_version: u32,
    pub adapter_id: String,
    pub candidate_id: String,
    pub binary_identity_digest: String,
    pub normalized_version: String,
    pub probe_state: String,
    pub manual_harness_required: bool,
    pub runnable: bool,
    pub supported: bool,
    pub process_start_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}
pub(crate) fn codex_command_catalog() -> &'static [CodexCommandCatalogEntry] {
    &CODEX_COMMAND_CATALOG
}

pub(crate) fn evaluate_codex_command_snapshot(
    snapshot: &CodexCommandSnapshot,
) -> Result<CodexCatalogEvaluation, String> {
    if snapshot.schema_version != CATALOG_SCHEMA_VERSION {
        return Err("Codex command snapshot schema is unsupported".into());
    }
    let catalog_ids = codex_command_catalog()
        .iter()
        .map(|entry| entry.candidate_id)
        .collect::<BTreeSet<_>>();
    let mut observations = BTreeMap::new();
    for observation in &snapshot.observations {
        let candidate_id = observation.candidate_id();
        if !catalog_ids.contains(candidate_id) {
            return Err("Codex command snapshot contains an unknown fixed candidate".into());
        }
        if observations.insert(candidate_id, observation).is_some() {
            return Err("Codex command snapshot contains a duplicate candidate".into());
        }
    }
    if observations.len() != catalog_ids.len() {
        return Ok(catalog_evaluation(
            "incomplete",
            "fixed_catalog_observation_missing",
            None,
            None,
        ));
    }
    if observations
        .values()
        .any(|value| matches!(value, CodexCandidateObservation::Unobserved { .. }))
    {
        return Ok(catalog_evaluation(
            "incomplete",
            "fixed_catalog_observation_missing",
            None,
            None,
        ));
    }
    if observations
        .values()
        .any(|value| matches!(value, CodexCandidateObservation::ObservationFailed { .. }))
    {
        return Ok(catalog_evaluation(
            "observation_failed",
            "fixed_catalog_observation_failed",
            None,
            None,
        ));
    }

    let present = observations
        .values()
        .filter_map(|observation| match observation {
            CodexCandidateObservation::Present {
                candidate_id,
                resolved_kind,
                executable,
                identity_digest,
            } => Some((candidate_id, resolved_kind, executable, identity_digest)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if present.is_empty() {
        return Ok(catalog_evaluation(
            "confirmed_absent_from_fixed_catalog",
            "all_fixed_catalog_candidates_confirmed_absent",
            None,
            None,
        ));
    }
    if present.len() != 1 {
        return Ok(catalog_evaluation(
            "ambiguous",
            "multiple_fixed_candidates_present",
            None,
            None,
        ));
    }
    let (candidate_id, resolved_kind, executable, identity_digest) = present[0];
    if *resolved_kind != CodexResolvedCandidateKind::RegularFile || !*executable {
        return Ok(catalog_evaluation(
            "rejected",
            "candidate_is_not_a_safe_executable_regular_file",
            Some(candidate_id),
            None,
        ));
    }
    let identity_digest = identity_digest
        .as_deref()
        .ok_or_else(|| "Codex candidate is missing an identity digest".to_string())?;
    validate_digest(identity_digest, "Codex binary identity digest")
        .map_err(|error| error.to_string())?;
    Ok(catalog_evaluation(
        "present_unprobed",
        "fixed_candidate_requires_opt_in_version_probe",
        Some(candidate_id),
        Some(identity_digest),
    ))
}

fn catalog_evaluation(
    state: &str,
    reason_code: &str,
    candidate_id: Option<&str>,
    identity_digest: Option<&str>,
) -> CodexCatalogEvaluation {
    CodexCatalogEvaluation {
        schema_version: CATALOG_SCHEMA_VERSION,
        adapter_id: CODEX_ADAPTER_ID.into(),
        state: state.into(),
        reason_code: reason_code.into(),
        candidate_id: candidate_id.map(str::to_string),
        binary_identity_digest: identity_digest.map(str::to_string),
        version_state: "not_observed".into(),
        runnable: false,
        supported: false,
        process_start_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
    }
}

pub(crate) fn plan_codex_version_probe(
    snapshot: &CodexCommandSnapshot,
) -> Result<CodexProbePlan, String> {
    let evaluation = evaluate_codex_command_snapshot(snapshot)?;
    if evaluation.state != "present_unprobed" {
        return Err(format!(
            "Codex fixed catalog is not ready for a version probe: {}",
            evaluation.reason_code
        ));
    }
    Ok(CodexProbePlan {
        schema_version: PROBE_SCHEMA_VERSION,
        adapter_id: CODEX_ADAPTER_ID.into(),
        candidate_id: evaluation
            .candidate_id
            .expect("present catalog evaluation has candidate identity"),
        binary_identity_digest: evaluation
            .binary_identity_digest
            .expect("present catalog evaluation has binary identity"),
        argument: CODEX_VERSION_ARGUMENT.into(),
        stdin_policy: "null".into(),
        output_policy: "bounded_stdout_discard_stderr".into(),
        timeout_milliseconds: VERSION_PROBE_TIMEOUT_MILLISECONDS,
        max_output_bytes: MAX_VERSION_OUTPUT_BYTES,
        shell_enabled: false,
        working_directory_enabled: false,
        inherited_environment_enabled: false,
        process_start_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
    })
}
pub(crate) fn evaluate_codex_version_probe(
    plan: &CodexProbePlan,
    observation: CodexVersionProbeObservation,
) -> Result<CodexVersionProbeEvaluation, String> {
    validate_probe_plan(plan)?;
    if observation.schema_version != PROBE_SCHEMA_VERSION {
        return Err("Codex version probe observation schema is unsupported".into());
    }
    if observation.candidate_id != plan.candidate_id
        || observation.identity_digest_before != plan.binary_identity_digest
    {
        return Err("Codex version probe does not match the selected binary identity".into());
    }
    validate_digest(
        &observation.identity_digest_after,
        "post-probe Codex binary identity digest",
    )
    .map_err(|error| error.to_string())?;
    if observation.identity_digest_after != observation.identity_digest_before {
        return Err("Codex binary identity changed during the version probe".into());
    }
    let CodexProbeOutcome::Completed {
        exit_success,
        output_truncated,
        version_output,
    } = observation.outcome
    else {
        return Err("Codex version probe did not complete within its fixed policy".into());
    };
    if !exit_success || output_truncated {
        return Err("Codex version probe did not complete cleanly within its fixed bounds".into());
    }
    if version_output.len() > plan.max_output_bytes {
        return Err("Codex version output exceeds the fixed byte bound".into());
    }
    let normalized_version = parse_codex_version(&version_output)?;
    Ok(CodexVersionProbeEvaluation {
        schema_version: PROBE_SCHEMA_VERSION,
        adapter_id: CODEX_ADAPTER_ID.into(),
        candidate_id: plan.candidate_id.clone(),
        binary_identity_digest: plan.binary_identity_digest.clone(),
        normalized_version,
        probe_state: "version_observed".into(),
        manual_harness_required: true,
        runnable: false,
        supported: false,
        process_start_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
    })
}
fn validate_probe_plan(plan: &CodexProbePlan) -> Result<(), String> {
    if plan.schema_version != PROBE_SCHEMA_VERSION
        || plan.adapter_id != CODEX_ADAPTER_ID
        || !codex_command_catalog()
            .iter()
            .any(|entry| entry.candidate_id == plan.candidate_id)
        || plan.argument != CODEX_VERSION_ARGUMENT
        || plan.stdin_policy != "null"
        || plan.output_policy != "bounded_stdout_discard_stderr"
        || plan.timeout_milliseconds != VERSION_PROBE_TIMEOUT_MILLISECONDS
        || plan.max_output_bytes != MAX_VERSION_OUTPUT_BYTES
        || plan.shell_enabled
        || plan.working_directory_enabled
        || plan.inherited_environment_enabled
        || plan.process_start_enabled
        || plan.provider_traffic != "none"
        || plan.writes_enabled
    {
        return Err("Codex version probe plan violates the fixed non-executing policy".into());
    }
    validate_digest(&plan.binary_identity_digest, "Codex binary identity digest")
        .map_err(|error| error.to_string())
}
fn parse_codex_version(output: &str) -> Result<String, String> {
    let line = output
        .strip_suffix("\r\n")
        .or_else(|| output.strip_suffix('\n'))
        .unwrap_or(output);
    if !line.is_ascii() || line.chars().any(char::is_control) {
        return Err("Codex version output contains unsupported characters".into());
    }
    let version = line
        .strip_prefix(CODEX_VERSION_PREFIX)
        .ok_or_else(|| "Codex version output identifies an unexpected product".to_string())?;
    if version.is_empty() || version.trim() != version || !valid_version_shape(version) {
        return Err("Codex version output does not match the bounded version syntax".into());
    }
    Ok(version.to_string())
}
fn valid_version_shape(version: &str) -> bool {
    let core_and_pre = version.split_once('+').map_or(version, |(left, build)| {
        if build.is_empty() || !valid_version_suffix(build) {
            return "";
        }
        left
    });
    let core = core_and_pre
        .split_once('-')
        .map_or(core_and_pre, |(left, pre)| {
            if pre.is_empty() || !valid_version_suffix(pre) {
                return "";
            }
            left
        });
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
}
fn valid_version_suffix(value: &str) -> bool {
    value.split('.').all(|part| {
        !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}
