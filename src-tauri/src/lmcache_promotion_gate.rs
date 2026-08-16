//! Benchmark-only LMCache promotion gate.
//!
//! No runtime dependency or installation is performed here. Native prefix
//! caching is the mandatory baseline; LMCache is considered only when a paired
//! benchmark demonstrates useful gains without unacceptable complexity.

use serde::{Deserialize, Serialize};

pub(crate) const LMCACHE_SOURCE_REVISION: &str =
    "LMCache/LMCache@e8f938189d42875abf469f25a34765659e0f9c2d";
pub(crate) const LMCACHE_SOURCE_DATE: &str = "2026-08-16T00:51:19Z";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheBenchmarkArm {
    pub measured: bool,
    pub sample_count: u64,
    pub native_prefix_cache_enabled: bool,
    pub median_ttft_micros: u64,
    pub gpu_prefill_millis: u64,
    pub successful_task_cost_microusd: u64,
    pub task_success_rate_basis_points: u16,
    /// Review score from 0 (no additional operations) to 100 (unacceptable).
    pub operational_complexity_score: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LmCacheBenchmarkEvidence {
    pub evidence_version: String,
    pub source_revision: String,
    pub native_prefix_only: CacheBenchmarkArm,
    pub native_prefix_plus_lmcache: CacheBenchmarkArm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LmCachePromotionPolicy {
    pub minimum_samples_per_arm: u64,
    pub minimum_ttft_gain_basis_points: u16,
    pub minimum_gpu_prefill_gain_basis_points: u16,
    pub minimum_successful_task_cost_gain_basis_points: u16,
    pub maximum_task_success_regression_basis_points: u16,
    pub maximum_operational_complexity_score: u8,
    pub maximum_operational_complexity_increase: u8,
}

impl Default for LmCachePromotionPolicy {
    fn default() -> Self {
        Self {
            minimum_samples_per_arm: 100,
            minimum_ttft_gain_basis_points: 1_500,
            minimum_gpu_prefill_gain_basis_points: 1_000,
            minimum_successful_task_cost_gain_basis_points: 1_000,
            maximum_task_success_regression_basis_points: 50,
            maximum_operational_complexity_score: 50,
            maximum_operational_complexity_increase: 20,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LmCachePromotionDecision {
    pub eligible_for_benchmark_only_promotion: bool,
    pub eligible_for_live_promotion: bool,
    pub ttft_gain_basis_points: Option<u16>,
    pub gpu_prefill_gain_basis_points: Option<u16>,
    pub successful_task_cost_gain_basis_points: Option<u16>,
    pub reasons: Vec<String>,
}

pub(crate) fn evaluate_lmcache_promotion(
    evidence: &LmCacheBenchmarkEvidence,
    policy: LmCachePromotionPolicy,
) -> LmCachePromotionDecision {
    let baseline = &evidence.native_prefix_only;
    let candidate = &evidence.native_prefix_plus_lmcache;
    let mut reasons = Vec::new();

    if evidence.source_revision != LMCACHE_SOURCE_REVISION {
        reasons.push("unpinned_lmcache_source".to_string());
    }
    if !baseline.measured || !candidate.measured {
        reasons.push("paired_benchmark_not_measured".to_string());
    }
    if !baseline.native_prefix_cache_enabled || !candidate.native_prefix_cache_enabled {
        reasons.push("native_prefix_cache_baseline_required".to_string());
    }
    if baseline.sample_count < policy.minimum_samples_per_arm
        || candidate.sample_count < policy.minimum_samples_per_arm
    {
        reasons.push("insufficient_paired_samples".to_string());
    }

    let ttft_gain = gain_basis_points(baseline.median_ttft_micros, candidate.median_ttft_micros);
    let gpu_gain = gain_basis_points(baseline.gpu_prefill_millis, candidate.gpu_prefill_millis);
    let cost_gain = gain_basis_points(
        baseline.successful_task_cost_microusd,
        candidate.successful_task_cost_microusd,
    );
    if ttft_gain.is_none_or(|gain| gain < policy.minimum_ttft_gain_basis_points) {
        reasons.push("ttft_gain_not_meaningful".to_string());
    }
    if gpu_gain.is_none_or(|gain| gain < policy.minimum_gpu_prefill_gain_basis_points) {
        reasons.push("gpu_prefill_gain_not_meaningful".to_string());
    }
    if cost_gain.is_none_or(|gain| gain < policy.minimum_successful_task_cost_gain_basis_points) {
        reasons.push("successful_task_cost_gain_not_meaningful".to_string());
    }
    if baseline
        .task_success_rate_basis_points
        .saturating_sub(candidate.task_success_rate_basis_points)
        > policy.maximum_task_success_regression_basis_points
    {
        reasons.push("task_success_regression_exceeds_limit".to_string());
    }
    if candidate.operational_complexity_score > policy.maximum_operational_complexity_score {
        reasons.push("operational_complexity_exceeds_limit".to_string());
    }
    if candidate
        .operational_complexity_score
        .saturating_sub(baseline.operational_complexity_score)
        > policy.maximum_operational_complexity_increase
    {
        reasons.push("operational_complexity_increase_exceeds_limit".to_string());
    }

    let eligible = reasons.is_empty();
    if eligible {
        reasons.push("paired_benchmark_gate_passed".to_string());
        reasons.push("benchmark_only_no_live_promotion".to_string());
    }
    LmCachePromotionDecision {
        eligible_for_benchmark_only_promotion: eligible,
        eligible_for_live_promotion: false,
        ttft_gain_basis_points: ttft_gain,
        gpu_prefill_gain_basis_points: gpu_gain,
        successful_task_cost_gain_basis_points: cost_gain,
        reasons,
    }
}

fn gain_basis_points(baseline: u64, candidate: u64) -> Option<u16> {
    if baseline == 0 || candidate >= baseline {
        return Some(0).filter(|_| baseline != 0);
    }
    Some((((baseline - candidate) as u128 * 10_000) / baseline as u128).min(10_000) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> LmCacheBenchmarkEvidence {
        serde_json::from_str(include_str!(
            "../../benchmarks/fixtures/lmcache-promotion-evidence.json"
        ))
        .unwrap()
    }

    #[test]
    fn paired_fixture_passes_benchmark_only_but_never_live_promotion() {
        let decision = evaluate_lmcache_promotion(&fixture(), LmCachePromotionPolicy::default());
        assert!(decision.eligible_for_benchmark_only_promotion);
        assert!(!decision.eligible_for_live_promotion);
        assert_eq!(decision.ttft_gain_basis_points, Some(3_000));
        assert_eq!(decision.gpu_prefill_gain_basis_points, Some(2_000));
        assert_eq!(decision.successful_task_cost_gain_basis_points, Some(2_000));
    }

    #[test]
    fn native_prefix_cache_must_be_the_measured_baseline() {
        let mut evidence = fixture();
        evidence.native_prefix_only.native_prefix_cache_enabled = false;
        let decision = evaluate_lmcache_promotion(&evidence, LmCachePromotionPolicy::default());
        assert!(!decision.eligible_for_benchmark_only_promotion);
        assert!(decision
            .reasons
            .contains(&"native_prefix_cache_baseline_required".to_string()));
    }

    #[test]
    fn weak_gains_or_success_regression_fail_promotion() {
        let mut evidence = fixture();
        evidence.native_prefix_plus_lmcache.median_ttft_micros = 95_000;
        evidence
            .native_prefix_plus_lmcache
            .task_success_rate_basis_points = 9_700;
        let decision = evaluate_lmcache_promotion(&evidence, LmCachePromotionPolicy::default());
        assert!(!decision.eligible_for_benchmark_only_promotion);
        assert!(decision
            .reasons
            .contains(&"ttft_gain_not_meaningful".to_string()));
        assert!(decision
            .reasons
            .contains(&"task_success_regression_exceeds_limit".to_string()));
    }

    #[test]
    fn excessive_operational_complexity_blocks_otherwise_good_metrics() {
        let mut evidence = fixture();
        evidence
            .native_prefix_plus_lmcache
            .operational_complexity_score = 70;
        let decision = evaluate_lmcache_promotion(&evidence, LmCachePromotionPolicy::default());
        assert!(!decision.eligible_for_benchmark_only_promotion);
        assert!(decision
            .reasons
            .contains(&"operational_complexity_exceeds_limit".to_string()));
    }
}
