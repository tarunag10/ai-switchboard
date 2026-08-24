//! Pure, bounded parsing for a native Codex Mach-O target.
//!
//! The parser never opens a path, invokes external tools, starts a process, or
//! treats an embedded signature blob as proof of signer identity.

use sha2::{Digest, Sha256};

const MACH_HEADER_64_BYTES: usize = 32;
const MH_MAGIC_64_LE: [u8; 4] = [0xcf, 0xfa, 0xed, 0xfe];
const CPU_TYPE_X86_64: u32 = 0x0100_0007;
const CPU_TYPE_ARM64: u32 = 0x0100_000c;
const MH_EXECUTE: u32 = 0x2;
const MH_DYLIB: u32 = 0x6;
const LC_CODE_SIGNATURE: u32 = 0x1d;
const CSMAGIC_EMBEDDED_SIGNATURE: u32 = 0xfade_0cc0;
const MAX_LOAD_COMMANDS: u32 = 4_096;
const MAX_LOAD_COMMAND_BYTES: usize = 16 * 1024 * 1024;
const MAX_CODE_SIGNATURE_BLOB_BYTES: usize = 16 * 1024 * 1024;
const MAX_CODE_SIGNATURE_INDEX_ENTRIES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexMachOArchitecture {
    Arm64,
    X86_64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexMachOFileType {
    Execute,
    DynamicLibrary,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexMachOInspection {
    pub architecture: CodexMachOArchitecture,
    pub file_type: CodexMachOFileType,
    pub load_commands_identity_digest: String,
    pub code_signature_blob_identity_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CodexMachOReadRequirements {
    pub header_and_load_commands_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexMachOReadPlan {
    architecture: CodexMachOArchitecture,
    file_type: CodexMachOFileType,
    load_commands_identity_digest: String,
    code_signature_range: Option<(usize, usize)>,
}

impl CodexMachOReadPlan {
    pub(super) fn code_signature_range(&self) -> Option<(u64, usize)> {
        self.code_signature_range
            .map(|(offset, size)| (offset as u64, size))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexMachOInspectionError {
    Truncated,
    UnsupportedContainer,
    UnsupportedArchitecture,
    InvalidLoadCommandEnvelope,
    InvalidLoadCommand,
    InvalidCodeSignatureCommand,
    InvalidCodeSignatureBlob,
}

pub(super) fn inspect_codex_macho(
    bytes: &[u8],
) -> Result<CodexMachOInspection, CodexMachOInspectionError> {
    let requirements = codex_macho_read_requirements(bytes, bytes.len() as u64)?;
    let plan = plan_codex_macho_read(
        &bytes[..requirements.header_and_load_commands_bytes],
        bytes.len() as u64,
    )?;
    let signature_blob = plan
        .code_signature_range
        .map(|(offset, size)| {
            bytes
                .get(offset..offset + size)
                .ok_or(CodexMachOInspectionError::InvalidCodeSignatureCommand)
        })
        .transpose()?;
    complete_codex_macho_read(plan, signature_blob)
}

pub(super) fn codex_macho_read_requirements(
    header: &[u8],
    file_size: u64,
) -> Result<CodexMachOReadRequirements, CodexMachOInspectionError> {
    if header.len() < MACH_HEADER_64_BYTES {
        return Err(CodexMachOInspectionError::Truncated);
    }
    if header[..4] != MH_MAGIC_64_LE {
        return Err(CodexMachOInspectionError::UnsupportedContainer);
    }
    match read_u32_le(header, 4)? {
        CPU_TYPE_ARM64 => CodexMachOArchitecture::Arm64,
        CPU_TYPE_X86_64 => CodexMachOArchitecture::X86_64,
        _ => return Err(CodexMachOInspectionError::UnsupportedArchitecture),
    };
    let command_count = read_u32_le(header, 16)?;
    let command_bytes = read_u32_le(header, 20)? as usize;
    if command_count == 0
        || command_count > MAX_LOAD_COMMANDS
        || command_bytes == 0
        || command_bytes > MAX_LOAD_COMMAND_BYTES
    {
        return Err(CodexMachOInspectionError::InvalidLoadCommandEnvelope);
    }
    let command_end = MACH_HEADER_64_BYTES
        .checked_add(command_bytes)
        .filter(|end| (*end as u64) <= file_size)
        .ok_or(CodexMachOInspectionError::InvalidLoadCommandEnvelope)?;
    Ok(CodexMachOReadRequirements {
        header_and_load_commands_bytes: command_end,
    })
}

pub(super) fn plan_codex_macho_read(
    header_and_load_commands: &[u8],
    file_size: u64,
) -> Result<CodexMachOReadPlan, CodexMachOInspectionError> {
    let requirements = codex_macho_read_requirements(header_and_load_commands, file_size)?;
    let command_end = requirements.header_and_load_commands_bytes;
    if header_and_load_commands.len() != command_end {
        return Err(CodexMachOInspectionError::InvalidLoadCommandEnvelope);
    }
    let architecture = match read_u32_le(header_and_load_commands, 4)? {
        CPU_TYPE_ARM64 => CodexMachOArchitecture::Arm64,
        CPU_TYPE_X86_64 => CodexMachOArchitecture::X86_64,
        _ => return Err(CodexMachOInspectionError::UnsupportedArchitecture),
    };
    let file_type = match read_u32_le(header_and_load_commands, 12)? {
        MH_EXECUTE => CodexMachOFileType::Execute,
        MH_DYLIB => CodexMachOFileType::DynamicLibrary,
        _ => CodexMachOFileType::Other,
    };
    let command_count = read_u32_le(header_and_load_commands, 16)?;

    let mut offset = MACH_HEADER_64_BYTES;
    let mut signature_range = None;
    for _ in 0..command_count {
        let header_end = offset
            .checked_add(8)
            .filter(|end| *end <= command_end)
            .ok_or(CodexMachOInspectionError::InvalidLoadCommand)?;
        let command = read_u32_le(header_and_load_commands, offset)?;
        let size = read_u32_le(header_and_load_commands, offset + 4)? as usize;
        if size < 8 || size % 8 != 0 {
            return Err(CodexMachOInspectionError::InvalidLoadCommand);
        }
        let next = offset
            .checked_add(size)
            .filter(|end| *end <= command_end && *end >= header_end)
            .ok_or(CodexMachOInspectionError::InvalidLoadCommand)?;
        if command == LC_CODE_SIGNATURE {
            if size != 16 || signature_range.is_some() {
                return Err(CodexMachOInspectionError::InvalidCodeSignatureCommand);
            }
            let data_offset = read_u32_le(header_and_load_commands, offset + 8)? as usize;
            let data_size = read_u32_le(header_and_load_commands, offset + 12)? as usize;
            if data_size > MAX_CODE_SIGNATURE_BLOB_BYTES {
                return Err(CodexMachOInspectionError::InvalidCodeSignatureCommand);
            }
            let data_end = data_offset
                .checked_add(data_size)
                .filter(|end| data_offset >= command_end && (*end as u64) <= file_size)
                .ok_or(CodexMachOInspectionError::InvalidCodeSignatureCommand)?;
            signature_range = Some((data_offset, data_end));
        }
        offset = next;
    }
    if offset != command_end {
        return Err(CodexMachOInspectionError::InvalidLoadCommandEnvelope);
    }

    Ok(CodexMachOReadPlan {
        architecture,
        file_type,
        load_commands_identity_digest: digest_identity(
            b"ai-switchboard-codex-macho-load-commands-v1\0",
            &[
                &command_count.to_be_bytes(),
                &header_and_load_commands[MACH_HEADER_64_BYTES..command_end],
            ],
        ),
        code_signature_range: signature_range.map(|(start, end)| (start, end - start)),
    })
}

pub(super) fn complete_codex_macho_read(
    plan: CodexMachOReadPlan,
    code_signature_blob: Option<&[u8]>,
) -> Result<CodexMachOInspection, CodexMachOInspectionError> {
    let code_signature_blob_identity_digest = match (plan.code_signature_range, code_signature_blob)
    {
        (None, None) => None,
        (Some((_, expected_size)), Some(blob)) if blob.len() == expected_size => {
            validate_code_signature_blob(blob, expected_size)?;
            Some(digest_identity(
                b"ai-switchboard-codex-code-signature-blob-v1\0",
                &[blob],
            ))
        }
        _ => return Err(CodexMachOInspectionError::InvalidCodeSignatureBlob),
    };
    Ok(CodexMachOInspection {
        architecture: plan.architecture,
        file_type: plan.file_type,
        load_commands_identity_digest: plan.load_commands_identity_digest,
        code_signature_blob_identity_digest,
    })
}

fn validate_code_signature_blob(
    blob: &[u8],
    command_size: usize,
) -> Result<(), CodexMachOInspectionError> {
    if blob.len() < 12
        || read_u32_be(blob, 0)? != CSMAGIC_EMBEDDED_SIGNATURE
        || read_u32_be(blob, 4)? as usize != command_size
    {
        return Err(CodexMachOInspectionError::InvalidCodeSignatureBlob);
    }
    let index_count = read_u32_be(blob, 8)? as usize;
    if index_count == 0 || index_count > MAX_CODE_SIGNATURE_INDEX_ENTRIES {
        return Err(CodexMachOInspectionError::InvalidCodeSignatureBlob);
    }
    let index_bytes = index_count
        .checked_mul(8)
        .and_then(|size| 12usize.checked_add(size))
        .ok_or(CodexMachOInspectionError::InvalidCodeSignatureBlob)?;
    if index_bytes > blob.len() {
        return Err(CodexMachOInspectionError::InvalidCodeSignatureBlob);
    }
    let mut child_ranges = Vec::with_capacity(index_count);
    for index in 0..index_count {
        let entry_offset = 12 + index * 8;
        let child_offset = read_u32_be(blob, entry_offset + 4)? as usize;
        let child_length_offset = child_offset
            .checked_add(4)
            .filter(|_| child_offset >= index_bytes)
            .ok_or(CodexMachOInspectionError::InvalidCodeSignatureBlob)?;
        let child_length = read_u32_be(blob, child_length_offset)? as usize;
        let child_end = child_offset
            .checked_add(child_length)
            .filter(|end| child_length >= 8 && *end <= blob.len())
            .ok_or(CodexMachOInspectionError::InvalidCodeSignatureBlob)?;
        child_ranges.push((child_offset, child_end));
    }
    child_ranges.sort_unstable();
    if child_ranges
        .windows(2)
        .any(|ranges| ranges[0].1 > ranges[1].0)
    {
        return Err(CodexMachOInspectionError::InvalidCodeSignatureBlob);
    }
    Ok(())
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, CodexMachOInspectionError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(CodexMachOInspectionError::Truncated)?;
    Ok(u32::from_le_bytes(value.try_into().expect("four bytes")))
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Result<u32, CodexMachOInspectionError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(CodexMachOInspectionError::InvalidCodeSignatureBlob)?;
    Ok(u32::from_be_bytes(value.try_into().expect("four bytes")))
}

fn digest_identity(domain: &[u8], values: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for value in values {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value);
    }
    format!("sha256:{:x}", hasher.finalize())
}
