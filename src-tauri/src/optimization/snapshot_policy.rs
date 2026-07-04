use super::action_policy::{
    actionable_model_route, load_action_policy, plan_prompt_cache_order, PromptSegmentPlan,
};
use super::model_routing::ModelRouteInput;
use super::snapshot_types::{ModelRoutingSnapshot, PromptCacheSegmentSnapshot};

pub(super) fn order_prompt_cache_segments(
    segments: Vec<PromptCacheSegmentSnapshot>,
) -> Vec<PromptCacheSegmentSnapshot> {
    let policy = load_action_policy();
    let plans: Vec<_> = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| PromptSegmentPlan {
            id: segment.id.clone(),
            stable: segment.changes_per_session <= 1
                || segment.id.contains("system")
                || segment.id.contains("repo"),
            cacheable_tokens: segment.cacheable_tokens,
            original_index: index,
        })
        .collect();
    let ordered_ids = plan_prompt_cache_order(&policy, &plans);
    ordered_ids
        .into_iter()
        .filter_map(|id| segments.iter().find(|segment| segment.id == id).cloned())
        .collect()
}

fn route_with_policy(input: ModelRouteInput) -> super::model_routing::ModelRouteDecision {
    actionable_model_route(&load_action_policy(), &input)
}

pub(super) fn route_to_snapshot(input: ModelRouteInput) -> ModelRoutingSnapshot {
    let current_model = input.requested_model.clone();
    let fallback_model = input.capable_model.clone();
    let task = input.task.clone();
    let decision = route_with_policy(input);
    ModelRoutingSnapshot {
        task,
        current_model,
        selected_model: decision.selected_model,
        fallback_model,
        reason: decision.reason,
        estimated_savings_percent: if decision.observe_only { 0 } else { 35 },
    }
}
