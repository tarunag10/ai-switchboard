//! Content-free evidence model for a collected Codex npm launcher chain.
//!
//! This module grants no path, collection, support, or execution authority.

use sha2::{Digest, Sha256};

use super::codex_macho::CodexMachOArchitecture;
use super::codex_probe_semver::is_strict_semver;
use super::session::validate_digest;

const SCHEMA_VERSION: u32 = 1;
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
            &evidence.derivation_identity_digest,
            "Codex npm descriptor derivation",
        ),
    ] {
        validate_digest(digest, label).map_err(|error| error.to_string())?;
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
        || !evidence.payload_file_is_regular
        || !evidence.payload_file_is_executable
    {
        return Err("Codex npm launcher evidence violates the fixed package policy".into());
    }
    let collection_identity_digest = digest_fields(&[
        evidence.candidate_id.as_str(),
        evidence.launcher_identity_digest.as_str(),
        evidence.launcher_symlink_identity_digest.as_str(),
        evidence.root_manifest_identity_digest.as_str(),
        evidence.root_package_name.as_str(),
        evidence.root_version.as_str(),
        evidence.root_bin_name.as_str(),
        evidence.root_bin_entrypoint.as_str(),
        evidence.dependency_alias.as_str(),
        evidence.dependency_version_spec.as_str(),
        evidence.platform_manifest_identity_digest.as_str(),
        evidence.platform_package_name.as_str(),
        evidence.platform_version.as_str(),
        evidence.platform_os.as_str(),
        evidence.platform_cpu.as_str(),
        evidence.payload_manifest_identity_digest.as_str(),
        &evidence.payload_layout_version.to_string(),
        evidence.payload_version.as_str(),
        evidence.payload_target.as_str(),
        evidence.payload_variant.as_str(),
        evidence.payload_entrypoint.as_str(),
        evidence.payload_resources_directory.as_str(),
        evidence.payload_path_directory.as_str(),
        evidence.payload_file_identity_digest.as_str(),
        evidence.derivation_identity_digest.as_str(),
        "regular-file",
        "executable",
    ]);
    Ok(CodexNpmLauncherChainObservation {
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
        derivation_identity_digest: evidence.derivation_identity_digest,
        collection_identity_digest,
        state: "collected_unbound_non_executing".into(),
    })
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
    hasher.update(b"ai-switchboard-codex-npm-launcher-chain-v1\0");
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}
