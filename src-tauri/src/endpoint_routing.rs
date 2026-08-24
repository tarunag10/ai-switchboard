//! Compatibility adapter for the shared, provider-neutral endpoint router.
//!
//! Model selection remains out of scope: the requested model is an immutable
//! hard requirement, never substituted. Valid inputs delegate entirely to
//! `switchboard-core`; invalid legacy inputs retain the existing infallible,
//! fail-closed no-selection shape.

use switchboard_core::router::{
    build_endpoint_route_plan, EndpointRoutePlanInput, ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION,
};

pub(crate) use switchboard_core::router::{
    EndpointHealth, EndpointPrivacy, EndpointRouteCandidate, EndpointRouteDecision,
    EndpointRouteRequest, PrivacyRequirement,
};

const NO_ELIGIBLE_ENDPOINT_REASON: &str = "no_eligible_endpoint_no_automatic_fallback";

pub(crate) fn decide_endpoint_route(
    request: &EndpointRouteRequest,
    candidates: &[EndpointRouteCandidate],
) -> EndpointRouteDecision {
    try_decide_endpoint_route(request, candidates)
        .unwrap_or_else(|_| invalid_input_decision(request))
}

fn try_decide_endpoint_route(
    request: &EndpointRouteRequest,
    candidates: &[EndpointRouteCandidate],
) -> anyhow::Result<EndpointRouteDecision> {
    let input = EndpointRoutePlanInput {
        schema_version: ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION,
        request: request.clone(),
        candidates: candidates.to_vec(),
    };
    build_endpoint_route_plan(&input).map(|plan| plan.endpoint)
}

fn invalid_input_decision(request: &EndpointRouteRequest) -> EndpointRouteDecision {
    EndpointRouteDecision {
        selected_endpoint_id: None,
        requested_model: request.requested_model.clone(),
        explanations: Vec::new(),
        reason: NO_ELIGIBLE_ENDPOINT_REASON.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::json;
    use switchboard_core::router::{MAX_ENDPOINT_CANDIDATES, MAX_ROUTER_SAFE_INTEGER};

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

    fn core_decision(
        request: &EndpointRouteRequest,
        candidates: &[EndpointRouteCandidate],
    ) -> EndpointRouteDecision {
        build_endpoint_route_plan(&EndpointRoutePlanInput {
            schema_version: ENDPOINT_ROUTE_PLAN_SCHEMA_VERSION,
            request: request.clone(),
            candidates: candidates.to_vec(),
        })
        .expect("valid core route plan")
        .endpoint
    }

    #[test]
    fn adapter_matches_core_decision_and_json_for_valid_input() {
        let candidates = [
            candidate("remote", EndpointPrivacy::Remote, 100, 2),
            candidate("local", EndpointPrivacy::Local, 0, 20),
        ];
        let adapter = decide_endpoint_route(&request(), &candidates);
        let core = core_decision(&request(), &candidates);

        assert_eq!(adapter, core);
        assert_eq!(
            serde_json::to_vec(&adapter).expect("adapter JSON"),
            serde_json::to_vec(&core).expect("core JSON")
        );
        assert_eq!(adapter.selected_endpoint_id.as_deref(), Some("local"));
        assert_eq!(adapter.requested_model, "exact-model");
        adapter.validate().expect("valid adapter decision");
    }

    #[test]
    fn adapter_output_is_deterministic_across_candidate_order() {
        let local = candidate("local", EndpointPrivacy::Local, 0, 20);
        let remote = candidate("remote", EndpointPrivacy::Remote, 100, 2);
        let first = decide_endpoint_route(&request(), &[remote.clone(), local.clone()]);
        let second = decide_endpoint_route(&request(), &[local, remote]);

        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).expect("first JSON"),
            serde_json::to_vec(&second).expect("second JSON")
        );
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

    #[test]
    fn unknown_rank_metrics_preserve_the_core_null_wire_shape() {
        let mut unbounded = request();
        unbounded.maximum_cost_microusd_per_million_input_tokens = None;
        unbounded.maximum_queue_latency_ms = None;
        let mut unknown = candidate("unknown", EndpointPrivacy::Local, 0, 0);
        unknown.cost_microusd_per_million_input_tokens = None;
        unknown.queue_latency_ms = None;

        let decision = decide_endpoint_route(&unbounded, &[unknown]);
        let rank = decision.explanations[0]
            .rank
            .as_ref()
            .expect("eligible unknown-metric rank");
        assert_eq!(rank.cost_microusd_per_million_input_tokens, None);
        assert_eq!(rank.queue_latency_ms, None);
        let value = serde_json::to_value(&decision).expect("decision JSON");
        assert!(value["explanations"][0]["rank"]["costMicrousdPerMillionInputTokens"].is_null());
        assert!(value["explanations"][0]["rank"]["queueLatencyMs"].is_null());
    }

    #[test]
    fn invalid_legacy_input_returns_a_valid_no_selection_decision() {
        let candidates = (0..=MAX_ENDPOINT_CANDIDATES)
            .map(|index| candidate(&format!("endpoint-{index}"), EndpointPrivacy::Local, 1, 1))
            .collect::<Vec<_>>();

        let decision = decide_endpoint_route(&request(), &candidates);
        assert_eq!(decision.selected_endpoint_id, None);
        assert!(decision.explanations.is_empty());
        assert_eq!(decision.reason, NO_ELIGIBLE_ENDPOINT_REASON);
        decision.validate().expect("valid fail-closed decision");
    }

    #[test]
    fn core_wire_validation_is_inherited_by_the_adapter_types() {
        let mut unknown = serde_json::to_value(request()).expect("request JSON");
        unknown["prompt"] = json!("must not be accepted");
        assert!(serde_json::from_value::<EndpointRouteRequest>(unknown).is_err());

        let mut unsafe_integer = request();
        unsafe_integer.maximum_queue_latency_ms = Some(MAX_ROUTER_SAFE_INTEGER + 1);
        let decision = decide_endpoint_route(&unsafe_integer, &[]);
        assert_eq!(decision.selected_endpoint_id, None);
        assert!(decision.explanations.is_empty());
    }
}
