//! Durable admission records for the future Workbench process supervisor.
//!
//! Admission is deliberately not launch. It records that an already verified,
//! canonical adapter has a live, plan-bound grant. No child, command, path,
//! argument, environment, prompt, credential, PID, output, provider request,
//! or workspace handle is accepted, stored, or created here.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::capability_grant::WorkbenchProcessStartGrant;
use super::events::validate_identifier;
use super::process_run_spec::ProcessRunSpec;
use super::{WorkbenchRunPlan, WorkbenchSession};

const ADMISSION_SCHEMA_VERSION: u32 = 1;
const ADMISSION_LEDGER_SCHEMA_VERSION: u32 = 1;
const ADMISSION_LEDGER_FILE: &str = "workbench-process-admissions.json";
const MAX_ADMISSIONS: usize = 128;
const AUTHORIZED_NOT_STARTED: &str = "authorized_not_started";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkbenchProcessAdmissionLedger {
    schema_version: u32,
    admissions: BTreeMap<String, WorkbenchProcessAdmission>,
}

impl Default for WorkbenchProcessAdmissionLedger {
    fn default() -> Self {
        Self {
            schema_version: ADMISSION_LEDGER_SCHEMA_VERSION,
            admissions: BTreeMap::new(),
        }
    }
}

pub(crate) struct WorkbenchProcessAdmissionStore {
    path: PathBuf,
}

pub(crate) fn admit_process(
    session: &WorkbenchSession,
    plan: &WorkbenchRunPlan,
    process: &ProcessRunSpec,
    grant: &WorkbenchProcessStartGrant,
    now: DateTime<Utc>,
) -> Result<WorkbenchProcessAdmission> {
    process.validate()?;
    grant.validate()?;
    if plan.adapter_id != "codex"
        || plan.session_id != session.session_id
        || plan.workspace_digest != session.workspace_digest
        || plan.execution_mode != "plan_only"
        || plan.provider_traffic != "none"
        || plan.writes_enabled
        || plan.command_readiness.is_none()
        || process.session_id != session.session_id
        || process.adapter_plan_id != plan.adapter_plan_id
        || grant.session_id != session.session_id
        || grant.plan_id != plan.plan_id
        || grant.process_run_id != process.run_id
        || grant.execution_enabled
        || grant.provider_traffic != "none"
        || grant.writes_enabled
    {
        bail!("Workbench process admission bindings are invalid");
    }
    let canonical = serde_json::json!({
        "sessionId": &session.session_id,
        "planId": &plan.plan_id,
        "processRunId": &process.run_id,
        "grantId": &grant.grant_id,
        "adapterId": "codex",
    });
    let digest = Sha256::digest(
        serde_json::to_vec(&canonical).context("canonicalizing Workbench process admission")?,
    );
    let mut admission = WorkbenchProcessAdmission {
        schema_version: ADMISSION_SCHEMA_VERSION,
        admission_id: format!("process-admission:{digest:x}"),
        session_id: session.session_id.clone(),
        plan_id: plan.plan_id.clone(),
        process_run_id: process.run_id.clone(),
        grant_id: grant.grant_id.clone(),
        adapter_id: "codex".into(),
        admitted_at: now.to_rfc3339(),
        state: AUTHORIZED_NOT_STARTED.into(),
        execution_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
        receipt_digest: String::new(),
    };
    admission.receipt_digest = admission_digest(&admission)?;
    admission.validate()?;
    Ok(admission)
}

impl WorkbenchProcessAdmission {
    fn validate(&self) -> Result<()> {
        if self.schema_version != ADMISSION_SCHEMA_VERSION
            || self.adapter_id != "codex"
            || self.state != AUTHORIZED_NOT_STARTED
            || self.execution_enabled
            || self.provider_traffic != "none"
            || self.writes_enabled
        {
            bail!("Workbench process admission violates the non-executing boundary");
        }
        for (value, label) in [
            (&self.admission_id, "process admission ID"),
            (&self.session_id, "session ID"),
            (&self.plan_id, "plan ID"),
            (&self.process_run_id, "process run ID"),
            (&self.grant_id, "process grant ID"),
        ] {
            validate_identifier(value, label)?;
        }
        DateTime::parse_from_rfc3339(&self.admitted_at)
            .map_err(|_| anyhow!("Workbench process admission time is invalid"))?;
        if self.receipt_digest != admission_digest(self)? {
            bail!("Workbench process admission receipt digest does not match its content");
        }
        Ok(())
    }
}

fn admission_digest(admission: &WorkbenchProcessAdmission) -> Result<String> {
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
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&canonical)?)
    ))
}

impl WorkbenchProcessAdmissionStore {
    pub(crate) fn in_app_storage() -> Self {
        Self {
            path: crate::storage::config_file(
                &crate::storage::app_data_dir(),
                ADMISSION_LEDGER_FILE,
            ),
        }
    }

    pub(crate) fn issue(
        &self,
        admission: WorkbenchProcessAdmission,
    ) -> Result<WorkbenchProcessAdmission> {
        admission.validate()?;
        let mut ledger = self.load()?;
        if let Some(existing) = ledger.admissions.get(&admission.admission_id) {
            return Ok(existing.clone());
        }
        if ledger.admissions.len() >= MAX_ADMISSIONS {
            bail!("Workbench process admission ledger is full");
        }
        ledger
            .admissions
            .insert(admission.admission_id.clone(), admission.clone());
        self.save(&ledger)?;
        Ok(admission)
    }

    pub(crate) fn list_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<WorkbenchProcessAdmission>> {
        validate_identifier(session_id, "session ID")?;
        let mut admissions = self
            .load()?
            .admissions
            .into_values()
            .filter(|admission| admission.session_id == session_id)
            .collect::<Vec<_>>();
        admissions.sort_by(|left, right| right.admitted_at.cmp(&left.admitted_at));
        Ok(admissions)
    }

    fn load(&self) -> Result<WorkbenchProcessAdmissionLedger> {
        if !self.path.exists() {
            return Ok(WorkbenchProcessAdmissionLedger::default());
        }
        let bytes = std::fs::read(&self.path).with_context(|| {
            format!(
                "reading Workbench process admission ledger {}",
                self.path.display()
            )
        })?;
        let ledger: WorkbenchProcessAdmissionLedger =
            serde_json::from_slice(&bytes).with_context(|| {
                format!(
                    "decoding Workbench process admission ledger {}",
                    self.path.display()
                )
            })?;
        if ledger.schema_version != ADMISSION_LEDGER_SCHEMA_VERSION
            || ledger.admissions.len() > MAX_ADMISSIONS
        {
            bail!("Workbench process admission ledger is unsupported or exceeds its retention cap");
        }
        for (admission_id, admission) in &ledger.admissions {
            if admission_id != &admission.admission_id {
                bail!("Workbench process admission ledger key does not match its receipt");
            }
            admission.validate()?;
        }
        Ok(ledger)
    }

    fn save(&self, ledger: &WorkbenchProcessAdmissionLedger) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = self
            .path
            .with_extension(format!("json.tmp.{}", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec_pretty(ledger)?)?;
        std::fs::rename(&temporary, &self.path).with_context(|| {
            format!(
                "committing Workbench process admission ledger {} -> {}",
                temporary.display(),
                self.path.display()
            )
        })
    }
}
