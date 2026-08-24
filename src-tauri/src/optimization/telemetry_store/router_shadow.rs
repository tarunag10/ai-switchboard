use std::{fmt, time::Duration as StdDuration};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(crate) const ROUTER_SHADOW_SCHEMA_VERSION: u32 = 1;
pub(crate) const MAX_ROUTER_SHADOW_RUNS_PER_GENERATION: i64 = 256;
pub(crate) const ROUTER_SHADOW_COMPLETION_WINDOW_HOURS: i64 = 24;
const ROUTER_SHADOW_COMPLETION_WINDOW_MS: u64 =
    ROUTER_SHADOW_COMPLETION_WINDOW_HOURS as u64 * 60 * 60 * 1_000;

const SQLITE_BUSY_TIMEOUT_SECONDS: u64 = 5;
const DECISION_ID_PREFIX: &str = "router-shadow-decision-";
const RUN_ID_PREFIX: &str = "router-run-";
const COMPLETION_ID_PREFIX: &str = "router-shadow-completion-";

#[derive(Debug)]
pub(crate) enum RouterShadowStoreError {
    Validation(String),
    DuplicateDecision,
    DuplicateCompletion,
    UnknownDecision,
    Expired,
    Corrupt(String),
    Sql(rusqlite::Error),
}

impl fmt::Display for RouterShadowStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => {
                write!(formatter, "invalid Router shadow record: {message}")
            }
            Self::DuplicateDecision => formatter.write_str("Router shadow decision already exists"),
            Self::DuplicateCompletion => {
                formatter.write_str("Router shadow run already has a completion")
            }
            Self::UnknownDecision => formatter.write_str("Router shadow decision is unknown"),
            Self::Expired => {
                formatter.write_str("Router shadow decision expired before completion")
            }
            Self::Corrupt(message) => write!(formatter, "corrupt Router shadow state: {message}"),
            Self::Sql(error) => write!(formatter, "Router shadow SQLite error: {error}"),
        }
    }
}

impl std::error::Error for RouterShadowStoreError {}

impl From<rusqlite::Error> for RouterShadowStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

macro_rules! fixed_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        pub(crate) enum $name {
            $(#[serde(rename = $value)] $variant),+
        }

        impl $name {
            fn as_db(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }

            fn from_db(value: &str) -> Result<Self, RouterShadowStoreError> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(RouterShadowStoreError::Corrupt(format!(
                        "unknown {} value",
                        stringify!($name)
                    ))),
                }
            }
        }
    };
}

fixed_enum!(RouterShadowSource {
    NativeProxyDirect => "native_proxy_direct",
});

fixed_enum!(RouterShadowMode {
    ObserveOnly => "observe_only",
});

fixed_enum!(RouterShadowClientClass {
    Codex => "codex",
    ClaudeCode => "claude_code",
    Cursor => "cursor",
    Continue => "continue",
    Aider => "aider",
    OtherManaged => "other_managed",
    Unknown => "unknown",
});

fixed_enum!(RouterShadowRequestClass {
    OpenAiResponses => "openai_responses",
    OpenAiChatCompletions => "openai_chat_completions",
    AnthropicMessages => "anthropic_messages",
});

fixed_enum!(RouterShadowDecisionStage {
    Observe => "observe",
});

fixed_enum!(RouterShadowTaskClass {
    CodeGeneration => "code_generation",
    CodeReview => "code_review",
    Debugging => "debugging",
    Planning => "planning",
    Research => "research",
    General => "general",
    Unclassified => "unclassified",
});

fixed_enum!(RouterShadowTaskClassificationSource {
    FixedRules => "fixed_rules",
    Unclassified => "unclassified",
});

fixed_enum!(RouterShadowFeature {
    Streaming => "streaming",
    Tools => "tools",
    ParallelTools => "parallel_tools",
    StructuredOutput => "structured_output",
    Vision => "vision",
});

impl RouterShadowFeature {
    const fn canonical_rank(self) -> u8 {
        match self {
            Self::Streaming => 0,
            Self::Tools => 1,
            Self::ParallelTools => 2,
            Self::StructuredOutput => 3,
            Self::Vision => 4,
        }
    }
}

fixed_enum!(RouterShadowStreamingIntent {
    Streaming => "streaming",
    Unary => "unary",
    Unknown => "unknown",
    HeaderBodyMismatch => "header_body_mismatch",
});

fixed_enum!(RouterShadowExecutionModelInvariant {
    PreserveRequested => "preserve_requested",
});

fixed_enum!(RouterShadowModelProvenance {
    VerifiedEndpointCatalog => "verified_endpoint_catalog",
    ProviderResponseCatalogMatch => "provider_response_catalog_match",
});

fixed_enum!(RouterShadowTransport {
    DirectOpenAi => "direct_open_ai",
    DirectAnthropic => "direct_anthropic",
});

fixed_enum!(RouterShadowOutcome {
    Completed => "completed",
    HttpError => "http_error",
    ConnectFailure => "connect_failure",
    StreamFailure => "stream_failure",
    ClientDisconnected => "client_disconnected",
    Timeout => "timeout",
    IntegrityFailure => "integrity_failure",
    ClockAnomaly => "clock_anomaly",
});

fixed_enum!(RouterShadowCostEvidenceState {
    Unavailable => "unavailable",
    ProviderReported => "provider_reported",
});

fixed_enum!(RouterShadowQualityEvidenceState {
    NotCollected => "not_collected",
});

fixed_enum!(RouterShadowReworkEvidenceState {
    NotCollected => "not_collected",
});

fixed_enum!(RouterShadowFailureClass {
    HttpStatus => "http_status",
    Connect => "connect",
    Timeout => "timeout",
    UpstreamRead => "upstream_read",
    DownstreamWrite => "downstream_write",
    ClientDisconnect => "client_disconnect",
    Integrity => "integrity",
    ClockRollback => "clock_rollback",
});

fixed_enum!(RouterShadowClockEvidence {
    WallClockObserved => "wall_clock_observed",
    MonotonicObserved => "monotonic_observed",
    ClockRollbackDetected => "clock_rollback_detected",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RouterShadowModelIdentity {
    pub(crate) provenance: RouterShadowModelProvenance,
    pub(crate) identity_digest: String,
}

impl RouterShadowModelIdentity {
    fn validate(&self) -> Result<(), RouterShadowStoreError> {
        validate_digest(&self.identity_digest, "model identity digest")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RouterShadowPrivacyV1 {
    pub(crate) request_body_retained: bool,
    pub(crate) response_body_retained: bool,
    pub(crate) prompt_retained: bool,
    pub(crate) headers_retained: bool,
    pub(crate) credentials_retained: bool,
    pub(crate) tool_payloads_retained: bool,
    pub(crate) filesystem_paths_retained: bool,
    pub(crate) endpoint_urls_retained: bool,
    pub(crate) provider_request_ids_retained: bool,
    pub(crate) request_body_hash_retained: bool,
}

impl RouterShadowPrivacyV1 {
    fn validate(&self) -> Result<(), RouterShadowStoreError> {
        if self.request_body_retained
            || self.response_body_retained
            || self.prompt_retained
            || self.headers_retained
            || self.credentials_retained
            || self.tool_payloads_retained
            || self.filesystem_paths_retained
            || self.endpoint_urls_retained
            || self.provider_request_ids_retained
            || self.request_body_hash_retained
        {
            return Err(RouterShadowStoreError::Validation(
                "privacy attestations must declare every content category unretained".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RouterShadowDecisionDraft {
    pub(crate) source: RouterShadowSource,
    pub(crate) client_class: RouterShadowClientClass,
    pub(crate) request_class: RouterShadowRequestClass,
    pub(crate) policy_digest: String,
    pub(crate) route_plan_digest: String,
    pub(crate) decision_stage: RouterShadowDecisionStage,
    pub(crate) routing_mode: RouterShadowMode,
    pub(crate) task_class: RouterShadowTaskClass,
    pub(crate) task_classification_source: RouterShadowTaskClassificationSource,
    pub(crate) requested_model: RouterShadowModelIdentity,
    pub(crate) proposed_model: Option<RouterShadowModelIdentity>,
    pub(crate) proposed_endpoint_identity_digest: Option<String>,
    pub(crate) required_features: Vec<RouterShadowFeature>,
    pub(crate) streaming_intent: RouterShadowStreamingIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RouterShadowDecisionV1 {
    pub(crate) schema_version: u32,
    pub(crate) generation_id: u64,
    pub(crate) generation_digest: String,
    pub(crate) decision_id: String,
    pub(crate) router_run_id: String,
    pub(crate) decided_at: String,
    pub(crate) source: RouterShadowSource,
    pub(crate) client_class: RouterShadowClientClass,
    pub(crate) request_class: RouterShadowRequestClass,
    pub(crate) policy_digest: String,
    pub(crate) route_plan_digest: String,
    pub(crate) decision_stage: RouterShadowDecisionStage,
    pub(crate) routing_mode: RouterShadowMode,
    pub(crate) task_class: RouterShadowTaskClass,
    pub(crate) task_classification_source: RouterShadowTaskClassificationSource,
    pub(crate) requested_model: RouterShadowModelIdentity,
    pub(crate) proposed_model: Option<RouterShadowModelIdentity>,
    pub(crate) proposed_endpoint_identity_digest: Option<String>,
    pub(crate) required_features: Vec<RouterShadowFeature>,
    pub(crate) streaming_intent: RouterShadowStreamingIntent,
    pub(crate) execution_model_invariant: RouterShadowExecutionModelInvariant,
    pub(crate) route_mutation_applied: bool,
    pub(crate) promotion_eligible: bool,
    pub(crate) privacy: RouterShadowPrivacyV1,
    pub(crate) canonical_digest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalRouterShadowDecision<'a> {
    schema_version: u32,
    generation_id: u64,
    generation_digest: &'a str,
    decision_id: &'a str,
    router_run_id: &'a str,
    decided_at: &'a str,
    source: RouterShadowSource,
    client_class: RouterShadowClientClass,
    request_class: RouterShadowRequestClass,
    policy_digest: &'a str,
    route_plan_digest: &'a str,
    decision_stage: RouterShadowDecisionStage,
    routing_mode: RouterShadowMode,
    task_class: RouterShadowTaskClass,
    task_classification_source: RouterShadowTaskClassificationSource,
    requested_model: &'a RouterShadowModelIdentity,
    proposed_model: Option<&'a RouterShadowModelIdentity>,
    proposed_endpoint_identity_digest: Option<&'a str>,
    required_features: &'a [RouterShadowFeature],
    streaming_intent: RouterShadowStreamingIntent,
    execution_model_invariant: RouterShadowExecutionModelInvariant,
    route_mutation_applied: bool,
    promotion_eligible: bool,
    privacy: &'a RouterShadowPrivacyV1,
}

impl RouterShadowDecisionV1 {
    fn from_draft(
        mut draft: RouterShadowDecisionDraft,
        generation_id: u64,
        generation_digest: String,
        decision_id: String,
        router_run_id: String,
        decided_at: DateTime<Utc>,
    ) -> Result<Self, RouterShadowStoreError> {
        draft
            .required_features
            .sort_by_key(|feature| feature.canonical_rank());
        let mut decision = Self {
            schema_version: ROUTER_SHADOW_SCHEMA_VERSION,
            generation_id,
            generation_digest,
            decision_id,
            router_run_id,
            decided_at: canonical_timestamp(decided_at),
            source: draft.source,
            client_class: draft.client_class,
            request_class: draft.request_class,
            policy_digest: draft.policy_digest,
            route_plan_digest: draft.route_plan_digest,
            decision_stage: draft.decision_stage,
            routing_mode: draft.routing_mode,
            task_class: draft.task_class,
            task_classification_source: draft.task_classification_source,
            requested_model: draft.requested_model,
            proposed_model: draft.proposed_model,
            proposed_endpoint_identity_digest: draft.proposed_endpoint_identity_digest,
            required_features: draft.required_features,
            streaming_intent: draft.streaming_intent,
            execution_model_invariant: RouterShadowExecutionModelInvariant::PreserveRequested,
            route_mutation_applied: false,
            promotion_eligible: false,
            privacy: RouterShadowPrivacyV1::default(),
            canonical_digest: String::new(),
        };
        decision.validate_content()?;
        decision.canonical_digest = decision.expected_digest()?;
        Ok(decision)
    }

    pub(crate) fn validate(&self) -> Result<(), RouterShadowStoreError> {
        self.validate_content()?;
        validate_digest(&self.canonical_digest, "decision digest")?;
        if self.canonical_digest != self.expected_digest()? {
            return Err(RouterShadowStoreError::Corrupt(
                "decision canonical digest mismatch".into(),
            ));
        }
        Ok(())
    }

    fn validate_content(&self) -> Result<(), RouterShadowStoreError> {
        if self.schema_version != ROUTER_SHADOW_SCHEMA_VERSION || self.generation_id == 0 {
            return Err(RouterShadowStoreError::Validation(
                "unsupported decision schema or generation".into(),
            ));
        }
        validate_prefixed_uuid(&self.decision_id, DECISION_ID_PREFIX, "decision ID")?;
        validate_prefixed_uuid(&self.router_run_id, RUN_ID_PREFIX, "router run ID")?;
        require_canonical_timestamp(&self.decided_at, "decision timestamp")?;
        validate_digest(&self.generation_digest, "generation digest")?;
        validate_digest(&self.policy_digest, "policy digest")?;
        validate_digest(&self.route_plan_digest, "route-plan digest")?;
        self.requested_model.validate()?;
        if self.requested_model.provenance != RouterShadowModelProvenance::VerifiedEndpointCatalog {
            return Err(RouterShadowStoreError::Validation(
                "requested model identity requires the verified endpoint catalog".into(),
            ));
        }
        if let Some(model) = &self.proposed_model {
            model.validate()?;
            if model.provenance != RouterShadowModelProvenance::VerifiedEndpointCatalog {
                return Err(RouterShadowStoreError::Validation(
                    "proposed model identity requires the verified endpoint catalog".into(),
                ));
            }
        }
        if let Some(digest) = &self.proposed_endpoint_identity_digest {
            validate_digest(digest, "endpoint identity digest")?;
        }
        if self.proposed_model.is_some() != self.proposed_endpoint_identity_digest.is_some() {
            return Err(RouterShadowStoreError::Validation(
                "proposed model and endpoint identity must be present together".into(),
            ));
        }
        if (self.task_class == RouterShadowTaskClass::Unclassified)
            != (self.task_classification_source
                == RouterShadowTaskClassificationSource::Unclassified)
        {
            return Err(RouterShadowStoreError::Validation(
                "unclassified task and classification source must agree".into(),
            ));
        }
        if self.required_features.len() > 5
            || !self
                .required_features
                .windows(2)
                .all(|values| values[0].canonical_rank() < values[1].canonical_rank())
        {
            return Err(RouterShadowStoreError::Validation(
                "required features must be canonical, unique, and bounded".into(),
            ));
        }
        if self.route_mutation_applied || self.promotion_eligible {
            return Err(RouterShadowStoreError::Validation(
                "shadow decisions cannot mutate routes or become promotion evidence".into(),
            ));
        }
        self.privacy.validate()
    }

    fn expected_digest(&self) -> Result<String, RouterShadowStoreError> {
        canonical_sha256(&CanonicalRouterShadowDecision {
            schema_version: self.schema_version,
            generation_id: self.generation_id,
            generation_digest: &self.generation_digest,
            decision_id: &self.decision_id,
            router_run_id: &self.router_run_id,
            decided_at: &self.decided_at,
            source: self.source,
            client_class: self.client_class,
            request_class: self.request_class,
            policy_digest: &self.policy_digest,
            route_plan_digest: &self.route_plan_digest,
            decision_stage: self.decision_stage,
            routing_mode: self.routing_mode,
            task_class: self.task_class,
            task_classification_source: self.task_classification_source,
            requested_model: &self.requested_model,
            proposed_model: self.proposed_model.as_ref(),
            proposed_endpoint_identity_digest: self.proposed_endpoint_identity_digest.as_deref(),
            required_features: &self.required_features,
            streaming_intent: self.streaming_intent,
            execution_model_invariant: self.execution_model_invariant,
            route_mutation_applied: self.route_mutation_applied,
            promotion_eligible: self.promotion_eligible,
            privacy: &self.privacy,
        })
    }

    #[cfg(test)]
    pub(super) fn canonical_json_for_tests(&self) -> String {
        serde_json::to_string(&CanonicalRouterShadowDecision {
            schema_version: self.schema_version,
            generation_id: self.generation_id,
            generation_digest: &self.generation_digest,
            decision_id: &self.decision_id,
            router_run_id: &self.router_run_id,
            decided_at: &self.decided_at,
            source: self.source,
            client_class: self.client_class,
            request_class: self.request_class,
            policy_digest: &self.policy_digest,
            route_plan_digest: &self.route_plan_digest,
            decision_stage: self.decision_stage,
            routing_mode: self.routing_mode,
            task_class: self.task_class,
            task_classification_source: self.task_classification_source,
            requested_model: &self.requested_model,
            proposed_model: self.proposed_model.as_ref(),
            proposed_endpoint_identity_digest: self.proposed_endpoint_identity_digest.as_deref(),
            required_features: &self.required_features,
            streaming_intent: self.streaming_intent,
            execution_model_invariant: self.execution_model_invariant,
            route_mutation_applied: self.route_mutation_applied,
            promotion_eligible: self.promotion_eligible,
            privacy: &self.privacy,
        })
        .expect("canonical Router shadow decision JSON")
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RouterShadowCompletionDraft {
    pub(crate) router_run_id: String,
    pub(crate) actual_transport: RouterShadowTransport,
    pub(crate) forwarded_model: RouterShadowModelIdentity,
    pub(crate) provider_reported_model: Option<RouterShadowModelIdentity>,
    pub(crate) upstream_started: bool,
    pub(crate) response_headers_received: bool,
    pub(crate) delivery_completed: bool,
    pub(crate) status_code: Option<u16>,
    pub(crate) transport_outcome: RouterShadowOutcome,
    pub(crate) observed_monotonic_latency_ms: Option<u64>,
    pub(crate) cost_evidence_state: RouterShadowCostEvidenceState,
    pub(crate) provider_billed_cost_microunits: Option<u64>,
    pub(crate) failure_class: Option<RouterShadowFailureClass>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RouterShadowCompletionV1 {
    pub(crate) schema_version: u32,
    pub(crate) completion_id: String,
    pub(crate) router_run_id: String,
    pub(crate) completed_at: String,
    pub(crate) actual_transport: RouterShadowTransport,
    pub(crate) forwarded_model: RouterShadowModelIdentity,
    pub(crate) provider_reported_model: Option<RouterShadowModelIdentity>,
    pub(crate) model_preserved: bool,
    pub(crate) upstream_started: bool,
    pub(crate) response_headers_received: bool,
    pub(crate) delivery_completed: bool,
    pub(crate) status_code: Option<u16>,
    pub(crate) transport_outcome: RouterShadowOutcome,
    pub(crate) latency_ms: u64,
    pub(crate) clock_evidence: RouterShadowClockEvidence,
    pub(crate) cost_evidence_state: RouterShadowCostEvidenceState,
    pub(crate) provider_billed_cost_microunits: Option<u64>,
    pub(crate) quality_evidence_state: RouterShadowQualityEvidenceState,
    pub(crate) rework_evidence_state: RouterShadowReworkEvidenceState,
    pub(crate) failure_class: Option<RouterShadowFailureClass>,
    pub(crate) decision_digest: String,
    pub(crate) policy_digest: String,
    pub(crate) route_plan_digest: String,
    pub(crate) canonical_digest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalRouterShadowCompletion<'a> {
    schema_version: u32,
    completion_id: &'a str,
    router_run_id: &'a str,
    completed_at: &'a str,
    actual_transport: RouterShadowTransport,
    forwarded_model: &'a RouterShadowModelIdentity,
    provider_reported_model: Option<&'a RouterShadowModelIdentity>,
    model_preserved: bool,
    upstream_started: bool,
    response_headers_received: bool,
    delivery_completed: bool,
    status_code: Option<u16>,
    transport_outcome: RouterShadowOutcome,
    latency_ms: u64,
    clock_evidence: RouterShadowClockEvidence,
    cost_evidence_state: RouterShadowCostEvidenceState,
    provider_billed_cost_microunits: Option<u64>,
    quality_evidence_state: RouterShadowQualityEvidenceState,
    rework_evidence_state: RouterShadowReworkEvidenceState,
    failure_class: Option<RouterShadowFailureClass>,
    decision_digest: &'a str,
    policy_digest: &'a str,
    route_plan_digest: &'a str,
}

impl RouterShadowCompletionV1 {
    fn from_draft(
        mut draft: RouterShadowCompletionDraft,
        decision: &RouterShadowDecisionV1,
        completion_id: String,
        completed_at: DateTime<Utc>,
    ) -> Result<Self, RouterShadowStoreError> {
        let decided_at = parse_timestamp(&decision.decided_at, "decision timestamp")?;
        let elapsed = completed_at.signed_duration_since(decided_at);
        let model_preserved = draft.forwarded_model == decision.requested_model;
        let (latency_ms, clock_evidence) = match draft.observed_monotonic_latency_ms {
            Some(value) => {
                if value >= ROUTER_SHADOW_COMPLETION_WINDOW_MS {
                    return Err(RouterShadowStoreError::Expired);
                }
                (value, RouterShadowClockEvidence::MonotonicObserved)
            }
            None if elapsed < Duration::zero() => {
                draft.transport_outcome = RouterShadowOutcome::ClockAnomaly;
                draft.failure_class = Some(RouterShadowFailureClass::ClockRollback);
                draft.upstream_started = false;
                draft.response_headers_received = false;
                draft.delivery_completed = false;
                draft.status_code = None;
                draft.provider_reported_model = None;
                draft.cost_evidence_state = RouterShadowCostEvidenceState::Unavailable;
                draft.provider_billed_cost_microunits = None;
                (0, RouterShadowClockEvidence::ClockRollbackDetected)
            }
            None => {
                if elapsed >= Duration::hours(ROUTER_SHADOW_COMPLETION_WINDOW_HOURS) {
                    return Err(RouterShadowStoreError::Expired);
                }
                (
                    elapsed.num_milliseconds() as u64,
                    RouterShadowClockEvidence::WallClockObserved,
                )
            }
        };
        if clock_evidence != RouterShadowClockEvidence::ClockRollbackDetected && !model_preserved {
            draft.transport_outcome = RouterShadowOutcome::IntegrityFailure;
            draft.failure_class = Some(RouterShadowFailureClass::Integrity);
        }
        let mut completion = Self {
            schema_version: ROUTER_SHADOW_SCHEMA_VERSION,
            completion_id,
            router_run_id: draft.router_run_id,
            completed_at: canonical_timestamp(completed_at),
            actual_transport: draft.actual_transport,
            forwarded_model: draft.forwarded_model,
            provider_reported_model: draft.provider_reported_model,
            model_preserved,
            upstream_started: draft.upstream_started,
            response_headers_received: draft.response_headers_received,
            delivery_completed: draft.delivery_completed,
            status_code: draft.status_code,
            transport_outcome: draft.transport_outcome,
            latency_ms,
            clock_evidence,
            cost_evidence_state: draft.cost_evidence_state,
            provider_billed_cost_microunits: draft.provider_billed_cost_microunits,
            quality_evidence_state: RouterShadowQualityEvidenceState::NotCollected,
            rework_evidence_state: RouterShadowReworkEvidenceState::NotCollected,
            failure_class: draft.failure_class,
            decision_digest: decision.canonical_digest.clone(),
            policy_digest: decision.policy_digest.clone(),
            route_plan_digest: decision.route_plan_digest.clone(),
            canonical_digest: String::new(),
        };
        completion.validate_content()?;
        completion.canonical_digest = completion.expected_digest()?;
        Ok(completion)
    }

    pub(crate) fn validate(&self) -> Result<(), RouterShadowStoreError> {
        self.validate_content()?;
        validate_digest(&self.canonical_digest, "completion digest")?;
        if self.canonical_digest != self.expected_digest()? {
            return Err(RouterShadowStoreError::Corrupt(
                "completion canonical digest mismatch".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn integrity_verified(&self) -> bool {
        self.model_preserved
            && !matches!(
                self.transport_outcome,
                RouterShadowOutcome::IntegrityFailure | RouterShadowOutcome::ClockAnomaly
            )
    }

    fn validate_content(&self) -> Result<(), RouterShadowStoreError> {
        if self.schema_version != ROUTER_SHADOW_SCHEMA_VERSION {
            return Err(RouterShadowStoreError::Validation(
                "unsupported completion schema".into(),
            ));
        }
        validate_prefixed_uuid(&self.completion_id, COMPLETION_ID_PREFIX, "completion ID")?;
        validate_prefixed_uuid(&self.router_run_id, RUN_ID_PREFIX, "router run ID")?;
        require_canonical_timestamp(&self.completed_at, "completion timestamp")?;
        self.forwarded_model.validate()?;
        if self.forwarded_model.provenance != RouterShadowModelProvenance::VerifiedEndpointCatalog {
            return Err(RouterShadowStoreError::Validation(
                "forwarded model identity requires the verified endpoint catalog".into(),
            ));
        }
        if let Some(model) = &self.provider_reported_model {
            model.validate()?;
            if model.provenance != RouterShadowModelProvenance::ProviderResponseCatalogMatch {
                return Err(RouterShadowStoreError::Validation(
                    "provider-reported model identity requires a catalog-matched response".into(),
                ));
            }
        }
        validate_digest(&self.decision_digest, "decision digest")?;
        validate_digest(&self.policy_digest, "policy digest")?;
        validate_digest(&self.route_plan_digest, "route-plan digest")?;
        if self.latency_ms > i64::MAX as u64
            || self
                .provider_billed_cost_microunits
                .is_some_and(|value| value > i64::MAX as u64)
        {
            return Err(RouterShadowStoreError::Validation(
                "latency or provider cost exceeds SQLite range".into(),
            ));
        }
        match self.cost_evidence_state {
            RouterShadowCostEvidenceState::Unavailable
                if self.provider_billed_cost_microunits.is_some() =>
            {
                return Err(RouterShadowStoreError::Validation(
                    "unavailable cost evidence cannot contain a value".into(),
                ));
            }
            RouterShadowCostEvidenceState::ProviderReported
                if self.provider_billed_cost_microunits.is_none() || !self.upstream_started =>
            {
                return Err(RouterShadowStoreError::Validation(
                    "provider cost requires an upstream request and value".into(),
                ));
            }
            _ => {}
        }
        if self.provider_reported_model.is_some() && !self.upstream_started {
            return Err(RouterShadowStoreError::Validation(
                "provider model identity requires an upstream request".into(),
            ));
        }
        if self.response_headers_received && !self.upstream_started {
            return Err(RouterShadowStoreError::Validation(
                "response headers require an upstream request".into(),
            ));
        }
        if self.response_headers_received != self.status_code.is_some() {
            return Err(RouterShadowStoreError::Validation(
                "status and response-header evidence must agree".into(),
            ));
        }
        if self
            .status_code
            .is_some_and(|status| !(100..=599).contains(&status))
        {
            return Err(RouterShadowStoreError::Validation(
                "status code is outside the HTTP range".into(),
            ));
        }
        if self.delivery_completed && !self.response_headers_received {
            return Err(RouterShadowStoreError::Validation(
                "completed delivery requires response headers".into(),
            ));
        }
        if self.clock_evidence != RouterShadowClockEvidence::ClockRollbackDetected
            && self.latency_ms >= ROUTER_SHADOW_COMPLETION_WINDOW_MS
        {
            return Err(RouterShadowStoreError::Validation(
                "latency is outside the exclusive completion window".into(),
            ));
        }
        match self.transport_outcome {
            RouterShadowOutcome::Completed => {
                if !self.model_preserved
                    || !self.delivery_completed
                    || !self
                        .status_code
                        .is_some_and(|status| (200..=299).contains(&status))
                    || self.failure_class.is_some()
                {
                    return Err(RouterShadowStoreError::Validation(
                        "completed transport requires preserved model, delivered 2xx, and no failure"
                            .into(),
                    ));
                }
            }
            RouterShadowOutcome::HttpError => {
                if !self.model_preserved
                    || !self
                        .status_code
                        .is_some_and(|status| (300..=599).contains(&status))
                    || self.failure_class != Some(RouterShadowFailureClass::HttpStatus)
                {
                    return Err(RouterShadowStoreError::Validation(
                        "HTTP error evidence is inconsistent".into(),
                    ));
                }
            }
            RouterShadowOutcome::ConnectFailure => {
                if !self.model_preserved
                    || self.upstream_started
                    || self.response_headers_received
                    || self.delivery_completed
                    || self.failure_class != Some(RouterShadowFailureClass::Connect)
                {
                    return Err(RouterShadowStoreError::Validation(
                        "connection failure evidence is inconsistent".into(),
                    ));
                }
            }
            RouterShadowOutcome::Timeout => {
                if !self.model_preserved
                    || self.delivery_completed
                    || self.failure_class != Some(RouterShadowFailureClass::Timeout)
                {
                    return Err(RouterShadowStoreError::Validation(
                        "timeout evidence is inconsistent".into(),
                    ));
                }
            }
            RouterShadowOutcome::StreamFailure => {
                if !self.model_preserved
                    || !self.upstream_started
                    || self.delivery_completed
                    || !matches!(
                        self.failure_class,
                        Some(RouterShadowFailureClass::UpstreamRead)
                            | Some(RouterShadowFailureClass::DownstreamWrite)
                    )
                {
                    return Err(RouterShadowStoreError::Validation(
                        "stream failure evidence is inconsistent".into(),
                    ));
                }
            }
            RouterShadowOutcome::ClientDisconnected => {
                if !self.model_preserved
                    || self.delivery_completed
                    || self.failure_class != Some(RouterShadowFailureClass::ClientDisconnect)
                {
                    return Err(RouterShadowStoreError::Validation(
                        "client disconnect evidence is inconsistent".into(),
                    ));
                }
            }
            RouterShadowOutcome::IntegrityFailure => {
                if self.model_preserved
                    || self.failure_class != Some(RouterShadowFailureClass::Integrity)
                {
                    return Err(RouterShadowStoreError::Validation(
                        "integrity failure requires a model mismatch".into(),
                    ));
                }
            }
            RouterShadowOutcome::ClockAnomaly => {
                if self.clock_evidence != RouterShadowClockEvidence::ClockRollbackDetected
                    || self.latency_ms != 0
                    || self.upstream_started
                    || self.response_headers_received
                    || self.delivery_completed
                    || self.failure_class != Some(RouterShadowFailureClass::ClockRollback)
                {
                    return Err(RouterShadowStoreError::Validation(
                        "clock anomaly evidence is inconsistent".into(),
                    ));
                }
            }
        }
        if self.clock_evidence == RouterShadowClockEvidence::ClockRollbackDetected
            && self.transport_outcome != RouterShadowOutcome::ClockAnomaly
        {
            return Err(RouterShadowStoreError::Validation(
                "clock rollback evidence requires the clock-anomaly terminal".into(),
            ));
        }
        Ok(())
    }

    fn expected_digest(&self) -> Result<String, RouterShadowStoreError> {
        canonical_sha256(&CanonicalRouterShadowCompletion {
            schema_version: self.schema_version,
            completion_id: &self.completion_id,
            router_run_id: &self.router_run_id,
            completed_at: &self.completed_at,
            actual_transport: self.actual_transport,
            forwarded_model: &self.forwarded_model,
            provider_reported_model: self.provider_reported_model.as_ref(),
            model_preserved: self.model_preserved,
            upstream_started: self.upstream_started,
            response_headers_received: self.response_headers_received,
            delivery_completed: self.delivery_completed,
            status_code: self.status_code,
            transport_outcome: self.transport_outcome,
            latency_ms: self.latency_ms,
            clock_evidence: self.clock_evidence,
            cost_evidence_state: self.cost_evidence_state,
            provider_billed_cost_microunits: self.provider_billed_cost_microunits,
            quality_evidence_state: self.quality_evidence_state,
            rework_evidence_state: self.rework_evidence_state,
            failure_class: self.failure_class,
            decision_digest: &self.decision_digest,
            policy_digest: &self.policy_digest,
            route_plan_digest: &self.route_plan_digest,
        })
    }

    #[cfg(test)]
    pub(super) fn canonical_json_for_tests(&self) -> String {
        serde_json::to_string(&CanonicalRouterShadowCompletion {
            schema_version: self.schema_version,
            completion_id: &self.completion_id,
            router_run_id: &self.router_run_id,
            completed_at: &self.completed_at,
            actual_transport: self.actual_transport,
            forwarded_model: &self.forwarded_model,
            provider_reported_model: self.provider_reported_model.as_ref(),
            model_preserved: self.model_preserved,
            upstream_started: self.upstream_started,
            response_headers_received: self.response_headers_received,
            delivery_completed: self.delivery_completed,
            status_code: self.status_code,
            transport_outcome: self.transport_outcome,
            latency_ms: self.latency_ms,
            clock_evidence: self.clock_evidence,
            cost_evidence_state: self.cost_evidence_state,
            provider_billed_cost_microunits: self.provider_billed_cost_microunits,
            quality_evidence_state: self.quality_evidence_state,
            rework_evidence_state: self.rework_evidence_state,
            failure_class: self.failure_class,
            decision_digest: &self.decision_digest,
            policy_digest: &self.policy_digest,
            route_plan_digest: &self.route_plan_digest,
        })
        .expect("canonical Router shadow completion JSON")
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalRouterShadowGeneration<'a> {
    schema_version: u32,
    generation_id: u64,
    created_at: &'a str,
}

fn router_shadow_generation_digest(
    generation_id: u64,
    created_at: &str,
) -> Result<String, RouterShadowStoreError> {
    canonical_sha256(&CanonicalRouterShadowGeneration {
        schema_version: ROUTER_SHADOW_SCHEMA_VERSION,
        generation_id,
        created_at,
    })
}

fn insert_router_shadow_generation(
    connection: &Connection,
    generation_id: i64,
    now: DateTime<Utc>,
) -> Result<String, RouterShadowStoreError> {
    let generation_id_u64 = to_u64(generation_id, "generation ID")?;
    let created_at = canonical_timestamp(now);
    let canonical_digest = router_shadow_generation_digest(generation_id_u64, &created_at)?;
    connection.execute(
        "INSERT INTO router_shadow_generations (\
            schema_version, generation_id, created_at, canonical_digest\
         ) VALUES (?1, ?2, ?3, ?4)",
        params![
            ROUTER_SHADOW_SCHEMA_VERSION as i64,
            generation_id,
            created_at,
            &canonical_digest,
        ],
    )?;
    Ok(canonical_digest)
}

fn validate_router_shadow_generation(
    connection: &Connection,
    generation_id: u64,
) -> Result<String, RouterShadowStoreError> {
    let row = connection
        .query_row(
            "SELECT schema_version, created_at, canonical_digest \
             FROM router_shadow_generations WHERE generation_id = ?1",
            params![generation_id as i64],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((schema_version, created_at, canonical_digest)) = row else {
        return Err(RouterShadowStoreError::Corrupt(
            "decision references a missing generation".into(),
        ));
    };
    if to_u32(schema_version, "generation schema")? != ROUTER_SHADOW_SCHEMA_VERSION {
        return Err(RouterShadowStoreError::Corrupt(
            "unsupported generation schema".into(),
        ));
    }
    require_canonical_timestamp(&created_at, "generation timestamp")?;
    validate_digest(&canonical_digest, "generation digest")?;
    if canonical_digest != router_shadow_generation_digest(generation_id, &created_at)? {
        return Err(RouterShadowStoreError::Corrupt(
            "generation canonical digest mismatch".into(),
        ));
    }
    Ok(canonical_digest)
}

pub(crate) fn initialize_router_shadow_schema(
    connection: &Connection,
) -> Result<(), RouterShadowStoreError> {
    connection.busy_timeout(StdDuration::from_secs(SQLITE_BUSY_TIMEOUT_SECONDS))?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS router_shadow_generations (
            schema_version INTEGER NOT NULL,
            generation_id INTEGER PRIMARY KEY,
            created_at TEXT NOT NULL,
            canonical_digest TEXT NOT NULL UNIQUE,
            CHECK (schema_version = 1),
            CHECK (generation_id > 0)
         );
         CREATE TABLE IF NOT EXISTS router_shadow_decisions (
            schema_version INTEGER NOT NULL,
            generation_id INTEGER NOT NULL,
            generation_digest TEXT NOT NULL,
            decision_id TEXT PRIMARY KEY,
            router_run_id TEXT NOT NULL UNIQUE,
            decided_at TEXT NOT NULL,
            source TEXT NOT NULL,
            client_class TEXT NOT NULL,
            request_class TEXT NOT NULL,
            policy_digest TEXT NOT NULL,
            route_plan_digest TEXT NOT NULL,
            decision_stage TEXT NOT NULL,
            routing_mode TEXT NOT NULL,
            task_class TEXT NOT NULL,
            task_classification_source TEXT NOT NULL,
            requested_model_json TEXT NOT NULL,
            proposed_model_json TEXT,
            proposed_endpoint_identity_digest TEXT,
            required_features_json TEXT NOT NULL,
            streaming_intent TEXT NOT NULL,
            execution_model_invariant TEXT NOT NULL,
            route_mutation_applied INTEGER NOT NULL CHECK (route_mutation_applied = 0),
            promotion_eligible INTEGER NOT NULL CHECK (promotion_eligible = 0),
            privacy_json TEXT NOT NULL,
            canonical_digest TEXT NOT NULL UNIQUE,
            CHECK (schema_version = 1),
            FOREIGN KEY (generation_id) REFERENCES router_shadow_generations(generation_id)
                ON UPDATE RESTRICT ON DELETE RESTRICT
         );
         CREATE TABLE IF NOT EXISTS router_shadow_completions (
            schema_version INTEGER NOT NULL,
            completion_id TEXT PRIMARY KEY,
            router_run_id TEXT NOT NULL UNIQUE,
            completed_at TEXT NOT NULL,
            actual_transport TEXT NOT NULL,
            forwarded_model_json TEXT NOT NULL,
            provider_reported_model_json TEXT,
            model_preserved INTEGER NOT NULL CHECK (model_preserved IN (0, 1)),
            upstream_started INTEGER NOT NULL CHECK (upstream_started IN (0, 1)),
            response_headers_received INTEGER NOT NULL CHECK (response_headers_received IN (0, 1)),
            delivery_completed INTEGER NOT NULL CHECK (delivery_completed IN (0, 1)),
            status_code INTEGER,
            transport_outcome TEXT NOT NULL,
            latency_ms INTEGER NOT NULL,
            clock_evidence TEXT NOT NULL,
            cost_evidence_state TEXT NOT NULL,
            provider_billed_cost_microunits INTEGER,
            quality_evidence_state TEXT NOT NULL,
            rework_evidence_state TEXT NOT NULL,
            failure_class TEXT,
            decision_digest TEXT NOT NULL,
            policy_digest TEXT NOT NULL,
            route_plan_digest TEXT NOT NULL,
            canonical_digest TEXT NOT NULL UNIQUE,
            CHECK (schema_version = 1),
            FOREIGN KEY (router_run_id) REFERENCES router_shadow_decisions(router_run_id)
                ON UPDATE RESTRICT ON DELETE RESTRICT
         );
         CREATE TRIGGER IF NOT EXISTS router_shadow_generations_no_update
         BEFORE UPDATE ON router_shadow_generations BEGIN
            SELECT RAISE(ABORT, 'router_shadow_generations is append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS router_shadow_generations_no_delete
         BEFORE DELETE ON router_shadow_generations BEGIN
            SELECT RAISE(ABORT, 'router_shadow_generations is append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS router_shadow_decisions_no_update
         BEFORE UPDATE ON router_shadow_decisions BEGIN
            SELECT RAISE(ABORT, 'router_shadow_decisions is append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS router_shadow_decisions_no_delete
         BEFORE DELETE ON router_shadow_decisions BEGIN
            SELECT RAISE(ABORT, 'router_shadow_decisions is append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS router_shadow_completions_no_update
         BEFORE UPDATE ON router_shadow_completions BEGIN
            SELECT RAISE(ABORT, 'router_shadow_completions is append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS router_shadow_completions_no_delete
         BEFORE DELETE ON router_shadow_completions BEGIN
            SELECT RAISE(ABORT, 'router_shadow_completions is append-only');
         END;",
    )?;
    Ok(())
}

pub(crate) fn insert_router_shadow_decision(
    connection: &mut Connection,
    draft: RouterShadowDecisionDraft,
) -> Result<RouterShadowDecisionV1, RouterShadowStoreError> {
    insert_router_shadow_decision_internal(connection, draft, None, Uuid::new_v4(), Uuid::new_v4())
}

fn insert_router_shadow_decision_at(
    connection: &mut Connection,
    draft: RouterShadowDecisionDraft,
    now: DateTime<Utc>,
    decision_uuid: Uuid,
    run_uuid: Uuid,
) -> Result<RouterShadowDecisionV1, RouterShadowStoreError> {
    insert_router_shadow_decision_internal(connection, draft, Some(now), decision_uuid, run_uuid)
}

fn insert_router_shadow_decision_internal(
    connection: &mut Connection,
    draft: RouterShadowDecisionDraft,
    injected_now: Option<DateTime<Utc>>,
    decision_uuid: Uuid,
    run_uuid: Uuid,
) -> Result<RouterShadowDecisionV1, RouterShadowStoreError> {
    initialize_router_shadow_schema(connection)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let now = injected_now.unwrap_or_else(Utc::now);
    let orphaned_decisions: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM router_shadow_decisions AS decisions \
         LEFT JOIN router_shadow_generations AS generations \
           ON generations.generation_id = decisions.generation_id \
         WHERE generations.generation_id IS NULL",
        [],
        |row| row.get(0),
    )?;
    if orphaned_decisions != 0 {
        return Err(RouterShadowStoreError::Corrupt(
            "Router shadow decisions have orphaned generation lineage".into(),
        ));
    }
    let latest_generation: Option<i64> = transaction.query_row(
        "SELECT MAX(generation_id) FROM router_shadow_generations",
        [],
        |row| row.get(0),
    )?;
    let mut generation_id = latest_generation.unwrap_or(0);
    let mut generation_digest = if generation_id == 0 {
        generation_id = 1;
        insert_router_shadow_generation(&transaction, generation_id, now)?
    } else {
        validate_router_shadow_generation(&transaction, generation_id as u64)?
    };
    let generation_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM router_shadow_decisions WHERE generation_id = ?1",
        params![generation_id],
        |row| row.get(0),
    )?;
    if generation_count > MAX_ROUTER_SHADOW_RUNS_PER_GENERATION {
        return Err(RouterShadowStoreError::Corrupt(
            "Router shadow generation exceeds its immutable capacity".into(),
        ));
    }
    if generation_count == MAX_ROUTER_SHADOW_RUNS_PER_GENERATION {
        generation_id = generation_id.checked_add(1).ok_or_else(|| {
            RouterShadowStoreError::Corrupt("generation identifier overflow".into())
        })?;
        generation_digest = insert_router_shadow_generation(&transaction, generation_id, now)?;
    }
    let decision = RouterShadowDecisionV1::from_draft(
        draft,
        generation_id as u64,
        generation_digest,
        format!("{DECISION_ID_PREFIX}{decision_uuid}"),
        format!("{RUN_ID_PREFIX}{run_uuid}"),
        now,
    )?;
    let duplicate: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM router_shadow_decisions
         WHERE decision_id = ?1 OR router_run_id = ?2",
        params![decision.decision_id, decision.router_run_id],
        |row| row.get(0),
    )?;
    if duplicate != 0 {
        return Err(RouterShadowStoreError::DuplicateDecision);
    }
    let requested_model = serde_json::to_string(&decision.requested_model).map_err(json_error)?;
    let proposed_model = decision
        .proposed_model
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(json_error)?;
    let features = serde_json::to_string(&decision.required_features).map_err(json_error)?;
    let privacy = serde_json::to_string(&decision.privacy).map_err(json_error)?;
    transaction.execute(
        "INSERT INTO router_shadow_decisions (
            schema_version, generation_id, generation_digest, decision_id, router_run_id, decided_at, source,
            client_class, request_class, policy_digest, route_plan_digest, decision_stage,
            routing_mode, task_class, task_classification_source, requested_model_json,
            proposed_model_json, proposed_endpoint_identity_digest, required_features_json,
            streaming_intent, execution_model_invariant, route_mutation_applied,
            promotion_eligible, privacy_json, canonical_digest
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25
         )",
        params![
            decision.schema_version as i64,
            decision.generation_id as i64,
            decision.generation_digest,
            decision.decision_id,
            decision.router_run_id,
            decision.decided_at,
            decision.source.as_db(),
            decision.client_class.as_db(),
            decision.request_class.as_db(),
            decision.policy_digest,
            decision.route_plan_digest,
            decision.decision_stage.as_db(),
            decision.routing_mode.as_db(),
            decision.task_class.as_db(),
            decision.task_classification_source.as_db(),
            requested_model,
            proposed_model,
            decision.proposed_endpoint_identity_digest,
            features,
            decision.streaming_intent.as_db(),
            decision.execution_model_invariant.as_db(),
            i64::from(decision.route_mutation_applied),
            i64::from(decision.promotion_eligible),
            privacy,
            decision.canonical_digest,
        ],
    )?;
    transaction.commit()?;
    Ok(decision)
}

pub(crate) fn complete_router_shadow_run(
    connection: &mut Connection,
    draft: RouterShadowCompletionDraft,
) -> Result<RouterShadowCompletionV1, RouterShadowStoreError> {
    complete_router_shadow_run_internal(connection, draft, None, Uuid::new_v4())
}

fn complete_router_shadow_run_at(
    connection: &mut Connection,
    draft: RouterShadowCompletionDraft,
    now: DateTime<Utc>,
    completion_uuid: Uuid,
) -> Result<RouterShadowCompletionV1, RouterShadowStoreError> {
    complete_router_shadow_run_internal(connection, draft, Some(now), completion_uuid)
}

fn complete_router_shadow_run_internal(
    connection: &mut Connection,
    draft: RouterShadowCompletionDraft,
    injected_now: Option<DateTime<Utc>>,
    completion_uuid: Uuid,
) -> Result<RouterShadowCompletionV1, RouterShadowStoreError> {
    initialize_router_shadow_schema(connection)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let now = injected_now.unwrap_or_else(Utc::now);
    let duplicate: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM router_shadow_completions WHERE router_run_id = ?1",
        params![draft.router_run_id],
        |row| row.get(0),
    )?;
    if duplicate != 0 {
        return Err(RouterShadowStoreError::DuplicateCompletion);
    }
    let decision = load_router_shadow_decision_from_connection(&transaction, &draft.router_run_id)?
        .ok_or(RouterShadowStoreError::UnknownDecision)?;
    let completion = RouterShadowCompletionV1::from_draft(
        draft,
        &decision,
        format!("{COMPLETION_ID_PREFIX}{completion_uuid}"),
        now,
    )?;
    validate_completion_binding(&decision, &completion)?;
    let forwarded_model = serde_json::to_string(&completion.forwarded_model).map_err(json_error)?;
    let provider_model = completion
        .provider_reported_model
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(json_error)?;
    transaction.execute(
        "INSERT INTO router_shadow_completions (
            schema_version, completion_id, router_run_id, completed_at, actual_transport,
            forwarded_model_json, provider_reported_model_json, model_preserved,
            upstream_started, response_headers_received, delivery_completed, status_code,
            transport_outcome, latency_ms, clock_evidence, cost_evidence_state,
            provider_billed_cost_microunits, quality_evidence_state, rework_evidence_state,
            failure_class, decision_digest, policy_digest, route_plan_digest, canonical_digest
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
         )",
        params![
            completion.schema_version as i64,
            completion.completion_id,
            completion.router_run_id,
            completion.completed_at,
            completion.actual_transport.as_db(),
            forwarded_model,
            provider_model,
            i64::from(completion.model_preserved),
            i64::from(completion.upstream_started),
            i64::from(completion.response_headers_received),
            i64::from(completion.delivery_completed),
            completion.status_code.map(i64::from),
            completion.transport_outcome.as_db(),
            completion.latency_ms as i64,
            completion.clock_evidence.as_db(),
            completion.cost_evidence_state.as_db(),
            completion
                .provider_billed_cost_microunits
                .map(|value| value as i64),
            completion.quality_evidence_state.as_db(),
            completion.rework_evidence_state.as_db(),
            completion
                .failure_class
                .map(RouterShadowFailureClass::as_db),
            completion.decision_digest,
            completion.policy_digest,
            completion.route_plan_digest,
            completion.canonical_digest,
        ],
    )?;
    transaction.commit()?;
    Ok(completion)
}

#[cfg(test)]
pub(super) fn insert_router_shadow_decision_at_for_tests(
    connection: &mut Connection,
    draft: RouterShadowDecisionDraft,
    now: DateTime<Utc>,
    decision_uuid: Uuid,
    run_uuid: Uuid,
) -> Result<RouterShadowDecisionV1, RouterShadowStoreError> {
    insert_router_shadow_decision_at(connection, draft, now, decision_uuid, run_uuid)
}

#[cfg(test)]
pub(super) fn complete_router_shadow_run_at_for_tests(
    connection: &mut Connection,
    draft: RouterShadowCompletionDraft,
    now: DateTime<Utc>,
    completion_uuid: Uuid,
) -> Result<RouterShadowCompletionV1, RouterShadowStoreError> {
    complete_router_shadow_run_at(connection, draft, now, completion_uuid)
}

pub(crate) fn load_router_shadow_decision(
    connection: &Connection,
    router_run_id: &str,
) -> Result<RouterShadowDecisionV1, RouterShadowStoreError> {
    initialize_router_shadow_schema(connection)?;
    validate_prefixed_uuid(router_run_id, RUN_ID_PREFIX, "router run ID")?;
    load_router_shadow_decision_from_connection(connection, router_run_id)?
        .ok_or(RouterShadowStoreError::UnknownDecision)
}

pub(crate) fn load_router_shadow_completion(
    connection: &Connection,
    router_run_id: &str,
) -> Result<Option<RouterShadowCompletionV1>, RouterShadowStoreError> {
    initialize_router_shadow_schema(connection)?;
    validate_prefixed_uuid(router_run_id, RUN_ID_PREFIX, "router run ID")?;
    let completion = load_router_shadow_completion_from_connection(connection, router_run_id)?;
    if let Some(completion) = completion {
        let decision = load_router_shadow_decision_from_connection(connection, router_run_id)?
            .ok_or_else(|| {
                RouterShadowStoreError::Corrupt(
                    "completion is missing its immutable decision".into(),
                )
            })?;
        validate_completion_binding(&decision, &completion)?;
        Ok(Some(completion))
    } else {
        Ok(None)
    }
}

#[allow(clippy::type_complexity)]
fn load_router_shadow_decision_from_connection(
    connection: &Connection,
    router_run_id: &str,
) -> Result<Option<RouterShadowDecisionV1>, RouterShadowStoreError> {
    let row = connection
        .query_row(
            "SELECT schema_version, generation_id, generation_digest, decision_id, router_run_id, decided_at,
                    source, client_class, request_class, policy_digest, route_plan_digest,
                    decision_stage, routing_mode, task_class, task_classification_source,
                    requested_model_json, proposed_model_json,
                    proposed_endpoint_identity_digest, required_features_json,
                    streaming_intent, execution_model_invariant, route_mutation_applied,
                    promotion_eligible, privacy_json, canonical_digest
             FROM router_shadow_decisions WHERE router_run_id = ?1",
            params![router_run_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, Option<String>>(16)?,
                    row.get::<_, Option<String>>(17)?,
                    row.get::<_, String>(18)?,
                    row.get::<_, String>(19)?,
                    row.get::<_, String>(20)?,
                    row.get::<_, i64>(21)?,
                    row.get::<_, i64>(22)?,
                    row.get::<_, String>(23)?,
                    row.get::<_, String>(24)?,
                ))
            },
        )
        .optional()?;
    let Some(row) = row else { return Ok(None) };
    let decision = RouterShadowDecisionV1 {
        schema_version: to_u32(row.0, "decision schema")?,
        generation_id: to_u64(row.1, "generation ID")?,
        generation_digest: row.2,
        decision_id: row.3,
        router_run_id: row.4,
        decided_at: row.5,
        source: RouterShadowSource::from_db(&row.6)?,
        client_class: RouterShadowClientClass::from_db(&row.7)?,
        request_class: RouterShadowRequestClass::from_db(&row.8)?,
        policy_digest: row.9,
        route_plan_digest: row.10,
        decision_stage: RouterShadowDecisionStage::from_db(&row.11)?,
        routing_mode: RouterShadowMode::from_db(&row.12)?,
        task_class: RouterShadowTaskClass::from_db(&row.13)?,
        task_classification_source: RouterShadowTaskClassificationSource::from_db(&row.14)?,
        requested_model: parse_json(&row.15, "requested model identity")?,
        proposed_model: row
            .16
            .as_deref()
            .map(|value| parse_json(value, "proposed model identity"))
            .transpose()?,
        proposed_endpoint_identity_digest: row.17,
        required_features: parse_json(&row.18, "required features")?,
        streaming_intent: RouterShadowStreamingIntent::from_db(&row.19)?,
        execution_model_invariant: RouterShadowExecutionModelInvariant::from_db(&row.20)?,
        route_mutation_applied: to_bool(row.21, "route mutation")?,
        promotion_eligible: to_bool(row.22, "promotion eligibility")?,
        privacy: parse_json(&row.23, "privacy attestations")?,
        canonical_digest: row.24,
    };
    decision.validate()?;
    let generation_digest = validate_router_shadow_generation(connection, decision.generation_id)?;
    if decision.generation_digest != generation_digest {
        return Err(RouterShadowStoreError::Corrupt(
            "decision generation binding mismatch".into(),
        ));
    }
    Ok(Some(decision))
}

#[allow(clippy::type_complexity)]
fn load_router_shadow_completion_from_connection(
    connection: &Connection,
    router_run_id: &str,
) -> Result<Option<RouterShadowCompletionV1>, RouterShadowStoreError> {
    let row = connection
        .query_row(
            "SELECT schema_version, completion_id, router_run_id, completed_at,
                    actual_transport, forwarded_model_json, provider_reported_model_json,
                    model_preserved, upstream_started, response_headers_received,
                    delivery_completed, status_code, transport_outcome, latency_ms,
                    clock_evidence, cost_evidence_state, provider_billed_cost_microunits,
                    quality_evidence_state, rework_evidence_state, failure_class,
                    decision_digest, policy_digest, route_plan_digest, canonical_digest
             FROM router_shadow_completions WHERE router_run_id = ?1",
            params![router_run_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, Option<i64>>(16)?,
                    row.get::<_, String>(17)?,
                    row.get::<_, String>(18)?,
                    row.get::<_, Option<String>>(19)?,
                    row.get::<_, String>(20)?,
                    row.get::<_, String>(21)?,
                    row.get::<_, String>(22)?,
                    row.get::<_, String>(23)?,
                ))
            },
        )
        .optional()?;
    let Some(row) = row else { return Ok(None) };
    let completion = RouterShadowCompletionV1 {
        schema_version: to_u32(row.0, "completion schema")?,
        completion_id: row.1,
        router_run_id: row.2,
        completed_at: row.3,
        actual_transport: RouterShadowTransport::from_db(&row.4)?,
        forwarded_model: parse_json(&row.5, "forwarded model identity")?,
        provider_reported_model: row
            .6
            .as_deref()
            .map(|value| parse_json(value, "provider model identity"))
            .transpose()?,
        model_preserved: to_bool(row.7, "model preserved")?,
        upstream_started: to_bool(row.8, "upstream started")?,
        response_headers_received: to_bool(row.9, "response headers")?,
        delivery_completed: to_bool(row.10, "delivery completed")?,
        status_code: row
            .11
            .map(|value| to_u16(value, "status code"))
            .transpose()?,
        transport_outcome: RouterShadowOutcome::from_db(&row.12)?,
        latency_ms: to_u64(row.13, "latency")?,
        clock_evidence: RouterShadowClockEvidence::from_db(&row.14)?,
        cost_evidence_state: RouterShadowCostEvidenceState::from_db(&row.15)?,
        provider_billed_cost_microunits: row
            .16
            .map(|value| to_u64(value, "provider cost"))
            .transpose()?,
        quality_evidence_state: RouterShadowQualityEvidenceState::from_db(&row.17)?,
        rework_evidence_state: RouterShadowReworkEvidenceState::from_db(&row.18)?,
        failure_class: row
            .19
            .as_deref()
            .map(RouterShadowFailureClass::from_db)
            .transpose()?,
        decision_digest: row.20,
        policy_digest: row.21,
        route_plan_digest: row.22,
        canonical_digest: row.23,
    };
    completion.validate()?;
    Ok(Some(completion))
}

fn validate_completion_binding(
    decision: &RouterShadowDecisionV1,
    completion: &RouterShadowCompletionV1,
) -> Result<(), RouterShadowStoreError> {
    if completion.router_run_id != decision.router_run_id
        || completion.decision_digest != decision.canonical_digest
        || completion.policy_digest != decision.policy_digest
        || completion.route_plan_digest != decision.route_plan_digest
    {
        return Err(RouterShadowStoreError::Validation(
            "completion does not match its immutable decision".into(),
        ));
    }
    let identities_match = completion.forwarded_model == decision.requested_model;
    if completion.model_preserved != identities_match {
        return Err(RouterShadowStoreError::Validation(
            "model-preserved flag does not match identity evidence".into(),
        ));
    }
    let expected_transport = match decision.request_class {
        RouterShadowRequestClass::OpenAiResponses
        | RouterShadowRequestClass::OpenAiChatCompletions => RouterShadowTransport::DirectOpenAi,
        RouterShadowRequestClass::AnthropicMessages => RouterShadowTransport::DirectAnthropic,
    };
    if completion.actual_transport != expected_transport {
        return Err(RouterShadowStoreError::Validation(
            "completion transport does not match the immutable request class".into(),
        ));
    }
    let mismatch_is_explicit = matches!(
        (completion.transport_outcome, completion.failure_class),
        (
            RouterShadowOutcome::IntegrityFailure,
            Some(RouterShadowFailureClass::Integrity)
        ) | (
            RouterShadowOutcome::ClockAnomaly,
            Some(RouterShadowFailureClass::ClockRollback)
        )
    );
    if !identities_match && !mismatch_is_explicit {
        return Err(RouterShadowStoreError::Validation(
            "model mismatch lacks an integrity-failure terminal".into(),
        ));
    }
    Ok(())
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, RouterShadowStoreError> {
    let canonical = serde_json::to_vec(value).map_err(json_error)?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

fn json_error(error: serde_json::Error) -> RouterShadowStoreError {
    RouterShadowStoreError::Validation(format!("canonical JSON: {error}"))
}

fn parse_json<T: for<'de> Deserialize<'de>>(
    value: &str,
    label: &str,
) -> Result<T, RouterShadowStoreError> {
    serde_json::from_str(value)
        .map_err(|error| RouterShadowStoreError::Corrupt(format!("{label}: {error}")))
}

fn canonical_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn require_canonical_timestamp(value: &str, label: &str) -> Result<(), RouterShadowStoreError> {
    let parsed = parse_timestamp(value, label)?;
    if canonical_timestamp(parsed) != value {
        return Err(RouterShadowStoreError::Validation(format!(
            "{label} is not canonical RFC3339 UTC"
        )));
    }
    Ok(())
}

fn parse_timestamp(value: &str, label: &str) -> Result<DateTime<Utc>, RouterShadowStoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|_| RouterShadowStoreError::Validation(format!("{label} is not RFC3339")))
}

fn validate_prefixed_uuid(
    value: &str,
    prefix: &str,
    label: &str,
) -> Result<(), RouterShadowStoreError> {
    let suffix = value.strip_prefix(prefix).ok_or_else(|| {
        RouterShadowStoreError::Validation(format!("{label} has the wrong native prefix"))
    })?;
    let parsed = Uuid::parse_str(suffix)
        .map_err(|_| RouterShadowStoreError::Validation(format!("{label} is not a UUID")))?;
    if parsed.to_string() != suffix {
        return Err(RouterShadowStoreError::Validation(format!(
            "{label} is not canonical"
        )));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), RouterShadowStoreError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(RouterShadowStoreError::Validation(format!(
            "{label} must use sha256"
        )));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RouterShadowStoreError::Validation(format!(
            "{label} is malformed"
        )));
    }
    Ok(())
}

fn to_bool(value: i64, label: &str) -> Result<bool, RouterShadowStoreError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(RouterShadowStoreError::Corrupt(format!(
            "{label} is not boolean"
        ))),
    }
}

fn to_u64(value: i64, label: &str) -> Result<u64, RouterShadowStoreError> {
    u64::try_from(value)
        .map_err(|_| RouterShadowStoreError::Corrupt(format!("{label} is negative")))
}

fn to_u32(value: i64, label: &str) -> Result<u32, RouterShadowStoreError> {
    u32::try_from(value)
        .map_err(|_| RouterShadowStoreError::Corrupt(format!("{label} is outside range")))
}

fn to_u16(value: i64, label: &str) -> Result<u16, RouterShadowStoreError> {
    u16::try_from(value)
        .map_err(|_| RouterShadowStoreError::Corrupt(format!("{label} is outside range")))
}
