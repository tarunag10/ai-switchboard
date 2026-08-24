use super::codex_probe_preflight::{
    CodexLauncherChainKind, CodexMachOArchitecture, CodexMachOFileType,
};
use super::codex_probe_preflight_test_support::{
    containment, digest, direct_target, evaluate, evaluate_npm, npm_receipt,
    npm_receipt_without_signature, process_spec,
};
use super::process_run_spec::process_run_spec_for;

#[test]
fn complete_synthetic_direct_evidence_remains_non_executing() {
    let result =
        evaluate(&process_spec(), &direct_target(), &containment()).expect("direct preflight");
    assert_eq!(
        result.state,
        "supplied_evidence_shape_complete_non_executing"
    );
    assert_eq!(result.schema_version, 2);
    assert_eq!(
        result.reason_code,
        "native_collection_and_manual_harness_still_required"
    );
    assert!(result.manual_opt_in_required);
    assert_eq!(result.attempt_id, containment().attempt_id);
    assert!(!result.runnable);
    assert!(!result.supported);
    assert!(!result.process_start_enabled);
    assert_eq!(result.provider_traffic, "none");
    assert!(!result.user_workspace_writes_enabled);
    for value in [
        &result.process_run_spec_digest,
        &result.probe_plan_digest,
        &result.launcher_chain_identity_digest,
        &result.containment_identity_digest,
        &result.preflight_identity_digest,
    ] {
        assert!(value.starts_with("sha256:"));
    }
    assert!(!format!("{result:?}").contains('/'));
}

#[test]
fn process_spec_probe_plan_and_workspace_are_bound() {
    let first = evaluate(&process_spec(), &direct_target(), &containment()).expect("first");
    let changed_spec = process_run_spec_for(
        "workbench:probe-test",
        "codex-1234567890ab",
        "codex",
        &digest('x'),
    )
    .expect("changed process spec");
    let second = evaluate(&changed_spec, &direct_target(), &containment()).expect("second");
    assert_ne!(
        first.process_run_spec_digest,
        second.process_run_spec_digest
    );
    assert_ne!(
        first.preflight_identity_digest,
        second.preflight_identity_digest
    );
    let claude_spec = process_run_spec_for(
        "workbench:probe-test",
        "claude_code-1234567890",
        "claude_code",
        &digest('x'),
    )
    .expect("Claude process spec");
    assert!(evaluate(&claude_spec, &direct_target(), &containment()).is_err());
    let mut tampered = process_spec();
    tampered.start_authorization = "granted".into();
    assert!(evaluate(&tampered, &direct_target(), &containment()).is_err());
}

#[test]
fn npm_chain_binds_alias_manifests_versions_layout_payload_and_roots() {
    let spec = process_spec();
    let receipt = npm_receipt();
    let first = evaluate_npm(&spec, &receipt, &containment()).expect("npm preflight");
    assert_eq!(first.state, "collected_target_shape_complete_non_executing");
    assert_eq!(
        first.reason_code,
        "restricted_helper_and_manual_harness_still_required"
    );
    let mut changed = containment();
    changed.disposable_roots_identity_digest = digest('z');
    let second = evaluate_npm(&spec, &receipt, &changed).expect("changed roots");
    assert_ne!(
        first.preflight_identity_digest,
        second.preflight_identity_digest
    );
    assert_ne!(
        first.launcher_chain_identity_digest,
        evaluate(&spec, &direct_target(), &containment())
            .expect("direct preflight")
            .launcher_chain_identity_digest
    );
    for case in ["attempt", "host", "boot"] {
        let mut changed = containment();
        match case {
            "attempt" => changed.attempt_id = "codex-probe:attempt-5678".into(),
            "host" => changed.host_instance_identity_digest = digest('u'),
            "boot" => changed.boot_session_identity_digest = digest('v'),
            _ => unreachable!(),
        }
        let rebound = evaluate_npm(&spec, &receipt, &changed).expect("rebound identity");
        assert_ne!(
            first.containment_identity_digest,
            rebound.containment_identity_digest
        );
        assert_ne!(
            first.preflight_identity_digest,
            rebound.preflight_identity_digest
        );
    }
    let unsigned = evaluate_npm(&spec, &npm_receipt_without_signature(), &containment())
        .expect("signature absence remains an observation, not a support decision");
    assert_ne!(
        first.launcher_chain_identity_digest,
        unsigned.launcher_chain_identity_digest
    );
    assert!(!unsigned.runnable);
    assert!(!unsigned.supported);
}

#[test]
fn fixed_candidate_and_launcher_binding_fail_closed() {
    for case in ["schema", "candidate", "launcher"] {
        let mut target = direct_target();
        match case {
            "schema" => target.schema_version += 1,
            "candidate" => target.candidate_id = "unknown".into(),
            "launcher" => target.launcher_identity_digest = digest('q'),
            _ => unreachable!(),
        }
        assert!(evaluate(&process_spec(), &target, &containment()).is_err());
    }
}

#[test]
fn macho_shape_architecture_and_identity_fail_closed() {
    for case in [
        "arch",
        "regular",
        "executable",
        "class",
        "dylib",
        "load_commands",
        "signature_blob",
        "interpreter",
        "path_lookup",
    ] {
        let mut target = direct_target();
        match case {
            "arch" => target.target_architecture = CodexMachOArchitecture::X86_64,
            "regular" => target.target_is_regular_file = false,
            "executable" => target.target_is_executable = false,
            "class" => target.macho_class_64 = false,
            "dylib" => target.macho_file_type = CodexMachOFileType::DynamicLibrary,
            "load_commands" => target.macho_load_commands_identity_digest = "bad".into(),
            "signature_blob" => target.code_signature_blob_identity_digest = Some("bad".into()),
            "interpreter" => target.interpreter_launcher_selected_for_execution = true,
            "path_lookup" => target.path_lookup_used = true,
            _ => unreachable!(),
        }
        assert!(evaluate(&process_spec(), &target, &containment()).is_err());
    }
    let mut other = direct_target();
    other.macho_file_type = CodexMachOFileType::Other;
    assert!(evaluate(&process_spec(), &other, &containment()).is_err());
}

#[test]
fn direct_and_npm_chain_specific_rules_fail_closed() {
    let mut direct = direct_target();
    direct.target_identity_digest = digest('6');
    assert!(evaluate(&process_spec(), &direct, &containment()).is_err());
    let mut direct_with_npm = direct_target();
    direct_with_npm.npm_collection_identity_digest = Some(digest('4'));
    assert!(evaluate(&process_spec(), &direct_with_npm, &containment()).is_err());
    let mut raw_npm = direct_target();
    raw_npm.chain_kind = CodexLauncherChainKind::SuppliedNpmPlatformPackageV1;
    assert!(evaluate(&process_spec(), &raw_npm, &containment()).is_err());

    for case in [
        "schema",
        "state",
        "launcher",
        "symlink",
        "root_manifest",
        "alias",
        "dependency_spec",
        "platform_manifest",
        "root_version",
        "platform_version",
        "payload_manifest",
        "target_triple",
        "layout",
        "payload",
        "macho_architecture",
        "macho_file_type",
        "load_commands",
        "signature",
        "derivation",
        "collection",
    ] {
        let mut npm = npm_receipt();
        match case {
            "schema" => npm.schema_version += 1,
            "state" => npm.state = "forged".into(),
            "launcher" => npm.launcher_identity_digest = digest('a'),
            "symlink" => npm.launcher_symlink_identity_digest = digest('b'),
            "root_manifest" => npm.root_manifest_identity_digest = digest('c'),
            "alias" => npm.dependency_alias = "@openai/codex-darwin-x64".into(),
            "dependency_spec" => npm.dependency_version_spec = "latest".into(),
            "platform_manifest" => npm.platform_manifest_identity_digest = digest('e'),
            "root_version" => npm.root_version = "invalid version".into(),
            "platform_version" => npm.platform_version = "1.2.3".into(),
            "payload_manifest" => npm.payload_manifest_identity_digest = digest('f'),
            "target_triple" => npm.payload_target = "x86_64-apple-darwin".into(),
            "layout" => npm.payload_layout_version += 1,
            "payload" => npm.payload_file_identity_digest = digest('g'),
            "macho_architecture" => npm.payload_macho_architecture = CodexMachOArchitecture::X86_64,
            "macho_file_type" => npm.payload_macho_file_type = CodexMachOFileType::DynamicLibrary,
            "load_commands" => npm.payload_macho_load_commands_identity_digest = digest('i'),
            "signature" => npm.payload_code_signature_blob_identity_digest = None,
            "derivation" => npm.derivation_identity_digest = digest('j'),
            "collection" => npm.collection_identity_digest = digest('k'),
            _ => unreachable!(),
        }
        assert!(evaluate_npm(&process_spec(), &npm, &containment()).is_err());
    }
    for version in [
        ".", "-", "+", "01.2.3", "1.02.3", "1.2.03", "1.2", "1.2.3-01",
    ] {
        let mut npm = npm_receipt();
        npm.root_version = version.into();
        npm.platform_version = format!("{version}-darwin-arm64");
        assert!(evaluate_npm(&process_spec(), &npm, &containment()).is_err());
    }
}

#[test]
fn restricted_helper_network_filesystem_and_identity_evidence_are_mandatory() {
    for case in [
        "helper_flag",
        "helper_identity",
        "entitlements_flag",
        "entitlements_identity",
        "sandbox",
        "fallback",
        "network",
        "writes",
        "reads",
        "cwd",
    ] {
        let mut value = containment();
        match case {
            "helper_flag" => value.helper_is_distinct_restricted_identity = false,
            "helper_identity" => {
                value.helper_code_identity_digest = value.app_code_identity_digest.clone()
            }
            "entitlements_flag" => value.helper_entitlements_are_narrower_than_app = false,
            "entitlements_identity" => {
                value.helper_entitlements_identity_digest =
                    value.app_entitlements_identity_digest.clone()
            }
            "sandbox" => value.sandbox_enforced = false,
            "fallback" => value.sandbox_failure_is_fatal = false,
            "network" => value.network_denied = false,
            "writes" => value.writes_denied_outside_disposable_roots = false,
            "reads" => value.read_scope_is_allowlisted = false,
            "cwd" => value.working_directory_is_disposable = false,
            _ => unreachable!(),
        }
        assert!(evaluate(&process_spec(), &direct_target(), &value).is_err());
    }
}

#[test]
fn environment_shell_path_and_credentials_are_never_inherited() {
    for case in ["environment", "values", "credentials", "shell", "path"] {
        let mut value = containment();
        match case {
            "environment" => value.environment_cleared = false,
            "values" => value.environment_values_are_disposable = false,
            "credentials" => value.provider_credentials_inherited = true,
            "shell" => value.shell_enabled = true,
            "path" => value.path_lookup_enabled = true,
            _ => unreachable!(),
        }
        assert!(evaluate(&process_spec(), &direct_target(), &value).is_err());
    }
}

#[test]
fn io_timeout_process_group_kill_and_reap_policy_is_exact() {
    for case in [
        "group", "stdin", "stderr", "drain", "output", "timeout", "grace", "kill", "reap",
    ] {
        let mut value = containment();
        match case {
            "group" => value.process_group_owned = false,
            "stdin" => value.stdin_null = false,
            "stderr" => value.stderr_discarded = false,
            "drain" => value.stdout_fully_drained = false,
            "output" => value.max_output_bytes += 1,
            "timeout" => value.timeout_milliseconds += 1,
            "grace" => value.term_grace_milliseconds += 1,
            "kill" => value.kill_after_grace = false,
            "reap" => value.descendants_reaped = false,
            _ => unreachable!(),
        }
        assert!(evaluate(&process_spec(), &direct_target(), &value).is_err());
    }
}

#[test]
fn containment_schema_profile_root_count_and_every_evidence_digest_are_strict() {
    for case in [
        "schema",
        "profile",
        "roots",
        "attempt",
        "host",
        "boot",
        "os",
        "app",
        "app_entitlements",
        "helper",
        "entitlements",
        "policy",
        "network",
        "filesystem",
        "process",
        "timeout",
        "root_digest",
    ] {
        let mut value = containment();
        match case {
            "schema" => value.schema_version += 1,
            "profile" => value.profile_id = "unreviewed".into(),
            "roots" => value.disposable_root_count += 1,
            "attempt" => value.attempt_id = "not an identifier".into(),
            "host" => value.host_instance_identity_digest = "bad".into(),
            "boot" => value.boot_session_identity_digest = "bad".into(),
            "os" => value.os_build_identity_digest = "bad".into(),
            "app" => value.app_code_identity_digest = "bad".into(),
            "app_entitlements" => value.app_entitlements_identity_digest = "bad".into(),
            "helper" => value.helper_code_identity_digest = "bad".into(),
            "entitlements" => value.helper_entitlements_identity_digest = "bad".into(),
            "policy" => value.enforcement_policy_identity_digest = "bad".into(),
            "network" => value.network_deny_evidence_digest = "bad".into(),
            "filesystem" => value.filesystem_scope_evidence_digest = "bad".into(),
            "process" => value.process_group_evidence_digest = "bad".into(),
            "timeout" => value.timeout_evidence_digest = "bad".into(),
            "root_digest" => value.disposable_roots_identity_digest = "bad".into(),
            _ => unreachable!(),
        }
        assert!(evaluate(&process_spec(), &direct_target(), &value).is_err());
    }
}

#[test]
fn preflight_source_smoke_check_rejects_known_authority_apis() {
    let source = concat!(
        include_str!("codex_probe_preflight.rs"),
        include_str!("codex_probe_preflight_digest.rs"),
        include_str!("codex_probe_semver.rs")
    )
    .chars()
    .filter(|character| !character.is_whitespace())
    .flat_map(char::to_lowercase)
    .collect::<String>();
    for forbidden in [
        "std::process",
        "tokio::process",
        "command::new",
        ".spawn(",
        "std::fs",
        "tokio::fs",
        "fs::read",
        "file::open",
        "openoptions",
        "std::env",
        "tauri::",
        "reqwest",
        "std::net",
        "tokio::net",
        "tcpstream",
        "unixstream",
        "libc",
        "nix::",
        "extern\"c\"",
        "unsafe{",
    ] {
        assert!(
            !source.contains(forbidden),
            "unexpected authority: {forbidden}"
        );
    }
}
