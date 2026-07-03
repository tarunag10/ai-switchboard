use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedundancyInputItem {
    pub(crate) source_id: String,
    pub(crate) content: String,
    pub(crate) estimated_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedundancyEvent {
    pub(crate) source_id: String,
    pub(crate) content_sha256: String,
    pub(crate) estimated_tokens: u64,
    pub(crate) repeated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedundancyReport {
    pub(crate) total_events: usize,
    pub(crate) repeated_events: usize,
    pub(crate) total_tokens: u64,
    pub(crate) repeated_tokens: u64,
    pub(crate) redundancy_percent: f64,
    pub(crate) events: Vec<RedundancyEvent>,
}

pub(crate) fn sha256_hex(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

pub(crate) fn build_redundancy_report(items: &[(String, String, u64)]) -> RedundancyReport {
    let mut seen = HashSet::new();
    let mut events = Vec::with_capacity(items.len());
    let mut total_tokens = 0_u64;
    let mut repeated_tokens = 0_u64;

    for (source_id, content, estimated_tokens) in items {
        let content_sha256 = sha256_hex(content);
        let repeated = !seen.insert(content_sha256.clone());
        total_tokens = total_tokens.saturating_add(*estimated_tokens);
        if repeated {
            repeated_tokens = repeated_tokens.saturating_add(*estimated_tokens);
        }
        events.push(RedundancyEvent {
            source_id: source_id.clone(),
            content_sha256,
            estimated_tokens: *estimated_tokens,
            repeated,
        });
    }

    let repeated_events = events.iter().filter(|event| event.repeated).count();
    let redundancy_percent = if total_tokens == 0 {
        0.0
    } else {
        (repeated_tokens as f64 / total_tokens as f64) * 100.0
    };

    RedundancyReport {
        total_events: events.len(),
        repeated_events,
        total_tokens,
        repeated_tokens,
        redundancy_percent,
        events,
    }
}

pub(crate) fn build_redundancy_report_from_inputs(
    items: Vec<RedundancyInputItem>,
) -> RedundancyReport {
    let tuples = items
        .into_iter()
        .map(|item| (item.source_id, item.content, item.estimated_tokens))
        .collect::<Vec<_>>();
    build_redundancy_report(&tuples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_repeated_content_without_storing_raw_text() {
        let report = build_redundancy_report(&[
            ("first".to_string(), "same".to_string(), 10),
            ("second".to_string(), "same".to_string(), 20),
            ("third".to_string(), "different".to_string(), 30),
        ]);

        assert_eq!(report.total_events, 3);
        assert_eq!(report.repeated_events, 1);
        assert_eq!(report.repeated_tokens, 20);
        assert!(report.events[1].repeated);
        assert_ne!(report.events[1].content_sha256, "same");
    }
}
