use chrono::Utc;
use std::collections::BTreeMap;

use super::action_policy::{
    actionable_compaction_decision, actionable_model_route, load_action_policy,
    plan_prompt_cache_order, PromptSegmentPlan,
};
use super::cache_metrics::CacheTokenMetrics;
use super::compaction::{decide_preemptive_compaction, CompactionInput};
use super::model_routing::{decide_model_route, ModelRouteInput};
use super::redundancy::build_redundancy_report;
use super::rtk_presets::all_presets;
use super::session_packs::{plan_agent_session_pack, AgentSessionStartInput};
use super::snapshot_types::{
    AgentPackSnapshot, CompactionSignalSnapshot, ModelRoutingSnapshot, OptimizationSnapshot,
    PromptCacheSegmentSnapshot, RedundancyFindingSnapshot, RtkPresetSnapshot, TokenXraySnapshot,
};
use super::telemetry::{self, TelemetrySnapshot};
use super::token_xray::{build_token_xray, TokenXrayInput};

pub(crate) fn build_optimization_snapshot() -> OptimizationSnapshot {
    let telemetry = telemetry::snapshot();
    if telemetry.has_observations() {
        return build_live_optimization_snapshot(telemetry);
    }

    let prompt_cache_segments = order_prompt_cache_segments(vec![
        PromptCacheSegmentSnapshot {
            id: "system".to_string(),
            label: "System prompt".to_string(),
            tokens: 7_800,
            cacheable_tokens: 7_200,
            hit_tokens: 6_100,
            changes_per_session: 2,
        },
        PromptCacheSegmentSnapshot {
            id: "repo-pack".to_string(),
            label: "Repo Intelligence pack".to_string(),
            tokens: 2_800,
            cacheable_tokens: 2_300,
            hit_tokens: 1_700,
            changes_per_session: 1,
        },
        PromptCacheSegmentSnapshot {
            id: "tool-output".to_string(),
            label: "Tool output".to_string(),
            tokens: 1_600,
            cacheable_tokens: 250,
            hit_tokens: 80,
            changes_per_session: 3,
        },
    ]);

    let cache_metrics = CacheTokenMetrics {
        prompt_tokens: 12_200,
        completion_tokens: 1_800,
        cache_creation_tokens: 2_000,
        cache_read_tokens: 7_880,
    };
    let xray = build_token_xray(TokenXrayInput {
        history_tokens: 4_200,
        file_read_tokens: 3_100,
        tool_output_tokens: 2_600,
        retry_tokens: 700,
        cache_metrics,
    });

    let redundancy = build_redundancy_report(&[
        (
            "AGENTS.md".to_string(),
            "stable switchboard instructions".to_string(),
            1_180,
        ),
        (
            "session-history".to_string(),
            "stable switchboard instructions".to_string(),
            1_180,
        ),
        (
            "rollout-memory".to_string(),
            "old rollout recap".to_string(),
            740,
        ),
    ]);

    let lint_route = decide_model_route(&ModelRouteInput {
        client: "codex".to_string(),
        task: "lint fix and commit message".to_string(),
        requested_model: "frontier".to_string(),
        cheap_model: "fast/local".to_string(),
        capable_model: "frontier".to_string(),
        enabled: true,
    });
    let plan_route = decide_model_route(&ModelRouteInput {
        client: "codex".to_string(),
        task: "multi-file implementation planning".to_string(),
        requested_model: "frontier".to_string(),
        cheap_model: "fast/local".to_string(),
        capable_model: "frontier".to_string(),
        enabled: true,
    });

    let compaction = decide_preemptive_compaction(CompactionInput {
        context_tokens: xray.total_tokens,
        context_window_tokens: 24_000,
        projected_next_turn_tokens: 3_500,
        threshold_percent: 72,
    });

    let agent_pack = plan_agent_session_pack(&AgentSessionStartInput {
        agent: "codex".to_string(),
        task: "Start Agent Session".to_string(),
        repo_root: "current".to_string(),
        pack_id: Some("implementation".to_string()),
        token_budget: 12_000,
        pack_estimated_tokens: 2_800,
        enabled: true,
    });

    OptimizationSnapshot {
        generated_at: Utc::now().to_rfc3339(),
        prompt_cache_segments,
        token_xray: TokenXraySnapshot {
            original_tokens: 22_600,
            optimized_tokens: xray
                .total_tokens
                .saturating_sub(cache_metrics.cache_read_tokens),
            system_tokens: 3_900,
            user_tokens: 4_700,
            tool_tokens: 3_300,
            pack_tokens: 2_800,
        },
        redundancy: vec![RedundancyFindingSnapshot {
            id: "duplicate-instructions".to_string(),
            label: "Repeated stable instructions".to_string(),
            duplicate_tokens: redundancy.repeated_tokens,
            locations: vec!["AGENTS.md".to_string(), "session history".to_string()],
            action: "Prefer cached session prefix and repo pack injection".to_string(),
        }],
        routing: vec![
            ModelRoutingSnapshot {
                task: "Lint fixes and commit copy".to_string(),
                current_model: "frontier".to_string(),
                selected_model: lint_route.selected_model,
                fallback_model: "frontier".to_string(),
                reason: lint_route.reason,
                estimated_savings_percent: 64,
            },
            ModelRoutingSnapshot {
                task: "Implementation planning".to_string(),
                current_model: "frontier".to_string(),
                selected_model: plan_route.selected_model,
                fallback_model: "fast/local".to_string(),
                reason: plan_route.reason,
                estimated_savings_percent: 18,
            },
        ],
        compaction: CompactionSignalSnapshot {
            should_compact: compaction.should_compact,
            context_used_percent: compaction.projected_utilization_percent,
            threshold_percent: 72,
            reason: compaction.reason,
        },
        agent_pack: AgentPackSnapshot {
            source: agent_pack.pack_id,
            injected: agent_pack.inject_pack,
            last_injected_at: Some(Utc::now().to_rfc3339()),
            status: agent_pack.reason,
        },
        rtk_presets: all_presets()
            .into_iter()
            .map(|preset| RtkPresetSnapshot {
                id: format!("{:?}", preset.framework).to_ascii_lowercase(),
                label: format!("{:?}", preset.framework),
                command: preset.command_prefix,
                focus: preset.keep_patterns.join(", "),
            })
            .collect(),
    }
}

fn build_live_optimization_snapshot(telemetry: TelemetrySnapshot) -> OptimizationSnapshot {
    let cache_metrics = telemetry.cache_metrics;
    let bucket_tokens = telemetry.token_buckets.iter().fold(
        BTreeMap::<String, u64>::new(),
        |mut totals, bucket| {
            *totals
                .entry(bucket.bucket.to_ascii_lowercase())
                .or_default() += bucket.tokens;
            totals
        },
    );
    let bucket_total = bucket_tokens.values().copied().sum::<u64>();
    let original_tokens = bucket_total.saturating_add(cache_metrics.total_tokens());
    let optimized_tokens = original_tokens.saturating_sub(cache_metrics.cache_read_tokens);

    let prompt_cache_segments = order_prompt_cache_segments(vec![PromptCacheSegmentSnapshot {
        id: "observed-cache".to_string(),
        label: "Observed prompt cache".to_string(),
        tokens: cache_metrics.prompt_tokens,
        cacheable_tokens: cache_metrics
            .cache_creation_tokens
            .saturating_add(cache_metrics.cache_read_tokens),
        hit_tokens: cache_metrics.cache_read_tokens,
        changes_per_session: 0,
    }]);

    let mut hashes_by_value = BTreeMap::<String, Vec<_>>::new();
    for hash in telemetry.redundancy_hashes {
        hashes_by_value
            .entry(hash.content_sha256.clone())
            .or_default()
            .push(hash);
    }
    let redundancy = hashes_by_value
        .into_iter()
        .filter_map(|(content_sha256, records)| {
            if records.len() < 2 {
                return None;
            }
            Some(RedundancyFindingSnapshot {
                id: format!("duplicate-{}", &content_sha256[..12]),
                label: "Duplicate payload hash".to_string(),
                duplicate_tokens: records.iter().map(|record| record.estimated_tokens).sum(),
                locations: records.into_iter().map(|record| record.source_id).collect(),
                action: "Deduplicate repeated payload before routing".to_string(),
            })
        })
        .collect();

    let compaction_input = CompactionInput {
        context_tokens: original_tokens,
        context_window_tokens: 200_000,
        projected_next_turn_tokens: bucket_tokens.get("history").copied().unwrap_or_default(),
        threshold_percent: 72,
    };
    let compaction = telemetry
        .compaction_decision
        .map(|record| super::snapshot_types::CompactionSignalSnapshot {
            should_compact: record.should_compact,
            context_used_percent: f64::from(record.context_used_percent),
            threshold_percent: record.threshold_percent,
            reason: record.reason,
        })
        .unwrap_or_else(|| {
            let decision = actionable_compaction_decision(&load_action_policy(), compaction_input);
            super::snapshot_types::CompactionSignalSnapshot {
                should_compact: decision.should_compact,
                context_used_percent: decision.utilization_percent,
                threshold_percent: 72,
                reason: decision.reason,
            }
        });

    let routing = if telemetry.routing_decisions.is_empty() {
        vec![route_to_snapshot(ModelRouteInput {
            client: "observed-session".to_string(),
            task: "general".to_string(),
            requested_model: "frontier".to_string(),
            cheap_model: "fast/local".to_string(),
            capable_model: "frontier".to_string(),
            enabled: true,
        })]
    } else {
        telemetry
            .routing_decisions
            .into_iter()
            .map(|record| ModelRoutingSnapshot {
                task: record.task,
                current_model: record.current_model,
                selected_model: record.selected_model,
                fallback_model: record.fallback_model,
                reason: record.reason,
                estimated_savings_percent: record.estimated_savings_percent,
            })
            .collect()
    };

    OptimizationSnapshot {
        generated_at: Utc::now().to_rfc3339(),
        prompt_cache_segments,
        token_xray: TokenXraySnapshot {
            original_tokens,
            optimized_tokens,
            system_tokens: bucket_tokens.get("system").copied().unwrap_or_default(),
            user_tokens: bucket_tokens.get("user").copied().unwrap_or_default(),
            tool_tokens: bucket_tokens
                .get("tool")
                .or_else(|| bucket_tokens.get("tool_output"))
                .copied()
                .unwrap_or_default(),
            pack_tokens: bucket_tokens
                .get("pack")
                .or_else(|| bucket_tokens.get("session_pack"))
                .copied()
                .unwrap_or_default(),
        },
        redundancy,
        routing,
        compaction,
        agent_pack: {
            let pack = plan_agent_session_pack(&AgentSessionStartInput {
                agent: "codex".to_string(),
                task: "Start Agent Session".to_string(),
                repo_root: "current".to_string(),
                pack_id: Some("implementation".to_string()),
                token_budget: 12_000,
                pack_estimated_tokens: 2_800,
                enabled: true,
            });
            AgentPackSnapshot {
                source: pack.pack_id,
                injected: pack.inject_pack,
                last_injected_at: Some(Utc::now().to_rfc3339()),
                status: pack.reason,
            }
        },
        rtk_presets: if telemetry.rtk_presets.is_empty() {
            all_presets()
                .into_iter()
                .map(|preset| RtkPresetSnapshot {
                    id: format!("{:?}", preset.framework).to_ascii_lowercase(),
                    label: format!("{:?}", preset.framework),
                    command: preset.command_prefix,
                    focus: preset.keep_patterns.join(", "),
                })
                .collect()
        } else {
            telemetry
                .rtk_presets
                .into_iter()
                .map(|preset| RtkPresetSnapshot {
                    id: preset.id,
                    label: preset.label,
                    command: preset.command,
                    focus: preset.focus,
                })
                .collect()
        },
    }
}

fn order_prompt_cache_segments(
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

fn route_to_snapshot(input: ModelRouteInput) -> ModelRoutingSnapshot {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_covers_all_requested_feature_groups() {
        let _guard = telemetry::test_guard();
        telemetry::reset_for_tests();
        let snapshot = build_optimization_snapshot();

        assert_eq!(snapshot.prompt_cache_segments.len(), 3);
        assert!(!snapshot.routing.is_empty());
        assert!(!snapshot.redundancy.is_empty());
        assert!(snapshot.agent_pack.injected);
        assert_eq!(snapshot.rtk_presets.len(), 4);
        assert!(snapshot.token_xray.original_tokens > snapshot.token_xray.optimized_tokens);
    }

    #[test]
    fn snapshot_uses_live_telemetry_when_observed() {
        let _guard = telemetry::test_guard();
        telemetry::reset_for_tests();
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
        assert_eq!(snapshot.token_xray.system_tokens, 10);
        assert_eq!(snapshot.token_xray.tool_tokens, 5);
        assert_eq!(snapshot.redundancy[0].duplicate_tokens, 20);
        assert_eq!(snapshot.routing[0].selected_model, "fast/local");
        telemetry::reset_for_tests();
    }
}
