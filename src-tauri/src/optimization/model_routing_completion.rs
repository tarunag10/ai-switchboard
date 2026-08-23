use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};
use uuid::Uuid;

use super::model_routing::{ModelRouteDecision, ModelRouteInput};

/// Native-issued capability for recording one completed routing run. The
/// handle is opaque for completion; its native-generated run identifier is
/// exposed only so the resulting redacted evidence can be exported without
/// inventing provenance. Completion still accepts only the handle identifier
/// and content-free metrics, never a caller-supplied route decision.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingCompletionHandle {
    pub(crate) handle_id: String,
    pub(crate) run_id: String,
    pub(crate) issued_at: String,
    pub(crate) expires_at: String,
    pub(crate) decision: ModelRouteDecision,
}

/// Content-free completion metrics accepted by a native-issued handle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingCompletionMetrics {
    pub(crate) succeeded: bool,
    pub(crate) successful_task_cost_microunits: Option<u64>,
    pub(crate) quality_score_bps: Option<u32>,
    pub(crate) latency_ms: u64,
    pub(crate) follow_up_rework: Option<bool>,
}

/// Internal state retained only for the short completion window. It is never
/// serialized or exposed through a command.
#[derive(Debug, Clone)]
pub(crate) struct PendingModelRoutingCompletion {
    pub(crate) run_id: String,
    pub(crate) decision: ModelRouteDecision,
    pub(crate) expires_monotonic: Instant,
}

/// Process-local capability retained briefly after a successful completion so
/// evidence export can resolve the native run without accepting a caller-made
/// run identifier.
#[derive(Debug, Clone)]
pub(crate) struct CompletedModelRoutingRun {
    pub(crate) run_id: String,
    pub(crate) task_class: String,
    pub(crate) expires_monotonic: Instant,
}

pub(crate) const MODEL_ROUTING_COMPLETION_HANDLE_TTL_SECS: i64 = 15 * 60;
pub(crate) const MAX_PENDING_MODEL_ROUTING_COMPLETION_HANDLES: usize = 256;
pub(crate) const MAX_COMPLETED_MODEL_ROUTING_RUNS: usize = 256;

pub(crate) fn new_completion_handle(
    decision: ModelRouteDecision,
    now: DateTime<Utc>,
) -> (ModelRoutingCompletionHandle, PendingModelRoutingCompletion) {
    let handle_id = Uuid::new_v4().to_string();
    let run_id = format!("routing-run-{}", Uuid::new_v4());
    let expires_at = now + chrono::Duration::seconds(MODEL_ROUTING_COMPLETION_HANDLE_TTL_SECS);
    (
        ModelRoutingCompletionHandle {
            handle_id,
            run_id: run_id.clone(),
            issued_at: now.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            decision: decision.clone(),
        },
        PendingModelRoutingCompletion {
            run_id,
            decision,
            expires_monotonic: Instant::now()
                + Duration::from_secs(MODEL_ROUTING_COMPLETION_HANDLE_TTL_SECS as u64),
        },
    )
}

pub(crate) fn validate_completion_handle_input(input: &ModelRouteInput) -> Result<(), String> {
    let valid_identifier = |value: &str, label: &str, max_len: usize| {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.len() > max_len || trimmed.chars().any(char::is_control) {
            return Err(format!("model-routing handle requires a valid {label}"));
        }
        Ok(())
    };
    valid_identifier(&input.client, "client", 128)?;
    valid_identifier(&input.task, "task", 8_192)?;
    valid_identifier(&input.requested_model, "requested model", 128)?;
    valid_identifier(&input.cheap_model, "cheap model", 128)?;
    valid_identifier(&input.capable_model, "capable model", 128)?;
    if input
        .cheap_model
        .trim()
        .eq_ignore_ascii_case(input.capable_model.trim())
    {
        return Err("model-routing handle requires distinct cheap and capable models".to_string());
    }
    Ok(())
}
