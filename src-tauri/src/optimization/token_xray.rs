use serde::{Deserialize, Serialize};

use super::cache_metrics::CacheTokenMetrics;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenXrayInput {
    pub(crate) history_tokens: u64,
    pub(crate) file_read_tokens: u64,
    pub(crate) tool_output_tokens: u64,
    pub(crate) retry_tokens: u64,
    pub(crate) cache_metrics: CacheTokenMetrics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenXrayReport {
    pub(crate) total_tokens: u64,
    pub(crate) history_percent: f64,
    pub(crate) file_read_percent: f64,
    pub(crate) tool_output_percent: f64,
    pub(crate) retry_percent: f64,
    pub(crate) cache_efficiency_percent: f64,
    pub(crate) biggest_bucket: String,
}

pub(crate) fn build_token_xray(input: TokenXrayInput) -> TokenXrayReport {
    let provider_usage_tokens = input.cache_metrics.total_tokens();
    let total_tokens = input
        .history_tokens
        .saturating_add(input.file_read_tokens)
        .saturating_add(input.tool_output_tokens)
        .saturating_add(input.retry_tokens)
        .saturating_add(provider_usage_tokens);

    let buckets = [
        ("history", input.history_tokens),
        ("file_reads", input.file_read_tokens),
        ("tool_output", input.tool_output_tokens),
        ("retries", input.retry_tokens),
        ("provider_usage", provider_usage_tokens),
    ];
    let biggest_bucket = buckets
        .iter()
        .max_by_key(|(_, tokens)| *tokens)
        .map(|(name, _)| (*name).to_string())
        .unwrap_or_else(|| "none".to_string());

    TokenXrayReport {
        total_tokens,
        history_percent: percent(input.history_tokens, total_tokens),
        file_read_percent: percent(input.file_read_tokens, total_tokens),
        tool_output_percent: percent(input.tool_output_tokens, total_tokens),
        retry_percent: percent(input.retry_tokens, total_tokens),
        cache_efficiency_percent: input.cache_metrics.cache_read_ratio() * 100.0,
        biggest_bucket,
    }
}

fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_largest_token_bucket() {
        let report = build_token_xray(TokenXrayInput {
            history_tokens: 10,
            file_read_tokens: 30,
            tool_output_tokens: 20,
            retry_tokens: 0,
            cache_metrics: CacheTokenMetrics::default(),
        });

        assert_eq!(report.total_tokens, 60);
        assert_eq!(report.biggest_bucket, "file_reads");
    }
}
