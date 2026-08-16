use serde::{Deserialize, Serialize};

use crate::storage::{app_data_dir, config_file};

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
            prompt_cache_reorder_enabled: true,
            preemptive_compaction_enabled: true,
            model_routing_enabled: true,
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

/// Content-free request characteristics consumed by the unified policy facade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestFacts {
    pub(crate) task: String,
    pub(crate) requested_model: String,
    pub(crate) cheap_model: String,
    pub(crate) capable_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientFacts {
    pub(crate) client: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextFacts {
    pub(crate) context_tokens: u64,
    pub(crate) context_window_tokens: u64,
    pub(crate) projected_next_turn_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheFacts {
    pub(crate) prompt_segments: Vec<PromptSegmentPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointFacts {
    pub(crate) endpoint_id: String,
    pub(crate) model_id: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserPolicy {
    pub(crate) actions: OptimizationActionPolicy,
    pub(crate) compaction_threshold_percent: u8,
}

impl Default for UserPolicy {
    fn default() -> Self {
        Self {
            actions: OptimizationActionPolicy::default(),
            compaction_threshold_percent: 90,
        }
    }
}

/// One conservative view over the existing independent policy decisions.
///
/// `selected_model` is always the user's requested model in Phase 1. The
/// existing model-routing heuristic is preserved as `model_route_observation`
/// so benchmark collection cannot silently promote it into live routing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteDecision {
    pub(crate) selected_endpoint: String,
    pub(crate) selected_model: String,
    pub(crate) prompt_segment_order: Vec<String>,
    pub(crate) compaction_action: CompactionDecision,
    pub(crate) model_route_observation: ModelRouteDecision,
    pub(crate) reasons: Vec<String>,
    pub(crate) observe_only: bool,
}

pub(crate) fn decide_route(
    request: &RequestFacts,
    client: &ClientFacts,
    context: ContextFacts,
    cache: &CacheFacts,
    endpoint: &EndpointFacts,
    policy: &UserPolicy,
) -> RouteDecision {
    let prompt_segment_order = plan_prompt_cache_order(&policy.actions, &cache.prompt_segments);
    let compaction_action = actionable_compaction_decision(
        &policy.actions,
        CompactionInput {
            context_tokens: context.context_tokens,
            context_window_tokens: context.context_window_tokens,
            projected_next_turn_tokens: context.projected_next_turn_tokens,
            threshold_percent: policy.compaction_threshold_percent,
        },
    );
    let model_route_observation = actionable_model_route(
        &policy.actions,
        &ModelRouteInput {
            client: client.client.clone(),
            task: request.task.clone(),
            requested_model: request.requested_model.clone(),
            cheap_model: request.cheap_model.clone(),
            capable_model: request.capable_model.clone(),
            enabled: endpoint.enabled,
        },
    );
    let mut reasons = vec![
        compaction_action.reason.clone(),
        model_route_observation.reason.clone(),
    ];
    if !endpoint.enabled {
        reasons.push("endpoint_disabled_no_automatic_fallback".to_string());
    }
    if endpoint.model_id != request.requested_model {
        reasons.push("configured_endpoint_model_not_promoted".to_string());
    }

    RouteDecision {
        selected_endpoint: endpoint.endpoint_id.clone(),
        selected_model: request.requested_model.clone(),
        prompt_segment_order,
        compaction_action,
        model_route_observation,
        reasons,
        observe_only: true,
    }
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

pub(crate) fn load_action_policy() -> OptimizationActionPolicy {
    let path = config_file(&app_data_dir(), "optimization-action-policy.json");
    let Ok(raw) = std::fs::read(&path) else {
        return OptimizationActionPolicy::default();
    };
    serde_json::from_slice(&raw).unwrap_or_default()
}

pub(crate) fn save_action_policy(
    policy: &OptimizationActionPolicy,
) -> Result<OptimizationActionPolicy, String> {
    let path = config_file(&app_data_dir(), "optimization-action-policy.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(policy).map_err(|err| err.to_string())?;
    std::fs::write(&path, bytes).map_err(|err| format!("write {}: {err}", path.display()))?;
    Ok(policy.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_cache_reorder_is_enabled_by_default() {
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
            vec!["system".to_string(), "user".to_string()]
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
    fn compaction_action_is_enabled_by_default() {
        let decision = actionable_compaction_decision(
            &OptimizationActionPolicy::default(),
            CompactionInput {
                context_tokens: 90,
                context_window_tokens: 100,
                projected_next_turn_tokens: 5,
                threshold_percent: 90,
            },
        );

        assert!(decision.should_compact);
        assert_eq!(decision.reason, "projected_context_crosses_threshold");
    }

    #[test]
    fn unified_facade_reuses_each_policy_and_never_promotes_model_routing() {
        let request = RequestFacts {
            task: "write commit message for staged diff".to_string(),
            requested_model: "capable".to_string(),
            cheap_model: "cheap".to_string(),
            capable_model: "capable".to_string(),
        };
        let decision = decide_route(
            &request,
            &ClientFacts {
                client: "codex".to_string(),
            },
            ContextFacts {
                context_tokens: 85,
                context_window_tokens: 100,
                projected_next_turn_tokens: 5,
            },
            &CacheFacts {
                prompt_segments: vec![
                    PromptSegmentPlan {
                        id: "user".to_string(),
                        stable: false,
                        cacheable_tokens: 10,
                        original_index: 0,
                    },
                    PromptSegmentPlan {
                        id: "system".to_string(),
                        stable: true,
                        cacheable_tokens: 100,
                        original_index: 1,
                    },
                ],
            },
            &EndpointFacts {
                endpoint_id: "current-provider".to_string(),
                model_id: "capable".to_string(),
                enabled: true,
            },
            &UserPolicy::default(),
        );

        assert_eq!(decision.selected_endpoint, "current-provider");
        assert_eq!(decision.selected_model, "capable");
        assert_eq!(decision.model_route_observation.selected_model, "cheap");
        assert!(decision.model_route_observation.observe_only);
        assert!(decision.compaction_action.should_compact);
        assert_eq!(decision.prompt_segment_order, vec!["system", "user"]);
        assert!(decision.observe_only);
    }

    #[test]
    fn unified_facade_honors_all_existing_disable_switches() {
        let decision = decide_route(
            &RequestFacts {
                task: "lint".to_string(),
                requested_model: "requested".to_string(),
                cheap_model: "cheap".to_string(),
                capable_model: "capable".to_string(),
            },
            &ClientFacts {
                client: "codex".to_string(),
            },
            ContextFacts {
                context_tokens: 95,
                context_window_tokens: 100,
                projected_next_turn_tokens: 5,
            },
            &CacheFacts {
                prompt_segments: vec![
                    PromptSegmentPlan {
                        id: "user".to_string(),
                        stable: false,
                        cacheable_tokens: 1,
                        original_index: 0,
                    },
                    PromptSegmentPlan {
                        id: "system".to_string(),
                        stable: true,
                        cacheable_tokens: 10,
                        original_index: 1,
                    },
                ],
            },
            &EndpointFacts {
                endpoint_id: "disabled-endpoint".to_string(),
                model_id: "different".to_string(),
                enabled: false,
            },
            &UserPolicy {
                actions: OptimizationActionPolicy {
                    prompt_cache_reorder_enabled: false,
                    preemptive_compaction_enabled: false,
                    model_routing_enabled: false,
                    max_prompt_reorder_items: 24,
                },
                compaction_threshold_percent: 90,
            },
        );

        assert_eq!(decision.prompt_segment_order, vec!["user", "system"]);
        assert!(!decision.compaction_action.should_compact);
        assert_eq!(decision.model_route_observation.selected_model, "requested");
        assert!(decision
            .reasons
            .contains(&"endpoint_disabled_no_automatic_fallback".to_string()));
        assert_eq!(decision.selected_model, "requested");
    }
}
