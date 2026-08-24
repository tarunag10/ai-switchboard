use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

use sha2::{Digest, Sha256};

use super::codex_npm_fs::{CodexNpmDirectory, CodexNpmFsError};

#[test]
fn reads_and_hashes_a_stable_nested_tree() {
    let fixture = Fixture::new();
    let package = fixture.package();
    fs::write(package.join("package.json"), b"stable manifest").expect("write manifest");
    let directory = fixture.open_package();

    directory.revalidate().expect("stable tree revalidates");
    let read = directory
        .read_regular_file(OsStr::new("package.json"), 64)
        .expect("bounded read");
    let hashed = directory
        .hash_regular_file(OsStr::new("package.json"), 64)
        .expect("bounded hash");
    let expected_digest: [u8; 32] = Sha256::digest(b"stable manifest").into();

    assert_eq!(read.bytes, b"stable manifest");
    assert_eq!(read.content_digest, expected_digest);
    assert!(!format!("{read:?}").contains("stable manifest"));
    assert_eq!(hashed.digest, expected_digest);
    assert_eq!(read.identity, hashed.identity);
    assert!(!read.executable);
    assert!(!hashed.executable);
}

#[test]
fn rejects_symlinked_parent_and_regular_file_leaf() {
    let fixture = Fixture::new();
    let real = fixture.root().join("real");
    fs::create_dir(&real).expect("real directory");
    symlink(&real, fixture.root().join("linked")).expect("parent symlink");
    assert!(matches!(
        CodexNpmDirectory::open(fixture.root(), &[OsStr::new("linked")]),
        Err(CodexNpmFsError::DirectoryOpenFailed)
    ));

    let package = fixture.package();
    fs::write(package.join("real.json"), b"{}").expect("real leaf");
    symlink("real.json", package.join("linked.json")).expect("leaf symlink");
    assert_eq!(
        fixture
            .open_package()
            .read_regular_file(OsStr::new("linked.json"), 16)
            .unwrap_err(),
        CodexNpmFsError::FileOpenFailed
    );
    assert_eq!(
        fixture
            .open_package()
            .hash_regular_file(OsStr::new("linked.json"), 16)
            .unwrap_err(),
        CodexNpmFsError::FileOpenFailed
    );
}

#[test]
fn rejects_non_normal_components_and_relative_roots() {
    let fixture = Fixture::new();
    for invalid in ["", ".", "..", "/absolute", "nested/leaf"] {
        assert!(matches!(
            CodexNpmDirectory::open(fixture.root(), &[OsStr::new(invalid)]),
            Err(CodexNpmFsError::InvalidComponent)
        ));
    }
    assert!(matches!(
        CodexNpmDirectory::open(std::path::Path::new("relative"), &[OsStr::new("package")]),
        Err(CodexNpmFsError::RootNotAbsolute)
    ));
    assert_eq!(
        fixture
            .open_package()
            .read_regular_file(OsStr::new(".."), 16)
            .unwrap_err(),
        CodexNpmFsError::InvalidComponent
    );
}

#[test]
fn reads_a_link_without_following_and_enforces_its_limit() {
    let fixture = Fixture::new();
    symlink("12345678", fixture.package().join("codex-link")).expect("leaf symlink");
    let directory = fixture.open_package();

    assert_eq!(
        directory
            .read_link(OsStr::new("codex-link"), 7)
            .unwrap_err(),
        CodexNpmFsError::LinkTargetTooLong
    );
    assert_eq!(
        directory
            .read_link(OsStr::new("codex-link"), 8)
            .expect("exact limit")
            .target,
        OsStr::new("12345678")
    );
}

#[test]
fn rejects_a_regular_file_over_the_configured_limit() {
    let fixture = Fixture::new();
    fs::write(fixture.package().join("large"), b"12345").expect("large leaf");
    let directory = fixture.open_package();

    assert_eq!(
        directory
            .read_regular_file(OsStr::new("large"), 4)
            .unwrap_err(),
        CodexNpmFsError::FileTooLarge
    );
    assert_eq!(
        directory
            .hash_regular_file(OsStr::new("large"), 4)
            .unwrap_err(),
        CodexNpmFsError::FileTooLarge
    );
}

#[test]
fn rejects_a_non_regular_leaf_without_blocking() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.package().join("directory-leaf")).expect("directory leaf");

    assert_eq!(
        fixture
            .open_package()
            .read_regular_file(OsStr::new("directory-leaf"), 64)
            .unwrap_err(),
        CodexNpmFsError::FileNotRegular
    );
    assert_eq!(
        fixture
            .open_package()
            .hash_regular_file(OsStr::new("directory-leaf"), 64)
            .unwrap_err(),
        CodexNpmFsError::FileNotRegular
    );
}

#[test]
fn reports_the_executable_mode_from_stable_metadata() {
    let fixture = Fixture::new();
    let binary = fixture.package().join("codex");
    fs::write(&binary, b"binary").expect("binary leaf");
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).expect("executable mode");
    let directory = fixture.open_package();

    assert!(
        directory
            .read_regular_file(OsStr::new("codex"), 64)
            .expect("read executable")
            .executable
    );
    assert!(
        directory
            .hash_regular_file(OsStr::new("codex"), 64)
            .expect("hash executable")
            .executable
    );
}

#[test]
fn rejects_group_or_world_writable_package_components_and_files() {
    let fixture = Fixture::new();
    fs::set_permissions(fixture.package(), fs::Permissions::from_mode(0o777))
        .expect("writable package");
    assert!(matches!(
        CodexNpmDirectory::open(
            fixture.root(),
            &[
                OsStr::new("node_modules"),
                OsStr::new("@openai"),
                OsStr::new("codex"),
            ],
        ),
        Err(CodexNpmFsError::PermissionsRejected)
    ));

    fs::set_permissions(fixture.package(), fs::Permissions::from_mode(0o755))
        .expect("restore package");
    let manifest = fixture.package().join("package.json");
    fs::write(&manifest, b"{}").expect("manifest");
    fs::set_permissions(&manifest, fs::Permissions::from_mode(0o666)).expect("writable manifest");
    assert_eq!(
        fixture
            .open_package()
            .read_regular_file(OsStr::new("package.json"), 16)
            .unwrap_err(),
        CodexNpmFsError::PermissionsRejected
    );
}

#[test]
fn revalidation_fails_after_a_directory_is_replaced() {
    let fixture = Fixture::new();
    let package = fixture.package();
    let directory = fixture.open_package();
    let displaced = fixture.root().join("package-old");
    fs::rename(&package, &displaced).expect("displace package");
    fs::create_dir(&package).expect("replacement package");

    assert_eq!(
        directory.revalidate().unwrap_err(),
        CodexNpmFsError::RevalidationFailed
    );
}

#[test]
fn source_contains_no_write_process_network_environment_or_framework_authority() {
    let source = include_str!("codex_npm_fs.rs");
    for forbidden in [
        ".write(",
        ".create(",
        "write(true)",
        "create(true)",
        "truncate(true)",
        "append(true)",
        "create_dir",
        "remove_file",
        "remove_dir",
        "rename(",
        "std::process",
        "tokio::process",
        "Command::new",
        "std::net",
        "reqwest",
        "std::env",
        "serde",
        "tauri::",
        "#[tauri::command]",
    ] {
        assert!(
            !source.contains(forbidden),
            "filesystem primitive acquired forbidden authority: {forbidden}"
        );
    }
}

struct Fixture {
    directory: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary root");
        fs::create_dir_all(directory.path().join("node_modules/@openai/codex"))
            .expect("nested package directory");
        Self { directory }
    }

    fn root(&self) -> &std::path::Path {
        self.directory.path()
    }

    fn package(&self) -> std::path::PathBuf {
        self.root().join("node_modules/@openai/codex")
    }

    fn open_package(&self) -> CodexNpmDirectory {
        CodexNpmDirectory::open(
            self.root(),
            &[
                OsStr::new("node_modules"),
                OsStr::new("@openai"),
                OsStr::new("codex"),
            ],
        )
        .expect("open nested package")
    }
}
