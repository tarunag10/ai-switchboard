//! Deterministic normalization of the existing local usage and savings ledger.

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Local, Utc};
use sha2::{Digest, Sha256};

use crate::analytics_models::{
    AnalyticsEvidenceConfidence, NormalizedAnalyticsEventV1, OptimizationImpactV1,
    TokenXrayEventKindV1,
};
use crate::models::{
    DashboardState, SavingsAttributionConfidence, SavingsAttributionEvent,
    SavingsAttributionSource, UsageEvent, UsageOutcome,
};

#[derive(Debug, Clone)]
pub(crate) struct NormalizedAnalytics {
    pub generated_at: DateTime<Utc>,
    pub usage: Vec<UsageEvent>,
    pub attribution: Vec<SavingsAttributionEvent>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub saved_tokens: u64,
    pub avoided_tokens: u64,
    pub estimated_savings_usd: f64,
    pub source_impacts: Vec<OptimizationImpactV1>,
    pub events: Vec<NormalizedAnalyticsEventV1>,
}

pub(crate) fn local_day_key(now: DateTime<Utc>) -> String {
    now.with_timezone(&Local).format("%Y-%m-%d").to_string()
}

pub(crate) fn is_today(timestamp: DateTime<Utc>, day_key: &str) -> bool {
    local_day_key(timestamp) == day_key
}

pub(crate) fn normalize(
    dashboard: &DashboardState,
    attribution_events: Vec<SavingsAttributionEvent>,
    usage_filter: impl Fn(&UsageEvent) -> bool,
    attribution_filter: impl Fn(&SavingsAttributionEvent) -> bool,
) -> NormalizedAnalytics {
    let usage: Vec<UsageEvent> = dashboard
        .recent_usage
        .iter()
        .filter(|event| usage_filter(event))
        .cloned()
        .collect();
    let attribution: Vec<SavingsAttributionEvent> = attribution_events
        .into_iter()
        .filter(|event| attribution_filter(event))
        .fold(
            (HashSet::new(), Vec::new()),
            |(mut seen, mut kept), event| {
                if seen.insert(event.id.clone()) {
                    kept.push(event);
                }
                (seen, kept)
            },
        )
        .1;
    let input_tokens = usage.iter().map(|event| event.estimated_input_tokens).sum();
    let output_tokens = usage
        .iter()
        .map(|event| event.estimated_output_tokens)
        .sum();
    // UsageEvent's cost field is already an explicitly named savings estimate.
    // Cost is sourced only from daily Headroom rollups elsewhere, never invented here.
    let estimated_savings_usd = attribution
        .iter()
        .map(|event| event.delta_usd.max(0.0))
        .sum();
    let mut saved_tokens = 0_u64;
    let mut avoided_tokens = 0_u64;
    for event in &attribution {
        if matches!(event.source, SavingsAttributionSource::RepoIntelligence) {
            avoided_tokens = avoided_tokens.saturating_add(event.delta_tokens_saved);
        } else {
            saved_tokens = saved_tokens.saturating_add(event.delta_tokens_saved);
        }
    }

    let mut grouped: BTreeMap<String, Vec<&SavingsAttributionEvent>> = BTreeMap::new();
    for event in &attribution {
        grouped
            .entry(format!("{:?}", event.source))
            .or_default()
            .push(event);
    }
    let source_impacts = grouped
        .into_iter()
        .map(|(source, events)| {
            let confidence = events
                .iter()
                .map(|event| confidence(event.confidence.clone()))
                .min_by_key(confidence_rank)
                .unwrap_or(AnalyticsEvidenceConfidence::Unavailable);
            OptimizationImpactV1 {
                source,
                confidence,
                tokens_saved: Some(events.iter().map(|event| event.delta_tokens_saved).sum()),
                estimated_savings_usd: Some(events.iter().map(|event| event.delta_usd).sum()),
                event_count: events.len() as u64,
                runtime_evidence_units: events.iter().map(|event| event.request_delta as u64).sum(),
                measured_event_count: events
                    .iter()
                    .filter(|event| {
                        matches!(event.confidence, SavingsAttributionConfidence::Measured)
                    })
                    .count() as u64,
                estimated_event_count: events
                    .iter()
                    .filter(|event| {
                        matches!(event.confidence, SavingsAttributionConfidence::Estimated)
                    })
                    .count() as u64,
                inferred_event_count: events
                    .iter()
                    .filter(|event| {
                        matches!(event.confidence, SavingsAttributionConfidence::Inferred)
                    })
                    .count() as u64,
                total_tokens_sent: events.iter().map(|event| event.total_tokens_sent).sum(),
                evidence: events
                    .iter()
                    .flat_map(|event| event.evidence.iter().cloned())
                    .take(5)
                    .collect(),
                last_observed_at: events.iter().map(|event| event.observed_at).max(),
            }
        })
        .collect();

    let mut events = usage.iter().map(normalize_usage_event).collect::<Vec<_>>();
    events.extend(attribution.iter().map(normalize_attribution_event));
    events.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    NormalizedAnalytics {
        generated_at: Utc::now(),
        usage,
        attribution,
        input_tokens,
        output_tokens,
        saved_tokens,
        avoided_tokens,
        estimated_savings_usd,
        source_impacts,
        events,
    }
}

fn normalize_usage_event(event: &UsageEvent) -> NormalizedAnalyticsEventV1 {
    let saved_tokens = event
        .stages
        .iter()
        .map(|stage| stage.estimated_tokens_saved)
        .sum();
    NormalizedAnalyticsEventV1 {
        schema_version: 1,
        id: stable_event_id("usage", &event.id),
        occurred_at: event.timestamp,
        kind: TokenXrayEventKindV1::Usage,
        label: "Agent request".into(),
        confidence: AnalyticsEvidenceConfidence::Estimated,
        input_tokens: event.estimated_input_tokens,
        output_tokens: event.estimated_output_tokens,
        saved_tokens,
        avoided_tokens: 0,
        request_count: 1,
        latency_ms: Some(event.latency_ms),
        outcome: Some(usage_outcome_label(&event.outcome).into()),
        source: "recent_usage".into(),
    }
}

fn normalize_attribution_event(event: &SavingsAttributionEvent) -> NormalizedAnalyticsEventV1 {
    let (saved_tokens, avoided_tokens) =
        if matches!(event.source, SavingsAttributionSource::RepoIntelligence) {
            (0, event.delta_tokens_saved)
        } else {
            (event.delta_tokens_saved, 0)
        };
    NormalizedAnalyticsEventV1 {
        schema_version: 1,
        id: stable_event_id("attribution", &event.id),
        occurred_at: event.observed_at,
        kind: TokenXrayEventKindV1::Savings,
        label: "Optimization attribution".into(),
        confidence: confidence(event.confidence.clone()),
        input_tokens: event.total_tokens_sent,
        output_tokens: 0,
        saved_tokens,
        avoided_tokens,
        request_count: event.request_delta as u64,
        latency_ms: None,
        outcome: None,
        source: attribution_source_label(&event.source).into(),
    }
}

fn stable_event_id(prefix: &str, value: &str) -> String {
    format!("{prefix}-{:x}", Sha256::digest(value.as_bytes()))
}

fn usage_outcome_label(outcome: &UsageOutcome) -> &'static str {
    match outcome {
        UsageOutcome::Success => "success",
        UsageOutcome::Bypassed => "bypassed",
        UsageOutcome::Error => "error",
    }
}

fn attribution_source_label(source: &SavingsAttributionSource) -> &'static str {
    match source {
        SavingsAttributionSource::HeadroomEngine => "headroom_engine",
        SavingsAttributionSource::Rtk => "rtk",
        SavingsAttributionSource::RepoIntelligence => "repo_intelligence",
        SavingsAttributionSource::Caveman => "caveman",
        SavingsAttributionSource::Ponytail => "ponytail",
        SavingsAttributionSource::Markitdown => "markitdown",
        SavingsAttributionSource::CompactChinese => "compact_chinese",
        SavingsAttributionSource::AgentMemory => "agent_memory",
    }
}

pub(crate) fn confidence(value: SavingsAttributionConfidence) -> AnalyticsEvidenceConfidence {
    match value {
        SavingsAttributionConfidence::Measured => AnalyticsEvidenceConfidence::Measured,
        SavingsAttributionConfidence::Estimated => AnalyticsEvidenceConfidence::Estimated,
        SavingsAttributionConfidence::Inferred => AnalyticsEvidenceConfidence::Inferred,
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

pub(crate) fn is_failure(outcome: &UsageOutcome) -> bool {
    matches!(outcome, UsageOutcome::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LaunchExperience, SavingsAttributionScope};
    use chrono::Duration;

    fn event(id: &str, source: SavingsAttributionSource) -> SavingsAttributionEvent {
        SavingsAttributionEvent {
            schema_version: 1,
            id: id.into(),
            observed_at: Utc::now(),
            scope: SavingsAttributionScope::Session,
            source,
            confidence: SavingsAttributionConfidence::Measured,
            delta_tokens_saved: 10,
            delta_usd: 0.1,
            total_tokens_sent: 100,
            request_delta: 1,
            evidence: vec!["fixture".into()],
            measurement_id: None,
            measurement_provenance: None,
        }
    }

    #[test]
    fn duplicate_attribution_ids_do_not_double_count_tokens() {
        let dashboard = DashboardState {
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
            recent_usage: vec![],
            insights: vec![],
            required_terms_version: 0,
            accepted_terms_version: 0,
            terms_url: String::new(),
        };
        let normalized = normalize(
            &dashboard,
            vec![
                event("same", SavingsAttributionSource::Rtk),
                event("same", SavingsAttributionSource::Rtk),
            ],
            |_| true,
            |_| true,
        );
        assert_eq!(normalized.saved_tokens, 10);
        let impact = normalized.source_impacts.first().expect("source impact");
        assert_eq!(impact.runtime_evidence_units, 1);
        assert_eq!(impact.measured_event_count, 1);
        assert_eq!(impact.estimated_event_count, 0);
        assert_eq!(impact.inferred_event_count, 0);
        assert_eq!(impact.total_tokens_sent, 100);
    }

    #[test]
    fn repo_intelligence_is_avoided_not_compression_saved() {
        let dashboard = DashboardState {
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
            recent_usage: vec![],
            insights: vec![],
            required_terms_version: 0,
            accepted_terms_version: 0,
            terms_url: String::new(),
        };
        let normalized = normalize(
            &dashboard,
            vec![event("repo", SavingsAttributionSource::RepoIntelligence)],
            |_| true,
            |_| true,
        );
        assert_eq!(normalized.saved_tokens, 0);
        assert_eq!(normalized.avoided_tokens, 10);
    }

    #[test]
    fn day_key_uses_calendar_local_date() {
        let now = Utc::now() - Duration::minutes(1);
        assert_eq!(local_day_key(now).len(), 10);
    }
}
