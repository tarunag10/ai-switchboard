//! Cache-aware compression recommendation with measured-success gates.
//!
//! This module recommends a profile; it never applies one. Unknown evidence
//! and incomplete benchmarks fall back to no compression.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompressionVariant {
    NoCompression,
    Normal,
    CacheSafe,
    Aggressive,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub(crate) enum ProviderPromptCacheEvidence {
    Unknown,
    ProviderDeclared,
    Measured {
        sample_count: u64,
        hit_rate_basis_points: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompressionBenchmarkResult {
    pub variant: CompressionVariant,
    pub measured: bool,
    pub sample_count: u64,
    pub agent_success_rate_basis_points: u16,
    pub relevant_fact_retention_basis_points: u16,
    pub wrong_omission_rate_basis_points: u16,
    pub input_tokens_saved_basis_points: u16,
    pub prompt_cache_hit_rate_basis_points: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FourVariantBenchmarkEvidence {
    pub evidence_version: String,
    pub deterministic_fixture_id: String,
    pub results: Vec<CompressionBenchmarkResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompressionBenchmarkGate {
    pub minimum_samples: u64,
    pub minimum_agent_success_rate_basis_points: u16,
    pub minimum_relevant_fact_retention_basis_points: u16,
    pub maximum_wrong_omission_rate_basis_points: u16,
}

impl Default for CompressionBenchmarkGate {
    fn default() -> Self {
        Self {
            minimum_samples: 30,
            minimum_agent_success_rate_basis_points: 9_800,
            minimum_relevant_fact_retention_basis_points: 9_900,
            maximum_wrong_omission_rate_basis_points: 100,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VariantGateResult {
    pub variant: CompressionVariant,
    pub eligible: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompressionRecommendationInput {
    pub prompt_cache_evidence: ProviderPromptCacheEvidence,
    pub aggressive_user_opt_in: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompressionRecommendation {
    pub selected_variant: CompressionVariant,
    pub reasons: Vec<String>,
    pub gate_results: Vec<VariantGateResult>,
}

pub(crate) fn evaluate_four_variant_gate(
    evidence: &FourVariantBenchmarkEvidence,
    gate: CompressionBenchmarkGate,
) -> Result<Vec<VariantGateResult>, String> {
    let variants: BTreeSet<_> = evidence
        .results
        .iter()
        .map(|result| result.variant)
        .collect();
    let required = BTreeSet::from([
        CompressionVariant::NoCompression,
        CompressionVariant::Normal,
        CompressionVariant::CacheSafe,
        CompressionVariant::Aggressive,
    ]);
    if evidence.results.len() != 4 || variants != required {
        return Err(
            "benchmark evidence must contain each of the four variants exactly once".to_string(),
        );
    }
    if evidence.evidence_version.trim().is_empty()
        || evidence.deterministic_fixture_id.trim().is_empty()
    {
        return Err("benchmark evidence requires version and deterministic fixture id".to_string());
    }

    Ok(evidence
        .results
        .iter()
        .map(|result| {
            let mut reasons = Vec::new();
            if !result.measured {
                reasons.push("success_not_measured".to_string());
            }
            if result.sample_count < gate.minimum_samples {
                reasons.push("insufficient_samples".to_string());
            }
            if result.agent_success_rate_basis_points < gate.minimum_agent_success_rate_basis_points
            {
                reasons.push("agent_success_below_gate".to_string());
            }
            if result.relevant_fact_retention_basis_points
                < gate.minimum_relevant_fact_retention_basis_points
            {
                reasons.push("fact_retention_below_gate".to_string());
            }
            if result.wrong_omission_rate_basis_points
                > gate.maximum_wrong_omission_rate_basis_points
            {
                reasons.push("wrong_omission_above_gate".to_string());
            }
            VariantGateResult {
                variant: result.variant,
                eligible: reasons.is_empty(),
                reasons,
            }
        })
        .collect())
}

pub(crate) fn recommend_compression_profile(
    input: &CompressionRecommendationInput,
    evidence: &FourVariantBenchmarkEvidence,
    gate: CompressionBenchmarkGate,
) -> CompressionRecommendation {
    let gate_results = match evaluate_four_variant_gate(evidence, gate) {
        Ok(results) => results,
        Err(reason) => {
            return CompressionRecommendation {
                selected_variant: CompressionVariant::NoCompression,
                reasons: vec!["invalid_benchmark_evidence".to_string(), reason],
                gate_results: Vec::new(),
            }
        }
    };
    let eligible: BTreeMap<_, _> = gate_results
        .iter()
        .map(|result| (result.variant, result.eligible))
        .collect();
    let passes = |variant| eligible.get(&variant).copied().unwrap_or(false);

    let (candidate, evidence_reason) = match input.prompt_cache_evidence {
        ProviderPromptCacheEvidence::Unknown => (
            CompressionVariant::Normal,
            "prompt_cache_evidence_unknown_use_normal".to_string(),
        ),
        ProviderPromptCacheEvidence::ProviderDeclared => (
            CompressionVariant::CacheSafe,
            "provider_declares_prompt_cache_preserve_prefix".to_string(),
        ),
        ProviderPromptCacheEvidence::Measured {
            sample_count,
            hit_rate_basis_points,
        } if sample_count >= gate.minimum_samples && hit_rate_basis_points >= 1_000 => (
            CompressionVariant::CacheSafe,
            "measured_prompt_cache_hits_preserve_prefix".to_string(),
        ),
        ProviderPromptCacheEvidence::Measured { .. } if input.aggressive_user_opt_in => (
            CompressionVariant::Aggressive,
            "low_measured_cache_value_with_aggressive_opt_in".to_string(),
        ),
        ProviderPromptCacheEvidence::Measured { .. } => (
            CompressionVariant::Normal,
            "low_or_insufficient_cache_evidence_use_normal".to_string(),
        ),
    };

    if candidate == CompressionVariant::Aggressive && !input.aggressive_user_opt_in {
        return conservative_fallback(gate_results, "aggressive_requires_explicit_user_opt_in");
    }
    if !passes(candidate) {
        return conservative_fallback(
            gate_results,
            &format!("recommended_variant_failed_measured_gate:{candidate:?}"),
        );
    }

    CompressionRecommendation {
        selected_variant: candidate,
        reasons: vec![evidence_reason, "measured_success_gate_passed".to_string()],
        gate_results,
    }
}

fn conservative_fallback(
    gate_results: Vec<VariantGateResult>,
    reason: &str,
) -> CompressionRecommendation {
    CompressionRecommendation {
        selected_variant: CompressionVariant::NoCompression,
        reasons: vec![
            reason.to_string(),
            "conservative_no_compression_fallback".to_string(),
        ],
        gate_results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> FourVariantBenchmarkEvidence {
        serde_json::from_str(include_str!(
            "../../benchmarks/fixtures/compression-four-variant-evidence.json"
        ))
        .unwrap()
    }

    #[test]
    fn deterministic_fixture_contains_exactly_four_variants_and_gates_aggressive() {
        let results =
            evaluate_four_variant_gate(&fixture(), CompressionBenchmarkGate::default()).unwrap();
        assert_eq!(results.len(), 4);
        assert!(
            results
                .iter()
                .find(|result| result.variant == CompressionVariant::Normal)
                .unwrap()
                .eligible
        );
        assert!(
            results
                .iter()
                .find(|result| result.variant == CompressionVariant::CacheSafe)
                .unwrap()
                .eligible
        );
        let aggressive = results
            .iter()
            .find(|result| result.variant == CompressionVariant::Aggressive)
            .unwrap();
        assert!(!aggressive.eligible);
        assert!(aggressive
            .reasons
            .contains(&"agent_success_below_gate".to_string()));
    }

    #[test]
    fn declared_or_measured_cache_evidence_recommends_cache_safe_only_after_gate() {
        for prompt_cache_evidence in [
            ProviderPromptCacheEvidence::ProviderDeclared,
            ProviderPromptCacheEvidence::Measured {
                sample_count: 80,
                hit_rate_basis_points: 7_200,
            },
        ] {
            let recommendation = recommend_compression_profile(
                &CompressionRecommendationInput {
                    prompt_cache_evidence,
                    aggressive_user_opt_in: false,
                },
                &fixture(),
                CompressionBenchmarkGate::default(),
            );
            assert_eq!(
                recommendation.selected_variant,
                CompressionVariant::CacheSafe
            );
        }
    }

    #[test]
    fn invalid_or_unmeasured_evidence_falls_back_to_no_compression() {
        let mut invalid = fixture();
        invalid.results.pop();
        let recommendation = recommend_compression_profile(
            &CompressionRecommendationInput {
                prompt_cache_evidence: ProviderPromptCacheEvidence::Unknown,
                aggressive_user_opt_in: false,
            },
            &invalid,
            CompressionBenchmarkGate::default(),
        );
        assert_eq!(
            recommendation.selected_variant,
            CompressionVariant::NoCompression
        );

        let mut unmeasured = fixture();
        unmeasured
            .results
            .iter_mut()
            .find(|result| result.variant == CompressionVariant::Normal)
            .unwrap()
            .measured = false;
        let recommendation = recommend_compression_profile(
            &CompressionRecommendationInput {
                prompt_cache_evidence: ProviderPromptCacheEvidence::Unknown,
                aggressive_user_opt_in: false,
            },
            &unmeasured,
            CompressionBenchmarkGate::default(),
        );
        assert_eq!(
            recommendation.selected_variant,
            CompressionVariant::NoCompression
        );
    }

    #[test]
    fn aggressive_requires_opt_in_and_passing_measured_evidence() {
        let mut evidence = fixture();
        let aggressive = evidence
            .results
            .iter_mut()
            .find(|result| result.variant == CompressionVariant::Aggressive)
            .unwrap();
        aggressive.agent_success_rate_basis_points = 9_900;
        aggressive.relevant_fact_retention_basis_points = 9_950;
        aggressive.wrong_omission_rate_basis_points = 50;
        let without_opt_in = recommend_compression_profile(
            &CompressionRecommendationInput {
                prompt_cache_evidence: ProviderPromptCacheEvidence::Measured {
                    sample_count: 80,
                    hit_rate_basis_points: 500,
                },
                aggressive_user_opt_in: false,
            },
            &evidence,
            CompressionBenchmarkGate::default(),
        );
        assert_eq!(without_opt_in.selected_variant, CompressionVariant::Normal);
        let with_opt_in = recommend_compression_profile(
            &CompressionRecommendationInput {
                prompt_cache_evidence: ProviderPromptCacheEvidence::Measured {
                    sample_count: 80,
                    hit_rate_basis_points: 500,
                },
                aggressive_user_opt_in: true,
            },
            &evidence,
            CompressionBenchmarkGate::default(),
        );
        assert_eq!(with_opt_in.selected_variant, CompressionVariant::Aggressive);
    }
}
