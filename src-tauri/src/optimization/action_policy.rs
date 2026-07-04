use serde::{Deserialize, Serialize};

use super::compaction::{decide_preemptive_compaction, CompactionDecision, CompactionInput};
use super::model_routing::{decide_model_route, ModelRouteDecision, ModelRouteInput};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationActionPolicy {
    pub(crate) prompt_cache_reorder_enabled: bool,
    pub(crate) preemptive_compaction_enabled: bool,
    pub(crate) model_routing_enabled: bool,
    pub(crate) max_prompt_reorder_items: usize,
}

impl Default for OptimizationActionPolicy {
    fn default() -> Self {
        Self {
            prompt_cache_reorder_enabled: false,
            preemptive_compaction_enabled: false,
            model_routing_enabled: false,
            max_prompt_reorder_items: 24,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptSegmentPlan {
    pub(crate) id: String,
    pub(crate) stable: bool,
    pub(crate) cacheable_tokens: u64,
    pub(crate) original_index: usize,
}

pub(crate) fn plan_prompt_cache_order(
    policy: &OptimizationActionPolicy,
    segments: &[PromptSegmentPlan],
) -> Vec<String> {
    let mut planned = segments.to_vec();
    if policy.prompt_cache_reorder_enabled && planned.len() <= policy.max_prompt_reorder_items {
        planned.sort_by(|left, right| {
            right
                .stable
                .cmp(&left.stable)
                .then(right.cacheable_tokens.cmp(&left.cacheable_tokens))
                .then(left.original_index.cmp(&right.original_index))
        });
    }
    planned.into_iter().map(|segment| segment.id).collect()
}

pub(crate) fn actionable_compaction_decision(
    policy: &OptimizationActionPolicy,
    input: CompactionInput,
) -> CompactionDecision {
    let mut decision = decide_preemptive_compaction(input);
    if !policy.preemptive_compaction_enabled {
        decision.should_compact = false;
        decision.reason = "preemptive_compaction_disabled".to_string();
    }
    decision
}

pub(crate) fn actionable_model_route(
    policy: &OptimizationActionPolicy,
    input: &ModelRouteInput,
) -> ModelRouteDecision {
    let mut gated_input = input.clone();
    gated_input.enabled = policy.model_routing_enabled && input.enabled;
    decide_model_route(&gated_input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_cache_reorder_is_disabled_by_default() {
        let segments = vec![
            PromptSegmentPlan {
                id: "user".to_string(),
                stable: false,
                cacheable_tokens: 1,
                original_index: 0,
            },
            PromptSegmentPlan {
                id: "system".to_string(),
                stable: true,
                cacheable_tokens: 100,
                original_index: 1,
            },
        ];

        assert_eq!(
            plan_prompt_cache_order(&OptimizationActionPolicy::default(), &segments),
            vec!["user".to_string(), "system".to_string()]
        );
    }

    #[test]
    fn enabled_prompt_cache_order_keeps_stable_segments_first() {
        let policy = OptimizationActionPolicy {
            prompt_cache_reorder_enabled: true,
            ..OptimizationActionPolicy::default()
        };
        let segments = vec![
            PromptSegmentPlan {
                id: "user".to_string(),
                stable: false,
                cacheable_tokens: 1,
                original_index: 0,
            },
            PromptSegmentPlan {
                id: "repo-pack".to_string(),
                stable: true,
                cacheable_tokens: 50,
                original_index: 1,
            },
            PromptSegmentPlan {
                id: "system".to_string(),
                stable: true,
                cacheable_tokens: 100,
                original_index: 2,
            },
        ];

        assert_eq!(
            plan_prompt_cache_order(&policy, &segments),
            vec![
                "system".to_string(),
                "repo-pack".to_string(),
                "user".to_string()
            ]
        );
    }

    #[test]
    fn compaction_action_is_disabled_by_default() {
        let decision = actionable_compaction_decision(
            &OptimizationActionPolicy::default(),
            CompactionInput {
                context_tokens: 90,
                context_window_tokens: 100,
                projected_next_turn_tokens: 5,
                threshold_percent: 90,
            },
        );

        assert!(!decision.should_compact);
        assert_eq!(decision.reason, "preemptive_compaction_disabled");
    }
}
