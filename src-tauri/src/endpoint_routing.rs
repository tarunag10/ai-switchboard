//! Explainable endpoint selection. Model selection is deliberately out of scope:
//! the requested model is an immutable hard requirement, never substituted.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EndpointHealth {
    Healthy,
    Degraded,
    Unknown,
    Unhealthy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EndpointPrivacy {
    Local,
    Lan,
    Remote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrivacyRequirement {
    Any,
    PreferLocal,
    RequireLocal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointRouteCandidate {
    pub id: String,
    pub enabled: bool,
    pub verified: bool,
    pub health: EndpointHealth,
    pub privacy: EndpointPrivacy,
    /// Estimated provider charge in micro-USD per million input tokens.
    pub cost_microusd_per_million_input_tokens: Option<u64>,
    pub queue_latency_ms: Option<u64>,
    pub features: BTreeSet<String>,
    pub available_models: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointRouteRequest {
    pub requested_model: String,
    pub required_features: BTreeSet<String>,
    pub privacy: PrivacyRequirement,
    pub maximum_cost_microusd_per_million_input_tokens: Option<u64>,
    pub maximum_queue_latency_ms: Option<u64>,
    pub preferred_endpoint_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointCandidateExplanation {
    pub endpoint_id: String,
    pub eligible: bool,
    pub reasons: Vec<String>,
    pub rank: Option<EndpointRank>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointRank {
    preferred_penalty: u8,
    health_penalty: u8,
    privacy_penalty: u8,
    unknown_cost_penalty: u8,
    cost_microusd_per_million_input_tokens: u64,
    unknown_queue_penalty: u8,
    queue_latency_ms: u64,
    endpoint_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndpointRouteDecision {
    pub selected_endpoint_id: Option<String>,
    /// Echoed unchanged to prove endpoint routing did not perform model routing.
    pub requested_model: String,
    pub explanations: Vec<EndpointCandidateExplanation>,
    pub reason: String,
}

pub(crate) fn decide_endpoint_route(
    request: &EndpointRouteRequest,
    candidates: &[EndpointRouteCandidate],
) -> EndpointRouteDecision {
    let mut explanations: Vec<_> = candidates
        .iter()
        .map(|candidate| explain_candidate(request, candidate))
        .collect();
    explanations.sort_by(|left, right| left.endpoint_id.cmp(&right.endpoint_id));

    let selected_endpoint_id = explanations
        .iter()
        .filter_map(|explanation| {
            explanation
                .rank
                .as_ref()
                .map(|rank| (rank, explanation.endpoint_id.clone()))
        })
        .min_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, id)| id);
    let reason = match &selected_endpoint_id {
        Some(id) => format!("selected_eligible_endpoint:{id}"),
        None => "no_eligible_endpoint_no_automatic_fallback".to_string(),
    };

    EndpointRouteDecision {
        selected_endpoint_id,
        requested_model: request.requested_model.clone(),
        explanations,
        reason,
    }
}

fn explain_candidate(
    request: &EndpointRouteRequest,
    candidate: &EndpointRouteCandidate,
) -> EndpointCandidateExplanation {
    let mut blockers = Vec::new();
    if !candidate.enabled {
        blockers.push("endpoint_disabled".to_string());
    }
    if !candidate.verified {
        blockers.push("endpoint_unverified".to_string());
    }
    match candidate.health {
        EndpointHealth::Unknown => blockers.push("health_unknown".to_string()),
        EndpointHealth::Unhealthy => blockers.push("endpoint_unhealthy".to_string()),
        EndpointHealth::Healthy | EndpointHealth::Degraded => {}
    }
    if !candidate
        .available_models
        .contains(&request.requested_model)
    {
        blockers.push("requested_model_unavailable".to_string());
    }
    for feature in request.required_features.difference(&candidate.features) {
        blockers.push(format!("required_feature_unavailable:{feature}"));
    }
    if request.privacy == PrivacyRequirement::RequireLocal
        && candidate.privacy != EndpointPrivacy::Local
    {
        blockers.push("privacy_requires_local_endpoint".to_string());
    }
    if let Some(maximum) = request.maximum_cost_microusd_per_million_input_tokens {
        match candidate.cost_microusd_per_million_input_tokens {
            Some(cost) if cost > maximum => blockers.push("cost_above_limit".to_string()),
            None => blockers.push("cost_unknown_under_bounded_policy".to_string()),
            _ => {}
        }
    }
    if let Some(maximum) = request.maximum_queue_latency_ms {
        match candidate.queue_latency_ms {
            Some(latency) if latency > maximum => {
                blockers.push("queue_latency_above_limit".to_string())
            }
            None => blockers.push("queue_latency_unknown_under_bounded_policy".to_string()),
            _ => {}
        }
    }
    if !blockers.is_empty() {
        return EndpointCandidateExplanation {
            endpoint_id: candidate.id.clone(),
            eligible: false,
            reasons: blockers,
            rank: None,
        };
    }

    let preferred_penalty = match &request.preferred_endpoint_id {
        Some(preferred) if preferred == &candidate.id => 0,
        Some(_) => 1,
        None => 0,
    };
    let health_penalty = u8::from(candidate.health == EndpointHealth::Degraded);
    let privacy_penalty = match request.privacy {
        PrivacyRequirement::PreferLocal => match candidate.privacy {
            EndpointPrivacy::Local => 0,
            EndpointPrivacy::Lan => 1,
            EndpointPrivacy::Remote => 2,
        },
        PrivacyRequirement::Any | PrivacyRequirement::RequireLocal => 0,
    };
    let rank = EndpointRank {
        preferred_penalty,
        health_penalty,
        privacy_penalty,
        unknown_cost_penalty: u8::from(candidate.cost_microusd_per_million_input_tokens.is_none()),
        cost_microusd_per_million_input_tokens: candidate
            .cost_microusd_per_million_input_tokens
            .unwrap_or(u64::MAX),
        unknown_queue_penalty: u8::from(candidate.queue_latency_ms.is_none()),
        queue_latency_ms: candidate.queue_latency_ms.unwrap_or(u64::MAX),
        endpoint_id: candidate.id.clone(),
    };
    EndpointCandidateExplanation {
        endpoint_id: candidate.id.clone(),
        eligible: true,
        reasons: vec![
            "health_acceptable".to_string(),
            "requested_model_available".to_string(),
            "required_features_available".to_string(),
            "privacy_policy_satisfied".to_string(),
            "cost_policy_satisfied".to_string(),
            "queue_latency_policy_satisfied".to_string(),
        ],
        rank: Some(rank),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        id: &str,
        privacy: EndpointPrivacy,
        cost: u64,
        queue: u64,
    ) -> EndpointRouteCandidate {
        EndpointRouteCandidate {
            id: id.to_string(),
            enabled: true,
            verified: true,
            health: EndpointHealth::Healthy,
            privacy,
            cost_microusd_per_million_input_tokens: Some(cost),
            queue_latency_ms: Some(queue),
            features: BTreeSet::from(["tools".to_string(), "streaming".to_string()]),
            available_models: BTreeSet::from(["exact-model".to_string()]),
        }
    }

    fn request() -> EndpointRouteRequest {
        EndpointRouteRequest {
            requested_model: "exact-model".to_string(),
            required_features: BTreeSet::from(["tools".to_string()]),
            privacy: PrivacyRequirement::PreferLocal,
            maximum_cost_microusd_per_million_input_tokens: Some(10_000),
            maximum_queue_latency_ms: Some(100),
            preferred_endpoint_id: None,
        }
    }

    #[test]
    fn selects_local_endpoint_without_changing_requested_model() {
        let decision = decide_endpoint_route(
            &request(),
            &[
                candidate("remote", EndpointPrivacy::Remote, 100, 2),
                candidate("local", EndpointPrivacy::Local, 0, 20),
            ],
        );
        assert_eq!(decision.selected_endpoint_id.as_deref(), Some("local"));
        assert_eq!(decision.requested_model, "exact-model");
    }

    #[test]
    fn fails_closed_with_explanations_when_requirements_are_not_proven() {
        let mut unavailable = candidate("unknown", EndpointPrivacy::Remote, 100, 2);
        unavailable.health = EndpointHealth::Unknown;
        unavailable.available_models.clear();
        unavailable.features.clear();
        let mut bounded = request();
        bounded.privacy = PrivacyRequirement::RequireLocal;
        let decision = decide_endpoint_route(&bounded, &[unavailable]);
        assert_eq!(decision.selected_endpoint_id, None);
        let reasons = &decision.explanations[0].reasons;
        assert!(reasons.contains(&"health_unknown".to_string()));
        assert!(reasons.contains(&"requested_model_unavailable".to_string()));
        assert!(reasons.contains(&"required_feature_unavailable:tools".to_string()));
        assert!(reasons.contains(&"privacy_requires_local_endpoint".to_string()));
    }

    #[test]
    fn preferred_endpoint_never_bypasses_hard_gates() {
        let mut preferred = candidate("preferred", EndpointPrivacy::Local, 0, 1);
        preferred.health = EndpointHealth::Unhealthy;
        let mut acceptable = candidate("acceptable", EndpointPrivacy::Remote, 10, 2);
        acceptable.health = EndpointHealth::Degraded;
        let mut input = request();
        input.preferred_endpoint_id = Some("preferred".to_string());
        let decision = decide_endpoint_route(&input, &[preferred, acceptable]);
        assert_eq!(decision.selected_endpoint_id.as_deref(), Some("acceptable"));
    }

    #[test]
    fn bounded_cost_and_queue_require_measured_values() {
        let mut unknown = candidate("unknown", EndpointPrivacy::Local, 0, 1);
        unknown.cost_microusd_per_million_input_tokens = None;
        unknown.queue_latency_ms = None;
        let decision = decide_endpoint_route(&request(), &[unknown]);
        assert_eq!(decision.selected_endpoint_id, None);
        assert_eq!(decision.explanations[0].reasons.len(), 2);
    }
}
