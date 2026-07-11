//! Versioned, content-free read models for local usage analytics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEvidenceConfidence {
    Measured,
    Estimated,
    Inferred,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenMetricV1 {
    pub value: Option<f64>,
    pub confidence: AnalyticsEvidenceConfidence,
    pub source: String,
    pub observed_at: Option<DateTime<Utc>>,
    pub caveat: Option<String>,
}

impl TokenMetricV1 {
    pub(crate) fn unavailable(source: impl Into<String>, caveat: impl Into<String>) -> Self {
        Self {
            value: None,
            confidence: AnalyticsEvidenceConfidence::Unavailable,
            source: source.into(),
            observed_at: None,
            caveat: Some(caveat.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsFreshness {
    Live,
    Recent,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPressureBandV1 {
    Normal,
    Elevated,
    High,
    Critical,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPressureV1 {
    pub used_tokens: Option<u64>,
    pub limit_tokens: Option<u64>,
    pub percent: Option<f64>,
    pub band: ContextPressureBandV1,
    pub limit_source: String,
    pub caveat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationImpactV1 {
    pub source: String,
    pub confidence: AnalyticsEvidenceConfidence,
    pub tokens_saved: Option<u64>,
    pub estimated_savings_usd: Option<f64>,
    pub event_count: u64,
    pub evidence: Vec<String>,
    pub last_observed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenXrayEventKindV1 {
    Usage,
    Savings,
    Failure,
    Anomaly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenXrayEventV1 {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: TokenXrayEventKindV1,
    pub label: String,
    pub confidence: AnalyticsEvidenceConfidence,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageAnomalyV1 {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenXrayMetricsV1 {
    pub input_tokens: TokenMetricV1,
    pub output_tokens: TokenMetricV1,
    pub cache_read_tokens: TokenMetricV1,
    pub cache_write_tokens: TokenMetricV1,
    pub saved_tokens: TokenMetricV1,
    pub avoided_tokens: TokenMetricV1,
    pub estimated_cost_usd: TokenMetricV1,
    pub estimated_savings_usd: TokenMetricV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenXraySnapshotV1 {
    pub schema_version: u8,
    pub generated_at: DateTime<Utc>,
    pub session_id: String,
    pub agent: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub freshness: AnalyticsFreshness,
    pub metrics: TokenXrayMetricsV1,
    pub context_pressure: ContextPressureV1,
    pub sources: Vec<OptimizationImpactV1>,
    pub timeline: Vec<TokenXrayEventV1>,
    pub anomalies: Vec<UsageAnomalyV1>,
}

/// A compact, content-free projection for polling Token X-Ray changes. This
/// deliberately excludes source evidence and anomaly detail, which can be
/// useful in the full local snapshot but are unnecessary for a live refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenXrayLiveUpdateV1 {
    pub schema_version: u8,
    pub revision: u64,
    pub generated_at: DateTime<Utc>,
    pub agent: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub freshness: AnalyticsFreshness,
    pub metrics: TokenXrayMetricsV1,
    pub context_pressure: ContextPressureV1,
    pub timeline: Vec<TokenXrayEventV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefingCompletenessV1 {
    Complete,
    Partial,
    InsufficientData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageTotalsV1 {
    pub requests: u64,
    pub active_agents: u64,
    pub input_tokens: TokenMetricV1,
    pub output_tokens: TokenMetricV1,
    pub saved_tokens: TokenMetricV1,
    pub avoided_tokens: TokenMetricV1,
    pub estimated_cost_usd: TokenMetricV1,
    pub estimated_savings_usd: TokenMetricV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRollupV1 {
    pub id: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub failures: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefingAttentionItemV1 {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub evidence: Vec<String>,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefingActionKindV1 {
    ReadOnly,
    Advisory,
    StateChanging,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefingRecommendationV1 {
    pub rule_id: String,
    pub severity: String,
    pub priority_score: u16,
    pub reason: String,
    pub evidence: Vec<String>,
    pub action_label: String,
    pub destination: String,
    pub action_kind: BriefingActionKindV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceCoverageV1 {
    pub measured_sources: u64,
    pub estimated_sources: u64,
    pub inferred_sources: u64,
    pub unavailable_metrics: u64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageBriefingV1 {
    pub schema_version: u8,
    pub day_key: String,
    pub timezone: String,
    pub generated_at: DateTime<Utc>,
    pub completeness: BriefingCompletenessV1,
    pub totals: DailyUsageTotalsV1,
    pub agents: Vec<UsageRollupV1>,
    pub providers: Vec<UsageRollupV1>,
    pub attention_items: Vec<BriefingAttentionItemV1>,
    pub recommendations: Vec<BriefingRecommendationV1>,
    pub evidence_coverage: EvidenceCoverageV1,
}
