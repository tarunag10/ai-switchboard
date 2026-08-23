use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};

use super::super::capability_grant::{WorkbenchProcessGrantStore, WorkbenchProcessStartGrant};
use super::super::events::validate_identifier;
use super::super::{ProcessRunSpec, WorkbenchProcessAdmission, WorkbenchSession};
use super::receipt::{
    validate_bindings, FakeProcessState, FakeTerminalOutcome, WorkbenchFakeProcessReceipt,
};
use super::registry::{load_registry, WorkbenchFakeProcessRegistry, MAX_FAKE_RUNS};

static WORKBENCH_FAKE_PROCESS_CONTROLLER_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct WorkbenchFakeProcessController {
    pub(super) path: PathBuf,
    pub(super) registry: WorkbenchFakeProcessRegistry,
    pub(super) persisted_bytes: Option<Vec<u8>>,
    pub(super) reconciled_orphan_count: usize,
}

impl WorkbenchFakeProcessController {
    pub(crate) fn open(path: PathBuf, owner_epoch: &str) -> Result<Self> {
        validate_identifier(owner_epoch, "fake process owner epoch")?;
        let (registry, persisted_bytes) = load_registry(&path, owner_epoch)?;
        let mut controller = Self {
            path,
            registry,
            persisted_bytes,
            reconciled_orphan_count: 0,
        };
        controller.reconcile_owner_epoch(owner_epoch)?;
        Ok(controller)
    }

    pub(crate) fn reconciled_orphan_count(&self) -> usize {
        self.reconciled_orphan_count
    }

    pub(crate) fn register(
        &mut self,
        session: &WorkbenchSession,
        process: &ProcessRunSpec,
        admission: &WorkbenchProcessAdmission,
    ) -> Result<WorkbenchFakeProcessReceipt> {
        self.ensure_current()?;
        if let Some(retired_binding) = self.registry.retired_runs.get(&process.run_id) {
            let proposed = WorkbenchFakeProcessReceipt::from_bindings(
                session,
                process,
                admission,
                self.registry.next_registered_sequence,
            )?;
            if retired_binding == &proposed.binding_digest {
                bail!("Workbench fake process run is terminal and cannot be registered again");
            }
            bail!("Workbench fake process run ID was retired for another admission");
        }
        if let Some(existing) = self.registry.runs.get(&process.run_id) {
            let proposed = WorkbenchFakeProcessReceipt::from_bindings(
                session,
                process,
                admission,
                existing.registered_sequence,
            )?;
            if existing.binding_digest == proposed.binding_digest {
                return Ok(existing.clone());
            }
            bail!("Workbench fake process run ID is already bound to another admission");
        }
        let mut next = self.registry.clone();
        if next.runs.len() >= MAX_FAKE_RUNS {
            let (terminal_run_id, retired_binding) = next
                .runs
                .iter()
                .filter(|(_, receipt)| receipt.state.is_terminal())
                .min_by_key(|(run_id, receipt)| (receipt.registered_sequence, *run_id))
                .map(|(run_id, receipt)| (run_id.clone(), receipt.binding_digest.clone()))
                .ok_or_else(|| anyhow!("Workbench fake process registry is full"))?;
            if next.retired_runs.len() >= super::registry::MAX_RETIRED_FAKE_RUNS {
                bail!("Workbench fake process retired-run history is full");
            }
            next.runs.remove(&terminal_run_id);
            next.retired_runs.insert(terminal_run_id, retired_binding);
        }
        let registered_sequence = next.next_registered_sequence;
        next.next_registered_sequence = next
            .next_registered_sequence
            .checked_add(1)
            .ok_or_else(|| anyhow!("Workbench fake process registration sequence overflow"))?;
        let proposed = WorkbenchFakeProcessReceipt::from_bindings(
            session,
            process,
            admission,
            registered_sequence,
        )?;
        next.runs.insert(process.run_id.clone(), proposed.clone());
        self.commit(next)?;
        Ok(proposed)
    }

    pub(crate) fn start(
        &mut self,
        session: &WorkbenchSession,
        process: &ProcessRunSpec,
        admission: &WorkbenchProcessAdmission,
        grant_store: &WorkbenchProcessGrantStore,
        now: DateTime<Utc>,
    ) -> Result<WorkbenchFakeProcessReceipt> {
        let grant = grant_store.require_active_for(
            &admission.grant_id,
            &session.session_id,
            &admission.plan_id,
            &process.run_id,
            now,
        )?;
        validate_start_authorization(session, process, admission, &grant, now)?;
        let registered = self.receipt(&process.run_id)?;
        let proposed = WorkbenchFakeProcessReceipt::from_bindings(
            session,
            process,
            admission,
            registered.registered_sequence,
        )?;
        if registered.binding_digest != proposed.binding_digest {
            bail!("Workbench fake process start bindings changed after registration");
        }
        self.mutate(&process.run_id, |receipt| match receipt.state {
            FakeProcessState::Authorized => {
                receipt.transition_to(FakeProcessState::Starting, None)?;
                Ok(true)
            }
            FakeProcessState::Starting | FakeProcessState::Running => Ok(false),
            FakeProcessState::Stopping => {
                bail!("Workbench fake process cannot restart while stopping")
            }
            state if state.is_terminal() => {
                bail!("Workbench fake process cannot restart after a terminal outcome")
            }
            _ => unreachable!("all fake process states are covered"),
        })
    }

    pub(crate) fn mark_running(&mut self, run_id: &str) -> Result<WorkbenchFakeProcessReceipt> {
        self.mutate(run_id, |receipt| match receipt.state {
            FakeProcessState::Starting => {
                receipt.transition_to(FakeProcessState::Running, None)?;
                Ok(true)
            }
            FakeProcessState::Running => Ok(false),
            _ => bail!("Workbench fake process can run only after starting"),
        })
    }

    pub(crate) fn observe_stream_bytes(
        &mut self,
        run_id: &str,
        bytes: &[u8],
    ) -> Result<WorkbenchFakeProcessReceipt> {
        self.mutate(run_id, |receipt| {
            if !matches!(
                receipt.state,
                FakeProcessState::Starting | FakeProcessState::Running | FakeProcessState::Stopping
            ) {
                bail!("Workbench fake process stream metadata requires an active lifecycle");
            }
            if !receipt.stream_metadata.observe(bytes)? {
                return Ok(false);
            }
            receipt.bump_revision()?;
            receipt.refresh_digest()?;
            receipt.validate()?;
            Ok(true)
        })
    }

    pub(crate) fn stop(&mut self, run_id: &str) -> Result<WorkbenchFakeProcessReceipt> {
        self.mutate(run_id, |receipt| match receipt.state {
            FakeProcessState::Authorized => {
                receipt.transition_to(
                    FakeProcessState::Cancelled,
                    Some(FakeTerminalOutcome::Cancelled),
                )?;
                Ok(true)
            }
            FakeProcessState::Starting | FakeProcessState::Running => {
                receipt.transition_to(FakeProcessState::Stopping, None)?;
                Ok(true)
            }
            FakeProcessState::Stopping => Ok(false),
            state if state.is_terminal() => Ok(false),
            _ => unreachable!("all fake process states are covered"),
        })
    }

    pub(crate) fn finalize(
        &mut self,
        run_id: &str,
        outcome: FakeTerminalOutcome,
    ) -> Result<WorkbenchFakeProcessReceipt> {
        if outcome == FakeTerminalOutcome::Orphaned {
            bail!("Workbench fake process orphan outcomes are restart-owned");
        }
        self.mutate(run_id, |receipt| {
            if receipt.state.is_terminal() {
                if receipt.terminal_outcome == Some(outcome) {
                    return Ok(false);
                }
                bail!("Workbench fake process already has a different terminal outcome");
            }
            if receipt.state == FakeProcessState::Authorized {
                bail!("Workbench fake process cannot finalize before starting");
            }
            receipt.transition_to(outcome.state(), Some(outcome))?;
            Ok(true)
        })
    }

    pub(crate) fn receipt(&self, run_id: &str) -> Result<WorkbenchFakeProcessReceipt> {
        self.ensure_current()?;
        validate_identifier(run_id, "process run ID")?;
        self.registry
            .runs
            .get(run_id)
            .cloned()
            .ok_or_else(|| anyhow!("Workbench fake process run is not registered"))
    }

    fn mutate<F>(&mut self, run_id: &str, mutation: F) -> Result<WorkbenchFakeProcessReceipt>
    where
        F: FnOnce(&mut WorkbenchFakeProcessReceipt) -> Result<bool>,
    {
        self.ensure_current()?;
        validate_identifier(run_id, "process run ID")?;
        let mut next = self.registry.clone();
        let receipt = next
            .runs
            .get_mut(run_id)
            .ok_or_else(|| anyhow!("Workbench fake process run is not registered"))?;
        let changed = mutation(receipt)?;
        let result = receipt.clone();
        if changed {
            self.commit(next)?;
        }
        Ok(result)
    }

    fn ensure_current(&self) -> Result<()> {
        let _guard = WORKBENCH_FAKE_PROCESS_CONTROLLER_LOCK
            .lock()
            .map_err(|_| anyhow!("Workbench fake process controller lock is unavailable"))?;
        let (persisted, persisted_bytes) = load_registry(&self.path, &self.registry.owner_epoch)?;
        if persisted.registry_digest != self.registry.registry_digest
            || persisted_bytes != self.persisted_bytes
        {
            bail!("Workbench fake process registry changed after it was opened; reopen before use");
        }
        Ok(())
    }

    fn reconcile_owner_epoch(&mut self, owner_epoch: &str) -> Result<()> {
        if self.registry.owner_epoch == owner_epoch {
            return Ok(());
        }
        let mut next = self.registry.clone();
        next.owner_epoch = owner_epoch.into();
        let mut reconciled = 0;
        for receipt in next.runs.values_mut() {
            if receipt.state.is_active_across_restart() {
                receipt.transition_to(
                    FakeProcessState::Orphaned,
                    Some(FakeTerminalOutcome::Orphaned),
                )?;
                reconciled += 1;
            }
        }
        self.commit(next)?;
        self.reconciled_orphan_count = reconciled;
        Ok(())
    }

    fn commit(&mut self, mut registry: WorkbenchFakeProcessRegistry) -> Result<()> {
        let _guard = WORKBENCH_FAKE_PROCESS_CONTROLLER_LOCK
            .lock()
            .map_err(|_| anyhow!("Workbench fake process controller lock is unavailable"))?;
        registry.refresh_digest()?;
        registry.validate()?;
        let replacement = serde_json::to_vec_pretty(&registry)?;
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
                "committing Workbench fake process registry {}",
                self.path.display()
            )
        })?;
        self.persisted_bytes = Some(replacement);
        self.registry = registry;
        Ok(())
    }
}

fn validate_start_authorization(
    session: &WorkbenchSession,
    process: &ProcessRunSpec,
    admission: &WorkbenchProcessAdmission,
    grant: &WorkbenchProcessStartGrant,
    now: DateTime<Utc>,
) -> Result<()> {
    validate_bindings(session, process, admission)?;
    grant.require_active_at(now)?;
    if grant.grant_id != admission.grant_id
        || grant.session_id != session.session_id
        || grant.plan_id != admission.plan_id
        || grant.process_run_id != process.run_id
        || grant.execution_enabled
        || grant.provider_traffic != "none"
        || grant.writes_enabled
    {
        bail!("Workbench fake process start authorization is not current for this admission");
    }
    Ok(())
}
