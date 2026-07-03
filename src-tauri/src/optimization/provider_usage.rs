use serde_json::Value;

use super::cache_metrics::{CacheTokenMetrics, ProviderTokenUsage};

pub(crate) fn parse_provider_cache_metrics(body: &[u8]) -> Option<CacheTokenMetrics> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let usage = value.get("usage")?;

    let input_tokens = read_u64(usage, &["input_tokens", "prompt_tokens"]);
    let output_tokens = read_u64(usage, &["output_tokens", "completion_tokens"]);
    let cache_creation_input_tokens = read_u64(
        usage,
        &[
            "cache_creation_input_tokens",
            "cache_creation_tokens",
            "prompt_cache_creation_tokens",
        ],
    );
    let cache_read_input_tokens = read_u64(
        usage,
        &[
            "cache_read_input_tokens",
            "cache_read_tokens",
            "prompt_cache_hit_tokens",
            "cached_tokens",
        ],
    );

    if input_tokens == 0
        && output_tokens == 0
        && cache_creation_input_tokens == 0
        && cache_read_input_tokens == 0
    {
        return None;
    }

    Some(CacheTokenMetrics::from_provider_usage(ProviderTokenUsage {
        input_tokens,
        output_tokens,
        cache_creation_input_tokens,
        cache_read_input_tokens,
    }))
}

fn read_u64(value: &Value, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_u64))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_anthropic_cache_usage() {
        let metrics = parse_provider_cache_metrics(
            br#"{
              "usage": {
                "input_tokens": 100,
                "output_tokens": 25,
                "cache_creation_input_tokens": 30,
                "cache_read_input_tokens": 70
              }
            }"#,
        )
        .expect("metrics");

        assert_eq!(metrics.prompt_tokens, 100);
        assert_eq!(metrics.completion_tokens, 25);
        assert_eq!(metrics.cache_creation_tokens, 30);
        assert_eq!(metrics.cache_read_tokens, 70);
    }

    #[test]
    fn parses_openai_cached_tokens() {
        let metrics = parse_provider_cache_metrics(
            br#"{
              "usage": {
                "prompt_tokens": 200,
                "completion_tokens": 40,
                "prompt_cache_hit_tokens": 120
              }
            }"#,
        )
        .expect("metrics");

        assert_eq!(metrics.prompt_tokens, 200);
        assert_eq!(metrics.completion_tokens, 40);
        assert_eq!(metrics.cache_read_tokens, 120);
    }

    #[test]
    fn ignores_non_usage_payloads() {
        assert!(parse_provider_cache_metrics(br#"{"ok":true}"#).is_none());
    }
}
