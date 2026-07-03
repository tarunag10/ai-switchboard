use chrono::Utc;
use serde::Serialize;

use super::cache_metrics::CacheTokenMetrics;
use super::compaction::{decide_preemptive_compaction, CompactionInput};
use super::model_routing::{decide_model_route, ModelRouteInput};
use super::redundancy::build_redundancy_report;
use super::rtk_presets::all_presets;
use super::session_packs::{plan_agent_session_pack, AgentSessionStartInput};
use super::token_xray::{build_token_xray, TokenXrayInput};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationSnapshot {
    pub(crate) generated_at: String,
    pub(crate) prompt_cache_segments: Vec<PromptCacheSegmentSnapshot>,
    pub(crate) token_xray: TokenXraySnapshot,
    pub(crate) redundancy: Vec<RedundancyFindingSnapshot>,
    pub(crate) routing: Vec<ModelRoutingSnapshot>,
    pub(crate) compaction: CompactionSignalSnapshot,
    pub(crate) agent_pack: AgentPackSnapshot,
    pub(crate) rtk_presets: Vec<RtkPresetSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptCacheSegmentSnapshot {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) tokens: u64,
    pub(crate) cacheable_tokens: u64,
    pub(crate) hit_tokens: u64,
    pub(crate) changes_per_session: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenXraySnapshot {
    pub(crate) original_tokens: u64,
    pub(crate) optimized_tokens: u64,
    pub(crate) system_tokens: u64,
    pub(crate) user_tokens: u64,
    pub(crate) tool_tokens: u64,
    pub(crate) pack_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedundancyFindingSnapshot {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) duplicate_tokens: u64,
    pub(crate) locations: Vec<String>,
    pub(crate) action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingSnapshot {
    pub(crate) task: String,
    pub(crate) current_model: String,
    pub(crate) selected_model: String,
    pub(crate) fallback_model: String,
    pub(crate) reason: String,
    pub(crate) estimated_savings_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompactionSignalSnapshot {
    pub(crate) should_compact: bool,
    pub(crate) context_used_percent: f64,
    pub(crate) threshold_percent: u8,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentPackSnapshot {
    pub(crate) source: String,
    pub(crate) injected: bool,
    pub(crate) last_injected_at: Option<String>,
    pub(crate) status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RtkPresetSnapshot {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) focus: String,
}

pub(crate) fn build_optimization_snapshot() -> OptimizationSnapshot {
    let prompt_cache_segments = vec![
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
    ];

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_covers_all_requested_feature_groups() {
        let snapshot = build_optimization_snapshot();

        assert_eq!(snapshot.prompt_cache_segments.len(), 3);
        assert!(!snapshot.routing.is_empty());
        assert!(!snapshot.redundancy.is_empty());
        assert!(snapshot.agent_pack.injected);
        assert_eq!(snapshot.rtk_presets.len(), 4);
        assert!(snapshot.token_xray.original_tokens > snapshot.token_xray.optimized_tokens);
    }
}
