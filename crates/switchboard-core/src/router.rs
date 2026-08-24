//! Strict, provider-neutral endpoint route planning.
//!
//! The planner consumes only caller-supplied, content-free endpoint metadata.
//! It never resolves providers, sends traffic, starts processes, or substitutes
//! the requested model. Its output is a deterministic observe-only plan.

use std::collections::BTreeSet;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{ExecutionMode, PlanningStrategy};

pub const ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION: u32 = 1;
pub const MAX_ENDPOINT_CANDIDATES: usize = 64;
pub const MAX_REQUIRED_FEATURES: usize = 32;
pub const MAX_CANDIDATE_FEATURES: usize = 64;
pub const MAX_AVAILABLE_MODELS: usize = 128;
pub const MAX_ROUTER_IDENTIFIER_BYTES: usize = 128;
/// Largest integer that round-trips exactly through a JavaScript `Number`.
pub const MAX_ROUTER_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;
const MAX_DECISION_REASONS: usize = MAX_REQUIRED_FEATURES + 8;
const MAX_REASON_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointHealth {
    Healthy,
    Degraded,
    Unknown,
    Unhealthy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointPrivacy {
    Local,
    Lan,
    Remote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyRequirement {
    Any,
    PreferLocal,
    RequireLocal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointRouteRequest {
    pub requested_model: String,
    pub required_features: BTreeSet<String>,
    pub privacy: PrivacyRequirement,
    pub maximum_cost_microusd_per_million_input_tokens: Option<u64>,
    pub maximum_queue_latency_ms: Option<u64>,
    pub preferred_endpoint_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointRouteCandidate {
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointRoutePlanInput {
    pub schema_version: u32,
    pub request: EndpointRouteRequest,
    pub candidates: Vec<EndpointRouteCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointRank {
    pub preferred_penalty: u8,
    pub health_penalty: u8,
    pub privacy_penalty: u8,
    pub unknown_cost_penalty: u8,
    pub cost_microusd_per_million_input_tokens: Option<u64>,
    pub unknown_queue_penalty: u8,
    pub queue_latency_ms: Option<u64>,
    pub endpoint_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointCandidateExplanation {
    pub endpoint_id: String,
    pub eligible: bool,
    pub reasons: Vec<String>,
    pub rank: Option<EndpointRank>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointRouteDecision {
    pub selected_endpoint_id: Option<String>,
    /// Echoed unchanged to prove endpoint routing did not perform model routing.
    pub requested_model: String,
    pub explanations: Vec<EndpointCandidateExplanation>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointRoutePlan {
    pub contract_version: u32,
    pub strategy: PlanningStrategy,
    pub execution_mode: ExecutionMode,
    pub requested_model: String,
    pub actual_model: String,
    pub provider_traffic_enabled: bool,
    pub process_start_enabled: bool,
    pub endpoint: EndpointRouteDecision,
}

impl EndpointRoutePlanInput {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION {
            bail!("endpoint route plan schema is unsupported");
        }
        self.request.validate()?;
        if self.candidates.len() > MAX_ENDPOINT_CANDIDATES {
            bail!("endpoint route candidate count exceeds the supported limit");
        }

        let mut endpoint_ids = BTreeSet::new();
        for candidate in &self.candidates {
            candidate.validate()?;
            if !endpoint_ids.insert(candidate.id.as_str()) {
                bail!("endpoint route candidates contain a duplicate endpoint ID");
            }
        }
        Ok(())
    }
}

impl EndpointRouteRequest {
    pub fn validate(&self) -> Result<()> {
        validate_identifier(&self.requested_model, "requested model")?;
        validate_identifier_set(
            &self.required_features,
            MAX_REQUIRED_FEATURES,
            "required feature",
        )?;
        if let Some(preferred_endpoint_id) = &self.preferred_endpoint_id {
            validate_identifier(preferred_endpoint_id, "preferred endpoint ID")?;
        }
        validate_wire_integer(
            self.maximum_cost_microusd_per_million_input_tokens,
            "maximum cost",
        )?;
        validate_wire_integer(self.maximum_queue_latency_ms, "maximum queue latency")?;
        Ok(())
    }
}

impl EndpointRouteCandidate {
    pub fn validate(&self) -> Result<()> {
        validate_identifier(&self.id, "endpoint ID")?;
        validate_identifier_set(&self.features, MAX_CANDIDATE_FEATURES, "endpoint feature")?;
        validate_identifier_set(
            &self.available_models,
            MAX_AVAILABLE_MODELS,
            "available model",
        )?;
        validate_wire_integer(self.cost_microusd_per_million_input_tokens, "endpoint cost")?;
        validate_wire_integer(self.queue_latency_ms, "endpoint queue latency")?;
        Ok(())
    }
}

impl EndpointRank {
    fn validate(&self) -> Result<()> {
        validate_identifier(&self.endpoint_id, "rank endpoint ID")?;
        validate_wire_integer(self.cost_microusd_per_million_input_tokens, "rank cost")?;
        validate_wire_integer(self.queue_latency_ms, "rank queue latency")?;
        if self.unknown_cost_penalty
            != u8::from(self.cost_microusd_per_million_input_tokens.is_none())
            || self.unknown_queue_penalty != u8::from(self.queue_latency_ms.is_none())
        {
            bail!("endpoint rank unknown-value penalties are inconsistent");
        }
        Ok(())
    }
}

impl EndpointRouteDecision {
    pub fn validate(&self) -> Result<()> {
        validate_identifier(&self.requested_model, "decision requested model")?;
        if self.explanations.len() > MAX_ENDPOINT_CANDIDATES {
            bail!("endpoint decision explanation count exceeds the supported limit");
        }

        let mut previous_endpoint_id: Option<&str> = None;
        for explanation in &self.explanations {
            validate_identifier(&explanation.endpoint_id, "explanation endpoint ID")?;
            if previous_endpoint_id
                .is_some_and(|previous| previous >= explanation.endpoint_id.as_str())
            {
                bail!("endpoint decision explanations are not strictly ordered");
            }
            previous_endpoint_id = Some(&explanation.endpoint_id);
            validate_reasons(&explanation.reasons)?;

            match (&explanation.rank, explanation.eligible) {
                (Some(rank), true) => {
                    rank.validate()?;
                    if rank.endpoint_id != explanation.endpoint_id {
                        bail!("endpoint decision rank does not match its explanation");
                    }
                }
                (None, false) => {}
                _ => bail!("endpoint decision eligibility and rank are inconsistent"),
            }
        }

        let expected_selected_endpoint_id = self
            .explanations
            .iter()
            .filter_map(|explanation| {
                explanation
                    .rank
                    .as_ref()
                    .map(|rank| (rank, explanation.endpoint_id.as_str()))
            })
            .min_by(|(left, _), (right, _)| left.cmp(right))
            .map(|(_, endpoint_id)| endpoint_id);
        match (&self.selected_endpoint_id, expected_selected_endpoint_id) {
            (Some(selected_endpoint_id), Some(expected_endpoint_id)) => {
                validate_identifier(selected_endpoint_id, "selected endpoint ID")?;
                if selected_endpoint_id != expected_endpoint_id {
                    bail!("selected endpoint does not match the deterministic rank");
                }
                if self.reason != format!("selected_eligible_endpoint:{selected_endpoint_id}") {
                    bail!("selected endpoint reason is inconsistent");
                }
            }
            (None, None) => {
                if self.reason != "no_eligible_endpoint_no_automatic_fallback" {
                    bail!("no-endpoint decision is inconsistent");
                }
            }
            _ => bail!("selected endpoint is inconsistent with eligible endpoint ranks"),
        }
        Ok(())
    }
}

impl EndpointRoutePlan {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION {
            bail!("endpoint route plan contract is unsupported");
        }
        if self.strategy != PlanningStrategy::DeterministicEndpoint
            || self.execution_mode != ExecutionMode::ObserveOnly
            || self.provider_traffic_enabled
            || self.process_start_enabled
        {
            bail!("endpoint route plan violates the observe-only boundary");
        }
        validate_identifier(&self.requested_model, "plan requested model")?;
        if self.actual_model != self.requested_model
            || self.endpoint.requested_model != self.requested_model
        {
            bail!("endpoint route plan changed the requested model");
        }
        self.endpoint.validate()
    }
}

pub fn build_endpoint_route_plan(input: &EndpointRoutePlanInput) -> Result<EndpointRoutePlan> {
    input.validate()?;

    let mut explanations = input
        .candidates
        .iter()
        .map(|candidate| explain_candidate(&input.request, candidate))
        .collect::<Vec<_>>();
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
        .map(|(_, endpoint_id)| endpoint_id);
    let reason = match &selected_endpoint_id {
        Some(endpoint_id) => format!("selected_eligible_endpoint:{endpoint_id}"),
        None => "no_eligible_endpoint_no_automatic_fallback".to_string(),
    };
    let requested_model = input.request.requested_model.clone();
    let plan = EndpointRoutePlan {
        contract_version: ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION,
        strategy: PlanningStrategy::DeterministicEndpoint,
        execution_mode: ExecutionMode::ObserveOnly,
        requested_model: requested_model.clone(),
        actual_model: requested_model.clone(),
        provider_traffic_enabled: false,
        process_start_enabled: false,
        endpoint: EndpointRouteDecision {
            selected_endpoint_id,
            requested_model,
            explanations,
            reason,
        },
    };
    plan.validate()?;
    Ok(plan)
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
                blockers.push("queue_latency_above_limit".to_string());
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
        cost_microusd_per_million_input_tokens: candidate.cost_microusd_per_million_input_tokens,
        unknown_queue_penalty: u8::from(candidate.queue_latency_ms.is_none()),
        queue_latency_ms: candidate.queue_latency_ms,
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

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value != value.trim()
        || value.len() > MAX_ROUTER_IDENTIFIER_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        bail!("router {label} must be a bounded opaque identifier");
    }
    Ok(())
}

fn validate_identifier_set(values: &BTreeSet<String>, maximum: usize, label: &str) -> Result<()> {
    if values.len() > maximum {
        bail!("router {label} count exceeds the supported limit");
    }
    for value in values {
        validate_identifier(value, label)?;
    }
    Ok(())
}

fn validate_wire_integer(value: Option<u64>, label: &str) -> Result<()> {
    if value.is_some_and(|value| value > MAX_ROUTER_SAFE_INTEGER) {
        bail!("router {label} exceeds the JavaScript-safe integer limit");
    }
    Ok(())
}

fn validate_reasons(reasons: &[String]) -> Result<()> {
    if reasons.is_empty() || reasons.len() > MAX_DECISION_REASONS {
        bail!("endpoint decision reason count is invalid");
    }
    for reason in reasons {
        if reason.is_empty()
            || reason != reason.trim()
            || reason.len() > MAX_REASON_BYTES
            || reason.chars().any(char::is_control)
        {
            bail!("endpoint decision contains an invalid reason");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> EndpointRouteRequest {
        EndpointRouteRequest {
            requested_model: "exact-model".to_string(),
            required_features: BTreeSet::from(["streaming".to_string(), "tools".to_string()]),
            privacy: PrivacyRequirement::PreferLocal,
            maximum_cost_microusd_per_million_input_tokens: Some(10_000),
            maximum_queue_latency_ms: Some(100),
            preferred_endpoint_id: None,
        }
    }

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
            features: BTreeSet::from(["streaming".to_string(), "tools".to_string()]),
            available_models: BTreeSet::from(["exact-model".to_string()]),
        }
    }

    fn input(candidates: Vec<EndpointRouteCandidate>) -> EndpointRoutePlanInput {
        EndpointRoutePlanInput {
            schema_version: ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION,
            request: request(),
            candidates,
        }
    }

    #[test]
    fn plan_is_deterministic_and_independent_of_candidate_order() {
        let local = candidate("local", EndpointPrivacy::Local, 100, 20);
        let remote = candidate("remote", EndpointPrivacy::Remote, 10, 2);
        let first = build_endpoint_route_plan(&input(vec![remote.clone(), local.clone()]))
            .expect("first plan");
        let second = build_endpoint_route_plan(&input(vec![local, remote])).expect("second plan");

        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first plan"),
            serde_json::to_vec(&second).expect("serialize second plan")
        );
        assert_eq!(
            first.endpoint.selected_endpoint_id.as_deref(),
            Some("local")
        );
        assert_eq!(
            first
                .endpoint
                .explanations
                .iter()
                .map(|explanation| explanation.endpoint_id.as_str())
                .collect::<Vec<_>>(),
            vec!["local", "remote"]
        );
    }

    #[test]
    fn equal_ranks_use_endpoint_id_as_a_stable_tiebreaker() {
        let alpha = candidate("alpha", EndpointPrivacy::Local, 100, 20);
        let zulu = candidate("zulu", EndpointPrivacy::Local, 100, 20);
        let plan = build_endpoint_route_plan(&input(vec![zulu, alpha])).expect("plan");
        assert_eq!(plan.endpoint.selected_endpoint_id.as_deref(), Some("alpha"));
    }

    #[test]
    fn wire_visible_metrics_accept_the_javascript_safe_integer_maximum() {
        let mut input = input(vec![candidate(
            "maximum",
            EndpointPrivacy::Local,
            MAX_ROUTER_SAFE_INTEGER,
            MAX_ROUTER_SAFE_INTEGER,
        )]);
        input.request.maximum_cost_microusd_per_million_input_tokens =
            Some(MAX_ROUTER_SAFE_INTEGER);
        input.request.maximum_queue_latency_ms = Some(MAX_ROUTER_SAFE_INTEGER);

        let plan = build_endpoint_route_plan(&input).expect("maximum safe integer plan");
        let rank = plan.endpoint.explanations[0]
            .rank
            .as_ref()
            .expect("eligible rank");
        assert_eq!(
            rank.cost_microusd_per_million_input_tokens,
            Some(MAX_ROUTER_SAFE_INTEGER)
        );
        assert_eq!(rank.queue_latency_ms, Some(MAX_ROUTER_SAFE_INTEGER));

        let value = serde_json::to_value(rank).expect("rank JSON");
        assert_eq!(
            value["costMicrousdPerMillionInputTokens"].as_u64(),
            Some(MAX_ROUTER_SAFE_INTEGER)
        );
        assert_eq!(
            value["queueLatencyMs"].as_u64(),
            Some(MAX_ROUTER_SAFE_INTEGER)
        );
    }

    #[test]
    fn wire_visible_metrics_reject_values_above_the_javascript_safe_integer_maximum() {
        let above_maximum = MAX_ROUTER_SAFE_INTEGER + 1;

        let mut request_cost = input(Vec::new());
        request_cost
            .request
            .maximum_cost_microusd_per_million_input_tokens = Some(above_maximum);
        assert!(build_endpoint_route_plan(&request_cost).is_err());

        let mut request_queue = input(Vec::new());
        request_queue.request.maximum_queue_latency_ms = Some(above_maximum);
        assert!(build_endpoint_route_plan(&request_queue).is_err());

        let mut candidate_cost = input(vec![candidate(
            "candidate-cost",
            EndpointPrivacy::Local,
            above_maximum,
            1,
        )]);
        candidate_cost
            .request
            .maximum_cost_microusd_per_million_input_tokens = None;
        assert!(build_endpoint_route_plan(&candidate_cost).is_err());

        let mut candidate_queue = input(vec![candidate(
            "candidate-queue",
            EndpointPrivacy::Local,
            1,
            above_maximum,
        )]);
        candidate_queue.request.maximum_queue_latency_ms = None;
        assert!(build_endpoint_route_plan(&candidate_queue).is_err());

        let mut tampered = build_endpoint_route_plan(&input(vec![candidate(
            "tampered",
            EndpointPrivacy::Local,
            1,
            1,
        )]))
        .expect("valid plan before tampering");
        tampered.endpoint.explanations[0]
            .rank
            .as_mut()
            .expect("eligible rank")
            .queue_latency_ms = Some(above_maximum);
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn unknown_rank_metrics_are_explicit_nulls_without_changing_ordering() {
        let known = candidate("known", EndpointPrivacy::Local, 100, 20);
        let mut unknown = candidate("unknown", EndpointPrivacy::Local, 0, 0);
        unknown.cost_microusd_per_million_input_tokens = None;
        unknown.queue_latency_ms = None;

        let mut first_input = input(vec![unknown.clone(), known.clone()]);
        first_input
            .request
            .maximum_cost_microusd_per_million_input_tokens = None;
        first_input.request.maximum_queue_latency_ms = None;
        let mut second_input = input(vec![known, unknown]);
        second_input
            .request
            .maximum_cost_microusd_per_million_input_tokens = None;
        second_input.request.maximum_queue_latency_ms = None;

        let first = build_endpoint_route_plan(&first_input).expect("first unknown-metric plan");
        let second = build_endpoint_route_plan(&second_input).expect("second unknown-metric plan");
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).expect("first plan JSON"),
            serde_json::to_vec(&second).expect("second plan JSON")
        );
        assert_eq!(
            first.endpoint.selected_endpoint_id.as_deref(),
            Some("known")
        );

        let unknown_rank = first
            .endpoint
            .explanations
            .iter()
            .find(|explanation| explanation.endpoint_id == "unknown")
            .and_then(|explanation| explanation.rank.as_ref())
            .expect("unknown-metric rank");
        assert_eq!(unknown_rank.unknown_cost_penalty, 1);
        assert_eq!(unknown_rank.cost_microusd_per_million_input_tokens, None);
        assert_eq!(unknown_rank.unknown_queue_penalty, 1);
        assert_eq!(unknown_rank.queue_latency_ms, None);

        let value = serde_json::to_value(unknown_rank).expect("unknown rank JSON");
        assert!(value["costMicrousdPerMillionInputTokens"].is_null());
        assert!(value["queueLatencyMs"].is_null());
        assert!(!serde_json::to_string(&first)
            .expect("plan JSON")
            .contains(&u64::MAX.to_string()));
    }

    #[test]
    fn plan_is_observe_only_and_never_changes_the_requested_model() {
        let plan = build_endpoint_route_plan(&input(vec![candidate(
            "local",
            EndpointPrivacy::Local,
            100,
            20,
        )]))
        .expect("plan");

        assert_eq!(plan.contract_version, ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION);
        assert_eq!(plan.strategy, PlanningStrategy::DeterministicEndpoint);
        assert_eq!(plan.execution_mode, ExecutionMode::ObserveOnly);
        assert_eq!(plan.requested_model, "exact-model");
        assert_eq!(plan.actual_model, plan.requested_model);
        assert_eq!(plan.endpoint.requested_model, plan.requested_model);
        assert!(!plan.provider_traffic_enabled);
        assert!(!plan.process_start_enabled);
        plan.validate().expect("valid observe-only plan");
    }

    #[test]
    fn hard_gates_fail_closed_without_preference_bypass_or_fallback() {
        let mut preferred = candidate("preferred", EndpointPrivacy::Remote, 1, 1);
        preferred.verified = false;
        preferred.health = EndpointHealth::Unhealthy;
        preferred.available_models.clear();
        preferred.features.clear();
        preferred.cost_microusd_per_million_input_tokens = None;
        preferred.queue_latency_ms = None;
        let mut input = input(vec![preferred]);
        input.request.privacy = PrivacyRequirement::RequireLocal;
        input.request.preferred_endpoint_id = Some("preferred".to_string());

        let plan = build_endpoint_route_plan(&input).expect("fail-closed plan");
        assert_eq!(plan.endpoint.selected_endpoint_id, None);
        assert_eq!(
            plan.endpoint.reason,
            "no_eligible_endpoint_no_automatic_fallback"
        );
        let reasons = &plan.endpoint.explanations[0].reasons;
        for expected in [
            "endpoint_unverified",
            "endpoint_unhealthy",
            "requested_model_unavailable",
            "required_feature_unavailable:streaming",
            "required_feature_unavailable:tools",
            "privacy_requires_local_endpoint",
            "cost_unknown_under_bounded_policy",
            "queue_latency_unknown_under_bounded_policy",
        ] {
            assert!(reasons.iter().any(|reason| reason == expected));
        }
    }

    #[test]
    fn duplicate_candidates_and_unsupported_schema_are_rejected() {
        let duplicate = candidate("duplicate", EndpointPrivacy::Local, 1, 1);
        assert!(build_endpoint_route_plan(&input(vec![duplicate.clone(), duplicate])).is_err());

        let mut unsupported = input(Vec::new());
        unsupported.schema_version += 1;
        assert!(build_endpoint_route_plan(&unsupported).is_err());
    }

    #[test]
    fn identifiers_candidate_counts_and_feature_sets_are_bounded() {
        let candidates = (0..=MAX_ENDPOINT_CANDIDATES)
            .map(|index| candidate(&format!("endpoint-{index}"), EndpointPrivacy::Local, 1, 1))
            .collect();
        assert!(build_endpoint_route_plan(&input(candidates)).is_err());

        let mut overlong_model = input(Vec::new());
        overlong_model.request.requested_model = "m".repeat(MAX_ROUTER_IDENTIFIER_BYTES + 1);
        assert!(build_endpoint_route_plan(&overlong_model).is_err());

        let mut invalid_endpoint = input(vec![candidate(
            "bad endpoint",
            EndpointPrivacy::Local,
            1,
            1,
        )]);
        assert!(build_endpoint_route_plan(&invalid_endpoint).is_err());
        invalid_endpoint.candidates[0].id = "endpoint".to_string();
        invalid_endpoint.candidates[0].features = (0..=MAX_CANDIDATE_FEATURES)
            .map(|index| format!("feature-{index}"))
            .collect();
        assert!(build_endpoint_route_plan(&invalid_endpoint).is_err());

        let mut available_models = input(vec![candidate("endpoint", EndpointPrivacy::Local, 1, 1)]);
        available_models.candidates[0].available_models = (0..=MAX_AVAILABLE_MODELS)
            .map(|index| format!("model-{index}"))
            .collect();
        assert!(build_endpoint_route_plan(&available_models).is_err());

        let mut required = input(Vec::new());
        required.request.required_features = (0..=MAX_REQUIRED_FEATURES)
            .map(|index| format!("required-{index}"))
            .collect();
        assert!(build_endpoint_route_plan(&required).is_err());
    }

    #[test]
    fn unknown_fields_are_rejected_at_every_object_boundary() {
        let valid_input = input(vec![candidate("local", EndpointPrivacy::Local, 1, 1)]);
        let valid_plan = build_endpoint_route_plan(&valid_input).expect("plan");

        let mut input_value = serde_json::to_value(&valid_input).expect("input JSON");
        input_value["prompt"] = json!("must not be accepted");
        assert!(serde_json::from_value::<EndpointRoutePlanInput>(input_value).is_err());

        let mut request_value = serde_json::to_value(&valid_input.request).expect("request JSON");
        request_value["provider"] = json!("must not be accepted");
        assert!(serde_json::from_value::<EndpointRouteRequest>(request_value).is_err());

        let mut candidate_value =
            serde_json::to_value(&valid_input.candidates[0]).expect("candidate JSON");
        candidate_value["command"] = json!("must not be accepted");
        assert!(serde_json::from_value::<EndpointRouteCandidate>(candidate_value).is_err());

        let mut decision_value = serde_json::to_value(&valid_plan.endpoint).expect("decision JSON");
        decision_value["response"] = json!("must not be accepted");
        assert!(serde_json::from_value::<EndpointRouteDecision>(decision_value).is_err());

        let mut plan_value = serde_json::to_value(valid_plan).expect("plan JSON");
        plan_value["executionEnabled"] = json!(true);
        assert!(serde_json::from_value::<EndpointRoutePlan>(plan_value).is_err());
    }

    #[test]
    fn validation_rejects_tampered_execution_model_and_decision_bindings() {
        let mut plan = build_endpoint_route_plan(&input(vec![candidate(
            "local",
            EndpointPrivacy::Local,
            1,
            1,
        )]))
        .expect("plan");
        plan.provider_traffic_enabled = true;
        assert!(plan.validate().is_err());

        let mut plan = build_endpoint_route_plan(&input(vec![candidate(
            "local",
            EndpointPrivacy::Local,
            1,
            1,
        )]))
        .expect("plan");
        plan.actual_model = "substituted-model".to_string();
        assert!(plan.validate().is_err());

        let mut plan = build_endpoint_route_plan(&input(vec![candidate(
            "local",
            EndpointPrivacy::Local,
            1,
            1,
        )]))
        .expect("plan");
        plan.endpoint.selected_endpoint_id = Some("missing".to_string());
        plan.endpoint.reason = "selected_eligible_endpoint:missing".to_string();
        assert!(plan.validate().is_err());

        let mut plan = build_endpoint_route_plan(&input(vec![
            candidate("local", EndpointPrivacy::Local, 100, 20),
            candidate("remote", EndpointPrivacy::Remote, 1, 1),
        ]))
        .expect("plan");
        plan.endpoint.selected_endpoint_id = Some("remote".to_string());
        plan.endpoint.reason = "selected_eligible_endpoint:remote".to_string();
        assert!(plan.validate().is_err());
    }
}
