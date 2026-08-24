use sha2::{Digest, Sha256};

use super::codex_command_catalog::{
    codex_command_catalog, plan_codex_version_probe, CodexCandidateObservation,
    CodexCommandSnapshot, CodexResolvedCandidateKind,
};
use super::codex_npm_chain_model::{
    bind_codex_npm_launcher_chain, CodexNpmCollectedEvidence, CodexNpmLauncherChainObservation,
};
use super::codex_probe_preflight::{
    evaluate_codex_manual_probe_preflight, CodexLauncherChainKind, CodexMachOArchitecture,
    CodexMachOFileType, CodexProbeContainmentObservation, CodexProbeTargetEvidence,
    CodexProbeTargetObservation,
};
use super::process_run_spec::{process_run_spec_for, ProcessRunSpec};

pub(super) fn digest(character: char) -> String {
    format!(
        "sha256:{:x}",
        Sha256::digest(character.to_string().as_bytes())
    )
}

pub(super) fn probe_plan() -> super::codex_command_catalog::CodexProbePlan {
    let selected = codex_command_catalog()
        .iter()
        .find(|entry| entry.candidate_id == "home-npm-global-bin")
        .expect("fixed npm candidate");
    let observations = codex_command_catalog()
        .iter()
        .map(|entry| {
            if entry.candidate_id == selected.candidate_id {
                CodexCandidateObservation::Present {
                    candidate_id: entry.candidate_id.into(),
                    resolved_kind: CodexResolvedCandidateKind::RegularFile,
                    executable: true,
                    identity_digest: Some(digest('1')),
                }
            } else {
                CodexCandidateObservation::ConfirmedAbsent {
                    candidate_id: entry.candidate_id.into(),
                }
            }
        })
        .collect();
    plan_codex_version_probe(&CodexCommandSnapshot {
        schema_version: 1,
        observations,
    })
    .expect("fixed probe plan")
}

pub(super) fn process_spec() -> ProcessRunSpec {
    process_run_spec_for(
        "workbench:probe-test",
        "codex-1234567890ab",
        "codex",
        &digest('w'),
    )
    .expect("Codex process spec")
}

pub(super) fn direct_target() -> CodexProbeTargetObservation {
    let plan = probe_plan();
    CodexProbeTargetObservation {
        schema_version: 2,
        candidate_id: plan.candidate_id,
        launcher_identity_digest: digest('1'),
        chain_kind: CodexLauncherChainKind::DirectMachO,
        npm_root_manifest_identity_digest: None,
        npm_root_manifest_name: None,
        npm_root_bin_name: None,
        npm_root_bin_relative_path: None,
        npm_dependency_alias: None,
        npm_dependency_version_spec: None,
        npm_platform_manifest_identity_digest: None,
        npm_platform_manifest_name: None,
        npm_root_version: None,
        npm_platform_version: None,
        npm_target_triple: None,
        npm_payload_layout: None,
        npm_launcher_symlink_identity_digest: None,
        npm_payload_manifest_identity_digest: None,
        npm_derivation_identity_digest: None,
        npm_collection_identity_digest: None,
        target_identity_digest: digest('1'),
        target_architecture: CodexMachOArchitecture::Arm64,
        target_is_regular_file: true,
        target_is_executable: true,
        macho_class_64: true,
        macho_file_type: CodexMachOFileType::Execute,
        macho_load_commands_identity_digest: digest('2'),
        code_signature_blob_identity_digest: Some(digest('3')),
        interpreter_launcher_selected_for_execution: false,
        path_lookup_used: false,
    }
}

pub(super) fn npm_receipt() -> CodexNpmLauncherChainObservation {
    npm_receipt_with_signature(Some(digest('3')))
}

pub(super) fn npm_receipt_without_signature() -> CodexNpmLauncherChainObservation {
    npm_receipt_with_signature(None)
}

fn npm_receipt_with_signature(signature: Option<String>) -> CodexNpmLauncherChainObservation {
    bind_codex_npm_launcher_chain(
        CodexMachOArchitecture::Arm64,
        CodexNpmCollectedEvidence {
            candidate_id: "home-npm-global-bin".into(),
            launcher_identity_digest: digest('1'),
            launcher_symlink_identity_digest: digest('s'),
            root_manifest_identity_digest: digest('4'),
            root_package_name: "@openai/codex".into(),
            root_version: "1.2.3".into(),
            root_bin_name: "codex".into(),
            root_bin_entrypoint: "bin/codex.js".into(),
            dependency_alias: "@openai/codex-darwin-arm64".into(),
            dependency_version_spec: "npm:@openai/codex@1.2.3-darwin-arm64".into(),
            platform_manifest_identity_digest: digest('5'),
            platform_package_name: "@openai/codex".into(),
            platform_version: "1.2.3-darwin-arm64".into(),
            platform_os: "darwin".into(),
            platform_cpu: "arm64".into(),
            payload_manifest_identity_digest: digest('m'),
            payload_layout_version: 1,
            payload_version: "1.2.3".into(),
            payload_target: "aarch64-apple-darwin".into(),
            payload_variant: "codex".into(),
            payload_entrypoint: "bin/codex".into(),
            payload_resources_directory: "codex-resources".into(),
            payload_path_directory: "codex-path".into(),
            payload_file_identity_digest: digest('6'),
            payload_macho_architecture: CodexMachOArchitecture::Arm64,
            payload_macho_file_type: CodexMachOFileType::Execute,
            payload_macho_load_commands_identity_digest: digest('2'),
            payload_code_signature_blob_identity_digest: signature,
            derivation_identity_digest: digest('d'),
            payload_file_is_regular: true,
            payload_file_is_executable: true,
        },
    )
    .expect("valid npm receipt")
}

pub(super) fn containment() -> CodexProbeContainmentObservation {
    let plan = probe_plan();
    CodexProbeContainmentObservation {
        schema_version: 2,
        profile_id: "macos-restricted-helper-v1".into(),
        attempt_id: "codex-probe:attempt-1234".into(),
        host_instance_identity_digest: digest('h'),
        boot_session_identity_digest: digest('j'),
        os_build_identity_digest: digest('a'),
        app_code_identity_digest: digest('b'),
        app_entitlements_identity_digest: digest('g'),
        helper_code_identity_digest: digest('c'),
        helper_entitlements_identity_digest: digest('d'),
        enforcement_policy_identity_digest: digest('e'),
        network_deny_evidence_digest: digest('f'),
        filesystem_scope_evidence_digest: digest('7'),
        process_group_evidence_digest: digest('8'),
        timeout_evidence_digest: digest('9'),
        disposable_roots_identity_digest: digest('0'),
        disposable_root_count: 5,
        helper_is_distinct_restricted_identity: true,
        helper_entitlements_are_narrower_than_app: true,
        sandbox_enforced: true,
        sandbox_failure_is_fatal: true,
        network_denied: true,
        writes_denied_outside_disposable_roots: true,
        read_scope_is_allowlisted: true,
        working_directory_is_disposable: true,
        environment_cleared: true,
        environment_values_are_disposable: true,
        provider_credentials_inherited: false,
        shell_enabled: false,
        path_lookup_enabled: false,
        process_group_owned: true,
        stdin_null: true,
        stderr_discarded: true,
        stdout_fully_drained: true,
        max_output_bytes: plan.max_output_bytes,
        timeout_milliseconds: plan.timeout_milliseconds,
        term_grace_milliseconds: 250,
        kill_after_grace: true,
        descendants_reaped: true,
    }
}

pub(super) fn evaluate(
    spec: &ProcessRunSpec,
    target: &CodexProbeTargetObservation,
    containment: &CodexProbeContainmentObservation,
) -> Result<super::codex_probe_preflight::CodexManualProbePreflight, String> {
    evaluate_codex_manual_probe_preflight(
        spec,
        &probe_plan(),
        CodexMachOArchitecture::Arm64,
        CodexProbeTargetEvidence::Direct(target),
        containment,
    )
}

pub(super) fn evaluate_npm(
    spec: &ProcessRunSpec,
    receipt: &CodexNpmLauncherChainObservation,
    containment: &CodexProbeContainmentObservation,
) -> Result<super::codex_probe_preflight::CodexManualProbePreflight, String> {
    evaluate_codex_manual_probe_preflight(
        spec,
        &probe_plan(),
        CodexMachOArchitecture::Arm64,
        CodexProbeTargetEvidence::CollectedNpm(receipt),
        containment,
    )
}
