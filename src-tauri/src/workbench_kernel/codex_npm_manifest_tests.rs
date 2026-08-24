use super::codex_npm_manifest::{
    is_codex_npm_semver, parse_codex_npm_payload_manifest, parse_codex_npm_platform_manifest,
    parse_codex_npm_root_manifest, CodexNpmManifestError, MAX_CODEX_NPM_MANIFEST_BYTES,
};

const HOST_ALIAS: &str = "@openai/codex-darwin-arm64";

fn root(version: &str) -> String {
    format!(
        r#"{{"name":"@openai/codex","version":"{version}","bin":{{"codex":"bin/codex.js"}},"optionalDependencies":{{"@openai/codex-darwin-x64":"npm:@openai/codex@{version}-darwin-x64","{HOST_ALIAS}":"npm:@openai/codex@{version}-darwin-arm64"}}}}"#
    )
}

fn platform(version: &str) -> String {
    format!(r#"{{"name":"@openai/codex","version":"{version}","os":["darwin"],"cpu":["arm64"]}}"#)
}

fn payload(version: &str) -> String {
    format!(
        r#"{{"layoutVersion":1,"version":"{version}","target":"aarch64-apple-darwin","variant":"codex","entrypoint":"bin/codex","resourcesDir":"codex-resources","pathDir":"codex-path"}}"#
    )
}

#[test]
fn parses_the_three_bounded_schemas() {
    let root = parse_codex_npm_root_manifest(root("1.2.3").as_bytes(), HOST_ALIAS).unwrap();
    assert_eq!(root.name, "@openai/codex");
    assert_eq!(root.version, "1.2.3");
    assert_eq!(root.bin_codex, "bin/codex.js");
    assert_eq!(root.host_dependency_alias, HOST_ALIAS);
    assert_eq!(
        root.host_dependency_spec,
        "npm:@openai/codex@1.2.3-darwin-arm64"
    );

    let platform = parse_codex_npm_platform_manifest(platform("1.2.3").as_bytes()).unwrap();
    assert_eq!(platform.os, "darwin");
    assert_eq!(platform.cpu, "arm64");

    let payload = parse_codex_npm_payload_manifest(payload("1.2.3").as_bytes()).unwrap();
    assert_eq!(payload.layout_version, 1);
    assert_eq!(payload.version, "1.2.3");
    assert_eq!(payload.target, "aarch64-apple-darwin");
    assert_eq!(payload.variant, "codex");
    assert_eq!(payload.entrypoint, "bin/codex");
    assert_eq!(payload.resources_dir, "codex-resources");
    assert_eq!(payload.path_dir, "codex-path");
}

#[test]
fn accepts_exactly_64_kib_and_rejects_limit_plus_one() {
    let mut boundary = root("1.2.3").into_bytes();
    boundary.resize(MAX_CODEX_NPM_MANIFEST_BYTES, b' ');
    assert!(parse_codex_npm_root_manifest(&boundary, HOST_ALIAS).is_ok());

    boundary.push(b' ');
    assert_eq!(
        parse_codex_npm_root_manifest(&boundary, HOST_ALIAS),
        Err(CodexNpmManifestError::InputTooLarge)
    );
}

#[test]
fn rejects_duplicate_security_sensitive_keys() {
    let duplicate_name = format!(
        r#"{{"name":"first","name":"second","version":"1.2.3","bin":{{"codex":"bin/codex.js"}},"optionalDependencies":{{"{HOST_ALIAS}":"1.2.3"}}}}"#
    );
    assert!(parse_codex_npm_root_manifest(duplicate_name.as_bytes(), HOST_ALIAS).is_err());

    let duplicate_dependencies = format!(
        r#"{{"name":"root","version":"1.2.3","bin":{{"codex":"bin/codex.js"}},"optionalDependencies":{{"{HOST_ALIAS}":"first"}},"optionalDependencies":{{"{HOST_ALIAS}":"second"}}}}"#
    );
    assert!(parse_codex_npm_root_manifest(duplicate_dependencies.as_bytes(), HOST_ALIAS).is_err());

    let duplicate_alias = format!(
        r#"{{"name":"root","version":"1.2.3","bin":{{"codex":"bin/codex.js"}},"optionalDependencies":{{"{HOST_ALIAS}":"first","{HOST_ALIAS}":"second"}}}}"#
    );
    assert_eq!(
        parse_codex_npm_root_manifest(duplicate_alias.as_bytes(), HOST_ALIAS),
        Err(CodexNpmManifestError::DuplicateHostAlias)
    );

    let duplicate_target = payload("1.2.3").replacen(
        r#""target":"aarch64-apple-darwin""#,
        r#""target":"aarch64-apple-darwin","target":"other""#,
        1,
    );
    assert!(parse_codex_npm_payload_manifest(duplicate_target.as_bytes()).is_err());
}

#[test]
fn rejects_deep_nesting_and_overlong_strings() {
    let mut deep = root("1.2.3");
    deep.pop();
    deep.push_str(r#","ignored":"#);
    deep.push_str(&"[".repeat(17));
    deep.push('0');
    deep.push_str(&"]".repeat(17));
    deep.push('}');
    assert_eq!(
        parse_codex_npm_root_manifest(deep.as_bytes(), HOST_ALIAS),
        Err(CodexNpmManifestError::ExcessiveNesting)
    );

    let long_name = "x".repeat(1025);
    let overlong = root("1.2.3").replacen("@openai/codex", &long_name, 1);
    assert_eq!(
        parse_codex_npm_root_manifest(overlong.as_bytes(), HOST_ALIAS),
        Err(CodexNpmManifestError::OverlongString)
    );
}

#[test]
fn rejects_wrong_types_and_platform_array_shape() {
    let wrong_name = root("1.2.3").replacen(r#""name":"@openai/codex""#, r#""name":7"#, 1);
    assert!(parse_codex_npm_root_manifest(wrong_name.as_bytes(), HOST_ALIAS).is_err());

    for replacement in [
        r#""os":"darwin""#,
        r#""os":[]"#,
        r#""os":["darwin","linux"]"#,
    ] {
        let malformed = platform("1.2.3").replacen(r#""os":["darwin"]"#, replacement, 1);
        assert!(parse_codex_npm_platform_manifest(malformed.as_bytes()).is_err());
    }

    let wrong_layout =
        payload("1.2.3").replacen(r#""layoutVersion":1"#, r#""layoutVersion":"1""#, 1);
    assert!(parse_codex_npm_payload_manifest(wrong_layout.as_bytes()).is_err());

    for required in ["resourcesDir", "pathDir"] {
        let missing = payload("1.2.3").replace(
            &format!(
                r#","{required}":"{}""#,
                if required == "resourcesDir" {
                    "codex-resources"
                } else {
                    "codex-path"
                }
            ),
            "",
        );
        assert!(
            parse_codex_npm_payload_manifest(missing.as_bytes()).is_err(),
            "missing field should reject: {required}"
        );
    }
}

#[test]
fn rejects_malformed_trailing_non_object_and_invalid_utf8() {
    assert!(parse_codex_npm_payload_manifest(b"{").is_err());

    let trailing = format!("{} true", payload("1.2.3"));
    assert!(parse_codex_npm_payload_manifest(trailing.as_bytes()).is_err());
    assert!(parse_codex_npm_payload_manifest(br#"[1,2,3]"#).is_err());

    let mut invalid_utf8 = payload("1.2.3").into_bytes();
    let string_byte = invalid_utf8.iter().position(|byte| *byte == b'a').unwrap();
    invalid_utf8[string_byte] = 0xff;
    assert_eq!(
        parse_codex_npm_payload_manifest(&invalid_utf8),
        Err(CodexNpmManifestError::InvalidUtf8)
    );
}

#[test]
fn requires_a_present_unique_selected_host_alias() {
    let missing = root("1.2.3").replace(HOST_ALIAS, "@openai/codex-darwin-x64-second");
    assert_eq!(
        parse_codex_npm_root_manifest(missing.as_bytes(), HOST_ALIAS),
        Err(CodexNpmManifestError::MissingHostAlias)
    );
    assert_eq!(
        parse_codex_npm_root_manifest(root("1.2.3").as_bytes(), ""),
        Err(CodexNpmManifestError::InvalidHostAlias)
    );
}

#[test]
fn semver_matrix_accepts_strict_prereleases_and_rejects_builds() {
    for valid in [
        "0.0.0",
        "1.2.3",
        "10.20.30",
        "0.147.0-alpha.6.5",
        "1.2.3-0",
        "1.2.3-alpha-1",
    ] {
        assert!(is_codex_npm_semver(valid), "rejected {valid:?}");
        assert!(parse_codex_npm_payload_manifest(payload(valid).as_bytes()).is_ok());
    }
    let sixty_four_bytes = format!("1.1.{}", "1".repeat(60));
    assert_eq!(sixty_four_bytes.len(), 64);
    assert!(parse_codex_npm_payload_manifest(payload(&sixty_four_bytes).as_bytes()).is_ok());

    for invalid in [
        "",
        "1",
        "1.2",
        "1.2.3.4",
        "01.2.3",
        "1.02.3",
        "1.2.03",
        "1.2.x",
        "1.2.3-",
        "1.2.3-alpha..1",
        "1.2.3-alpha.01",
        "1.2.3+build",
        "1.2.3-alpha+build",
    ] {
        assert!(!is_codex_npm_semver(invalid), "accepted {invalid:?}");
        assert_eq!(
            parse_codex_npm_payload_manifest(payload(invalid).as_bytes()),
            Err(CodexNpmManifestError::InvalidVersion),
            "unexpected SemVer result for {invalid:?}"
        );
    }
    let too_long = format!("1.1.{}", "1".repeat(61));
    assert_eq!(
        parse_codex_npm_payload_manifest(payload(&too_long).as_bytes()),
        Err(CodexNpmManifestError::InvalidVersion)
    );
}

#[test]
fn current_parser_source_has_no_filesystem_or_execution_authority() {
    let source = include_str!("codex_npm_manifest.rs");
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
            "manifest parser acquired forbidden authority: {forbidden}"
        );
    }
}
