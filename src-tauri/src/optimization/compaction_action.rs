use chrono::Utc;
use serde::Serialize;

use super::snapshot;
use super::telemetry::{self, CompactionDecisionRecord};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreemptiveCompactionReceipt {
    pub(crate) recorded_at: String,
    pub(crate) triggered: bool,
    pub(crate) context_used_percent: f64,
    pub(crate) threshold_percent: u8,
    pub(crate) reason: String,
    pub(crate) action: String,
}

pub(crate) fn run_preemptive_compaction() -> PreemptiveCompactionReceipt {
    let compaction = snapshot::build_optimization_snapshot().compaction;

    telemetry::record_compaction_decision(CompactionDecisionRecord {
        should_compact: compaction.should_compact,
        context_used_percent: compaction.context_used_percent.round().clamp(0.0, 100.0) as u8,
        threshold_percent: compaction.threshold_percent,
        reason: compaction.reason.clone(),
    });

    let action = if compaction.should_compact {
        "Preemptive compaction queued for the current optimization session."
    } else {
        "Context is below the compaction threshold; Switchboard recorded the check."
    };

    PreemptiveCompactionReceipt {
        recorded_at: Utc::now().to_rfc3339(),
        triggered: compaction.should_compact,
        context_used_percent: compaction.context_used_percent,
        threshold_percent: compaction.threshold_percent,
        reason: compaction.reason,
        action: action.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_preemptive_compaction_receipt() {
        let _guard = telemetry::test_guard();
        telemetry::reset_for_tests();

        let receipt = run_preemptive_compaction();
        let snapshot = telemetry::snapshot();

        assert!(receipt.recorded_at.contains('T'));
        assert!(snapshot.compaction_decision.is_some());
    }
}
