//! Pure readiness contract for a future opt-in Codex version harness.
//!
//! This module accepts only content-free evidence. It does not inspect paths,
//! start a process, construct a command, persist state, or grant execution.

use super::codex_command_catalog::{validate_probe_plan, CodexProbePlan};
pub(super) use super::codex_macho::{CodexMachOArchitecture, CodexMachOFileType};
use super::codex_npm_chain_model::{
    validate_codex_npm_launcher_chain_observation, CodexNpmLauncherChainObservation,
};
use super::codex_probe_preflight_digest::{
    bounded_digest, containment_digest, containment_digests, launcher_chain_digest,
    probe_plan_digest,
};
use super::codex_probe_semver::is_strict_semver;
use super::events::validate_identifier;
use super::process_run_spec::{process_run_spec_digest, ProcessRunSpec};
use super::session::validate_digest;

const PREFLIGHT_SCHEMA_VERSION: u32 = 2;
const CODEX_ADAPTER_ID: &str = "codex";
const CONTAINMENT_PROFILE_ID: &str = "macos-restricted-helper-v1";
const NPM_MANIFEST_NAME: &str = "@openai/codex";
const NPM_ROOT_BIN_NAME: &str = "codex";
const NPM_ROOT_BIN_RELATIVE_PATH: &str = "bin/codex.js";
const REQUIRED_DISPOSABLE_ROOTS: u8 = 5;
const TERM_GRACE_MILLISECONDS: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexLauncherChainKind {
    DirectMachO,
    SuppliedNpmPlatformPackageV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexNpmPayloadLayout {
    VendorTargetBinV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexProbeTargetObservation {
    pub schema_version: u32,
    pub candidate_id: String,
    pub launcher_identity_digest: String,
    pub chain_kind: CodexLauncherChainKind,
    pub npm_root_manifest_identity_digest: Option<String>,
    pub npm_root_manifest_name: Option<String>,
    pub npm_root_bin_name: Option<String>,
    pub npm_root_bin_relative_path: Option<String>,
    pub npm_dependency_alias: Option<String>,
    pub npm_dependency_version_spec: Option<String>,
    pub npm_platform_manifest_identity_digest: Option<String>,
    pub npm_platform_manifest_name: Option<String>,
    pub npm_root_version: Option<String>,
    pub npm_platform_version: Option<String>,
    pub npm_target_triple: Option<String>,
    pub npm_payload_layout: Option<CodexNpmPayloadLayout>,
    pub npm_launcher_symlink_identity_digest: Option<String>,
    pub npm_payload_manifest_identity_digest: Option<String>,
    pub npm_derivation_identity_digest: Option<String>,
    pub npm_collection_identity_digest: Option<String>,
    pub target_identity_digest: String,
    pub target_architecture: CodexMachOArchitecture,
    pub target_is_regular_file: bool,
    pub target_is_executable: bool,
    pub macho_class_64: bool,
    pub macho_file_type: CodexMachOFileType,
    pub macho_load_commands_identity_digest: String,
    pub code_signature_blob_identity_digest: Option<String>,
    pub interpreter_launcher_selected_for_execution: bool,
    pub path_lookup_used: bool,
}

pub(super) enum CodexProbeTargetEvidence<'a> {
    Direct(&'a CodexProbeTargetObservation),
    CollectedNpm(&'a CodexNpmLauncherChainObservation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexPreflightTargetOrigin {
    DirectMachO,
    CollectedNpmSchemaV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexProbeContainmentObservation {
    pub schema_version: u32,
    pub profile_id: String,
    pub attempt_id: String,
    pub host_instance_identity_digest: String,
    pub boot_session_identity_digest: String,
    pub os_build_identity_digest: String,
    pub app_code_identity_digest: String,
    pub app_entitlements_identity_digest: String,
    pub helper_code_identity_digest: String,
    pub helper_entitlements_identity_digest: String,
    pub enforcement_policy_identity_digest: String,
    pub network_deny_evidence_digest: String,
    pub filesystem_scope_evidence_digest: String,
    pub process_group_evidence_digest: String,
    pub timeout_evidence_digest: String,
    pub disposable_roots_identity_digest: String,
    pub disposable_root_count: u8,
    pub helper_is_distinct_restricted_identity: bool,
    pub helper_entitlements_are_narrower_than_app: bool,
    pub sandbox_enforced: bool,
    pub sandbox_failure_is_fatal: bool,
    pub network_denied: bool,
    pub writes_denied_outside_disposable_roots: bool,
    pub read_scope_is_allowlisted: bool,
    pub working_directory_is_disposable: bool,
    pub environment_cleared: bool,
    pub environment_values_are_disposable: bool,
    pub provider_credentials_inherited: bool,
    pub shell_enabled: bool,
    pub path_lookup_enabled: bool,
    pub process_group_owned: bool,
    pub stdin_null: bool,
    pub stderr_discarded: bool,
    pub stdout_fully_drained: bool,
    pub max_output_bytes: usize,
    pub timeout_milliseconds: u64,
    pub term_grace_milliseconds: u64,
    pub kill_after_grace: bool,
    pub descendants_reaped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexManualProbePreflight {
    target_origin: CodexPreflightTargetOrigin,
    pub schema_version: u32,
    pub adapter_id: String,
    pub attempt_id: String,
    pub process_run_id: String,
    pub process_run_spec_digest: String,
    pub probe_plan_digest: String,
    pub candidate_id: String,
    pub launcher_chain_identity_digest: String,
    pub containment_identity_digest: String,
    pub preflight_identity_digest: String,
    pub state: String,
    pub reason_code: String,
    pub manual_opt_in_required: bool,
    pub runnable: bool,
    pub supported: bool,
    pub process_start_enabled: bool,
    pub provider_traffic: String,
    pub user_workspace_writes_enabled: bool,
}

pub(super) fn evaluate_codex_manual_probe_preflight(
    process_spec: &ProcessRunSpec,
    probe_plan: &CodexProbePlan,
    host_architecture: CodexMachOArchitecture,
    target: CodexProbeTargetEvidence<'_>,
    containment: &CodexProbeContainmentObservation,
) -> Result<CodexManualProbePreflight, String> {
    process_spec.validate().map_err(|error| error.to_string())?;
    if process_spec.adapter_id != CODEX_ADAPTER_ID {
        return Err("Codex probe preflight requires the canonical Codex process spec".into());
    }
    validate_probe_plan(probe_plan)?;
    let normalized_target;
    let (target, target_origin) = match target {
        CodexProbeTargetEvidence::Direct(target) => {
            if target.chain_kind != CodexLauncherChainKind::DirectMachO {
                return Err("Raw npm target evidence is not accepted by Codex preflight".into());
            }
            (target, CodexPreflightTargetOrigin::DirectMachO)
        }
        CodexProbeTargetEvidence::CollectedNpm(receipt) => {
            validate_codex_npm_launcher_chain_observation(host_architecture, receipt)?;
            normalized_target = normalize_collected_npm_target(receipt);
            (
                &normalized_target,
                CodexPreflightTargetOrigin::CollectedNpmSchemaV2,
            )
        }
    };
    validate_target(probe_plan, host_architecture, target)?;
    validate_containment(probe_plan, containment)?;
    let process_digest =
        process_run_spec_digest(process_spec).map_err(|error| error.to_string())?;
    let probe_digest = probe_plan_digest(probe_plan);
    let chain_digest = launcher_chain_digest(target);
    let containment_digest = containment_digest(containment);
    let preflight_digest = bounded_digest(
        b"ai-switchboard-codex-probe-preflight-v2\0",
        &[
            process_spec.run_id.as_str(),
            process_digest.as_str(),
            probe_digest.as_str(),
            chain_digest.as_str(),
            containment_digest.as_str(),
        ],
    );
    Ok(CodexManualProbePreflight {
        target_origin,
        schema_version: PREFLIGHT_SCHEMA_VERSION,
        adapter_id: CODEX_ADAPTER_ID.into(),
        attempt_id: containment.attempt_id.clone(),
        process_run_id: process_spec.run_id.clone(),
        process_run_spec_digest: process_digest,
        probe_plan_digest: probe_digest,
        candidate_id: probe_plan.candidate_id.clone(),
        launcher_chain_identity_digest: chain_digest,
        containment_identity_digest: containment_digest,
        preflight_identity_digest: preflight_digest,
        state: if target_origin == CodexPreflightTargetOrigin::CollectedNpmSchemaV2 {
            "collected_target_shape_complete_non_executing"
        } else {
            "supplied_evidence_shape_complete_non_executing"
        }
        .into(),
        reason_code: if target_origin == CodexPreflightTargetOrigin::CollectedNpmSchemaV2 {
            "restricted_helper_and_manual_harness_still_required"
        } else {
            "native_collection_and_manual_harness_still_required"
        }
        .into(),
        manual_opt_in_required: true,
        runnable: false,
        supported: false,
        process_start_enabled: false,
        provider_traffic: "none".into(),
        user_workspace_writes_enabled: false,
    })
}

impl CodexManualProbePreflight {
    pub(super) fn has_collected_npm_origin(&self) -> bool {
        self.target_origin == CodexPreflightTargetOrigin::CollectedNpmSchemaV2
    }

    pub(super) fn validate_for_collected_helper(
        &self,
        process_spec: &ProcessRunSpec,
        probe_plan: &CodexProbePlan,
        containment: &CodexProbeContainmentObservation,
    ) -> Result<(), String> {
        if !self.has_collected_npm_origin() {
            return Err("Codex helper requires collected npm preflight provenance".into());
        }
        process_spec.validate().map_err(|error| error.to_string())?;
        validate_probe_plan(probe_plan)?;
        validate_containment(probe_plan, containment)?;
        validate_identifier(&self.attempt_id, "Codex probe attempt ID")
            .map_err(|error| error.to_string())?;
        validate_identifier(&self.process_run_id, "process run ID")
            .map_err(|error| error.to_string())?;
        validate_identifier(&self.candidate_id, "Codex candidate ID")
            .map_err(|error| error.to_string())?;
        for (digest, label) in [
            (&self.process_run_spec_digest, "process run spec"),
            (&self.probe_plan_digest, "Codex probe plan"),
            (
                &self.launcher_chain_identity_digest,
                "Codex launcher chain identity",
            ),
            (
                &self.containment_identity_digest,
                "Codex containment identity",
            ),
            (&self.preflight_identity_digest, "Codex preflight identity"),
        ] {
            validate_digest(digest, label).map_err(|error| error.to_string())?;
        }
        let process_digest =
            process_run_spec_digest(process_spec).map_err(|error| error.to_string())?;
        let probe_digest = probe_plan_digest(probe_plan);
        let containment_digest = containment_digest(containment);
        let expected_preflight_digest = bounded_digest(
            b"ai-switchboard-codex-probe-preflight-v2\0",
            &[
                process_spec.run_id.as_str(),
                process_digest.as_str(),
                probe_digest.as_str(),
                self.launcher_chain_identity_digest.as_str(),
                containment_digest.as_str(),
            ],
        );
        if self.schema_version != PREFLIGHT_SCHEMA_VERSION
            || self.adapter_id != CODEX_ADAPTER_ID
            || self.attempt_id != containment.attempt_id
            || self.process_run_id != process_spec.run_id
            || self.process_run_spec_digest != process_digest
            || self.probe_plan_digest != probe_digest
            || self.candidate_id != probe_plan.candidate_id
            || self.containment_identity_digest != containment_digest
            || self.preflight_identity_digest != expected_preflight_digest
            || self.state != "collected_target_shape_complete_non_executing"
            || self.reason_code != "restricted_helper_and_manual_harness_still_required"
            || !self.manual_opt_in_required
            || self.runnable
            || self.supported
            || self.process_start_enabled
            || self.provider_traffic != "none"
            || self.user_workspace_writes_enabled
        {
            return Err("Codex collected preflight is invalid or has been modified".into());
        }
        Ok(())
    }
}

fn normalize_collected_npm_target(
    receipt: &CodexNpmLauncherChainObservation,
) -> CodexProbeTargetObservation {
    CodexProbeTargetObservation {
        schema_version: PREFLIGHT_SCHEMA_VERSION,
        candidate_id: receipt.candidate_id.clone(),
        launcher_identity_digest: receipt.launcher_identity_digest.clone(),
        chain_kind: CodexLauncherChainKind::SuppliedNpmPlatformPackageV1,
        npm_root_manifest_identity_digest: Some(receipt.root_manifest_identity_digest.clone()),
        npm_root_manifest_name: Some(NPM_MANIFEST_NAME.into()),
        npm_root_bin_name: Some(NPM_ROOT_BIN_NAME.into()),
        npm_root_bin_relative_path: Some(NPM_ROOT_BIN_RELATIVE_PATH.into()),
        npm_dependency_alias: Some(receipt.dependency_alias.clone()),
        npm_dependency_version_spec: Some(receipt.dependency_version_spec.clone()),
        npm_platform_manifest_identity_digest: Some(
            receipt.platform_manifest_identity_digest.clone(),
        ),
        npm_platform_manifest_name: Some(NPM_MANIFEST_NAME.into()),
        npm_root_version: Some(receipt.root_version.clone()),
        npm_platform_version: Some(receipt.platform_version.clone()),
        npm_target_triple: Some(receipt.payload_target.clone()),
        npm_payload_layout: Some(CodexNpmPayloadLayout::VendorTargetBinV1),
        npm_launcher_symlink_identity_digest: Some(
            receipt.launcher_symlink_identity_digest.clone(),
        ),
        npm_payload_manifest_identity_digest: Some(
            receipt.payload_manifest_identity_digest.clone(),
        ),
        npm_derivation_identity_digest: Some(receipt.derivation_identity_digest.clone()),
        npm_collection_identity_digest: Some(receipt.collection_identity_digest.clone()),
        target_identity_digest: receipt.payload_file_identity_digest.clone(),
        target_architecture: receipt.payload_macho_architecture,
        target_is_regular_file: true,
        target_is_executable: true,
        macho_class_64: true,
        macho_file_type: receipt.payload_macho_file_type,
        macho_load_commands_identity_digest: receipt
            .payload_macho_load_commands_identity_digest
            .clone(),
        code_signature_blob_identity_digest: receipt
            .payload_code_signature_blob_identity_digest
            .clone(),
        interpreter_launcher_selected_for_execution: false,
        path_lookup_used: false,
    }
}

fn validate_target(
    probe_plan: &CodexProbePlan,
    host_architecture: CodexMachOArchitecture,
    target: &CodexProbeTargetObservation,
) -> Result<(), String> {
    if target.schema_version != PREFLIGHT_SCHEMA_VERSION
        || target.candidate_id != probe_plan.candidate_id
        || target.launcher_identity_digest != probe_plan.binary_identity_digest
    {
        return Err("Codex probe target does not match the fixed candidate identity".into());
    }
    for (digest, label) in [
        (&target.launcher_identity_digest, "Codex launcher identity"),
        (&target.target_identity_digest, "Codex target identity"),
        (
            &target.macho_load_commands_identity_digest,
            "Codex Mach-O load commands identity",
        ),
    ] {
        validate_digest(digest, label).map_err(|error| error.to_string())?;
    }
    if let Some(digest) = target.code_signature_blob_identity_digest.as_deref() {
        validate_digest(digest, "Codex code-signature blob identity")
            .map_err(|error| error.to_string())?;
    }
    if target.target_architecture != host_architecture
        || !target.target_is_regular_file
        || !target.target_is_executable
        || !target.macho_class_64
        || target.macho_file_type != CodexMachOFileType::Execute
        || target.interpreter_launcher_selected_for_execution
        || target.path_lookup_used
    {
        return Err("Codex probe target violates the native fixed-target policy".into());
    }
    match target.chain_kind {
        CodexLauncherChainKind::DirectMachO => validate_direct_chain(target),
        CodexLauncherChainKind::SuppliedNpmPlatformPackageV1 => {
            validate_npm_chain(host_architecture, target)
        }
    }
}

fn validate_direct_chain(target: &CodexProbeTargetObservation) -> Result<(), String> {
    let npm_fields_absent = target.npm_root_manifest_identity_digest.is_none()
        && target.npm_root_manifest_name.is_none()
        && target.npm_root_bin_name.is_none()
        && target.npm_root_bin_relative_path.is_none()
        && target.npm_dependency_alias.is_none()
        && target.npm_dependency_version_spec.is_none()
        && target.npm_platform_manifest_identity_digest.is_none()
        && target.npm_platform_manifest_name.is_none()
        && target.npm_root_version.is_none()
        && target.npm_platform_version.is_none()
        && target.npm_target_triple.is_none()
        && target.npm_payload_layout.is_none()
        && target.npm_launcher_symlink_identity_digest.is_none()
        && target.npm_payload_manifest_identity_digest.is_none()
        && target.npm_derivation_identity_digest.is_none()
        && target.npm_collection_identity_digest.is_none();
    if !npm_fields_absent || target.target_identity_digest != target.launcher_identity_digest {
        return Err("Direct Codex target identity is not self-contained".into());
    }
    Ok(())
}

fn validate_npm_chain(
    host_architecture: CodexMachOArchitecture,
    target: &CodexProbeTargetObservation,
) -> Result<(), String> {
    let root_manifest = required_option(
        target.npm_root_manifest_identity_digest.as_deref(),
        "Codex npm root manifest identity",
    )?;
    let platform_manifest = required_option(
        target.npm_platform_manifest_identity_digest.as_deref(),
        "Codex npm platform manifest identity",
    )?;
    validate_digest(root_manifest, "Codex npm root manifest identity")
        .map_err(|error| error.to_string())?;
    validate_digest(platform_manifest, "Codex npm platform manifest identity")
        .map_err(|error| error.to_string())?;
    for (digest, label) in [
        (
            required_option(
                target.npm_launcher_symlink_identity_digest.as_deref(),
                "Codex npm launcher symlink identity",
            )?,
            "Codex npm launcher symlink identity",
        ),
        (
            required_option(
                target.npm_payload_manifest_identity_digest.as_deref(),
                "Codex npm payload manifest identity",
            )?,
            "Codex npm payload manifest identity",
        ),
        (
            required_option(
                target.npm_derivation_identity_digest.as_deref(),
                "Codex npm derivation identity",
            )?,
            "Codex npm derivation identity",
        ),
        (
            required_option(
                target.npm_collection_identity_digest.as_deref(),
                "Codex npm collection identity",
            )?,
            "Codex npm collection identity",
        ),
    ] {
        validate_digest(digest, label).map_err(|error| error.to_string())?;
    }
    let (alias, suffix, target_triple) = match host_architecture {
        CodexMachOArchitecture::Arm64 => (
            "@openai/codex-darwin-arm64",
            "darwin-arm64",
            "aarch64-apple-darwin",
        ),
        CodexMachOArchitecture::X86_64 => (
            "@openai/codex-darwin-x64",
            "darwin-x64",
            "x86_64-apple-darwin",
        ),
    };
    let root_version = required_option(target.npm_root_version.as_deref(), "Codex npm version")?;
    let expected_platform_version = format!("{root_version}-{suffix}");
    let expected_dependency_spec = format!("npm:{NPM_MANIFEST_NAME}@{expected_platform_version}");
    if !is_strict_semver(root_version)
        || target.npm_root_manifest_name.as_deref() != Some(NPM_MANIFEST_NAME)
        || target.npm_root_bin_name.as_deref() != Some(NPM_ROOT_BIN_NAME)
        || target.npm_root_bin_relative_path.as_deref() != Some(NPM_ROOT_BIN_RELATIVE_PATH)
        || target.npm_dependency_alias.as_deref() != Some(alias)
        || target.npm_dependency_version_spec.as_deref() != Some(expected_dependency_spec.as_str())
        || target.npm_platform_manifest_name.as_deref() != Some(NPM_MANIFEST_NAME)
        || target.npm_platform_version.as_deref() != Some(expected_platform_version.as_str())
        || target.npm_target_triple.as_deref() != Some(target_triple)
        || target.npm_payload_layout != Some(CodexNpmPayloadLayout::VendorTargetBinV1)
        || target.target_identity_digest == target.launcher_identity_digest
    {
        return Err("Codex npm launcher chain violates the approved package policy".into());
    }
    Ok(())
}

pub(super) fn validate_containment(
    probe_plan: &CodexProbePlan,
    value: &CodexProbeContainmentObservation,
) -> Result<(), String> {
    validate_identifier(&value.attempt_id, "Codex probe attempt ID")
        .map_err(|error| error.to_string())?;
    for (digest, label) in containment_digests(value) {
        validate_digest(digest, label).map_err(|error| error.to_string())?;
    }
    if value.schema_version != PREFLIGHT_SCHEMA_VERSION
        || value.profile_id != CONTAINMENT_PROFILE_ID
        || value.disposable_root_count != REQUIRED_DISPOSABLE_ROOTS
        || !value.helper_is_distinct_restricted_identity
        || !value.helper_entitlements_are_narrower_than_app
        || value.app_code_identity_digest == value.helper_code_identity_digest
        || value.app_entitlements_identity_digest == value.helper_entitlements_identity_digest
        || !value.sandbox_enforced
        || !value.sandbox_failure_is_fatal
        || !value.network_denied
        || !value.writes_denied_outside_disposable_roots
        || !value.read_scope_is_allowlisted
        || !value.working_directory_is_disposable
        || !value.environment_cleared
        || !value.environment_values_are_disposable
        || value.provider_credentials_inherited
        || value.shell_enabled
        || value.path_lookup_enabled
        || !value.process_group_owned
        || !value.stdin_null
        || !value.stderr_discarded
        || !value.stdout_fully_drained
        || value.max_output_bytes != probe_plan.max_output_bytes
        || value.timeout_milliseconds != probe_plan.timeout_milliseconds
        || value.term_grace_milliseconds != TERM_GRACE_MILLISECONDS
        || !value.kill_after_grace
        || !value.descendants_reaped
    {
        return Err("Codex manual probe containment evidence violates the fixed policy".into());
    }
    Ok(())
}

fn required_option<'a>(value: Option<&'a str>, label: &str) -> Result<&'a str, String> {
    value.ok_or_else(|| format!("{label} is missing"))
}
