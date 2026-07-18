use super::*;

fn with_isolated_home<T>(run: impl FnOnce() -> T) -> T {
    let home = tempfile::tempdir().expect("temp home");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());
    let result = run();
    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    result
}

#[test]
fn snapshot_without_live_data_uses_empty_metrics_by_default() {
    let _guard = telemetry::test_guard();
    telemetry::reset_for_tests();
    let previous_demo = std::env::var_os("AI_SWITCHBOARD_DEMO_OPTIMIZATION");
    std::env::remove_var("AI_SWITCHBOARD_DEMO_OPTIMIZATION");

    with_isolated_home(|| {
        let snapshot = build_optimization_snapshot();

        assert!(snapshot.prompt_cache_clients.is_empty());
        assert!(snapshot.routing.is_empty());
        assert!(snapshot.redundancy.is_empty());
        assert_eq!(snapshot.token_xray.original_tokens, 0);
        assert_eq!(snapshot.token_xray.optimized_tokens, 0);
    });

    match previous_demo {
        Some(value) => std::env::set_var("AI_SWITCHBOARD_DEMO_OPTIMIZATION", value),
        None => std::env::remove_var("AI_SWITCHBOARD_DEMO_OPTIMIZATION"),
    }
}

#[test]
fn snapshot_covers_all_requested_feature_groups() {
    let _guard = telemetry::test_guard();
    telemetry::reset_for_tests();
    let previous_demo = std::env::var_os("AI_SWITCHBOARD_DEMO_OPTIMIZATION");
    std::env::set_var("AI_SWITCHBOARD_DEMO_OPTIMIZATION", "1");
    with_isolated_home(|| {
        let snapshot = build_optimization_snapshot();

        assert!(!snapshot.prompt_cache_segments.is_empty());
        assert!(!snapshot.routing.is_empty());
        assert!(snapshot.redundancy.is_empty() || snapshot.redundancy[0].read_count > 1);
        assert!(snapshot.agent_pack.injected);
        assert_eq!(snapshot.rtk_presets.len(), 4);
        assert!(snapshot.prompt_cache_clients.len() <= 1);
        assert!(snapshot.token_xray.original_tokens >= snapshot.token_xray.optimized_tokens);
    });
    match previous_demo {
        Some(value) => std::env::set_var("AI_SWITCHBOARD_DEMO_OPTIMIZATION", value),
        None => std::env::remove_var("AI_SWITCHBOARD_DEMO_OPTIMIZATION"),
    }
}

#[test]
fn snapshot_uses_live_telemetry_when_observed() {
    let _guard = telemetry::test_guard();
    telemetry::reset_for_tests();
    with_isolated_home(|| {
        telemetry::record_prompt_cache_metrics(CacheTokenMetrics {
            prompt_tokens: 100,
            completion_tokens: 20,
            cache_creation_tokens: 40,
            cache_read_tokens: 30,
        });
        telemetry::record_token_xray_bucket("system", 10);
        telemetry::record_token_xray_bucket("tool", 5);
        telemetry::record_redundancy_hash(
            "request-a",
            "abc123abc123abc123abc123abc123abc123abc123abc123abc123abc123abcd",
            12,
        );
        telemetry::record_redundancy_hash(
            "request-b",
            "abc123abc123abc123abc123abc123abc123abc123abc123abc123abc123abcd",
            8,
        );
        telemetry::record_routing_decision(telemetry::RoutingDecisionRecord {
            task: "lint".to_string(),
            current_model: "frontier".to_string(),
            selected_model: "fast/local".to_string(),
            fallback_model: "frontier".to_string(),
            reason: "observed route".to_string(),
            estimated_savings_percent: 42,
        });

        let snapshot = build_optimization_snapshot();

        assert_eq!(snapshot.prompt_cache_segments.len(), 1);
        assert_eq!(snapshot.prompt_cache_segments[0].hit_tokens, 30);
        assert_eq!(snapshot.prompt_cache_clients[0].cache_read_tokens, 30);
        assert!(snapshot
            .token_xray
            .buckets
            .iter()
            .any(|bucket| bucket.id == "system"));
        assert!(snapshot.redundancy[0].read_count >= 2);
        assert!(snapshot.redundancy[0].proof.contains("hash"));
        assert_eq!(snapshot.token_xray.system_tokens, 10);
        assert_eq!(snapshot.token_xray.tool_tokens, 5);
        assert_eq!(snapshot.redundancy[0].duplicate_tokens, 20);
        assert_eq!(snapshot.routing[0].selected_model, "fast/local");
        telemetry::reset_for_tests();
    });
}
