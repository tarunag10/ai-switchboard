use crate::optimization;

#[tauri::command]
pub fn get_optimization_snapshot() -> optimization::OptimizationSnapshot {
    optimization::snapshot::build_optimization_snapshot()
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

/// Converts content-free completion metrics into validated redacted evidence
/// and persists it through the same telemetry boundary as imported evidence.
/// This does not change the routing decision or enable automatic routing.
#[tauri::command]
pub fn record_model_routing_completion(
    decision: optimization::model_routing::ModelRouteDecision,
    completion: optimization::model_routing::ModelRoutingCompletionEvidence,
) -> Result<(), String> {
    let observation = optimization::model_routing::observation_from_completed_route(
        &decision,
        completion,
    )?;
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

    use super::{record_model_routing_completion, record_model_routing_evidence};
    use crate::optimization::model_routing::{
        decide_model_route, ModelRouteInput, ModelRoutingCompletionEvidence,
        ModelRoutingEvidenceArm, ModelRoutingEvidenceObservation,
    };
    use crate::optimization::telemetry_store::export_model_routing_evidence;

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
    fn record_model_routing_completion_rejects_missing_quality_before_persistence() {
        let decision = decide_model_route(&ModelRouteInput {
            client: "claude_code".to_string(),
            task: "format this file".to_string(),
            requested_model: "frontier".to_string(),
            cheap_model: "fast/local".to_string(),
            capable_model: "frontier".to_string(),
            enabled: true,
        });
        let error = record_model_routing_completion(
            decision,
            ModelRoutingCompletionEvidence {
                run_id: "run-completion-1".to_string(),
                captured_at: "2026-08-21T00:00:00Z".to_string(),
                succeeded: true,
                successful_task_cost_microunits: Some(100),
                quality_score_bps: None,
                latency_ms: 10,
                follow_up_rework: Some(false),
            },
        )
        .expect_err("completion bridge must require explicit quality evidence");
        assert!(error.contains("explicit quality score"));
    }

    #[test]
    fn completion_harness_exports_one_pair_and_stays_observe_only() {
        let _guard = crate::optimization::telemetry::test_guard();
        let home = tempdir().expect("temp home");
        let previous_home = env::var_os("HOME");
        env::set_var("HOME", home.path());

        let baseline = decide_model_route(&ModelRouteInput {
            client: "claude_code".to_string(),
            task: "format this file".to_string(),
            requested_model: "frontier".to_string(),
            cheap_model: "fast/local".to_string(),
            capable_model: "frontier".to_string(),
            enabled: true,
        });
        // The production facade remains observe-only, so the harness uses a
        // redacted candidate arm decision without invoking an automatic route.
        let mut candidate = baseline.clone();
        candidate.actual_model = "fast/local".to_string();
        candidate.baseline_model = "frontier".to_string();
        candidate.candidate_model = "fast/local".to_string();
        assert!(baseline.observe_only && candidate.observe_only);

        let run_id = "completion-harness-run";
        let baseline_time = chrono::Utc::now().to_rfc3339();
        record_model_routing_completion(
            baseline.clone(),
            ModelRoutingCompletionEvidence {
                run_id: run_id.to_string(),
                captured_at: baseline_time.clone(),
                succeeded: true,
                successful_task_cost_microunits: Some(1_000),
                quality_score_bps: Some(9_800),
                latency_ms: 800,
                follow_up_rework: Some(false),
            },
        )
        .expect("baseline completion should persist");
        record_model_routing_completion(
            candidate,
            ModelRoutingCompletionEvidence {
                run_id: run_id.to_string(),
                captured_at: (chrono::Utc::now() + chrono::Duration::milliseconds(1)).to_rfc3339(),
                succeeded: true,
                successful_task_cost_microunits: Some(700),
                quality_score_bps: Some(9_800),
                latency_ms: 820,
                follow_up_rework: Some(false),
            },
        )
        .expect("candidate completion should persist");

        let artifact = export_model_routing_evidence(run_id, "formatting")
            .expect("completion harness should export evidence");
        assert_eq!(artifact.baseline.sample_count, 1);
        assert_eq!(artifact.candidate.sample_count, 1);
        assert!(!artifact.promotion_eligible);

        let duplicate_error = record_model_routing_completion(
            baseline,
            ModelRoutingCompletionEvidence {
                run_id: run_id.to_string(),
                captured_at: baseline_time,
                succeeded: true,
                successful_task_cost_microunits: Some(1_000),
                quality_score_bps: Some(9_800),
                latency_ms: 800,
                follow_up_rework: Some(false),
            },
        )
        .expect_err("duplicate completion must fail closed");
        assert!(duplicate_error.contains("duplicate"));

        match previous_home {
            Some(value) => env::set_var("HOME", value),
            None => env::remove_var("HOME"),
        }
    }
}
