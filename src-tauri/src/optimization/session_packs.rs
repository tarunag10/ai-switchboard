use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSessionStartInput {
    pub(crate) agent: String,
    pub(crate) task: String,
    pub(crate) repo_root: String,
    pub(crate) pack_id: Option<String>,
    pub(crate) token_budget: u64,
    pub(crate) pack_estimated_tokens: u64,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSessionPackPlan {
    pub(crate) inject_pack: bool,
    pub(crate) pack_id: String,
    pub(crate) remaining_budget_tokens: u64,
    pub(crate) reason: String,
}

pub(crate) fn plan_agent_session_pack(input: &AgentSessionStartInput) -> AgentSessionPackPlan {
    let pack_id = input
        .pack_id
        .clone()
        .unwrap_or_else(|| "implementation".to_string());

    if !input.enabled {
        return AgentSessionPackPlan {
            inject_pack: false,
            pack_id,
            remaining_budget_tokens: input.token_budget,
            reason: "session_pack_injection_disabled".to_string(),
        };
    }

    if input.pack_estimated_tokens > input.token_budget {
        return AgentSessionPackPlan {
            inject_pack: false,
            pack_id,
            remaining_budget_tokens: input.token_budget,
            reason: "pack_exceeds_session_budget".to_string(),
        };
    }

    AgentSessionPackPlan {
        inject_pack: true,
        pack_id,
        remaining_budget_tokens: input.token_budget - input.pack_estimated_tokens,
        reason: "pack_fits_session_budget".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_when_pack_fits_budget() {
        let plan = plan_agent_session_pack(&AgentSessionStartInput {
            agent: "codex".to_string(),
            task: "implement".to_string(),
            repo_root: "/tmp/repo".to_string(),
            pack_id: None,
            token_budget: 1_000,
            pack_estimated_tokens: 250,
            enabled: true,
        });

        assert!(plan.inject_pack);
        assert_eq!(plan.pack_id, "implementation");
        assert_eq!(plan.remaining_budget_tokens, 750);
    }
}
