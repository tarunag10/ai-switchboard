use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::events::{
    new_event, transition_status, validate_event, validate_identifier, WorkbenchEvent,
    WorkbenchEventKind, WorkbenchSessionAction, WorkbenchSessionStatus, MAX_EVENT_COUNT,
    WORKBENCH_EVENT_SCHEMA_VERSION,
};

const SHA256_PREFIX: &str = "sha256:";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateWorkbenchSessionInput {
    pub workspace_digest: String,
    pub task_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchSession {
    pub schema_version: u32,
    pub session_id: String,
    pub workspace_digest: String,
    pub task_class: String,
    pub status: WorkbenchSessionStatus,
    pub parent_session_id: Option<String>,
    pub fork_event_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub execution_mode: String,
    pub provider_traffic: String,
    pub events: Vec<WorkbenchEvent>,
}

pub(crate) fn validate_digest(value: &str, label: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
        bail!("Workbench {label} must be a SHA-256 digest");
    };
    if hex.len() != 64 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        bail!("Workbench {label} must be a SHA-256 digest");
    }
    Ok(())
}

fn validate_task_class(value: &str) -> Result<()> {
    validate_identifier(value, "task class")?;
    if !matches!(value, "coding" | "review" | "analysis" | "planning") {
        bail!("Workbench task class is not supported in plan-only mode");
    }
    Ok(())
}

impl WorkbenchSession {
    pub(crate) fn create(input: CreateWorkbenchSessionInput) -> Result<Self> {
        validate_digest(&input.workspace_digest, "workspace digest")?;
        validate_task_class(&input.task_class)?;
        let session_id = format!("workbench:{}", Uuid::new_v4());
        let created_at = Utc::now().to_rfc3339();
        let event = new_event(&session_id, 0, WorkbenchEventKind::Started, None);
        Ok(Self {
            schema_version: WORKBENCH_EVENT_SCHEMA_VERSION,
            session_id,
            workspace_digest: input.workspace_digest,
            task_class: input.task_class,
            status: WorkbenchSessionStatus::Active,
            parent_session_id: None,
            fork_event_id: None,
            created_at: created_at.clone(),
            updated_at: created_at,
            execution_mode: "plan_only".into(),
            provider_traffic: "none".into(),
            events: vec![event],
        })
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.schema_version != WORKBENCH_EVENT_SCHEMA_VERSION {
            bail!("Unsupported Workbench session schema version");
        }
        validate_identifier(&self.session_id, "session ID")?;
        validate_digest(&self.workspace_digest, "workspace digest")?;
        validate_task_class(&self.task_class)?;
        if self.execution_mode != "plan_only" || self.provider_traffic != "none" {
            bail!("Workbench session violates the plan-only execution boundary");
        }
        if self.events.is_empty() || self.events.len() > MAX_EVENT_COUNT {
            bail!("Workbench session has an invalid event count");
        }
        let mut derived_status = None;
        for (index, event) in self.events.iter().enumerate() {
            validate_event(event, &self.session_id, index)?;
            if index == 0 {
                if event.kind != WorkbenchEventKind::Started {
                    bail!("Workbench session must start with a root started event");
                }
                derived_status = Some(WorkbenchSessionStatus::Active);
                continue;
            }
            derived_status = Some(transition_status(
                derived_status.expect("first event establishes Workbench status"),
                event.kind,
            )?);
        }
        if Some(self.status) != derived_status {
            bail!("Workbench session status does not match its event ledger");
        }
        if let Some(parent_session_id) = &self.parent_session_id {
            validate_identifier(parent_session_id, "parent session ID")?;
            let fork_event_id = self
                .fork_event_id
                .as_deref()
                .ok_or_else(|| anyhow!("Forked Workbench session requires a parent event ID"))?;
            validate_identifier(fork_event_id, "fork event ID")?;
            if self.events[0].parent_event_id.as_deref() != Some(fork_event_id) {
                bail!("Forked Workbench session root event does not reference its fork event");
            }
        } else if self.fork_event_id.is_some() || self.events[0].parent_event_id.is_some() {
            bail!("Root Workbench session cannot contain fork lineage");
        }
        Ok(())
    }

    pub(crate) fn transition(&mut self, action: WorkbenchSessionAction) -> Result<()> {
        if self.events.len() >= MAX_EVENT_COUNT {
            bail!("Workbench session reached its event retention cap");
        }
        let kind = match action {
            WorkbenchSessionAction::Pause => WorkbenchEventKind::Paused,
            WorkbenchSessionAction::Resume => WorkbenchEventKind::Resumed,
            WorkbenchSessionAction::Cancel => WorkbenchEventKind::Cancelled,
            WorkbenchSessionAction::Complete => WorkbenchEventKind::Completed,
        };
        let next_status = transition_status(self.status, kind)?;
        self.events.push(new_event(
            &self.session_id,
            self.events.len() as u64,
            kind,
            None,
        ));
        self.status = next_status;
        self.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    pub(crate) fn fork_at_event(&mut self, event_id: &str) -> Result<Self> {
        validate_identifier(event_id, "fork event ID")?;
        if !matches!(
            self.status,
            WorkbenchSessionStatus::Active | WorkbenchSessionStatus::Paused
        ) {
            bail!("Workbench session can only fork while active or paused");
        }
        if !self.events.iter().any(|event| event.event_id == event_id) {
            bail!("Workbench fork event does not belong to this session");
        }
        if self.events.len() >= MAX_EVENT_COUNT {
            bail!("Workbench session reached its event retention cap");
        }
        self.events.push(new_event(
            &self.session_id,
            self.events.len() as u64,
            WorkbenchEventKind::Forked,
            Some(event_id.to_string()),
        ));
        self.updated_at = Utc::now().to_rfc3339();
        let session_id = deterministic_fork_session_id(&self.session_id, event_id);
        let created_at = Utc::now().to_rfc3339();
        let root_event = new_event(
            &session_id,
            0,
            WorkbenchEventKind::Started,
            Some(event_id.to_string()),
        );
        Ok(Self {
            schema_version: WORKBENCH_EVENT_SCHEMA_VERSION,
            session_id,
            workspace_digest: self.workspace_digest.clone(),
            task_class: self.task_class.clone(),
            status: WorkbenchSessionStatus::Active,
            parent_session_id: Some(self.session_id.clone()),
            fork_event_id: Some(event_id.to_string()),
            created_at: created_at.clone(),
            updated_at: created_at,
            execution_mode: "plan_only".into(),
            provider_traffic: "none".into(),
            events: vec![root_event],
        })
    }
}

pub(crate) fn deterministic_fork_session_id(parent_session_id: &str, event_id: &str) -> String {
    let digest = Sha256::digest(format!("{parent_session_id}:{event_id}").as_bytes());
    format!("fork:{:x}", digest)[..37].to_string()
}

#[cfg(test)]
mod tests {
    use super::{CreateWorkbenchSessionInput, WorkbenchSession};
    use crate::workbench_kernel::events::{WorkbenchSessionAction, WorkbenchSessionStatus};

    fn input() -> CreateWorkbenchSessionInput {
        CreateWorkbenchSessionInput {
            workspace_digest: format!("sha256:{}", "a".repeat(64)),
            task_class: "coding".into(),
        }
    }

    #[test]
    fn session_is_content_free_and_lifecycle_valid() {
        let mut session = WorkbenchSession::create(input()).expect("create session");
        session
            .transition(WorkbenchSessionAction::Pause)
            .expect("pause");
        session
            .transition(WorkbenchSessionAction::Resume)
            .expect("resume");
        session
            .transition(WorkbenchSessionAction::Complete)
            .expect("complete");
        session.validate().expect("valid session ledger");
        assert_eq!(session.status, WorkbenchSessionStatus::Completed);
        assert_eq!(session.execution_mode, "plan_only");
        assert_eq!(session.provider_traffic, "none");
    }

    #[test]
    fn fork_is_deterministic_and_preserves_lineage_without_content() {
        let mut session = WorkbenchSession::create(input()).expect("create session");
        let event_id = session.events[0].event_id.clone();
        let fork = session.fork_at_event(&event_id).expect("fork session");
        session.validate().expect("valid parent");
        fork.validate().expect("valid fork");
        assert_eq!(
            fork.parent_session_id.as_deref(),
            Some(session.session_id.as_str())
        );
        assert_eq!(fork.fork_event_id.as_deref(), Some(event_id.as_str()));
        assert!(fork.session_id.starts_with("fork:"));
    }

    #[test]
    fn invalid_digest_and_terminal_transition_are_rejected() {
        let invalid = WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: "workspace-path".into(),
            task_class: "coding".into(),
        });
        assert!(invalid.is_err());
        let mut session = WorkbenchSession::create(input()).expect("create session");
        session
            .transition(WorkbenchSessionAction::Cancel)
            .expect("cancel");
        assert!(session.transition(WorkbenchSessionAction::Resume).is_err());
    }

    #[test]
    fn session_input_rejects_prompt_like_fields() {
        let payload = format!(
            r#"{{"workspaceDigest":"sha256:{}","taskClass":"coding","prompt":"do work"}}"#,
            "f".repeat(64)
        );
        assert!(serde_json::from_str::<CreateWorkbenchSessionInput>(&payload).is_err());
    }
}
