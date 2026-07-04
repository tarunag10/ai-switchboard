use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::storage::app_data_dir;

use super::action_policy::load_action_policy;
use super::model_routing::{decide_model_route, ModelRouteInput};

const RECEIPT_FILE: &str = "model-routing-validation-receipt.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingValidationReceipt {
    pub(crate) generated_at: String,
    pub(crate) policy_enabled: bool,
    pub(crate) checks: Vec<ModelRoutingValidationCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingValidationCheck {
    pub(crate) client: String,
    pub(crate) task: String,
    pub(crate) requested_model: String,
    pub(crate) selected_model: String,
    pub(crate) fallback_model: String,
    pub(crate) status: String,
    pub(crate) reason: String,
    pub(crate) observe_only: bool,
}

pub(crate) fn validate_model_routing() -> Result<ModelRoutingValidationReceipt, String> {
    let policy = load_action_policy();
    let checks = validation_inputs(policy.model_routing_enabled)
        .into_iter()
        .map(|input| {
            let decision = decide_model_route(&input);
            let status = if decision.observe_only {
                "observed"
            } else if decision.selected_model == input.cheap_model {
                "passed"
            } else {
                "capable"
            };
            ModelRoutingValidationCheck {
                client: input.client,
                task: input.task,
                requested_model: input.requested_model,
                selected_model: decision.selected_model,
                fallback_model: input.capable_model,
                status: status.to_string(),
                reason: decision.reason,
                observe_only: decision.observe_only,
            }
        })
        .collect();

    let receipt = ModelRoutingValidationReceipt {
        generated_at: Utc::now().to_rfc3339(),
        policy_enabled: policy.model_routing_enabled,
        checks,
    };
    save_receipt(&receipt)?;
    Ok(receipt)
}

fn validation_inputs(enabled: bool) -> Vec<ModelRouteInput> {
    [
        (
            "Claude Code",
            "lint fix and commit message",
            "claude-sonnet",
        ),
        ("Codex", "rename variable and summarize diff", "gpt-5-codex"),
        (
            "Gemini CLI",
            "format imports and explain failure",
            "gemini-pro",
        ),
        ("OpenCode", "test failure triage", "opencode-pro"),
        ("Windsurf", "small refactor summary", "windsurf-pro"),
        ("Zed", "commit message polish", "zed-pro"),
    ]
    .into_iter()
    .map(|(client, task, requested_model)| ModelRouteInput {
        client: client.to_string(),
        task: task.to_string(),
        requested_model: requested_model.to_string(),
        cheap_model: "fast/local".to_string(),
        capable_model: requested_model.to_string(),
        enabled,
    })
    .collect()
}

fn save_receipt(receipt: &ModelRoutingValidationReceipt) -> Result<(), String> {
    let dir = app_data_dir().join("config");
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("creating {} failed: {err}", dir.display()))?;
    let path = dir.join(RECEIPT_FILE);
    let bytes = serde_json::to_vec_pretty(receipt).map_err(|err| err.to_string())?;
    std::fs::write(&path, bytes).map_err(|err| format!("writing {} failed: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::validation_inputs;
    use super::*;

    #[test]
    fn validation_matrix_covers_managed_clients() {
        let inputs = validation_inputs(true);
        assert_eq!(inputs.len(), 6);
        assert!(inputs.iter().any(|input| input.client == "Codex"));
        assert!(inputs.iter().all(|input| input.enabled));
    }

    #[test]
    fn disabled_policy_returns_observed_checks() {
        let checks: Vec<_> = validation_inputs(false)
            .into_iter()
            .map(|input| decide_model_route(&input))
            .collect();
        assert!(checks.iter().all(|decision| decision.observe_only));
    }
}
