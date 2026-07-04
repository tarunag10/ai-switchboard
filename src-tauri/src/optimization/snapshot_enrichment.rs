use std::collections::BTreeMap;

use super::cache_metrics::CacheTokenMetrics;
use super::snapshot_types::{PromptCacheClientSnapshot, TokenXrayBucketSnapshot};

pub(super) fn percent_u8(part: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    ((part.saturating_mul(100) / total).min(100)) as u8
}

pub(super) fn fallback_prompt_cache_clients() -> Vec<PromptCacheClientSnapshot> {
    vec![
        PromptCacheClientSnapshot {
            client: "Codex".to_string(),
            provider: "OpenAI".to_string(),
            prompt_tokens: 12_200,
            cache_read_tokens: 7_800,
            cache_creation_tokens: 2_100,
            efficiency_percent: 64,
            proof: "Fallback sample until provider usage telemetry arrives".to_string(),
        },
        PromptCacheClientSnapshot {
            client: "Claude Code".to_string(),
            provider: "Anthropic".to_string(),
            prompt_tokens: 8_100,
            cache_read_tokens: 4_600,
            cache_creation_tokens: 1_900,
            efficiency_percent: 57,
            proof: "Fallback sample until provider usage telemetry arrives".to_string(),
        },
    ]
}

pub(super) fn live_prompt_cache_clients(
    metrics: &CacheTokenMetrics,
) -> Vec<PromptCacheClientSnapshot> {
    if metrics.total_tokens() == 0 {
        return Vec::new();
    }
    vec![PromptCacheClientSnapshot {
        client: "Observed client traffic".to_string(),
        provider: "Provider usage telemetry".to_string(),
        prompt_tokens: metrics.prompt_tokens,
        cache_read_tokens: metrics.cache_read_tokens,
        cache_creation_tokens: metrics.cache_creation_tokens,
        efficiency_percent: percent_u8(metrics.cache_read_tokens, metrics.prompt_tokens.max(1)),
        proof: "Parsed provider cache_read/cache_creation tokens from live response usage"
            .to_string(),
    }]
}

pub(super) fn fallback_token_buckets() -> Vec<TokenXrayBucketSnapshot> {
    let buckets = [
        ("system", "System prompts", 3_900, "stable prefix"),
        ("user", "User/history", 4_700, "conversation"),
        ("tool", "Tool output", 3_300, "tool telemetry"),
        ("pack", "Repo pack", 2_800, "Start Agent Session"),
    ];
    let total = buckets.iter().map(|(_, _, tokens, _)| *tokens).sum::<u64>();
    buckets
        .into_iter()
        .map(|(id, label, tokens, source)| TokenXrayBucketSnapshot {
            id: id.to_string(),
            label: label.to_string(),
            tokens,
            percent: percent_u8(tokens, total),
            source: source.to_string(),
        })
        .collect()
}

pub(super) fn live_token_buckets(
    bucket_tokens: &BTreeMap<String, u64>,
) -> Vec<TokenXrayBucketSnapshot> {
    let total = bucket_tokens.values().copied().sum::<u64>();
    bucket_tokens
        .iter()
        .map(|(bucket, tokens)| TokenXrayBucketSnapshot {
            id: bucket.clone(),
            label: token_bucket_label(bucket),
            tokens: *tokens,
            percent: percent_u8(*tokens, total),
            source: "live proxy/session telemetry".to_string(),
        })
        .collect()
}

fn token_bucket_label(bucket: &str) -> String {
    match bucket {
        "system" => "System prompts".to_string(),
        "user" | "history" => "User/history".to_string(),
        "tool" | "tool_output" => "Tool output".to_string(),
        "pack" | "session_pack" => "Repo pack".to_string(),
        "retry" | "retries" => "Retries".to_string(),
        other => other.replace('_', " "),
    }
}
