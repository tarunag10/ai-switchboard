//! Deterministic normalization of the existing local usage and savings ledger.

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Local, Utc};

use crate::analytics_models::{AnalyticsEvidenceConfidence, OptimizationImpactV1};
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
                evidence: events
                    .iter()
                    .flat_map(|event| event.evidence.iter().cloned())
                    .take(5)
                    .collect(),
                last_observed_at: events.iter().map(|event| event.observed_at).max(),
            }
        })
        .collect();

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
