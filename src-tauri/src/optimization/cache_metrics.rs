use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProviderTokenUsage {
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) cache_creation_input_tokens: u64,
    pub(crate) cache_read_input_tokens: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CacheTokenMetrics {
    pub(crate) prompt_tokens: u64,
    pub(crate) completion_tokens: u64,
    pub(crate) cache_creation_tokens: u64,
    pub(crate) cache_read_tokens: u64,
}

impl CacheTokenMetrics {
    pub(crate) const fn from_provider_usage(usage: ProviderTokenUsage) -> Self {
        Self {
            prompt_tokens: usage.input_tokens,
            completion_tokens: usage.output_tokens,
            cache_creation_tokens: usage.cache_creation_input_tokens,
            cache_read_tokens: usage.cache_read_input_tokens,
        }
    }

    pub(crate) const fn total_tokens(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }

    pub(crate) const fn cached_prompt_tokens(&self) -> u64 {
        self.cache_creation_tokens + self.cache_read_tokens
    }

    pub(crate) fn cache_read_ratio(&self) -> f64 {
        if self.prompt_tokens == 0 {
            return 0.0;
        }

        self.cache_read_tokens as f64 / self.prompt_tokens as f64
    }

    pub(crate) fn saturating_add(self, other: Self) -> Self {
        Self {
            prompt_tokens: self.prompt_tokens.saturating_add(other.prompt_tokens),
            completion_tokens: self
                .completion_tokens
                .saturating_add(other.completion_tokens),
            cache_creation_tokens: self
                .cache_creation_tokens
                .saturating_add(other.cache_creation_tokens),
            cache_read_tokens: self
                .cache_read_tokens
                .saturating_add(other.cache_read_tokens),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheTokenMetrics, ProviderTokenUsage};

    #[test]
    fn maps_provider_usage_to_cache_metrics() {
        let metrics = CacheTokenMetrics::from_provider_usage(ProviderTokenUsage {
            input_tokens: 1_000,
            output_tokens: 200,
            cache_creation_input_tokens: 300,
            cache_read_input_tokens: 500,
        });

        assert_eq!(metrics.prompt_tokens, 1_000);
        assert_eq!(metrics.completion_tokens, 200);
        assert_eq!(metrics.cached_prompt_tokens(), 800);
        assert_eq!(metrics.total_tokens(), 1_200);
        assert_eq!(metrics.cache_read_ratio(), 0.5);
    }

    #[test]
    fn cache_read_ratio_handles_empty_prompt() {
        assert_eq!(CacheTokenMetrics::default().cache_read_ratio(), 0.0);
    }
}
