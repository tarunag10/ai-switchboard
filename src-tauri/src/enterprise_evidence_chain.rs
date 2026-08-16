//! Content-free Phase 5 exit proof for an enterprise-routed request.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterpriseEvidenceChain {
    pub request_id: String,
    pub client_identity_reference: String,
    pub optimization_policy_reference: String,
    pub endpoint_policy_reference: String,
    pub routing_evidence_reference: String,
    pub telemetry_request_id: String,
    pub audit_request_id: String,
    pub recovery_reference: String,
    pub content_free: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterpriseEvidenceDecision {
    pub complete: bool,
    pub reasons: Vec<String>,
}

pub(crate) fn evaluate_enterprise_evidence_chain(
    chain: &EnterpriseEvidenceChain,
) -> EnterpriseEvidenceDecision {
    let mut reasons = Vec::new();
    for (name, value) in [
        ("request_id", chain.request_id.as_str()),
        ("client_identity", chain.client_identity_reference.as_str()),
        (
            "optimization_policy",
            chain.optimization_policy_reference.as_str(),
        ),
        ("endpoint_policy", chain.endpoint_policy_reference.as_str()),
        (
            "routing_evidence",
            chain.routing_evidence_reference.as_str(),
        ),
        ("recovery", chain.recovery_reference.as_str()),
    ] {
        if !safe_reference(value) {
            reasons.push(format!("{name}_reference_invalid"));
        }
    }
    if chain.telemetry_request_id != chain.request_id {
        reasons.push("telemetry_request_mismatch".to_string());
    }
    if chain.audit_request_id != chain.request_id {
        reasons.push("audit_request_mismatch".to_string());
    }
    if !chain.content_free {
        reasons.push("content_free_evidence_not_proven".to_string());
    }
    EnterpriseEvidenceDecision {
        complete: reasons.is_empty(),
        reasons,
    }
}

fn safe_reference(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
        && !["bearer", "api_key", "apikey", "secret", "token="]
            .iter()
            .any(|needle| lowered.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> EnterpriseEvidenceChain {
        EnterpriseEvidenceChain {
            request_id: "req-001".into(),
            client_identity_reference: "client:codex".into(),
            optimization_policy_reference: "policy:balanced-v1".into(),
            endpoint_policy_reference: "endpoint:dynamo-prod".into(),
            routing_evidence_reference: "route:decision-001".into(),
            telemetry_request_id: "req-001".into(),
            audit_request_id: "req-001".into(),
            recovery_reference: "rollback:profile-v1".into(),
            content_free: true,
        }
    }

    #[test]
    fn complete_chain_proves_the_phase_five_exit_sequence() {
        assert_eq!(
            evaluate_enterprise_evidence_chain(&chain()),
            EnterpriseEvidenceDecision {
                complete: true,
                reasons: Vec::new(),
            }
        );
    }

    #[test]
    fn mismatched_or_content_bearing_evidence_fails_closed() {
        let mut input = chain();
        input.telemetry_request_id = "req-other".into();
        input.recovery_reference = "secret=plaintext".into();
        input.content_free = false;
        let decision = evaluate_enterprise_evidence_chain(&input);
        assert!(!decision.complete);
        assert!(decision
            .reasons
            .contains(&"telemetry_request_mismatch".to_string()));
        assert!(decision
            .reasons
            .contains(&"recovery_reference_invalid".to_string()));
        assert!(decision
            .reasons
            .contains(&"content_free_evidence_not_proven".to_string()));
    }
}
