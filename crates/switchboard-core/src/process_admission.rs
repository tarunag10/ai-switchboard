//! Provider-neutral process-admission receipts.
//!
//! This module owns only the content-free admission schema, identity, digest,
//! and validation rules. Plan/session/process-spec checks, persistence,
//! locking, and process launch remain outside core.

use anyhow::{bail, Context, Result};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ADMISSION_SCHEMA_VERSION: u32 = 1;
pub const ADMISSION_ADAPTER_ID: &str = "codex";
pub const AUTHORIZED_NOT_STARTED: &str = "authorized_not_started";

const MAX_IDENTIFIER_LENGTH: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchProcessAdmission {
    pub schema_version: u32,
    pub admission_id: String,
    pub session_id: String,
    pub plan_id: String,
    pub process_run_id: String,
    pub grant_id: String,
    pub adapter_id: String,
    pub admitted_at: String,
    pub state: String,
    pub execution_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
    pub receipt_digest: String,
}

/// Returns the deterministic identity used by the admission receipt.
pub fn process_admission_id_for(
    session_id: &str,
    plan_id: &str,
    process_run_id: &str,
    grant_id: &str,
    adapter_id: &str,
) -> Result<String> {
    let canonical = serde_json::json!({
        "sessionId": session_id,
        "planId": plan_id,
        "processRunId": process_run_id,
        "grantId": grant_id,
        "adapterId": adapter_id,
    });
    let bytes =
        serde_json::to_vec(&canonical).context("canonicalizing process admission identity")?;
    Ok(format!("process-admission:{:x}", Sha256::digest(bytes)))
}

/// Computes the receipt digest while excluding the identity and digest fields.
pub fn process_admission_digest(admission: &WorkbenchProcessAdmission) -> Result<String> {
    let canonical = serde_json::json!({
        "schemaVersion": admission.schema_version,
        "admissionId": &admission.admission_id,
        "sessionId": &admission.session_id,
        "planId": &admission.plan_id,
        "processRunId": &admission.process_run_id,
        "grantId": &admission.grant_id,
        "adapterId": &admission.adapter_id,
        "admittedAt": &admission.admitted_at,
        "state": &admission.state,
        "executionEnabled": admission.execution_enabled,
        "providerTraffic": &admission.provider_traffic,
        "writesEnabled": admission.writes_enabled,
    });
    let bytes = serde_json::to_vec(&canonical).context("canonicalizing process admission")?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

/// Validates receipt-only invariants. Cross-object bindings remain adapter-owned.
pub fn validate_process_admission(admission: &WorkbenchProcessAdmission) -> Result<()> {
    if admission.schema_version != ADMISSION_SCHEMA_VERSION
        || admission.adapter_id != ADMISSION_ADAPTER_ID
        || admission.state != AUTHORIZED_NOT_STARTED
        || admission.execution_enabled
        || admission.provider_traffic != "none"
        || admission.writes_enabled
    {
        bail!("Workbench process admission violates the non-executing boundary");
    }
    for (value, label) in [
        (&admission.admission_id, "process admission ID"),
        (&admission.session_id, "session ID"),
        (&admission.plan_id, "plan ID"),
        (&admission.process_run_id, "process run ID"),
        (&admission.grant_id, "process grant ID"),
    ] {
        validate_identifier(value, label)?;
    }
    DateTime::parse_from_rfc3339(&admission.admitted_at)
        .map_err(|_| anyhow::anyhow!("Workbench process admission time is invalid"))?;
    if admission.admission_id
        != process_admission_id_for(
            &admission.session_id,
            &admission.plan_id,
            &admission.process_run_id,
            &admission.grant_id,
            &admission.adapter_id,
        )?
    {
        bail!("Workbench process admission ID does not match its binding");
    }
    if admission.receipt_digest != process_admission_digest(admission)? {
        bail!("Workbench process admission receipt digest does not match its content");
    }
    Ok(())
}

impl WorkbenchProcessAdmission {
    pub fn validate(&self) -> Result<()> {
        validate_process_admission(self)
    }
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn admission() -> WorkbenchProcessAdmission {
        let mut admission = WorkbenchProcessAdmission {
            schema_version: ADMISSION_SCHEMA_VERSION,
            admission_id: process_admission_id_for(
                "workbench:test",
                "run-plan:test",
                "process-run:test",
                "process-grant:test",
                ADMISSION_ADAPTER_ID,
            )
            .expect("admission identity"),
            session_id: "workbench:test".into(),
            plan_id: "run-plan:test".into(),
            process_run_id: "process-run:test".into(),
            grant_id: "process-grant:test".into(),
            adapter_id: ADMISSION_ADAPTER_ID.into(),
            admitted_at: "2026-08-23T00:00:00Z".into(),
            state: AUTHORIZED_NOT_STARTED.into(),
            execution_enabled: false,
            provider_traffic: "none".into(),
            writes_enabled: false,
            receipt_digest: String::new(),
        };
        admission.receipt_digest = process_admission_digest(&admission).expect("admission digest");
        admission
    }

    #[test]
    fn serde_contract_rejects_unknown_fields() {
        let mut value = serde_json::to_value(admission()).expect("serialize admission");
        value["command"] = serde_json::json!(["codex"]);
        assert!(serde_json::from_value::<WorkbenchProcessAdmission>(value).is_err());
    }

    #[test]
    fn identity_and_digest_are_deterministic_and_tamper_evident() {
        let original = admission();
        assert_eq!(
            process_admission_id_for(
                &original.session_id,
                &original.plan_id,
                &original.process_run_id,
                &original.grant_id,
                &original.adapter_id,
            )
            .expect("identity"),
            original.admission_id
        );
        let mut changed = original.clone();
        changed.grant_id = "process-grant:changed".into();
        assert_ne!(
            process_admission_id_for(
                &original.session_id,
                &original.plan_id,
                &original.process_run_id,
                &original.grant_id,
                &original.adapter_id,
            )
            .expect("identity"),
            process_admission_id_for(
                &changed.session_id,
                &changed.plan_id,
                &changed.process_run_id,
                &changed.grant_id,
                &changed.adapter_id,
            )
            .expect("changed identity")
        );

        changed = original.clone();
        changed.state = "tampered".into();
        assert!(changed.validate().is_err());
    }

    #[test]
    fn validation_is_strictly_plan_only() {
        let mut changed = admission();
        changed.execution_enabled = true;
        changed.receipt_digest = process_admission_digest(&changed).expect("digest");
        assert!(changed.validate().is_err());

        let mut changed = admission();
        changed.provider_traffic = "enabled".into();
        changed.receipt_digest = process_admission_digest(&changed).expect("digest");
        assert!(changed.validate().is_err());

        admission().validate().expect("valid plan-only admission");
    }
}
