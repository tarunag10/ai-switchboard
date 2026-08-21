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

#[tauri::command]
pub fn export_model_routing_evidence(
    run_id: String,
    task_class: String,
) -> Result<optimization::telemetry_store::ModelRoutingEvidenceArtifact, String> {
    optimization::telemetry_store::export_model_routing_evidence(&run_id, &task_class)
}

#[cfg(test)]
mod tests {
    use super::record_model_routing_evidence;
    use crate::optimization::model_routing::{
        ModelRoutingEvidenceArm, ModelRoutingEvidenceObservation,
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
}
