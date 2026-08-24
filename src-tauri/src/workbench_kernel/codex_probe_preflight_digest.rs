use sha2::{Digest, Sha256};

use super::codex_command_catalog::CodexProbePlan;
use super::codex_probe_preflight::{
    CodexLauncherChainKind, CodexMachOArchitecture, CodexProbeContainmentObservation,
    CodexProbeTargetObservation,
};

pub(super) fn probe_plan_digest(plan: &CodexProbePlan) -> String {
    let timeout_milliseconds = plan.timeout_milliseconds.to_string();
    let max_output_bytes = plan.max_output_bytes.to_string();
    bounded_digest(
        b"ai-switchboard-codex-probe-plan-v1\0",
        &[
            plan.adapter_id.as_str(),
            plan.candidate_id.as_str(),
            plan.binary_identity_digest.as_str(),
            plan.argument.as_str(),
            plan.stdin_policy.as_str(),
            plan.output_policy.as_str(),
            timeout_milliseconds.as_str(),
            max_output_bytes.as_str(),
        ],
    )
}

pub(super) fn launcher_chain_digest(target: &CodexProbeTargetObservation) -> String {
    let schema_version = target.schema_version.to_string();
    let kind = match target.chain_kind {
        CodexLauncherChainKind::DirectMachO => "direct-macho",
        CodexLauncherChainKind::SuppliedNpmPlatformPackageV1 => "supplied-npm-platform-package-v1",
    };
    let payload_layout = match target.npm_payload_layout {
        Some(super::codex_probe_preflight::CodexNpmPayloadLayout::VendorTargetBinV1) => {
            "vendor-target-bin-v1"
        }
        None => "none",
    };
    let macho_file_type = match target.macho_file_type {
        super::codex_probe_preflight::CodexMachOFileType::Execute => "execute",
        super::codex_probe_preflight::CodexMachOFileType::DynamicLibrary => "dynamic-library",
        super::codex_probe_preflight::CodexMachOFileType::Other => "other",
    };
    let target_policy_flags = bool_flags(&[
        target.target_is_regular_file,
        target.target_is_executable,
        target.macho_class_64,
        target.interpreter_launcher_selected_for_execution,
        target.path_lookup_used,
    ]);
    bounded_digest(
        b"ai-switchboard-codex-launcher-chain-v2\0",
        &[
            schema_version.as_str(),
            target.candidate_id.as_str(),
            kind,
            architecture_id(target.target_architecture),
            target.launcher_identity_digest.as_str(),
            target
                .npm_root_manifest_identity_digest
                .as_deref()
                .unwrap_or("none"),
            target.npm_root_manifest_name.as_deref().unwrap_or("none"),
            target.npm_root_bin_name.as_deref().unwrap_or("none"),
            target
                .npm_root_bin_relative_path
                .as_deref()
                .unwrap_or("none"),
            target.npm_dependency_alias.as_deref().unwrap_or("none"),
            target
                .npm_dependency_version_spec
                .as_deref()
                .unwrap_or("none"),
            target
                .npm_platform_manifest_identity_digest
                .as_deref()
                .unwrap_or("none"),
            target
                .npm_platform_manifest_name
                .as_deref()
                .unwrap_or("none"),
            target.npm_root_version.as_deref().unwrap_or("none"),
            target.npm_platform_version.as_deref().unwrap_or("none"),
            target.npm_target_triple.as_deref().unwrap_or("none"),
            payload_layout,
            target
                .npm_launcher_symlink_identity_digest
                .as_deref()
                .unwrap_or("none"),
            target
                .npm_payload_manifest_identity_digest
                .as_deref()
                .unwrap_or("none"),
            target
                .npm_derivation_identity_digest
                .as_deref()
                .unwrap_or("none"),
            target
                .npm_collection_identity_digest
                .as_deref()
                .unwrap_or("none"),
            target.target_identity_digest.as_str(),
            macho_file_type,
            target.macho_load_commands_identity_digest.as_str(),
            target
                .code_signature_blob_identity_digest
                .as_deref()
                .unwrap_or("none"),
            target_policy_flags.as_str(),
        ],
    )
}

pub(super) fn containment_digest(value: &CodexProbeContainmentObservation) -> String {
    let schema_version = value.schema_version.to_string();
    let disposable_root_count = value.disposable_root_count.to_string();
    let max_output_bytes = value.max_output_bytes.to_string();
    let timeout_milliseconds = value.timeout_milliseconds.to_string();
    let term_grace_milliseconds = value.term_grace_milliseconds.to_string();
    let policy_flags = bool_flags(&[
        value.helper_is_distinct_restricted_identity,
        value.helper_entitlements_are_narrower_than_app,
        value.sandbox_enforced,
        value.sandbox_failure_is_fatal,
        value.network_denied,
        value.writes_denied_outside_disposable_roots,
        value.read_scope_is_allowlisted,
        value.working_directory_is_disposable,
        value.environment_cleared,
        value.environment_values_are_disposable,
        value.provider_credentials_inherited,
        value.shell_enabled,
        value.path_lookup_enabled,
        value.process_group_owned,
        value.stdin_null,
        value.stderr_discarded,
        value.stdout_fully_drained,
        value.kill_after_grace,
        value.descendants_reaped,
    ]);
    let mut values = vec![
        schema_version.as_str(),
        value.profile_id.as_str(),
        value.attempt_id.as_str(),
    ];
    values.extend(
        containment_digests(value)
            .into_iter()
            .map(|(digest, _)| digest),
    );
    values.extend([
        disposable_root_count.as_str(),
        max_output_bytes.as_str(),
        timeout_milliseconds.as_str(),
        term_grace_milliseconds.as_str(),
        policy_flags.as_str(),
    ]);
    bounded_digest(b"ai-switchboard-codex-containment-v1\0", &values)
}

pub(super) fn containment_digests(
    value: &CodexProbeContainmentObservation,
) -> Vec<(&str, &'static str)> {
    vec![
        (
            &value.host_instance_identity_digest,
            "host instance identity",
        ),
        (&value.boot_session_identity_digest, "boot session identity"),
        (&value.os_build_identity_digest, "macOS build identity"),
        (&value.app_code_identity_digest, "app code identity"),
        (
            &value.app_entitlements_identity_digest,
            "app entitlements identity",
        ),
        (
            &value.helper_code_identity_digest,
            "restricted helper code identity",
        ),
        (
            &value.helper_entitlements_identity_digest,
            "restricted helper entitlements identity",
        ),
        (
            &value.enforcement_policy_identity_digest,
            "enforcement policy identity",
        ),
        (&value.network_deny_evidence_digest, "network deny evidence"),
        (
            &value.filesystem_scope_evidence_digest,
            "filesystem scope evidence",
        ),
        (
            &value.process_group_evidence_digest,
            "process group evidence",
        ),
        (&value.timeout_evidence_digest, "timeout evidence"),
        (
            &value.disposable_roots_identity_digest,
            "disposable roots identity",
        ),
    ]
}

fn architecture_id(value: CodexMachOArchitecture) -> &'static str {
    match value {
        CodexMachOArchitecture::Arm64 => "arm64",
        CodexMachOArchitecture::X86_64 => "x86_64",
    }
}

fn bool_flags(values: &[bool]) -> String {
    values
        .iter()
        .map(|value| if *value { '1' } else { '0' })
        .collect()
}

pub(super) fn bounded_digest(domain: &[u8], values: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for value in values {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}
