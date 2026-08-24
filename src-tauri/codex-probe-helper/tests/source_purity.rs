use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rust_sources(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read source directory") {
        let path = entry.expect("source directory entry").path();
        if path.is_dir() {
            collect_rust_sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn cargo_metadata(manifest: &Path) -> Value {
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("metadata JSON")
}

fn json_contains_string(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(value) => value.contains(needle),
        Value::Array(values) => values
            .iter()
            .any(|value| json_contains_string(value, needle)),
        Value::Object(values) => values
            .iter()
            .any(|(key, value)| key.contains(needle) || json_contains_string(value, needle)),
        _ => false,
    }
}

#[test]
fn production_source_has_no_execution_transport_or_platform_surface() {
    let expected_paths = BTreeSet::from([
        "src/digest.rs".to_owned(),
        "src/error.rs".to_owned(),
        "src/lib.rs".to_owned(),
        "src/protocol.rs".to_owned(),
    ]);
    let mut source_paths = Vec::new();
    collect_rust_sources(&crate_root().join("src"), &mut source_paths);
    let actual_paths: BTreeSet<_> = source_paths
        .iter()
        .map(|path| {
            path.strip_prefix(crate_root())
                .expect("crate-relative source")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(actual_paths, expected_paths, "unexpected production module");
    let forbidden = [
        "std::process",
        "std::{",
        "std::net",
        "std::os",
        "std::io",
        "Command::new",
        "std::fs",
        "std::env",
        "std::path",
        "std::io::Read",
        "std::io::Write",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "UnixStream",
        "UnixListener",
        "tokio",
        "reqwest",
        "tauri",
        "unsafe {",
        "libc::",
        "nix::",
        "extern crate",
        "#[path",
        "Foundation",
        "Security.framework",
    ];

    for path in source_paths {
        let relative = path
            .strip_prefix(crate_root())
            .expect("crate-relative source")
            .display();
        let source = fs::read_to_string(&path).expect("read source");
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "production source {relative} contains forbidden surface {needle}"
            );
        }
    }

    assert!(!crate_root().join("src/main.rs").exists());
    assert!(!crate_root().join("build.rs").exists());
}

#[test]
fn manifest_is_a_single_library_with_three_pinned_dependencies() {
    let manifest = fs::read_to_string(crate_root().join("Cargo.toml")).expect("read manifest");
    for forbidden in [
        "[[bin]]",
        "[features]",
        "build =",
        "tauri",
        "tokio",
        "reqwest",
        "libc",
        "nix",
        "anyhow",
        "thiserror",
        "chrono",
        "uuid",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "manifest contains forbidden entry {forbidden}"
        );
    }

    let metadata = cargo_metadata(&crate_root().join("Cargo.toml"));
    let packages = metadata["packages"].as_array().expect("packages array");
    assert_eq!(packages.len(), 1);
    let package = &packages[0];
    assert_eq!(package["name"], "codex-probe-helper");

    let targets = package["targets"].as_array().expect("targets array");
    assert_eq!(
        targets.len(),
        3,
        "library plus two integration-test targets"
    );
    let library_targets: Vec<_> = targets
        .iter()
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "lib"))
        })
        .collect();
    assert_eq!(library_targets.len(), 1);
    assert!(targets.iter().all(|target| {
        target["kind"]
            .as_array()
            .is_some_and(|kinds| kinds.iter().all(|kind| kind == "lib" || kind == "test"))
    }));

    let dependencies: BTreeSet<_> = package["dependencies"]
        .as_array()
        .expect("dependencies array")
        .iter()
        .map(|dependency| dependency["name"].as_str().expect("dependency name"))
        .collect();
    assert_eq!(
        dependencies,
        BTreeSet::from(["serde", "serde_json", "sha2"])
    );
}

#[test]
fn parent_application_bundles_only_the_separate_helper_app() {
    let parent = crate_root().parent().expect("src-tauri parent");
    let tauri_config = fs::read_to_string(parent.join("tauri.conf.json")).expect("Tauri config");
    let tauri_json: Value = serde_json::from_str(&tauri_config).expect("valid Tauri config");
    assert_eq!(
        tauri_json["build"]["beforeBundleCommand"],
        "./scripts/prepare-codex-probe-helper-app.sh"
    );
    assert_eq!(
        tauri_json["bundle"]["macOS"]["files"]["Helpers/AI Switchboard Codex Probe.app"],
        "target/codex-probe-helper-bundle/AI Switchboard Codex Probe.app"
    );
    assert!(json_contains_string(
        &tauri_json,
        "codex-probe-helper-bundle"
    ));

    let app_metadata = cargo_metadata(&parent.join("Cargo.toml"));
    for package in app_metadata["packages"].as_array().expect("app packages") {
        assert_ne!(package["name"], "codex-probe-helper");
        assert!(package["dependencies"]
            .as_array()
            .expect("app dependencies")
            .iter()
            .all(|dependency| dependency["name"] != "codex-probe-helper"));
    }
}
