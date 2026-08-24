#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

use super::codex_command_catalog::{plan_codex_version_probe, CodexProbePlan};
use super::codex_command_collector::collect_codex_command_snapshot_with_roots;
use super::codex_macho::CodexMachOArchitecture;
use super::codex_npm_fs::CodexNpmFsError;
use super::codex_npm_launcher_chain::{
    collect_codex_npm_launcher_chain_with_context, CodexNpmChainCollectionError,
    CodexNpmCollectorHookPoint, CodexNpmObject,
};

#[test]
fn collects_the_exact_arm64_chain_without_execution_authority() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    let observation = fixture
        .collect(&plan, &mut |_| {})
        .expect("collect stable chain");

    assert_eq!(observation.candidate_id, "home-npm-global-bin");
    assert_eq!(observation.root_version, "0.142.3");
    assert_eq!(observation.dependency_alias, "@openai/codex-darwin-arm64");
    assert_eq!(observation.payload_target, "aarch64-apple-darwin");
    assert_eq!(observation.payload_entrypoint, "bin/codex");
    assert_eq!(observation.state, "collected_unbound_non_executing");
    for digest in [
        observation.launcher_identity_digest,
        observation.launcher_symlink_identity_digest,
        observation.root_manifest_identity_digest,
        observation.platform_manifest_identity_digest,
        observation.payload_manifest_identity_digest,
        observation.payload_file_identity_digest,
        observation.derivation_identity_digest,
        observation.collection_identity_digest,
    ] {
        assert!(digest.starts_with("sha256:"));
    }
}

#[test]
fn unsupported_fixed_candidate_is_not_reinterpreted_as_an_npm_path() {
    let fixture = Fixture::new();
    let mut plan = fixture.plan();
    plan.candidate_id = "home-local-bin".into();
    assert_eq!(
        fixture.collect(&plan, &mut |_| {}).unwrap_err(),
        CodexNpmChainCollectionError::UnsupportedCandidate
    );
}

#[test]
fn launcher_link_must_have_the_exact_raw_relative_target() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.launcher_link()).expect("remove exact link");
    symlink(
        "../lib/node_modules/@openai/codex/bin/./codex.js",
        fixture.launcher_link(),
    )
    .expect("alternate lexical link");
    let plan = fixture.plan();
    assert_eq!(
        fixture.collect(&plan, &mut |_| {}).unwrap_err(),
        CodexNpmChainCollectionError::LauncherLinkMismatch
    );
}

#[test]
fn root_platform_and_payload_manifest_policy_fail_closed() {
    for case in ["root", "platform", "payload"] {
        let fixture = Fixture::new();
        let plan = fixture.plan();
        match case {
            "root" => fs::write(
                fixture.package().join("package.json"),
                root_manifest().replace("@openai/codex\"", "other\""),
            )
            .expect("tamper root"),
            "platform" => fs::write(
                fixture.platform().join("package.json"),
                platform_manifest().replace("arm64", "x64"),
            )
            .expect("tamper platform"),
            "payload" => fs::write(
                fixture.payload().join("codex-package.json"),
                payload_manifest().replace("bin/codex", "bin/other"),
            )
            .expect("tamper payload"),
            _ => unreachable!(),
        }
        assert_eq!(
            fixture.collect(&plan, &mut |_| {}).unwrap_err(),
            CodexNpmChainCollectionError::PackagePolicyRejected,
            "case should reject: {case}"
        );
    }
}

#[test]
fn descriptor_traversal_rejects_a_symlinked_package_parent() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    let node_modules = fixture.npm_root().join("lib/node_modules");
    let displaced = fixture.npm_root().join("lib/node_modules-real");
    fs::rename(&node_modules, &displaced).expect("displace node_modules");
    symlink(&displaced, &node_modules).expect("symlink package parent");

    assert!(matches!(
        fixture.collect(&plan, &mut |_| {}),
        Err(CodexNpmChainCollectionError::Filesystem(
            CodexNpmObject::PackageRoot,
            CodexNpmFsError::DirectoryOpenFailed
        ))
    ));
}

#[test]
fn manifest_mutation_after_read_is_detected_before_receipt_binding() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    let manifest = fixture.package().join("package.json");
    let result = fixture.collect(&plan, &mut |point| {
        if point == CodexNpmCollectorHookPoint::AfterRootManifest {
            fs::write(&manifest, root_manifest().replace("0.142.3", "0.142.4"))
                .expect("mutate root manifest");
        }
    });
    assert!(matches!(
        result,
        Err(CodexNpmChainCollectionError::Filesystem(
            CodexNpmObject::RootManifest,
            CodexNpmFsError::FileChanged
        ))
    ));
}

#[test]
fn payload_file_mutation_after_hash_is_detected() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    let payload = fixture.payload().join("bin/codex");
    let result = fixture.collect(&plan, &mut |point| {
        if point == CodexNpmCollectorHookPoint::AfterPayloadHash {
            fs::write(&payload, b"changed native payload").expect("mutate payload");
            make_executable(&payload);
        }
    });
    assert!(matches!(
        result,
        Err(CodexNpmChainCollectionError::Filesystem(
            CodexNpmObject::PayloadFile,
            CodexNpmFsError::FileChanged
        ))
    ));
}

#[test]
fn launcher_replacement_after_plan_validation_invalidates_the_plan() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    let link = fixture.launcher_link();
    let result = fixture.collect(&plan, &mut |point| {
        if point == CodexNpmCollectorHookPoint::AfterPlanValidation {
            fs::remove_file(&link).expect("remove launcher link");
            symlink("../lib/node_modules/@openai/codex/bin/codex.js", &link)
                .expect("replace launcher link");
        }
    });
    assert_eq!(
        result.unwrap_err(),
        CodexNpmChainCollectionError::ProbeIdentityMismatch
    );
}

#[test]
fn launcher_swap_and_restore_cannot_mix_probe_and_descriptor_generations() {
    let fixture = Fixture::new();
    let plan = fixture.plan();
    let launcher = fixture.package().join("bin/codex.js");
    let original = fixture.package().join("bin/codex-original.js");
    let mut swapped = false;
    let result = fixture.collect(&plan, &mut |point| match point {
        CodexNpmCollectorHookPoint::AfterLauncherLink => {
            fs::rename(&launcher, &original).expect("retain launcher generation A");
            fs::write(&launcher, b"#!/usr/bin/env node\n// generation B\n")
                .expect("install launcher generation B");
            make_executable(&launcher);
            swapped = true;
        }
        CodexNpmCollectorHookPoint::AfterLauncherFileRead if swapped => {
            fs::remove_file(&launcher).expect("remove launcher generation B");
            fs::rename(&original, &launcher).expect("restore launcher generation A");
        }
        _ => {}
    });
    assert_eq!(
        result.unwrap_err(),
        CodexNpmChainCollectionError::ProbeIdentityMismatch
    );
}

#[test]
fn transitive_collector_sources_contain_no_execution_network_or_renderer_authority() {
    let collector_source = include_str!("codex_npm_launcher_chain.rs");
    assert!(
        !collector_source.contains("codex_command_collector"),
        "production collector must not regain the path-based collector dependency"
    );
    let source = [
        collector_source,
        include_str!("codex_npm_launcher_chain_digest.rs"),
        include_str!("codex_npm_chain_model.rs"),
        include_str!("codex_npm_manifest.rs"),
        include_str!("codex_npm_fs.rs"),
        include_str!("codex_command_identity.rs"),
    ]
    .join("\n");
    for forbidden in [
        "std::process",
        "tokio::process",
        "Command::new",
        ".spawn(",
        "\"node\"",
        "\"npm\"",
        "require.resolve",
        "std::env",
        "reqwest",
        "std::net",
        "TcpStream",
        "serde_json::Value",
        "fs::write",
        "File::create",
        "tauri::",
        "#[tauri::command]",
    ] {
        assert!(
            !source.contains(forbidden),
            "collector acquired forbidden authority: {forbidden}"
        );
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("npm collector fixture");
        let root = directory.path().join("root");
        let home = root.join("Users/researcher");
        let npm_root = home.join(".npm-global");
        let package = npm_root.join("lib/node_modules/@openai/codex");
        let platform = package.join("node_modules/@openai/codex-darwin-arm64");
        let payload = platform.join("vendor/aarch64-apple-darwin");
        fs::create_dir_all(npm_root.join("bin")).expect("npm bin");
        fs::create_dir_all(package.join("bin")).expect("launcher bin");
        fs::create_dir_all(payload.join("bin")).expect("payload bin");
        fs::create_dir(payload.join("codex-resources")).expect("payload resources");
        fs::create_dir(payload.join("codex-path")).expect("payload path");
        fs::write(package.join("package.json"), root_manifest()).expect("root manifest");
        fs::write(platform.join("package.json"), platform_manifest()).expect("platform manifest");
        fs::write(payload.join("codex-package.json"), payload_manifest())
            .expect("payload manifest");
        let launcher = package.join("bin/codex.js");
        fs::write(&launcher, b"static launcher bytes; never interpreted").expect("launcher");
        make_executable(&launcher);
        let native = payload.join("bin/codex");
        fs::write(&native, b"synthetic native payload bytes").expect("native payload");
        make_executable(&native);
        symlink(
            "../lib/node_modules/@openai/codex/bin/codex.js",
            npm_root.join("bin/codex"),
        )
        .expect("launcher link");
        Self {
            _directory: directory,
            root,
            home,
        }
    }

    fn npm_root(&self) -> PathBuf {
        self.home.join(".npm-global")
    }

    fn package(&self) -> PathBuf {
        self.npm_root().join("lib/node_modules/@openai/codex")
    }

    fn platform(&self) -> PathBuf {
        self.package()
            .join("node_modules/@openai/codex-darwin-arm64")
    }

    fn payload(&self) -> PathBuf {
        self.platform().join("vendor/aarch64-apple-darwin")
    }

    fn launcher_link(&self) -> PathBuf {
        self.npm_root().join("bin/codex")
    }

    fn plan(&self) -> CodexProbePlan {
        plan_codex_version_probe(&collect_codex_command_snapshot_with_roots(
            Some(&self.home),
            &self.root,
        ))
        .expect("fixed npm launcher plan")
    }

    fn collect(
        &self,
        plan: &CodexProbePlan,
        hook: &mut impl FnMut(CodexNpmCollectorHookPoint),
    ) -> Result<
        super::codex_npm_chain_model::CodexNpmLauncherChainObservation,
        CodexNpmChainCollectionError,
    > {
        collect_codex_npm_launcher_chain_with_context(
            plan,
            CodexMachOArchitecture::Arm64,
            &self.home,
            &self.root,
            hook,
        )
    }
}

fn make_executable(path: &Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make executable");
}

fn root_manifest() -> String {
    r#"{"name":"@openai/codex","version":"0.142.3","bin":{"codex":"bin/codex.js"},"optionalDependencies":{"@openai/codex-darwin-arm64":"npm:@openai/codex@0.142.3-darwin-arm64"}}"#.into()
}

fn platform_manifest() -> String {
    r#"{"name":"@openai/codex","version":"0.142.3-darwin-arm64","os":["darwin"],"cpu":["arm64"]}"#
        .into()
}

fn payload_manifest() -> String {
    r#"{"layoutVersion":1,"version":"0.142.3","target":"aarch64-apple-darwin","variant":"codex","entrypoint":"bin/codex","resourcesDir":"codex-resources","pathDir":"codex-path"}"#.into()
}
