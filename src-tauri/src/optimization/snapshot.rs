use chrono::Utc;
use std::collections::BTreeMap;

use super::action_policy::{actionable_compaction_decision, load_action_policy};
use super::cache_metrics::CacheTokenMetrics;
use super::compaction::{decide_preemptive_compaction, CompactionInput};
use super::model_routing::{decide_model_route, ModelRouteInput};
use super::redundancy::build_redundancy_report;
use super::rtk_presets::all_presets;
use super::session_packs::{plan_agent_session_pack, AgentSessionStartInput};
use super::snapshot_enrichment::{
    fallback_prompt_cache_clients, fallback_token_buckets, live_prompt_cache_clients,
    live_token_buckets, percent_u8,
};
use super::snapshot_policy::order_prompt_cache_segments;
use super::snapshot_types::PromptCacheClientSnapshot;
use super::snapshot_types::{
    AgentPackSnapshot, CompactionSignalSnapshot, CompressionBypassSnapshot, ModelRoutingSnapshot,
    OptimizationSnapshot, PromptCacheSegmentSnapshot, RedundancyFindingSnapshot, RtkPresetSnapshot,
    TokenXraySnapshot,
};
use super::telemetry::{self, TelemetrySnapshot};
use super::token_xray::{build_token_xray, TokenXrayInput};

fn compression_bypass_snapshot() -> CompressionBypassSnapshot {
    let anthropic = crate::proxy_intercept::headroom_compression_bypass_active(false);
    let openai = crate::proxy_intercept::headroom_compression_bypass_active(true);

    CompressionBypassSnapshot {
        anthropic,
        openai,
        any: anthropic || openai,
    }
}

pub(crate) fn build_optimization_snapshot() -> OptimizationSnapshot {
    let telemetry = telemetry::snapshot();
    if telemetry.has_observations() {
        return build_live_optimization_snapshot(telemetry);
    }

    if !demo_optimization_fallbacks_enabled() {
        return build_live_optimization_snapshot(TelemetrySnapshot::default());
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
        prompt_cache_clients: prompt_cache_clients_for_snapshot(&cache_metrics),
        token_xray: TokenXraySnapshot {
            original_tokens: 22_600,
            optimized_tokens: xray
                .total_tokens
                .saturating_sub(cache_metrics.cache_read_tokens),
            system_tokens: 3_900,
            user_tokens: 4_700,
            tool_tokens: 3_300,
            pack_tokens: 2_800,
            buckets: fallback_token_buckets(),
        },
        redundancy: vec![RedundancyFindingSnapshot {
            id: "duplicate-instructions".to_string(),
            label: "Repeated stable instructions".to_string(),
            duplicate_tokens: redundancy.repeated_tokens,
            locations: vec!["AGENTS.md".to_string(), "session history".to_string()],
            action: "Prefer cached session prefix and repo pack injection".to_string(),
            read_count: 3,
            duplicate_percent: 35,
            proof: "fallback duplicate hash across session history and repo pack".to_string(),
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
        bypass: compression_bypass_snapshot(),
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
            let read_count = records.len() as u64;
            let duplicate_tokens = records.iter().map(|record| record.estimated_tokens).sum();
            let locations: Vec<String> = records
                .iter()
                .map(|record| record.source_id.clone())
                .collect();
            Some(RedundancyFindingSnapshot {
                id: format!("duplicate-{}", &content_sha256[..12]),
                label: "Duplicate payload hash".to_string(),
                duplicate_tokens,
                locations,
                action: "Deduplicate repeated payload before routing".to_string(),
                read_count,
                duplicate_percent: percent_u8(duplicate_tokens, original_tokens.max(1)),
                proof: format!("same content hash observed {read_count} times"),
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
        Vec::new()
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
        prompt_cache_clients: prompt_cache_clients_for_snapshot(&cache_metrics),
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
            buckets: live_token_buckets(&bucket_tokens),
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
        bypass: compression_bypass_snapshot(),
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

fn prompt_cache_clients_for_snapshot(
    metrics: &CacheTokenMetrics,
) -> Vec<PromptCacheClientSnapshot> {
    let live = live_prompt_cache_clients(metrics);
    if live.is_empty() && demo_optimization_fallbacks_enabled() {
        fallback_prompt_cache_clients()
    } else {
        live
    }
}

fn demo_optimization_fallbacks_enabled() -> bool {
    matches!(
        std::env::var("AI_SWITCHBOARD_DEMO_OPTIMIZATION")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

#[cfg(test)]
mod tests {

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
    use super::*;

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
}
