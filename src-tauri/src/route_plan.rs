//! Canonical, content-free route planning boundary.
//!
//! This composes the existing endpoint eligibility decision with the existing
//! model-routing promotion stage. It is a policy/benchmark contract only: it
//! does not send requests, translate bodies, retry, or change live proxy flow.

use serde::{Deserialize, Serialize};

use crate::endpoint_routing::{
    decide_endpoint_route, EndpointRouteCandidate, EndpointRouteDecision, EndpointRouteRequest,
};
use crate::optimization::model_routing::{
    decide_model_route_experiment, ModelRouteDecision, ModelRouteInput,
    ModelRoutingBenchmarkEvidence, ModelRoutingExperimentPolicy, ModelRoutingStage,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RoutePlanExecutionMode {
    ObserveOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RoutePlanStrategy {
    DeterministicEndpoint,
    ObserveOnlyShadow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoutePlan {
    pub strategy: RoutePlanStrategy,
    pub execution_mode: RoutePlanExecutionMode,
    pub requested_model: String,
    pub actual_model: String,
    pub proposed_model: Option<String>,
    pub endpoint: EndpointRouteDecision,
    pub model: Option<ModelRouteDecision>,
}

pub(crate) fn build_route_plan(
    endpoint_request: &EndpointRouteRequest,
    endpoint_candidates: &[EndpointRouteCandidate],
    model_input: Option<&ModelRouteInput>,
    model_policy: &ModelRoutingExperimentPolicy,
    user_approved: bool,
    evidence: Option<&ModelRoutingBenchmarkEvidence>,
) -> RoutePlan {
    let endpoint = decide_endpoint_route(endpoint_request, endpoint_candidates);
    let model = model_input
        .map(|input| decide_model_route_experiment(input, model_policy, user_approved, evidence));
    let actual_model = model
        .as_ref()
        .map(|decision| decision.actual_model.clone())
        .unwrap_or_else(|| endpoint_request.requested_model.clone());
    let proposed_model = model
        .as_ref()
        .map(|decision| decision.selected_model.clone());
    RoutePlan {
        strategy: RoutePlanStrategy::ObserveOnlyShadow,
        execution_mode: RoutePlanExecutionMode::ObserveOnly,
        requested_model: endpoint_request.requested_model.clone(),
        actual_model,
        proposed_model,
        endpoint,
        model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn request() -> EndpointRouteRequest {
        EndpointRouteRequest {
            requested_model: "fixture-model".to_string(),
            required_features: BTreeSet::from(["streaming".to_string()]),
            privacy: crate::endpoint_routing::PrivacyRequirement::RequireLocal,
            maximum_cost_microusd_per_million_input_tokens: None,
            maximum_queue_latency_ms: None,
            preferred_endpoint_id: None,
        }
    }

    fn candidate(id: &str) -> EndpointRouteCandidate {
        EndpointRouteCandidate {
            id: id.to_string(),
            enabled: true,
            verified: true,
            health: crate::endpoint_routing::EndpointHealth::Healthy,
            privacy: crate::endpoint_routing::EndpointPrivacy::Local,
            cost_microusd_per_million_input_tokens: Some(1),
            queue_latency_ms: Some(10),
            features: BTreeSet::from(["streaming".to_string()]),
            available_models: BTreeSet::from(["fixture-model".to_string()]),
        }
    }

    #[test]
    fn composes_endpoint_decision_without_enabling_execution() {
        let plan = build_route_plan(
            &request(),
            &[candidate("local")],
            None,
            &ModelRoutingExperimentPolicy::default(),
            false,
            None,
        );
        assert_eq!(plan.strategy, RoutePlanStrategy::ObserveOnlyShadow);
        assert_eq!(plan.execution_mode, RoutePlanExecutionMode::ObserveOnly);
        assert_eq!(plan.endpoint.selected_endpoint_id.as_deref(), Some("local"));
        assert_eq!(plan.actual_model, "fixture-model");
        assert!(plan.proposed_model.is_none());
    }

    #[test]
    fn preserves_fail_closed_no_eligible_endpoint() {
        let mut candidate = candidate("remote");
        candidate.privacy = crate::endpoint_routing::EndpointPrivacy::Remote;
        let model_input = ModelRouteInput {
            client: "fixture".to_string(),
            task: "implement a feature".to_string(),
            requested_model: "fixture-model".to_string(),
            cheap_model: "cheap-model".to_string(),
            capable_model: "capable-model".to_string(),
            enabled: true,
        };
        let mut policy = ModelRoutingExperimentPolicy::default();
        policy.stage = ModelRoutingStage::AutomaticAllowlisted;
        let plan = build_route_plan(
            &request(),
            &[candidate],
            Some(&model_input),
            &policy,
            false,
            None,
        );
        assert!(plan.endpoint.selected_endpoint_id.is_none());
        assert_eq!(
            plan.endpoint.reason,
            "no_eligible_endpoint_no_automatic_fallback"
        );
        assert_eq!(plan.actual_model, "fixture-model");
        assert_eq!(plan.proposed_model.as_deref(), Some("capable-model"));
        assert_eq!(
            plan.model.as_ref().map(|decision| decision.stage),
            Some(ModelRoutingStage::AutomaticAllowlisted)
        );
    }
}
