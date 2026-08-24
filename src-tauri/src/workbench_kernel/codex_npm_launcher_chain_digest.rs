//! Domain-separated identities for the fixed Codex npm collector.

use super::codex_command_catalog::CodexProbePlan;
use super::codex_command_identity::evidence_identity_digest;
use super::codex_npm_fs::{CodexNpmDirectory, CodexNpmRegularFile, CodexNpmRegularFileHash};

pub(super) fn file_identity(domain: &[u8], file: &CodexNpmRegularFile) -> String {
    evidence_identity_digest(domain, &[&file.identity], &[&file.content_digest])
}

pub(super) fn hashed_file_identity(domain: &[u8], file: &CodexNpmRegularFileHash) -> String {
    evidence_identity_digest(domain, &[&file.identity], &[&file.digest])
}

pub(super) fn derivation_digest(
    plan: &CodexProbePlan,
    symlink_identity: &str,
    launcher_link_target: &[u8],
    directories: &[&CodexNpmDirectory],
    files: &[&str],
) -> String {
    let directory_digests = directories
        .iter()
        .enumerate()
        .map(|(index, directory)| directory.identity_digest(&format!("chain-{index}")))
        .collect::<Vec<_>>();
    let mut values = vec![
        plan.binary_identity_digest.as_bytes(),
        launcher_link_target,
        symlink_identity.as_bytes(),
    ];
    values.extend(directory_digests.iter().map(String::as_bytes));
    values.extend(files.iter().map(|value| value.as_bytes()));
    evidence_identity_digest(
        b"ai-switchboard-codex-npm-descriptor-derivation-v1\0",
        &[],
        &values,
    )
}
