//! Durable, one-shot bookkeeping authority for a future Codex probe attempt.
//!
//! Reserving an authority is deliberately not launch or execution authority. This module has
//! no executable, path, argument, environment, transport, helper invocation,
//! process, provider, renderer, or workspace-write surface. Its only active
//! transition is `available_no_process` to `reserved_no_process`.

// This foundation is intentionally private until a separately gated native
// composition point exists; its production API is exercised only by focused
// contract tests in this phase.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::capability_grant::{WorkbenchAuthorityTransaction, WorkbenchProcessGrantStore};
use super::codex_probe_preflight_digest::bounded_digest;
use super::codex_restricted_helper_preparation::{
    CodexHelperLaunchPreparationReceipt, CodexHelperLaunchRequest,
};
use super::events::{validate_identifier, WorkbenchSessionStatus};
use super::process_run_spec::{process_run_spec_digest, ProcessRunSpec};
use super::process_supervisor::WorkbenchProcessAdmissionStore;
use super::run_contract::{validate_workbench_run_plan, workbench_run_plan_snapshot_digest};
use super::session::validate_digest;
use super::storage::WorkbenchStore;
use super::{WorkbenchRunPlan, WorkbenchSession};

const AUTHORITY_SCHEMA_VERSION: u32 = 2;
const LEDGER_SCHEMA_VERSION: u32 = 2;
const AUTHORITY_LEDGER_FILE: &str = "workbench-codex-probe-attempt-authorities.json";
const ANCHOR_SCHEMA_VERSION: u32 = 2;
const AUTHORITY_SCOPE: &str = "claim_and_reserve_no_process";
const PROVIDER_TRAFFIC_NONE: &str = "none";
pub(super) const CODEX_PROBE_ATTEMPT_AUTHORITY_TTL_SECONDS: i64 = 60;
pub(super) const CODEX_PROBE_LAUNCH_RESERVATION_TTL_SECONDS: i64 = 10;
pub(super) const MAX_CODEX_PROBE_ATTEMPT_AUTHORITIES: usize = 128;

static CODEX_PROBE_ATTEMPT_AUTHORITY_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum CodexProbeAttemptState {
    AvailableNoProcess,
    ReservedNoProcess,
    AbandonedReservationRestart,
    LegacyClaimedNoReservation,
    AbandonedRestart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CodexProbeAttemptBinding {
    pub attempt_id: String,
    pub session_id: String,
    pub session_snapshot_digest: String,
    pub plan_id: String,
    pub plan_snapshot_digest: String,
    pub process_run_id: String,
    pub process_run_spec_digest: String,
    pub grant_id: String,
    pub grant_receipt_digest: String,
    pub admission_id: String,
    pub admission_receipt_digest: String,
    pub preparation_request_id: String,
    pub preparation_request_digest: String,
    pub preparation_receipt_id: String,
    pub preparation_receipt_digest: String,
    pub launch_binding_digest: String,
    pub authority_binding_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CodexProbeAttemptAuthority {
    pub schema_version: u32,
    pub authority_id: String,
    pub binding: CodexProbeAttemptBinding,
    pub owner_epoch: String,
    pub issued_at: String,
    pub expires_at: String,
    pub state: CodexProbeAttemptState,
    pub transition_at: Option<String>,
    pub claim_id: Option<String>,
    pub pre_reservation_record_digest: Option<String>,
    pub reservation_id: Option<String>,
    pub reservation_binding_digest: Option<String>,
    pub reservation_expires_at: Option<String>,
    pub reservation_closed_at: Option<String>,
    pub revision: u32,
    pub manual_opt_in_confirmed: bool,
    pub scope: String,
    pub helper_invoked: bool,
    pub process_started: bool,
    pub process_start_enabled: bool,
    pub launch_reserved: bool,
    pub execution_reserved: bool,
    pub execution_enabled: bool,
    pub runnable: bool,
    pub supported: bool,
    pub provider_traffic: String,
    pub user_workspace_writes_enabled: bool,
    pub record_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CodexProbeAttemptAuthorityLedger {
    schema_version: u32,
    ledger_id: String,
    generation: u64,
    owner_epoch: String,
    authorities: BTreeMap<String, CodexProbeAttemptAuthority>,
    ledger_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CodexProbeAttemptAuthorityAnchor {
    schema_version: u32,
    ledger_id: String,
    initialized_at: String,
    latest_generation: u64,
    latest_ledger_digest: String,
    anchor_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CodexProbeAttemptAuthorityAnchorV1 {
    schema_version: u32,
    ledger_id: String,
    initialized_at: String,
    anchor_digest: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CodexProbeAttemptStateV1 {
    AvailableNoProcess,
    ClaimedNoProcess,
    AbandonedRestart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CodexProbeAttemptAuthorityV1 {
    schema_version: u32,
    authority_id: String,
    binding: CodexProbeAttemptBinding,
    owner_epoch: String,
    issued_at: String,
    expires_at: String,
    state: CodexProbeAttemptStateV1,
    transition_at: Option<String>,
    claim_id: Option<String>,
    revision: u32,
    manual_opt_in_confirmed: bool,
    scope: String,
    helper_invoked: bool,
    process_started: bool,
    process_start_enabled: bool,
    execution_reserved: bool,
    execution_enabled: bool,
    runnable: bool,
    supported: bool,
    provider_traffic: String,
    user_workspace_writes_enabled: bool,
    record_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CodexProbeAttemptAuthorityLedgerV1 {
    schema_version: u32,
    ledger_id: String,
    owner_epoch: String,
    authorities: BTreeMap<String, CodexProbeAttemptAuthorityV1>,
    ledger_digest: String,
}

pub(super) struct CodexProbeAttemptContext<'a> {
    pub session: &'a WorkbenchSession,
    pub session_store: &'a WorkbenchStore,
    pub current_plan: &'a WorkbenchRunPlan,
    pub process: &'a ProcessRunSpec,
    pub grant_store: &'a WorkbenchProcessGrantStore,
    pub admission_store: &'a WorkbenchProcessAdmissionStore,
    pub request: &'a CodexHelperLaunchRequest,
    pub preparation_receipt: &'a CodexHelperLaunchPreparationReceipt,
}

pub(super) struct CodexProbeAttemptAuthorityStore {
    path: PathBuf,
    anchor_path: PathBuf,
    anchor: CodexProbeAttemptAuthorityAnchor,
    ledger: CodexProbeAttemptAuthorityLedger,
    persisted_bytes: Option<Vec<u8>>,
    persisted_anchor_bytes: Option<Vec<u8>>,
    reconciled_abandoned_count: usize,
}

type LoadedCodexProbeAttemptAuthorityLedger = (
    CodexProbeAttemptAuthorityAnchor,
    CodexProbeAttemptAuthorityLedger,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    bool,
);

impl CodexProbeAttemptBinding {
    fn from_context(context: &CodexProbeAttemptContext<'_>) -> Result<Self> {
        let request = context.request;
        let receipt = context.preparation_receipt;
        let mut binding = Self {
            attempt_id: request.binding.attempt_id.clone(),
            session_id: request.binding.session_id.clone(),
            session_snapshot_digest: request.binding.session_snapshot_digest.clone(),
            plan_id: request.binding.plan_id.clone(),
            plan_snapshot_digest: request.binding.plan_snapshot_digest.clone(),
            process_run_id: request.binding.process_run_id.clone(),
            process_run_spec_digest: request.binding.process_run_spec_digest.clone(),
            grant_id: request.binding.grant_id.clone(),
            grant_receipt_digest: request.binding.grant_receipt_digest.clone(),
            admission_id: request.binding.admission_id.clone(),
            admission_receipt_digest: request.binding.admission_receipt_digest.clone(),
            preparation_request_id: request.request_id.clone(),
            preparation_request_digest: request.request_digest.clone(),
            preparation_receipt_id: receipt.receipt_id.clone(),
            preparation_receipt_digest: receipt.receipt_digest.clone(),
            launch_binding_digest: request.binding.binding_digest.clone(),
            authority_binding_digest: String::new(),
        };
        binding.authority_binding_digest = authority_binding_digest(&binding);
        binding.validate()?;
        Ok(binding)
    }

    fn validate(&self) -> Result<()> {
        for (value, label) in [
            (&self.attempt_id, "Codex probe attempt ID"),
            (&self.session_id, "Codex probe session ID"),
            (&self.plan_id, "Codex probe plan ID"),
            (&self.process_run_id, "Codex probe process run ID"),
            (&self.grant_id, "Codex probe grant ID"),
            (&self.admission_id, "Codex probe admission ID"),
            (
                &self.preparation_request_id,
                "Codex probe preparation request ID",
            ),
            (
                &self.preparation_receipt_id,
                "Codex probe preparation receipt ID",
            ),
        ] {
            validate_identifier(value, label)?;
        }
        for (value, label) in [
            (&self.session_snapshot_digest, "session snapshot digest"),
            (&self.plan_snapshot_digest, "plan snapshot digest"),
            (&self.process_run_spec_digest, "process run spec digest"),
            (&self.grant_receipt_digest, "grant receipt digest"),
            (&self.admission_receipt_digest, "admission receipt digest"),
            (
                &self.preparation_request_digest,
                "preparation request digest",
            ),
            (
                &self.preparation_receipt_digest,
                "preparation receipt digest",
            ),
            (&self.launch_binding_digest, "launch binding digest"),
            (&self.authority_binding_digest, "authority binding digest"),
        ] {
            validate_digest(value, label)?;
        }
        if self.authority_binding_digest != authority_binding_digest(self) {
            bail!("Codex probe attempt authority binding digest does not match its content");
        }
        Ok(())
    }
}

impl CodexProbeAttemptAuthority {
    fn available(
        binding: CodexProbeAttemptBinding,
        owner_epoch: &str,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<Self> {
        let authority_id = authority_id_for(&binding);
        let mut authority = Self {
            schema_version: AUTHORITY_SCHEMA_VERSION,
            authority_id,
            binding,
            owner_epoch: owner_epoch.into(),
            issued_at: issued_at.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            state: CodexProbeAttemptState::AvailableNoProcess,
            transition_at: None,
            claim_id: None,
            pre_reservation_record_digest: None,
            reservation_id: None,
            reservation_binding_digest: None,
            reservation_expires_at: None,
            reservation_closed_at: None,
            revision: 0,
            manual_opt_in_confirmed: true,
            scope: AUTHORITY_SCOPE.into(),
            helper_invoked: false,
            process_started: false,
            process_start_enabled: false,
            launch_reserved: false,
            execution_reserved: false,
            execution_enabled: false,
            runnable: false,
            supported: false,
            provider_traffic: PROVIDER_TRAFFIC_NONE.into(),
            user_workspace_writes_enabled: false,
            record_digest: String::new(),
        };
        authority.refresh_digest()?;
        authority.validate()?;
        Ok(authority)
    }

    fn validate(&self) -> Result<()> {
        self.binding.validate()?;
        validate_identifier(&self.authority_id, "Codex probe attempt authority ID")?;
        validate_identifier(&self.owner_epoch, "Codex probe attempt owner epoch")?;
        validate_digest(&self.record_digest, "Codex probe attempt record digest")?;
        let issued_at = parse_time(&self.issued_at, "authority issue time")?;
        let expires_at = parse_time(&self.expires_at, "authority expiry time")?;
        if issued_at >= expires_at
            || self.schema_version != AUTHORITY_SCHEMA_VERSION
            || self.authority_id != authority_id_for(&self.binding)
            || !self.manual_opt_in_confirmed
            || self.scope != AUTHORITY_SCOPE
            || self.helper_invoked
            || self.process_started
            || self.process_start_enabled
            || self.execution_reserved
            || self.execution_enabled
            || self.runnable
            || self.supported
            || self.provider_traffic != PROVIDER_TRAFFIC_NONE
            || self.user_workspace_writes_enabled
        {
            bail!("Codex probe attempt authority violates the no-process boundary");
        }
        match self.state {
            CodexProbeAttemptState::AvailableNoProcess => {
                if self.revision != 0
                    || self.transition_at.is_some()
                    || self.claim_id.is_some()
                    || self.pre_reservation_record_digest.is_some()
                    || self.reservation_id.is_some()
                    || self.reservation_binding_digest.is_some()
                    || self.reservation_expires_at.is_some()
                    || self.reservation_closed_at.is_some()
                    || self.launch_reserved
                {
                    bail!("available Codex probe attempt authority has transition metadata");
                }
            }
            CodexProbeAttemptState::ReservedNoProcess => {
                if self.revision != 1
                    || self.transition_at.is_none()
                    || self.claim_id.is_none()
                    || self.pre_reservation_record_digest.is_none()
                    || self.reservation_id.is_none()
                    || self.reservation_binding_digest.is_none()
                    || self.reservation_expires_at.is_none()
                    || self.reservation_closed_at.is_some()
                    || !self.launch_reserved
                {
                    bail!("reserved Codex probe attempt authority lacks terminal metadata");
                }
            }
            CodexProbeAttemptState::AbandonedReservationRestart => {
                if self.revision != 2
                    || self.transition_at.is_none()
                    || self.claim_id.is_none()
                    || self.pre_reservation_record_digest.is_none()
                    || self.reservation_id.is_none()
                    || self.reservation_binding_digest.is_none()
                    || self.reservation_expires_at.is_none()
                    || self.reservation_closed_at.is_none()
                    || self.launch_reserved
                {
                    bail!("restart-abandoned Codex probe reservation has invalid metadata");
                }
            }
            CodexProbeAttemptState::LegacyClaimedNoReservation => {
                if self.revision != 1
                    || self.transition_at.is_none()
                    || self.claim_id.is_none()
                    || self.pre_reservation_record_digest.is_some()
                    || self.reservation_id.is_some()
                    || self.reservation_binding_digest.is_some()
                    || self.reservation_expires_at.is_some()
                    || self.reservation_closed_at.is_some()
                    || self.launch_reserved
                {
                    bail!("legacy claimed Codex probe authority has invalid terminal metadata");
                }
            }
            CodexProbeAttemptState::AbandonedRestart => {
                if self.revision != 1
                    || self.transition_at.is_none()
                    || self.claim_id.is_some()
                    || self.pre_reservation_record_digest.is_some()
                    || self.reservation_id.is_some()
                    || self.reservation_binding_digest.is_some()
                    || self.reservation_expires_at.is_some()
                    || self.reservation_closed_at.is_some()
                    || self.launch_reserved
                {
                    bail!("abandoned Codex probe attempt authority has invalid terminal metadata");
                }
            }
        }
        if let Some(transition_at) = &self.transition_at {
            let transition_at = parse_time(transition_at, "authority transition time")?;
            if transition_at < issued_at {
                bail!("Codex probe attempt transition predates authority issue");
            }
        }
        if let Some(closed_at) = &self.reservation_closed_at {
            let closed_at = parse_time(closed_at, "reservation close time")?;
            let transition_at = self
                .transition_at
                .as_deref()
                .ok_or_else(|| anyhow!("closed Codex probe reservation has no transition time"))?;
            if closed_at < parse_time(transition_at, "reservation transition time")? {
                bail!("Codex probe reservation closes before it was reserved");
            }
        }
        if let Some(claim_id) = &self.claim_id {
            validate_identifier(claim_id, "Codex probe attempt claim ID")?;
            let expected = claim_id_for(
                &self.authority_id,
                self.transition_at
                    .as_deref()
                    .ok_or_else(|| anyhow!("Codex probe attempt claim has no transition time"))?,
                &self.binding.authority_binding_digest,
            );
            if claim_id != &expected {
                bail!("Codex probe attempt claim ID does not match its transition");
            }
        }
        if matches!(
            self.state,
            CodexProbeAttemptState::ReservedNoProcess
                | CodexProbeAttemptState::AbandonedReservationRestart
        ) {
            let transition_at = self
                .transition_at
                .as_deref()
                .ok_or_else(|| anyhow!("Codex probe reservation has no transition time"))?;
            let claim_id = self
                .claim_id
                .as_deref()
                .ok_or_else(|| anyhow!("Codex probe reservation has no claim ID"))?;
            let reservation_id = self
                .reservation_id
                .as_deref()
                .ok_or_else(|| anyhow!("Codex probe reservation has no reservation ID"))?;
            let pre_reservation_record_digest = self
                .pre_reservation_record_digest
                .as_deref()
                .ok_or_else(|| anyhow!("Codex probe reservation has no pre-transition digest"))?;
            let reservation_binding_digest = self
                .reservation_binding_digest
                .as_deref()
                .ok_or_else(|| anyhow!("Codex probe reservation has no binding digest"))?;
            let reservation_expires_at = self
                .reservation_expires_at
                .as_deref()
                .ok_or_else(|| anyhow!("Codex probe reservation has no expiry"))?;
            validate_identifier(reservation_id, "Codex probe launch reservation ID")?;
            validate_digest(
                pre_reservation_record_digest,
                "Codex probe pre-reservation record digest",
            )?;
            validate_digest(
                reservation_binding_digest,
                "Codex probe launch reservation binding digest",
            )?;
            let transition_time = parse_time(transition_at, "reservation transition time")?;
            let reservation_expiry = parse_time(reservation_expires_at, "reservation expiry time")?;
            let maximum_reservation_expiry = transition_time
                .checked_add_signed(Duration::seconds(
                    CODEX_PROBE_LAUNCH_RESERVATION_TTL_SECONDS,
                ))
                .ok_or_else(|| anyhow!("Codex probe launch reservation validity overflows"))?;
            if reservation_expiry <= transition_time
                || reservation_expiry > expires_at
                || reservation_expiry > maximum_reservation_expiry
            {
                bail!("Codex probe launch reservation has an invalid validity window");
            }
            if pre_reservation_record_digest != self.reconstructed_available_record_digest()? {
                bail!("Codex probe launch reservation predecessor digest is not authoritative");
            }
            let expected_binding_digest = reservation_binding_digest_for(
                &self.authority_id,
                claim_id,
                transition_at,
                &self.owner_epoch,
                &self.binding.authority_binding_digest,
                pre_reservation_record_digest,
                reservation_expires_at,
            );
            if reservation_binding_digest != expected_binding_digest
                || reservation_id != reservation_id_for(reservation_binding_digest)
            {
                bail!("Codex probe launch reservation does not match its authority");
            }
        }
        if self.record_digest != authority_record_digest(self)? {
            bail!("Codex probe attempt authority record digest does not match its content");
        }
        Ok(())
    }

    fn refresh_digest(&mut self) -> Result<()> {
        self.record_digest = authority_record_digest(self)?;
        Ok(())
    }

    fn reconstructed_available_record_digest(&self) -> Result<String> {
        let mut predecessor = self.clone();
        predecessor.state = CodexProbeAttemptState::AvailableNoProcess;
        predecessor.transition_at = None;
        predecessor.claim_id = None;
        predecessor.pre_reservation_record_digest = None;
        predecessor.reservation_id = None;
        predecessor.reservation_binding_digest = None;
        predecessor.reservation_expires_at = None;
        predecessor.reservation_closed_at = None;
        predecessor.revision = 0;
        predecessor.launch_reserved = false;
        predecessor.record_digest = authority_record_digest(&predecessor)?;
        predecessor.validate()?;
        Ok(predecessor.record_digest)
    }

    fn claim_and_reserve(&mut self, now: DateTime<Utc>) -> Result<()> {
        if self.state != CodexProbeAttemptState::AvailableNoProcess || self.revision != 0 {
            bail!("Codex probe attempt authority is terminal and cannot be claimed again");
        }
        let issued_at = parse_time(&self.issued_at, "authority issue time")?;
        let expires_at = parse_time(&self.expires_at, "authority expiry time")?;
        if now < issued_at {
            bail!("Codex probe attempt claim time rolls back before authority issue");
        }
        if now >= expires_at {
            bail!("Codex probe attempt authority is expired");
        }
        let reservation_expires_at =
            (now + Duration::seconds(CODEX_PROBE_LAUNCH_RESERVATION_TTL_SECONDS)).min(expires_at);
        if reservation_expires_at <= now {
            bail!("Codex probe launch reservation has no remaining validity window");
        }
        let transition_at = now.to_rfc3339();
        let pre_reservation_record_digest = self.record_digest.clone();
        self.state = CodexProbeAttemptState::ReservedNoProcess;
        self.transition_at = Some(transition_at.clone());
        let claim_id = claim_id_for(
            &self.authority_id,
            &transition_at,
            &self.binding.authority_binding_digest,
        );
        let reservation_expires_at = reservation_expires_at.to_rfc3339();
        let reservation_binding_digest = reservation_binding_digest_for(
            &self.authority_id,
            &claim_id,
            &transition_at,
            &self.owner_epoch,
            &self.binding.authority_binding_digest,
            &pre_reservation_record_digest,
            &reservation_expires_at,
        );
        self.claim_id = Some(claim_id);
        self.pre_reservation_record_digest = Some(pre_reservation_record_digest);
        self.reservation_id = Some(reservation_id_for(&reservation_binding_digest));
        self.reservation_binding_digest = Some(reservation_binding_digest);
        self.reservation_expires_at = Some(reservation_expires_at);
        self.reservation_closed_at = None;
        self.revision = 1;
        self.launch_reserved = true;
        self.refresh_digest()?;
        self.validate()
    }

    fn abandon_for_restart(&mut self, now: DateTime<Utc>) -> Result<bool> {
        let issued_at = parse_time(&self.issued_at, "authority issue time")?;
        if now < issued_at {
            bail!("Codex probe restart time rolls back before authority issue");
        }
        match self.state {
            CodexProbeAttemptState::AvailableNoProcess => {
                self.state = CodexProbeAttemptState::AbandonedRestart;
                self.transition_at = Some(now.to_rfc3339());
                self.claim_id = None;
                self.revision = 1;
            }
            CodexProbeAttemptState::ReservedNoProcess => {
                let transition_at = parse_time(
                    self.transition_at
                        .as_deref()
                        .ok_or_else(|| anyhow!("Codex probe reservation has no transition time"))?,
                    "reservation transition time",
                )?;
                if now < transition_at {
                    bail!("Codex probe restart time rolls back before reservation");
                }
                self.state = CodexProbeAttemptState::AbandonedReservationRestart;
                self.reservation_closed_at = Some(now.to_rfc3339());
                self.launch_reserved = false;
                self.revision = 2;
            }
            CodexProbeAttemptState::AbandonedReservationRestart
            | CodexProbeAttemptState::LegacyClaimedNoReservation
            | CodexProbeAttemptState::AbandonedRestart => return Ok(false),
        }
        self.refresh_digest()?;
        self.validate()?;
        Ok(true)
    }
}

impl CodexProbeAttemptAuthorityLedger {
    fn empty(owner_epoch: &str, ledger_id: &str) -> Result<Self> {
        validate_identifier(owner_epoch, "Codex probe attempt owner epoch")?;
        validate_identifier(ledger_id, "Codex probe attempt ledger ID")?;
        let mut ledger = Self {
            schema_version: LEDGER_SCHEMA_VERSION,
            ledger_id: ledger_id.into(),
            generation: 0,
            owner_epoch: owner_epoch.into(),
            authorities: BTreeMap::new(),
            ledger_digest: String::new(),
        };
        ledger.refresh_digest()?;
        Ok(ledger)
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != LEDGER_SCHEMA_VERSION
            || self.authorities.len() > MAX_CODEX_PROBE_ATTEMPT_AUTHORITIES
        {
            bail!("Codex probe attempt authority ledger is unsupported or full");
        }
        validate_identifier(&self.ledger_id, "Codex probe attempt ledger ID")?;
        validate_identifier(&self.owner_epoch, "Codex probe attempt owner epoch")?;
        validate_digest(&self.ledger_digest, "Codex probe attempt ledger digest")?;
        let mut attempt_ids = BTreeSet::new();
        for (authority_id, authority) in &self.authorities {
            if authority_id != &authority.authority_id
                || !attempt_ids.insert(authority.binding.attempt_id.as_str())
            {
                bail!("Codex probe attempt authority ledger key or attempt is duplicated");
            }
            authority.validate()?;
            if authority.state == CodexProbeAttemptState::AvailableNoProcess
                && authority.owner_epoch != self.owner_epoch
            {
                bail!("available Codex probe authority belongs to another owner epoch");
            }
        }
        if self.ledger_digest != authority_ledger_digest(self)? {
            bail!("Codex probe attempt authority ledger digest does not match its content");
        }
        Ok(())
    }

    fn refresh_digest(&mut self) -> Result<()> {
        self.ledger_digest = authority_ledger_digest(self)?;
        Ok(())
    }
}

impl CodexProbeAttemptAuthorityV1 {
    fn validate(&self) -> Result<()> {
        self.binding.validate()?;
        validate_identifier(
            &self.authority_id,
            "legacy Codex probe attempt authority ID",
        )?;
        validate_identifier(&self.owner_epoch, "legacy Codex probe attempt owner epoch")?;
        validate_digest(
            &self.record_digest,
            "legacy Codex probe attempt record digest",
        )?;
        let issued_at = parse_time(&self.issued_at, "legacy authority issue time")?;
        let expires_at = parse_time(&self.expires_at, "legacy authority expiry time")?;
        if issued_at >= expires_at
            || self.schema_version != 1
            || self.authority_id != authority_id_for(&self.binding)
            || !self.manual_opt_in_confirmed
            || self.scope != "claim_only_no_process"
            || self.helper_invoked
            || self.process_started
            || self.process_start_enabled
            || self.execution_reserved
            || self.execution_enabled
            || self.runnable
            || self.supported
            || self.provider_traffic != PROVIDER_TRAFFIC_NONE
            || self.user_workspace_writes_enabled
        {
            bail!("legacy Codex probe authority violates the no-process boundary");
        }
        match self.state {
            CodexProbeAttemptStateV1::AvailableNoProcess => {
                if self.revision != 0 || self.transition_at.is_some() || self.claim_id.is_some() {
                    bail!("legacy available Codex probe authority has transition metadata");
                }
            }
            CodexProbeAttemptStateV1::ClaimedNoProcess => {
                if self.revision != 1 || self.transition_at.is_none() || self.claim_id.is_none() {
                    bail!("legacy claimed Codex probe authority lacks terminal metadata");
                }
            }
            CodexProbeAttemptStateV1::AbandonedRestart => {
                if self.revision != 1 || self.transition_at.is_none() || self.claim_id.is_some() {
                    bail!("legacy abandoned Codex probe authority has invalid terminal metadata");
                }
            }
        }
        if let Some(transition_at) = &self.transition_at {
            let transition_at = parse_time(transition_at, "legacy authority transition time")?;
            if transition_at < issued_at {
                bail!("legacy Codex probe authority transition predates issue");
            }
        }
        if let Some(claim_id) = &self.claim_id {
            validate_identifier(claim_id, "legacy Codex probe attempt claim ID")?;
            let expected = claim_id_for(
                &self.authority_id,
                self.transition_at.as_deref().ok_or_else(|| {
                    anyhow!("legacy Codex probe attempt claim has no transition time")
                })?,
                &self.binding.authority_binding_digest,
            );
            if claim_id != &expected {
                bail!("legacy Codex probe attempt claim ID does not match its transition");
            }
        }
        if self.record_digest != authority_record_digest_v1(self)? {
            bail!("legacy Codex probe authority record digest does not match its content");
        }
        Ok(())
    }

    fn into_current(self) -> Result<CodexProbeAttemptAuthority> {
        self.validate()?;
        let state = match self.state {
            CodexProbeAttemptStateV1::AvailableNoProcess => {
                CodexProbeAttemptState::AvailableNoProcess
            }
            CodexProbeAttemptStateV1::ClaimedNoProcess => {
                CodexProbeAttemptState::LegacyClaimedNoReservation
            }
            CodexProbeAttemptStateV1::AbandonedRestart => CodexProbeAttemptState::AbandonedRestart,
        };
        let mut current = CodexProbeAttemptAuthority {
            schema_version: AUTHORITY_SCHEMA_VERSION,
            authority_id: self.authority_id,
            binding: self.binding,
            owner_epoch: self.owner_epoch,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            state,
            transition_at: self.transition_at,
            claim_id: self.claim_id,
            pre_reservation_record_digest: None,
            reservation_id: None,
            reservation_binding_digest: None,
            reservation_expires_at: None,
            reservation_closed_at: None,
            revision: self.revision,
            manual_opt_in_confirmed: self.manual_opt_in_confirmed,
            scope: AUTHORITY_SCOPE.into(),
            helper_invoked: false,
            process_started: false,
            process_start_enabled: false,
            launch_reserved: false,
            execution_reserved: false,
            execution_enabled: false,
            runnable: false,
            supported: false,
            provider_traffic: PROVIDER_TRAFFIC_NONE.into(),
            user_workspace_writes_enabled: false,
            record_digest: String::new(),
        };
        current.refresh_digest()?;
        current.validate()?;
        Ok(current)
    }
}

impl CodexProbeAttemptAuthorityLedgerV1 {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.authorities.len() > MAX_CODEX_PROBE_ATTEMPT_AUTHORITIES
        {
            bail!("legacy Codex probe authority ledger is unsupported or full");
        }
        validate_identifier(&self.ledger_id, "legacy Codex probe attempt ledger ID")?;
        validate_identifier(&self.owner_epoch, "legacy Codex probe attempt owner epoch")?;
        validate_digest(
            &self.ledger_digest,
            "legacy Codex probe attempt ledger digest",
        )?;
        let mut attempt_ids = BTreeSet::new();
        for (authority_id, authority) in &self.authorities {
            if authority_id != &authority.authority_id
                || !attempt_ids.insert(authority.binding.attempt_id.as_str())
            {
                bail!("legacy Codex probe authority ledger key or attempt is duplicated");
            }
            authority.validate()?;
            if authority.state == CodexProbeAttemptStateV1::AvailableNoProcess
                && authority.owner_epoch != self.owner_epoch
            {
                bail!("legacy available Codex probe authority belongs to another owner epoch");
            }
        }
        if self.ledger_digest != authority_ledger_digest_v1(self)? {
            bail!("legacy Codex probe authority ledger digest does not match its content");
        }
        Ok(())
    }

    fn into_current(self) -> Result<CodexProbeAttemptAuthorityLedger> {
        self.validate()?;
        let mut authorities = BTreeMap::new();
        for (authority_id, authority) in self.authorities {
            authorities.insert(authority_id, authority.into_current()?);
        }
        let mut current = CodexProbeAttemptAuthorityLedger {
            schema_version: LEDGER_SCHEMA_VERSION,
            ledger_id: self.ledger_id,
            generation: 0,
            owner_epoch: self.owner_epoch,
            authorities,
            ledger_digest: String::new(),
        };
        current.refresh_digest()?;
        current.validate()?;
        Ok(current)
    }
}

impl CodexProbeAttemptAuthorityAnchor {
    fn new(now: DateTime<Utc>, ledger: &CodexProbeAttemptAuthorityLedger) -> Result<Self> {
        let mut anchor = Self {
            schema_version: ANCHOR_SCHEMA_VERSION,
            ledger_id: ledger.ledger_id.clone(),
            initialized_at: now.to_rfc3339(),
            latest_generation: ledger.generation,
            latest_ledger_digest: ledger.ledger_digest.clone(),
            anchor_digest: String::new(),
        };
        anchor.anchor_digest = authority_anchor_digest(&anchor)?;
        anchor.validate()?;
        Ok(anchor)
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != ANCHOR_SCHEMA_VERSION {
            bail!("Codex probe attempt authority anchor schema is unsupported");
        }
        validate_identifier(&self.ledger_id, "Codex probe attempt ledger ID")?;
        parse_time(&self.initialized_at, "anchor initialization time")?;
        validate_digest(
            &self.latest_ledger_digest,
            "Codex probe attempt anchor ledger digest",
        )?;
        validate_digest(&self.anchor_digest, "Codex probe attempt anchor digest")?;
        if self.anchor_digest != authority_anchor_digest(self)? {
            bail!("Codex probe attempt authority anchor digest does not match its content");
        }
        Ok(())
    }

    fn advanced_to(&self, ledger: &CodexProbeAttemptAuthorityLedger) -> Result<Self> {
        if self.ledger_id != ledger.ledger_id || ledger.generation <= self.latest_generation {
            bail!("Codex probe attempt anchor cannot advance to this ledger generation");
        }
        let mut next = Self {
            schema_version: ANCHOR_SCHEMA_VERSION,
            ledger_id: self.ledger_id.clone(),
            initialized_at: self.initialized_at.clone(),
            latest_generation: ledger.generation,
            latest_ledger_digest: ledger.ledger_digest.clone(),
            anchor_digest: String::new(),
        };
        next.anchor_digest = authority_anchor_digest(&next)?;
        next.validate()?;
        Ok(next)
    }
}

impl CodexProbeAttemptAuthorityAnchorV1 {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            bail!("legacy Codex probe attempt authority anchor schema is unsupported");
        }
        validate_identifier(&self.ledger_id, "legacy Codex probe attempt ledger ID")?;
        parse_time(&self.initialized_at, "legacy anchor initialization time")?;
        validate_digest(
            &self.anchor_digest,
            "legacy Codex probe attempt anchor digest",
        )?;
        if self.anchor_digest != authority_anchor_digest_v1(self)? {
            bail!("legacy Codex probe attempt anchor digest does not match its content");
        }
        Ok(())
    }

    fn into_current(
        self,
        ledger: &CodexProbeAttemptAuthorityLedger,
    ) -> Result<CodexProbeAttemptAuthorityAnchor> {
        self.validate()?;
        if self.ledger_id != ledger.ledger_id {
            bail!("legacy Codex probe attempt anchor does not match its ledger");
        }
        let mut current = CodexProbeAttemptAuthorityAnchor {
            schema_version: ANCHOR_SCHEMA_VERSION,
            ledger_id: self.ledger_id,
            initialized_at: self.initialized_at,
            latest_generation: ledger.generation,
            latest_ledger_digest: ledger.ledger_digest.clone(),
            anchor_digest: String::new(),
        };
        current.anchor_digest = authority_anchor_digest(&current)?;
        current.validate()?;
        Ok(current)
    }
}

impl CodexProbeAttemptAuthorityStore {
    #[allow(dead_code)]
    pub(super) fn in_app_storage(owner_epoch: &str, now: DateTime<Utc>) -> Result<Self> {
        Self::open(
            crate::storage::config_file(&crate::storage::app_data_dir(), AUTHORITY_LEDGER_FILE),
            owner_epoch,
            now,
        )
    }

    pub(super) fn open(path: PathBuf, owner_epoch: &str, now: DateTime<Utc>) -> Result<Self> {
        validate_identifier(owner_epoch, "Codex probe attempt owner epoch")?;
        let anchor_path = authority_anchor_path(&path);
        let (anchor, ledger, persisted_anchor_bytes, persisted_bytes, migration_required) =
            load_authority_ledger(&path, &anchor_path, owner_epoch, now)?;
        let mut store = Self {
            path,
            anchor_path,
            anchor,
            ledger,
            persisted_bytes,
            persisted_anchor_bytes,
            reconciled_abandoned_count: 0,
        };
        if migration_required {
            store.commit(store.ledger.clone())?;
        }
        store.reconcile_owner_epoch(owner_epoch, now)?;
        Ok(store)
    }

    pub(super) fn reconciled_abandoned_count(&self) -> usize {
        self.reconciled_abandoned_count
    }

    pub(super) fn issue(
        &mut self,
        context: CodexProbeAttemptContext<'_>,
        confirmation_phrase: &str,
        now: DateTime<Utc>,
    ) -> Result<CodexProbeAttemptAuthority> {
        let transaction = context.grant_store.begin_authority_transaction()?;
        transaction.require_authority_directory(self.authority_directory()?)?;
        self.ensure_current()?;
        if now < parse_time(&self.anchor.initialized_at, "anchor initialization time")? {
            bail!("Codex probe attempt issue time rolls back before ledger initialization");
        }
        let (binding, grant_expires_at) = validate_context(&context, &transaction, now)?;
        let expected_phrase = codex_probe_attempt_confirmation_phrase(&binding.attempt_id);
        if confirmation_phrase.len() > expected_phrase.len() + 8
            || confirmation_phrase != expected_phrase
        {
            bail!("Codex probe attempt confirmation phrase does not match the prepared attempt");
        }
        if let Some(existing) = self
            .ledger
            .authorities
            .values()
            .find(|authority| authority.binding.attempt_id == binding.attempt_id)
        {
            if existing.binding == binding
                && existing.state == CodexProbeAttemptState::AvailableNoProcess
                && existing.owner_epoch == self.ledger.owner_epoch
                && now >= parse_time(&existing.issued_at, "authority issue time")?
                && now < parse_time(&existing.expires_at, "authority expiry time")?
            {
                return Ok(existing.clone());
            }
            bail!("Codex probe attempt is already bound or terminal");
        }
        if self.ledger.authorities.len() >= MAX_CODEX_PROBE_ATTEMPT_AUTHORITIES {
            bail!("Codex probe attempt authority ledger is full");
        }
        let expires_at = (now + Duration::seconds(CODEX_PROBE_ATTEMPT_AUTHORITY_TTL_SECONDS))
            .min(grant_expires_at);
        if expires_at <= now {
            bail!("Codex probe attempt authority has no remaining validity window");
        }
        let authority = CodexProbeAttemptAuthority::available(
            binding,
            &self.ledger.owner_epoch,
            now,
            expires_at,
        )?;
        let mut next = self.ledger.clone();
        next.authorities
            .insert(authority.authority_id.clone(), authority.clone());
        self.commit(next)?;
        drop(transaction);
        Ok(authority)
    }

    pub(super) fn claim(
        &mut self,
        authority_id: &str,
        expected_record_digest: &str,
        context: CodexProbeAttemptContext<'_>,
        now: DateTime<Utc>,
    ) -> Result<CodexProbeAttemptAuthority> {
        let transaction = context.grant_store.begin_authority_transaction()?;
        transaction.require_authority_directory(self.authority_directory()?)?;
        self.ensure_current()?;
        validate_identifier(authority_id, "Codex probe attempt authority ID")?;
        validate_digest(expected_record_digest, "expected authority record digest")?;
        let (expected_binding, _) = validate_context(&context, &transaction, now)?;
        let mut next = self.ledger.clone();
        let authority = next
            .authorities
            .get_mut(authority_id)
            .ok_or_else(|| anyhow!("Codex probe attempt authority was not found"))?;
        if authority.binding != expected_binding
            || authority.owner_epoch != self.ledger.owner_epoch
            || authority.record_digest != expected_record_digest
        {
            bail!("Codex probe attempt authority changed or is bound to another context");
        }
        authority.claim_and_reserve(now)?;
        let claimed = authority.clone();
        self.commit(next)?;
        drop(transaction);
        Ok(claimed)
    }

    pub(super) fn get(&self, authority_id: &str) -> Result<CodexProbeAttemptAuthority> {
        self.ensure_current()?;
        validate_identifier(authority_id, "Codex probe attempt authority ID")?;
        self.ledger
            .authorities
            .get(authority_id)
            .cloned()
            .ok_or_else(|| anyhow!("Codex probe attempt authority was not found"))
    }

    fn authority_directory(&self) -> Result<&std::path::Path> {
        self.path
            .parent()
            .ok_or_else(|| anyhow!("Codex probe attempt authority ledger has no parent directory"))
    }

    fn ensure_current(&self) -> Result<()> {
        let _guard = CODEX_PROBE_ATTEMPT_AUTHORITY_LOCK
            .lock()
            .map_err(|_| anyhow!("Codex probe attempt authority lock is unavailable"))?;
        if self.persisted_bytes.is_none() && self.persisted_anchor_bytes.is_none() {
            if read_regular_file(&self.path, "authority ledger")?.is_some()
                || read_regular_file(&self.anchor_path, "authority anchor")?.is_some()
            {
                bail!("Codex probe attempt authority storage appeared after open; reopen first");
            }
            return Ok(());
        }
        let (anchor, persisted, anchor_bytes, persisted_bytes, migration_required) =
            load_authority_ledger(
                &self.path,
                &self.anchor_path,
                &self.ledger.owner_epoch,
                Utc::now(),
            )?;
        if anchor != self.anchor
            || migration_required
            || anchor_bytes != self.persisted_anchor_bytes
            || persisted.ledger_digest != self.ledger.ledger_digest
            || persisted_bytes != self.persisted_bytes
        {
            bail!("Codex probe attempt authority ledger changed; reopen before use");
        }
        Ok(())
    }

    fn reconcile_owner_epoch(&mut self, owner_epoch: &str, now: DateTime<Utc>) -> Result<()> {
        if self.ledger.owner_epoch == owner_epoch {
            return Ok(());
        }
        if now < parse_time(&self.anchor.initialized_at, "anchor initialization time")? {
            bail!("Codex probe owner epoch change rolls back before ledger initialization");
        }
        for authority in self.ledger.authorities.values() {
            let issued_at = parse_time(&authority.issued_at, "authority issue time")?;
            let transition_at = authority
                .transition_at
                .as_deref()
                .map(|value| parse_time(value, "authority transition time"))
                .transpose()?;
            if now < issued_at || transition_at.is_some_and(|transition_at| now < transition_at) {
                bail!("Codex probe owner epoch change rolls back durable authority time");
            }
        }
        let mut next = self.ledger.clone();
        next.owner_epoch = owner_epoch.into();
        let mut reconciled = 0;
        for authority in next.authorities.values_mut() {
            if authority.abandon_for_restart(now)? {
                reconciled += 1;
            }
        }
        self.commit(next)?;
        self.reconciled_abandoned_count = reconciled;
        Ok(())
    }

    fn commit(&mut self, mut ledger: CodexProbeAttemptAuthorityLedger) -> Result<()> {
        let _guard = CODEX_PROBE_ATTEMPT_AUTHORITY_LOCK
            .lock()
            .map_err(|_| anyhow!("Codex probe attempt authority lock is unavailable"))?;
        self.anchor.validate()?;
        self.ledger.validate()?;
        validate_anchor_ledger_pair(&self.anchor, &self.ledger)?;
        if ledger.ledger_id != self.ledger.ledger_id || ledger.generation != self.ledger.generation
        {
            bail!("Codex probe attempt ledger cannot skip or reuse a generation");
        }
        ledger.generation = ledger
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow!("Codex probe attempt ledger generation is exhausted"))?;
        ledger.refresh_digest()?;
        ledger.validate()?;
        let next_anchor = self.anchor.advanced_to(&ledger)?;
        validate_ledger_chronology(&self.anchor, &ledger)?;
        let replacement = serde_json::to_vec_pretty(&ledger)?;
        let anchor_replacement = serde_json::to_vec_pretty(&next_anchor)?;
        let anchor_result = match &self.persisted_anchor_bytes {
            Some(expected) => crate::managed_files::atomic_write_bytes_if_unchanged(
                &self.anchor_path,
                expected,
                &anchor_replacement,
            ),
            None => crate::managed_files::atomic_write_bytes_if_absent(
                &self.anchor_path,
                &anchor_replacement,
            ),
        };
        anchor_result.with_context(|| {
            format!(
                "advancing Codex probe attempt authority anchor {}",
                self.anchor_path.display()
            )
        })?;
        let result = match &self.persisted_bytes {
            Some(expected) => crate::managed_files::atomic_write_bytes_if_unchanged(
                &self.path,
                expected,
                &replacement,
            ),
            None => crate::managed_files::atomic_write_bytes_if_absent(&self.path, &replacement),
        };
        result.with_context(|| {
            format!(
                "committing Codex probe attempt authority ledger {}",
                self.path.display()
            )
        })?;
        self.persisted_anchor_bytes = Some(anchor_replacement);
        self.persisted_bytes = Some(replacement);
        self.anchor = next_anchor;
        self.ledger = ledger;
        Ok(())
    }
}

pub(super) fn codex_probe_attempt_confirmation_phrase(attempt_id: &str) -> String {
    format!("AUTHORIZE CODEX VERSION PROBE {attempt_id}")
}

fn validate_context(
    context: &CodexProbeAttemptContext<'_>,
    transaction: &WorkbenchAuthorityTransaction,
    now: DateTime<Utc>,
) -> Result<(CodexProbeAttemptBinding, DateTime<Utc>)> {
    context.session.validate()?;
    let durable_session = context
        .session_store
        .get_for_authority_transaction(transaction, &context.session.session_id)?;
    if durable_session != *context.session {
        bail!("Codex probe attempt session snapshot is not the durable current session");
    }
    validate_workbench_run_plan(context.current_plan)?;
    context.process.validate()?;
    if durable_session.status != WorkbenchSessionStatus::Active
        || context.current_plan.session_id != context.session.session_id
        || context.current_plan.workspace_digest != context.session.workspace_digest
        || context.current_plan.adapter_id != "codex"
        || context.current_plan.execution_mode != "plan_only"
        || context.current_plan.provider_traffic != PROVIDER_TRAFFIC_NONE
        || context.current_plan.writes_enabled
        || context.process.session_id != context.session.session_id
        || context.process.run_id != context.request.binding.process_run_id
    {
        bail!("Codex probe attempt context is not the current plan-only Codex context");
    }
    let grant = context.grant_store.require_active_for_transaction(
        transaction,
        &context.request.binding.grant_id,
        &context.session.session_id,
        &context.current_plan.plan_id,
        &context.process.run_id,
        now,
    )?;
    let admission = context.admission_store.require_exact_for(
        &context.request.binding.admission_id,
        &context.session.session_id,
        &context.current_plan.plan_id,
        &context.process.run_id,
        &grant.grant_id,
    )?;
    context.request.validate()?;
    context.preparation_receipt.validate_for(context.request)?;

    let session_snapshot = serde_json::to_string(&durable_session)?;
    let session_snapshot_digest = bounded_digest(
        b"ai-switchboard-codex-helper-session-snapshot-v1\0",
        &[session_snapshot.as_str()],
    );
    let plan_snapshot_digest = workbench_run_plan_snapshot_digest(context.current_plan)?;
    let process_digest = process_run_spec_digest(context.process)?;
    let request = context.request;
    let receipt = context.preparation_receipt;
    if request.binding.session_id != context.session.session_id
        || request.binding.session_snapshot_digest != session_snapshot_digest
        || request.binding.workspace_digest != context.session.workspace_digest
        || request.binding.plan_id != context.current_plan.plan_id
        || request.binding.plan_snapshot_digest != plan_snapshot_digest
        || request.binding.process_run_id != context.process.run_id
        || request.binding.process_run_spec_digest != process_digest
        || request.binding.grant_id != grant.grant_id
        || request.binding.grant_receipt_digest != grant.receipt_digest
        || request.binding.admission_id != admission.admission_id
        || request.binding.admission_receipt_digest != admission.receipt_digest
        || receipt.request_id != request.request_id
        || receipt.request_digest != request.request_digest
    {
        bail!("Codex probe attempt preparation is not bound to current durable evidence");
    }
    let grant_issued_at = parse_time(&grant.issued_at, "grant issue time")?;
    let grant_expires_at = parse_time(&grant.expires_at, "grant expiry time")?;
    let admitted_at = parse_time(&admission.admitted_at, "admission time")?;
    let prepared_at = parse_time(&receipt.prepared_at, "preparation time")?;
    if now < grant_issued_at || now < admitted_at || now < prepared_at {
        bail!("Codex probe attempt clock rolls back before durable evidence");
    }
    let binding = CodexProbeAttemptBinding::from_context(context)?;
    Ok((binding, grant_expires_at))
}

fn authority_binding_digest(binding: &CodexProbeAttemptBinding) -> String {
    bounded_digest(
        b"ai-switchboard-codex-probe-attempt-authority-binding-v1\0",
        &[
            binding.attempt_id.as_str(),
            binding.session_id.as_str(),
            binding.session_snapshot_digest.as_str(),
            binding.plan_id.as_str(),
            binding.plan_snapshot_digest.as_str(),
            binding.process_run_id.as_str(),
            binding.process_run_spec_digest.as_str(),
            binding.grant_id.as_str(),
            binding.grant_receipt_digest.as_str(),
            binding.admission_id.as_str(),
            binding.admission_receipt_digest.as_str(),
            binding.preparation_request_id.as_str(),
            binding.preparation_request_digest.as_str(),
            binding.preparation_receipt_id.as_str(),
            binding.preparation_receipt_digest.as_str(),
            binding.launch_binding_digest.as_str(),
        ],
    )
}

fn authority_id_for(binding: &CodexProbeAttemptBinding) -> String {
    format!(
        "codex-probe-authority:{}",
        binding
            .authority_binding_digest
            .trim_start_matches("sha256:")
    )
}

fn claim_id_for(authority_id: &str, transition_at: &str, binding_digest: &str) -> String {
    let digest = bounded_digest(
        b"ai-switchboard-codex-probe-attempt-claim-id-v1\0",
        &[authority_id, transition_at, binding_digest],
    );
    format!("codex-probe-claim:{}", digest.trim_start_matches("sha256:"))
}

fn reservation_binding_digest_for(
    authority_id: &str,
    claim_id: &str,
    transition_at: &str,
    owner_epoch: &str,
    authority_binding_digest: &str,
    pre_reservation_record_digest: &str,
    reservation_expires_at: &str,
) -> String {
    bounded_digest(
        b"ai-switchboard-codex-probe-launch-reservation-binding-v1\0",
        &[
            authority_id,
            claim_id,
            transition_at,
            owner_epoch,
            authority_binding_digest,
            pre_reservation_record_digest,
            reservation_expires_at,
        ],
    )
}

fn reservation_id_for(reservation_binding_digest: &str) -> String {
    format!(
        "codex-probe-reservation:{}",
        reservation_binding_digest.trim_start_matches("sha256:")
    )
}

fn authority_record_digest(authority: &CodexProbeAttemptAuthority) -> Result<String> {
    sha256_json(&serde_json::json!({
        "schemaVersion": authority.schema_version,
        "authorityId": &authority.authority_id,
        "binding": &authority.binding,
        "ownerEpoch": &authority.owner_epoch,
        "issuedAt": &authority.issued_at,
        "expiresAt": &authority.expires_at,
        "state": authority.state,
        "transitionAt": &authority.transition_at,
        "claimId": &authority.claim_id,
        "preReservationRecordDigest": &authority.pre_reservation_record_digest,
        "reservationId": &authority.reservation_id,
        "reservationBindingDigest": &authority.reservation_binding_digest,
        "reservationExpiresAt": &authority.reservation_expires_at,
        "reservationClosedAt": &authority.reservation_closed_at,
        "revision": authority.revision,
        "manualOptInConfirmed": authority.manual_opt_in_confirmed,
        "scope": &authority.scope,
        "helperInvoked": authority.helper_invoked,
        "processStarted": authority.process_started,
        "processStartEnabled": authority.process_start_enabled,
        "launchReserved": authority.launch_reserved,
        "executionReserved": authority.execution_reserved,
        "executionEnabled": authority.execution_enabled,
        "runnable": authority.runnable,
        "supported": authority.supported,
        "providerTraffic": &authority.provider_traffic,
        "userWorkspaceWritesEnabled": authority.user_workspace_writes_enabled,
    }))
}

fn authority_record_digest_v1(authority: &CodexProbeAttemptAuthorityV1) -> Result<String> {
    sha256_json(&serde_json::json!({
        "schemaVersion": authority.schema_version,
        "authorityId": &authority.authority_id,
        "binding": &authority.binding,
        "ownerEpoch": &authority.owner_epoch,
        "issuedAt": &authority.issued_at,
        "expiresAt": &authority.expires_at,
        "state": authority.state,
        "transitionAt": &authority.transition_at,
        "claimId": &authority.claim_id,
        "revision": authority.revision,
        "manualOptInConfirmed": authority.manual_opt_in_confirmed,
        "scope": &authority.scope,
        "helperInvoked": authority.helper_invoked,
        "processStarted": authority.process_started,
        "processStartEnabled": authority.process_start_enabled,
        "executionReserved": authority.execution_reserved,
        "executionEnabled": authority.execution_enabled,
        "runnable": authority.runnable,
        "supported": authority.supported,
        "providerTraffic": &authority.provider_traffic,
        "userWorkspaceWritesEnabled": authority.user_workspace_writes_enabled,
    }))
}

fn authority_ledger_digest(ledger: &CodexProbeAttemptAuthorityLedger) -> Result<String> {
    sha256_json(&serde_json::json!({
        "schemaVersion": ledger.schema_version,
        "ledgerId": &ledger.ledger_id,
        "generation": ledger.generation,
        "ownerEpoch": &ledger.owner_epoch,
        "authorities": &ledger.authorities,
    }))
}

fn authority_ledger_digest_v1(ledger: &CodexProbeAttemptAuthorityLedgerV1) -> Result<String> {
    sha256_json(&serde_json::json!({
        "schemaVersion": ledger.schema_version,
        "ledgerId": &ledger.ledger_id,
        "ownerEpoch": &ledger.owner_epoch,
        "authorities": &ledger.authorities,
    }))
}

fn authority_anchor_digest(anchor: &CodexProbeAttemptAuthorityAnchor) -> Result<String> {
    sha256_json(&serde_json::json!({
        "schemaVersion": anchor.schema_version,
        "ledgerId": &anchor.ledger_id,
        "initializedAt": &anchor.initialized_at,
        "latestGeneration": anchor.latest_generation,
        "latestLedgerDigest": &anchor.latest_ledger_digest,
    }))
}

fn authority_anchor_digest_v1(anchor: &CodexProbeAttemptAuthorityAnchorV1) -> Result<String> {
    sha256_json(&serde_json::json!({
        "schemaVersion": anchor.schema_version,
        "ledgerId": &anchor.ledger_id,
        "initializedAt": &anchor.initialized_at,
    }))
}

fn validate_anchor_ledger_pair(
    anchor: &CodexProbeAttemptAuthorityAnchor,
    ledger: &CodexProbeAttemptAuthorityLedger,
) -> Result<()> {
    if anchor.ledger_id != ledger.ledger_id
        || anchor.latest_generation != ledger.generation
        || anchor.latest_ledger_digest != ledger.ledger_digest
    {
        bail!("Codex probe attempt authority ledger is not the anchored latest generation");
    }
    Ok(())
}

fn validate_ledger_chronology(
    anchor: &CodexProbeAttemptAuthorityAnchor,
    ledger: &CodexProbeAttemptAuthorityLedger,
) -> Result<()> {
    let initialized_at = parse_time(&anchor.initialized_at, "anchor initialization time")?;
    for authority in ledger.authorities.values() {
        let issued_at = parse_time(&authority.issued_at, "authority issue time")?;
        let transition_at = authority
            .transition_at
            .as_deref()
            .map(|value| parse_time(value, "authority transition time"))
            .transpose()?;
        let closed_at = authority
            .reservation_closed_at
            .as_deref()
            .map(|value| parse_time(value, "reservation close time"))
            .transpose()?;
        if issued_at < initialized_at
            || transition_at.is_some_and(|transition_at| transition_at < initialized_at)
            || closed_at.is_some_and(|closed_at| closed_at < initialized_at)
        {
            bail!("Codex probe attempt authority predates ledger initialization");
        }
    }
    Ok(())
}

fn sha256_json(value: &serde_json::Value) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value)?)
    ))
}

fn parse_time(value: &str, label: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| anyhow!("Codex probe attempt {label} is invalid"))
}

#[cfg(test)]
pub(super) fn downgrade_codex_probe_authority_ledger_to_v1_for_test(
    path: &std::path::Path,
) -> Result<()> {
    let bytes = fs::read(path)?;
    let current: CodexProbeAttemptAuthorityLedger = serde_json::from_slice(&bytes)?;
    current.validate()?;
    let anchor_path = authority_anchor_path(path);
    let current_anchor: CodexProbeAttemptAuthorityAnchor =
        serde_json::from_slice(&fs::read(&anchor_path)?)?;
    current_anchor.validate()?;
    validate_anchor_ledger_pair(&current_anchor, &current)?;
    let mut authorities = BTreeMap::new();
    for (authority_id, authority) in current.authorities {
        let state = match authority.state {
            CodexProbeAttemptState::AvailableNoProcess => {
                CodexProbeAttemptStateV1::AvailableNoProcess
            }
            CodexProbeAttemptState::ReservedNoProcess
            | CodexProbeAttemptState::AbandonedReservationRestart
            | CodexProbeAttemptState::LegacyClaimedNoReservation => {
                CodexProbeAttemptStateV1::ClaimedNoProcess
            }
            CodexProbeAttemptState::AbandonedRestart => CodexProbeAttemptStateV1::AbandonedRestart,
        };
        let mut legacy = CodexProbeAttemptAuthorityV1 {
            schema_version: 1,
            authority_id: authority.authority_id,
            binding: authority.binding,
            owner_epoch: authority.owner_epoch,
            issued_at: authority.issued_at,
            expires_at: authority.expires_at,
            state,
            transition_at: authority.transition_at,
            claim_id: authority.claim_id,
            revision: authority.revision,
            manual_opt_in_confirmed: authority.manual_opt_in_confirmed,
            scope: "claim_only_no_process".into(),
            helper_invoked: false,
            process_started: false,
            process_start_enabled: false,
            execution_reserved: false,
            execution_enabled: false,
            runnable: false,
            supported: false,
            provider_traffic: PROVIDER_TRAFFIC_NONE.into(),
            user_workspace_writes_enabled: false,
            record_digest: String::new(),
        };
        legacy.record_digest = authority_record_digest_v1(&legacy)?;
        legacy.validate()?;
        authorities.insert(authority_id, legacy);
    }
    let mut legacy = CodexProbeAttemptAuthorityLedgerV1 {
        schema_version: 1,
        ledger_id: current.ledger_id,
        owner_epoch: current.owner_epoch,
        authorities,
        ledger_digest: String::new(),
    };
    legacy.ledger_digest = authority_ledger_digest_v1(&legacy)?;
    legacy.validate()?;
    let mut legacy_anchor = CodexProbeAttemptAuthorityAnchorV1 {
        schema_version: 1,
        ledger_id: current_anchor.ledger_id,
        initialized_at: current_anchor.initialized_at,
        anchor_digest: String::new(),
    };
    legacy_anchor.anchor_digest = authority_anchor_digest_v1(&legacy_anchor)?;
    legacy_anchor.validate()?;
    fs::write(anchor_path, serde_json::to_vec_pretty(&legacy_anchor)?)?;
    fs::write(path, serde_json::to_vec_pretty(&legacy)?)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn rewrite_codex_probe_reservation_with_self_consistent_tamper_for_test(
    path: &std::path::Path,
    tamper: &str,
) -> Result<()> {
    let anchor_path = authority_anchor_path(path);
    let mut ledger: CodexProbeAttemptAuthorityLedger = serde_json::from_slice(&fs::read(path)?)?;
    let authority = ledger
        .authorities
        .values_mut()
        .next()
        .ok_or_else(|| anyhow!("test reservation record is missing"))?;
    match tamper {
        "predecessor" => {
            authority.pre_reservation_record_digest = Some(format!("sha256:{}", "f".repeat(64)));
        }
        "ttl" => {
            let transition = parse_time(
                authority
                    .transition_at
                    .as_deref()
                    .ok_or_else(|| anyhow!("test reservation transition is missing"))?,
                "test reservation transition time",
            )?;
            authority.reservation_expires_at =
                Some((transition + Duration::seconds(11)).to_rfc3339());
        }
        _ => bail!("test reservation tamper is unsupported"),
    }
    let transition_at = authority
        .transition_at
        .as_deref()
        .ok_or_else(|| anyhow!("test reservation transition is missing"))?;
    let claim_id = authority
        .claim_id
        .as_deref()
        .ok_or_else(|| anyhow!("test reservation claim is missing"))?;
    let predecessor_digest = authority
        .pre_reservation_record_digest
        .as_deref()
        .ok_or_else(|| anyhow!("test reservation predecessor is missing"))?;
    let reservation_expiry = authority
        .reservation_expires_at
        .as_deref()
        .ok_or_else(|| anyhow!("test reservation expiry is missing"))?;
    let binding_digest = reservation_binding_digest_for(
        &authority.authority_id,
        claim_id,
        transition_at,
        &authority.owner_epoch,
        &authority.binding.authority_binding_digest,
        predecessor_digest,
        reservation_expiry,
    );
    authority.reservation_id = Some(reservation_id_for(&binding_digest));
    authority.reservation_binding_digest = Some(binding_digest);
    authority.record_digest = authority_record_digest(authority)?;
    ledger.ledger_digest = authority_ledger_digest(&ledger)?;

    let mut anchor: CodexProbeAttemptAuthorityAnchor =
        serde_json::from_slice(&fs::read(&anchor_path)?)?;
    anchor.latest_ledger_digest = ledger.ledger_digest.clone();
    anchor.anchor_digest = authority_anchor_digest(&anchor)?;
    fs::write(anchor_path, serde_json::to_vec_pretty(&anchor)?)?;
    fs::write(path, serde_json::to_vec_pretty(&ledger)?)?;
    Ok(())
}

fn load_authority_ledger(
    path: &std::path::Path,
    anchor_path: &std::path::Path,
    owner_epoch: &str,
    now: DateTime<Utc>,
) -> Result<LoadedCodexProbeAttemptAuthorityLedger> {
    let anchor_bytes = read_regular_file(anchor_path, "authority anchor")?;
    let ledger_bytes = read_regular_file(path, "authority ledger")?;
    match (anchor_bytes, ledger_bytes) {
        (None, None) => {
            let ledger_id = format!("codex-probe-ledger:{}", Uuid::new_v4());
            let ledger = CodexProbeAttemptAuthorityLedger::empty(owner_epoch, &ledger_id)?;
            let anchor = CodexProbeAttemptAuthorityAnchor::new(now, &ledger)?;
            Ok((anchor, ledger, None, None, false))
        }
        (Some(_), None) => {
            bail!("Codex probe attempt authority ledger was deleted while its anchor remains")
        }
        (None, Some(_)) => {
            bail!("Codex probe attempt authority anchor is missing for an existing ledger")
        }
        (Some(anchor_bytes), Some(ledger_bytes)) => {
            let schema_version = serde_json::from_slice::<serde_json::Value>(&ledger_bytes)
                .with_context(|| {
                    format!(
                        "decoding Codex probe attempt authority ledger envelope {}",
                        path.display()
                    )
                })?
                .get("schemaVersion")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| anyhow!("Codex probe attempt ledger schema version is missing"))?;
            let (ledger, ledger_migration_required) = match schema_version {
                1 => {
                    let legacy: CodexProbeAttemptAuthorityLedgerV1 =
                        serde_json::from_slice(&ledger_bytes).with_context(|| {
                            format!(
                                "decoding legacy Codex probe attempt authority ledger {}",
                                path.display()
                            )
                        })?;
                    (legacy.into_current()?, true)
                }
                value if value == u64::from(LEDGER_SCHEMA_VERSION) => {
                    let current: CodexProbeAttemptAuthorityLedger =
                        serde_json::from_slice(&ledger_bytes).with_context(|| {
                            format!(
                                "decoding Codex probe attempt authority ledger {}",
                                path.display()
                            )
                        })?;
                    (current, false)
                }
                _ => bail!("Codex probe attempt authority ledger schema is unsupported"),
            };
            let anchor_schema_version = serde_json::from_slice::<serde_json::Value>(&anchor_bytes)
                .with_context(|| {
                    format!(
                        "decoding Codex probe attempt authority anchor envelope {}",
                        anchor_path.display()
                    )
                })?
                .get("schemaVersion")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| anyhow!("Codex probe attempt anchor schema version is missing"))?;
            let (anchor, anchor_migration_required) = match anchor_schema_version {
                1 => {
                    let legacy: CodexProbeAttemptAuthorityAnchorV1 =
                        serde_json::from_slice(&anchor_bytes).with_context(|| {
                            format!(
                                "decoding legacy Codex probe attempt authority anchor {}",
                                anchor_path.display()
                            )
                        })?;
                    (legacy.into_current(&ledger)?, true)
                }
                value if value == u64::from(ANCHOR_SCHEMA_VERSION) => {
                    let current: CodexProbeAttemptAuthorityAnchor =
                        serde_json::from_slice(&anchor_bytes).with_context(|| {
                            format!(
                                "decoding Codex probe attempt authority anchor {}",
                                anchor_path.display()
                            )
                        })?;
                    (current, false)
                }
                _ => bail!("Codex probe attempt authority anchor schema is unsupported"),
            };
            anchor.validate()?;
            ledger.validate()?;
            validate_anchor_ledger_pair(&anchor, &ledger)?;
            validate_ledger_chronology(&anchor, &ledger)?;
            Ok((
                anchor,
                ledger,
                Some(anchor_bytes),
                Some(ledger_bytes),
                ledger_migration_required || anchor_migration_required,
            ))
        }
    }
}

fn authority_anchor_path(ledger_path: &std::path::Path) -> PathBuf {
    ledger_path.with_extension("anchor.json")
}

fn read_regular_file(path: &std::path::Path, label: &str) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "refusing symlinked Codex probe attempt {label} {}",
                path.display()
            )
        }
        Ok(metadata) if !metadata.file_type().is_file() => {
            bail!(
                "Codex probe attempt {label} is not a regular file {}",
                path.display()
            )
        }
        Ok(_) => fs::read(path)
            .with_context(|| format!("reading Codex probe attempt {label} {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error)
            .with_context(|| format!("inspecting Codex probe attempt {label} {}", path.display())),
    }
}
