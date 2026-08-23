use super::codex_command_catalog::{
    codex_command_catalog, evaluate_codex_command_snapshot, evaluate_codex_version_probe,
    plan_codex_version_probe, CodexCandidateObservation, CodexCommandSnapshot, CodexProbeOutcome,
    CodexResolvedCandidateKind, CodexVersionProbeObservation,
};

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn absent_snapshot() -> CodexCommandSnapshot {
    CodexCommandSnapshot {
        schema_version: 1,
        observations: codex_command_catalog()
            .iter()
            .map(|entry| CodexCandidateObservation::ConfirmedAbsent {
                candidate_id: entry.candidate_id.into(),
            })
            .collect(),
    }
}

fn selected_snapshot(candidate_id: &str) -> CodexCommandSnapshot {
    let mut snapshot = absent_snapshot();
    let selected = snapshot
        .observations
        .iter_mut()
        .find(|candidate| match candidate {
            CodexCandidateObservation::ConfirmedAbsent { candidate_id: id } => id == candidate_id,
            _ => false,
        })
        .expect("known catalog candidate");
    *selected = CodexCandidateObservation::Present {
        candidate_id: candidate_id.into(),
        resolved_kind: CodexResolvedCandidateKind::RegularFile,
        executable: true,
        identity_digest: Some(digest('a')),
    };
    snapshot
}

fn successful_probe(candidate_id: &str) -> CodexVersionProbeObservation {
    CodexVersionProbeObservation {
        schema_version: 1,
        candidate_id: candidate_id.into(),
        identity_digest_before: digest('a'),
        identity_digest_after: digest('a'),
        outcome: CodexProbeOutcome::Completed {
            exit_success: true,
            output_truncated: false,
            version_output: "codex-cli 0.141.0\n".into(),
        },
    }
}

#[test]
fn catalog_is_fixed_ordered_unique_and_codex_only() {
    let catalog = codex_command_catalog();
    assert_eq!(catalog.len(), 7);
    assert_eq!(catalog[0].candidate_id, "home-local-bin");
    assert_eq!(catalog[0].location_template, "$HOME/.local/bin/codex");
    assert!(catalog
        .iter()
        .any(|entry| entry.location_template == "/opt/homebrew/bin/codex"));
    assert!(catalog
        .iter()
        .any(|entry| entry.candidate_id == "home-npm-global-bin"));
    assert!(catalog
        .iter()
        .all(|entry| entry.location_template.ends_with("/codex")));
    let ids = catalog
        .iter()
        .map(|entry| entry.candidate_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), catalog.len());
}

#[test]
fn incomplete_failure_and_fixed_catalog_absence_remain_distinct() {
    let mut incomplete = absent_snapshot();
    incomplete.observations.pop();
    assert_eq!(
        evaluate_codex_command_snapshot(&incomplete).unwrap().state,
        "incomplete"
    );

    let mut unobserved = absent_snapshot();
    unobserved.observations[0] = CodexCandidateObservation::Unobserved {
        candidate_id: "home-local-bin".into(),
    };
    assert_eq!(
        evaluate_codex_command_snapshot(&unobserved).unwrap().state,
        "incomplete"
    );

    let mut failed = absent_snapshot();
    failed.observations[0] = CodexCandidateObservation::ObservationFailed {
        candidate_id: "home-local-bin".into(),
    };
    assert_eq!(
        evaluate_codex_command_snapshot(&failed).unwrap().state,
        "observation_failed"
    );

    let mut mixed = failed;
    mixed.observations[1] = CodexCandidateObservation::Unobserved {
        candidate_id: "opt-homebrew-bin".into(),
    };
    assert_eq!(
        evaluate_codex_command_snapshot(&mixed).unwrap().state,
        "incomplete"
    );

    let absent = evaluate_codex_command_snapshot(&absent_snapshot()).unwrap();
    assert_eq!(absent.state, "confirmed_absent_from_fixed_catalog");
    assert_eq!(absent.version_state, "not_observed");
    assert!(!absent.runnable);
    assert!(!absent.supported);
    assert!(!absent.process_start_enabled);
}

#[test]
fn schema_alias_unknown_and_duplicate_inputs_fail_closed() {
    let mut schema = absent_snapshot();
    schema.schema_version = 2;
    assert!(evaluate_codex_command_snapshot(&schema).is_err());

    let mut alias = absent_snapshot();
    alias.observations[0] = CodexCandidateObservation::ConfirmedAbsent {
        candidate_id: "codex_cli".into(),
    };
    assert!(evaluate_codex_command_snapshot(&alias).is_err());

    let mut unknown = absent_snapshot();
    unknown.observations[0] = CodexCandidateObservation::ConfirmedAbsent {
        candidate_id: "shell-lookup".into(),
    };
    assert!(evaluate_codex_command_snapshot(&unknown).is_err());

    let mut duplicate = absent_snapshot();
    duplicate
        .observations
        .push(duplicate.observations[0].clone());
    assert!(evaluate_codex_command_snapshot(&duplicate).is_err());
}

#[test]
fn one_regular_candidate_is_present_unprobed_and_probe_plan_is_non_executing() {
    let snapshot = selected_snapshot("home-local-bin");
    let evaluation = evaluate_codex_command_snapshot(&snapshot).unwrap();
    assert_eq!(evaluation.state, "present_unprobed");
    assert_eq!(evaluation.candidate_id.as_deref(), Some("home-local-bin"));
    assert!(!evaluation.runnable);
    assert!(!evaluation.supported);

    let plan = plan_codex_version_probe(&snapshot).unwrap();
    assert_eq!(plan.argument, "--version");
    assert_eq!(plan.stdin_policy, "null");
    assert_eq!(plan.output_policy, "bounded_stdout_discard_stderr");
    assert_eq!(plan.timeout_milliseconds, 2_000);
    assert_eq!(plan.max_output_bytes, 128);
    assert!(!plan.shell_enabled);
    assert!(!plan.working_directory_enabled);
    assert!(!plan.inherited_environment_enabled);
    assert!(!plan.process_start_enabled);
    assert_eq!(plan.provider_traffic, "none");
    assert!(!plan.writes_enabled);
}

#[test]
fn multiple_present_candidates_are_ambiguous_regardless_of_input_order() {
    let mut snapshot = selected_snapshot("home-local-bin");
    snapshot.observations[1] = CodexCandidateObservation::Present {
        candidate_id: "opt-homebrew-bin".into(),
        resolved_kind: CodexResolvedCandidateKind::RegularFile,
        executable: true,
        identity_digest: Some(digest('b')),
    };
    assert_eq!(
        evaluate_codex_command_snapshot(&snapshot).unwrap().state,
        "ambiguous"
    );
    snapshot.observations.reverse();
    assert_eq!(
        evaluate_codex_command_snapshot(&snapshot).unwrap().state,
        "ambiguous"
    );
    assert!(plan_codex_version_probe(&snapshot).is_err());
}

#[test]
fn unsafe_file_kinds_non_executable_and_malformed_identity_are_rejected() {
    for resolved_kind in [
        CodexResolvedCandidateKind::Directory,
        CodexResolvedCandidateKind::SpecialFile,
        CodexResolvedCandidateKind::UnresolvedSymlink,
        CodexResolvedCandidateKind::UnsafeResolution,
    ] {
        let mut snapshot = selected_snapshot("home-local-bin");
        let CodexCandidateObservation::Present {
            resolved_kind: kind,
            ..
        } = &mut snapshot.observations[0]
        else {
            unreachable!()
        };
        *kind = resolved_kind;
        assert_eq!(
            evaluate_codex_command_snapshot(&snapshot).unwrap().state,
            "rejected"
        );
    }

    let mut non_executable = selected_snapshot("home-local-bin");
    let CodexCandidateObservation::Present { executable, .. } = &mut non_executable.observations[0]
    else {
        unreachable!()
    };
    *executable = false;
    assert_eq!(
        evaluate_codex_command_snapshot(&non_executable)
            .unwrap()
            .state,
        "rejected"
    );

    let mut bad_identity = selected_snapshot("home-local-bin");
    let CodexCandidateObservation::Present {
        identity_digest, ..
    } = &mut bad_identity.observations[0]
    else {
        unreachable!()
    };
    *identity_digest = Some("sha256:not-a-digest".into());
    assert!(evaluate_codex_command_snapshot(&bad_identity).is_err());
}

#[test]
fn matching_probe_observes_version_without_claiming_support_or_runnability() {
    let plan = plan_codex_version_probe(&selected_snapshot("home-local-bin")).unwrap();
    let evaluation =
        evaluate_codex_version_probe(&plan, successful_probe("home-local-bin")).unwrap();
    assert_eq!(evaluation.normalized_version, "0.141.0");
    assert_eq!(evaluation.probe_state, "version_observed");
    assert!(evaluation.manual_harness_required);
    assert!(!evaluation.runnable);
    assert!(!evaluation.supported);
    assert!(!evaluation.process_start_enabled);
    assert_eq!(evaluation.provider_traffic, "none");
    assert!(!evaluation.writes_enabled);
}

#[test]
fn schema_candidate_identity_and_plan_drift_are_rejected() {
    let plan = plan_codex_version_probe(&selected_snapshot("home-local-bin")).unwrap();

    let mut schema = successful_probe("home-local-bin");
    schema.schema_version = 2;
    assert!(evaluate_codex_version_probe(&plan, schema).is_err());

    let wrong_candidate = successful_probe("opt-homebrew-bin");
    assert!(evaluate_codex_version_probe(&plan, wrong_candidate).is_err());

    let mut before_drift = successful_probe("home-local-bin");
    before_drift.identity_digest_before = digest('b');
    assert!(evaluate_codex_version_probe(&plan, before_drift).is_err());

    let mut after_drift = successful_probe("home-local-bin");
    after_drift.identity_digest_after = digest('b');
    assert!(evaluate_codex_version_probe(&plan, after_drift).is_err());

    let mut tampered_plan = plan.clone();
    tampered_plan.inherited_environment_enabled = true;
    assert!(
        evaluate_codex_version_probe(&tampered_plan, successful_probe("home-local-bin")).is_err()
    );
}

#[test]
fn timeout_spawn_failure_nonzero_truncation_and_bad_output_are_rejected() {
    let plan = plan_codex_version_probe(&selected_snapshot("home-local-bin")).unwrap();
    let mut cases = vec![
        CodexProbeOutcome::TimedOut,
        CodexProbeOutcome::SpawnFailed,
        CodexProbeOutcome::Completed {
            exit_success: false,
            output_truncated: false,
            version_output: String::new(),
        },
        CodexProbeOutcome::Completed {
            exit_success: true,
            output_truncated: true,
            version_output: "codex-cli 1.2.3".into(),
        },
    ];
    for version_output in [
        "other-cli 1.2.3".into(),
        "codex-cli 01.2.3".into(),
        "codex-cli 1.2".into(),
        "codex-cli 1.2.3\nsecret".into(),
        format!("codex-cli 1.2.3+{}", "a".repeat(128)),
    ] {
        cases.push(CodexProbeOutcome::Completed {
            exit_success: true,
            output_truncated: false,
            version_output,
        });
    }
    for outcome in cases {
        let mut observation = successful_probe("home-local-bin");
        observation.outcome = outcome;
        assert!(evaluate_codex_version_probe(&plan, observation).is_err());
    }
}

#[test]
fn source_boundary_contains_no_collector_executor_network_or_renderer_command() {
    let source = include_str!("codex_command_catalog.rs");
    let compact = source
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect::<String>();
    let forbidden = [
        ["std", "process"].join("::"),
        ["tokio", "process"].join("::"),
        ["async", "process"].join("_"),
        ["sub", "process"].concat(),
        "duct::".into(),
        ["Command", "new"].join("::"),
        [".", "spawn("].concat(),
        [".", "output("].concat(),
        [".", "status("].concat(),
        ["std", "env"].join("::"),
        ["dirs", "home_dir"].join("::"),
        "home_dir(".into(),
        ["std", "fs"].join("::"),
        ["tokio", "fs"].join("::"),
        ["std", "net"].join("::"),
        ["tokio", "net"].join("::"),
        ["req", "west"].concat(),
        "ureq".into(),
        "hyper::".into(),
        "TcpStream".into(),
        "UdpSocket".into(),
        "curl".into(),
        ["P", "A", "T", "H"].concat(),
        ["tauri", "command"].join("::"),
        "Serialize".into(),
    ];
    for token in forbidden {
        assert!(!compact.contains(&token), "forbidden source token: {token}");
    }
}
