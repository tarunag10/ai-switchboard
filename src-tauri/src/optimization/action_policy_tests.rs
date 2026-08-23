use super::action_policy::*;
use super::compaction::CompactionInput;

#[test]
fn prompt_cache_reorder_is_enabled_by_default() {
    let segments = vec![
        PromptSegmentPlan { id: "user".to_string(), stable: false, cacheable_tokens: 1, original_index: 0 },
        PromptSegmentPlan { id: "system".to_string(), stable: true, cacheable_tokens: 100, original_index: 1 },
    ];
    assert_eq!(plan_prompt_cache_order(&OptimizationActionPolicy::default(), &segments), vec!["system".to_string(), "user".to_string()]);
}

#[test]
fn enabled_prompt_cache_order_keeps_stable_segments_first() {
    let policy = OptimizationActionPolicy { prompt_cache_reorder_enabled: true, ..OptimizationActionPolicy::default() };
    let segments = vec![
        PromptSegmentPlan { id: "user".to_string(), stable: false, cacheable_tokens: 1, original_index: 0 },
        PromptSegmentPlan { id: "repo-pack".to_string(), stable: true, cacheable_tokens: 50, original_index: 1 },
        PromptSegmentPlan { id: "system".to_string(), stable: true, cacheable_tokens: 100, original_index: 2 },
    ];
    assert_eq!(plan_prompt_cache_order(&policy, &segments), vec!["system".to_string(), "repo-pack".to_string(), "user".to_string()]);
}

#[test]
fn compaction_action_is_enabled_by_default() {
    let decision = actionable_compaction_decision(&OptimizationActionPolicy::default(), CompactionInput { context_tokens: 90, context_window_tokens: 100, projected_next_turn_tokens: 5, threshold_percent: 90 });
    assert!(decision.should_compact);
    assert_eq!(decision.reason, "projected_context_crosses_threshold");
}

fn default_route() -> RouteDecision {
    decide_route(
        &RequestFacts { task: "write commit message for staged diff".to_string(), requested_model: "capable".to_string(), cheap_model: "cheap".to_string(), capable_model: "capable".to_string(), value: ActionScoreInput { input_cost_saved: 40, prefill_compute_saved: 20, context_headroom_value: 10, optimization_latency_cost: 5, cache_break_cost: 10, quality_risk: 15 } },
        &ClientFacts { client: "codex".to_string() },
        ContextFacts { context_tokens: 85, context_window_tokens: 100, projected_next_turn_tokens: 5 },
        &CacheFacts { prompt_segments: vec![PromptSegmentPlan { id: "user".to_string(), stable: false, cacheable_tokens: 10, original_index: 0 }, PromptSegmentPlan { id: "system".to_string(), stable: true, cacheable_tokens: 100, original_index: 1 }] },
        &EndpointFacts { endpoint_id: "current-provider".to_string(), model_id: "capable".to_string(), enabled: true },
        &UserPolicy::default(),
    )
}

#[test]
fn unified_facade_reuses_each_policy_and_never_promotes_model_routing() {
    let decision = default_route();
    assert_eq!(decision.selected_endpoint, "current-provider");
    assert_eq!(decision.selected_model, "capable");
    assert_eq!(decision.model_route_observation.selected_model, "cheap");
    assert!(decision.model_route_observation.observe_only);
    assert!(decision.compaction_action.should_compact);
    assert_eq!(decision.prompt_segment_order, vec!["system", "user"]);
    assert!(decision.observe_only);
    assert_eq!(decision.action_score.net_value, 40);
    assert!(decision.action_score.favorable);
}

#[test]
fn unified_facade_honors_all_existing_disable_switches() {
    let decision = decide_route(
        &RequestFacts { task: "lint".to_string(), requested_model: "requested".to_string(), cheap_model: "cheap".to_string(), capable_model: "capable".to_string(), value: ActionScoreInput::default() },
        &ClientFacts { client: "codex".to_string() },
        ContextFacts { context_tokens: 95, context_window_tokens: 100, projected_next_turn_tokens: 5 },
        &CacheFacts { prompt_segments: vec![PromptSegmentPlan { id: "user".to_string(), stable: false, cacheable_tokens: 1, original_index: 0 }, PromptSegmentPlan { id: "system".to_string(), stable: true, cacheable_tokens: 10, original_index: 1 }] },
        &EndpointFacts { endpoint_id: "disabled-endpoint".to_string(), model_id: "different".to_string(), enabled: false },
        &UserPolicy { actions: OptimizationActionPolicy { prompt_cache_reorder_enabled: false, preemptive_compaction_enabled: false, model_routing_enabled: false, max_prompt_reorder_items: 24 }, compaction_threshold_percent: 90 },
    );
    assert_eq!(decision.prompt_segment_order, vec!["user", "system"]);
    assert!(!decision.compaction_action.should_compact);
    assert_eq!(decision.model_route_observation.selected_model, "requested");
    assert!(decision.reasons.contains(&"endpoint_disabled_no_automatic_fallback".to_string()));
    assert_eq!(decision.selected_model, "requested");
}

#[test]
fn policy_score_explains_every_benefit_and_cost_without_ml() {
    let score = score_action(ActionScoreInput { input_cost_saved: 100, prefill_compute_saved: 40, context_headroom_value: 30, optimization_latency_cost: 20, cache_break_cost: 50, quality_risk: 110 });
    assert_eq!(score.net_value, -10);
    assert!(!score.favorable);
    assert!(score.explanation.contains("input_saved=100"));
    assert!(score.explanation.contains("quality_risk=110"));
}

#[test]
fn policy_score_rejects_negative_costs_during_deserialization() {
    let raw = r#"{"inputCostSaved":1,"prefillComputeSaved":1,"contextHeadroomValue":1,"optimizationLatencyCost":-1,"cacheBreakCost":1,"qualityRisk":1}"#;
    assert!(serde_json::from_str::<ActionScoreInput>(raw).is_err());
}
