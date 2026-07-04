use std::collections::VecDeque;
#[cfg(test)]
use std::sync::MutexGuard;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::cache_metrics::CacheTokenMetrics;
use super::telemetry_store;

const MAX_CACHE_EVENTS: usize = 128;
const MAX_TOKEN_BUCKETS: usize = 32;
const MAX_REDUNDANCY_HASHES: usize = 256;
const MAX_ROUTING_DECISIONS: usize = 64;
const MAX_RTK_PRESETS: usize = 32;

static COLLECTOR: OnceLock<Mutex<TelemetryCollector>> = OnceLock::new();
#[cfg(test)]
static TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenBucketMetrics {
    pub(crate) bucket: String,
    pub(crate) tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedundancyHashRecord {
    pub(crate) source_id: String,
    pub(crate) content_sha256: String,
    pub(crate) estimated_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompactionDecisionRecord {
    pub(crate) should_compact: bool,
    pub(crate) context_used_percent: u8,
    pub(crate) threshold_percent: u8,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoutingDecisionRecord {
    pub(crate) task: String,
    pub(crate) current_model: String,
    pub(crate) selected_model: String,
    pub(crate) fallback_model: String,
    pub(crate) reason: String,
    pub(crate) estimated_savings_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RtkPresetMetadata {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) focus: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TelemetrySnapshot {
    pub(crate) cache_metrics: CacheTokenMetrics,
    pub(crate) token_buckets: Vec<TokenBucketMetrics>,
    pub(crate) redundancy_hashes: Vec<RedundancyHashRecord>,
    pub(crate) compaction_decision: Option<CompactionDecisionRecord>,
    pub(crate) routing_decisions: Vec<RoutingDecisionRecord>,
    pub(crate) rtk_presets: Vec<RtkPresetMetadata>,
}

impl TelemetrySnapshot {
    pub(crate) fn has_observations(&self) -> bool {
        self.cache_metrics.total_tokens() > 0
            || !self.token_buckets.is_empty()
            || !self.redundancy_hashes.is_empty()
            || self.compaction_decision.is_some()
            || !self.routing_decisions.is_empty()
            || !self.rtk_presets.is_empty()
    }
}

#[derive(Debug, Default)]
struct TelemetryCollector {
    cache_events: VecDeque<CacheTokenMetrics>,
    token_buckets: VecDeque<TokenBucketMetrics>,
    redundancy_hashes: VecDeque<RedundancyHashRecord>,
    compaction_decision: Option<CompactionDecisionRecord>,
    routing_decisions: VecDeque<RoutingDecisionRecord>,
    rtk_presets: VecDeque<RtkPresetMetadata>,
}

pub(crate) fn record_prompt_cache_metrics(metrics: CacheTokenMetrics) {
    telemetry_store::record_prompt_cache_metrics(&metrics);
    with_collector(|collector| {
        push_bounded(&mut collector.cache_events, metrics, MAX_CACHE_EVENTS)
    });
}

pub(crate) fn record_token_xray_bucket(bucket: impl Into<String>, tokens: u64) {
    let bucket = bucket.into();
    if bucket.trim().is_empty() || tokens == 0 {
        return;
    }
    with_collector(|collector| {
        push_bounded(
            &mut collector.token_buckets,
            TokenBucketMetrics { bucket, tokens },
            MAX_TOKEN_BUCKETS,
        );
    });
}

pub(crate) fn record_redundancy_hash(
    source_id: impl Into<String>,
    content_sha256: impl Into<String>,
    estimated_tokens: u64,
) {
    let source_id = source_id.into();
    let content_sha256 = content_sha256.into();
    if source_id.trim().is_empty() || content_sha256.trim().is_empty() {
        return;
    }
    with_collector(|collector| {
        push_bounded(
            &mut collector.redundancy_hashes,
            RedundancyHashRecord {
                source_id,
                content_sha256,
                estimated_tokens,
            },
            MAX_REDUNDANCY_HASHES,
        );
    });
}

pub(crate) fn record_redundancy_payload_hash(
    source_id: impl Into<String>,
    payload: &[u8],
    estimated_tokens: u64,
) {
    record_redundancy_hash(source_id, sha256_hex(payload), estimated_tokens);
}

pub(crate) fn record_compaction_decision(decision: CompactionDecisionRecord) {
    with_collector(|collector| {
        collector.compaction_decision = Some(decision);
    });
}

pub(crate) fn record_routing_decision(decision: RoutingDecisionRecord) {
    with_collector(|collector| {
        push_bounded(
            &mut collector.routing_decisions,
            decision,
            MAX_ROUTING_DECISIONS,
        );
    });
}

pub(crate) fn record_rtk_preset_metadata(metadata: RtkPresetMetadata) {
    with_collector(|collector| {
        push_bounded(&mut collector.rtk_presets, metadata, MAX_RTK_PRESETS);
    });
}

pub(crate) fn snapshot() -> TelemetrySnapshot {
    with_collector(|collector| TelemetrySnapshot {
        cache_metrics: telemetry_store::prompt_cache_totals(),
        token_buckets: collector.token_buckets.iter().cloned().collect(),
        redundancy_hashes: collector.redundancy_hashes.iter().cloned().collect(),
        compaction_decision: collector
            .compaction_decision
            .clone()
            .or_else(telemetry_store::latest_compaction_decision),
        routing_decisions: collector.routing_decisions.iter().cloned().collect(),
        rtk_presets: collector.rtk_presets.iter().cloned().collect(),
    })
}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    with_collector(|collector| *collector = TelemetryCollector::default());
    telemetry_store::reset_for_tests();
}

#[cfg(test)]
pub(crate) fn test_guard() -> MutexGuard<'static, ()> {
    TEST_GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("optimization telemetry test guard poisoned")
}

fn with_collector<T>(f: impl FnOnce(&mut TelemetryCollector) -> T) -> T {
    let mut collector = COLLECTOR
        .get_or_init(|| Mutex::new(TelemetryCollector::default()))
        .lock()
        .expect("optimization telemetry collector poisoned");
    f(&mut collector)
}

fn push_bounded<T>(items: &mut VecDeque<T>, item: T, max_len: usize) {
    if items.len() == max_len {
        items.pop_front();
    }
    items.push_back(item);
}

fn sha256_hex(payload: &[u8]) -> String {
    format!("{:x}", Sha256::digest(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_hash_only_without_payload_content() {
        let _guard = test_guard();
        reset_for_tests();

        record_redundancy_payload_hash("request", b"very secret prompt", 12);
        let snapshot = snapshot();

        assert_eq!(snapshot.redundancy_hashes.len(), 1);
        assert_eq!(snapshot.redundancy_hashes[0].source_id, "request");
        assert_eq!(snapshot.redundancy_hashes[0].estimated_tokens, 12);
        assert_ne!(
            snapshot.redundancy_hashes[0].content_sha256,
            "very secret prompt"
        );
        assert_eq!(snapshot.redundancy_hashes[0].content_sha256.len(), 64);
    }

    #[test]
    fn aggregates_cache_metrics() {
        let _guard = test_guard();
        reset_for_tests();

        record_prompt_cache_metrics(CacheTokenMetrics {
            prompt_tokens: 10,
            completion_tokens: 2,
            cache_creation_tokens: 3,
            cache_read_tokens: 4,
        });
        record_prompt_cache_metrics(CacheTokenMetrics {
            prompt_tokens: 7,
            completion_tokens: 1,
            cache_creation_tokens: 0,
            cache_read_tokens: 6,
        });

        let metrics = snapshot().cache_metrics;
        assert_eq!(metrics.prompt_tokens, 17);
        assert_eq!(metrics.completion_tokens, 3);
        assert_eq!(metrics.cache_creation_tokens, 3);
        assert_eq!(metrics.cache_read_tokens, 10);
    }
}
