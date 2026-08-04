//! Provider-billed counterfactual measurement for P1 savings supremacy.
//!
//! Records measured savings only when an independent before/after provider-billed
//! token pair is supplied. Read-only normalization helpers expose Codex/Claude
//! usage and Headroom /stats surfaces to Token X-Ray without inventing zeros.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{
    SavingsAttributionConfidence, SavingsAttributionEvent, SavingsAttributionScope,
    SavingsAttributionSource,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderBilledProvider {
    Codex,
    Claude,
    HeadroomStats,
}

impl ProviderBilledProvider {
    fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude",
            Self::HeadroomStats => "Headroom /stats",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBilledReading {
    pub provider: ProviderBilledProvider,
    pub billed_input_tokens: u64,
    pub source_endpoint: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBilledCounterfactualRequest {
    pub provider: ProviderBilledProvider,
    pub baseline_tokens: u64,
    pub optimized_tokens: u64,
    pub baseline_evidence: String,
    pub optimized_evidence: String,
    #[serde(default = "default_request_delta")]
    pub request_delta: usize,
}

fn default_request_delta() -> usize {
    1
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderBilledValidationError {
    InvalidBaselineTokens,
    InvalidOptimizedTokens,
    InvalidRequestDelta,
    MissingBaselineEvidence,
    MissingOptimizedEvidence,
    EmptyDelta,
}

pub fn validate_provider_billed_counterfactual(
    request: &ProviderBilledCounterfactualRequest,
) -> Result<(), ProviderBilledValidationError> {
    if !valid_token_count(request.baseline_tokens) {
        return Err(ProviderBilledValidationError::InvalidBaselineTokens);
    }
    if !valid_token_count(request.optimized_tokens) {
        return Err(ProviderBilledValidationError::InvalidOptimizedTokens);
    }
    if request.request_delta == 0 {
        return Err(ProviderBilledValidationError::InvalidRequestDelta);
    }
    if request.baseline_evidence.trim().is_empty() {
        return Err(ProviderBilledValidationError::MissingBaselineEvidence);
    }
    if request.optimized_evidence.trim().is_empty() {
        return Err(ProviderBilledValidationError::MissingOptimizedEvidence);
    }
    if request.baseline_tokens <= request.optimized_tokens {
        return Err(ProviderBilledValidationError::EmptyDelta);
    }
    Ok(())
}

fn valid_token_count(value: u64) -> bool {
    value > 0
}

/// Extract billed input tokens from a Codex `GET /wham/usage` JSON payload when
/// the upstream response exposes an explicit used-token counter.
pub fn extract_codex_billed_input_tokens(payload: &Value) -> Option<u64> {
    payload
        .pointer("/rate_limit/primary_window/used_tokens")
        .and_then(Value::as_u64)
        .or_else(|| {
            payload
                .pointer("/rate_limit/secondary_window/used_tokens")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            find_u64_key_recursive(
                payload,
                &[
                    "used_tokens",
                    "usedTokens",
                    "input_tokens",
                    "inputTokens",
                    "total_input_tokens",
                    "totalInputTokens",
                ],
            )
        })
        .filter(|value| *value > 0)
}

/// Claude OAuth usage currently exposes utilization windows, not billed token
/// counters. Return `None` unless an explicit token field is present.
pub fn extract_claude_billed_input_tokens(payload: &Value) -> Option<u64> {
    find_u64_key_recursive(
        payload,
        &[
            "input_tokens",
            "inputTokens",
            "total_input_tokens",
            "totalInputTokens",
            "billed_input_tokens",
            "billedInputTokens",
        ],
    )
    .filter(|value| *value > 0)
}

/// Headroom /stats after-compression billed input tokens.
pub fn extract_headroom_billed_input_tokens(session_total_tokens_sent: Option<u64>) -> Option<u64> {
    session_total_tokens_sent.filter(|value| *value > 0)
}

/// Counterfactual baseline from Headroom /stats when saved + sent are both present.
pub fn extract_headroom_baseline_tokens(
    session_total_tokens_sent: Option<u64>,
    session_estimated_tokens_saved: Option<u64>,
) -> Option<u64> {
    let optimized = extract_headroom_billed_input_tokens(session_total_tokens_sent)?;
    let saved = session_estimated_tokens_saved.filter(|value| *value > 0)?;
    Some(optimized.saturating_add(saved))
}

pub fn build_provider_billed_attribution_event(
    request: &ProviderBilledCounterfactualRequest,
) -> Result<SavingsAttributionEvent, ProviderBilledValidationError> {
    validate_provider_billed_counterfactual(request)?;
    let delta_tokens = request.baseline_tokens - request.optimized_tokens;
    Ok(SavingsAttributionEvent {
        schema_version: 1,
        id: Uuid::new_v4().to_string(),
        observed_at: Utc::now(),
        scope: SavingsAttributionScope::Session,
        source: SavingsAttributionSource::HeadroomEngine,
        confidence: SavingsAttributionConfidence::Measured,
        delta_tokens_saved: delta_tokens,
        delta_usd: 0.0,
        total_tokens_sent: request.optimized_tokens,
        request_delta: request.request_delta,
        evidence: vec![format!(
            "{} provider-billed counterfactual measured {delta_tokens} saved tokens from {} before to {} after.",
            request.provider.label(),
            request.baseline_tokens,
            request.optimized_tokens
        ), format!(
            "Baseline evidence: {}.",
            request.baseline_evidence.trim()
        ), format!(
            "Optimized evidence: {}.",
            request.optimized_evidence.trim()
        )],
    })
}

fn find_u64_key_recursive(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(Value::as_u64) {
                    return Some(found);
                }
            }
            map.values().find_map(|child| find_u64_key_recursive(child, keys))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|child| find_u64_key_recursive(child, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_complete_counterfactual_pair() {
        let request = ProviderBilledCounterfactualRequest {
            provider: ProviderBilledProvider::Codex,
            baseline_tokens: 12_000,
            optimized_tokens: 4_200,
            baseline_evidence: "Codex /wham/usage before Switchboard".into(),
            optimized_evidence: "Codex /wham/usage after Switchboard".into(),
            request_delta: 3,
        };
        let event = build_provider_billed_attribution_event(&request).expect("event");
        assert_eq!(event.confidence, SavingsAttributionConfidence::Measured);
        assert_eq!(event.delta_tokens_saved, 7_800);
        assert_eq!(event.request_delta, 3);
    }

    #[test]
    fn rejects_incomplete_counterfactual_pair() {
        let request = ProviderBilledCounterfactualRequest {
            provider: ProviderBilledProvider::Claude,
            baseline_tokens: 1_000,
            optimized_tokens: 1_200,
            baseline_evidence: "before".into(),
            optimized_evidence: "after".into(),
            request_delta: 1,
        };
        assert_eq!(
            validate_provider_billed_counterfactual(&request),
            Err(ProviderBilledValidationError::EmptyDelta)
        );
    }

    #[test]
    fn extracts_codex_and_headroom_counters() {
        let codex = json!({
            "rate_limit": {
                "primary_window": { "used_tokens": 4200 }
            }
        });
        assert_eq!(extract_codex_billed_input_tokens(&codex), Some(4200));

        let stats_sent = Some(1800u64);
        let stats_saved = Some(900u64);
        assert_eq!(extract_headroom_billed_input_tokens(stats_sent), Some(1800));
        assert_eq!(
            extract_headroom_baseline_tokens(stats_sent, stats_saved),
            Some(2700)
        );
    }

    #[test]
    fn claude_usage_without_token_fields_stays_unavailable() {
        let body = json!({
            "five_hour": { "utilization": 0.42, "resets_at": "2026-07-30T12:00:00Z" }
        });
        assert_eq!(extract_claude_billed_input_tokens(&body), None);
    }
}
