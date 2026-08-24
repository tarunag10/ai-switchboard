//! Bounded Mach-O inspection and hashing through one npm payload descriptor.

use std::ffi::OsStr;

use super::codex_macho::{
    codex_macho_read_requirements, complete_codex_macho_read, plan_codex_macho_read,
    CodexMachOInspection, CodexMachOInspectionError,
};
use super::codex_npm_fs::{CodexNpmDirectory, CodexNpmFsError, CodexNpmRegularFileHash};

const MACH_HEADER_64_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexNpmMachOCollectionError {
    Filesystem(CodexNpmFsError),
    Inspection(CodexMachOInspectionError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexNpmMachOFile {
    pub file: CodexNpmRegularFileHash,
    pub inspection: CodexMachOInspection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexNpmMachOHookPoint {
    AfterHeaderRead,
    AfterLoadCommandsRead,
    AfterSignatureRead,
}

pub(super) fn inspect_and_hash_codex_npm_macho(
    directory: &CodexNpmDirectory,
    leaf: &OsStr,
    max_bytes: u64,
) -> Result<CodexNpmMachOFile, CodexNpmMachOCollectionError> {
    inspect_and_hash_codex_npm_macho_with_hook(directory, leaf, max_bytes, &mut |_| {})
}

pub(super) fn inspect_and_hash_codex_npm_macho_with_hook(
    directory: &CodexNpmDirectory,
    leaf: &OsStr,
    max_bytes: u64,
    hook: &mut impl FnMut(CodexNpmMachOHookPoint),
) -> Result<CodexNpmMachOFile, CodexNpmMachOCollectionError> {
    let stable = directory
        .open_stable_regular_file(leaf, max_bytes)
        .map_err(CodexNpmMachOCollectionError::Filesystem)?;
    let mut reader = stable.into_hash_reader();
    let file_size = reader.byte_count();
    if file_size < MACH_HEADER_64_BYTES as u64 {
        return Err(CodexNpmMachOCollectionError::Inspection(
            CodexMachOInspectionError::Truncated,
        ));
    }

    let mut header = [0u8; MACH_HEADER_64_BYTES];
    reader
        .read_exact(&mut header)
        .map_err(CodexNpmMachOCollectionError::Filesystem)?;
    hook(CodexNpmMachOHookPoint::AfterHeaderRead);
    let requirements = codex_macho_read_requirements(&header, file_size)
        .map_err(CodexNpmMachOCollectionError::Inspection)?;
    let mut header_and_load_commands = bounded_buffer(
        requirements.header_and_load_commands_bytes,
        CodexNpmFsError::CapacityUnavailable,
    )?;
    header_and_load_commands[..MACH_HEADER_64_BYTES].copy_from_slice(&header);
    reader
        .read_exact(&mut header_and_load_commands[MACH_HEADER_64_BYTES..])
        .map_err(CodexNpmMachOCollectionError::Filesystem)?;
    hook(CodexNpmMachOHookPoint::AfterLoadCommandsRead);
    let plan = plan_codex_macho_read(&header_and_load_commands, file_size)
        .map_err(CodexNpmMachOCollectionError::Inspection)?;
    drop(header_and_load_commands);
    let signature_blob = plan
        .code_signature_range()
        .map(|(offset, size)| {
            reader
                .hash_until(offset)
                .map_err(CodexNpmMachOCollectionError::Filesystem)?;
            let mut bytes = bounded_buffer(size, CodexNpmFsError::CapacityUnavailable)?;
            reader
                .read_exact(&mut bytes)
                .map_err(CodexNpmMachOCollectionError::Filesystem)?;
            Ok(bytes)
        })
        .transpose()?;
    hook(CodexNpmMachOHookPoint::AfterSignatureRead);
    let inspection = complete_codex_macho_read(plan, signature_blob.as_deref())
        .map_err(CodexNpmMachOCollectionError::Inspection)?;
    drop(signature_blob);
    let file = reader
        .finish()
        .map_err(CodexNpmMachOCollectionError::Filesystem)?;
    Ok(CodexNpmMachOFile { file, inspection })
}

fn bounded_buffer(
    size: usize,
    capacity_error: CodexNpmFsError,
) -> Result<Vec<u8>, CodexNpmMachOCollectionError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| CodexNpmMachOCollectionError::Filesystem(capacity_error))?;
    bytes.resize(size, 0);
    Ok(bytes)
}
