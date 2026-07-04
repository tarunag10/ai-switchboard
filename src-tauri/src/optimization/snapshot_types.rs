use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationSnapshot {
    pub(crate) generated_at: String,
    pub(crate) prompt_cache_segments: Vec<PromptCacheSegmentSnapshot>,
    pub(crate) prompt_cache_clients: Vec<PromptCacheClientSnapshot>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptCacheClientSnapshot {
    pub(crate) client: String,
    pub(crate) provider: String,
    pub(crate) prompt_tokens: u64,
    pub(crate) cache_read_tokens: u64,
    pub(crate) cache_creation_tokens: u64,
    pub(crate) efficiency_percent: u8,
    pub(crate) proof: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenXrayBucketSnapshot {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) tokens: u64,
    pub(crate) percent: u8,
    pub(crate) source: String,
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
    pub(crate) buckets: Vec<TokenXrayBucketSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedundancyFindingSnapshot {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) duplicate_tokens: u64,
    pub(crate) locations: Vec<String>,
    pub(crate) action: String,
    pub(crate) read_count: u64,
    pub(crate) duplicate_percent: u8,
    pub(crate) proof: String,
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
