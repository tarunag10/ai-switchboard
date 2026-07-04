use tempfile::tempdir;

use super::telemetry::{RedundancyHashRecord, RoutingDecisionRecord};
use super::telemetry_store::*;
use super::CacheTokenMetrics;

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
