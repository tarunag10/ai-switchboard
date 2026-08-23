use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::optimization::model_routing::{
    aggregate_model_routing_evidence, ModelRoutingEvidenceArm,
    ModelRoutingEvidenceObservation,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingEvidenceArtifact {
    pub(crate) schema_version: u32,
    pub(crate) evidence_class: &'static str,
    pub(crate) minimum_samples: u64,
    pub(crate) baseline: ModelRoutingEvidenceArmMetrics,
    pub(crate) candidate: ModelRoutingEvidenceArmMetrics,
    pub(crate) provenance: ModelRoutingEvidenceProvenance,
    pub(crate) promotion_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingEvidenceArmMetrics {
    pub(crate) sample_count: u64,
    pub(crate) successful_task_count: u64,
    pub(crate) success_rate_bps: u32,
    pub(crate) quality_score_bps: u32,
    pub(crate) p95_latency_ms: u64,
    pub(crate) successful_task_cost_micros: u64,
    pub(crate) follow_up_rework_rate_bps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRoutingEvidenceProvenance {
    pub(crate) task_class: String,
    pub(crate) baseline_model: String,
    pub(crate) candidate_model: String,
    pub(crate) source: &'static str,
    pub(crate) cost_attribution: &'static str,
    pub(crate) provider_id: Option<String>,
    pub(crate) run_id: String,
    pub(crate) captured_at: String,
}

const MAX_MODEL_ROUTING_EVIDENCE_EVENTS_PER_RUN: i64 = 10_000;

fn validate_export_row(
    captured_at: &str,
    succeeded: i64,
    successful_task_cost_microunits: Option<i64>,
    quality_score_bps: i64,
    latency_ms: i64,
    follow_up_rework: i64,
) -> rusqlite::Result<()> {
    let parsed = DateTime::parse_from_rfc3339(captured_at.trim())
        .ok()
        .map(|value| value.with_timezone(&Utc));
    let now = Utc::now();
    let timestamp_invalid = parsed.is_none() || parsed.is_some_and(|value| {
        value > now + chrono::Duration::minutes(5)
            || value < now - chrono::Duration::days(7)
    });
    let cost_invalid = successful_task_cost_microunits.is_some_and(|value| value < 0);
    let contract_invalid = timestamp_invalid
        || !matches!(succeeded, 0 | 1)
        || !matches!(follow_up_rework, 0 | 1)
        || quality_score_bps < 0
        || quality_score_bps > 10_000
        || latency_ms < 0
        || cost_invalid
        || (succeeded == 1) != successful_task_cost_microunits.is_some();
    if contract_invalid {
        return Err(rusqlite::Error::InvalidParameterName(
            "invalid persisted model-routing evidence row".to_string(),
        ));
    }
    Ok(())
}

/// Persist only redacted routing outcome metrics. Request and response bodies
/// are intentionally not represented by this schema.
pub(crate) fn record_model_routing_evidence(
    observation: &ModelRoutingEvidenceObservation,
) -> Result<(), String> {
    try_record_model_routing_evidence(observation)
        .map_err(|error| format!("model-routing evidence persist failed: {error}"))
}

fn try_record_model_routing_evidence(
    observation: &ModelRoutingEvidenceObservation,
) -> rusqlite::Result<()> {
    let valid_identifier = |value: &str| {
        let trimmed = value.trim();
        !trimmed.is_empty()
            && trimmed.len() <= 128
            && trimmed.chars().all(|character| !character.is_control())
    };
    let captured_at = DateTime::parse_from_rfc3339(observation.captured_at.trim())
        .ok()
        .map(|value| value.with_timezone(&Utc));
    let now = Utc::now();
    let valid = !valid_identifier(&observation.run_id)
        || captured_at.is_none()
        || captured_at.is_some_and(|value| {
            value > now + chrono::Duration::minutes(5)
                || value < now - chrono::Duration::days(7)
        })
        || !valid_identifier(&observation.captured_at)
        || !valid_identifier(&observation.task_class)
        || !valid_identifier(&observation.baseline_model)
        || !valid_identifier(&observation.candidate_model)
        || observation
            .baseline_model
            .trim()
            .eq_ignore_ascii_case(observation.candidate_model.trim())
        || observation.quality_score_bps > 10_000
        || observation.latency_ms > i64::MAX as u64
        || observation
            .successful_task_cost_microunits
            .is_some_and(|value| value > i64::MAX as u64);
    if valid {
        return Err(rusqlite::Error::InvalidParameterName(
            "invalid redacted model-routing evidence observation".to_string(),
        ));
    }
    let captured_at = captured_at
        .expect("validated model-routing evidence timestamp")
        .to_rfc3339();
    if observation.succeeded && observation.successful_task_cost_microunits.is_none() {
        return Err(rusqlite::Error::InvalidParameterName(
            "successful model-routing observations require cost evidence".to_string(),
        ));
    }
    let arm = match observation.arm {
        ModelRoutingEvidenceArm::Baseline => "baseline",
        ModelRoutingEvidenceArm::Candidate => "candidate",
    };
    let conn = super::open_connection()?;
    let existing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM model_routing_evidence_events WHERE run_id = ?1",
        params![observation.run_id.trim()],
        |row| row.get(0),
    )?;
    if existing >= MAX_MODEL_ROUTING_EVIDENCE_EVENTS_PER_RUN {
        return Err(rusqlite::Error::InvalidParameterName(
            "model-routing evidence run exceeds its bounded event limit".to_string(),
        ));
    }
    let mut run_rows = conn.prepare(
        "SELECT task_class, baseline_model, candidate_model
         FROM model_routing_evidence_events WHERE run_id = ?1",
    )?;
    let mut rows = run_rows.query(params![observation.run_id.trim()])?;
    while let Some(row) = rows.next()? {
        let existing_task: String = row.get(0)?;
        let existing_baseline: String = row.get(1)?;
        let existing_candidate: String = row.get(2)?;
        if existing_task != observation.task_class.trim().to_ascii_lowercase()
            || existing_baseline != observation.baseline_model.trim()
            || existing_candidate != observation.candidate_model.trim()
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "model-routing evidence run cannot mix task or model identities".to_string(),
            ));
        }
    }
    let duplicate: i64 = conn.query_row(
        "SELECT COUNT(*) FROM model_routing_evidence_events
         WHERE run_id = ?1 AND captured_at = ?2 AND task_class = ?3 AND arm = ?4",
        params![
            observation.run_id.trim(),
            captured_at,
            observation.task_class.trim().to_ascii_lowercase(),
            arm,
        ],
        |row| row.get(0),
    )?;
    if duplicate > 0 {
        return Err(rusqlite::Error::InvalidParameterName(
            "duplicate model-routing evidence observation".to_string(),
        ));
    }
    conn.execute(
        "INSERT INTO model_routing_evidence_events (
            run_id, captured_at, task_class, arm, baseline_model, candidate_model,
            succeeded, successful_task_cost_microunits, quality_score_bps, latency_ms,
            follow_up_rework
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            observation.run_id.trim(),
            captured_at,
            observation.task_class.trim().to_ascii_lowercase(),
            arm,
            observation.baseline_model.trim(),
            observation.candidate_model.trim(),
            i64::from(observation.succeeded),
            observation.successful_task_cost_microunits.map(|value| value as i64),
            observation.quality_score_bps as i64,
            observation.latency_ms as i64,
            i64::from(observation.follow_up_rework),
        ],
    )?;
    Ok(())
}

/// Reconcile one complete run into the machine-readable evidence shape used
/// by the repository checker. Local runtime evidence is permanently
/// observe-only; it cannot turn on automatic routing by itself.
pub(crate) fn export_model_routing_evidence(
    run_id: &str,
    task_class: &str,
) -> Result<ModelRoutingEvidenceArtifact, String> {
    let run_id = run_id.trim();
    let task_class = task_class.trim().to_ascii_lowercase();
    if run_id.is_empty() || task_class.is_empty() {
        return Err("model-routing evidence export requires run_id and task_class".to_string());
    }
    let conn = super::open_connection().map_err(|error| format!("open model-routing evidence: {error}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT captured_at, arm, baseline_model, candidate_model, succeeded,
                    successful_task_cost_microunits, quality_score_bps, latency_ms,
                    follow_up_rework
             FROM model_routing_evidence_events
             WHERE run_id = ?1 AND task_class = ?2
             ORDER BY id ASC",
        )
        .map_err(|error| format!("prepare model-routing evidence export: {error}"))?;
    let rows = stmt
        .query_map(params![run_id, task_class], |row| {
            let captured_at: String = row.get(0)?;
            let succeeded: i64 = row.get(4)?;
            let successful_task_cost_microunits: Option<i64> = row.get(5)?;
            let quality_score_bps: i64 = row.get(6)?;
            let latency_ms: i64 = row.get(7)?;
            let follow_up_rework: i64 = row.get(8)?;
            validate_export_row(
                &captured_at,
                succeeded,
                successful_task_cost_microunits,
                quality_score_bps,
                latency_ms,
                follow_up_rework,
            )?;
            let arm: String = row.get(1)?;
            let arm = match arm.as_str() {
                "baseline" => ModelRoutingEvidenceArm::Baseline,
                "candidate" => ModelRoutingEvidenceArm::Candidate,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok(ModelRoutingEvidenceObservation {
                run_id: run_id.to_string(),
                captured_at,
                task_class: task_class.clone(),
                arm,
                baseline_model: row.get(2)?,
                candidate_model: row.get(3)?,
                succeeded: succeeded != 0,
                successful_task_cost_microunits: successful_task_cost_microunits.map(|v| v as u64),
                quality_score_bps: quality_score_bps as u32,
                latency_ms: latency_ms as u64,
                follow_up_rework: follow_up_rework != 0,
            })
        })
        .map_err(|error| format!("query model-routing evidence export: {error}"))?;
    let observations = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| format!("read model-routing evidence export: {error}"))?;
    if observations.is_empty() {
        return Err("model-routing evidence export has no observations".to_string());
    }
    let baseline_model = observations[0].baseline_model.clone();
    let candidate_model = observations[0].candidate_model.clone();
    if observations.iter().any(|observation| {
        observation.baseline_model != baseline_model || observation.candidate_model != candidate_model
    }) {
        return Err("model-routing evidence export mixes model identities".to_string());
    }
    let captured_at = observations
        .iter()
        .filter_map(|observation| {
            DateTime::parse_from_rfc3339(observation.captured_at.trim())
                .ok()
                .map(|parsed| (parsed.with_timezone(&Utc), observation.captured_at.clone()))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, captured_at)| captured_at)
        .ok_or_else(|| "model-routing evidence export contains no valid timestamps".to_string())?;
    let samples = observations.iter().map(|observation| observation.sample()).collect::<Vec<_>>();
    let evidence = aggregate_model_routing_evidence(&samples, &task_class)?;
    let minimum_samples =
        crate::optimization::model_routing::load_model_routing_experiment_policy()
            .thresholds
            .minimum_sample_size;
    let rate = |successes: u64| -> u32 {
        ((successes.saturating_mul(10_000)) / evidence.sample_size.max(1)) as u32
    };
    let rework = |arm: ModelRoutingEvidenceArm| -> u32 {
        let selected = samples.iter().filter(|sample| sample.arm == arm).collect::<Vec<_>>();
        ((selected.iter().filter(|sample| sample.follow_up_rework).count() as u64 * 10_000)
            / selected.len().max(1) as u64) as u32
    };
    let quality = |arm: ModelRoutingEvidenceArm| -> u32 {
        let selected = samples.iter().filter(|sample| sample.arm == arm).collect::<Vec<_>>();
        (selected.iter().map(|sample| sample.quality_score_bps as u64).sum::<u64>()
            / selected.len().max(1) as u64) as u32
    };
    let baseline_count = samples.iter().filter(|sample| sample.arm == ModelRoutingEvidenceArm::Baseline).count() as u64;
    let candidate_count = samples.iter().filter(|sample| sample.arm == ModelRoutingEvidenceArm::Candidate).count() as u64;
    Ok(ModelRoutingEvidenceArtifact {
        schema_version: 1,
        evidence_class: "local_runtime_observation",
        minimum_samples,
        baseline: ModelRoutingEvidenceArmMetrics {
            sample_count: baseline_count,
            successful_task_count: evidence.baseline_successes,
            success_rate_bps: rate(evidence.baseline_successes),
            quality_score_bps: quality(ModelRoutingEvidenceArm::Baseline),
            p95_latency_ms: evidence.baseline_p95_latency_ms,
            successful_task_cost_micros: evidence.baseline_average_success_cost_microunits,
            follow_up_rework_rate_bps: rework(ModelRoutingEvidenceArm::Baseline),
        },
        candidate: ModelRoutingEvidenceArmMetrics {
            sample_count: candidate_count,
            successful_task_count: evidence.candidate_successes,
            success_rate_bps: rate(evidence.candidate_successes),
            quality_score_bps: quality(ModelRoutingEvidenceArm::Candidate),
            p95_latency_ms: evidence.candidate_p95_latency_ms,
            successful_task_cost_micros: evidence.candidate_average_success_cost_microunits,
            follow_up_rework_rate_bps: evidence.follow_up_rework_rate_bps,
        },
        provenance: ModelRoutingEvidenceProvenance {
            task_class,
            baseline_model,
            candidate_model,
            source: "local_runtime_observation",
            cost_attribution: "local_estimate",
            provider_id: None,
            run_id: run_id.to_string(),
            captured_at,
        },
        promotion_eligible: false,
    })
}
