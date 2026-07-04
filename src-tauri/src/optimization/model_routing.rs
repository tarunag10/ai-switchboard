use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRouteInput {
    pub(crate) client: String,
    pub(crate) task: String,
    pub(crate) requested_model: String,
    pub(crate) cheap_model: String,
    pub(crate) capable_model: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRouteDecision {
    pub(crate) selected_model: String,
    pub(crate) observe_only: bool,
    pub(crate) reason: String,
}

pub(crate) fn decide_model_route(input: &ModelRouteInput) -> ModelRouteDecision {
    if !input.enabled {
        return ModelRouteDecision {
            selected_model: input.requested_model.clone(),
            observe_only: true,
            reason: "routing_disabled".to_string(),
        };
    }

    let task = input.task.to_ascii_lowercase();
    let trivial = [
        "commit message",
        "lint",
        "format",
        "typo",
        "rename",
        "summarize diff",
    ]
    .iter()
    .any(|needle| task.contains(needle));

    if trivial {
        ModelRouteDecision {
            selected_model: input.cheap_model.clone(),
            observe_only: true,
            reason: "trivial_task_candidate".to_string(),
        }
    } else {
        ModelRouteDecision {
            selected_model: input.capable_model.clone(),
            observe_only: true,
            reason: "capable_model_candidate".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_trivial_tasks_to_cheaper_model_in_observe_mode() {
        let decision = decide_model_route(&ModelRouteInput {
            client: "codex".to_string(),
            task: "write commit message for staged diff".to_string(),
            requested_model: "gpt-5.5".to_string(),
            cheap_model: "gpt-5.4-mini".to_string(),
            capable_model: "gpt-5.5".to_string(),
            enabled: true,
        });

        assert_eq!(decision.selected_model, "gpt-5.4-mini");
        assert!(decision.observe_only);
    }
}
