use super::codex_macho::{
    codex_macho_read_requirements, complete_codex_macho_read, inspect_codex_macho,
    plan_codex_macho_read, CodexMachOArchitecture, CodexMachOFileType, CodexMachOInspectionError,
};

const CPU_X86_64: u32 = 0x0100_0007;
const CPU_ARM64: u32 = 0x0100_000c;
const LC_UUID: u32 = 0x1b;
const LC_CODE_SIGNATURE: u32 = 0x1d;

#[test]
fn parses_an_arm64_executable_with_a_signature_blob_shape() {
    let bytes = macho_with_signature_shape(CPU_ARM64, 2, 7);
    let inspection = inspect_codex_macho(&bytes).expect("valid Mach-O signature-command shape");

    assert_eq!(inspection.architecture, CodexMachOArchitecture::Arm64);
    assert_eq!(inspection.file_type, CodexMachOFileType::Execute);
    assert!(inspection
        .load_commands_identity_digest
        .starts_with("sha256:"));
    assert!(inspection
        .code_signature_blob_identity_digest
        .expect("signature blob digest")
        .starts_with("sha256:"));
}

#[test]
fn parses_an_unsigned_x86_64_dynamic_library_as_non_executable_shape() {
    let bytes = unsigned_macho(CPU_X86_64, 6, 3);
    let inspection = inspect_codex_macho(&bytes).expect("valid unsigned Mach-O");

    assert_eq!(inspection.architecture, CodexMachOArchitecture::X86_64);
    assert_eq!(inspection.file_type, CodexMachOFileType::DynamicLibrary);
    assert_eq!(inspection.code_signature_blob_identity_digest, None);
}

#[test]
fn staged_segments_match_the_whole_buffer_inspection() {
    let bytes = macho_with_signature_shape(CPU_ARM64, 2, 7);
    let expected = inspect_codex_macho(&bytes).expect("whole-buffer inspection");
    let requirements =
        codex_macho_read_requirements(&bytes[..32], bytes.len() as u64).expect("read requirements");
    assert_eq!(requirements.header_and_load_commands_bytes, 72);
    let plan = plan_codex_macho_read(
        &bytes[..requirements.header_and_load_commands_bytes],
        bytes.len() as u64,
    )
    .expect("staged plan");
    let (offset, size) = plan.code_signature_range().expect("signature range");
    let actual =
        complete_codex_macho_read(plan, Some(&bytes[offset as usize..offset as usize + size]))
            .expect("staged completion");
    assert_eq!(actual, expected);
}

#[test]
fn staged_completion_requires_the_exact_planned_signature_segment() {
    let bytes = macho_with_signature_shape(CPU_ARM64, 2, 7);
    let plan = plan_codex_macho_read(&bytes[..72], bytes.len() as u64).expect("staged plan");
    assert_eq!(
        complete_codex_macho_read(plan, None),
        Err(CodexMachOInspectionError::InvalidCodeSignatureBlob)
    );
}

#[test]
fn rejects_truncated_and_non_64_bit_little_endian_containers() {
    assert_eq!(
        inspect_codex_macho(&[0; 12]),
        Err(CodexMachOInspectionError::Truncated)
    );
    for magic in [
        [0xfe, 0xed, 0xfa, 0xce],
        [0xca, 0xfe, 0xba, 0xbe],
        [0xfe, 0xed, 0xfa, 0xcf],
    ] {
        let mut bytes = vec![0; 32];
        bytes[..4].copy_from_slice(&magic);
        assert_eq!(
            inspect_codex_macho(&bytes),
            Err(CodexMachOInspectionError::UnsupportedContainer)
        );
    }
}

#[test]
fn rejects_an_unapproved_cpu_type() {
    assert_eq!(
        inspect_codex_macho(&unsigned_macho(12, 2, 1)),
        Err(CodexMachOInspectionError::UnsupportedArchitecture)
    );
}

#[test]
fn rejects_unbounded_or_incomplete_load_command_envelopes() {
    let mut too_many = unsigned_macho(CPU_ARM64, 2, 1);
    write_u32_le(&mut too_many, 16, 4_097);
    assert_eq!(
        inspect_codex_macho(&too_many),
        Err(CodexMachOInspectionError::InvalidLoadCommandEnvelope)
    );

    let mut truncated = unsigned_macho(CPU_ARM64, 2, 1);
    write_u32_le(&mut truncated, 20, 10_000);
    assert_eq!(
        inspect_codex_macho(&truncated),
        Err(CodexMachOInspectionError::InvalidLoadCommandEnvelope)
    );

    let mut count_mismatch = unsigned_macho(CPU_ARM64, 2, 1);
    count_mismatch.extend([0; 8]);
    write_u32_le(&mut count_mismatch, 20, 32);
    assert_eq!(
        inspect_codex_macho(&count_mismatch),
        Err(CodexMachOInspectionError::InvalidLoadCommandEnvelope)
    );
}

#[test]
fn rejects_misaligned_or_overrunning_load_commands() {
    let mut misaligned = unsigned_macho(CPU_ARM64, 2, 1);
    write_u32_le(&mut misaligned, 36, 9);
    assert_eq!(
        inspect_codex_macho(&misaligned),
        Err(CodexMachOInspectionError::InvalidLoadCommand)
    );

    let mut overrunning = unsigned_macho(CPU_ARM64, 2, 1);
    write_u32_le(&mut overrunning, 36, 32);
    assert_eq!(
        inspect_codex_macho(&overrunning),
        Err(CodexMachOInspectionError::InvalidLoadCommand)
    );
}

#[test]
fn rejects_duplicate_code_signature_commands() {
    let mut bytes = macho_with_signature_shape(CPU_ARM64, 2, 1);
    let original_signature = bytes[56..72].to_vec();
    bytes.splice(56..56, original_signature);
    write_u32_le(&mut bytes, 16, 3);
    write_u32_le(&mut bytes, 20, 56);
    write_u32_le(&mut bytes, 64, 88);
    write_u32_le(&mut bytes, 80, 88);
    assert_eq!(
        inspect_codex_macho(&bytes),
        Err(CodexMachOInspectionError::InvalidCodeSignatureCommand)
    );
}

#[test]
fn rejects_signature_ranges_inside_commands_or_beyond_eof() {
    let mut overlapping = macho_with_signature_shape(CPU_ARM64, 2, 1);
    write_u32_le(&mut overlapping, 64, 32);
    assert_eq!(
        inspect_codex_macho(&overlapping),
        Err(CodexMachOInspectionError::InvalidCodeSignatureCommand)
    );

    let mut beyond = macho_with_signature_shape(CPU_ARM64, 2, 1);
    write_u32_le(&mut beyond, 64, 1_000);
    assert_eq!(
        inspect_codex_macho(&beyond),
        Err(CodexMachOInspectionError::InvalidCodeSignatureCommand)
    );

    let mut oversized = macho_with_signature_shape(CPU_ARM64, 2, 1);
    write_u32_le(&mut oversized, 68, 16 * 1024 * 1024 + 1);
    oversized.resize(72 + 16 * 1024 * 1024 + 1, 0);
    assert_eq!(
        inspect_codex_macho(&oversized),
        Err(CodexMachOInspectionError::InvalidCodeSignatureCommand)
    );
}

#[test]
fn rejects_non_superblob_or_internally_truncated_signature_data() {
    let mut bad_magic = macho_with_signature_shape(CPU_ARM64, 2, 1);
    bad_magic[72] = 0;
    assert_eq!(
        inspect_codex_macho(&bad_magic),
        Err(CodexMachOInspectionError::InvalidCodeSignatureBlob)
    );

    let mut bad_count = macho_with_signature_shape(CPU_ARM64, 2, 1);
    write_u32_be(&mut bad_count, 80, 2);
    assert_eq!(
        inspect_codex_macho(&bad_count),
        Err(CodexMachOInspectionError::InvalidCodeSignatureBlob)
    );

    let mut overlapping_children = Vec::new();
    for value in [0xfade_0cc0, 36, 2, 0, 28, 1, 28, 0xfade_0c02, 8] {
        append_u32_be(&mut overlapping_children, value);
    }
    assert_eq!(
        inspect_codex_macho(&macho_with_signature_blob(
            CPU_ARM64,
            2,
            1,
            &overlapping_children,
        )),
        Err(CodexMachOInspectionError::InvalidCodeSignatureBlob)
    );
}

#[test]
fn current_parser_source_uses_no_execution_or_platform_trust_authority() {
    let source = include_str!("codex_macho.rs");
    for forbidden in [
        "std::process",
        "tokio::process",
        "Command::new",
        "posix_spawn",
        "std::fs",
        "File::",
        "extern \"C\"",
        "libc::",
        "unsafe",
        "mmap",
        "PROT_EXEC",
        "dlopen",
        "codesign",
        "SecStaticCode",
        "SecCodeCopySigningInformation",
        "spctl",
        "reqwest",
        "std::net",
        "TcpStream",
        "UdpSocket",
        "std::env",
        "serde",
        "#[tauri::command]",
        "tauri::",
        "OpenOptions",
    ] {
        assert!(
            !source.contains(forbidden),
            "parser acquired forbidden authority: {forbidden}"
        );
    }
}

#[test]
fn load_command_identity_changes_with_command_content() {
    let first = inspect_codex_macho(&unsigned_macho(CPU_ARM64, 2, 1)).expect("first");
    let second = inspect_codex_macho(&unsigned_macho(CPU_ARM64, 2, 2)).expect("second");
    assert_ne!(
        first.load_commands_identity_digest,
        second.load_commands_identity_digest
    );
}

#[test]
fn signature_blob_identity_changes_with_blob_content() {
    let mut first_bytes = macho_with_signature_shape(CPU_ARM64, 2, 1);
    let mut second_bytes = first_bytes.clone();
    first_bytes.push(0);
    second_bytes.push(1);
    write_u32_le(&mut first_bytes, 68, 29);
    write_u32_le(&mut second_bytes, 68, 29);
    write_u32_be(&mut first_bytes, 76, 29);
    write_u32_be(&mut second_bytes, 76, 29);
    let first = inspect_codex_macho(&first_bytes).expect("first");
    let second = inspect_codex_macho(&second_bytes).expect("second");
    assert_ne!(
        first.code_signature_blob_identity_digest,
        second.code_signature_blob_identity_digest
    );
}

fn unsigned_macho(cpu: u32, file_type: u32, marker: u8) -> Vec<u8> {
    let mut bytes = header(cpu, file_type, 1, 24);
    append_u32_le(&mut bytes, LC_UUID);
    append_u32_le(&mut bytes, 24);
    bytes.extend([marker; 16]);
    bytes
}

fn macho_with_signature_shape(cpu: u32, file_type: u32, marker: u8) -> Vec<u8> {
    let mut blob = Vec::new();
    for value in [0xfade_0cc0, 28, 1, 0, 20, 0xfade_0c02, 8] {
        append_u32_be(&mut blob, value);
    }
    macho_with_signature_blob(cpu, file_type, marker, &blob)
}

fn macho_with_signature_blob(cpu: u32, file_type: u32, marker: u8, blob: &[u8]) -> Vec<u8> {
    let mut bytes = header(cpu, file_type, 2, 40);
    append_u32_le(&mut bytes, LC_UUID);
    append_u32_le(&mut bytes, 24);
    bytes.extend([marker; 16]);
    append_u32_le(&mut bytes, LC_CODE_SIGNATURE);
    append_u32_le(&mut bytes, 16);
    append_u32_le(&mut bytes, 72);
    append_u32_le(&mut bytes, blob.len() as u32);
    bytes.extend(blob);
    bytes
}

fn header(cpu: u32, file_type: u32, command_count: u32, command_bytes: u32) -> Vec<u8> {
    let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
    for value in [cpu, 0, file_type, command_count, command_bytes, 0, 0] {
        append_u32_le(&mut bytes, value);
    }
    bytes
}

fn append_u32_le(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_le_bytes());
}

fn append_u32_be(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_be_bytes());
}

fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u32_be(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}
