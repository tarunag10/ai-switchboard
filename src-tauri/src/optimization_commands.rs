use crate::optimization;
use crate::state::AppState;
use chrono::Utc;
use std::time::Instant;
use tauri::State;

#[tauri::command]
pub fn get_optimization_snapshot() -> optimization::OptimizationSnapshot {
    optimization::snapshot::build_optimization_snapshot()
}

/// Returns bounded, content-free proxy transport observations. This is a
/// diagnostic surface only; it is not model-routing quality or cost evidence.
#[tauri::command]
pub fn get_transport_observations() -> Vec<crate::transport_observations::TransportObservation> {
    crate::transport_observations::global().snapshot()
}

#[tauri::command]
pub fn run_preemptive_compaction() -> optimization::compaction_action::PreemptiveCompactionReceipt {
    optimization::compaction_action::run_preemptive_compaction()
}

#[tauri::command]
pub fn get_optimization_action_policy() -> optimization::action_policy::OptimizationActionPolicy {
    optimization::action_policy::load_action_policy()
}

#[tauri::command]
pub fn set_optimization_action_policy(
    policy: optimization::action_policy::OptimizationActionPolicy,
) -> Result<optimization::action_policy::OptimizationActionPolicy, String> {
    optimization::action_policy::save_action_policy(&policy)
}

#[tauri::command]
pub fn get_model_routing_experiment_policy(
) -> optimization::model_routing::ModelRoutingExperimentPolicy {
    optimization::model_routing::load_model_routing_experiment_policy()
}

#[tauri::command]
pub fn set_model_routing_experiment_policy(
    policy: optimization::model_routing::ModelRoutingExperimentPolicy,
) -> Result<optimization::model_routing::ModelRoutingExperimentPolicy, String> {
    optimization::model_routing::save_model_routing_experiment_policy(&policy)
}

#[tauri::command]
pub fn validate_model_routing(
) -> Result<optimization::model_routing_validation::ModelRoutingValidationReceipt, String> {
    optimization::model_routing_validation::validate_model_routing()
}

/// Records only redacted, task-class-scoped routing metrics. Prompts and
/// responses never cross this command boundary.
#[tauri::command]
pub fn record_model_routing_evidence(
    observation: optimization::model_routing::ModelRoutingEvidenceObservation,
) -> Result<(), String> {
    optimization::telemetry_store::record_model_routing_evidence(&observation)
}

/// Issues a native-bound completion capability. Routing remains observe-only;
/// the caller receives the proposal for display/use, but cannot forge the
/// decision accepted by the completion command.
#[tauri::command]
pub fn issue_model_routing_completion_handle(
    input: optimization::model_routing::ModelRouteInput,
    state: State<'_, AppState>,
) -> Result<optimization::model_routing::ModelRoutingCompletionHandle, String> {
    issue_model_routing_completion_handle_for_state(input, &state)
}

fn issue_model_routing_completion_handle_for_state(
    input: optimization::model_routing::ModelRouteInput,
    state: &AppState,
) -> Result<optimization::model_routing::ModelRoutingCompletionHandle, String> {
    optimization::model_routing::validate_completion_handle_input(&input)?;
    let policy = optimization::model_routing::load_model_routing_experiment_policy();
    let decision = optimization::model_routing::decide_model_route_experiment(
        &input, &policy, false, None,
    );
    if !decision.observe_only || decision.actual_model != input.requested_model {
        return Err("model-routing completion handles require observe-only routing".to_string());
    }
    let now = Utc::now();
    let (handle, pending) =
        optimization::model_routing::new_completion_handle(decision, now);
    let mut handles = state.model_routing_completion_handles.lock();
    handles.retain(|_, value| value.expires_monotonic > Instant::now());
    if handles.len() >= optimization::model_routing::MAX_PENDING_MODEL_ROUTING_COMPLETION_HANDLES {
        return Err("too many pending model-routing completion handles".to_string());
    }
    handles.insert(handle.handle_id.clone(), pending);
    Ok(handle)
}

/// Consumes a native-issued capability before persisting content-free
/// completion metrics. A consumed, expired, or unknown handle cannot be used
/// again, even if persistence later fails.
#[tauri::command]
pub fn complete_model_routing_completion(
    handle_id: String,
    metrics: optimization::model_routing::ModelRoutingCompletionMetrics,
    state: State<'_, AppState>,
) -> Result<(), String> {
    complete_model_routing_completion_for_state(handle_id, metrics, &state)
}

fn complete_model_routing_completion_for_state(
    handle_id: String,
    metrics: optimization::model_routing::ModelRoutingCompletionMetrics,
    state: &AppState,
) -> Result<(), String> {
    let now = Utc::now();
    let handle_id = handle_id.trim();
    if handle_id.is_empty() || handle_id.len() > 128 {
        return Err("model-routing completion handle is invalid".to_string());
    }
    let pending = {
        let handles = state.model_routing_completion_handles.lock();
        handles
            .get(handle_id)
            .filter(|value| value.expires_monotonic > Instant::now())
            .cloned()
            .ok_or_else(|| "model-routing completion handle is unknown, expired, or already consumed".to_string())?
    };
    let completion = optimization::model_routing::ModelRoutingCompletionEvidence {
        run_id: pending.run_id.clone(),
        captured_at: now.to_rfc3339(),
        succeeded: metrics.succeeded,
        successful_task_cost_microunits: metrics.successful_task_cost_microunits,
        quality_score_bps: metrics.quality_score_bps,
        latency_ms: metrics.latency_ms,
        follow_up_rework: metrics.follow_up_rework,
    };
    let observation = optimization::model_routing::observation_from_completed_route(
        &pending.decision,
        completion,
    )?;
    {
        let mut handles = state.model_routing_completion_handles.lock();
        let now_monotonic = Instant::now();
        handles.retain(|_, value| value.expires_monotonic > now_monotonic);
        handles
            .remove(handle_id)
            .ok_or_else(|| "model-routing completion handle is unknown, expired, or already consumed".to_string())?
        ;
    }
    optimization::telemetry_store::record_model_routing_evidence(&observation)
}

#[tauri::command]
pub fn export_model_routing_evidence(
    run_id: String,
    task_class: String,
) -> Result<optimization::telemetry_store::ModelRoutingEvidenceArtifact, String> {
    optimization::telemetry_store::export_model_routing_evidence(&run_id, &task_class)
}

#[cfg(test)]
mod tests {
    use std::env;

    use tempfile::tempdir;

    use super::{
        complete_model_routing_completion_for_state, export_model_routing_evidence,
        issue_model_routing_completion_handle_for_state, record_model_routing_evidence,
    };
    use crate::optimization::model_routing::{
        ModelRouteInput, ModelRoutingCompletionMetrics, ModelRoutingEvidenceArm,
        ModelRoutingEvidenceObservation,
    };

    #[test]
    fn record_model_routing_evidence_propagates_validation_errors() {
        let result = record_model_routing_evidence(ModelRoutingEvidenceObservation {
            run_id: String::new(),
            captured_at: "2026-08-21T00:00:00Z".to_string(),
            task_class: "formatting".to_string(),
            arm: ModelRoutingEvidenceArm::Baseline,
            baseline_model: "baseline".to_string(),
            candidate_model: "candidate".to_string(),
            succeeded: false,
            successful_task_cost_microunits: None,
            quality_score_bps: 0,
            latency_ms: 1,
            follow_up_rework: false,
        });

        let error = result.expect_err("invalid evidence must not report success");
        assert!(error.contains("invalid redacted model-routing evidence"));
    }

    #[test]
    fn completion_handle_is_observe_only_and_one_shot() {
        let _guard = crate::optimization::telemetry::test_guard();
        let home = tempdir().expect("temp home");
        let previous_home = env::var_os("HOME");
        env::set_var("HOME", home.path());
        let state = crate::state::AppState::new_in(home.path().join("state"))
            .expect("app state");
        let handle = issue_model_routing_completion_handle_for_state(ModelRouteInput {
            client: "claude_code".to_string(),
            task: "format this file".to_string(),
            requested_model: "frontier".to_string(),
            cheap_model: "fast/local".to_string(),
            capable_model: "frontier".to_string(),
            enabled: true,
        }, &state)
        .expect("observe-only handle should issue");
        assert!(handle.decision.observe_only);
        assert_eq!(handle.decision.actual_model, "frontier");
        let invalid_metrics = complete_model_routing_completion_for_state(
            handle.handle_id.clone(),
            ModelRoutingCompletionMetrics {
                succeeded: true,
                successful_task_cost_microunits: Some(1_000),
                quality_score_bps: None,
                latency_ms: 800,
                follow_up_rework: Some(false),
            },
            &state,
        )
        .expect_err("invalid metrics must fail before consuming the handle");
        assert!(invalid_metrics.contains("explicit quality score"));
        complete_model_routing_completion_for_state(
            handle.handle_id.clone(),
            ModelRoutingCompletionMetrics {
                succeeded: true,
                successful_task_cost_microunits: Some(1_000),
                quality_score_bps: Some(9_800),
                latency_ms: 800,
                follow_up_rework: Some(false),
            },
            &state,
        )
        .expect("completion should persist");
        record_model_routing_evidence(ModelRoutingEvidenceObservation {
            run_id: handle.run_id.clone(),
            captured_at: chrono::Utc::now().to_rfc3339(),
            task_class: "formatting".to_string(),
            arm: ModelRoutingEvidenceArm::Candidate,
            baseline_model: "frontier".to_string(),
            candidate_model: "fast/local".to_string(),
            succeeded: true,
            successful_task_cost_microunits: Some(700),
            quality_score_bps: 9_700,
            latency_ms: 820,
            follow_up_rework: false,
        })
        .expect("paired candidate evidence should persist under the native run ID");
        let artifact = export_model_routing_evidence(handle.run_id.clone(), "formatting".to_string())
            .expect("native-issued run ID should export its evidence");
        assert_eq!(artifact.provenance.run_id, handle.run_id);
        assert_eq!(artifact.provenance.task_class, "formatting");
        assert!(!artifact.promotion_eligible);
        let duplicate_error = complete_model_routing_completion_for_state(
            handle.handle_id,
            ModelRoutingCompletionMetrics {
                succeeded: true,
                successful_task_cost_microunits: Some(1_000),
                quality_score_bps: Some(9_800),
                latency_ms: 800,
                follow_up_rework: Some(false),
            },
            &state,
        )
        .expect_err("consumed handle must not be reusable");
        assert!(duplicate_error.contains("unknown, expired, or already consumed"));

        let expired_handle = issue_model_routing_completion_handle_for_state(
            ModelRouteInput {
                client: "claude_code".to_string(),
                task: "format this file".to_string(),
                requested_model: "frontier".to_string(),
                cheap_model: "fast/local".to_string(),
                capable_model: "frontier".to_string(),
                enabled: true,
            },
            &state,
        )
        .expect("second handle should issue");
        state
            .model_routing_completion_handles
            .lock()
            .get_mut(&expired_handle.handle_id)
            .expect("pending handle")
            .expires_monotonic = std::time::Instant::now() - std::time::Duration::from_secs(1);
        let expired_error = complete_model_routing_completion_for_state(
            expired_handle.handle_id.clone(),
            ModelRoutingCompletionMetrics {
                succeeded: true,
                successful_task_cost_microunits: Some(1_000),
                quality_score_bps: Some(9_800),
                latency_ms: 800,
                follow_up_rework: Some(false),
            },
            &state,
        )
        .expect_err("expired handles must fail closed");
        assert!(expired_error.contains("unknown, expired, or already consumed"));
        let replacement_handle = issue_model_routing_completion_handle_for_state(
            ModelRouteInput {
                client: "claude_code".to_string(),
                task: "format this file".to_string(),
                requested_model: "frontier".to_string(),
                cheap_model: "fast/local".to_string(),
                capable_model: "frontier".to_string(),
                enabled: true,
            },
            &state,
        )
        .expect("monotonically expired handle should be pruned before replacement");
        assert!(!state
            .model_routing_completion_handles
            .lock()
            .contains_key(&expired_handle.handle_id));
        assert!(state
            .model_routing_completion_handles
            .lock()
            .contains_key(&replacement_handle.handle_id));


        match previous_home {
            Some(value) => env::set_var("HOME", value),
            None => env::remove_var("HOME"),
        }
    }
}
