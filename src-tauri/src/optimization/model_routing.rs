use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::storage::{app_data_dir, config_file};

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ModelRoutingStage {
    #[default]
    Observe,
    UserApproved,
    AutomaticAllowlisted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingThresholds {
    pub(crate) minimum_sample_size: u64,
    pub(crate) maximum_success_regression_bps: u32,
    pub(crate) maximum_quality_regression_bps: u32,
    pub(crate) minimum_cost_improvement_bps: u32,
    pub(crate) maximum_rework_rate_bps: u32,
    pub(crate) maximum_latency_regression_ms: u64,
}

impl Default for ModelRoutingThresholds {
    fn default() -> Self {
        Self {
            minimum_sample_size: 100,
            maximum_success_regression_bps: 100,
            maximum_quality_regression_bps: 100,
            minimum_cost_improvement_bps: 1_000,
            maximum_rework_rate_bps: 500,
            maximum_latency_regression_ms: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingExperimentPolicy {
    pub(crate) global_enabled: bool,
    pub(crate) stage: ModelRoutingStage,
    pub(crate) disabled_clients: Vec<String>,
    pub(crate) automatic_task_allowlist: Vec<String>,
    pub(crate) thresholds: ModelRoutingThresholds,
}

impl Default for ModelRoutingExperimentPolicy {
    fn default() -> Self {
        Self {
            global_enabled: true,
            stage: ModelRoutingStage::Observe,
            disabled_clients: Vec::new(),
            automatic_task_allowlist: Vec::new(),
            thresholds: ModelRoutingThresholds::default(),
        }
    }
}

pub(crate) fn load_model_routing_experiment_policy() -> ModelRoutingExperimentPolicy {
    let path = config_file(&app_data_dir(), "model-routing-experiment-policy.json");
    let Ok(raw) = std::fs::read(&path) else {
        return ModelRoutingExperimentPolicy::default();
    };
    serde_json::from_slice::<ModelRoutingExperimentPolicy>(&raw)
        .ok()
        .filter(|policy| validate_experiment_policy(policy).is_ok())
        .unwrap_or_default()
}

pub(crate) fn save_model_routing_experiment_policy(
    policy: &ModelRoutingExperimentPolicy,
) -> Result<ModelRoutingExperimentPolicy, String> {
    validate_experiment_policy(policy)?;
    let path = config_file(&app_data_dir(), "model-routing-experiment-policy.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create model-routing policy directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(policy)
        .map_err(|error| format!("serialize model-routing policy: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes)
        .map_err(|error| format!("write model-routing policy: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("secure model-routing policy: {error}"))?;
    }
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("replace model-routing policy: {error}"))?;
    Ok(policy.clone())
}

fn validate_experiment_policy(policy: &ModelRoutingExperimentPolicy) -> Result<(), String> {
    let thresholds = &policy.thresholds;
    if thresholds.maximum_success_regression_bps > 10_000
        || thresholds.maximum_quality_regression_bps > 10_000
        || thresholds.minimum_cost_improvement_bps > 10_000
        || thresholds.maximum_rework_rate_bps > 10_000
        || thresholds.minimum_sample_size == 0
    {
        return Err("model-routing thresholds must use a positive sample size and basis points from 0 to 10000".to_string());
    }
    for (label, values) in [
        ("disabled client", &policy.disabled_clients),
        ("automatic task class", &policy.automatic_task_allowlist),
    ] {
        if values.len() > 64
            || values
                .iter()
                .any(|value| value.trim().is_empty() || value.len() > 64)
        {
            return Err(format!(
                "{label} entries must contain 1 to 64 short identifiers"
            ));
        }
        let unique = values
            .iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        if unique.len() != values.len() {
            return Err(format!("{label} entries must be unique"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingBenchmarkEvidence {
    pub(crate) sample_size: u64,
    pub(crate) baseline_successes: u64,
    pub(crate) candidate_successes: u64,
    pub(crate) baseline_average_success_cost_microunits: u64,
    pub(crate) candidate_average_success_cost_microunits: u64,
    pub(crate) baseline_quality_score_bps: u32,
    pub(crate) candidate_quality_score_bps: u32,
    pub(crate) baseline_p95_latency_ms: u64,
    pub(crate) candidate_p95_latency_ms: u64,
    pub(crate) follow_up_rework_rate_bps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingEvidenceAssessment {
    pub(crate) success_regression_bps: i32,
    pub(crate) quality_regression_bps: i32,
    pub(crate) cost_improvement_bps: i32,
    pub(crate) latency_regression_ms: i64,
    pub(crate) follow_up_rework_rate_bps: u32,
    pub(crate) passed: bool,
    pub(crate) explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRouteDecision {
    /// Proposed model. `actual_model` remains requested during observation.
    pub(crate) selected_model: String,
    pub(crate) actual_model: String,
    pub(crate) observe_only: bool,
    pub(crate) reason: String,
    pub(crate) reasons: Vec<String>,
    pub(crate) stage: ModelRoutingStage,
    pub(crate) task_class: String,
    pub(crate) baseline_model: String,
    pub(crate) candidate_model: String,
    pub(crate) evidence: Option<ModelRoutingEvidenceAssessment>,
}

/// Native-issued capability for recording one completed routing run. The
/// handle is opaque for completion; its native-generated run identifier is
/// exposed only so the resulting redacted evidence can be exported without
/// inventing provenance. Completion still accepts only the handle identifier
/// and content-free metrics, never a caller-supplied route decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingCompletionHandle {
    pub(crate) handle_id: String,
    pub(crate) run_id: String,
    pub(crate) issued_at: String,
    pub(crate) expires_at: String,
    pub(crate) decision: ModelRouteDecision,
}

/// Content-free completion metrics accepted by a native-issued handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub(crate) const MODEL_ROUTING_COMPLETION_HANDLE_TTL_SECS: i64 = 15 * 60;
pub(crate) const MAX_PENDING_MODEL_ROUTING_COMPLETION_HANDLES: usize = 256;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingTaskOutcome {
    pub(crate) succeeded: bool,
    pub(crate) successful_task_cost_microunits: Option<u64>,
    pub(crate) follow_up_rework: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingExperimentRecord {
    pub(crate) task_class: String,
    pub(crate) proposed_model: String,
    pub(crate) actual_model: String,
    pub(crate) reason: String,
    pub(crate) observe_only: bool,
    pub(crate) outcome: ModelRoutingTaskOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ModelRoutingEvidenceArm {
    Baseline,
    Candidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelRoutingEvidenceSample {
    pub(crate) task_class: String,
    pub(crate) baseline_model: String,
    pub(crate) candidate_model: String,
    pub(crate) arm: ModelRoutingEvidenceArm,
    pub(crate) succeeded: bool,
    pub(crate) successful_task_cost_microunits: Option<u64>,
    pub(crate) quality_score_bps: u32,
    pub(crate) latency_ms: u64,
    pub(crate) follow_up_rework: bool,
}

/// Metadata-only observation captured from a completed routing run. The run
/// identifier and model identities make reconciliation deterministic without
/// retaining request or response content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingEvidenceObservation {
    pub(crate) run_id: String,
    pub(crate) captured_at: String,
    pub(crate) task_class: String,
    pub(crate) arm: ModelRoutingEvidenceArm,
    pub(crate) baseline_model: String,
    pub(crate) candidate_model: String,
    pub(crate) succeeded: bool,
    pub(crate) successful_task_cost_microunits: Option<u64>,
    pub(crate) quality_score_bps: u32,
    pub(crate) latency_ms: u64,
    pub(crate) follow_up_rework: bool,
}

/// Content-free metrics supplied by a completed route. Quality, rework, and
/// successful-task cost are intentionally caller-provided: routing code must
/// never infer them from prompts, responses, token counts, or latency alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingCompletionEvidence {
    pub(crate) run_id: String,
    pub(crate) captured_at: String,
    pub(crate) succeeded: bool,
    pub(crate) successful_task_cost_microunits: Option<u64>,
    pub(crate) quality_score_bps: Option<u32>,
    pub(crate) latency_ms: u64,
    pub(crate) follow_up_rework: Option<bool>,
}

impl ModelRoutingEvidenceObservation {
    pub(crate) fn sample(&self) -> ModelRoutingEvidenceSample {
        ModelRoutingEvidenceSample {
            task_class: self.task_class.trim().to_ascii_lowercase(),
            baseline_model: self.baseline_model.trim().to_string(),
            candidate_model: self.candidate_model.trim().to_string(),
            arm: self.arm,
            succeeded: self.succeeded,
            successful_task_cost_microunits: self.successful_task_cost_microunits,
            quality_score_bps: self.quality_score_bps,
            latency_ms: self.latency_ms,
            follow_up_rework: self.follow_up_rework,
        }
    }
}

/// Build a redacted evidence observation from a completed route. This adapter
/// does not persist data or promote routing; callers must pass the returned
/// observation through the validated telemetry-store boundary.
pub(crate) fn observation_from_completed_route(
    decision: &ModelRouteDecision,
    completion: ModelRoutingCompletionEvidence,
) -> Result<ModelRoutingEvidenceObservation, String> {
    let valid_identifier = |value: &str| {
        let trimmed = value.trim();
        !trimmed.is_empty()
            && trimmed.len() <= 128
            && trimmed.chars().all(|character| !character.is_control())
    };
    if !valid_identifier(&completion.run_id) {
        return Err("completed route requires a valid run identifier".to_string());
    }
    if !valid_identifier(&completion.captured_at) {
        return Err("completed route requires a valid capture timestamp".to_string());
    }
    if !valid_identifier(&decision.task_class) {
        return Err("completed route requires a valid task class".to_string());
    }
    if completion.latency_ms > i64::MAX as u64 {
        return Err("completed route latency is out of range".to_string());
    }
    let arm = if decision.actual_model == decision.baseline_model {
        ModelRoutingEvidenceArm::Baseline
    } else if decision.actual_model == decision.candidate_model {
        ModelRoutingEvidenceArm::Candidate
    } else {
        return Err("completed route model does not match baseline or candidate identity".to_string());
    };
    let quality_score_bps = completion
        .quality_score_bps
        .ok_or_else(|| "completed route requires an explicit quality score".to_string())?;
    let follow_up_rework = completion
        .follow_up_rework
        .ok_or_else(|| "completed route requires an explicit rework result".to_string())?;
    if quality_score_bps > 10_000 {
        return Err("completed route quality score must be between 0 and 10000 basis points".to_string());
    }
    if completion.succeeded != completion.successful_task_cost_microunits.is_some() {
        return Err(
            "successful-task cost is required only for successful completed routes".to_string(),
        );
    }

    Ok(ModelRoutingEvidenceObservation {
        run_id: completion.run_id,
        captured_at: completion.captured_at,
        task_class: decision.task_class.clone(),
        arm,
        baseline_model: decision.baseline_model.clone(),
        candidate_model: decision.candidate_model.clone(),
        succeeded: completion.succeeded,
        successful_task_cost_microunits: completion.successful_task_cost_microunits,
        quality_score_bps,
        latency_ms: completion.latency_ms,
        follow_up_rework,
    })
}

/// Deterministically reconciles redacted, task-class-scoped observations into
/// the benchmark shape consumed by the routing promotion gate. It never
/// accepts prompts, outputs, credentials, or mixed task classes.
pub(crate) fn aggregate_model_routing_evidence(
    samples: &[ModelRoutingEvidenceSample],
    expected_task_class: &str,
) -> Result<ModelRoutingBenchmarkEvidence, String> {
    let expected = expected_task_class.trim().to_ascii_lowercase();
    if expected.is_empty() {
        return Err("model-routing evidence requires a task class".to_string());
    }
    if samples.is_empty() {
        return Err("model-routing evidence requires baseline and candidate samples".to_string());
    }

    let mut baseline = Vec::new();
    let mut candidate = Vec::new();
    let mut model_pair: Option<(String, String)> = None;
    for sample in samples {
        let task_class = sample.task_class.trim().to_ascii_lowercase();
        if task_class != expected {
            return Err("model-routing evidence cannot mix task classes".to_string());
        }
        let baseline_model = sample.baseline_model.trim();
        let candidate_model = sample.candidate_model.trim();
        if baseline_model.is_empty()
            || candidate_model.is_empty()
            || baseline_model == candidate_model
        {
            return Err("model-routing evidence requires distinct model identities".to_string());
        }
        if let Some((expected_baseline, expected_candidate)) = model_pair.as_ref() {
            if baseline_model != expected_baseline || candidate_model != expected_candidate {
                return Err("model-routing evidence cannot mix model identities".to_string());
            }
        } else {
            model_pair = Some((baseline_model.to_string(), candidate_model.to_string()));
        }
        if sample.quality_score_bps > 10_000 {
            return Err("model-routing quality scores must be at most 10000 basis points".to_string());
        }
        match sample.arm {
            ModelRoutingEvidenceArm::Baseline => baseline.push(sample),
            ModelRoutingEvidenceArm::Candidate => candidate.push(sample),
        }
    }
    if baseline.len() != candidate.len() || baseline.is_empty() {
        return Err("model-routing evidence requires equal non-zero baseline and candidate samples".to_string());
    }

    fn success_count(samples: &[&ModelRoutingEvidenceSample]) -> u64 {
        samples.iter().filter(|sample| sample.succeeded).count() as u64
    }
    fn average_success_cost(samples: &[&ModelRoutingEvidenceSample]) -> Result<u64, String> {
        let successful = samples.iter().filter(|sample| sample.succeeded).collect::<Vec<_>>();
        if successful.is_empty() {
            return Err("model-routing evidence requires a successful task cost in each arm".to_string());
        }
        let mut total = 0u64;
        for sample in &successful {
            let cost = sample
                .successful_task_cost_microunits
                .ok_or_else(|| "successful model-routing samples require a cost".to_string())?;
            total = total
                .checked_add(cost)
                .ok_or_else(|| "model-routing evidence cost total overflowed".to_string())?;
        }
        Ok(total / successful.len() as u64)
    }
    fn average_quality(samples: &[&ModelRoutingEvidenceSample]) -> u32 {
        let total: u64 = samples.iter().map(|sample| sample.quality_score_bps as u64).sum();
        (total / samples.len() as u64) as u32
    }
    fn p95_latency(samples: &[&ModelRoutingEvidenceSample]) -> u64 {
        let mut latencies = samples.iter().map(|sample| sample.latency_ms).collect::<Vec<_>>();
        latencies.sort_unstable();
        let index = ((latencies.len() * 95).div_ceil(100)).saturating_sub(1);
        latencies[index]
    }
    fn rework_rate(samples: &[&ModelRoutingEvidenceSample]) -> u32 {
        ((samples.iter().filter(|sample| sample.follow_up_rework).count() as u64 * 10_000)
            / samples.len() as u64) as u32
    }

    Ok(ModelRoutingBenchmarkEvidence {
        sample_size: baseline.len() as u64,
        baseline_successes: success_count(&baseline),
        candidate_successes: success_count(&candidate),
        baseline_average_success_cost_microunits: average_success_cost(&baseline)?,
        candidate_average_success_cost_microunits: average_success_cost(&candidate)?,
        baseline_quality_score_bps: average_quality(&baseline),
        candidate_quality_score_bps: average_quality(&candidate),
        baseline_p95_latency_ms: p95_latency(&baseline),
        candidate_p95_latency_ms: p95_latency(&candidate),
        follow_up_rework_rate_bps: rework_rate(&candidate),
    })
}

pub(crate) fn record_model_routing_outcome(
    decision: &ModelRouteDecision,
    outcome: ModelRoutingTaskOutcome,
) -> ModelRoutingExperimentRecord {
    ModelRoutingExperimentRecord {
        task_class: decision.task_class.clone(),
        proposed_model: decision.selected_model.clone(),
        actual_model: decision.actual_model.clone(),
        reason: decision.reason.clone(),
        observe_only: decision.observe_only,
        outcome,
    }
}

pub(crate) fn decide_model_route(input: &ModelRouteInput) -> ModelRouteDecision {
    decide_model_route_experiment(input, &ModelRoutingExperimentPolicy::default(), false, None)
}

/// Deterministic routing promotion gate. It consumes no request content, model
/// output, or learned classifier: task classes and thresholds stay auditable.
pub(crate) fn decide_model_route_experiment(
    input: &ModelRouteInput,
    policy: &ModelRoutingExperimentPolicy,
    user_approved: bool,
    evidence: Option<&ModelRoutingBenchmarkEvidence>,
) -> ModelRouteDecision {
    let task_class = classify_task(&input.task).to_string();
    let proposed_model = if is_low_risk_task_class(&task_class) {
        input.cheap_model.clone()
    } else {
        input.capable_model.clone()
    };
    let mut reasons = vec![format!("task_class={task_class}")];

    if !input.enabled || !policy.global_enabled {
        reasons.push("routing_disabled_globally".to_string());
        return decision(
            input,
            input.requested_model.clone(),
            true,
            "routing_disabled",
            reasons,
            policy,
            task_class,
            None,
        );
    }
    if policy
        .disabled_clients
        .iter()
        .any(|client| client.trim().eq_ignore_ascii_case(&input.client))
    {
        reasons.push(format!("routing_disabled_for_client={}", input.client));
        return decision(
            input,
            input.requested_model.clone(),
            true,
            "client_routing_disabled",
            reasons,
            policy,
            task_class,
            None,
        );
    }

    match policy.stage {
        ModelRoutingStage::Observe => {
            reasons.push(format!("proposed_model={proposed_model}"));
            reasons.push(format!("actual_model={}", input.requested_model));
            decision(
                input,
                proposed_model,
                true,
                candidate_reason(&task_class),
                reasons,
                policy,
                task_class,
                None,
            )
        }
        ModelRoutingStage::UserApproved => {
            if !user_approved {
                reasons.push("user_approval_required".to_string());
                return decision(
                    input,
                    proposed_model,
                    true,
                    "awaiting_user_approval",
                    reasons,
                    policy,
                    task_class,
                    None,
                );
            }
            reasons.push("user_approved_route".to_string());
            decision(
                input,
                proposed_model,
                false,
                "user_approved",
                reasons,
                policy,
                task_class,
                None,
            )
        }
        ModelRoutingStage::AutomaticAllowlisted => {
            if !policy
                .automatic_task_allowlist
                .iter()
                .any(|allowed| allowed.trim().eq_ignore_ascii_case(&task_class))
            {
                reasons.push("task_class_not_allowlisted".to_string());
                return decision(
                    input,
                    proposed_model,
                    true,
                    "automatic_task_not_allowlisted",
                    reasons,
                    policy,
                    task_class,
                    None,
                );
            }
            let Some(evidence) = evidence else {
                reasons.push("benchmark_evidence_missing".to_string());
                return decision(
                    input,
                    proposed_model,
                    true,
                    "automatic_evidence_missing",
                    reasons,
                    policy,
                    task_class,
                    None,
                );
            };
            if let Err(reason) = validate_benchmark_evidence(evidence) {
                reasons.push(reason);
                return decision(
                    input,
                    proposed_model,
                    true,
                    "automatic_evidence_invalid",
                    reasons,
                    policy,
                    task_class,
                    None,
                );
            }
            let assessment = assess_evidence(evidence, &policy.thresholds);
            reasons.push(assessment.explanation.clone());
            if !assessment.passed {
                return decision(
                    input,
                    proposed_model,
                    true,
                    "automatic_thresholds_failed",
                    reasons,
                    policy,
                    task_class,
                    Some(assessment),
                );
            }
            reasons.push("automatic_route_benchmark_gate_passed".to_string());
            decision(
                input,
                proposed_model,
                false,
                "automatic_allowlisted",
                reasons,
                policy,
                task_class,
                Some(assessment),
            )
        }
    }
}

fn decision(
    input: &ModelRouteInput,
    proposed_model: String,
    observe_only: bool,
    reason: &str,
    reasons: Vec<String>,
    policy: &ModelRoutingExperimentPolicy,
    task_class: String,
    evidence: Option<ModelRoutingEvidenceAssessment>,
) -> ModelRouteDecision {
    ModelRouteDecision {
        actual_model: if observe_only {
            input.requested_model.clone()
        } else {
            proposed_model.clone()
        },
        selected_model: proposed_model.clone(),
        observe_only,
        reason: reason.to_string(),
        reasons,
        stage: policy.stage,
        task_class,
        baseline_model: input.requested_model.clone(),
        candidate_model: proposed_model.clone(),
        evidence,
    }
}

fn classify_task(task: &str) -> &'static str {
    let task = task.to_ascii_lowercase();
    if task.contains("format") || task.contains("lint") {
        "formatting"
    } else if task.contains("commit message") {
        "commit_message"
    } else if task.contains("rename") || task.contains("typo") {
        "rename"
    } else if task.contains("summarize diff") || task.contains("diff summary") {
        "diff_summary"
    } else {
        "general"
    }
}

fn is_low_risk_task_class(task_class: &str) -> bool {
    matches!(
        task_class,
        "formatting" | "commit_message" | "rename" | "diff_summary"
    )
}

fn candidate_reason(task_class: &str) -> &'static str {
    if is_low_risk_task_class(task_class) {
        "trivial_task_candidate"
    } else {
        "capable_model_candidate"
    }
}

pub(crate) fn assess_evidence(
    evidence: &ModelRoutingBenchmarkEvidence,
    thresholds: &ModelRoutingThresholds,
) -> ModelRoutingEvidenceAssessment {
    let baseline_success_bps = rate_bps(evidence.baseline_successes, evidence.sample_size);
    let candidate_success_bps = rate_bps(evidence.candidate_successes, evidence.sample_size);
    let success_regression_bps = baseline_success_bps.saturating_sub(candidate_success_bps);
    let quality_regression_bps = evidence
        .baseline_quality_score_bps
        .saturating_sub(evidence.candidate_quality_score_bps) as i32;
    let cost_improvement_bps = if evidence.baseline_average_success_cost_microunits == 0 {
        0
    } else {
        let delta = evidence.baseline_average_success_cost_microunits as i128
            - evidence.candidate_average_success_cost_microunits as i128;
        ((delta * 10_000) / evidence.baseline_average_success_cost_microunits as i128)
            .clamp(i32::MIN as i128, i32::MAX as i128) as i32
    };
    let enough_samples = evidence.sample_size >= thresholds.minimum_sample_size;
    let success_ok = success_regression_bps <= thresholds.maximum_success_regression_bps as i32;
    let quality_ok = quality_regression_bps <= thresholds.maximum_quality_regression_bps as i32;
    let cost_ok = cost_improvement_bps >= thresholds.minimum_cost_improvement_bps as i32;
    let latency_regression_ms = if evidence.candidate_p95_latency_ms >= evidence.baseline_p95_latency_ms {
        (evidence.candidate_p95_latency_ms - evidence.baseline_p95_latency_ms)
            .min(i64::MAX as u64) as i64
    } else {
        -(evidence
            .baseline_p95_latency_ms
            .saturating_sub(evidence.candidate_p95_latency_ms)
            .min(i64::MAX as u64) as i64)
    };
    let latency_ok = latency_regression_ms <= thresholds.maximum_latency_regression_ms as i64;
    let rework_ok = evidence.follow_up_rework_rate_bps <= thresholds.maximum_rework_rate_bps;
    let passed = enough_samples && success_ok && quality_ok && cost_ok && latency_ok && rework_ok;
    let explanation = format!(
        "benchmark_gate: samples={}/{} success_regression_bps={}/{} quality_regression_bps={}/{} cost_improvement_bps={}/{} latency_regression_ms={}/{} rework_bps={}/{} passed={passed}",
        evidence.sample_size,
        thresholds.minimum_sample_size,
        success_regression_bps,
        thresholds.maximum_success_regression_bps,
        quality_regression_bps,
        thresholds.maximum_quality_regression_bps,
        cost_improvement_bps,
        thresholds.minimum_cost_improvement_bps,
        latency_regression_ms,
        thresholds.maximum_latency_regression_ms,
        evidence.follow_up_rework_rate_bps,
        thresholds.maximum_rework_rate_bps,
    );
    ModelRoutingEvidenceAssessment {
        success_regression_bps,
        quality_regression_bps,
        cost_improvement_bps,
        latency_regression_ms,
        follow_up_rework_rate_bps: evidence.follow_up_rework_rate_bps,
        passed,
        explanation,
    }
}

fn validate_benchmark_evidence(evidence: &ModelRoutingBenchmarkEvidence) -> Result<(), String> {
    if evidence.sample_size == 0 {
        return Err("benchmark_evidence_invalid: sample_size must be positive".to_string());
    }
    if evidence.baseline_successes > evidence.sample_size
        || evidence.candidate_successes > evidence.sample_size
    {
        return Err("benchmark_evidence_invalid: successes cannot exceed sample_size".to_string());
    }
    if evidence.baseline_successes == 0 || evidence.candidate_successes == 0 {
        return Err("benchmark_evidence_invalid: each arm requires at least one successful task".to_string());
    }
    if evidence.baseline_quality_score_bps > 10_000
        || evidence.candidate_quality_score_bps > 10_000
        || evidence.follow_up_rework_rate_bps > 10_000
    {
        return Err("benchmark_evidence_invalid: basis-point metrics must be at most 10000".to_string());
    }
    Ok(())
}

fn rate_bps(successes: u64, sample_size: u64) -> i32 {
    if sample_size == 0 {
        0
    } else {
        ((successes.min(sample_size) as u128 * 10_000) / sample_size as u128) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ModelRouteInput {
        ModelRouteInput {
            client: "codex".to_string(),
            task: "format imports and lint".to_string(),
            requested_model: "frontier".to_string(),
            cheap_model: "fast/local".to_string(),
            capable_model: "frontier".to_string(),
            enabled: true,
        }
    }

    #[test]
    fn defaults_to_observation_and_records_proposal_separately_from_actual() {
        let decision = decide_model_route(&input());
        assert_eq!(decision.selected_model, "fast/local");
        assert_eq!(decision.actual_model, "frontier");
        assert!(decision.observe_only);
        assert_eq!(decision.stage, ModelRoutingStage::Observe);
        assert!(decision
            .reasons
            .iter()
            .any(|reason| reason == "task_class=formatting"));
        assert_eq!(decision.baseline_model, "frontier");
        assert_eq!(decision.candidate_model, "fast/local");
        assert_eq!(ModelRoutingExperimentPolicy::default().thresholds.minimum_sample_size, 100);
    }

    #[test]
    fn completion_handle_input_rejects_invalid_or_ambiguous_route_identity() {
        let mut invalid = input();
        invalid.client = " \n".to_string();
        assert!(validate_completion_handle_input(&invalid).is_err());

        let mut ambiguous = input();
        ambiguous.capable_model = ambiguous.cheap_model.clone();
        let error = validate_completion_handle_input(&ambiguous)
            .expect_err("identical experiment models must be rejected");
        assert!(error.contains("distinct cheap and capable models"));
    }

    #[test]
    fn completion_handles_are_unique_and_use_monotonic_expiry() {
        let now = chrono::Utc::now();
        let (first, pending) = new_completion_handle(decide_model_route(&input()), now);
        let (second, _) = new_completion_handle(decide_model_route(&input()), now);
        assert_ne!(first.handle_id, second.handle_id);
        assert!(pending.expires_monotonic > std::time::Instant::now());
        assert_eq!(first.decision.actual_model, input().requested_model);
        assert!(chrono::DateTime::parse_from_rfc3339(&first.expires_at).is_ok());
    }

    fn completion() -> ModelRoutingCompletionEvidence {
        ModelRoutingCompletionEvidence {
            run_id: "run-1".to_string(),
            captured_at: "2026-08-21T00:00:00Z".to_string(),
            succeeded: true,
            successful_task_cost_microunits: Some(100),
            quality_score_bps: Some(9_500),
            latency_ms: 120,
            follow_up_rework: Some(false),
        }
    }

    #[test]
    fn completed_route_adapter_builds_redacted_baseline_observation() {
        let decision = decide_model_route(&input());
        let observation = observation_from_completed_route(&decision, completion()).unwrap();
        assert_eq!(observation.arm, ModelRoutingEvidenceArm::Baseline);
        assert_eq!(observation.task_class, "formatting");
        assert_eq!(observation.baseline_model, "frontier");
        assert_eq!(observation.candidate_model, "fast/local");
        assert_eq!(observation.successful_task_cost_microunits, Some(100));
    }

    #[test]
    fn completed_route_adapter_requires_explicit_quality_rework_and_cost_contract() {
        let decision = decide_model_route(&input());

        let mut missing_quality = completion();
        missing_quality.quality_score_bps = None;
        assert!(observation_from_completed_route(&decision, missing_quality)
            .unwrap_err()
            .contains("explicit quality"));

        let mut missing_rework = completion();
        missing_rework.follow_up_rework = None;
        assert!(observation_from_completed_route(&decision, missing_rework)
            .unwrap_err()
            .contains("explicit rework"));

        let mut missing_cost = completion();
        missing_cost.successful_task_cost_microunits = None;
        assert!(observation_from_completed_route(&decision, missing_cost).is_err());

        let mut failed_with_cost = completion();
        failed_with_cost.succeeded = false;
        assert!(observation_from_completed_route(&decision, failed_with_cost).is_err());

        let mut invalid_quality = completion();
        invalid_quality.quality_score_bps = Some(10_001);
        assert!(observation_from_completed_route(&decision, invalid_quality)
            .unwrap_err()
            .contains("basis points"));
    }

    #[test]
    fn completed_route_adapter_rejects_unknown_actual_model() {
        let mut decision = decide_model_route(&input());
        decision.actual_model = "unregistered-model".to_string();
        assert!(observation_from_completed_route(&decision, completion())
            .unwrap_err()
            .contains("baseline or candidate"));
    }

    #[test]
    fn completed_route_adapter_rejects_invalid_context_identity_and_latency() {
        let decision = decide_model_route(&input());

        let mut missing_run_id = completion();
        missing_run_id.run_id = "  ".to_string();
        assert!(observation_from_completed_route(&decision, missing_run_id)
            .unwrap_err()
            .contains("run identifier"));

        let mut invalid_timestamp = completion();
        invalid_timestamp.captured_at = "\u{0000}".to_string();
        assert!(observation_from_completed_route(&decision, invalid_timestamp)
            .unwrap_err()
            .contains("capture timestamp"));

        let mut invalid_task_class = completion();
        let mut invalid_decision = decision.clone();
        invalid_decision.task_class = "\n".to_string();
        assert!(observation_from_completed_route(&invalid_decision, invalid_task_class)
            .unwrap_err()
            .contains("task class"));

        let mut oversized_latency = completion();
        oversized_latency.latency_ms = i64::MAX as u64 + 1;
        assert!(observation_from_completed_route(&decision, oversized_latency)
            .unwrap_err()
            .contains("latency"));
    }

    #[test]
    fn user_approved_stage_never_routes_without_current_approval() {
        let policy = ModelRoutingExperimentPolicy {
            stage: ModelRoutingStage::UserApproved,
            ..Default::default()
        };
        assert!(decide_model_route_experiment(&input(), &policy, false, None).observe_only);
        let approved = decide_model_route_experiment(&input(), &policy, true, None);
        assert!(!approved.observe_only);
        assert_eq!(approved.actual_model, "fast/local");
    }

    #[test]
    fn automatic_stage_requires_allowlist_and_all_benchmark_thresholds() {
        let policy = ModelRoutingExperimentPolicy {
            stage: ModelRoutingStage::AutomaticAllowlisted,
            automatic_task_allowlist: vec!["formatting".to_string()],
            ..Default::default()
        };
        let passing = ModelRoutingBenchmarkEvidence {
            sample_size: 100,
            baseline_successes: 98,
            candidate_successes: 98,
            baseline_average_success_cost_microunits: 1_000,
            candidate_average_success_cost_microunits: 700,
            baseline_quality_score_bps: 9_800,
            candidate_quality_score_bps: 9_800,
            baseline_p95_latency_ms: 800,
            candidate_p95_latency_ms: 820,
            follow_up_rework_rate_bps: 300,
        };
        let decision = decide_model_route_experiment(&input(), &policy, false, Some(&passing));
        assert!(!decision.observe_only);
        assert!(decision.evidence.as_ref().unwrap().passed);

        let failing = ModelRoutingBenchmarkEvidence {
            candidate_successes: 80,
            ..passing
        };
        let decision = decide_model_route_experiment(&input(), &policy, false, Some(&failing));
        assert!(decision.observe_only);
        assert_eq!(decision.reason, "automatic_thresholds_failed");
    }

    #[test]
    fn automatic_allowlist_normalizes_case_and_whitespace() {
        let policy = ModelRoutingExperimentPolicy {
            stage: ModelRoutingStage::AutomaticAllowlisted,
            automatic_task_allowlist: vec!["  FoRmAtTiNg  ".to_string()],
            ..Default::default()
        };
        let evidence = ModelRoutingBenchmarkEvidence {
            sample_size: 100,
            baseline_successes: 98,
            candidate_successes: 98,
            baseline_average_success_cost_microunits: 1_000,
            candidate_average_success_cost_microunits: 700,
            baseline_quality_score_bps: 9_800,
            candidate_quality_score_bps: 9_800,
            baseline_p95_latency_ms: 800,
            candidate_p95_latency_ms: 820,
            follow_up_rework_rate_bps: 300,
        };
        let decision = decide_model_route_experiment(&input(), &policy, false, Some(&evidence));
        assert!(!decision.observe_only);
        assert_eq!(decision.reason, "automatic_allowlisted");
    }

    #[test]
    fn global_and_per_client_kill_switches_preserve_requested_model() {
        let global = ModelRoutingExperimentPolicy {
            global_enabled: false,
            stage: ModelRoutingStage::AutomaticAllowlisted,
            ..Default::default()
        };
        assert_eq!(
            decide_model_route_experiment(&input(), &global, true, None).actual_model,
            "frontier"
        );
        let client = ModelRoutingExperimentPolicy {
            disabled_clients: vec!["CODEX".to_string()],
            stage: ModelRoutingStage::UserApproved,
            ..Default::default()
        };
        assert_eq!(
            decide_model_route_experiment(&input(), &client, true, None).reason,
            "client_routing_disabled"
        );

        let whitespace_client = ModelRoutingExperimentPolicy {
            disabled_clients: vec!["  codex  ".to_string()],
            stage: ModelRoutingStage::UserApproved,
            ..Default::default()
        };
        assert_eq!(
            decide_model_route_experiment(&input(), &whitespace_client, true, None).reason,
            "client_routing_disabled"
        );
    }

    #[test]
    fn rework_can_erase_savings_and_block_automatic_routing() {
        let evidence = ModelRoutingBenchmarkEvidence {
            sample_size: 100,
            baseline_successes: 98,
            candidate_successes: 98,
            baseline_average_success_cost_microunits: 1_000,
            candidate_average_success_cost_microunits: 500,
            baseline_quality_score_bps: 9_800,
            candidate_quality_score_bps: 9_800,
            baseline_p95_latency_ms: 800,
            candidate_p95_latency_ms: 820,
            follow_up_rework_rate_bps: 900,
        };
        let assessment = assess_evidence(&evidence, &ModelRoutingThresholds::default());
        assert_eq!(assessment.cost_improvement_bps, 5_000);
        assert!(!assessment.passed);
        assert!(assessment.explanation.contains("rework_bps=900/500"));
    }

    #[test]
    fn quality_and_latency_regressions_block_automatic_routing() {
        let evidence = ModelRoutingBenchmarkEvidence {
            sample_size: 100,
            baseline_successes: 100,
            candidate_successes: 100,
            baseline_average_success_cost_microunits: 1_000,
            candidate_average_success_cost_microunits: 500,
            baseline_quality_score_bps: 10_000,
            candidate_quality_score_bps: 9_700,
            baseline_p95_latency_ms: 800,
            candidate_p95_latency_ms: 900,
            follow_up_rework_rate_bps: 0,
        };
        let assessment = assess_evidence(&evidence, &ModelRoutingThresholds::default());
        assert_eq!(assessment.quality_regression_bps, 300);
        assert_eq!(assessment.latency_regression_ms, 100);
        assert!(!assessment.passed);
        assert!(assessment.explanation.contains("quality_regression_bps=300/100"));
        assert!(assessment.explanation.contains("latency_regression_ms=100/50"));
    }

    #[test]
    fn automatic_routing_rejects_impossible_benchmark_counts() {
        let policy = ModelRoutingExperimentPolicy {
            stage: ModelRoutingStage::AutomaticAllowlisted,
            automatic_task_allowlist: vec!["formatting".to_string()],
            ..Default::default()
        };
        let evidence = ModelRoutingBenchmarkEvidence {
            sample_size: 10,
            baseline_successes: 11,
            candidate_successes: 10,
            baseline_average_success_cost_microunits: 1,
            candidate_average_success_cost_microunits: 1,
            baseline_quality_score_bps: 10_000,
            candidate_quality_score_bps: 10_000,
            baseline_p95_latency_ms: 1,
            candidate_p95_latency_ms: 1,
            follow_up_rework_rate_bps: 0,
        };
        let decision = decide_model_route_experiment(&input(), &policy, false, Some(&evidence));
        assert!(decision.observe_only);
        assert_eq!(decision.reason, "automatic_evidence_invalid");
        assert!(decision.evidence.is_none());
    }

    #[test]
    fn automatic_routing_rejects_zero_success_arms_even_with_costs() {
        let policy = ModelRoutingExperimentPolicy {
            stage: ModelRoutingStage::AutomaticAllowlisted,
            automatic_task_allowlist: vec!["formatting".to_string()],
            ..Default::default()
        };
        let evidence = ModelRoutingBenchmarkEvidence {
            sample_size: 100,
            baseline_successes: 0,
            candidate_successes: 0,
            baseline_average_success_cost_microunits: 1_000,
            candidate_average_success_cost_microunits: 500,
            baseline_quality_score_bps: 10_000,
            candidate_quality_score_bps: 10_000,
            baseline_p95_latency_ms: 800,
            candidate_p95_latency_ms: 800,
            follow_up_rework_rate_bps: 0,
        };
        let decision = decide_model_route_experiment(&input(), &policy, false, Some(&evidence));
        assert!(decision.observe_only);
        assert_eq!(decision.reason, "automatic_evidence_invalid");
    }

    #[test]
    fn latency_regression_does_not_wrap_for_large_unsigned_values() {
        let evidence = ModelRoutingBenchmarkEvidence {
            sample_size: 1,
            baseline_successes: 1,
            candidate_successes: 1,
            baseline_average_success_cost_microunits: 1,
            candidate_average_success_cost_microunits: 1,
            baseline_quality_score_bps: 10_000,
            candidate_quality_score_bps: 10_000,
            baseline_p95_latency_ms: u64::MAX,
            candidate_p95_latency_ms: 0,
            follow_up_rework_rate_bps: 0,
        };
        let assessment = assess_evidence(&evidence, &ModelRoutingThresholds::default());
        assert_eq!(assessment.latency_regression_ms, -i64::MAX);
    }

    #[test]
    fn observation_record_keeps_proposal_actual_model_and_task_outcome() {
        let decision = decide_model_route(&input());
        let record = record_model_routing_outcome(
            &decision,
            ModelRoutingTaskOutcome {
                succeeded: true,
                successful_task_cost_microunits: Some(725),
                follow_up_rework: false,
            },
        );
        assert_eq!(record.proposed_model, "fast/local");
        assert_eq!(record.actual_model, "frontier");
        assert!(record.outcome.succeeded);
        assert_eq!(record.outcome.successful_task_cost_microunits, Some(725));
    }

    #[test]
    fn evidence_aggregation_is_deterministic_and_redacted() {
        let samples = vec![
            ModelRoutingEvidenceSample {
                task_class: "low_risk".to_string(),
                baseline_model: "frontier".to_string(),
                candidate_model: "fast/local".to_string(),
                arm: ModelRoutingEvidenceArm::Baseline,
                succeeded: true,
                successful_task_cost_microunits: Some(1_000),
                quality_score_bps: 9_900,
                latency_ms: 800,
                follow_up_rework: false,
            },
            ModelRoutingEvidenceSample {
                task_class: "low_risk".to_string(),
                baseline_model: "frontier".to_string(),
                candidate_model: "fast/local".to_string(),
                arm: ModelRoutingEvidenceArm::Candidate,
                succeeded: true,
                successful_task_cost_microunits: Some(700),
                quality_score_bps: 9_850,
                latency_ms: 780,
                follow_up_rework: false,
            },
        ];
        let evidence = aggregate_model_routing_evidence(&samples, "low_risk").unwrap();
        assert_eq!(evidence.sample_size, 1);
        assert_eq!(evidence.candidate_average_success_cost_microunits, 700);
        assert_eq!(evidence.candidate_p95_latency_ms, 780);
        assert_eq!(evidence.follow_up_rework_rate_bps, 0);
    }

    #[test]
    fn evidence_aggregation_canonicalizes_task_and_model_identity_whitespace() {
        let samples = vec![
            ModelRoutingEvidenceSample {
                task_class: " Low_Risk ".to_string(),
                baseline_model: " frontier ".to_string(),
                candidate_model: " fast/local ".to_string(),
                arm: ModelRoutingEvidenceArm::Baseline,
                succeeded: true,
                successful_task_cost_microunits: Some(1_000),
                quality_score_bps: 9_900,
                latency_ms: 800,
                follow_up_rework: false,
            },
            ModelRoutingEvidenceSample {
                task_class: "low_risk".to_string(),
                baseline_model: "frontier".to_string(),
                candidate_model: "fast/local".to_string(),
                arm: ModelRoutingEvidenceArm::Candidate,
                succeeded: true,
                successful_task_cost_microunits: Some(700),
                quality_score_bps: 9_850,
                latency_ms: 780,
                follow_up_rework: false,
            },
        ];
        assert!(aggregate_model_routing_evidence(&samples, " LOW_RISK ").is_ok());
    }

    #[test]
    fn evidence_aggregation_rejects_mixed_or_incomplete_samples() {
        let mixed = ModelRoutingEvidenceSample {
            task_class: "high_risk".to_string(),
            baseline_model: "frontier".to_string(),
            candidate_model: "fast/local".to_string(),
            arm: ModelRoutingEvidenceArm::Candidate,
            succeeded: true,
            successful_task_cost_microunits: Some(700),
            quality_score_bps: 9_850,
            latency_ms: 780,
            follow_up_rework: false,
        };
        assert!(aggregate_model_routing_evidence(&[mixed], "low_risk").is_err());

        let incomplete = ModelRoutingEvidenceSample {
            task_class: "low_risk".to_string(),
            baseline_model: "frontier".to_string(),
            candidate_model: "fast/local".to_string(),
            arm: ModelRoutingEvidenceArm::Baseline,
            succeeded: true,
            successful_task_cost_microunits: None,
            quality_score_bps: 9_850,
            latency_ms: 780,
            follow_up_rework: false,
        };
        assert!(aggregate_model_routing_evidence(&[incomplete], "low_risk").is_err());

        let mismatched = ModelRoutingEvidenceSample {
            task_class: "low_risk".to_string(),
            baseline_model: "frontier".to_string(),
            candidate_model: "another/local".to_string(),
            arm: ModelRoutingEvidenceArm::Candidate,
            succeeded: true,
            successful_task_cost_microunits: Some(700),
            quality_score_bps: 9_850,
            latency_ms: 780,
            follow_up_rework: false,
        };
        let baseline = ModelRoutingEvidenceSample {
            task_class: "low_risk".to_string(),
            baseline_model: "frontier".to_string(),
            candidate_model: "fast/local".to_string(),
            arm: ModelRoutingEvidenceArm::Baseline,
            succeeded: true,
            successful_task_cost_microunits: Some(1_000),
            quality_score_bps: 9_900,
            latency_ms: 800,
            follow_up_rework: false,
        };
        assert!(aggregate_model_routing_evidence(&[baseline, mismatched], "low_risk").is_err());
    }

    #[test]
    fn extreme_candidate_cost_fails_closed_instead_of_wrapping_positive() {
        let evidence = ModelRoutingBenchmarkEvidence {
            sample_size: 100,
            baseline_successes: 100,
            candidate_successes: 100,
            baseline_average_success_cost_microunits: 1,
            candidate_average_success_cost_microunits: u64::MAX,
            baseline_quality_score_bps: 10_000,
            candidate_quality_score_bps: 10_000,
            baseline_p95_latency_ms: 1,
            candidate_p95_latency_ms: 1,
            follow_up_rework_rate_bps: 0,
        };
        let assessment = assess_evidence(&evidence, &ModelRoutingThresholds::default());
        assert_eq!(assessment.cost_improvement_bps, i32::MIN);
        assert!(!assessment.passed);
    }

    #[test]
    fn persisted_policy_validation_rejects_invalid_thresholds_and_duplicate_clients() {
        let invalid_threshold = ModelRoutingExperimentPolicy {
            thresholds: ModelRoutingThresholds {
                maximum_success_regression_bps: 10_001,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(validate_experiment_policy(&invalid_threshold).is_err());

        let duplicate_client = ModelRoutingExperimentPolicy {
            disabled_clients: vec!["codex".to_string(), "CODEX".to_string()],
            ..Default::default()
        };
        assert!(validate_experiment_policy(&duplicate_client).is_err());
    }
}
