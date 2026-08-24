#![cfg(unix)]

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;

use sha2::{Digest, Sha256};

use super::codex_macho::{CodexMachOArchitecture, CodexMachOFileType};
use super::codex_npm_fs::CodexNpmDirectory;
use super::codex_npm_macho::{
    inspect_and_hash_codex_npm_macho, inspect_and_hash_codex_npm_macho_with_hook,
    CodexNpmMachOCollectionError, CodexNpmMachOHookPoint,
};

#[test]
fn inspects_segments_and_hashes_one_stable_descriptor() {
    let fixture = tempfile::tempdir().expect("temporary root");
    let bin = fixture.path().join("bin");
    fs::create_dir(&bin).expect("bin directory");
    let payload = bin.join("codex");
    let bytes = signed_macho();
    fs::write(&payload, &bytes).expect("payload");
    make_executable(&payload);
    let directory = CodexNpmDirectory::open(fixture.path(), &[OsStr::new("bin")])
        .expect("descriptor directory");

    let result =
        inspect_and_hash_codex_npm_macho(&directory, OsStr::new("codex"), 256 * 1024 * 1024)
            .expect("same-descriptor inspection");
    assert_eq!(
        result.inspection.architecture,
        CodexMachOArchitecture::Arm64
    );
    assert_eq!(result.inspection.file_type, CodexMachOFileType::Execute);
    assert!(result
        .inspection
        .code_signature_blob_identity_digest
        .expect("signature shape")
        .starts_with("sha256:"));
    let expected_digest: [u8; 32] = Sha256::digest(bytes).into();
    assert_eq!(result.file.digest, expected_digest);
}

#[test]
fn unsigned_shape_is_collected_as_explicit_signature_absence() {
    let fixture = tempfile::tempdir().expect("temporary root");
    let bin = fixture.path().join("bin");
    fs::create_dir(&bin).expect("bin directory");
    let payload = bin.join("codex");
    fs::write(&payload, unsigned_macho()).expect("payload");
    make_executable(&payload);
    let directory = CodexNpmDirectory::open(fixture.path(), &[OsStr::new("bin")])
        .expect("descriptor directory");
    let result = inspect_and_hash_codex_npm_macho(&directory, OsStr::new("codex"), 1024)
        .expect("unsigned shape");
    assert_eq!(result.inspection.code_signature_blob_identity_digest, None);
}

#[test]
fn malformed_or_truncated_payloads_fail_without_an_observation() {
    for bytes in [vec![0; 12], {
        let mut value = signed_macho();
        value[72] = 0;
        value
    }] {
        let fixture = tempfile::tempdir().expect("temporary root");
        let bin = fixture.path().join("bin");
        fs::create_dir(&bin).expect("bin directory");
        let payload = bin.join("codex");
        fs::write(&payload, bytes).expect("payload");
        make_executable(&payload);
        let directory = CodexNpmDirectory::open(fixture.path(), &[OsStr::new("bin")])
            .expect("descriptor directory");
        assert!(matches!(
            inspect_and_hash_codex_npm_macho(&directory, OsStr::new("codex"), 1024),
            Err(CodexNpmMachOCollectionError::Inspection(_))
        ));
    }
}

#[test]
fn segment_swap_and_restore_is_rejected_by_same_descriptor_revalidation() {
    let fixture = tempfile::tempdir().expect("temporary root");
    let bin = fixture.path().join("bin");
    fs::create_dir(&bin).expect("bin directory");
    let payload = bin.join("codex");
    let original = signed_macho();
    let mut changed = original.clone();
    changed[40] ^= 1;
    fs::write(&payload, &original).expect("payload");
    make_executable(&payload);
    let directory = CodexNpmDirectory::open(fixture.path(), &[OsStr::new("bin")])
        .expect("descriptor directory");

    let result = inspect_and_hash_codex_npm_macho_with_hook(
        &directory,
        OsStr::new("codex"),
        1024,
        &mut |point| match point {
            CodexNpmMachOHookPoint::AfterHeaderRead => {
                fs::write(&payload, &changed).expect("install changed generation");
                make_executable(&payload);
            }
            CodexNpmMachOHookPoint::AfterLoadCommandsRead => {
                fs::write(&payload, &original).expect("restore original generation");
                make_executable(&payload);
            }
            CodexNpmMachOHookPoint::AfterSignatureRead => {}
        },
    );
    assert!(matches!(
        result,
        Err(CodexNpmMachOCollectionError::Filesystem(
            super::codex_npm_fs::CodexNpmFsError::FileChanged
        ))
    ));
}

#[test]
fn path_replacement_after_open_cannot_rebind_the_original_descriptor() {
    let fixture = tempfile::tempdir().expect("temporary root");
    let bin = fixture.path().join("bin");
    fs::create_dir(&bin).expect("bin directory");
    let payload = bin.join("codex");
    let displaced = bin.join("codex-original");
    fs::write(&payload, signed_macho()).expect("payload");
    make_executable(&payload);
    let directory = CodexNpmDirectory::open(fixture.path(), &[OsStr::new("bin")])
        .expect("descriptor directory");

    let result = inspect_and_hash_codex_npm_macho_with_hook(
        &directory,
        OsStr::new("codex"),
        1024,
        &mut |point| {
            if point == CodexNpmMachOHookPoint::AfterHeaderRead {
                fs::rename(&payload, &displaced).expect("displace original");
                fs::write(&payload, signed_macho()).expect("replacement payload");
                make_executable(&payload);
            }
        },
    );
    assert!(matches!(
        result,
        Err(CodexNpmMachOCollectionError::Filesystem(
            super::codex_npm_fs::CodexNpmFsError::FileChanged
        ))
    ));
}

#[test]
fn truncation_during_segment_collection_is_classified_as_file_change() {
    let fixture = tempfile::tempdir().expect("temporary root");
    let bin = fixture.path().join("bin");
    fs::create_dir(&bin).expect("bin directory");
    let payload = bin.join("codex");
    fs::write(&payload, signed_macho()).expect("payload");
    make_executable(&payload);
    let directory = CodexNpmDirectory::open(fixture.path(), &[OsStr::new("bin")])
        .expect("descriptor directory");

    let result = inspect_and_hash_codex_npm_macho_with_hook(
        &directory,
        OsStr::new("codex"),
        1024,
        &mut |point| {
            if point == CodexNpmMachOHookPoint::AfterHeaderRead {
                fs::OpenOptions::new()
                    .write(true)
                    .open(&payload)
                    .expect("open payload")
                    .set_len(16)
                    .expect("truncate payload");
            }
        },
    );
    assert!(matches!(
        result,
        Err(CodexNpmMachOCollectionError::Filesystem(
            super::codex_npm_fs::CodexNpmFsError::FileChanged
        ))
    ));
}

#[test]
fn collector_source_has_no_execution_network_path_or_full_file_read_authority() {
    let source = include_str!("codex_npm_macho.rs");
    for forbidden in [
        "std::process",
        "tokio::process",
        "Command::new",
        "std::net",
        "reqwest",
        "std::env",
        "tauri::",
        "#[tauri::command]",
        "read_to_end",
        "fs::read",
        "canonicalize",
    ] {
        assert!(
            !source.contains(forbidden),
            "Mach-O collector acquired forbidden authority: {forbidden}"
        );
    }
}

fn signed_macho() -> Vec<u8> {
    let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
    for value in [0x0100_000c, 0, 2, 2, 40, 0, 0] {
        bytes.extend((value as u32).to_le_bytes());
    }
    bytes.extend(0x1bu32.to_le_bytes());
    bytes.extend(24u32.to_le_bytes());
    bytes.extend([7; 16]);
    bytes.extend(0x1du32.to_le_bytes());
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(72u32.to_le_bytes());
    bytes.extend(28u32.to_le_bytes());
    for value in [0xfade_0cc0u32, 28, 1, 0, 20, 0xfade_0c02, 8] {
        bytes.extend(value.to_be_bytes());
    }
    bytes
}

fn unsigned_macho() -> Vec<u8> {
    let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
    for value in [0x0100_000c, 0, 2, 1, 24, 0, 0] {
        bytes.extend((value as u32).to_le_bytes());
    }
    bytes.extend(0x1bu32.to_le_bytes());
    bytes.extend(24u32.to_le_bytes());
    bytes.extend([7; 16]);
    bytes
}

fn make_executable(path: &std::path::Path) {
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("permissions");
}
