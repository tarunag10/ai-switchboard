use super::codex_macho::CodexMachOArchitecture;
use super::codex_npm_chain_model::{
    bind_codex_npm_launcher_chain, codex_npm_host_policy, CodexNpmCollectedEvidence,
};

#[test]
fn arm64_evidence_is_bound_without_becoming_executable_authority() {
    let observation =
        bind_codex_npm_launcher_chain(CodexMachOArchitecture::Arm64, arm64_evidence())
            .expect("valid arm64 evidence");
    assert_eq!(observation.schema_version, 1);
    assert_eq!(observation.payload_target, "aarch64-apple-darwin");
    assert_eq!(observation.payload_entrypoint, "bin/codex");
    assert_eq!(observation.state, "collected_unbound_non_executing");
    assert!(observation
        .collection_identity_digest
        .starts_with("sha256:"));
}

#[test]
fn x86_64_policy_uses_the_exact_platform_alias_and_target() {
    assert_eq!(
        codex_npm_host_policy(CodexMachOArchitecture::X86_64).platform_directory_name,
        "codex-darwin-x64"
    );
    let mut evidence = arm64_evidence();
    evidence.dependency_alias = "@openai/codex-darwin-x64".into();
    evidence.dependency_version_spec = "npm:@openai/codex@0.142.3-darwin-x64".into();
    evidence.platform_version = "0.142.3-darwin-x64".into();
    evidence.platform_cpu = "x64".into();
    evidence.payload_target = "x86_64-apple-darwin".into();
    let observation = bind_codex_npm_launcher_chain(CodexMachOArchitecture::X86_64, evidence)
        .expect("valid x86 evidence");
    assert_eq!(observation.dependency_alias, "@openai/codex-darwin-x64");
    assert_eq!(observation.payload_target, "x86_64-apple-darwin");
}

#[test]
fn every_fixed_shape_field_fails_closed_when_tampered() {
    for case in [
        "candidate",
        "root_name",
        "root_bin",
        "root_entrypoint",
        "alias",
        "dependency",
        "platform_name",
        "platform_version",
        "os",
        "cpu",
        "layout",
        "payload_version",
        "target",
        "variant",
        "entrypoint",
        "resources",
        "path_directory",
        "regular",
        "executable",
    ] {
        let mut evidence = arm64_evidence();
        match case {
            "candidate" => evidence.candidate_id = "other".into(),
            "root_name" => evidence.root_package_name = "other".into(),
            "root_bin" => evidence.root_bin_name = "other".into(),
            "root_entrypoint" => evidence.root_bin_entrypoint = "other".into(),
            "alias" => evidence.dependency_alias = "other".into(),
            "dependency" => evidence.dependency_version_spec = "other".into(),
            "platform_name" => evidence.platform_package_name = "other".into(),
            "platform_version" => evidence.platform_version = "other".into(),
            "os" => evidence.platform_os = "other".into(),
            "cpu" => evidence.platform_cpu = "other".into(),
            "layout" => evidence.payload_layout_version = 2,
            "payload_version" => evidence.payload_version = "9.9.9".into(),
            "target" => evidence.payload_target = "other".into(),
            "variant" => evidence.payload_variant = "other".into(),
            "entrypoint" => evidence.payload_entrypoint = "other".into(),
            "resources" => evidence.payload_resources_directory = "other".into(),
            "path_directory" => evidence.payload_path_directory = "other".into(),
            "regular" => evidence.payload_file_is_regular = false,
            "executable" => evidence.payload_file_is_executable = false,
            _ => unreachable!(),
        }
        assert!(
            bind_codex_npm_launcher_chain(CodexMachOArchitecture::Arm64, evidence).is_err(),
            "case should fail closed: {case}"
        );
    }
}

#[test]
fn every_opaque_identity_must_be_a_valid_digest() {
    for case in [
        "launcher",
        "launcher_symlink",
        "root",
        "platform",
        "payload_manifest",
        "payload",
        "derivation",
    ] {
        let mut evidence = arm64_evidence();
        match case {
            "launcher" => evidence.launcher_identity_digest = "bad".into(),
            "launcher_symlink" => evidence.launcher_symlink_identity_digest = "bad".into(),
            "root" => evidence.root_manifest_identity_digest = "bad".into(),
            "platform" => evidence.platform_manifest_identity_digest = "bad".into(),
            "payload_manifest" => evidence.payload_manifest_identity_digest = "bad".into(),
            "payload" => evidence.payload_file_identity_digest = "bad".into(),
            "derivation" => evidence.derivation_identity_digest = "bad".into(),
            _ => unreachable!(),
        }
        assert!(bind_codex_npm_launcher_chain(CodexMachOArchitecture::Arm64, evidence).is_err());
    }
}

#[test]
fn collection_digest_changes_with_valid_version_or_identity_evidence() {
    let first = bind_codex_npm_launcher_chain(CodexMachOArchitecture::Arm64, arm64_evidence())
        .expect("first");
    let mut changed = arm64_evidence();
    changed.root_version = "0.143.0-alpha.1".into();
    changed.dependency_version_spec = "npm:@openai/codex@0.143.0-alpha.1-darwin-arm64".into();
    changed.platform_version = "0.143.0-alpha.1-darwin-arm64".into();
    changed.payload_version = "0.143.0-alpha.1".into();
    changed.payload_file_identity_digest = digest('f');
    let second =
        bind_codex_npm_launcher_chain(CodexMachOArchitecture::Arm64, changed).expect("second");
    assert_ne!(
        first.collection_identity_digest,
        second.collection_identity_digest
    );
}

#[test]
fn current_model_source_has_no_collection_or_execution_authority() {
    let source = include_str!("codex_npm_chain_model.rs");
    for forbidden in [
        "std::fs",
        "std::process",
        "tokio::process",
        "std::net",
        "reqwest",
        "std::env",
        "libc::",
        "unsafe",
        "tauri::",
        "#[tauri::command]",
    ] {
        assert!(
            !source.contains(forbidden),
            "model acquired forbidden authority: {forbidden}"
        );
    }
}

fn arm64_evidence() -> CodexNpmCollectedEvidence {
    CodexNpmCollectedEvidence {
        candidate_id: "home-npm-global-bin".into(),
        launcher_identity_digest: digest('1'),
        launcher_symlink_identity_digest: digest('6'),
        root_manifest_identity_digest: digest('2'),
        root_package_name: "@openai/codex".into(),
        root_version: "0.142.3".into(),
        root_bin_name: "codex".into(),
        root_bin_entrypoint: "bin/codex.js".into(),
        dependency_alias: "@openai/codex-darwin-arm64".into(),
        dependency_version_spec: "npm:@openai/codex@0.142.3-darwin-arm64".into(),
        platform_manifest_identity_digest: digest('3'),
        platform_package_name: "@openai/codex".into(),
        platform_version: "0.142.3-darwin-arm64".into(),
        platform_os: "darwin".into(),
        platform_cpu: "arm64".into(),
        payload_manifest_identity_digest: digest('4'),
        payload_layout_version: 1,
        payload_version: "0.142.3".into(),
        payload_target: "aarch64-apple-darwin".into(),
        payload_variant: "codex".into(),
        payload_entrypoint: "bin/codex".into(),
        payload_resources_directory: "codex-resources".into(),
        payload_path_directory: "codex-path".into(),
        payload_file_identity_digest: digest('5'),
        derivation_identity_digest: digest('7'),
        payload_file_is_regular: true,
        payload_file_is_executable: true,
    }
}

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}
