//! Current-process Token X-Ray read model.

use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};

use crate::analytics_models::{
    AnalyticsEvidenceConfidence, AnalyticsFreshness, ContextPressureBandV1, ContextPressureV1,
    TokenMetricV1, TokenXrayEventKindV1, TokenXrayEventV1, TokenXrayLiveUpdateV1,
    TokenXrayMetricsV1, TokenXraySnapshotV1, UsageAnomalyV1,
};
use crate::analytics_normalization::{confidence, normalize};
use crate::models::{DashboardState, SavingsAttributionEvent, UsageOutcome};
use crate::optimization::CacheTokenMetrics;

const TIMELINE_LIMIT: usize = 50;
const LIVE_UPDATE_TIMELINE_LIMIT: usize = 12;

/// Coalesces content-free live projections by their material values. A caller
/// may advance its local revision only when this returns an update; repeated
/// polls and duplicate tracker writes therefore cannot create UI churn.
#[derive(Default)]
pub(crate) struct TokenXrayUpdateCoalescer {
    last_fingerprint: Option<String>,
}

impl TokenXrayUpdateCoalescer {
    pub(crate) fn project(
        &mut self,
        snapshot: TokenXraySnapshotV1,
        revision: u64,
    ) -> Option<TokenXrayLiveUpdateV1> {
        let update = live_update_projection(snapshot, revision);
        let fingerprint = update_fingerprint(&update);
        if self.last_fingerprint.as_deref() == Some(fingerprint.as_str()) {
            return None;
        }
        self.last_fingerprint = Some(fingerprint);
        Some(update)
    }
}

/// Projects a bounded update payload for the local UI. Metrics are cloned as
/// is so unavailable evidence remains unavailable rather than becoming zero.
pub(crate) fn live_update_projection(
    snapshot: TokenXraySnapshotV1,
    revision: u64,
) -> TokenXrayLiveUpdateV1 {
    TokenXrayLiveUpdateV1 {
        schema_version: snapshot.schema_version,
        revision,
        generated_at: snapshot.generated_at,
        agent: snapshot.agent,
        provider: snapshot.provider,
        model: snapshot.model,
        freshness: snapshot.freshness,
        metrics: snapshot.metrics,
        context_pressure: snapshot.context_pressure,
        timeline: snapshot
            .timeline
            .into_iter()
            .take(LIVE_UPDATE_TIMELINE_LIMIT)
            .collect(),
    }
}

fn update_fingerprint(update: &TokenXrayLiveUpdateV1) -> String {
    // The generation timestamp and monotonic revision are transport metadata,
    // not a material usage change. Excluding them is what suppresses duplicate
    // emissions from repeated reads of unchanged local evidence.
    let mut comparable = update.clone();
    comparable.revision = 0;
    comparable.generated_at =
        chrono::DateTime::from_timestamp(0, 0).expect("Unix epoch is a valid timestamp");
    let bytes = serde_json::to_vec(&comparable).expect("live update serializes");
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

/// Builds a content-free snapshot from explicitly supplied cache evidence.
/// Keeping the optional cache result at the boundary prevents a failed local
/// telemetry read from being represented as a measured zero.
pub(crate) fn build_snapshot_with_cache_metrics(
    dashboard: &DashboardState,
    attribution: Vec<SavingsAttributionEvent>,
    cache_metrics: Option<CacheTokenMetrics>,
) -> TokenXraySnapshotV1 {
    let normalized = normalize(dashboard, attribution, |_| true, |_| true);
    let latest = normalized.usage.iter().max_by_key(|event| event.timestamp);
    let newest_at = latest.map(|event| event.timestamp).or_else(|| {
        normalized
            .attribution
            .iter()
            .map(|event| event.observed_at)
            .max()
    });
    let freshness = freshness(newest_at);
    let observed_at = newest_at;
    let input = metric(
        normalized.input_tokens,
        "recent_usage",
        observed_at,
        "Input tokens are proxy estimates from the current in-memory session.",
    );
    let output = metric(
        normalized.output_tokens,
        "recent_usage",
        observed_at,
        "Output tokens are proxy estimates from the current in-memory session.",
    );
    let saved = metric(
        normalized.saved_tokens,
        "savings_attribution_ledger",
        observed_at,
        "Repo Intelligence avoidance is reported separately from request compression.",
    );
    let avoided = metric(
        normalized.avoided_tokens,
        "savings_attribution_ledger",
        observed_at,
        "Avoided tokens are counterfactual context avoided before a full repository scan.",
    );
    let savings_usd = TokenMetricV1 {
        value: Some(normalized.estimated_savings_usd),
        confidence: normalized
            .source_impacts
            .iter()
            .map(|impact| impact.confidence)
            .min_by_key(confidence_rank)
            .unwrap_or(AnalyticsEvidenceConfidence::Unavailable),
        source: "savings_attribution_ledger".into(),
        observed_at,
        caveat: Some("Only evidence-backed savings attribution is included.".into()),
    };
    let timeline = timeline(&normalized.usage, &normalized.attribution);
    let anomalies = dashboard
        .insights
        .iter()
        .filter(|insight| !matches!(insight.severity, crate::models::InsightSeverity::Info))
        .map(|insight| UsageAnomalyV1 {
            id: insight.id.clone(),
            severity: format!("{:?}", insight.severity).to_lowercase(),
            message: insight.title.clone(),
            evidence: vec![insight.evidence.clone()],
        })
        .collect();

    let latest_model = latest.and_then(|event| known_model(&event.upstream_target));
    let context_pressure = context_pressure(
        normalized
            .input_tokens
            .saturating_add(normalized.output_tokens),
        latest_model,
    );

    TokenXraySnapshotV1 {
        schema_version: 1,
        generated_at: normalized.generated_at,
        session_id: "current-local-session".into(),
        // Client and upstream strings are runtime-provided. Emit only a
        // small allowlist of product/provider labels so query strings, local
        // paths, or arbitrary client labels never escape into analytics.
        agent: latest.map(|event| safe_agent_label(&event.client)),
        provider: latest.map(|event| safe_provider_label(&event.upstream_target)),
        model: latest_model.map(|model| model.id.to_string()),
        freshness,
        metrics: TokenXrayMetricsV1 {
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache_metric(cache_metrics, CacheMetricKind::Read, observed_at),
            cache_write_tokens: cache_metric(cache_metrics, CacheMetricKind::Write, observed_at),
            saved_tokens: saved,
            avoided_tokens: avoided,
            estimated_cost_usd: TokenMetricV1::unavailable(
                "provider_pricing",
                "Exact request-level pricing is not available in the current local session.",
            ),
            estimated_savings_usd: savings_usd,
        },
        context_pressure,
        sources: normalized.source_impacts,
        timeline,
        anomalies,
    }
}

#[derive(Clone, Copy)]
enum CacheMetricKind {
    Read,
    Write,
}

fn cache_metric(
    cache_metrics: Option<CacheTokenMetrics>,
    kind: CacheMetricKind,
    observed_at: Option<chrono::DateTime<Utc>>,
) -> TokenMetricV1 {
    let Some(metrics) = cache_metrics else {
        return TokenMetricV1::unavailable(
            "provider_cache_metrics",
            "Provider cache telemetry could not be read locally; no zero was assumed.",
        );
    };
    let (value, label) = match kind {
        CacheMetricKind::Read => (metrics.cache_read_tokens, "cache-read"),
        CacheMetricKind::Write => (metrics.cache_creation_tokens, "cache-write"),
    };
    TokenMetricV1 {
        value: Some(value as f64),
        confidence: AnalyticsEvidenceConfidence::Measured,
        source: "provider_cache_metrics_aggregate".into(),
        observed_at,
        caveat: Some(format!(
            "Measured aggregate {label} tokens from locally parsed provider usage; this is not a per-request value."
        )),
    }
}

#[derive(Clone, Copy)]
struct KnownModel {
    id: &'static str,
    context_limit: u64,
}

// This intentionally conservative registry contains only model identifiers
// with stable published context windows. Unknown (including aliases such as
// "latest") remain unavailable rather than inheriting a provider-wide guess.
const KNOWN_MODELS: &[KnownModel] = &[
    KnownModel {
        id: "gemini-2.5-flash",
        context_limit: 1_048_576,
    },
    KnownModel {
        id: "gemini-2.5-pro",
        context_limit: 1_048_576,
    },
    KnownModel {
        id: "gemini-2.0-flash",
        context_limit: 1_048_576,
    },
    KnownModel {
        id: "claude-3-5-sonnet",
        context_limit: 200_000,
    },
    KnownModel {
        id: "claude-3-7-sonnet",
        context_limit: 200_000,
    },
    KnownModel {
        id: "claude-sonnet-4",
        context_limit: 200_000,
    },
    KnownModel {
        id: "claude-opus-4",
        context_limit: 200_000,
    },
    KnownModel {
        id: "gpt-4.1",
        context_limit: 1_047_576,
    },
    KnownModel {
        id: "gpt-4o-mini",
        context_limit: 128_000,
    },
    KnownModel {
        id: "gpt-4o",
        context_limit: 128_000,
    },
    KnownModel {
        id: "o4-mini",
        context_limit: 200_000,
    },
    KnownModel {
        id: "o3",
        context_limit: 200_000,
    },
];

fn known_model(value: &str) -> Option<KnownModel> {
    let normalized = value.to_ascii_lowercase();
    KNOWN_MODELS
        .iter()
        .copied()
        .find(|model| normalized.contains(model.id))
}

fn context_pressure(used_tokens: u64, model: Option<KnownModel>) -> ContextPressureV1 {
    let Some(model) = model else {
        return ContextPressureV1 {
            used_tokens: Some(used_tokens),
            limit_tokens: None,
            percent: None,
            band: ContextPressureBandV1::Unavailable,
            limit_source: "unavailable".into(),
            caveat: Some(
                "No known model-specific context-window limit is available for this session."
                    .into(),
            ),
        };
    };
    let percent = (used_tokens as f64 / model.context_limit as f64) * 100.0;
    let band = if percent >= 85.0 {
        ContextPressureBandV1::Critical
    } else if percent >= 70.0 {
        ContextPressureBandV1::High
    } else if percent >= 50.0 {
        ContextPressureBandV1::Elevated
    } else {
        ContextPressureBandV1::Normal
    };
    ContextPressureV1 {
        used_tokens: Some(used_tokens),
        limit_tokens: Some(model.context_limit),
        percent: Some(percent),
        band,
        limit_source: format!("static_model_registry:{}", model.id),
        caveat: Some(
            "Projected from current-session token estimates and a known model-specific context-window limit."
                .into(),
        ),
    }
}

fn safe_agent_label(value: &str) -> String {
    let value = value.to_ascii_lowercase();
    if value.contains("claude") {
        "Claude Code"
    } else if value.contains("codex") {
        "Codex"
    } else if value.contains("cursor") {
        "Cursor"
    } else if value.contains("windsurf") {
        "Windsurf"
    } else {
        "Local agent"
    }
    .to_string()
}

fn safe_provider_label(value: &str) -> String {
    let value = value.to_ascii_lowercase();
    if value.contains("anthropic") || value.contains("claude") {
        "Anthropic"
    } else if value.contains("openai") || value.contains("gpt-") || value.contains("/o3") {
        "OpenAI"
    } else if value.contains("google") || value.contains("gemini") {
        "Google"
    } else if value.contains("ollama") {
        "Ollama"
    } else {
        "Unknown provider"
    }
    .to_string()
}

fn metric(
    value: u64,
    source: &str,
    observed_at: Option<chrono::DateTime<Utc>>,
    caveat: &str,
) -> TokenMetricV1 {
    TokenMetricV1 {
        value: Some(value as f64),
        confidence: AnalyticsEvidenceConfidence::Estimated,
        source: source.into(),
        observed_at,
        caveat: Some(caveat.into()),
    }
}

fn freshness(observed_at: Option<chrono::DateTime<Utc>>) -> AnalyticsFreshness {
    let Some(observed_at) = observed_at else {
        return AnalyticsFreshness::Unavailable;
    };
    let age = Utc::now() - observed_at;
    if age <= Duration::minutes(2) {
        AnalyticsFreshness::Live
    } else if age <= Duration::minutes(30) {
        AnalyticsFreshness::Recent
    } else {
        AnalyticsFreshness::Stale
    }
}

fn confidence_rank(value: &AnalyticsEvidenceConfidence) -> u8 {
    match value {
        AnalyticsEvidenceConfidence::Measured => 0,
        AnalyticsEvidenceConfidence::Estimated => 1,
        AnalyticsEvidenceConfidence::Inferred => 2,
        AnalyticsEvidenceConfidence::Unavailable => 3,
    }
}

fn timeline(
    usage: &[crate::models::UsageEvent],
    attribution: &[SavingsAttributionEvent],
) -> Vec<TokenXrayEventV1> {
    let mut events: Vec<TokenXrayEventV1> = usage
        .iter()
        .map(|event| TokenXrayEventV1 {
            id: opaque_event_id(&event.id),
            occurred_at: event.timestamp,
            kind: if matches!(event.outcome, UsageOutcome::Error) {
                TokenXrayEventKindV1::Failure
            } else {
                TokenXrayEventKindV1::Usage
            },
            label: format!("{} request", safe_agent_label(&event.client)),
            confidence: AnalyticsEvidenceConfidence::Estimated,
            detail: Some(format!(
                "{} input + {} output estimated tokens",
                event.estimated_input_tokens, event.estimated_output_tokens
            )),
        })
        .chain(attribution.iter().map(|event| TokenXrayEventV1 {
            id: opaque_event_id(&event.id),
            occurred_at: event.observed_at,
            kind: TokenXrayEventKindV1::Savings,
            label: format!("{:?} savings evidence", event.source),
            confidence: confidence(event.confidence.clone()),
            detail: Some(format!("{} tokens", event.delta_tokens_saved)),
        }))
        .collect();
    events.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));
    events.truncate(TIMELINE_LIMIT);
    events
}

fn opaque_event_id(value: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(value.as_bytes());
    format!("event-{:x}", digest.finalize())[..22].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LaunchExperience, UsageEvent};

    fn dashboard_with_usage(usage: Vec<UsageEvent>) -> DashboardState {
        DashboardState {
            app_version: String::new(),
            launch_experience: LaunchExperience::Dashboard,
            bootstrap_complete: false,
            python_runtime_installed: false,
            lifetime_requests: 0,
            lifetime_estimated_savings_usd: 0.0,
            lifetime_estimated_tokens_saved: 0,
            session_requests: 0,
            session_estimated_savings_usd: 0.0,
            session_estimated_tokens_saved: 0,
            session_savings_pct: 0.0,
            output_reduction: None,
            daily_savings: vec![],
            hourly_savings: vec![],
            savings_history_loaded: false,
            tools: vec![],
            clients: vec![],
            recent_usage: usage,
            insights: vec![],
            required_terms_version: 0,
            accepted_terms_version: 0,
            terms_url: String::new(),
        }
    }

    fn usage(upstream_target: &str, input: u64, output: u64) -> UsageEvent {
        UsageEvent {
            id: "fixture-event-id".into(),
            timestamp: Utc::now(),
            client: "codex?token=should-not-leak".into(),
            workspace: "/private/workspace/should-not-leak".into(),
            upstream_target: upstream_target.into(),
            stages: vec![],
            estimated_input_tokens: input,
            estimated_output_tokens: output,
            estimated_cost_savings_usd: 0.0,
            latency_ms: 0,
            outcome: UsageOutcome::Success,
        }
    }

    #[test]
    fn freshness_is_unavailable_without_events() {
        assert!(matches!(freshness(None), AnalyticsFreshness::Unavailable));
    }

    #[test]
    fn unknown_model_pressure_has_no_false_precision() {
        let pressure = context_pressure(1_000, None);
        assert!(pressure.percent.is_none());
        assert!(matches!(pressure.band, ContextPressureBandV1::Unavailable));
    }

    #[test]
    fn known_model_gets_projected_pressure_and_measured_cache_aggregates() {
        let snapshot = build_snapshot_with_cache_metrics(
            &dashboard_with_usage(vec![usage("https://api.openai.com/gpt-4o", 96_000, 8_000)]),
            vec![],
            Some(CacheTokenMetrics {
                prompt_tokens: 100_000,
                completion_tokens: 8_000,
                cache_creation_tokens: 2_000,
                cache_read_tokens: 25_000,
            }),
        );
        assert_eq!(snapshot.model.as_deref(), Some("gpt-4o"));
        assert_eq!(snapshot.context_pressure.limit_tokens, Some(128_000));
        assert!(matches!(
            snapshot.context_pressure.band,
            ContextPressureBandV1::High
        ));
        assert_eq!(snapshot.metrics.cache_read_tokens.value, Some(25_000.0));
        assert!(matches!(
            snapshot.metrics.cache_read_tokens.confidence,
            AnalyticsEvidenceConfidence::Measured
        ));
    }

    #[test]
    fn unknown_model_and_failed_cache_read_remain_unavailable() {
        let snapshot = build_snapshot_with_cache_metrics(
            &dashboard_with_usage(vec![usage("https://example.test/model=latest", 10, 2)]),
            vec![],
            None,
        );
        assert!(snapshot.model.is_none());
        assert!(snapshot.context_pressure.percent.is_none());
        assert!(matches!(
            snapshot.context_pressure.band,
            ContextPressureBandV1::Unavailable
        ));
        assert!(snapshot.metrics.cache_read_tokens.value.is_none());
    }

    #[test]
    fn serialized_snapshot_redacts_runtime_fixture_values() {
        let snapshot = build_snapshot_with_cache_metrics(
            &dashboard_with_usage(vec![usage(
                "https://api.openai.com/gpt-4o?api_key=super-secret-value",
                10,
                2,
            )]),
            vec![],
            None,
        );
        let serialized = serde_json::to_string(&snapshot).expect("serializes snapshot");
        for secret in [
            "super-secret-value",
            "should-not-leak",
            "/private/workspace",
            "fixture-event-id",
        ] {
            assert!(
                !serialized.contains(secret),
                "content-free snapshot leaked fixture value: {secret}"
            );
        }
        assert_eq!(snapshot.provider.as_deref(), Some("OpenAI"));
        assert_eq!(snapshot.agent.as_deref(), Some("Codex"));
    }

    #[test]
    fn live_projection_is_bounded_and_preserves_unavailable_metrics() {
        let usage = (0..(LIVE_UPDATE_TIMELINE_LIMIT as u64 + 4))
            .map(|index| usage("https://example.test/model=latest", index + 1, 2))
            .collect();
        let snapshot =
            build_snapshot_with_cache_metrics(&dashboard_with_usage(usage), vec![], None);
        let update = live_update_projection(snapshot, 7);

        assert_eq!(update.revision, 7);
        assert_eq!(update.timeline.len(), LIVE_UPDATE_TIMELINE_LIMIT);
        assert!(update.metrics.cache_read_tokens.value.is_none());
        assert!(matches!(
            update.metrics.cache_read_tokens.confidence,
            AnalyticsEvidenceConfidence::Unavailable
        ));
    }

    #[test]
    fn live_update_coalescer_suppresses_duplicate_projections() {
        let snapshot = build_snapshot_with_cache_metrics(
            &dashboard_with_usage(vec![usage("https://api.openai.com/gpt-4o", 10, 2)]),
            vec![],
            None,
        );
        let mut coalescer = TokenXrayUpdateCoalescer::default();

        assert!(coalescer.project(snapshot.clone(), 1).is_some());
        assert!(coalescer.project(snapshot, 2).is_none());
    }

    #[test]
    fn live_update_coalescer_emits_after_material_usage_change() {
        let first = build_snapshot_with_cache_metrics(
            &dashboard_with_usage(vec![usage("https://api.openai.com/gpt-4o", 10, 2)]),
            vec![],
            None,
        );
        let changed = build_snapshot_with_cache_metrics(
            &dashboard_with_usage(vec![usage("https://api.openai.com/gpt-4o", 11, 2)]),
            vec![],
            None,
        );
        let mut coalescer = TokenXrayUpdateCoalescer::default();

        assert!(coalescer.project(first, 1).is_some());
        let update = coalescer
            .project(changed, 2)
            .expect("material usage changes emit an update");
        assert_eq!(update.revision, 2);
        assert_eq!(update.metrics.input_tokens.value, Some(11.0));
    }
}
