//! Race-safe, non-executing collection for the fixed home npm Codex chain.
//!
//! The collector never interprets the JavaScript launcher, searches PATH,
//! starts a process, reads credentials, accesses the network, or exposes paths.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path};

use super::codex_command_catalog::{validate_probe_plan, CodexProbePlan};
use super::codex_command_identity::{
    account_home_directory, evidence_identity_digest, identity_digest, MetadataIdentity,
};
use super::codex_macho::CodexMachOArchitecture;
use super::codex_macho::CodexMachOInspectionError;
use super::codex_npm_chain_model::{
    bind_codex_npm_launcher_chain, codex_npm_host_policy, CodexNpmCollectedEvidence,
    CodexNpmLauncherChainObservation,
};
use super::codex_npm_fs::{CodexNpmDirectory, CodexNpmFsError, CodexNpmRegularFile};
use super::codex_npm_launcher_chain_digest::{
    derivation_digest, file_identity, hashed_file_identity,
};
use super::codex_npm_macho::{inspect_and_hash_codex_npm_macho, CodexNpmMachOCollectionError};
use super::codex_npm_manifest::{
    parse_codex_npm_payload_manifest, parse_codex_npm_platform_manifest,
    parse_codex_npm_root_manifest, CodexNpmManifestError, MAX_CODEX_NPM_MANIFEST_BYTES,
};

const CANDIDATE_ID: &str = "home-npm-global-bin";
const LAUNCHER_LINK_TARGET: &[u8] = b"../lib/node_modules/@openai/codex/bin/codex.js";
const MAX_LAUNCHER_LINK_BYTES: usize = 128;
const MAX_LAUNCHER_BYTES: u64 = 1024 * 1024;
const MAX_NATIVE_PAYLOAD_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexNpmObject {
    NpmBin,
    LauncherLink,
    PackageRoot,
    Launcher,
    RootManifest,
    PlatformPackage,
    PlatformManifest,
    PayloadRoot,
    PayloadManifest,
    PayloadResources,
    PayloadPath,
    PayloadFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexNpmChainCollectionError {
    InvalidProbePlan,
    UnsupportedCandidate,
    AccountHomeUnavailable,
    ProbeIdentityMismatch,
    LauncherLinkMismatch,
    LauncherNotExecutable,
    PayloadLayoutRejected,
    Filesystem(CodexNpmObject, CodexNpmFsError),
    Manifest(CodexNpmObject, CodexNpmManifestError),
    MachO(CodexMachOInspectionError),
    PackagePolicyRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexNpmCollectorHookPoint {
    AfterPlanValidation,
    AfterLauncherLink,
    AfterLauncherFileRead,
    AfterRootManifest,
    AfterPlatformManifest,
    AfterPayloadManifest,
    AfterPayloadHash,
    BeforeFinalRevalidation,
}

pub(super) fn collect_codex_npm_launcher_chain(
    plan: &CodexProbePlan,
    architecture: CodexMachOArchitecture,
) -> Result<CodexNpmLauncherChainObservation, CodexNpmChainCollectionError> {
    let home = account_home_directory()
        .map_err(|_| CodexNpmChainCollectionError::AccountHomeUnavailable)?;
    collect_codex_npm_launcher_chain_with_context(
        plan,
        architecture,
        &home,
        Path::new("/"),
        &mut |_| {},
    )
}

pub(super) fn collect_codex_npm_launcher_chain_with_context(
    plan: &CodexProbePlan,
    architecture: CodexMachOArchitecture,
    home: &Path,
    filesystem_root: &Path,
    hook: &mut impl FnMut(CodexNpmCollectorHookPoint),
) -> Result<CodexNpmLauncherChainObservation, CodexNpmChainCollectionError> {
    validate_probe_plan(plan).map_err(|_| CodexNpmChainCollectionError::InvalidProbePlan)?;
    if plan.candidate_id != CANDIDATE_ID {
        return Err(CodexNpmChainCollectionError::UnsupportedCandidate);
    }
    let home_components = relative_home_components(home, filesystem_root)?;
    hook(CodexNpmCollectorHookPoint::AfterPlanValidation);

    let policy = codex_npm_host_policy(architecture);
    let npm_root = extended(&home_components, &[".npm-global"]);
    let npm_bin = open_directory(
        filesystem_root,
        &extended(&npm_root, &["bin"]),
        CodexNpmObject::NpmBin,
    )?;
    let launcher_link = npm_bin
        .read_link(OsStr::new("codex"), MAX_LAUNCHER_LINK_BYTES)
        .map_err(|error| fs_error(CodexNpmObject::LauncherLink, error))?;
    if launcher_link.target.as_encoded_bytes() != LAUNCHER_LINK_TARGET {
        return Err(CodexNpmChainCollectionError::LauncherLinkMismatch);
    }
    let launcher_symlink_identity_digest = evidence_identity_digest(
        b"ai-switchboard-codex-npm-launcher-symlink-v1\0",
        &[&launcher_link.identity],
        &[
            plan.binary_identity_digest.as_bytes(),
            launcher_link.target.as_encoded_bytes(),
            npm_bin.identity_digest("npm-bin").as_bytes(),
        ],
    );
    hook(CodexNpmCollectorHookPoint::AfterLauncherLink);

    let package_components = extended(&npm_root, &["lib", "node_modules", "@openai", "codex"]);
    let package = open_directory(
        filesystem_root,
        &package_components,
        CodexNpmObject::PackageRoot,
    )?;
    let root_file = read_file(
        &package,
        "package.json",
        MAX_CODEX_NPM_MANIFEST_BYTES as u64,
        CodexNpmObject::RootManifest,
    )?;
    let root_manifest = parse_codex_npm_root_manifest(&root_file.bytes, policy.dependency_alias)
        .map_err(|error| manifest_error(CodexNpmObject::RootManifest, error))?;
    let root_identity = file_identity(b"ai-switchboard-codex-npm-root-manifest-v1\0", &root_file);
    hook(CodexNpmCollectorHookPoint::AfterRootManifest);

    let launcher_directory = open_directory(
        filesystem_root,
        &extended(&package_components, &["bin"]),
        CodexNpmObject::Launcher,
    )?;
    let launcher_file = read_file(
        &launcher_directory,
        "codex.js",
        MAX_LAUNCHER_BYTES,
        CodexNpmObject::Launcher,
    )?;
    if !launcher_file.executable {
        return Err(CodexNpmChainCollectionError::LauncherNotExecutable);
    }
    hook(CodexNpmCollectorHookPoint::AfterLauncherFileRead);
    let observed_launcher_identity = identity_digest(
        CANDIDATE_ID,
        &launcher_link.identity,
        &launcher_file.identity,
        launcher_file.content_digest,
    );
    if observed_launcher_identity != plan.binary_identity_digest {
        return Err(CodexNpmChainCollectionError::ProbeIdentityMismatch);
    }
    let launcher_file_identity = file_identity(
        b"ai-switchboard-codex-npm-launcher-file-v1\0",
        &launcher_file,
    );

    let platform_components = extended(
        &package_components,
        &["node_modules", "@openai", policy.platform_directory_name],
    );
    let platform = open_directory(
        filesystem_root,
        &platform_components,
        CodexNpmObject::PlatformPackage,
    )?;
    let platform_file = read_file(
        &platform,
        "package.json",
        MAX_CODEX_NPM_MANIFEST_BYTES as u64,
        CodexNpmObject::PlatformManifest,
    )?;
    let platform_manifest = parse_codex_npm_platform_manifest(&platform_file.bytes)
        .map_err(|error| manifest_error(CodexNpmObject::PlatformManifest, error))?;
    let platform_identity = file_identity(
        b"ai-switchboard-codex-npm-platform-manifest-v1\0",
        &platform_file,
    );
    hook(CodexNpmCollectorHookPoint::AfterPlatformManifest);

    let payload_components = extended(&platform_components, &["vendor", policy.target_triple]);
    let payload = open_directory(
        filesystem_root,
        &payload_components,
        CodexNpmObject::PayloadRoot,
    )?;
    let payload_manifest_file = read_file(
        &payload,
        "codex-package.json",
        MAX_CODEX_NPM_MANIFEST_BYTES as u64,
        CodexNpmObject::PayloadManifest,
    )?;
    let payload_manifest = parse_codex_npm_payload_manifest(&payload_manifest_file.bytes)
        .map_err(|error| manifest_error(CodexNpmObject::PayloadManifest, error))?;
    let payload_layout_version = u32::try_from(payload_manifest.layout_version)
        .map_err(|_| CodexNpmChainCollectionError::PayloadLayoutRejected)?;
    let payload_manifest_identity = file_identity(
        b"ai-switchboard-codex-npm-payload-manifest-v1\0",
        &payload_manifest_file,
    );
    hook(CodexNpmCollectorHookPoint::AfterPayloadManifest);

    let payload_resources = open_directory(
        filesystem_root,
        &extended(&payload_components, &["codex-resources"]),
        CodexNpmObject::PayloadResources,
    )?;
    let payload_path = open_directory(
        filesystem_root,
        &extended(&payload_components, &["codex-path"]),
        CodexNpmObject::PayloadPath,
    )?;

    let payload_bin = open_directory(
        filesystem_root,
        &extended(&payload_components, &["bin"]),
        CodexNpmObject::PayloadFile,
    )?;
    let payload_macho = inspect_and_hash_codex_npm_macho(
        &payload_bin,
        OsStr::new("codex"),
        MAX_NATIVE_PAYLOAD_BYTES,
    )
    .map_err(|error| match error {
        CodexNpmMachOCollectionError::Filesystem(error) => {
            fs_error(CodexNpmObject::PayloadFile, error)
        }
        CodexNpmMachOCollectionError::Inspection(error) => {
            CodexNpmChainCollectionError::MachO(error)
        }
    })?;
    let payload_file = payload_macho.file;
    let payload_inspection = payload_macho.inspection;
    let payload_signature_identity = payload_inspection
        .code_signature_blob_identity_digest
        .clone();
    let payload_signature_state = if payload_signature_identity.is_some() {
        "signature-blob-present"
    } else {
        "signature-blob-absent"
    };
    let payload_signature_value = payload_signature_identity.as_deref().unwrap_or("none");
    let payload_file_identity =
        hashed_file_identity(b"ai-switchboard-codex-npm-payload-file-v1\0", &payload_file);
    hook(CodexNpmCollectorHookPoint::AfterPayloadHash);
    hook(CodexNpmCollectorHookPoint::BeforeFinalRevalidation);

    for (directory, object) in [
        (&npm_bin, CodexNpmObject::NpmBin),
        (&package, CodexNpmObject::PackageRoot),
        (&launcher_directory, CodexNpmObject::Launcher),
        (&platform, CodexNpmObject::PlatformPackage),
        (&payload, CodexNpmObject::PayloadRoot),
        (&payload_resources, CodexNpmObject::PayloadResources),
        (&payload_path, CodexNpmObject::PayloadPath),
        (&payload_bin, CodexNpmObject::PayloadFile),
    ] {
        directory
            .revalidate()
            .map_err(|error| fs_error(object, error))?;
    }
    revalidate_file(
        &package,
        "package.json",
        &root_file.identity,
        CodexNpmObject::RootManifest,
    )?;
    revalidate_file(
        &launcher_directory,
        "codex.js",
        &launcher_file.identity,
        CodexNpmObject::Launcher,
    )?;
    revalidate_file(
        &platform,
        "package.json",
        &platform_file.identity,
        CodexNpmObject::PlatformManifest,
    )?;
    revalidate_file(
        &payload,
        "codex-package.json",
        &payload_manifest_file.identity,
        CodexNpmObject::PayloadManifest,
    )?;
    payload_bin
        .revalidate_regular_file(
            OsStr::new("codex"),
            &payload_file.identity,
            MAX_NATIVE_PAYLOAD_BYTES,
        )
        .map_err(|error| fs_error(CodexNpmObject::PayloadFile, error))?;
    let final_link = npm_bin
        .read_link(OsStr::new("codex"), MAX_LAUNCHER_LINK_BYTES)
        .map_err(|error| fs_error(CodexNpmObject::LauncherLink, error))?;
    if final_link != launcher_link {
        return Err(CodexNpmChainCollectionError::LauncherLinkMismatch);
    }
    let derivation_identity_digest = derivation_digest(
        plan,
        &launcher_symlink_identity_digest,
        LAUNCHER_LINK_TARGET,
        &[
            &npm_bin,
            &package,
            &launcher_directory,
            &platform,
            &payload,
            &payload_resources,
            &payload_path,
            &payload_bin,
        ],
        &[
            launcher_file_identity.as_str(),
            root_identity.as_str(),
            platform_identity.as_str(),
            payload_manifest_identity.as_str(),
            payload_file_identity.as_str(),
            payload_inspection.load_commands_identity_digest.as_str(),
            payload_signature_state,
            payload_signature_value,
        ],
    );
    bind_codex_npm_launcher_chain(
        architecture,
        CodexNpmCollectedEvidence {
            candidate_id: plan.candidate_id.clone(),
            launcher_identity_digest: plan.binary_identity_digest.clone(),
            launcher_symlink_identity_digest,
            root_manifest_identity_digest: root_identity,
            root_package_name: root_manifest.name,
            root_version: root_manifest.version,
            root_bin_name: "codex".into(),
            root_bin_entrypoint: root_manifest.bin_codex,
            dependency_alias: root_manifest.host_dependency_alias,
            dependency_version_spec: root_manifest.host_dependency_spec,
            platform_manifest_identity_digest: platform_identity,
            platform_package_name: platform_manifest.name,
            platform_version: platform_manifest.version,
            platform_os: platform_manifest.os,
            platform_cpu: platform_manifest.cpu,
            payload_manifest_identity_digest: payload_manifest_identity,
            payload_layout_version,
            payload_version: payload_manifest.version,
            payload_target: payload_manifest.target,
            payload_variant: payload_manifest.variant,
            payload_entrypoint: payload_manifest.entrypoint,
            payload_resources_directory: payload_manifest.resources_dir,
            payload_path_directory: payload_manifest.path_dir,
            payload_file_identity_digest: payload_file_identity,
            payload_macho_architecture: payload_inspection.architecture,
            payload_macho_file_type: payload_inspection.file_type,
            payload_macho_load_commands_identity_digest: payload_inspection
                .load_commands_identity_digest,
            payload_code_signature_blob_identity_digest: payload_signature_identity,
            derivation_identity_digest,
            payload_file_is_regular: true,
            payload_file_is_executable: payload_file.executable,
        },
    )
    .map_err(|_| CodexNpmChainCollectionError::PackagePolicyRejected)
}

fn relative_home_components(
    home: &Path,
    root: &Path,
) -> Result<Vec<OsString>, CodexNpmChainCollectionError> {
    let relative = home
        .strip_prefix(root)
        .map_err(|_| CodexNpmChainCollectionError::AccountHomeUnavailable)?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_os_string()),
            _ => Err(CodexNpmChainCollectionError::AccountHomeUnavailable),
        })
        .collect::<Result<Vec<_>, _>>()?;
    (!components.is_empty())
        .then_some(components)
        .ok_or(CodexNpmChainCollectionError::AccountHomeUnavailable)
}

fn extended(base: &[OsString], suffix: &[&str]) -> Vec<OsString> {
    base.iter()
        .cloned()
        .chain(suffix.iter().map(OsString::from))
        .collect()
}

fn open_directory(
    root: &Path,
    components: &[OsString],
    object: CodexNpmObject,
) -> Result<CodexNpmDirectory, CodexNpmChainCollectionError> {
    let references = components
        .iter()
        .map(OsString::as_os_str)
        .collect::<Vec<_>>();
    CodexNpmDirectory::open(root, &references).map_err(|error| fs_error(object, error))
}

fn read_file(
    directory: &CodexNpmDirectory,
    name: &str,
    max_bytes: u64,
    object: CodexNpmObject,
) -> Result<CodexNpmRegularFile, CodexNpmChainCollectionError> {
    directory
        .read_regular_file(OsStr::new(name), max_bytes)
        .map_err(|error| fs_error(object, error))
}

fn revalidate_file(
    directory: &CodexNpmDirectory,
    name: &str,
    identity: &MetadataIdentity,
    object: CodexNpmObject,
) -> Result<(), CodexNpmChainCollectionError> {
    directory
        .revalidate_regular_file(
            OsStr::new(name),
            identity,
            MAX_CODEX_NPM_MANIFEST_BYTES.max(MAX_LAUNCHER_BYTES as usize) as u64,
        )
        .map_err(|error| fs_error(object, error))
}

fn fs_error(object: CodexNpmObject, error: CodexNpmFsError) -> CodexNpmChainCollectionError {
    CodexNpmChainCollectionError::Filesystem(object, error)
}

fn manifest_error(
    object: CodexNpmObject,
    error: CodexNpmManifestError,
) -> CodexNpmChainCollectionError {
    CodexNpmChainCollectionError::Manifest(object, error)
}
