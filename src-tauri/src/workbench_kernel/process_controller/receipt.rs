use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::super::events::{validate_identifier, WorkbenchSessionStatus};
use super::super::session::validate_digest;
use super::super::{ProcessRunSpec, WorkbenchProcessAdmission, WorkbenchSession};
use super::stream::FakeStreamMetadata;

const CONTROLLER_SCHEMA_VERSION: u32 = 1;
const EXECUTION_MODE: &str = "fake_no_process";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FakeProcessState {
    Authorized,
    Starting,
    Running,
    Stopping,
    Succeeded,
    Failed,
    Cancelled,
    Orphaned,
}

impl FakeProcessState {
    pub(super) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Orphaned
        )
    }

    pub(super) fn is_active_across_restart(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Stopping)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FakeTerminalOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Orphaned,
}

impl FakeTerminalOutcome {
    pub(super) fn state(self) -> FakeProcessState {
        match self {
            Self::Succeeded => FakeProcessState::Succeeded,
            Self::Failed => FakeProcessState::Failed,
            Self::Cancelled => FakeProcessState::Cancelled,
            Self::Orphaned => FakeProcessState::Orphaned,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkbenchFakeProcessReceipt {
    pub schema_version: u32,
    pub receipt_id: String,
    pub binding_digest: String,
    pub admission_id: String,
    pub admission_receipt_digest: String,
    pub process_run_id: String,
    pub process_run_spec_digest: String,
    pub session_id: String,
    pub session_binding_digest: String,
    pub workspace_digest: String,
    pub state: FakeProcessState,
    pub terminal_outcome: Option<FakeTerminalOutcome>,
    pub registered_sequence: u64,
    pub revision: u64,
    pub stream_metadata: FakeStreamMetadata,
    pub execution_mode: String,
    pub provider_traffic: String,
    pub writes_enabled: bool,
    pub receipt_digest: String,
}

impl WorkbenchFakeProcessReceipt {
    pub(super) fn from_bindings(
        session: &WorkbenchSession,
        process: &ProcessRunSpec,
        admission: &WorkbenchProcessAdmission,
        registered_sequence: u64,
    ) -> Result<Self> {
        validate_bindings(session, process, admission)?;
        let process_run_spec_digest = sha256_serializable(process)?;
        let session_binding_digest = sha256_json(&serde_json::json!({
            "schemaVersion": session.schema_version,
            "sessionId": &session.session_id,
            "workspaceDigest": &session.workspace_digest,
            "taskClass": &session.task_class,
            "status": session.status,
            "executionMode": &session.execution_mode,
            "providerTraffic": &session.provider_traffic,
        }))?;
        let binding_digest = binding_digest(
            &admission.admission_id,
            &admission.receipt_digest,
            &process.run_id,
            &process_run_spec_digest,
            &session.session_id,
            &session_binding_digest,
            &session.workspace_digest,
        )?;
        let receipt_id = format!(
            "fake-process-receipt:{}",
            &binding_digest
                .strip_prefix("sha256:")
                .expect("generated digest has SHA-256 prefix")[..32]
        );
        let mut receipt = Self {
            schema_version: CONTROLLER_SCHEMA_VERSION,
            receipt_id,
            binding_digest,
            admission_id: admission.admission_id.clone(),
            admission_receipt_digest: admission.receipt_digest.clone(),
            process_run_id: process.run_id.clone(),
            process_run_spec_digest,
            session_id: session.session_id.clone(),
            session_binding_digest,
            workspace_digest: session.workspace_digest.clone(),
            state: FakeProcessState::Authorized,
            terminal_outcome: None,
            registered_sequence,
            revision: 0,
            stream_metadata: FakeStreamMetadata::empty()?,
            execution_mode: EXECUTION_MODE.into(),
            provider_traffic: "none".into(),
            writes_enabled: false,
            receipt_digest: String::new(),
        };
        receipt.refresh_digest()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.schema_version != CONTROLLER_SCHEMA_VERSION
            || self.execution_mode != EXECUTION_MODE
            || self.provider_traffic != "none"
            || self.writes_enabled
        {
            bail!("Workbench fake process receipt violates the no-process boundary");
        }
        for (value, label) in [
            (&self.receipt_id, "fake process receipt ID"),
            (&self.admission_id, "process admission ID"),
            (&self.process_run_id, "process run ID"),
            (&self.session_id, "session ID"),
        ] {
            validate_identifier(value, label)?;
        }
        for (value, label) in [
            (&self.binding_digest, "fake process binding digest"),
            (
                &self.admission_receipt_digest,
                "process admission receipt digest",
            ),
            (&self.process_run_spec_digest, "process run spec digest"),
            (&self.session_binding_digest, "session binding digest"),
            (&self.workspace_digest, "workspace digest"),
            (&self.receipt_digest, "fake process receipt digest"),
        ] {
            validate_digest(value, label)?;
        }
        let expected_outcome = match self.state {
            FakeProcessState::Succeeded => Some(FakeTerminalOutcome::Succeeded),
            FakeProcessState::Failed => Some(FakeTerminalOutcome::Failed),
            FakeProcessState::Cancelled => Some(FakeTerminalOutcome::Cancelled),
            FakeProcessState::Orphaned => Some(FakeTerminalOutcome::Orphaned),
            FakeProcessState::Authorized
            | FakeProcessState::Starting
            | FakeProcessState::Running
            | FakeProcessState::Stopping => None,
        };
        if self.terminal_outcome != expected_outcome
            || (self.state != FakeProcessState::Authorized && self.revision == 0)
        {
            bail!("Workbench fake process receipt lifecycle metadata is inconsistent");
        }
        self.stream_metadata.validate()?;
        let expected_binding = binding_digest(
            &self.admission_id,
            &self.admission_receipt_digest,
            &self.process_run_id,
            &self.process_run_spec_digest,
            &self.session_id,
            &self.session_binding_digest,
            &self.workspace_digest,
        )?;
        if self.binding_digest != expected_binding {
            bail!("Workbench fake process binding digest does not match its identifiers");
        }
        let expected_receipt_id = format!(
            "fake-process-receipt:{}",
            &self
                .binding_digest
                .strip_prefix("sha256:")
                .expect("validated digest has SHA-256 prefix")[..32]
        );
        if self.receipt_id != expected_receipt_id
            || self.receipt_digest != self.expected_digest()?
        {
            bail!("Workbench fake process receipt digest does not match its content");
        }
        Ok(())
    }

    pub(super) fn transition_to(
        &mut self,
        state: FakeProcessState,
        terminal_outcome: Option<FakeTerminalOutcome>,
    ) -> Result<()> {
        self.state = state;
        self.terminal_outcome = terminal_outcome;
        self.bump_revision()?;
        self.refresh_digest()?;
        self.validate()
    }

    pub(super) fn bump_revision(&mut self) -> Result<()> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| anyhow!("Workbench fake process receipt revision overflow"))?;
        Ok(())
    }

    pub(super) fn refresh_digest(&mut self) -> Result<()> {
        self.receipt_digest = self.expected_digest()?;
        Ok(())
    }

    fn expected_digest(&self) -> Result<String> {
        sha256_json(&serde_json::json!({
            "schemaVersion": self.schema_version,
            "receiptId": &self.receipt_id,
            "bindingDigest": &self.binding_digest,
            "admissionId": &self.admission_id,
            "admissionReceiptDigest": &self.admission_receipt_digest,
            "processRunId": &self.process_run_id,
            "processRunSpecDigest": &self.process_run_spec_digest,
            "sessionId": &self.session_id,
            "sessionBindingDigest": &self.session_binding_digest,
            "workspaceDigest": &self.workspace_digest,
            "state": self.state,
            "terminalOutcome": self.terminal_outcome,
            "registeredSequence": self.registered_sequence,
            "revision": self.revision,
            "streamMetadata": &self.stream_metadata,
            "executionMode": &self.execution_mode,
            "providerTraffic": &self.provider_traffic,
            "writesEnabled": self.writes_enabled,
        }))
    }
}

pub(super) fn validate_bindings(
    session: &WorkbenchSession,
    process: &ProcessRunSpec,
    admission: &WorkbenchProcessAdmission,
) -> Result<()> {
    session.validate()?;
    process.validate()?;
    admission.validate()?;
    if session.status != WorkbenchSessionStatus::Active
        || process.session_id != session.session_id
        || process.workspace_digest != session.workspace_digest
        || admission.session_id != session.session_id
        || admission.process_run_id != process.run_id
        || admission.adapter_id != process.adapter_id
        || admission.state != "authorized_not_started"
        || admission.execution_enabled
        || admission.provider_traffic != "none"
        || admission.writes_enabled
    {
        bail!("Workbench fake process bindings are invalid");
    }
    Ok(())
}

fn binding_digest(
    admission_id: &str,
    admission_receipt_digest: &str,
    process_run_id: &str,
    process_run_spec_digest: &str,
    session_id: &str,
    session_binding_digest: &str,
    workspace_digest: &str,
) -> Result<String> {
    sha256_json(&serde_json::json!({
        "admissionId": admission_id,
        "admissionReceiptDigest": admission_receipt_digest,
        "processRunId": process_run_id,
        "processRunSpecDigest": process_run_spec_digest,
        "sessionId": session_id,
        "sessionBindingDigest": session_binding_digest,
        "workspaceDigest": workspace_digest,
    }))
}

fn sha256_serializable<T: Serialize>(value: &T) -> Result<String> {
    sha256_bytes(&serde_json::to_vec(value)?)
}

pub(super) fn sha256_json(value: &serde_json::Value) -> Result<String> {
    sha256_bytes(&serde_json::to_vec(value)?)
}

fn sha256_bytes(bytes: &[u8]) -> Result<String> {
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
