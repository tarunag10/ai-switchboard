use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OptimizationPolicy {
    pub(crate) enabled: bool,
    pub(crate) collect_token_metrics: bool,
    pub(crate) collect_cache_metrics: bool,
}

impl OptimizationPolicy {
    pub(crate) const fn disabled() -> Self {
        Self {
            enabled: false,
            collect_token_metrics: false,
            collect_cache_metrics: false,
        }
    }

    pub(crate) const fn metrics_enabled() -> Self {
        Self {
            enabled: true,
            collect_token_metrics: true,
            collect_cache_metrics: true,
        }
    }

    pub(crate) const fn allows_token_metrics(&self) -> bool {
        self.enabled && self.collect_token_metrics
    }

    pub(crate) const fn allows_cache_metrics(&self) -> bool {
        self.enabled && self.collect_cache_metrics
    }
}

impl Default for OptimizationPolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use super::OptimizationPolicy;

    #[test]
    fn default_policy_collects_nothing() {
        let policy = OptimizationPolicy::default();

        assert!(!policy.enabled);
        assert!(!policy.allows_token_metrics());
        assert!(!policy.allows_cache_metrics());
    }

    #[test]
    fn metric_collection_requires_global_enablement() {
        let policy = OptimizationPolicy {
            enabled: false,
            collect_token_metrics: true,
            collect_cache_metrics: true,
        };

        assert!(!policy.allows_token_metrics());
        assert!(!policy.allows_cache_metrics());
    }
}
