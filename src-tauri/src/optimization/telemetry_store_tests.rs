use tempfile::tempdir;

use super::cache_metrics::CacheTokenMetrics;
use super::model_routing::{ModelRoutingEvidenceArm, ModelRoutingEvidenceObservation};
use super::telemetry::{RedundancyHashRecord, RoutingDecisionRecord, RtkPresetMetadata};
use super::telemetry_store::*;

#[test]
fn prompt_cache_metrics_round_trip_through_sqlite() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    let first = CacheTokenMetrics {
        prompt_tokens: 100,
        completion_tokens: 20,
        cache_read_tokens: 40,
        cache_creation_tokens: 10,
    };
    let second = CacheTokenMetrics {
        prompt_tokens: 50,
        completion_tokens: 10,
        cache_read_tokens: 25,
        cache_creation_tokens: 5,
    };

    record_prompt_cache_metrics(&first);
    record_prompt_cache_metrics(&second);

    assert_eq!(
        prompt_cache_totals(),
        CacheTokenMetrics {
            prompt_tokens: 150,
            completion_tokens: 30,
            cache_read_tokens: 65,
            cache_creation_tokens: 15,
        }
    );
    let evidence = prompt_cache_totals_evidence_result()
        .expect("cache evidence query")
        .expect("recorded rows have evidence");
    assert_eq!(evidence.metrics.cache_read_tokens, 65);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn absent_cache_rows_are_unavailable_evidence_not_measured_zero() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    assert!(prompt_cache_totals_evidence_result()
        .expect("empty cache evidence query")
        .is_none());

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}
#[test]
fn routing_decisions_round_trip_through_sqlite() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    reset_for_tests();
    record_routing_decision(&RoutingDecisionRecord {
        task: "commit message".to_string(),
        current_model: "gpt-5".to_string(),
        selected_model: "gpt-5-mini".to_string(),
        fallback_model: "gpt-5".to_string(),
        reason: "trivial task".to_string(),
        estimated_savings_percent: 42,
    });

    let decisions = recent_routing_decisions(8);
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].selected_model, "gpt-5-mini");
    assert_eq!(decisions[0].estimated_savings_percent, 42);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn model_routing_evidence_round_trips_and_exports_observe_only_artifact() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    reset_for_tests();
    for arm in [ModelRoutingEvidenceArm::Baseline, ModelRoutingEvidenceArm::Candidate] {
        record_model_routing_evidence(&ModelRoutingEvidenceObservation {
            run_id: "run-1".to_string(),
            captured_at: chrono::Utc::now().to_rfc3339(),
            task_class: " Formatting ".to_string(),
            arm,
            baseline_model: "frontier".to_string(),
            candidate_model: "fast/local".to_string(),
            succeeded: true,
            successful_task_cost_microunits: Some(if arm == ModelRoutingEvidenceArm::Baseline { 1000 } else { 700 }),
            quality_score_bps: 9800,
            latency_ms: if arm == ModelRoutingEvidenceArm::Baseline { 800 } else { 820 },
            follow_up_rework: false,
        });
    }

    let artifact = export_model_routing_evidence("run-1", "formatting")
        .expect("exported evidence");
    assert_eq!(artifact.evidence_class, "local_runtime_observation");
    assert_eq!(artifact.baseline.sample_count, 1);
    assert_eq!(artifact.baseline.successful_task_count, 1);
    assert_eq!(artifact.candidate.successful_task_count, 1);
    assert_eq!(artifact.candidate.successful_task_cost_micros, 700);
    assert!(!artifact.promotion_eligible);
    assert_eq!(artifact.provenance.task_class, "formatting");
    assert_eq!(artifact.provenance.cost_attribution, "local_estimate");
    assert_eq!(artifact.provenance.provider_id, None);
    let serialized = serde_json::to_value(&artifact).expect("serialized evidence");
    assert_eq!(serialized["evidenceClass"], "local_runtime_observation");
    assert_eq!(serialized["promotionEligible"], false);
    assert!(serialized["baseline"]["successfulTaskCostMicros"].is_number());

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn model_routing_evidence_exports_per_arm_success_rates() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    reset_for_tests();
    for (arm, outcomes) in [
        (ModelRoutingEvidenceArm::Baseline, [true, false]),
        (ModelRoutingEvidenceArm::Candidate, [true, true]),
    ] {
        for (index, succeeded) in outcomes.into_iter().enumerate() {
            record_model_routing_evidence(&ModelRoutingEvidenceObservation {
                run_id: "run-2".to_string(),
                captured_at: (chrono::Utc::now() + chrono::Duration::milliseconds(index as i64)).to_rfc3339(),
                task_class: "formatting".to_string(),
                arm,
                baseline_model: "frontier".to_string(),
                candidate_model: "fast/local".to_string(),
                succeeded,
                successful_task_cost_microunits: succeeded.then_some(if arm == ModelRoutingEvidenceArm::Baseline { 1000 } else { 700 }),
                quality_score_bps: 9800,
                latency_ms: 800,
                follow_up_rework: false,
            })
            .expect("record routing evidence");
        }
    }

    let artifact = export_model_routing_evidence("run-2", "formatting")
        .expect("exported evidence");
    assert_eq!(artifact.baseline.sample_count, 2);
    assert_eq!(artifact.baseline.successful_task_count, 1);
    assert_eq!(artifact.baseline.success_rate_bps, 5_000);
    assert_eq!(artifact.candidate.sample_count, 2);
    assert_eq!(artifact.candidate.successful_task_count, 2);
    assert_eq!(artifact.candidate.success_rate_bps, 10_000);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn model_routing_evidence_export_uses_latest_timestamp_instant() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    reset_for_tests();
    let day = chrono::Utc::now().date_naive();
    let base = day
        .and_hms_opt(12, 0, 0)
        .expect("valid noon")
        .and_utc()
        - chrono::Duration::days(1);
    let earlier = base
        .with_timezone(&chrono::FixedOffset::east_opt(14 * 60 * 60).expect("valid offset"))
        .to_rfc3339();
    let later = (base + chrono::Duration::hours(1))
        .with_timezone(&chrono::FixedOffset::west_opt(12 * 60 * 60).expect("valid offset"))
        .to_rfc3339();
    for (arm, captured_at) in [
        (ModelRoutingEvidenceArm::Baseline, earlier),
        (ModelRoutingEvidenceArm::Candidate, later.clone()),
    ] {
        record_model_routing_evidence(&ModelRoutingEvidenceObservation {
            run_id: "run-offsets".to_string(),
            captured_at,
            task_class: "formatting".to_string(),
            arm,
            baseline_model: "frontier".to_string(),
            candidate_model: "fast/local".to_string(),
            succeeded: true,
            successful_task_cost_microunits: Some(700),
            quality_score_bps: 9800,
            latency_ms: 800,
            follow_up_rework: false,
        })
        .expect("record offset timestamp");
    }

    let artifact = export_model_routing_evidence("run-offsets", "formatting")
        .expect("export offset evidence");
    assert_eq!(artifact.provenance.captured_at, later);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn model_routing_evidence_rejects_overflow_and_invalid_timestamps() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());
    reset_for_tests();

    let mut observation = ModelRoutingEvidenceObservation {
        run_id: "run-1".to_string(),
        captured_at: chrono::Utc::now().to_rfc3339(),
        task_class: "formatting".to_string(),
        arm: ModelRoutingEvidenceArm::Baseline,
        baseline_model: "frontier".to_string(),
        candidate_model: "fast/local".to_string(),
        succeeded: true,
        successful_task_cost_microunits: Some(i64::MAX as u64 + 1),
        quality_score_bps: 9000,
        latency_ms: 10,
        follow_up_rework: false,
    };
    assert!(record_model_routing_evidence(&observation).is_err());
    observation.successful_task_cost_microunits = Some(10);
    observation.captured_at = "not-a-timestamp".to_string();
    assert!(record_model_routing_evidence(&observation).is_err());
    observation.captured_at = (chrono::Utc::now() - chrono::Duration::days(8)).to_rfc3339();
    assert!(record_model_routing_evidence(&observation).is_err());
    observation.captured_at = chrono::Utc::now().to_rfc3339();
    observation.successful_task_cost_microunits = None;
    assert!(record_model_routing_evidence(&observation)
        .expect_err("successful observations without cost must fail closed")
        .to_string()
        .contains("require cost evidence"));

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn model_routing_evidence_rejects_exact_duplicate_callbacks() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());
    reset_for_tests();

    let observation = ModelRoutingEvidenceObservation {
        run_id: "run-duplicate".to_string(),
        captured_at: chrono::Utc::now().to_rfc3339(),
        task_class: "formatting".to_string(),
        arm: ModelRoutingEvidenceArm::Baseline,
        baseline_model: "frontier".to_string(),
        candidate_model: "fast/local".to_string(),
        succeeded: true,
        successful_task_cost_microunits: Some(1000),
        quality_score_bps: 9800,
        latency_ms: 800,
        follow_up_rework: false,
    };
    record_model_routing_evidence(&observation).expect("first callback should persist");
    let error = record_model_routing_evidence(&observation)
        .expect_err("exact duplicate callback must fail closed");
    assert!(error.contains("duplicate"));

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}
#[test]
fn token_xray_buckets_round_trip_through_sqlite() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    reset_for_tests();
    record_token_xray_bucket("tool", 12);
    record_token_xray_bucket("tool", 8);
    record_token_xray_bucket("history", 5);

    let buckets = token_xray_bucket_totals();
    assert_eq!(buckets.len(), 2);
    assert!(buckets
        .iter()
        .any(|bucket| bucket.bucket == "tool" && bucket.tokens == 20));
    assert!(buckets
        .iter()
        .any(|bucket| bucket.bucket == "history" && bucket.tokens == 5));

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}
#[test]
fn redundancy_hashes_round_trip_through_sqlite() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    reset_for_tests();
    record_redundancy_hash(&RedundancyHashRecord {
        source_id: "AGENTS.md".to_string(),
        content_sha256: "abc123".repeat(11),
        estimated_tokens: 12,
    });

    let records = recent_redundancy_hashes(8);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].source_id, "AGENTS.md");
    assert_eq!(records[0].estimated_tokens, 12);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn rtk_preset_metadata_round_trip_through_sqlite() {
    let _guard = crate::optimization::telemetry::test_guard();
    let home = tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());

    reset_for_tests();
    record_rtk_preset_metadata(&RtkPresetMetadata {
        id: "pytest".to_string(),
        label: "pytest".to_string(),
        command: "rtk pytest".to_string(),
        focus: "failure-only test output".to_string(),
    });

    let metadata = recent_rtk_preset_metadata(8);
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].id, "pytest");
    assert_eq!(metadata[0].command, "rtk pytest");

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}
