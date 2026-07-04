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
pub fn validate_model_routing(
) -> Result<optimization::model_routing_validation::ModelRoutingValidationReceipt, String> {
    optimization::model_routing_validation::validate_model_routing()
}
