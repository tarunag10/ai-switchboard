//! Offline semantic-cache experiment contract.
//!
//! Unlike an exact-response cache, lookup requires an explicit semantic
//! representation, cosine-similarity threshold, namespace isolation, task
//! compatibility, and fresh code evidence. The policy is fail-closed and does
//! not store or serve entries itself.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheNamespace {
    pub workspace_id: String,
    pub account_id: String,
    pub model_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SemanticRepresentation {
    pub encoder_id: String,
    pub encoder_version: String,
    pub provider_implementation_fingerprint: String,
    pub provider_verified: bool,
    /// Deterministic signed, quantized embedding components.
    pub quantized_embedding: Vec<i16>,
    pub source_fingerprint: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TurnKind {
    PlainText,
    ToolTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepositoryState {
    Stable,
    Changing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActionRisk {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskIntent {
    QuestionAnswer,
    Summarization,
    Classification,
    ArbitraryCodeGeneration,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SemanticTaskConstraints {
    pub task_family: String,
    pub intent: TaskIntent,
    pub turn_kind: TurnKind,
    pub repository_state: RepositoryState,
    pub action_risk: ActionRisk,
    pub deterministic: bool,
    /// Temperature multiplied by 1,000.
    pub temperature_milli: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeFreshnessEvidence {
    pub repository_revision: String,
    pub dependency_fingerprints: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SemanticCacheEntry {
    pub id: String,
    pub namespace: CacheNamespace,
    pub representation: SemanticRepresentation,
    pub task_constraints: SemanticTaskConstraints,
    pub code_freshness: CodeFreshnessEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SemanticCacheLookup {
    pub namespace: CacheNamespace,
    pub representation: SemanticRepresentation,
    pub task_constraints: SemanticTaskConstraints,
    pub current_code: CodeFreshnessEvidence,
    pub minimum_similarity_basis_points: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticCacheRejection {
    ToolTurn,
    ChangingRepository,
    HighRiskAction,
    ArbitraryCodeGeneration,
    NonDeterministicRequest,
    HighTemperatureRequest,
    InvalidSimilarityThreshold,
    InvalidSemanticRepresentation,
    NamespaceMismatch,
    TaskConstraintMismatch,
    StaleCode,
    BelowSimilarityThreshold,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CandidateEvaluation {
    pub entry_id: String,
    pub similarity_basis_points: Option<u16>,
    pub rejection: Option<SemanticCacheRejection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SemanticCacheDecision {
    pub selected_entry_id: Option<String>,
    pub hard_rejection: Option<SemanticCacheRejection>,
    pub candidates: Vec<CandidateEvaluation>,
}

pub(crate) fn evaluate_semantic_cache_lookup(
    lookup: &SemanticCacheLookup,
    entries: &[SemanticCacheEntry],
) -> SemanticCacheDecision {
    if let Some(rejection) = request_rejection(lookup) {
        return SemanticCacheDecision {
            selected_entry_id: None,
            hard_rejection: Some(rejection),
            candidates: Vec::new(),
        };
    }

    let mut candidates: Vec<_> = entries
        .iter()
        .map(|entry| evaluate_candidate(lookup, entry))
        .collect();
    candidates.sort_by(|left, right| left.entry_id.cmp(&right.entry_id));
    let selected_entry_id = candidates
        .iter()
        .filter(|candidate| candidate.rejection.is_none())
        .max_by(|left, right| {
            left.similarity_basis_points
                .cmp(&right.similarity_basis_points)
                .then_with(|| right.entry_id.cmp(&left.entry_id))
        })
        .map(|candidate| candidate.entry_id.clone());

    SemanticCacheDecision {
        selected_entry_id,
        hard_rejection: None,
        candidates,
    }
}

fn request_rejection(lookup: &SemanticCacheLookup) -> Option<SemanticCacheRejection> {
    let constraints = &lookup.task_constraints;
    if constraints.turn_kind == TurnKind::ToolTurn {
        return Some(SemanticCacheRejection::ToolTurn);
    }
    if constraints.repository_state == RepositoryState::Changing {
        return Some(SemanticCacheRejection::ChangingRepository);
    }
    if constraints.action_risk == ActionRisk::High {
        return Some(SemanticCacheRejection::HighRiskAction);
    }
    if constraints.intent == TaskIntent::ArbitraryCodeGeneration {
        return Some(SemanticCacheRejection::ArbitraryCodeGeneration);
    }
    if !constraints.deterministic {
        return Some(SemanticCacheRejection::NonDeterministicRequest);
    }
    if constraints.temperature_milli > 200 {
        return Some(SemanticCacheRejection::HighTemperatureRequest);
    }
    if !(9_000..=10_000).contains(&lookup.minimum_similarity_basis_points) {
        return Some(SemanticCacheRejection::InvalidSimilarityThreshold);
    }
    if !valid_representation(&lookup.representation) {
        return Some(SemanticCacheRejection::InvalidSemanticRepresentation);
    }
    None
}

fn evaluate_candidate(
    lookup: &SemanticCacheLookup,
    entry: &SemanticCacheEntry,
) -> CandidateEvaluation {
    let reject = |rejection| CandidateEvaluation {
        entry_id: entry.id.clone(),
        similarity_basis_points: None,
        rejection: Some(rejection),
    };
    if entry.namespace != lookup.namespace {
        return reject(SemanticCacheRejection::NamespaceMismatch);
    }
    if entry.task_constraints != lookup.task_constraints {
        return reject(SemanticCacheRejection::TaskConstraintMismatch);
    }
    if entry.code_freshness != lookup.current_code {
        return reject(SemanticCacheRejection::StaleCode);
    }
    if entry.representation.encoder_id != lookup.representation.encoder_id
        || entry.representation.encoder_version != lookup.representation.encoder_version
        || entry.representation.provider_implementation_fingerprint
            != lookup.representation.provider_implementation_fingerprint
        || !valid_representation(&entry.representation)
    {
        return reject(SemanticCacheRejection::InvalidSemanticRepresentation);
    }
    let Some(similarity) = cosine_similarity_basis_points(
        &lookup.representation.quantized_embedding,
        &entry.representation.quantized_embedding,
    ) else {
        return reject(SemanticCacheRejection::InvalidSemanticRepresentation);
    };
    if similarity < lookup.minimum_similarity_basis_points {
        return CandidateEvaluation {
            entry_id: entry.id.clone(),
            similarity_basis_points: Some(similarity),
            rejection: Some(SemanticCacheRejection::BelowSimilarityThreshold),
        };
    }
    CandidateEvaluation {
        entry_id: entry.id.clone(),
        similarity_basis_points: Some(similarity),
        rejection: None,
    }
}

fn valid_representation(representation: &SemanticRepresentation) -> bool {
    !representation.encoder_id.trim().is_empty()
        && !representation.encoder_version.trim().is_empty()
        && !representation
            .provider_implementation_fingerprint
            .trim()
            .is_empty()
        && representation.provider_verified
        && !representation.source_fingerprint.trim().is_empty()
        && representation.quantized_embedding.len() >= 3
        && representation
            .quantized_embedding
            .iter()
            .any(|value| *value != 0)
}

fn cosine_similarity_basis_points(left: &[i16], right: &[i16]) -> Option<u16> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let dot: f64 = left
        .iter()
        .zip(right)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum();
    let left_norm: f64 = left
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    let right_norm: f64 = right
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        return None;
    }
    Some(((dot / (left_norm * right_norm)).clamp(0.0, 1.0) * 10_000.0).round() as u16)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SemanticCacheQualityBenchmark {
    pub measured: bool,
    pub sample_count: u64,
    pub true_positive_hits: u64,
    pub false_positive_hits: u64,
    pub successful_cached_tasks: u64,
    pub unsafe_hits: u64,
}

pub(crate) fn quality_benchmark_passes(benchmark: &SemanticCacheQualityBenchmark) -> bool {
    if !benchmark.measured || benchmark.sample_count < 100 || benchmark.unsafe_hits != 0 {
        return false;
    }
    let total_hits = benchmark.true_positive_hits + benchmark.false_positive_hits;
    total_hits > 0
        && u128::from(benchmark.true_positive_hits) * 10_000 >= u128::from(total_hits) * 9_800
        && u128::from(benchmark.successful_cached_tasks) * 10_000
            >= u128::from(benchmark.true_positive_hits) * 9_800
}

pub(crate) fn semantic_experiment_promotion_ready(
    representation: &SemanticRepresentation,
    benchmark: &SemanticCacheQualityBenchmark,
) -> bool {
    valid_representation(representation) && quality_benchmark_passes(benchmark)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup() -> SemanticCacheLookup {
        SemanticCacheLookup {
            namespace: CacheNamespace {
                workspace_id: "workspace-a".into(),
                account_id: "account-a".into(),
                model_id: "model-a".into(),
            },
            representation: SemanticRepresentation {
                encoder_id: "encoder".into(),
                encoder_version: "1".into(),
                provider_implementation_fingerprint: "sha256:provider".into(),
                provider_verified: true,
                quantized_embedding: vec![100, 50, 25],
                source_fingerprint: "query".into(),
            },
            task_constraints: SemanticTaskConstraints {
                task_family: "explain".into(),
                intent: TaskIntent::QuestionAnswer,
                turn_kind: TurnKind::PlainText,
                repository_state: RepositoryState::Stable,
                action_risk: ActionRisk::Low,
                deterministic: true,
                temperature_milli: 0,
            },
            current_code: CodeFreshnessEvidence {
                repository_revision: "abc".into(),
                dependency_fingerprints: BTreeMap::from([("src/lib.rs".into(), "hash-a".into())]),
            },
            minimum_similarity_basis_points: 9_500,
        }
    }

    fn entry() -> SemanticCacheEntry {
        let request = lookup();
        SemanticCacheEntry {
            id: "entry-a".into(),
            namespace: request.namespace,
            representation: SemanticRepresentation {
                source_fingerprint: "cached".into(),
                ..request.representation
            },
            task_constraints: request.task_constraints,
            code_freshness: request.current_code,
        }
    }

    #[test]
    fn hits_only_with_explicit_semantic_and_isolation_contract() {
        let decision = evaluate_semantic_cache_lookup(&lookup(), &[entry()]);
        assert_eq!(decision.selected_entry_id.as_deref(), Some("entry-a"));
        assert_eq!(decision.candidates[0].similarity_basis_points, Some(10_000));
    }

    #[test]
    fn namespace_and_stale_code_are_rejected_before_similarity_can_hit() {
        let mut foreign = entry();
        foreign.namespace.account_id = "other-account".into();
        let mut stale = entry();
        stale.id = "stale".into();
        stale.code_freshness.repository_revision = "old".into();
        let decision = evaluate_semantic_cache_lookup(&lookup(), &[foreign, stale]);
        assert_eq!(decision.selected_entry_id, None);
        assert!(decision.candidates.iter().any(
            |candidate| candidate.rejection == Some(SemanticCacheRejection::NamespaceMismatch)
        ));
        assert!(decision
            .candidates
            .iter()
            .any(|candidate| candidate.rejection == Some(SemanticCacheRejection::StaleCode)));
    }

    #[test]
    fn different_provider_implementation_fingerprint_is_rejected() {
        let mut incompatible = entry();
        incompatible
            .representation
            .provider_implementation_fingerprint = "sha256:different-provider-build".into();

        let decision = evaluate_semantic_cache_lookup(&lookup(), &[incompatible]);

        assert_eq!(decision.selected_entry_id, None);
        assert_eq!(
            decision.candidates[0].rejection,
            Some(SemanticCacheRejection::InvalidSemanticRepresentation)
        );
    }

    #[test]
    fn hard_rejects_unsafe_and_nondeterministic_request_classes() {
        let cases: [(fn(&mut SemanticCacheLookup), SemanticCacheRejection); 6] = [
            (
                |value: &mut SemanticCacheLookup| {
                    value.task_constraints.turn_kind = TurnKind::ToolTurn
                },
                SemanticCacheRejection::ToolTurn,
            ),
            (
                |value: &mut SemanticCacheLookup| {
                    value.task_constraints.repository_state = RepositoryState::Changing
                },
                SemanticCacheRejection::ChangingRepository,
            ),
            (
                |value: &mut SemanticCacheLookup| {
                    value.task_constraints.action_risk = ActionRisk::High
                },
                SemanticCacheRejection::HighRiskAction,
            ),
            (
                |value: &mut SemanticCacheLookup| {
                    value.task_constraints.intent = TaskIntent::ArbitraryCodeGeneration
                },
                SemanticCacheRejection::ArbitraryCodeGeneration,
            ),
            (
                |value: &mut SemanticCacheLookup| value.task_constraints.deterministic = false,
                SemanticCacheRejection::NonDeterministicRequest,
            ),
            (
                |value: &mut SemanticCacheLookup| value.task_constraints.temperature_milli = 700,
                SemanticCacheRejection::HighTemperatureRequest,
            ),
        ];
        for (mutate, expected) in cases {
            let mut request = lookup();
            mutate(&mut request);
            assert_eq!(
                evaluate_semantic_cache_lookup(&request, &[entry()]).hard_rejection,
                Some(expected)
            );
        }
    }

    #[test]
    fn quality_benchmark_requires_precision_task_success_and_zero_unsafe_hits() {
        let passing: SemanticCacheQualityBenchmark = serde_json::from_str(include_str!(
            "../../benchmarks/fixtures/semantic-cache-quality-evidence.json"
        ))
        .unwrap();
        assert!(quality_benchmark_passes(&passing));
        let mut unsafe_benchmark = passing;
        unsafe_benchmark.unsafe_hits = 1;
        assert!(!quality_benchmark_passes(&unsafe_benchmark));
    }

    #[test]
    fn promotion_requires_a_verified_representation_provider() {
        let benchmark: SemanticCacheQualityBenchmark = serde_json::from_str(include_str!(
            "../../benchmarks/fixtures/semantic-cache-quality-evidence.json"
        ))
        .unwrap();
        let mut representation = lookup().representation;
        assert!(semantic_experiment_promotion_ready(
            &representation,
            &benchmark
        ));
        representation.provider_verified = false;
        assert!(!semantic_experiment_promotion_ready(
            &representation,
            &benchmark
        ));
    }
}
