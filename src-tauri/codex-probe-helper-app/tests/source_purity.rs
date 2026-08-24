mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use support::run_command;

const EXPECTED_MANIFEST: &str = r#"[package]
name = "codex-probe-helper-app"
version = "0.1.0"
edition = "2021"
license = "MIT"
publish = false

[[bin]]
name = "ai-switchboard-codex-probe"
path = "src/main.rs"

[dependencies]
codex-probe-helper = { path = "../codex-probe-helper" }

[dev-dependencies]
sha2 = "=0.10.9"
"#;

const EXPECTED_INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>AI Switchboard Codex Probe</string>
    <key>CFBundleExecutable</key>
    <string>ai-switchboard-codex-probe</string>
    <key>CFBundleIdentifier</key>
    <string>com.tarunagarwal.mac-ai-switchboard.codex-probe</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>AI Switchboard Codex Probe</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSBackgroundOnly</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
</dict>
</plist>
"#;

const EXPECTED_ENTITLEMENTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
</dict>
</plist>
"#;

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read helper-app directory") {
        let path = entry.expect("helper-app entry").path();
        assert!(!fs::symlink_metadata(&path)
            .expect("entry metadata")
            .file_type()
            .is_symlink());
        if path == crate_root().join("target") && path.is_dir() {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

#[test]
fn source_directory_contains_only_reviewed_files_besides_cargo_output() {
    let mut files = Vec::new();
    collect_files(crate_root(), &mut files);
    let actual: BTreeSet<_> = files
        .iter()
        .map(|path| {
            path.strip_prefix(crate_root())
                .expect("crate-relative path")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let expected = BTreeSet::from([
        "Cargo.lock".to_owned(),
        "Cargo.toml".to_owned(),
        "Entitlements.plist".to_owned(),
        "Info.plist".to_owned(),
        "README.md".to_owned(),
        "src/main.rs".to_owned(),
        "tests/protocol_stdio_contract.rs".to_owned(),
        "tests/source_purity.rs".to_owned(),
        "tests/support/mod.rs".to_owned(),
    ]);
    assert_eq!(actual, expected);
}

#[test]
fn manifest_and_cargo_metadata_expose_one_binary_and_one_runtime_dependency() {
    let manifest = fs::read_to_string(crate_root().join("Cargo.toml")).expect("read manifest");
    assert_eq!(manifest, EXPECTED_MANIFEST);

    let mut command = Command::new(env!("CARGO"));
    command
        .args([
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(crate_root().join("Cargo.toml"));
    let output = run_command(&mut command, None, Duration::ZERO);
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata = String::from_utf8(output.stdout).expect("metadata UTF-8");
    assert_eq!(
        metadata
            .matches("\"name\":\"codex-probe-helper-app\"")
            .count(),
        1
    );
    assert_eq!(
        metadata.matches("\"name\":\"codex-probe-helper\"").count(),
        1
    );
    assert_eq!(metadata.matches("\"name\":\"sha2\"").count(), 1);
    assert!(metadata.contains("\"kind\":[\"bin\"]"));
    assert!(!metadata.contains("\"kind\":[\"lib\"]"));
    assert!(metadata.contains("\"kind\":null"));
    assert!(metadata.contains("\"kind\":\"dev\""));
    assert!(!metadata.contains("\"custom-build\""));

    let lock = fs::read_to_string(crate_root().join("Cargo.lock")).expect("read lockfile");
    assert!(lock.contains("name = \"codex-probe-helper-app\""));
    for forbidden in ["tauri", "tokio", "reqwest", "nix", "anyhow"] {
        assert!(!lock.contains(&format!("name = \"{forbidden}\"")));
    }
}

#[test]
fn production_source_is_bounded_io_only() {
    let source = fs::read_to_string(crate_root().join("src/main.rs")).expect("read main source");
    for required in [
        "#![forbid(unsafe_code)]",
        "MAX_FRAME_BYTES",
        "read_exact",
        "decode_preparation_request",
        "prepare_shape_consistent_non_executing_response",
        "encode_preparation_response",
        "write_all",
    ] {
        assert!(
            source.contains(required),
            "missing required boundary {required}"
        );
    }
    for forbidden in [
        "std::args",
        "std::env",
        "std::fs",
        "std::path",
        "std::net",
        "std::process",
        "std::os",
        "Command",
        "File::",
        "OpenOptions",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "UnixStream",
        "UnixListener",
        "tokio",
        "reqwest",
        "tauri",
        "libc",
        "nix",
        "shell",
        "argv",
        "args()",
        "--version",
        "workspace",
        "provider",
        "credential",
        "eprintln!",
        "println!",
        "dbg!",
        "unsafe {",
        "extern crate",
    ] {
        assert!(
            !source.contains(forbidden),
            "production source contains forbidden surface {forbidden}"
        );
    }
}

#[test]
fn nested_app_identity_and_entitlements_are_exact() {
    let info = fs::read_to_string(crate_root().join("Info.plist")).expect("read Info.plist");
    let entitlements =
        fs::read_to_string(crate_root().join("Entitlements.plist")).expect("read entitlements");
    assert_eq!(info, EXPECTED_INFO_PLIST);
    assert_eq!(entitlements, EXPECTED_ENTITLEMENTS);
    assert_eq!(entitlements.matches("<key>").count(), 1);

    for forbidden in [
        "com.apple.security.network.client",
        "com.apple.security.network.server",
        "com.apple.security.cs.allow-jit",
        "com.apple.security.cs.allow-unsigned-executable-memory",
        "com.apple.security.cs.disable-library-validation",
        "com.apple.security.inherit",
        "com.apple.security.files.user-selected",
        "com.apple.security.temporary-exception",
    ] {
        assert!(!entitlements.contains(forbidden));
    }
}

#[test]
fn readme_preserves_the_unbundled_unlaunched_release_boundary() {
    let readme = fs::read_to_string(crate_root().join("README.md")).expect("read README");
    for required in [
        "MIT-licensed",
        "private, non-commercial research use",
        "non-executing protocol-v1",
        "defines no bundling, installation, independent signing, launch, or parent-app connection",
        "linker ad-hoc signed",
        "not independent or release signing",
        "separate phase",
    ] {
        assert!(readme.contains(required), "README missing {required}");
    }
}

#[test]
fn parent_build_graph_and_production_sources_do_not_reference_the_helper() {
    let parent = crate_root().parent().expect("src-tauri parent");
    let repository = parent.parent().expect("repository parent");
    let build_graph = [
        parent.join("Cargo.toml"),
        parent.join("build.rs"),
        parent.join("tauri.conf.json"),
        repository.join("package.json"),
    ];
    for path in build_graph {
        let source = fs::read_to_string(&path).expect("read parent build file");
        for forbidden in [
            "codex-probe-helper-app",
            "ai-switchboard-codex-probe",
            "com.tarunagarwal.mac-ai-switchboard.codex-probe",
        ] {
            assert!(
                !source.contains(forbidden),
                "parent graph references helper identity in {}",
                path.display()
            );
        }
    }

    let mut production_sources = Vec::new();
    collect_files(&parent.join("src"), &mut production_sources);
    for path in production_sources
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
    {
        let source = fs::read_to_string(&path).expect("read parent build/source file");
        for forbidden in [
            "codex-probe-helper-app",
            "\"ai-switchboard-codex-probe\"",
            "com.tarunagarwal.mac-ai-switchboard.codex-probe",
        ] {
            assert!(
                !source.contains(forbidden),
                "parent graph/source references helper identity in {}",
                path.display()
            );
        }
    }
}
