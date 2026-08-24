//! Content-free evidence model for a collected Codex npm launcher chain.
//!
//! This module grants no path, collection, support, or execution authority.

use sha2::{Digest, Sha256};

use super::codex_macho::{CodexMachOArchitecture, CodexMachOFileType};
use super::codex_probe_semver::is_strict_semver;
use super::session::validate_digest;

const SCHEMA_VERSION: u32 = 2;
const COLLECTION_STATE: &str = "collected_macho_shape_bound_non_executing";
const CANDIDATE_ID: &str = "home-npm-global-bin";
const PACKAGE_NAME: &str = "@openai/codex";
const ROOT_BIN_NAME: &str = "codex";
const ROOT_BIN_ENTRYPOINT: &str = "bin/codex.js";
const PAYLOAD_LAYOUT_VERSION: u32 = 1;
const PAYLOAD_VARIANT: &str = "codex";
const PAYLOAD_ENTRYPOINT: &str = "bin/codex";
const PAYLOAD_RESOURCES_DIRECTORY: &str = "codex-resources";
const PAYLOAD_PATH_DIRECTORY: &str = "codex-path";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexNpmCollectedEvidence {
    pub candidate_id: String,
    pub launcher_identity_digest: String,
    pub launcher_symlink_identity_digest: String,
    pub root_manifest_identity_digest: String,
    pub root_package_name: String,
    pub root_version: String,
    pub root_bin_name: String,
    pub root_bin_entrypoint: String,
    pub dependency_alias: String,
    pub dependency_version_spec: String,
    pub platform_manifest_identity_digest: String,
    pub platform_package_name: String,
    pub platform_version: String,
    pub platform_os: String,
    pub platform_cpu: String,
    pub payload_manifest_identity_digest: String,
    pub payload_layout_version: u32,
    pub payload_version: String,
    pub payload_target: String,
    pub payload_variant: String,
    pub payload_entrypoint: String,
    pub payload_resources_directory: String,
    pub payload_path_directory: String,
    pub payload_file_identity_digest: String,
    pub payload_macho_architecture: CodexMachOArchitecture,
    pub payload_macho_file_type: CodexMachOFileType,
    pub payload_macho_load_commands_identity_digest: String,
    pub payload_code_signature_blob_identity_digest: Option<String>,
    pub derivation_identity_digest: String,
    pub payload_file_is_regular: bool,
    pub payload_file_is_executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexNpmLauncherChainObservation {
    pub schema_version: u32,
    pub candidate_id: String,
    pub launcher_identity_digest: String,
    pub launcher_symlink_identity_digest: String,
    pub root_manifest_identity_digest: String,
    pub root_version: String,
    pub dependency_alias: String,
    pub dependency_version_spec: String,
    pub platform_manifest_identity_digest: String,
    pub platform_version: String,
    pub payload_manifest_identity_digest: String,
    pub payload_layout_version: u32,
    pub payload_target: String,
    pub payload_entrypoint: String,
    pub payload_file_identity_digest: String,
    pub payload_macho_architecture: CodexMachOArchitecture,
    pub payload_macho_file_type: CodexMachOFileType,
    pub payload_macho_load_commands_identity_digest: String,
    pub payload_code_signature_blob_identity_digest: Option<String>,
    pub derivation_identity_digest: String,
    pub collection_identity_digest: String,
    pub state: String,
}

pub(super) fn bind_codex_npm_launcher_chain(
    architecture: CodexMachOArchitecture,
    evidence: CodexNpmCollectedEvidence,
) -> Result<CodexNpmLauncherChainObservation, String> {
    for (digest, label) in [
        (&evidence.launcher_identity_digest, "Codex npm launcher"),
        (
            &evidence.launcher_symlink_identity_digest,
            "Codex npm launcher symlink",
        ),
        (
            &evidence.root_manifest_identity_digest,
            "Codex npm root manifest",
        ),
        (
            &evidence.platform_manifest_identity_digest,
            "Codex npm platform manifest",
        ),
        (
            &evidence.payload_manifest_identity_digest,
            "Codex npm payload manifest",
        ),
        (
            &evidence.payload_file_identity_digest,
            "Codex npm payload file",
        ),
        (
            &evidence.payload_macho_load_commands_identity_digest,
            "Codex npm payload Mach-O load commands",
        ),
        (
            &evidence.derivation_identity_digest,
            "Codex npm descriptor derivation",
        ),
    ] {
        validate_digest(digest, label).map_err(|error| error.to_string())?;
    }
    if let Some(digest) = evidence
        .payload_code_signature_blob_identity_digest
        .as_deref()
    {
        validate_digest(digest, "Codex npm payload code-signature blob")
            .map_err(|error| error.to_string())?;
    }
    let policy = codex_npm_host_policy(architecture);
    let platform_version = format!("{}-{}", evidence.root_version, policy.platform_suffix);
    let dependency_spec = format!("npm:{PACKAGE_NAME}@{platform_version}");
    if !is_strict_semver(&evidence.root_version)
        || evidence.root_version.contains('+')
        || evidence.candidate_id != CANDIDATE_ID
        || evidence.root_package_name != PACKAGE_NAME
        || evidence.root_bin_name != ROOT_BIN_NAME
        || evidence.root_bin_entrypoint != ROOT_BIN_ENTRYPOINT
        || evidence.dependency_alias != policy.dependency_alias
        || evidence.dependency_version_spec != dependency_spec
        || evidence.platform_package_name != PACKAGE_NAME
        || evidence.platform_version != platform_version
        || evidence.platform_os != "darwin"
        || evidence.platform_cpu != policy.platform_cpu
        || evidence.payload_layout_version != PAYLOAD_LAYOUT_VERSION
        || evidence.payload_version != evidence.root_version
        || evidence.payload_target != policy.target_triple
        || evidence.payload_variant != PAYLOAD_VARIANT
        || evidence.payload_entrypoint != PAYLOAD_ENTRYPOINT
        || evidence.payload_resources_directory != PAYLOAD_RESOURCES_DIRECTORY
        || evidence.payload_path_directory != PAYLOAD_PATH_DIRECTORY
        || evidence.payload_macho_architecture != architecture
        || evidence.payload_macho_file_type != CodexMachOFileType::Execute
        || !evidence.payload_file_is_regular
        || !evidence.payload_file_is_executable
    {
        return Err("Codex npm launcher evidence violates the fixed package policy".into());
    }
    let mut observation = CodexNpmLauncherChainObservation {
        schema_version: SCHEMA_VERSION,
        candidate_id: evidence.candidate_id,
        launcher_identity_digest: evidence.launcher_identity_digest,
        launcher_symlink_identity_digest: evidence.launcher_symlink_identity_digest,
        root_manifest_identity_digest: evidence.root_manifest_identity_digest,
        root_version: evidence.root_version,
        dependency_alias: evidence.dependency_alias,
        dependency_version_spec: evidence.dependency_version_spec,
        platform_manifest_identity_digest: evidence.platform_manifest_identity_digest,
        platform_version: evidence.platform_version,
        payload_manifest_identity_digest: evidence.payload_manifest_identity_digest,
        payload_layout_version: evidence.payload_layout_version,
        payload_target: evidence.payload_target,
        payload_entrypoint: evidence.payload_entrypoint,
        payload_file_identity_digest: evidence.payload_file_identity_digest,
        payload_macho_architecture: evidence.payload_macho_architecture,
        payload_macho_file_type: evidence.payload_macho_file_type,
        payload_macho_load_commands_identity_digest: evidence
            .payload_macho_load_commands_identity_digest,
        payload_code_signature_blob_identity_digest: evidence
            .payload_code_signature_blob_identity_digest,
        derivation_identity_digest: evidence.derivation_identity_digest,
        collection_identity_digest: String::new(),
        state: COLLECTION_STATE.into(),
    };
    observation.collection_identity_digest = collection_digest(&observation, &policy);
    Ok(observation)
}

pub(super) fn validate_codex_npm_launcher_chain_observation(
    architecture: CodexMachOArchitecture,
    observation: &CodexNpmLauncherChainObservation,
) -> Result<(), String> {
    for (digest, label) in [
        (&observation.launcher_identity_digest, "Codex npm launcher"),
        (
            &observation.launcher_symlink_identity_digest,
            "Codex npm launcher symlink",
        ),
        (
            &observation.root_manifest_identity_digest,
            "Codex npm root manifest",
        ),
        (
            &observation.platform_manifest_identity_digest,
            "Codex npm platform manifest",
        ),
        (
            &observation.payload_manifest_identity_digest,
            "Codex npm payload manifest",
        ),
        (
            &observation.payload_file_identity_digest,
            "Codex npm payload file",
        ),
        (
            &observation.payload_macho_load_commands_identity_digest,
            "Codex npm payload Mach-O load commands",
        ),
        (
            &observation.derivation_identity_digest,
            "Codex npm descriptor derivation",
        ),
        (
            &observation.collection_identity_digest,
            "Codex npm collection identity",
        ),
    ] {
        validate_digest(digest, label).map_err(|error| error.to_string())?;
    }
    if let Some(digest) = observation
        .payload_code_signature_blob_identity_digest
        .as_deref()
    {
        validate_digest(digest, "Codex npm payload code-signature blob")
            .map_err(|error| error.to_string())?;
    }
    let policy = codex_npm_host_policy(architecture);
    let platform_version = format!("{}-{}", observation.root_version, policy.platform_suffix);
    let dependency_spec = format!("npm:{PACKAGE_NAME}@{platform_version}");
    if observation.schema_version != SCHEMA_VERSION
        || observation.state != COLLECTION_STATE
        || observation.candidate_id != CANDIDATE_ID
        || !is_strict_semver(&observation.root_version)
        || observation.root_version.contains('+')
        || observation.dependency_alias != policy.dependency_alias
        || observation.dependency_version_spec != dependency_spec
        || observation.platform_version != platform_version
        || observation.payload_layout_version != PAYLOAD_LAYOUT_VERSION
        || observation.payload_target != policy.target_triple
        || observation.payload_entrypoint != PAYLOAD_ENTRYPOINT
        || observation.payload_macho_architecture != architecture
        || observation.payload_macho_file_type != CodexMachOFileType::Execute
        || collection_digest(observation, &policy) != observation.collection_identity_digest
    {
        return Err("Codex npm launcher observation failed receipt validation".into());
    }
    Ok(())
}

fn collection_digest(
    observation: &CodexNpmLauncherChainObservation,
    policy: &CodexNpmHostPolicy,
) -> String {
    let payload_layout_version = observation.payload_layout_version.to_string();
    let (signature_state, signature_digest) = match observation
        .payload_code_signature_blob_identity_digest
        .as_deref()
    {
        Some(digest) => ("signature-blob-present", digest),
        None => ("signature-blob-absent", "none"),
    };
    digest_fields(&[
        observation.candidate_id.as_str(),
        observation.launcher_identity_digest.as_str(),
        observation.launcher_symlink_identity_digest.as_str(),
        observation.root_manifest_identity_digest.as_str(),
        PACKAGE_NAME,
        observation.root_version.as_str(),
        ROOT_BIN_NAME,
        ROOT_BIN_ENTRYPOINT,
        observation.dependency_alias.as_str(),
        observation.dependency_version_spec.as_str(),
        observation.platform_manifest_identity_digest.as_str(),
        PACKAGE_NAME,
        observation.platform_version.as_str(),
        "darwin",
        policy.platform_cpu,
        observation.payload_manifest_identity_digest.as_str(),
        payload_layout_version.as_str(),
        observation.root_version.as_str(),
        observation.payload_target.as_str(),
        PAYLOAD_VARIANT,
        observation.payload_entrypoint.as_str(),
        PAYLOAD_RESOURCES_DIRECTORY,
        PAYLOAD_PATH_DIRECTORY,
        observation.payload_file_identity_digest.as_str(),
        macho_architecture_id(observation.payload_macho_architecture),
        macho_file_type_id(observation.payload_macho_file_type),
        observation
            .payload_macho_load_commands_identity_digest
            .as_str(),
        signature_state,
        signature_digest,
        observation.derivation_identity_digest.as_str(),
        "regular-file",
        "executable",
    ])
}

fn macho_architecture_id(value: CodexMachOArchitecture) -> &'static str {
    match value {
        CodexMachOArchitecture::Arm64 => "arm64",
        CodexMachOArchitecture::X86_64 => "x86_64",
    }
}

fn macho_file_type_id(value: CodexMachOFileType) -> &'static str {
    match value {
        CodexMachOFileType::Execute => "execute",
        CodexMachOFileType::DynamicLibrary => "dynamic-library",
        CodexMachOFileType::Other => "other",
    }
}

pub(super) struct CodexNpmHostPolicy {
    pub dependency_alias: &'static str,
    pub platform_directory_name: &'static str,
    pub platform_suffix: &'static str,
    pub target_triple: &'static str,
    pub platform_cpu: &'static str,
}

pub(super) fn codex_npm_host_policy(architecture: CodexMachOArchitecture) -> CodexNpmHostPolicy {
    match architecture {
        CodexMachOArchitecture::Arm64 => CodexNpmHostPolicy {
            dependency_alias: "@openai/codex-darwin-arm64",
            platform_directory_name: "codex-darwin-arm64",
            platform_suffix: "darwin-arm64",
            target_triple: "aarch64-apple-darwin",
            platform_cpu: "arm64",
        },
        CodexMachOArchitecture::X86_64 => CodexNpmHostPolicy {
            dependency_alias: "@openai/codex-darwin-x64",
            platform_directory_name: "codex-darwin-x64",
            platform_suffix: "darwin-x64",
            target_triple: "x86_64-apple-darwin",
            platform_cpu: "x64",
        },
    }
}

fn digest_fields(fields: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ai-switchboard-codex-npm-launcher-chain-v2\0");
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}
