use serde::{Deserialize, Serialize};

use super::cache_metrics::CacheTokenMetrics;
use super::policy::OptimizationPolicy;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TokenLedgerEntry {
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) cache_metrics: CacheTokenMetrics,
}

impl TokenLedgerEntry {
    pub(crate) const fn observed(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_metrics: CacheTokenMetrics {
                prompt_tokens: input_tokens,
                completion_tokens: output_tokens,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
            },
        }
    }

    pub(crate) const fn with_cache_metrics(cache_metrics: CacheTokenMetrics) -> Self {
        Self {
            input_tokens: cache_metrics.prompt_tokens,
            output_tokens: cache_metrics.completion_tokens,
            cache_metrics,
        }
    }

    pub(crate) const fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TokenLedger {
    entries: Vec<TokenLedgerEntry>,
}

impl TokenLedger {
    pub(crate) fn entries(&self) -> &[TokenLedgerEntry] {
        &self.entries
    }

    pub(crate) fn record(&mut self, policy: &OptimizationPolicy, entry: TokenLedgerEntry) -> bool {
        if !policy.allows_token_metrics() {
            return false;
        }

        self.entries.push(entry);
        true
    }

    pub(crate) fn totals(&self) -> TokenLedgerEntry {
        self.entries
            .iter()
            .copied()
            .fold(TokenLedgerEntry::default(), |total, entry| {
                TokenLedgerEntry {
                    input_tokens: total.input_tokens.saturating_add(entry.input_tokens),
                    output_tokens: total.output_tokens.saturating_add(entry.output_tokens),
                    cache_metrics: total.cache_metrics.saturating_add(entry.cache_metrics),
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenLedger, TokenLedgerEntry};
    use crate::optimization::{CacheTokenMetrics, OptimizationPolicy};

    #[test]
    fn default_policy_prevents_recording() {
        let mut ledger = TokenLedger::default();

        let recorded = ledger.record(
            &OptimizationPolicy::default(),
            TokenLedgerEntry::observed(8, 2),
        );

        assert!(!recorded);
        assert!(ledger.entries().is_empty());
    }

    #[test]
    fn enabled_policy_records_counts_only() {
        let mut ledger = TokenLedger::default();

        let recorded = ledger.record(
            &OptimizationPolicy::metrics_enabled(),
            TokenLedgerEntry::with_cache_metrics(CacheTokenMetrics {
                prompt_tokens: 10,
                completion_tokens: 4,
                cache_creation_tokens: 6,
                cache_read_tokens: 2,
            }),
        );

        assert!(recorded);
        assert_eq!(ledger.entries().len(), 1);
        assert_eq!(ledger.entries()[0].total_tokens(), 14);
    }

    #[test]
    fn totals_saturate_across_entries() {
        let mut ledger = TokenLedger::default();
        let policy = OptimizationPolicy::metrics_enabled();

        ledger.record(&policy, TokenLedgerEntry::observed(u64::MAX, 1));
        ledger.record(&policy, TokenLedgerEntry::observed(1, u64::MAX));

        let totals = ledger.totals();
        assert_eq!(totals.input_tokens, u64::MAX);
        assert_eq!(totals.output_tokens, u64::MAX);
    }
}
