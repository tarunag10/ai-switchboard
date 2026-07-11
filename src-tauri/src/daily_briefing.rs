//! Current-day, local-only Daily AI Usage Briefing.

use std::collections::BTreeMap;

use chrono::{Local, Utc};
use serde::Serialize;

use crate::analytics_models::{
    AnalyticsEvidenceConfidence, BriefingActionKindV1, BriefingAttentionItemV1,
    BriefingCompletenessV1, BriefingRecommendationV1, DailyUsageBriefingV1, DailyUsageTotalsV1,
    EvidenceCoverageV1, TokenMetricV1, UsageRollupV1,
};
use crate::analytics_normalization::{is_failure, is_today, local_day_key, normalize};
use crate::models::{DashboardState, SavingsAttributionEvent, ToolStatus};

pub(crate) fn build_briefing(
    dashboard: &DashboardState,
    attribution: Vec<SavingsAttributionEvent>,
) -> DailyUsageBriefingV1 {
    let now = Utc::now();
    let day_key = local_day_key(now);
    let normalized = normalize(
        dashboard,
        attribution,
        |event| is_today(event.timestamp, &day_key),
        |event| is_today(event.observed_at, &day_key),
    );
    let daily = dashboard
        .daily_savings
        .iter()
        .find(|point| point.date == day_key);
    let costs = daily
        .map(|point| point.actual_cost_usd)
        .filter(|value| *value > 0.0);
    let cost_metric = match costs {
        Some(value) => TokenMetricV1 {
            value: Some(value),
            confidence: AnalyticsEvidenceConfidence::Measured,
            source: "headroom_daily_rollup".into(),
            observed_at: Some(now),
            caveat: None,
        },
        None => TokenMetricV1::unavailable(
            "headroom_daily_rollup",
            "No measured per-token spend is available for today.",
        ),
    };
    let source_confidences: Vec<_> = normalized
        .source_impacts
        .iter()
        .map(|impact| impact.confidence)
        .collect();
    let unavailable_metrics = u64::from(cost_metric.value.is_none());
    let completeness = if normalized.usage.is_empty() && normalized.attribution.is_empty() {
        BriefingCompletenessV1::InsufficientData
    } else if unavailable_metrics > 0 {
        BriefingCompletenessV1::Partial
    } else {
        BriefingCompletenessV1::Complete
    };
    let totals = DailyUsageTotalsV1 {
        requests: normalized.usage.len() as u64,
        active_agents: normalized.usage.iter().map(|event| event.client.as_str()).collect::<std::collections::HashSet<_>>().len() as u64,
        input_tokens: token_metric(normalized.input_tokens, "recent_usage", now),
        output_tokens: token_metric(normalized.output_tokens, "recent_usage", now),
        saved_tokens: token_metric(normalized.saved_tokens, "savings_attribution_ledger", now),
        avoided_tokens: token_metric(normalized.avoided_tokens, "savings_attribution_ledger", now),
        estimated_cost_usd: cost_metric,
        estimated_savings_usd: TokenMetricV1 {
            value: Some(normalized.estimated_savings_usd),
            confidence: source_confidences.into_iter().min_by_key(confidence_rank).unwrap_or(AnalyticsEvidenceConfidence::Unavailable),
            source: "savings_attribution_ledger".into(), observed_at: Some(now),
            caveat: Some("Savings are evidence-backed estimates and may include different optimization categories.".into()),
        },
    };
    let agents = rollups(&normalized.usage, |event| event.client.clone());
    let providers = rollups(&normalized.usage, |event| event.upstream_target.clone());
    let attention_items = attention_items(dashboard, &normalized.usage);
    let recommendations = recommendations(
        dashboard,
        &normalized.usage,
        normalized.input_tokens,
        normalized.saved_tokens,
    );
    let evidence_coverage = EvidenceCoverageV1 {
        measured_sources: normalized.source_impacts.iter().filter(|impact| matches!(impact.confidence, AnalyticsEvidenceConfidence::Measured)).count() as u64,
        estimated_sources: normalized.source_impacts.iter().filter(|impact| matches!(impact.confidence, AnalyticsEvidenceConfidence::Estimated)).count() as u64,
        inferred_sources: normalized.source_impacts.iter().filter(|impact| matches!(impact.confidence, AnalyticsEvidenceConfidence::Inferred)).count() as u64,
        unavailable_metrics,
        notes: vec!["Current-day briefing uses local in-memory usage plus the existing savings attribution ledger; it does not persist prompts or responses.".into()],
    };
    DailyUsageBriefingV1 {
        schema_version: 1,
        day_key,
        timezone: Local::now().format("%Z").to_string(),
        generated_at: now,
        completeness,
        totals,
        agents,
        providers,
        attention_items,
        recommendations,
        evidence_coverage,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageBriefingExportV1 {
    pub briefing: DailyUsageBriefingV1,
    pub markdown: String,
}

pub(crate) fn export(briefing: DailyUsageBriefingV1) -> DailyUsageBriefingExportV1 {
    let mut markdown = format!("# Daily AI Usage Briefing — {}\n\n", briefing.day_key);
    markdown.push_str(&format!(
        "Requests: {} · Active agents: {}\n\n",
        briefing.totals.requests, briefing.totals.active_agents
    ));
    if let Some(tokens) = briefing.totals.input_tokens.value {
        markdown.push_str(&format!("Input tokens: {:.0}\n", tokens));
    }
    if let Some(tokens) = briefing.totals.saved_tokens.value {
        markdown.push_str(&format!("Saved tokens: {:.0}\n", tokens));
    }
    if let Some(cost) = briefing.totals.estimated_cost_usd.value {
        markdown.push_str(&format!("Estimated cost: ${cost:.4}\n"));
    }
    if !briefing.recommendations.is_empty() {
        markdown.push_str("\n## Recommended next steps\n");
        for recommendation in &briefing.recommendations {
            markdown.push_str(&format!(
                "- {}: {}\n",
                recommendation.action_label, recommendation.reason
            ));
        }
    }
    DailyUsageBriefingExportV1 { briefing, markdown }
}

fn token_metric(value: u64, source: &str, now: chrono::DateTime<Utc>) -> TokenMetricV1 {
    TokenMetricV1 {
        value: Some(value as f64),
        confidence: AnalyticsEvidenceConfidence::Estimated,
        source: source.into(),
        observed_at: Some(now),
        caveat: Some("Proxy estimates; provider request telemetry may differ.".into()),
    }
}

fn rollups(
    usage: &[crate::models::UsageEvent],
    key: impl Fn(&crate::models::UsageEvent) -> String,
) -> Vec<UsageRollupV1> {
    let mut grouped: BTreeMap<String, UsageRollupV1> = BTreeMap::new();
    for event in usage {
        let entry = grouped
            .entry(key(event).to_lowercase())
            .or_insert_with(|| UsageRollupV1 {
                id: key(event),
                requests: 0,
                input_tokens: 0,
                output_tokens: 0,
                failures: 0,
            });
        entry.requests += 1;
        entry.input_tokens = entry
            .input_tokens
            .saturating_add(event.estimated_input_tokens);
        entry.output_tokens = entry
            .output_tokens
            .saturating_add(event.estimated_output_tokens);
        entry.failures += u64::from(is_failure(&event.outcome));
    }
    grouped.into_values().collect()
}

fn attention_items(
    dashboard: &DashboardState,
    usage: &[crate::models::UsageEvent],
) -> Vec<BriefingAttentionItemV1> {
    let mut items: Vec<_> = dashboard
        .insights
        .iter()
        .filter(|insight| !matches!(insight.severity, crate::models::InsightSeverity::Info))
        .map(|insight| BriefingAttentionItemV1 {
            id: insight.id.clone(),
            severity: format!("{:?}", insight.severity).to_lowercase(),
            title: insight.title.clone(),
            detail: insight.recommendation.clone(),
            evidence: vec![insight.evidence.clone()],
            destination: "/usage".into(),
        })
        .collect();
    let failures = usage
        .iter()
        .filter(|event| is_failure(&event.outcome))
        .count();
    if failures >= 2 {
        items.push(BriefingAttentionItemV1 {
            id: "repeated-route-failure".into(),
            severity: "warning".into(),
            title: "Repeated route failures".into(),
            detail: format!("{failures} local usage events ended in an error today."),
            evidence: vec!["recent_usage.outcome=error".into()],
            destination: "/doctor".into(),
        });
    }
    for tool in dashboard
        .tools
        .iter()
        .filter(|tool| tool.enabled && matches!(tool.status, ToolStatus::Degraded))
    {
        items.push(BriefingAttentionItemV1 { id: format!("runtime-degraded:{}", tool.id), severity: "warning".into(), title: format!("{} is degraded", tool.name), detail: "An enabled runtime needs attention before it can provide reliable optimization evidence.".into(), evidence: vec![tool.id.clone()], destination: "/doctor".into() });
    }
    items
}

fn recommendations(
    dashboard: &DashboardState,
    usage: &[crate::models::UsageEvent],
    input_tokens: u64,
    saved_tokens: u64,
) -> Vec<BriefingRecommendationV1> {
    let mut result = Vec::new();
    let failures = usage
        .iter()
        .filter(|event| is_failure(&event.outcome))
        .count();
    if failures >= 2 {
        result.push(recommendation(
            "repeated-route-failure",
            "warning",
            90,
            format!("{failures} requests failed today."),
            "Open Doctor",
            "/doctor",
        ));
    }
    if dashboard
        .tools
        .iter()
        .any(|tool| tool.enabled && matches!(tool.status, ToolStatus::Degraded))
    {
        result.push(recommendation(
            "runtime-degraded",
            "warning",
            80,
            "An enabled runtime is degraded, so optimization evidence may be incomplete.".into(),
            "Inspect runtime health",
            "/doctor",
        ));
    }
    if input_tokens >= 50_000 && saved_tokens.saturating_mul(100) < input_tokens.saturating_mul(10)
    {
        result.push(recommendation(
            "low-savings-high-volume",
            "info",
            50,
            "High estimated input volume has less than 10% tracked compression savings.".into(),
            "Review optimization mode",
            "/usage",
        ));
    }
    result.sort_by(|left, right| right.priority_score.cmp(&left.priority_score));
    result.truncate(3);
    result
}

fn recommendation(
    rule_id: &str,
    severity: &str,
    priority_score: u16,
    reason: String,
    action_label: &str,
    destination: &str,
) -> BriefingRecommendationV1 {
    BriefingRecommendationV1 {
        rule_id: rule_id.into(),
        severity: severity.into(),
        priority_score,
        reason,
        evidence: vec![format!("rule:{rule_id}")],
        action_label: action_label.into(),
        destination: destination.into(),
        action_kind: BriefingActionKindV1::Advisory,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_is_content_free_markdown() {
        assert!(export(DailyUsageBriefingV1 {
            schema_version: 1,
            day_key: "2026-01-01".into(),
            timezone: "UTC".into(),
            generated_at: Utc::now(),
            completeness: BriefingCompletenessV1::InsufficientData,
            totals: DailyUsageTotalsV1 {
                requests: 0,
                active_agents: 0,
                input_tokens: TokenMetricV1::unavailable("x", "x"),
                output_tokens: TokenMetricV1::unavailable("x", "x"),
                saved_tokens: TokenMetricV1::unavailable("x", "x"),
                avoided_tokens: TokenMetricV1::unavailable("x", "x"),
                estimated_cost_usd: TokenMetricV1::unavailable("x", "x"),
                estimated_savings_usd: TokenMetricV1::unavailable("x", "x")
            },
            agents: vec![],
            providers: vec![],
            attention_items: vec![],
            recommendations: vec![],
            evidence_coverage: EvidenceCoverageV1 {
                measured_sources: 0,
                estimated_sources: 0,
                inferred_sources: 0,
                unavailable_metrics: 8,
                notes: vec![]
            }
        })
        .markdown
        .contains("Daily AI Usage Briefing"));
    }
}
