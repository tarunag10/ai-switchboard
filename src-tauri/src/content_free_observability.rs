//! Content-free observability contract for optimization requests.
//!
//! This module creates local OpenTelemetry-style span data and Prometheus-style
//! metric samples. It has no network exporter and its typed schema contains no
//! prompt, response, header, credential, or arbitrary error fields.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const EVENT_SCHEMA_VERSION: u8 = 1;
const MAX_IDENTIFIER_LEN: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RequestStatus {
    Success,
    Failure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FailureReason {
    ClientAdapterUnavailable,
    EngineUnavailable,
    EndpointUnhealthy,
    ModelUnavailable,
    PolicyRejected,
    AuthenticationRejected,
    RateLimited,
    Timeout,
    UpstreamFailure,
    VerificationFailed,
    InternalFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CacheResult {
    Hit,
    Miss,
    Bypass,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompressionResult {
    Applied,
    Bypassed,
    Failed,
    NotRequested,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenMeasurements {
    pub before: u64,
    pub after: u64,
    pub cache_read: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LatencyMeasurements {
    pub optimizer_micros: u64,
    pub ttft_micros: Option<u64>,
    pub inter_token_micros: Option<u64>,
    pub end_to_end_micros: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContentFreeTelemetryEvent {
    schema_version: u8,
    pub request_id: String,
    pub client_adapter: String,
    pub optimization_profile: String,
    pub engine: String,
    pub action: String,
    pub endpoint: String,
    pub model: String,
    pub tokens: TokenMeasurements,
    pub latency: LatencyMeasurements,
    pub cache_result: CacheResult,
    pub compression_result: CompressionResult,
    pub status: RequestStatus,
    pub failure_reason: Option<FailureReason>,
    pub quality_outcome_reference: Option<String>,
}

impl ContentFreeTelemetryEvent {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        request_id: impl Into<String>,
        client_adapter: impl Into<String>,
        optimization_profile: impl Into<String>,
        engine: impl Into<String>,
        action: impl Into<String>,
        endpoint: impl Into<String>,
        model: impl Into<String>,
        tokens: TokenMeasurements,
        latency: LatencyMeasurements,
        cache_result: CacheResult,
        compression_result: CompressionResult,
        status: RequestStatus,
        failure_reason: Option<FailureReason>,
        quality_outcome_reference: Option<String>,
    ) -> Result<Self, String> {
        let event = Self {
            schema_version: EVENT_SCHEMA_VERSION,
            request_id: request_id.into(),
            client_adapter: client_adapter.into(),
            optimization_profile: optimization_profile.into(),
            engine: engine.into(),
            action: action.into(),
            endpoint: endpoint.into(),
            model: model.into(),
            tokens,
            latency,
            cache_result,
            compression_result,
            status,
            failure_reason,
            quality_outcome_reference,
        };
        event.validate()?;
        Ok(event)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.schema_version != EVENT_SCHEMA_VERSION {
            return Err("unsupported content-free telemetry schema".to_string());
        }
        for (name, value) in [
            ("request_id", self.request_id.as_str()),
            ("client_adapter", self.client_adapter.as_str()),
            ("optimization_profile", self.optimization_profile.as_str()),
            ("engine", self.engine.as_str()),
            ("action", self.action.as_str()),
            ("endpoint", self.endpoint.as_str()),
            ("model", self.model.as_str()),
        ] {
            validate_identifier(name, value)?;
        }
        if let Some(reference) = &self.quality_outcome_reference {
            validate_identifier("quality_outcome_reference", reference)?;
        }
        match (self.status, self.failure_reason) {
            (RequestStatus::Success, None) | (RequestStatus::Failure, Some(_)) => {}
            (RequestStatus::Success, Some(_)) => {
                return Err("successful event must not contain a failure reason".to_string())
            }
            (RequestStatus::Failure, None) => {
                return Err("failed event requires a typed failure reason".to_string())
            }
        }
        Ok(())
    }
}

fn validate_identifier(name: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    let lowercase = trimmed.to_ascii_lowercase();
    let secret_like = ["bearer ", "api_key", "apikey", "secret=", "token="]
        .iter()
        .any(|needle| lowercase.contains(needle))
        || lowercase.starts_with("sk-");
    if trimmed.is_empty()
        || trimmed.len() > MAX_IDENTIFIER_LEN
        || trimmed != value
        || trimmed.chars().any(char::is_control)
        || secret_like
    {
        return Err(format!("{name} is not a safe content-free identifier"));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum OtelAttributeValue {
    Text(String),
    Integer(u64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OtelSpanRecord {
    pub name: String,
    pub attributes: BTreeMap<String, OtelAttributeValue>,
    pub status: RequestStatus,
}

pub(crate) fn to_otel_span(event: &ContentFreeTelemetryEvent) -> Result<OtelSpanRecord, String> {
    event.validate()?;
    let mut attributes = BTreeMap::from([
        ("request.id".to_string(), text(&event.request_id)),
        (
            "ai.switchboard.client_adapter".to_string(),
            text(&event.client_adapter),
        ),
        (
            "ai.switchboard.optimization_profile".to_string(),
            text(&event.optimization_profile),
        ),
        ("ai.switchboard.engine".to_string(), text(&event.engine)),
        ("ai.switchboard.action".to_string(), text(&event.action)),
        ("ai.switchboard.endpoint".to_string(), text(&event.endpoint)),
        ("ai.switchboard.model".to_string(), text(&event.model)),
        (
            "ai.switchboard.tokens.before".to_string(),
            integer(event.tokens.before),
        ),
        (
            "ai.switchboard.tokens.after".to_string(),
            integer(event.tokens.after),
        ),
        (
            "ai.switchboard.tokens.cache_read".to_string(),
            integer(event.tokens.cache_read),
        ),
        (
            "ai.switchboard.latency.optimizer_us".to_string(),
            integer(event.latency.optimizer_micros),
        ),
        (
            "ai.switchboard.latency.e2e_us".to_string(),
            integer(event.latency.end_to_end_micros),
        ),
        (
            "ai.switchboard.cache_result".to_string(),
            text(cache_result_name(event.cache_result)),
        ),
        (
            "ai.switchboard.compression_result".to_string(),
            text(compression_result_name(event.compression_result)),
        ),
        (
            "ai.switchboard.status".to_string(),
            text(status_name(event.status)),
        ),
    ]);
    if let Some(value) = event.latency.ttft_micros {
        attributes.insert("ai.switchboard.latency.ttft_us".to_string(), integer(value));
    }
    if let Some(value) = event.latency.inter_token_micros {
        attributes.insert(
            "ai.switchboard.latency.inter_token_us".to_string(),
            integer(value),
        );
    }
    if let Some(reason) = event.failure_reason {
        attributes.insert("error.type".to_string(), text(failure_reason_name(reason)));
    }
    if let Some(reference) = &event.quality_outcome_reference {
        attributes.insert(
            "ai.switchboard.quality_outcome_reference".to_string(),
            text(reference),
        );
    }
    Ok(OtelSpanRecord {
        name: "ai_switchboard.optimization.request".to_string(),
        attributes,
        status: event.status,
    })
}

fn text(value: impl Into<String>) -> OtelAttributeValue {
    OtelAttributeValue::Text(value.into())
}

fn integer(value: u64) -> OtelAttributeValue {
    OtelAttributeValue::Integer(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrometheusMetricKind {
    Counter,
    HistogramObservation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrometheusMetricSample {
    pub name: String,
    pub kind: PrometheusMetricKind,
    pub value: f64,
    pub labels: BTreeMap<String, String>,
    /// Request IDs are exemplars, never high-cardinality metric labels.
    pub exemplar_request_id: String,
}

pub(crate) fn to_prometheus_samples(
    event: &ContentFreeTelemetryEvent,
) -> Result<Vec<PrometheusMetricSample>, String> {
    event.validate()?;
    let labels = BTreeMap::from([
        ("client_adapter".to_string(), event.client_adapter.clone()),
        ("profile".to_string(), event.optimization_profile.clone()),
        ("engine".to_string(), event.engine.clone()),
        ("action".to_string(), event.action.clone()),
        ("endpoint".to_string(), event.endpoint.clone()),
        ("model".to_string(), event.model.clone()),
        (
            "cache_result".to_string(),
            cache_result_name(event.cache_result).to_string(),
        ),
        (
            "compression_result".to_string(),
            compression_result_name(event.compression_result).to_string(),
        ),
        ("status".to_string(), status_name(event.status).to_string()),
        (
            "failure_reason".to_string(),
            event
                .failure_reason
                .map(failure_reason_name)
                .unwrap_or("none")
                .to_string(),
        ),
    ]);
    let counter = |name: &str, value: u64| PrometheusMetricSample {
        name: name.to_string(),
        kind: PrometheusMetricKind::Counter,
        value: value as f64,
        labels: labels.clone(),
        exemplar_request_id: event.request_id.clone(),
    };
    let histogram = |name: &str, micros: u64| PrometheusMetricSample {
        name: name.to_string(),
        kind: PrometheusMetricKind::HistogramObservation,
        value: micros as f64 / 1_000_000.0,
        labels: labels.clone(),
        exemplar_request_id: event.request_id.clone(),
    };
    let mut samples = vec![
        counter("ai_switchboard_requests_total", 1),
        counter("ai_switchboard_tokens_before_total", event.tokens.before),
        counter("ai_switchboard_tokens_after_total", event.tokens.after),
        counter(
            "ai_switchboard_cache_read_tokens_total",
            event.tokens.cache_read,
        ),
        histogram(
            "ai_switchboard_optimizer_latency_seconds",
            event.latency.optimizer_micros,
        ),
        histogram(
            "ai_switchboard_end_to_end_latency_seconds",
            event.latency.end_to_end_micros,
        ),
    ];
    if let Some(value) = event.latency.ttft_micros {
        samples.push(histogram("ai_switchboard_ttft_seconds", value));
    }
    if let Some(value) = event.latency.inter_token_micros {
        samples.push(histogram(
            "ai_switchboard_inter_token_latency_seconds",
            value,
        ));
    }
    Ok(samples)
}

fn status_name(value: RequestStatus) -> &'static str {
    match value {
        RequestStatus::Success => "success",
        RequestStatus::Failure => "failure",
    }
}
fn cache_result_name(value: CacheResult) -> &'static str {
    match value {
        CacheResult::Hit => "hit",
        CacheResult::Miss => "miss",
        CacheResult::Bypass => "bypass",
        CacheResult::Unavailable => "unavailable",
    }
}
fn compression_result_name(value: CompressionResult) -> &'static str {
    match value {
        CompressionResult::Applied => "applied",
        CompressionResult::Bypassed => "bypassed",
        CompressionResult::Failed => "failed",
        CompressionResult::NotRequested => "not_requested",
    }
}
fn failure_reason_name(value: FailureReason) -> &'static str {
    match value {
        FailureReason::ClientAdapterUnavailable => "client_adapter_unavailable",
        FailureReason::EngineUnavailable => "engine_unavailable",
        FailureReason::EndpointUnhealthy => "endpoint_unhealthy",
        FailureReason::ModelUnavailable => "model_unavailable",
        FailureReason::PolicyRejected => "policy_rejected",
        FailureReason::AuthenticationRejected => "authentication_rejected",
        FailureReason::RateLimited => "rate_limited",
        FailureReason::Timeout => "timeout",
        FailureReason::UpstreamFailure => "upstream_failure",
        FailureReason::VerificationFailed => "verification_failed",
        FailureReason::InternalFailure => "internal_failure",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ContentFreeTelemetryEvent {
        let event: ContentFreeTelemetryEvent = serde_json::from_str(include_str!(
            "../../benchmarks/fixtures/content-free-observability-event.json"
        ))
        .unwrap();
        event.validate().unwrap();
        event
    }

    #[test]
    fn fixture_projects_all_required_fields_to_otel_without_content() {
        let span = to_otel_span(&fixture()).unwrap();
        let encoded = serde_json::to_string(&span).unwrap();
        for required in [
            "request.id",
            "client_adapter",
            "engine",
            "action",
            "endpoint",
            "model",
            "tokens.before",
            "latency.e2e_us",
            "cache_result",
            "compression_result",
            "status",
        ] {
            assert!(encoded.contains(required), "missing {required}");
        }
        for forbidden in ["prompt", "response", "authorization", "api_key", "secret"] {
            assert!(!encoded.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn prometheus_projection_uses_request_id_only_as_an_exemplar() {
        let samples = to_prometheus_samples(&fixture()).unwrap();
        assert_eq!(samples.len(), 8);
        assert!(samples
            .iter()
            .all(|sample| sample.exemplar_request_id == "req-01HXYZ"));
        assert!(samples
            .iter()
            .all(|sample| !sample.labels.contains_key("request_id")));
        assert!(samples
            .iter()
            .any(|sample| sample.name == "ai_switchboard_ttft_seconds"));
    }

    #[test]
    fn failure_requires_a_typed_reason_and_success_forbids_one() {
        let mut event = fixture();
        event.status = RequestStatus::Failure;
        assert!(event.validate().is_err());
        event.failure_reason = Some(FailureReason::EndpointUnhealthy);
        assert!(event.validate().is_ok());
        event.status = RequestStatus::Success;
        assert!(event.validate().is_err());
    }

    #[test]
    fn secret_like_or_free_form_identifiers_are_rejected() {
        let mut event = fixture();
        event.endpoint = "sk-private-key".to_string();
        assert!(event.validate().is_err());
        event = fixture();
        event.model = "Bearer credential".to_string();
        assert!(event.validate().is_err());
        event = fixture();
        event.action = "route\nraw prompt".to_string();
        assert!(event.validate().is_err());
    }

    #[test]
    fn constructor_enforces_the_same_content_free_contract() {
        let source = fixture();
        let rebuilt = ContentFreeTelemetryEvent::new(
            source.request_id,
            source.client_adapter,
            source.optimization_profile,
            source.engine,
            source.action,
            source.endpoint,
            source.model,
            source.tokens,
            source.latency,
            source.cache_result,
            source.compression_result,
            source.status,
            source.failure_reason,
            source.quality_outcome_reference,
        );
        assert!(rebuilt.is_ok());
    }
}
