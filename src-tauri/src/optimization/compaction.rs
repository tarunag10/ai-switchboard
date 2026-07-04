use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompactionInput {
    pub(crate) context_tokens: u64,
    pub(crate) context_window_tokens: u64,
    pub(crate) projected_next_turn_tokens: u64,
    pub(crate) threshold_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompactionDecision {
    pub(crate) should_compact: bool,
    pub(crate) utilization_percent: f64,
    pub(crate) projected_utilization_percent: f64,
    pub(crate) tokens_until_threshold: i64,
    pub(crate) reason: String,
}

pub(crate) fn decide_preemptive_compaction(input: CompactionInput) -> CompactionDecision {
    if input.context_window_tokens == 0 {
        return CompactionDecision {
            should_compact: false,
            utilization_percent: 0.0,
            projected_utilization_percent: 0.0,
            tokens_until_threshold: 0,
            reason: "missing_context_window".to_string(),
        };
    }

    let threshold_percent = input.threshold_percent.clamp(1, 100) as u64;
    let threshold_tokens = input
        .context_window_tokens
        .saturating_mul(threshold_percent)
        / 100;
    let projected_tokens = input
        .context_tokens
        .saturating_add(input.projected_next_turn_tokens);
    let should_compact = projected_tokens >= threshold_tokens;

    CompactionDecision {
        should_compact,
        utilization_percent: percent(input.context_tokens, input.context_window_tokens),
        projected_utilization_percent: percent(projected_tokens, input.context_window_tokens),
        tokens_until_threshold: threshold_tokens as i64 - projected_tokens as i64,
        reason: if should_compact {
            "projected_context_crosses_threshold".to_string()
        } else {
            "projected_context_below_threshold".to_string()
        },
    }
}

fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggers_before_threshold_crossing() {
        let decision = decide_preemptive_compaction(CompactionInput {
            context_tokens: 80,
            context_window_tokens: 100,
            projected_next_turn_tokens: 10,
            threshold_percent: 90,
        });

        assert!(decision.should_compact);
        assert_eq!(decision.reason, "projected_context_crosses_threshold");
    }

    #[test]
    fn stays_quiet_below_threshold() {
        let decision = decide_preemptive_compaction(CompactionInput {
            context_tokens: 60,
            context_window_tokens: 100,
            projected_next_turn_tokens: 10,
            threshold_percent: 90,
        });

        assert!(!decision.should_compact);
        assert_eq!(decision.tokens_until_threshold, 20);
    }
}
