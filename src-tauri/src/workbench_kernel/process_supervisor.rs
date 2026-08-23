//! Durable admission records for the future Workbench process supervisor.
//!
//! Admission is deliberately not launch. It records that an already verified,
//! canonical adapter has a live, plan-bound grant. No child, command, path,
//! argument, environment, prompt, credential, PID, output, provider request,
//! or workspace handle is accepted, stored, or created here.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::capability_grant::WorkbenchProcessStartGrant;
use super::events::{validate_identifier, WorkbenchSessionStatus};
use super::process_run_spec::ProcessRunSpec;
use super::{WorkbenchRunPlan, WorkbenchSession};

const ADMISSION_SCHEMA_VERSION: u32 = 1;
const ADMISSION_LEDGER_SCHEMA_VERSION: u32 = 1;
const ADMISSION_LEDGER_FILE: &str = "workbench-process-admissions.json";
const MAX_ADMISSIONS: usize = 128;
const AUTHORIZED_NOT_STARTED: &str = "authorized_not_started";

static WORKBENCH_PROCESS_ADMISSION_LOCK: Mutex<()> = Mutex::new(());

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
    session.validate()?;
    process.validate()?;
    grant.require_active_at(now)?;
    if session.status != WorkbenchSessionStatus::Active {
        bail!("Workbench process admission requires an active session");
    }
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
    pub(crate) fn validate(&self) -> Result<()> {
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

    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn issue(
        &self,
        admission: WorkbenchProcessAdmission,
    ) -> Result<WorkbenchProcessAdmission> {
        let _guard = WORKBENCH_PROCESS_ADMISSION_LOCK
            .lock()
            .map_err(|_| anyhow!("Workbench process admission ledger lock is unavailable"))?;
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
        let _guard = WORKBENCH_PROCESS_ADMISSION_LOCK
            .lock()
            .map_err(|_| anyhow!("Workbench process admission ledger lock is unavailable"))?;
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

#[cfg(test)]
mod tests {
    use super::{
        admission_digest, admit_process, WorkbenchProcessAdmission,
        WorkbenchProcessAdmissionLedger, WorkbenchProcessAdmissionStore, MAX_ADMISSIONS,
    };
    use crate::models::SwitchboardMode;
    use crate::workbench_kernel::capability_grant::{
        issue_process_start_grant, process_start_confirmation_phrase, process_start_grant_digest,
        WorkbenchProcessStartGrant,
    };
    use crate::workbench_kernel::events::WorkbenchSessionAction;
    use crate::workbench_kernel::process_run_spec::process_run_spec_for;
    use crate::workbench_kernel::session::{CreateWorkbenchSessionInput, WorkbenchSession};
    use crate::workbench_kernel::{CapabilityRequest, RouterDecisionReference, WorkbenchRunPlan};
    use chrono::{Duration, TimeZone, Utc};
    use std::sync::{Arc, Barrier};

    fn session() -> WorkbenchSession {
        WorkbenchSession::create(CreateWorkbenchSessionInput {
            workspace_digest: format!("sha256:{}", "a".repeat(64)),
            task_class: "coding".into(),
        })
        .expect("create session")
    }

    fn plan(session: &WorkbenchSession) -> WorkbenchRunPlan {
        let adapter_plan_id = "codex-1234567890ab".to_string();
        WorkbenchRunPlan {
            schema_version: 1,
            plan_id: "run-plan:1234567890abcdef1234567890abcdef".into(),
            session_id: session.session_id.clone(),
            adapter_id: "codex".into(),
            workspace_digest: session.workspace_digest.clone(),
            context_pack_digest: None,
            router_decision: RouterDecisionReference {
                decision_id: "routing-decision-test".into(),
                decision_stage: "observe".into(),
                routing_mode: "observe_only".into(),
                evidence_digest: format!("sha256:{}", "b".repeat(64)),
            },
            replay_reference: None,
            preset: None,
            requested_mode: SwitchboardMode::Off,
            adapter_plan_id: adapter_plan_id.clone(),
            adapter_action: "cleanup_managed_routing".into(),
            adapter_reversible: true,
            command_readiness: Some(crate::workbench_kernel::WorkbenchAdapterCommandReadiness {
                schema_version: 1,
                adapter_id: "codex".into(),
                adapter_contract_version: 1,
                adapter_plan_id: adapter_plan_id.clone(),
                logical_binary: "codex".into(),
                known_candidate_present: false,
                discovery_mode: "fixed_known_location_metadata_only".into(),
                cli_version_probe_state: "not_probed".into(),
                version_probe_reason: "deferred".into(),
                process_start_enabled: false,
                provider_traffic: "none".into(),
                writes_enabled: false,
            }),
            process_containment: Some(
                process_run_spec_for(
                    &session.session_id,
                    &adapter_plan_id,
                    "codex",
                    &session.workspace_digest,
                )
                .expect("process containment"),
            ),
            capability_requests: vec![CapabilityRequest {
                capability_id: "adapter_command_readiness".into(),
                scope: "session".into(),
                approval_state: "pending".into(),
                execution_enabled: false,
            }],
            execution_mode: "plan_only".into(),
            provider_traffic: "none".into(),
            writes_enabled: false,
        }
    }

    fn grant(
        session: &WorkbenchSession,
        plan: &WorkbenchRunPlan,
        now: chrono::DateTime<Utc>,
    ) -> WorkbenchProcessStartGrant {
        issue_process_start_grant(session, plan, &process_start_confirmation_phrase(plan), now)
            .expect("issue process grant")
    }

    fn admission_variant(
        base: &WorkbenchProcessAdmission,
        suffix: &str,
    ) -> WorkbenchProcessAdmission {
        let mut variant = base.clone();
        variant.admission_id = format!("process-admission:{suffix}");
        variant.grant_id = format!("process-grant:{suffix}");
        variant.receipt_digest = admission_digest(&variant).expect("digest admission variant");
        variant
    }

    #[test]
    fn active_admission_is_content_free_idempotent_and_durable() {
        let session = session();
        let plan = plan(&session);
        let process = plan
            .process_containment
            .as_ref()
            .expect("process containment");
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let admission = admit_process(&session, &plan, process, &grant(&session, &plan, now), now)
            .expect("admit active process");
        assert_eq!(admission.state, "authorized_not_started");
        assert!(!admission.execution_enabled);
        assert_eq!(admission.provider_traffic, "none");
        assert!(!admission.writes_enabled);

        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("admissions.json");
        let store = WorkbenchProcessAdmissionStore::at(path.clone());
        let first = store.issue(admission.clone()).expect("persist admission");
        let second = store.issue(admission).expect("deduplicate admission");
        assert_eq!(first, second);
        let reloaded = WorkbenchProcessAdmissionStore::at(path)
            .list_for_session(&session.session_id)
            .expect("reload admissions");
        assert_eq!(reloaded, vec![first]);
    }

    #[test]
    fn admission_rejects_non_active_sessions_even_with_a_preexisting_grant() {
        for action in [
            WorkbenchSessionAction::Pause,
            WorkbenchSessionAction::Cancel,
            WorkbenchSessionAction::Complete,
        ] {
            let mut session = session();
            let plan = plan(&session);
            let process = plan
                .process_containment
                .as_ref()
                .expect("process containment");
            let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
            let grant = grant(&session, &plan, now);
            session.transition(action).expect("transition session");
            assert!(admit_process(&session, &plan, process, &grant, now).is_err());
        }
    }

    #[test]
    fn admission_rejects_expired_revoked_corrupt_and_misbound_grants() {
        let session = session();
        let plan = plan(&session);
        let process = plan
            .process_containment
            .as_ref()
            .expect("process containment");
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let grant = grant(&session, &plan, now);

        assert!(admit_process(
            &session,
            &plan,
            process,
            &grant,
            now + Duration::seconds(901)
        )
        .is_err());
        assert!(
            admit_process(&session, &plan, process, &grant, now - Duration::seconds(1)).is_err()
        );

        let mut revoked = grant.clone();
        revoked.status = "revoked".into();
        revoked.revoked_at = Some((now + Duration::seconds(1)).to_rfc3339());
        revoked.receipt_digest =
            process_start_grant_digest(&revoked).expect("digest revoked grant");
        assert!(admit_process(
            &session,
            &plan,
            process,
            &revoked,
            now + Duration::seconds(2)
        )
        .is_err());

        let mut corrupt = grant.clone();
        corrupt.receipt_digest = format!("sha256:{}", "0".repeat(64));
        assert!(admit_process(&session, &plan, process, &corrupt, now).is_err());

        let valid_grant = grant.clone();
        let mut misbound = grant;
        misbound.plan_id = "run-plan:different".into();
        misbound.receipt_digest =
            process_start_grant_digest(&misbound).expect("digest misbound grant");
        assert!(admit_process(&session, &plan, process, &misbound, now).is_err());

        let mut unknown_adapter = plan.clone();
        unknown_adapter.adapter_id = "unknown_adapter".into();
        assert!(admit_process(&session, &unknown_adapter, process, &valid_grant, now).is_err());
    }

    #[test]
    fn admission_store_enforces_capacity_and_serializes_concurrent_issue() {
        let session = session();
        let plan = plan(&session);
        let process = plan
            .process_containment
            .as_ref()
            .expect("process containment");
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let base = admit_process(&session, &plan, process, &grant(&session, &plan, now), now)
            .expect("admit active process");

        let directory = tempfile::tempdir().expect("temporary directory");
        let concurrent_path = directory.path().join("concurrent.json");
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();
        for index in 0..2 {
            let path = concurrent_path.clone();
            let barrier = Arc::clone(&barrier);
            let admission = admission_variant(&base, &format!("parallel-{index}"));
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                WorkbenchProcessAdmissionStore::at(path).issue(admission)
            }));
        }
        barrier.wait();
        for handle in handles {
            handle
                .join()
                .expect("join concurrent admission")
                .expect("persist concurrent admission");
        }
        assert_eq!(
            WorkbenchProcessAdmissionStore::at(concurrent_path)
                .list_for_session(&session.session_id)
                .expect("list concurrent admissions")
                .len(),
            2
        );

        let full_path = directory.path().join("full.json");
        let full_store = WorkbenchProcessAdmissionStore::at(full_path);
        for index in 0..MAX_ADMISSIONS {
            full_store
                .issue(admission_variant(&base, &format!("capacity-{index}")))
                .expect("fill admission ledger");
        }
        assert!(full_store
            .issue(admission_variant(&base, "capacity-overflow"))
            .is_err());
    }

    #[test]
    fn persisted_admissions_and_ledger_reject_unknown_content_fields() {
        let session = session();
        let plan = plan(&session);
        let process = plan
            .process_containment
            .as_ref()
            .expect("process containment");
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let admission = admit_process(&session, &plan, process, &grant(&session, &plan, now), now)
            .expect("admit active process");
        for forbidden in ["prompt", "path", "credential", "argv", "output"] {
            let mut value = serde_json::to_value(&admission).expect("serialize admission");
            value[forbidden] = serde_json::json!("must not be persisted");
            assert!(
                serde_json::from_value::<WorkbenchProcessAdmission>(value).is_err(),
                "admission accepted forbidden field {forbidden}"
            );
        }
        let ledger = serde_json::json!({
            "schemaVersion": 1,
            "admissions": {},
            "workingDirectory": "/must/not/persist"
        });
        assert!(serde_json::from_value::<WorkbenchProcessAdmissionLedger>(ledger).is_err());
    }
}
