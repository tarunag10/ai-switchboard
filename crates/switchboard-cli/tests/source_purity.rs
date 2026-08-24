use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_MANIFEST: &str = r#"[package]
name = "switchboard-cli"
version = "0.1.0"
description = "Read-only cross-platform CLI for AI Switchboard core contracts."
license = "MIT"
edition = "2021"
rust-version = "1.96"
publish = false

[[bin]]
name = "switchboard"
path = "src/main.rs"

[dependencies]
chrono = "0.4"
serde_json = "=1.0.149"
switchboard-core = { path = "../switchboard-core" }
switchboard-runtime = { path = "../switchboard-runtime" }
"#;

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

#[test]
fn manifest_is_exactly_the_reviewed_read_only_dependency_set() {
    let manifest = fs::read_to_string(crate_root().join("Cargo.toml")).expect("manifest");
    assert_eq!(manifest, EXPECTED_MANIFEST);
    for forbidden in [
        "tauri", "tokio", "reqwest", "ureq", "hyper", "rusqlite", "dirs", "keyring", "libc", "nix",
        "clap",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "forbidden dependency {forbidden}"
        );
    }
}

#[test]
fn production_source_has_no_file_process_network_or_provider_surface() {
    let mut files = Vec::new();
    collect_rust_sources(&crate_root().join("src"), &mut files);
    let paths = files
        .iter()
        .map(|path| {
            path.strip_prefix(crate_root())
                .expect("crate-relative source")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        paths,
        BTreeSet::from(["src/lib.rs".to_string(), "src/main.rs".to_string()])
    );

    for path in files {
        let source = fs::read_to_string(&path).expect("production source");
        for forbidden in [
            "std::fs",
            "std::net",
            "Command::new",
            ".spawn(",
            "TcpStream",
            "TcpListener",
            "UdpSocket",
            "UnixStream",
            "tokio",
            "reqwest",
            "ureq",
            "hyper",
            "tauri",
            "unsafe {",
            "libc::",
            "nix::",
        ] {
            assert!(
                !source.contains(forbidden),
                "production source {} contains forbidden surface {forbidden}",
                path.display()
            );
        }
    }

    assert!(!crate_root().join("build.rs").exists());
}
