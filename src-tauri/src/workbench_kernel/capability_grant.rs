//! Expiry-bound, content-free authorization receipts for future executors.
//!
//! Grants record a user's explicit consent for a specific prepared Workbench
//! plan. They do not start a process, resolve a binary, read credentials, send
//! provider traffic, alter configuration, or make execution available.

use std::collections::BTreeMap;
use std::path::PathBuf;

#[cfg(unix)]
use std::fs::OpenOptions;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use switchboard_runtime::RuntimeClock;
use uuid::Uuid;

use super::events::{validate_identifier, WorkbenchSessionStatus};
use super::runtime_time::utc_from_runtime_clock;
use super::{WorkbenchRunPlan, WorkbenchSession};

pub(crate) const PROCESS_START_CAPABILITY_ID: &str = "adapter_process_start";
pub(crate) const PROCESS_START_GRANT_TTL_SECONDS: i64 = 15 * 60;
pub(crate) const PROCESS_START_GRANT_CONFIRMATION_PREFIX: &str = "AUTHORIZE FUTURE PROCESS";
const GRANT_SCHEMA_VERSION: u32 = 1;
const GRANT_LEDGER_SCHEMA_VERSION: u32 = 1;
const GRANT_LEDGER_FILE: &str = "workbench-process-grants.json";
const AUTHORITY_TRANSACTION_LOCK_FILE: &str = ".workbench-authority-transaction.lock";
const MAX_GRANTS: usize = 128;
const GRANTED: &str = "granted";
const EXPIRED: &str = "expired";
const REVOKED: &str = "revoked";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchProcessStartGrant {
    pub schema_version: u32,
    pub grant_id: String,
    pub session_id: String,
    pub plan_id: String,
    pub process_run_id: String,
    pub capability_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub status: String,
    pub revoked_at: Option<String>,
    pub execution_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchProcessStartGrantView {
    pub schema_version: u32,
    pub grant_id: String,
    pub session_id: String,
    pub plan_id: String,
    pub process_run_id: String,
    pub capability_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub effective_state: String,
    pub execution_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchProcessStartGrantPolicy {
    pub confirmation_template: String,
    pub ttl_seconds: i64,
    pub execution_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkbenchProcessGrantLedger {
    schema_version: u32,
    grants: BTreeMap<String, WorkbenchProcessStartGrant>,
}

impl Default for WorkbenchProcessGrantLedger {
    fn default() -> Self {
        Self {
            schema_version: GRANT_LEDGER_SCHEMA_VERSION,
            grants: BTreeMap::new(),
        }
    }
}

pub(crate) struct WorkbenchProcessGrantStore {
    path: PathBuf,
}

/// Cross-process serialization shared by grant mutation and one-shot attempt
/// claims. The lock carries no authority and stores no content.
pub(crate) struct WorkbenchAuthorityTransaction {
    authority_directory: PathBuf,
    #[cfg(unix)]
    file: std::fs::File,
    #[cfg(not(unix))]
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl WorkbenchAuthorityTransaction {
    pub(crate) fn require_authority_directory(&self, directory: &std::path::Path) -> Result<()> {
        let canonical = std::fs::canonicalize(directory).with_context(|| {
            format!(
                "resolving Workbench authority transaction directory {}",
                directory.display()
            )
        })?;
        if canonical != self.authority_directory {
            bail!("Workbench authority transaction belongs to another storage directory");
        }
        Ok(())
    }
}

#[cfg(not(unix))]
static WORKBENCH_AUTHORITY_TRANSACTION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(unix)]
impl Drop for WorkbenchAuthorityTransaction {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;

        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

pub(crate) fn process_start_confirmation_phrase(plan: &WorkbenchRunPlan) -> String {
    format!("{PROCESS_START_GRANT_CONFIRMATION_PREFIX} {}", plan.plan_id)
}

pub(crate) fn process_start_grant_policy() -> WorkbenchProcessStartGrantPolicy {
    WorkbenchProcessStartGrantPolicy {
        confirmation_template: format!("{PROCESS_START_GRANT_CONFIRMATION_PREFIX} {{planId}}"),
        ttl_seconds: PROCESS_START_GRANT_TTL_SECONDS,
        execution_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
    }
}

pub(crate) fn issue_process_start_grant(
    session: &WorkbenchSession,
    plan: &WorkbenchRunPlan,
    confirmation_phrase: &str,
    now: DateTime<Utc>,
) -> Result<WorkbenchProcessStartGrant> {
    if session.status != WorkbenchSessionStatus::Active {
        bail!("Workbench process authorization requires an active session");
    }
    if plan.session_id != session.session_id {
        bail!("Workbench process authorization plan belongs to another session");
    }
    if plan.workspace_digest != session.workspace_digest
        || plan.execution_mode != "plan_only"
        || plan.provider_traffic != "none"
        || plan.writes_enabled
    {
        bail!("Workbench process authorization requires a valid plan-only session plan");
    }
    if !plan.capability_requests.iter().any(|request| {
        request.capability_id == "adapter_command_readiness" && !request.execution_enabled
    }) {
        bail!("Workbench process authorization requires adapter command readiness");
    }
    let process = plan
        .process_containment
        .as_ref()
        .ok_or_else(|| anyhow!("Workbench process authorization requires native containment"))?;
    process.validate()?;
    if process.session_id != session.session_id
        || process.adapter_plan_id != plan.adapter_plan_id
        || process.start_authorization != "not_granted"
    {
        bail!("Workbench process authorization containment binding is invalid");
    }
    let expected_phrase = process_start_confirmation_phrase(plan);
    if confirmation_phrase.len() > expected_phrase.len() + 8
        || confirmation_phrase != expected_phrase
    {
        bail!("Workbench process authorization phrase does not match the prepared plan");
    }
    let expires_at = now + Duration::seconds(PROCESS_START_GRANT_TTL_SECONDS);
    let mut grant = WorkbenchProcessStartGrant {
        schema_version: GRANT_SCHEMA_VERSION,
        grant_id: format!("process-grant:{}", Uuid::new_v4()),
        session_id: session.session_id.clone(),
        plan_id: plan.plan_id.clone(),
        process_run_id: process.run_id.clone(),
        capability_id: PROCESS_START_CAPABILITY_ID.into(),
        issued_at: now.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
        status: GRANTED.into(),
        revoked_at: None,
        execution_enabled: false,
        provider_traffic: "none".into(),
        writes_enabled: false,
        receipt_digest: String::new(),
    };
    grant.receipt_digest = process_start_grant_digest(&grant)?;
    grant.validate()?;
    Ok(grant)
}

pub(crate) fn issue_process_start_grant_with_clock<C>(
    store: &WorkbenchProcessGrantStore,
    clock: &C,
    session: &WorkbenchSession,
    plan: &WorkbenchRunPlan,
    confirmation_phrase: &str,
) -> Result<WorkbenchProcessStartGrantView>
where
    C: RuntimeClock + ?Sized,
{
    let now = utc_from_runtime_clock(clock)?;
    let grant = issue_process_start_grant(session, plan, confirmation_phrase, now)?;
    store.issue(grant, now)
}

pub(crate) fn process_start_grant_digest(grant: &WorkbenchProcessStartGrant) -> Result<String> {
    switchboard_core::process_grant::process_start_grant_digest(&core_grant(grant))
}

fn core_grant(
    grant: &WorkbenchProcessStartGrant,
) -> switchboard_core::process_grant::WorkbenchProcessStartGrant {
    switchboard_core::process_grant::WorkbenchProcessStartGrant {
        schema_version: grant.schema_version,
        grant_id: grant.grant_id.clone(),
        session_id: grant.session_id.clone(),
        plan_id: grant.plan_id.clone(),
        process_run_id: grant.process_run_id.clone(),
        capability_id: grant.capability_id.clone(),
        issued_at: grant.issued_at.clone(),
        expires_at: grant.expires_at.clone(),
        status: grant.status.clone(),
        revoked_at: grant.revoked_at.clone(),
        execution_enabled: grant.execution_enabled,
        provider_traffic: grant.provider_traffic.clone(),
        writes_enabled: grant.writes_enabled,
        receipt_digest: grant.receipt_digest.clone(),
    }
}

impl WorkbenchProcessStartGrant {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.schema_version != GRANT_SCHEMA_VERSION {
            bail!("Workbench process grant schema is unsupported");
        }
        for (value, label) in [
            (&self.grant_id, "process grant ID"),
            (&self.session_id, "session ID"),
            (&self.plan_id, "plan ID"),
            (&self.process_run_id, "process run ID"),
        ] {
            validate_identifier(value, label)?;
        }
        if self.capability_id != PROCESS_START_CAPABILITY_ID
            || !matches!(self.status.as_str(), GRANTED | EXPIRED | REVOKED)
            || self.execution_enabled
            || self.provider_traffic != "none"
            || self.writes_enabled
        {
            bail!("Workbench process grant violates the non-executing boundary");
        }
        let issued_at = DateTime::parse_from_rfc3339(&self.issued_at)
            .map_err(|_| anyhow!("Workbench process grant issue time is invalid"))?
            .with_timezone(&Utc);
        let expires_at = DateTime::parse_from_rfc3339(&self.expires_at)
            .map_err(|_| anyhow!("Workbench process grant expiry time is invalid"))?
            .with_timezone(&Utc);
        if expires_at.signed_duration_since(issued_at)
            != Duration::seconds(PROCESS_START_GRANT_TTL_SECONDS)
        {
            bail!("Workbench process grant expiry must use the native fixed policy");
        }
        match (&self.status[..], self.revoked_at.as_deref()) {
            (GRANTED | EXPIRED, None) => {}
            (REVOKED, Some(revoked_at)) => {
                let revoked_at = DateTime::parse_from_rfc3339(revoked_at)
                    .map_err(|_| anyhow!("Workbench process grant revoke time is invalid"))?
                    .with_timezone(&Utc);
                if revoked_at < issued_at || revoked_at > expires_at {
                    bail!("Workbench process grant revoke time is outside its validity window");
                }
            }
            _ => bail!("Workbench process grant revoke state is invalid"),
        }
        if self.receipt_digest != process_start_grant_digest(self)? {
            bail!("Workbench process grant receipt digest does not match its content");
        }
        Ok(())
    }

    fn effective_state_at(&self, now: DateTime<Utc>) -> Result<&'static str> {
        let issued_at = DateTime::parse_from_rfc3339(&self.issued_at)
            .map_err(|_| anyhow!("Workbench process grant issue time is invalid"))?
            .with_timezone(&Utc);
        let expires_at = DateTime::parse_from_rfc3339(&self.expires_at)
            .map_err(|_| anyhow!("Workbench process grant expiry time is invalid"))?
            .with_timezone(&Utc);
        Ok(if self.status == REVOKED {
            REVOKED
        } else if self.status == EXPIRED || now < issued_at || now >= expires_at {
            EXPIRED
        } else {
            "active"
        })
    }

    pub(crate) fn require_active_at(&self, now: DateTime<Utc>) -> Result<()> {
        self.validate()?;
        if self.effective_state_at(now)? != "active" {
            bail!("Workbench process grant is not active");
        }
        Ok(())
    }

    fn view_at(&self, now: DateTime<Utc>) -> WorkbenchProcessStartGrantView {
        let effective_state = self.effective_state_at(now).unwrap_or(EXPIRED);
        WorkbenchProcessStartGrantView {
            schema_version: self.schema_version,
            grant_id: self.grant_id.clone(),
            session_id: self.session_id.clone(),
            plan_id: self.plan_id.clone(),
            process_run_id: self.process_run_id.clone(),
            capability_id: self.capability_id.clone(),
            issued_at: self.issued_at.clone(),
            expires_at: self.expires_at.clone(),
            effective_state: effective_state.into(),
            execution_enabled: false,
            provider_traffic: "none".into(),
            writes_enabled: false,
            receipt_digest: self.receipt_digest.clone(),
        }
    }
}

impl WorkbenchProcessGrantStore {
    pub(crate) fn in_app_storage() -> Self {
        Self {
            path: crate::storage::config_file(&crate::storage::app_data_dir(), GRANT_LEDGER_FILE),
        }
    }

    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn for_authority_directory(directory: &std::path::Path) -> Self {
        Self {
            path: directory.join(GRANT_LEDGER_FILE),
        }
    }

    fn authority_directory(&self) -> Result<&std::path::Path> {
        self.path
            .parent()
            .ok_or_else(|| anyhow!("Workbench process grant ledger has no parent directory"))
    }

    pub(crate) fn begin_authority_transaction(&self) -> Result<WorkbenchAuthorityTransaction> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| anyhow!("Workbench process grant ledger has no parent directory"))?;
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "creating Workbench authority transaction directory {}",
                parent.display()
            )
        })?;
        let authority_directory = std::fs::canonicalize(parent).with_context(|| {
            format!(
                "resolving Workbench authority transaction directory {}",
                parent.display()
            )
        })?;
        let lock_path = authority_directory.join(AUTHORITY_TRANSACTION_LOCK_FILE);

        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            use std::os::unix::fs::OpenOptionsExt;

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open(&lock_path)
                .with_context(|| {
                    format!(
                        "opening Workbench authority transaction lock {}",
                        lock_path.display()
                    )
                })?;
            if !file
                .metadata()
                .with_context(|| {
                    format!(
                        "inspecting Workbench authority transaction lock {}",
                        lock_path.display()
                    )
                })?
                .file_type()
                .is_file()
            {
                bail!(
                    "Workbench authority transaction lock is not a regular file {}",
                    lock_path.display()
                );
            }
            let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
            if result != 0 {
                return Err(std::io::Error::last_os_error()).with_context(|| {
                    format!(
                        "locking Workbench authority transaction {}",
                        lock_path.display()
                    )
                });
            }
            Ok(WorkbenchAuthorityTransaction {
                authority_directory,
                file,
            })
        }

        #[cfg(not(unix))]
        {
            let guard = WORKBENCH_AUTHORITY_TRANSACTION_LOCK
                .lock()
                .map_err(|_| anyhow!("Workbench authority transaction lock is unavailable"))?;
            Ok(WorkbenchAuthorityTransaction {
                authority_directory,
                _guard: guard,
            })
        }
    }

    pub(crate) fn list_for_session(
        &self,
        session_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<WorkbenchProcessStartGrantView>> {
        let _transaction = self.begin_authority_transaction()?;
        validate_identifier(session_id, "session ID")?;
        let mut ledger = self.load()?;
        if expire_stale_grants(&mut ledger.grants, now)? {
            self.save(&ledger)?;
        }
        let mut grants = ledger
            .grants
            .into_values()
            .filter(|grant| grant.session_id == session_id)
            .map(|grant| grant.view_at(now))
            .collect::<Vec<_>>();
        grants.sort_by(|left, right| right.issued_at.cmp(&left.issued_at));
        Ok(grants)
    }

    pub(crate) fn snapshot(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<WorkbenchProcessStartGrantView>> {
        let mut grants = self
            .load()?
            .grants
            .into_values()
            .map(|grant| grant.view_at(now))
            .collect::<Vec<_>>();
        grants.sort_by(|left, right| right.issued_at.cmp(&left.issued_at));
        Ok(grants)
    }

    pub(crate) fn issue(
        &self,
        grant: WorkbenchProcessStartGrant,
        now: DateTime<Utc>,
    ) -> Result<WorkbenchProcessStartGrantView> {
        let _transaction = self.begin_authority_transaction()?;
        grant.validate()?;
        let mut ledger = self.load()?;
        let changed = expire_stale_grants(&mut ledger.grants, now)?;
        if let Some(existing) = ledger
            .grants
            .values()
            .find(|existing| {
                existing.session_id == grant.session_id
                    && existing.plan_id == grant.plan_id
                    && existing.process_run_id == grant.process_run_id
                    && existing.capability_id == grant.capability_id
                    && existing.effective_state_at(now).unwrap_or(EXPIRED) == "active"
            })
            .cloned()
        {
            if changed {
                self.save(&ledger)?;
            }
            return Ok(existing.view_at(now));
        }
        trim_inactive_grants(&mut ledger.grants, now);
        if ledger.grants.len() >= MAX_GRANTS {
            bail!(
                "Workbench process grant ledger is full; expire or revoke an existing grant first"
            );
        }
        let view = grant.view_at(now);
        ledger.grants.insert(grant.grant_id.clone(), grant);
        self.save(&ledger)?;
        Ok(view)
    }

    pub(crate) fn revoke(
        &self,
        grant_id: &str,
        now: DateTime<Utc>,
    ) -> Result<WorkbenchProcessStartGrantView> {
        let _transaction = self.begin_authority_transaction()?;
        validate_identifier(grant_id, "process grant ID")?;
        let mut ledger = self.load()?;
        let changed = expire_stale_grants(&mut ledger.grants, now)?;
        let grant = ledger
            .grants
            .get_mut(grant_id)
            .ok_or_else(|| anyhow!("Workbench process grant was not found"))?;
        if grant.status == GRANTED {
            let issued_at = DateTime::parse_from_rfc3339(&grant.issued_at)
                .map_err(|_| anyhow!("Workbench process grant issue time is invalid"))?
                .with_timezone(&Utc);
            let expires_at = DateTime::parse_from_rfc3339(&grant.expires_at)
                .map_err(|_| anyhow!("Workbench process grant expiry time is invalid"))?
                .with_timezone(&Utc);
            let revoked_at = now.max(issued_at).min(expires_at);
            grant.status = REVOKED.into();
            grant.revoked_at = Some(revoked_at.to_rfc3339());
            grant.receipt_digest = process_start_grant_digest(grant)?;
            grant.validate()?;
            let view = grant.view_at(now);
            self.save(&ledger)?;
            return Ok(view);
        }
        let view = grant.view_at(now);
        if changed {
            self.save(&ledger)?;
        }
        Ok(view)
    }

    pub(crate) fn require_active_for(
        &self,
        grant_id: &str,
        session_id: &str,
        plan_id: &str,
        process_run_id: &str,
        now: DateTime<Utc>,
    ) -> Result<WorkbenchProcessStartGrant> {
        let transaction = self.begin_authority_transaction()?;
        self.require_active_for_transaction(
            &transaction,
            grant_id,
            session_id,
            plan_id,
            process_run_id,
            now,
        )
    }

    pub(crate) fn require_active_for_transaction(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
        grant_id: &str,
        session_id: &str,
        plan_id: &str,
        process_run_id: &str,
        now: DateTime<Utc>,
    ) -> Result<WorkbenchProcessStartGrant> {
        transaction.require_authority_directory(self.authority_directory()?)?;
        for (value, label) in [
            (grant_id, "process grant ID"),
            (session_id, "session ID"),
            (plan_id, "plan ID"),
            (process_run_id, "process run ID"),
        ] {
            validate_identifier(value, label)?;
        }
        let mut ledger = self.load()?;
        if expire_stale_grants(&mut ledger.grants, now)? {
            self.save(&ledger)?;
        }
        let grant = ledger
            .grants
            .get(grant_id)
            .cloned()
            .ok_or_else(|| anyhow!("Workbench process grant was not found"))?;
        if grant.session_id != session_id
            || grant.plan_id != plan_id
            || grant.process_run_id != process_run_id
            || grant.capability_id != PROCESS_START_CAPABILITY_ID
        {
            bail!("Workbench process grant is not active for this native plan");
        }
        grant
            .require_active_at(now)
            .map_err(|_| anyhow!("Workbench process grant is not active for this native plan"))?;
        Ok(grant)
    }

    pub(crate) fn require_current_for(
        &self,
        grant_id: &str,
        session_id: &str,
        plan_id: &str,
        process_run_id: &str,
    ) -> Result<WorkbenchProcessStartGrant> {
        let transaction = self.begin_authority_transaction()?;
        self.require_current_for_transaction(
            &transaction,
            grant_id,
            session_id,
            plan_id,
            process_run_id,
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn require_current_for_transaction(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
        grant_id: &str,
        session_id: &str,
        plan_id: &str,
        process_run_id: &str,
    ) -> Result<WorkbenchProcessStartGrant> {
        transaction.require_authority_directory(self.authority_directory()?)?;
        for (value, label) in [
            (grant_id, "process grant ID"),
            (session_id, "session ID"),
            (plan_id, "plan ID"),
            (process_run_id, "process run ID"),
        ] {
            validate_identifier(value, label)?;
        }
        let grant = self
            .load()?
            .grants
            .get(grant_id)
            .cloned()
            .ok_or_else(|| anyhow!("Workbench process grant was not found"))?;
        grant.validate()?;
        if grant.session_id != session_id
            || grant.plan_id != plan_id
            || grant.process_run_id != process_run_id
            || grant.capability_id != PROCESS_START_CAPABILITY_ID
        {
            bail!("Workbench process grant is not authoritative for this native plan");
        }
        Ok(grant)
    }

    pub(crate) fn revoke_for_terminal_session(
        &self,
        session_id: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let _transaction = self.begin_authority_transaction()?;
        validate_identifier(session_id, "session ID")?;
        let mut ledger = self.load()?;
        let mut changed = expire_stale_grants(&mut ledger.grants, now)?;
        for grant in ledger
            .grants
            .values_mut()
            .filter(|grant| grant.session_id == session_id && grant.status == GRANTED)
        {
            let issued_at = DateTime::parse_from_rfc3339(&grant.issued_at)
                .map_err(|_| anyhow!("Workbench process grant issue time is invalid"))?
                .with_timezone(&Utc);
            let expires_at = DateTime::parse_from_rfc3339(&grant.expires_at)
                .map_err(|_| anyhow!("Workbench process grant expiry time is invalid"))?
                .with_timezone(&Utc);
            let revoked_at = now.max(issued_at).min(expires_at);
            grant.status = REVOKED.into();
            grant.revoked_at = Some(revoked_at.to_rfc3339());
            grant.receipt_digest = process_start_grant_digest(grant)?;
            grant.validate()?;
            changed = true;
        }
        if changed {
            self.save(&ledger)?;
        }
        Ok(())
    }

    fn load(&self) -> Result<WorkbenchProcessGrantLedger> {
        if !self.path.exists() {
            return Ok(WorkbenchProcessGrantLedger::default());
        }
        let bytes = std::fs::read(&self.path).with_context(|| {
            format!(
                "reading Workbench process grant ledger {}",
                self.path.display()
            )
        })?;
        let ledger: WorkbenchProcessGrantLedger =
            serde_json::from_slice(&bytes).with_context(|| {
                format!(
                    "decoding Workbench process grant ledger {}",
                    self.path.display()
                )
            })?;
        if ledger.schema_version != GRANT_LEDGER_SCHEMA_VERSION || ledger.grants.len() > MAX_GRANTS
        {
            bail!("Workbench process grant ledger is unsupported or exceeds its retention cap");
        }
        for (grant_id, grant) in &ledger.grants {
            if grant_id != &grant.grant_id {
                bail!("Workbench process grant ledger key does not match its receipt");
            }
            grant.validate()?;
        }
        Ok(ledger)
    }

    fn save(&self, ledger: &WorkbenchProcessGrantLedger) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "creating Workbench process grant directory {}",
                    parent.display()
                )
            })?;
        }
        let temporary = self
            .path
            .with_extension(format!("json.tmp.{}", std::process::id()));
        let bytes =
            serde_json::to_vec_pretty(ledger).context("encoding Workbench process grant ledger")?;
        std::fs::write(&temporary, bytes).with_context(|| {
            format!(
                "writing Workbench process grant ledger {}",
                temporary.display()
            )
        })?;
        std::fs::rename(&temporary, &self.path).with_context(|| {
            format!(
                "committing Workbench process grant ledger {} -> {}",
                temporary.display(),
                self.path.display()
            )
        })
    }
}

fn trim_inactive_grants(
    grants: &mut BTreeMap<String, WorkbenchProcessStartGrant>,
    now: DateTime<Utc>,
) {
    if grants.len() < MAX_GRANTS {
        return;
    }
    let mut inactive = grants
        .iter()
        .filter(|(_, grant)| grant.view_at(now).effective_state != "active")
        .map(|(grant_id, grant)| (grant.issued_at.clone(), grant_id.clone()))
        .collect::<Vec<_>>();
    inactive.sort();
    for (_, grant_id) in inactive {
        if grants.len() < MAX_GRANTS {
            break;
        }
        grants.remove(&grant_id);
    }
}

fn expire_stale_grants(
    grants: &mut BTreeMap<String, WorkbenchProcessStartGrant>,
    now: DateTime<Utc>,
) -> Result<bool> {
    let mut changed = false;
    for grant in grants.values_mut() {
        let expires_at = DateTime::parse_from_rfc3339(&grant.expires_at)
            .map_err(|_| anyhow!("Workbench process grant expiry time is invalid"))?
            .with_timezone(&Utc);
        if grant.status == GRANTED && now >= expires_at {
            grant.status = EXPIRED.into();
            grant.receipt_digest = process_start_grant_digest(grant)?;
            grant.validate()?;
            changed = true;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::{
        issue_process_start_grant, issue_process_start_grant_with_clock,
        process_start_confirmation_phrase, WorkbenchProcessGrantLedger, WorkbenchProcessGrantStore,
        WorkbenchProcessStartGrant,
    };
    use crate::models::SwitchboardMode;
    use crate::workbench_kernel::events::WorkbenchSessionStatus;
    use crate::workbench_kernel::process_run_spec::process_run_spec_for;
    use crate::workbench_kernel::session::{CreateWorkbenchSessionInput, WorkbenchSession};
    use crate::workbench_kernel::{CapabilityRequest, RouterDecisionReference, WorkbenchRunPlan};
    use chrono::{Duration, TimeZone, Utc};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use switchboard_runtime::{RuntimeClock, RuntimeClockError};

    #[derive(Debug)]
    struct CountingClock {
        unix_millis: i64,
        calls: AtomicUsize,
    }

    impl CountingClock {
        fn new(unix_millis: i64) -> Self {
            Self {
                unix_millis,
                calls: AtomicUsize::new(0),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl RuntimeClock for CountingClock {
        fn unix_millis(&self) -> i64 {
            self.unix_millis
        }

        fn try_unix_millis(&self) -> Result<i64, RuntimeClockError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.unix_millis)
        }
    }

    #[derive(Debug, Default)]
    struct FailingClock {
        calls: AtomicUsize,
    }

    impl RuntimeClock for FailingClock {
        fn unix_millis(&self) -> i64 {
            0
        }

        fn try_unix_millis(&self) -> Result<i64, RuntimeClockError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(RuntimeClockError::Failed("injected grant clock failure"))
        }
    }

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

    #[test]
    fn grant_requires_active_session_and_exact_confirmation_phrase() {
        let mut session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        assert!(issue_process_start_grant(&session, &plan, "wrong", now).is_err());
        let grant = issue_process_start_grant(
            &session,
            &plan,
            &process_start_confirmation_phrase(&plan),
            now,
        )
        .expect("issue non-executing grant");
        assert!(!grant.execution_enabled);
        assert_eq!(grant.provider_traffic, "none");
        session.status = WorkbenchSessionStatus::Paused;
        assert!(issue_process_start_grant(
            &session,
            &plan,
            &process_start_confirmation_phrase(&plan),
            now,
        )
        .is_err());
    }

    #[test]
    fn runtime_clock_grant_issuance_uses_one_exact_timestamp() {
        let session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 25, 4, 5, 6).unwrap() + Duration::milliseconds(789);
        let clock = CountingClock::new(now.timestamp_millis());
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));

        let view = issue_process_start_grant_with_clock(
            &store,
            &clock,
            &session,
            &plan,
            &process_start_confirmation_phrase(&plan),
        )
        .expect("issue grant with runtime clock");

        assert_eq!(clock.calls(), 1);
        assert_eq!(view.issued_at, now.to_rfc3339());
        assert_eq!(
            view.expires_at,
            (now + Duration::seconds(super::PROCESS_START_GRANT_TTL_SECONDS)).to_rfc3339()
        );
        let persisted = store
            .list_for_session(&session.session_id, now)
            .expect("load persisted grant");
        assert_eq!(persisted, vec![view]);
    }

    #[test]
    fn runtime_clock_failures_and_invalid_millis_do_not_write_grant_ledger() {
        let session = session();
        let plan = plan(&session);
        let directory = tempfile::tempdir().expect("temporary directory");

        let failing_path = directory.path().join("failing-grants.json");
        let failing_store = WorkbenchProcessGrantStore::at(failing_path.clone());
        let failing_clock = FailingClock::default();
        let error = issue_process_start_grant_with_clock(
            &failing_store,
            &failing_clock,
            &session,
            &plan,
            &process_start_confirmation_phrase(&plan),
        )
        .expect_err("fallible runtime clock must deny grant issuance");
        assert!(error.to_string().contains("injected grant clock failure"));
        assert_eq!(failing_clock.calls.load(Ordering::SeqCst), 1);
        assert!(!failing_path.exists());

        for (index, unix_millis) in [-1, i64::MAX].into_iter().enumerate() {
            let path = directory
                .path()
                .join(format!("invalid-grants-{index}.json"));
            let store = WorkbenchProcessGrantStore::at(path.clone());
            let clock = CountingClock::new(unix_millis);
            assert!(issue_process_start_grant_with_clock(
                &store,
                &clock,
                &session,
                &plan,
                &process_start_confirmation_phrase(&plan),
            )
            .is_err());
            assert_eq!(clock.calls(), 1);
            assert!(!path.exists());
        }
    }

    #[test]
    fn grant_store_is_idempotent_for_active_plan_then_revocable_and_expiring() {
        let session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
        let issued = issue_process_start_grant(
            &session,
            &plan,
            &process_start_confirmation_phrase(&plan),
            now,
        )
        .expect("issue grant");
        let first = store.issue(issued, now).expect("persist grant");
        let second = store
            .issue(
                issue_process_start_grant(
                    &session,
                    &plan,
                    &process_start_confirmation_phrase(&plan),
                    now + Duration::seconds(1),
                )
                .expect("reissue same plan"),
                now + Duration::seconds(1),
            )
            .expect("deduplicate active grant");
        assert_eq!(first.grant_id, second.grant_id);
        assert_eq!(first.effective_state, "active");
        let revoked = store
            .revoke(&first.grant_id, now + Duration::seconds(10))
            .expect("revoke grant");
        assert_eq!(revoked.effective_state, "revoked");
        let listed = store
            .list_for_session(&session.session_id, now + Duration::seconds(20))
            .expect("list grants");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].effective_state, "revoked");
    }

    #[test]
    fn receipt_integrity_and_expiry_state_fail_closed() {
        let session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let mut grant = issue_process_start_grant(
            &session,
            &plan,
            &process_start_confirmation_phrase(&plan),
            now,
        )
        .expect("issue grant");
        grant.execution_enabled = true;
        assert!(grant.validate().is_err());
        let fresh = issue_process_start_grant(
            &session,
            &plan,
            &process_start_confirmation_phrase(&plan),
            now,
        )
        .expect("issue fresh grant");
        fresh
            .require_active_at(now)
            .expect("grant is active at issue time");
        assert!(fresh.require_active_at(now - Duration::seconds(1)).is_err());
        assert!(fresh
            .require_active_at(now + Duration::seconds(901))
            .is_err());
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("grants.json");
        let store = WorkbenchProcessGrantStore::at(path.clone());
        store.issue(fresh, now).expect("persist grant");
        let before_rollback = std::fs::read(&path).expect("read grant ledger before rollback");
        assert_eq!(
            store
                .list_for_session(&session.session_id, now - Duration::seconds(1))
                .expect("clock rollback fails closed")[0]
                .effective_state,
            "expired"
        );
        assert_eq!(
            std::fs::read(&path).expect("read grant ledger after rollback"),
            before_rollback,
            "clock rollback must deny without persisting ordinary expiry"
        );
        assert_eq!(
            store
                .list_for_session(&session.session_id, now + Duration::seconds(901))
                .expect("list expired grant")[0]
                .effective_state,
            "expired"
        );
    }

    #[test]
    fn persisted_grants_and_ledger_reject_unknown_content_fields() {
        let session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let grant = issue_process_start_grant(
            &session,
            &plan,
            &process_start_confirmation_phrase(&plan),
            now,
        )
        .expect("issue grant");
        for forbidden in ["prompt", "path", "credential", "argv", "output"] {
            let mut value = serde_json::to_value(&grant).expect("serialize grant");
            value[forbidden] = serde_json::json!("must not be persisted");
            assert!(
                serde_json::from_value::<WorkbenchProcessStartGrant>(value).is_err(),
                "grant accepted forbidden field {forbidden}"
            );
        }
        let ledger = serde_json::json!({
            "schemaVersion": 1,
            "grants": {},
            "environment": {"SECRET": "must not be persisted"}
        });
        assert!(serde_json::from_value::<WorkbenchProcessGrantLedger>(ledger).is_err());
    }

    #[test]
    fn terminal_session_revocation_retires_active_grants() {
        let session = session();
        let plan = plan(&session);
        let now = Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap();
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = WorkbenchProcessGrantStore::at(directory.path().join("grants.json"));
        store
            .issue(
                issue_process_start_grant(
                    &session,
                    &plan,
                    &process_start_confirmation_phrase(&plan),
                    now,
                )
                .expect("issue grant"),
                now,
            )
            .expect("persist grant");

        store
            .revoke_for_terminal_session(&session.session_id, now + Duration::seconds(1))
            .expect("revoke terminal-session grants");

        assert_eq!(
            store
                .list_for_session(&session.session_id, now + Duration::seconds(2))
                .expect("list grant")[0]
                .effective_state,
            "revoked"
        );
    }
}
