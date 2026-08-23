//! Content-free transport observations for the local proxy.
//!
//! This recorder is deliberately separate from model-routing evidence. It
//! records bounded lifecycle metadata only; it cannot infer task quality,
//! provider-billed cost, model identity, or promotion eligibility.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_OBSERVATIONS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TransportRoute { Headroom, DirectAnthropic, DirectOpenai, Cache }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TransportOutcome {
    Success, UpstreamHttpError, ConnectFailure, WriteFailure, ReadFailure,
    Timeout, ClientDisconnect, LocalRejection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportObservation {
    pub(crate) event_id: String,
    pub(crate) started_at_ms: u128,
    pub(crate) completed_at_ms: Option<u128>,
    pub(crate) route: TransportRoute,
    pub(crate) request_class: String,
    pub(crate) streaming: bool,
    pub(crate) status_code: Option<u16>,
    pub(crate) terminal_outcome: Option<TransportOutcome>,
}

#[derive(Clone, Default)]
pub(crate) struct TransportObservationRecorder {
    observations: Arc<Mutex<VecDeque<TransportObservation>>>,
}

impl TransportObservationRecorder {
    pub(crate) fn begin(&self, route: TransportRoute, request_class: impl Into<String>, streaming: bool) -> String {
        let event_id = Uuid::new_v4().to_string();
        let observation = TransportObservation {
            event_id: event_id.clone(), started_at_ms: now_ms(), completed_at_ms: None,
            route, request_class: bounded_request_class(request_class.into()), streaming,
            status_code: None, terminal_outcome: None,
        };
        let mut observations = self.observations.lock().expect("transport recorder poisoned");
        if observations.len() == MAX_OBSERVATIONS { observations.pop_front(); }
        observations.push_back(observation);
        event_id
    }

    /// Completes an event at most once; duplicate or unknown IDs are ignored.
    pub(crate) fn finish(&self, event_id: &str, status_code: Option<u16>, outcome: TransportOutcome) -> bool {
        let mut observations = self.observations.lock().expect("transport recorder poisoned");
        let Some(observation) = observations.iter_mut().find(|item| item.event_id == event_id) else { return false; };
        if observation.completed_at_ms.is_some() { return false; }
        observation.completed_at_ms = Some(now_ms());
        observation.status_code = status_code;
        observation.terminal_outcome = Some(outcome);
        true
    }

    pub(crate) fn snapshot(&self) -> Vec<TransportObservation> {
        self.observations.lock().expect("transport recorder poisoned").iter().cloned().collect()
    }
}

pub(crate) fn global() -> &'static TransportObservationRecorder {
    static RECORDER: OnceLock<TransportObservationRecorder> = OnceLock::new();
    RECORDER.get_or_init(TransportObservationRecorder::default)
}

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_millis()).unwrap_or(0)
}

fn bounded_request_class(value: String) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() { return "unknown".to_string(); }
    normalized.chars().take(64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finalization_is_exactly_once_and_content_free() {
        let recorder = TransportObservationRecorder::default();
        let id = recorder.begin(TransportRoute::DirectOpenai, " /v1/responses?secret=1 ", true);
        assert!(recorder.finish(&id, Some(200), TransportOutcome::Success));
        assert!(!recorder.finish(&id, Some(500), TransportOutcome::ReadFailure));
        let observation = recorder.snapshot().pop().expect("observation");
        assert_eq!(observation.request_class, "/v1/responses?secret=1");
        assert_eq!(observation.status_code, Some(200));
        assert_eq!(observation.terminal_outcome, Some(TransportOutcome::Success));
        assert!(observation.completed_at_ms.is_some());
    }

    #[test]
    fn unknown_event_cannot_create_completion() {
        let recorder = TransportObservationRecorder::default();
        assert!(!recorder.finish("missing", None, TransportOutcome::LocalRejection));
        assert!(recorder.snapshot().is_empty());
    }

    #[test]
    fn recorder_is_bounded() {
        let recorder = TransportObservationRecorder::default();
        for _ in 0..(MAX_OBSERVATIONS + 5) { recorder.begin(TransportRoute::Headroom, "anthropic", false); }
        assert_eq!(recorder.snapshot().len(), MAX_OBSERVATIONS);
    }
}

