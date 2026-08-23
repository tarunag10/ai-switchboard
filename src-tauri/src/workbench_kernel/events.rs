use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub const WORKBENCH_EVENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_EVENT_COUNT: usize = 512;
const MAX_IDENTIFIER_LENGTH: usize = 128;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchEventKind {
    Started,
    Attached,
    Checkpoint,
    Paused,
    Resumed,
    Cancelled,
    Completed,
    Forked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchSessionStatus {
    Active,
    Paused,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchSessionAction {
    Pause,
    Resume,
    Cancel,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEvent {
    pub event_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub kind: WorkbenchEventKind,
    pub parent_event_id: Option<String>,
    pub occurred_at: String,
}

pub(crate) fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.len() > MAX_IDENTIFIER_LENGTH
        || value.chars().any(char::is_control)
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.')
        })
    {
        bail!("Workbench {label} must be a bounded opaque identifier");
    }
    Ok(())
}

pub(crate) fn transition_status(
    status: WorkbenchSessionStatus,
    kind: WorkbenchEventKind,
) -> Result<WorkbenchSessionStatus> {
    match (status, kind) {
        (WorkbenchSessionStatus::Active, WorkbenchEventKind::Paused) => {
            Ok(WorkbenchSessionStatus::Paused)
        }
        (WorkbenchSessionStatus::Paused, WorkbenchEventKind::Resumed) => {
            Ok(WorkbenchSessionStatus::Active)
        }
        (
            WorkbenchSessionStatus::Active | WorkbenchSessionStatus::Paused,
            WorkbenchEventKind::Cancelled,
        ) => Ok(WorkbenchSessionStatus::Cancelled),
        (WorkbenchSessionStatus::Active, WorkbenchEventKind::Completed) => {
            Ok(WorkbenchSessionStatus::Completed)
        }
        (
            WorkbenchSessionStatus::Active | WorkbenchSessionStatus::Paused,
            WorkbenchEventKind::Attached
            | WorkbenchEventKind::Checkpoint
            | WorkbenchEventKind::Forked,
        ) => Ok(status),
        _ => bail!("Workbench event {kind:?} is not allowed from {status:?}"),
    }
}

pub(crate) fn new_event(
    session_id: &str,
    sequence: u64,
    kind: WorkbenchEventKind,
    parent_event_id: Option<String>,
) -> WorkbenchEvent {
    WorkbenchEvent {
        event_id: format!("{session_id}:{sequence}"),
        session_id: session_id.to_string(),
        sequence,
        kind,
        parent_event_id,
        occurred_at: Utc::now().to_rfc3339(),
    }
}

pub(crate) fn validate_event(
    event: &WorkbenchEvent,
    expected_session_id: &str,
    index: usize,
) -> Result<()> {
    validate_identifier(&event.event_id, "event ID")?;
    validate_identifier(&event.session_id, "session ID")?;
    if event.session_id != expected_session_id {
        return Err(anyhow!(
            "Workbench event {index} belongs to another session"
        ));
    }
    if event.sequence != index as u64 {
        return Err(anyhow!(
            "Workbench event sequence must be contiguous at index {index}"
        ));
    }
    if let Some(parent_event_id) = &event.parent_event_id {
        validate_identifier(parent_event_id, "parent event ID")?;
    }
    chrono::DateTime::parse_from_rfc3339(&event.occurred_at)
        .map_err(|_| anyhow!("Workbench event {index} has an invalid timestamp"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        transition_status, validate_identifier, WorkbenchEventKind, WorkbenchSessionStatus,
    };

    #[test]
    fn identifiers_are_opaque_and_content_free() {
        assert!(validate_identifier("workbench:7f3a", "session ID").is_ok());
        assert!(validate_identifier("/Users/alice/project", "session ID").is_err());
        assert!(validate_identifier("prompt with spaces", "session ID").is_err());
        assert!(validate_identifier("secret\nvalue", "session ID").is_err());
    }

    #[test]
    fn session_transitions_are_fail_closed() {
        assert_eq!(
            transition_status(WorkbenchSessionStatus::Active, WorkbenchEventKind::Paused)
                .expect("pause active session"),
            WorkbenchSessionStatus::Paused
        );
        assert!(transition_status(
            WorkbenchSessionStatus::Paused,
            WorkbenchEventKind::Completed
        )
        .is_err());
        assert!(transition_status(
            WorkbenchSessionStatus::Completed,
            WorkbenchEventKind::Attached
        )
        .is_err());
    }
}
