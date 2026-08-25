//! Durable, content-free current-plan heads for the plan-only Workbench.
//!
//! A head is state, not launch authority. This module stores no path, prompt,
//! credential, provider payload, process identity, transport, or output. It has
//! no command surface and can be used only while the caller holds the canonical
//! Workbench authority transaction for the same storage directory.

#![cfg_attr(not(test), allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use switchboard_core::plan_head::plan_head_identity;
use uuid::Uuid;

use super::WorkbenchStore;
use crate::workbench_kernel::capability_grant::WorkbenchAuthorityTransaction;
use crate::workbench_kernel::events::{validate_identifier, WorkbenchSessionStatus};
use crate::workbench_kernel::run_contract::{
    validate_workbench_run_plan, workbench_run_plan_snapshot_digest,
};
use crate::workbench_kernel::session::{validate_digest, WorkbenchSession};
use crate::workbench_kernel::WorkbenchRunPlan;

const PLAN_HEAD_SCHEMA_VERSION: u32 = 1;
const PLAN_HEAD_LEDGER_SCHEMA_VERSION: u32 = 1;
const PLAN_HEAD_LEDGER_FILE: &str = "workbench-current-plan-heads.json";
const MAX_CURRENT_PLAN_HEADS: usize = 128;
const MAX_RETIRED_PLAN_HEADS: usize = 512;
const MAX_PLAN_SNAPSHOT_BYTES: usize = 256 * 1024;
const MAX_PLAN_HEAD_LEDGER_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::workbench_kernel) struct WorkbenchPlanHead {
    pub schema_version: u32,
    pub head_id: String,
    pub session_id: String,
    pub session_snapshot_digest: String,
    pub generation: u64,
    pub plan_id: String,
    pub plan_snapshot_digest: String,
    pub plan_snapshot_json: String,
    pub predecessor_head_id: Option<String>,
    pub predecessor_record_digest: Option<String>,
    pub execution_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
    pub record_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchPlanHeadCorrelationSummary {
    pub schema_version: u32,
    pub head_id: String,
    pub session_id: String,
    pub plan_id: String,
    pub generation: u64,
    pub session_snapshot_digest: String,
    pub plan_snapshot_digest: String,
    pub predecessor_head_id: Option<String>,
    pub predecessor_record_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RetiredWorkbenchPlanHead {
    head: WorkbenchPlanHead,
    superseded_by_head_id: String,
    tombstone_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkbenchPlanHeadLedger {
    schema_version: u32,
    ledger_id: String,
    generation: u64,
    current_heads: BTreeMap<String, WorkbenchPlanHead>,
    retired_heads: BTreeMap<String, RetiredWorkbenchPlanHead>,
    ledger_digest: String,
}

#[derive(Debug, Clone)]
pub(in crate::workbench_kernel) struct WorkbenchPlanHeadStore {
    path: PathBuf,
}

impl WorkbenchPlanHead {
    fn new(
        ledger_id: &str,
        generation: u64,
        session: &WorkbenchSession,
        plan: &WorkbenchRunPlan,
        plan_snapshot_json: String,
        plan_snapshot_digest: String,
        predecessor: Option<&WorkbenchPlanHead>,
    ) -> Result<Self> {
        let session_snapshot_digest = workbench_session_snapshot_digest(session)?;
        let predecessor_head_id = predecessor.map(|head| head.head_id.clone());
        let predecessor_record_digest = predecessor.map(|head| head.record_digest.clone());
        let head_id = plan_head_id_for(
            ledger_id,
            generation,
            &session.session_id,
            &session_snapshot_digest,
            &plan_snapshot_digest,
            predecessor_head_id.as_deref(),
            predecessor_record_digest.as_deref(),
        );
        let mut head = Self {
            schema_version: PLAN_HEAD_SCHEMA_VERSION,
            head_id,
            session_id: session.session_id.clone(),
            session_snapshot_digest,
            generation,
            plan_id: plan.plan_id.clone(),
            plan_snapshot_digest,
            plan_snapshot_json,
            predecessor_head_id,
            predecessor_record_digest,
            execution_enabled: false,
            provider_traffic: "none".into(),
            writes_enabled: false,
            record_digest: String::new(),
        };
        head.record_digest = plan_head_record_digest(&head)?;
        head.validate(ledger_id)?;
        Ok(head)
    }

    fn validate(&self, ledger_id: &str) -> Result<WorkbenchRunPlan> {
        if self.schema_version != PLAN_HEAD_SCHEMA_VERSION
            || self.generation == 0
            || self.execution_enabled
            || self.provider_traffic != "none"
            || self.writes_enabled
        {
            bail!("Workbench current-plan head violates its plan-only schema");
        }
        for (value, label) in [
            (self.head_id.as_str(), "plan head ID"),
            (self.session_id.as_str(), "plan head session ID"),
            (self.plan_id.as_str(), "plan head plan ID"),
        ] {
            validate_identifier(value, label)?;
        }
        validate_digest(
            &self.session_snapshot_digest,
            "plan head session snapshot digest",
        )?;
        validate_digest(&self.plan_snapshot_digest, "plan head snapshot digest")?;
        validate_digest(&self.record_digest, "plan head record digest")?;
        if self.plan_snapshot_json.len() > MAX_PLAN_SNAPSHOT_BYTES {
            bail!("Workbench current-plan snapshot exceeds its byte cap");
        }
        match (
            self.predecessor_head_id.as_deref(),
            self.predecessor_record_digest.as_deref(),
        ) {
            (None, None) => {}
            (Some(head_id), Some(record_digest)) => {
                validate_identifier(head_id, "predecessor plan head ID")?;
                validate_digest(record_digest, "predecessor plan head record digest")?;
            }
            _ => bail!("Workbench current-plan head has incomplete predecessor evidence"),
        }
        if self.head_id
            != plan_head_id_for(
                ledger_id,
                self.generation,
                &self.session_id,
                &self.session_snapshot_digest,
                &self.plan_snapshot_digest,
                self.predecessor_head_id.as_deref(),
                self.predecessor_record_digest.as_deref(),
            )
        {
            bail!("Workbench current-plan head ID does not match its full binding");
        }
        let plan: WorkbenchRunPlan = serde_json::from_str(&self.plan_snapshot_json)
            .context("decoding Workbench current-plan snapshot")?;
        validate_workbench_run_plan(&plan)?;
        let canonical = serde_json::to_string(&plan)
            .context("canonicalizing Workbench current-plan snapshot")?;
        if canonical != self.plan_snapshot_json
            || plan.session_id != self.session_id
            || plan.plan_id != self.plan_id
            || workbench_run_plan_snapshot_digest(&plan)? != self.plan_snapshot_digest
        {
            bail!("Workbench current-plan snapshot does not match its durable head");
        }
        if self.record_digest != plan_head_record_digest(self)? {
            bail!("Workbench current-plan head record digest does not match its content");
        }
        Ok(plan)
    }
}

impl RetiredWorkbenchPlanHead {
    fn new(head: WorkbenchPlanHead, superseded_by_head_id: &str) -> Result<Self> {
        validate_identifier(superseded_by_head_id, "superseding plan head ID")?;
        let mut tombstone = Self {
            head,
            superseded_by_head_id: superseded_by_head_id.into(),
            tombstone_digest: String::new(),
        };
        tombstone.tombstone_digest = plan_head_tombstone_digest(&tombstone)?;
        Ok(tombstone)
    }

    fn validate(&self, ledger_id: &str) -> Result<()> {
        self.head.validate(ledger_id)?;
        validate_identifier(&self.superseded_by_head_id, "superseding plan head ID")?;
        validate_digest(&self.tombstone_digest, "plan head tombstone digest")?;
        if self.tombstone_digest != plan_head_tombstone_digest(self)? {
            bail!("Workbench current-plan tombstone digest does not match its content");
        }
        Ok(())
    }
}

impl WorkbenchPlanHeadLedger {
    fn empty() -> Result<Self> {
        let mut ledger = Self {
            schema_version: PLAN_HEAD_LEDGER_SCHEMA_VERSION,
            ledger_id: format!("workbench-plan-head-ledger:{}", Uuid::new_v4()),
            generation: 0,
            current_heads: BTreeMap::new(),
            retired_heads: BTreeMap::new(),
            ledger_digest: String::new(),
        };
        ledger.refresh_digest()?;
        Ok(ledger)
    }

    fn refresh_digest(&mut self) -> Result<()> {
        self.ledger_digest = plan_head_ledger_digest(self)?;
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != PLAN_HEAD_LEDGER_SCHEMA_VERSION {
            bail!("Workbench current-plan ledger schema is unsupported");
        }
        validate_identifier(&self.ledger_id, "plan head ledger ID")?;
        validate_digest(&self.ledger_digest, "plan head ledger digest")?;
        if self.current_heads.len() > MAX_CURRENT_PLAN_HEADS
            || self.retired_heads.len() > MAX_RETIRED_PLAN_HEADS
        {
            bail!("Workbench current-plan ledger exceeds its retention cap");
        }
        let mut generations = BTreeSet::new();
        let mut records = BTreeMap::new();
        for (session_id, head) in &self.current_heads {
            if session_id != &head.session_id {
                bail!("Workbench current-plan ledger key does not match its session");
            }
            head.validate(&self.ledger_id)?;
            if !generations.insert(head.generation)
                || records.insert(head.head_id.as_str(), head).is_some()
            {
                bail!("Workbench current-plan ledger reuses a head identity or generation");
            }
        }
        for (head_id, tombstone) in &self.retired_heads {
            if head_id != &tombstone.head.head_id {
                bail!("Workbench retired plan-head key does not match its record");
            }
            tombstone.validate(&self.ledger_id)?;
            if !generations.insert(tombstone.head.generation)
                || records
                    .insert(tombstone.head.head_id.as_str(), &tombstone.head)
                    .is_some()
            {
                bail!("Workbench current-plan ledger reuses a head identity or generation");
            }
        }
        if self.generation != generations.len() as u64
            || generations.last().copied().unwrap_or(0) != self.generation
        {
            bail!("Workbench current-plan ledger generation history is incomplete");
        }
        for head in records.values() {
            if let (Some(predecessor_id), Some(predecessor_digest)) = (
                head.predecessor_head_id.as_deref(),
                head.predecessor_record_digest.as_deref(),
            ) {
                let predecessor = self
                    .retired_heads
                    .get(predecessor_id)
                    .ok_or_else(|| anyhow!("Workbench plan-head predecessor is not retired"))?;
                if predecessor.head.record_digest != predecessor_digest
                    || predecessor.head.session_id != head.session_id
                    || predecessor.head.generation >= head.generation
                    || predecessor.superseded_by_head_id != head.head_id
                {
                    bail!("Workbench current-plan predecessor chain is invalid");
                }
            }
        }
        for tombstone in self.retired_heads.values() {
            let successor = records
                .get(tombstone.superseded_by_head_id.as_str())
                .ok_or_else(|| anyhow!("Workbench retired plan head has no successor"))?;
            if successor.session_id != tombstone.head.session_id
                || successor.generation <= tombstone.head.generation
                || successor.predecessor_head_id.as_deref() != Some(tombstone.head.head_id.as_str())
                || successor.predecessor_record_digest.as_deref()
                    != Some(tombstone.head.record_digest.as_str())
            {
                bail!("Workbench retired plan-head successor is invalid");
            }
        }
        if self.ledger_digest != plan_head_ledger_digest(self)? {
            bail!("Workbench current-plan ledger digest does not match its content");
        }
        Ok(())
    }
}

impl WorkbenchPlanHeadStore {
    pub(in crate::workbench_kernel) fn in_app_storage() -> Self {
        Self {
            path: crate::storage::config_file(
                &crate::storage::app_data_dir(),
                PLAN_HEAD_LEDGER_FILE,
            ),
        }
    }

    pub(in crate::workbench_kernel) fn for_authority_directory(directory: &Path) -> Self {
        Self {
            path: directory.join(PLAN_HEAD_LEDGER_FILE),
        }
    }

    #[cfg(test)]
    pub(in crate::workbench_kernel) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub(in crate::workbench_kernel) fn publish_for_authority_transaction(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
        session_store: &WorkbenchStore,
        session: &WorkbenchSession,
        plan: &WorkbenchRunPlan,
    ) -> Result<WorkbenchPlanHead> {
        transaction.require_authority_directory(self.authority_directory()?)?;
        let durable_session =
            session_store.get_for_authority_transaction(transaction, &session.session_id)?;
        durable_session.validate()?;
        validate_workbench_run_plan(plan)?;
        if durable_session != *session
            || durable_session.status != WorkbenchSessionStatus::Active
            || plan.session_id != durable_session.session_id
            || plan.workspace_digest != durable_session.workspace_digest
        {
            bail!("Workbench current-plan publication does not match the durable active session");
        }
        let plan_snapshot_json =
            serde_json::to_string(plan).context("encoding Workbench current-plan snapshot")?;
        if plan_snapshot_json.len() > MAX_PLAN_SNAPSHOT_BYTES {
            bail!("Workbench current-plan snapshot exceeds its byte cap");
        }
        let plan_snapshot_digest = workbench_run_plan_snapshot_digest(plan)?;
        let session_snapshot_digest = workbench_session_snapshot_digest(&durable_session)?;
        let (mut ledger, expected_bytes) = self.load()?;
        if let Some(current) = ledger.current_heads.get(&durable_session.session_id) {
            if current.session_snapshot_digest == session_snapshot_digest
                && current.plan_snapshot_digest == plan_snapshot_digest
                && current.plan_snapshot_json == plan_snapshot_json
            {
                current.validate(&ledger.ledger_id)?;
                return Ok(current.clone());
            }
        }
        if !ledger
            .current_heads
            .contains_key(&durable_session.session_id)
            && ledger.current_heads.len() >= MAX_CURRENT_PLAN_HEADS
        {
            bail!("Workbench current-plan ledger is full");
        }
        if ledger
            .current_heads
            .contains_key(&durable_session.session_id)
            && ledger.retired_heads.len() >= MAX_RETIRED_PLAN_HEADS
        {
            bail!("Workbench retired plan-head ledger is full");
        }
        let predecessor = ledger
            .current_heads
            .get(&durable_session.session_id)
            .cloned();
        let generation = ledger
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow!("Workbench current-plan generation is exhausted"))?;
        let head = WorkbenchPlanHead::new(
            &ledger.ledger_id,
            generation,
            &durable_session,
            plan,
            plan_snapshot_json,
            plan_snapshot_digest,
            predecessor.as_ref(),
        )?;
        if let Some(predecessor) = predecessor {
            let predecessor_id = predecessor.head_id.clone();
            let tombstone = RetiredWorkbenchPlanHead::new(predecessor, &head.head_id)?;
            if ledger
                .retired_heads
                .insert(predecessor_id, tombstone)
                .is_some()
            {
                bail!("Workbench current-plan predecessor was already retired");
            }
        }
        ledger
            .current_heads
            .insert(durable_session.session_id.clone(), head.clone());
        ledger.generation = generation;
        ledger.refresh_digest()?;
        ledger.validate()?;
        self.save(&ledger, expected_bytes.as_deref())?;
        Ok(head)
    }

    pub(in crate::workbench_kernel) fn require_current_for_authority_transaction(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
        session_store: &WorkbenchStore,
        session: &WorkbenchSession,
        plan: &WorkbenchRunPlan,
    ) -> Result<WorkbenchPlanHead> {
        transaction.require_authority_directory(self.authority_directory()?)?;
        let durable_session =
            session_store.get_for_authority_transaction(transaction, &session.session_id)?;
        durable_session.validate()?;
        validate_workbench_run_plan(plan)?;
        if durable_session != *session || durable_session.status != WorkbenchSessionStatus::Active {
            bail!("Workbench current-plan lookup requires the durable active session");
        }
        let (ledger, _) = self.load()?;
        let head = ledger
            .current_heads
            .get(&durable_session.session_id)
            .cloned()
            .ok_or_else(|| anyhow!("Workbench current-plan head is missing"))?;
        let durable_plan = head.validate(&ledger.ledger_id)?;
        if durable_plan != *plan
            || head.session_snapshot_digest != workbench_session_snapshot_digest(&durable_session)?
        {
            bail!("Workbench supplied plan is not the durable current-plan head");
        }
        Ok(head)
    }

    pub(in crate::workbench_kernel) fn current_plan_head_correlation_summary_for_authority_transaction(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
        session_store: &WorkbenchStore,
        session: &WorkbenchSession,
        plan: &WorkbenchRunPlan,
    ) -> Result<WorkbenchPlanHeadCorrelationSummary> {
        let head = self.require_current_for_authority_transaction(
            transaction,
            session_store,
            session,
            plan,
        )?;
        Ok(WorkbenchPlanHeadCorrelationSummary {
            schema_version: PLAN_HEAD_SCHEMA_VERSION,
            head_id: head.head_id,
            session_id: head.session_id,
            plan_id: head.plan_id,
            generation: head.generation,
            session_snapshot_digest: head.session_snapshot_digest,
            plan_snapshot_digest: head.plan_snapshot_digest,
            predecessor_head_id: head.predecessor_head_id,
            predecessor_record_digest: head.predecessor_record_digest,
        })
    }

    fn authority_directory(&self) -> Result<&Path> {
        self.path
            .parent()
            .ok_or_else(|| anyhow!("Workbench current-plan ledger has no parent directory"))
    }

    fn load(&self) -> Result<(WorkbenchPlanHeadLedger, Option<Vec<u8>>)> {
        let Some(bytes) = read_regular_file(&self.path)? else {
            return Ok((WorkbenchPlanHeadLedger::empty()?, None));
        };
        if bytes.len() > MAX_PLAN_HEAD_LEDGER_BYTES {
            bail!("Workbench current-plan ledger exceeds its byte cap");
        }
        let ledger: WorkbenchPlanHeadLedger =
            serde_json::from_slice(&bytes).with_context(|| {
                format!(
                    "decoding Workbench current-plan ledger {}",
                    self.path.display()
                )
            })?;
        ledger.validate()?;
        Ok((ledger, Some(bytes)))
    }

    fn save(&self, ledger: &WorkbenchPlanHeadLedger, expected: Option<&[u8]>) -> Result<()> {
        let replacement =
            serde_json::to_vec_pretty(ledger).context("encoding Workbench current-plan ledger")?;
        if replacement.len() > MAX_PLAN_HEAD_LEDGER_BYTES {
            bail!("Workbench current-plan ledger exceeds its byte cap");
        }
        match expected {
            Some(expected) => crate::managed_files::atomic_write_bytes_if_unchanged(
                &self.path,
                expected,
                &replacement,
            ),
            None => crate::managed_files::atomic_write_bytes_if_absent(&self.path, &replacement),
        }
        .with_context(|| {
            format!(
                "committing Workbench current-plan ledger {}",
                self.path.display()
            )
        })
    }
}

fn read_regular_file(path: &Path) -> Result<Option<Vec<u8>>> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    }
    #[cfg(not(unix))]
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "refusing symlinked Workbench current-plan ledger {}",
                path.display()
            )
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "inspecting Workbench current-plan ledger {}",
                    path.display()
                )
            })
        }
    }
    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("opening Workbench current-plan ledger {}", path.display())
            })
        }
    };
    let metadata = file.metadata().with_context(|| {
        format!(
            "inspecting Workbench current-plan ledger {}",
            path.display()
        )
    })?;
    if !metadata.file_type().is_file() {
        bail!(
            "Workbench current-plan ledger is not a regular file {}",
            path.display()
        );
    }
    if metadata.len() > MAX_PLAN_HEAD_LEDGER_BYTES as u64 {
        bail!("Workbench current-plan ledger exceeds its byte cap");
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.by_ref()
        .take((MAX_PLAN_HEAD_LEDGER_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading Workbench current-plan ledger {}", path.display()))?;
    if bytes.len() > MAX_PLAN_HEAD_LEDGER_BYTES {
        bail!("Workbench current-plan ledger exceeds its byte cap");
    }
    Ok(Some(bytes))
}

fn workbench_session_snapshot_digest(session: &WorkbenchSession) -> Result<String> {
    session.validate()?;
    let bytes = serde_json::to_vec(session).context("canonicalizing Workbench session snapshot")?;
    Ok(domain_digest(
        b"ai-switchboard-workbench-session-plan-head-v1\0",
        &[&bytes],
    ))
}

fn plan_head_id_for(
    ledger_id: &str,
    generation: u64,
    session_id: &str,
    session_snapshot_digest: &str,
    plan_snapshot_digest: &str,
    predecessor_head_id: Option<&str>,
    predecessor_record_digest: Option<&str>,
) -> String {
    plan_head_identity(
        ledger_id,
        generation,
        session_id,
        session_snapshot_digest,
        plan_snapshot_digest,
        predecessor_head_id,
        predecessor_record_digest,
    )
}

fn plan_head_record_digest(head: &WorkbenchPlanHead) -> Result<String> {
    sha256_json(&serde_json::json!({
        "schemaVersion": head.schema_version,
        "headId": &head.head_id,
        "sessionId": &head.session_id,
        "sessionSnapshotDigest": &head.session_snapshot_digest,
        "generation": head.generation,
        "planId": &head.plan_id,
        "planSnapshotDigest": &head.plan_snapshot_digest,
        "planSnapshotJson": &head.plan_snapshot_json,
        "predecessorHeadId": &head.predecessor_head_id,
        "predecessorRecordDigest": &head.predecessor_record_digest,
        "executionEnabled": head.execution_enabled,
        "providerTraffic": &head.provider_traffic,
        "writesEnabled": head.writes_enabled,
    }))
}

fn plan_head_tombstone_digest(tombstone: &RetiredWorkbenchPlanHead) -> Result<String> {
    sha256_json(&serde_json::json!({
        "head": &tombstone.head,
        "supersededByHeadId": &tombstone.superseded_by_head_id,
    }))
}

fn plan_head_ledger_digest(ledger: &WorkbenchPlanHeadLedger) -> Result<String> {
    sha256_json(&serde_json::json!({
        "schemaVersion": ledger.schema_version,
        "ledgerId": &ledger.ledger_id,
        "generation": ledger.generation,
        "currentHeads": &ledger.current_heads,
        "retiredHeads": &ledger.retired_heads,
    }))
}

fn domain_digest(domain: &[u8], fields: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn sha256_json(value: &serde_json::Value) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value)?)
    ))
}

#[cfg(test)]
mod tests;
