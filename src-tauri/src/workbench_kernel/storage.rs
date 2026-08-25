use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use switchboard_runtime::{PortableRuntime, RuntimeClock};
use uuid::Uuid;

use super::capability_grant::{WorkbenchAuthorityTransaction, WorkbenchProcessGrantStore};
use super::events::WorkbenchSessionStatus;
use super::session::{
    deterministic_fork_session_id, CreateWorkbenchSessionInput, WorkbenchSession,
};

pub(super) mod run_plan_head;

const WORKBENCH_LEDGER_FILE: &str = "workbench-sessions.json";
const WORKBENCH_LEDGER_SCHEMA_VERSION: u32 = 1;
const MAX_SESSIONS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkbenchLedger {
    schema_version: u32,
    sessions: BTreeMap<String, WorkbenchSession>,
}

impl Default for WorkbenchLedger {
    fn default() -> Self {
        Self {
            schema_version: WORKBENCH_LEDGER_SCHEMA_VERSION,
            sessions: BTreeMap::new(),
        }
    }
}

pub(crate) struct WorkbenchStore {
    path: PathBuf,
}

impl WorkbenchStore {
    pub(crate) fn in_app_storage() -> Self {
        Self {
            path: crate::storage::config_file(
                &crate::storage::app_data_dir(),
                WORKBENCH_LEDGER_FILE,
            ),
        }
    }

    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn list(&self) -> Result<Vec<WorkbenchSession>> {
        let mut sessions = self.load()?.sessions.into_values().collect::<Vec<_>>();
        sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(sessions)
    }

    pub(crate) fn get(&self, session_id: &str) -> Result<WorkbenchSession> {
        self.load()?
            .sessions
            .remove(session_id)
            .ok_or_else(|| anyhow!("Workbench session was not found"))
    }

    pub(crate) fn get_for_authority_transaction(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
        session_id: &str,
    ) -> Result<WorkbenchSession> {
        transaction.require_authority_directory(self.authority_directory()?)?;
        self.get(session_id)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn require_authority_transaction(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
    ) -> Result<()> {
        transaction.require_authority_directory(self.authority_directory()?)
    }

    pub(crate) fn create(&self, input: CreateWorkbenchSessionInput) -> Result<WorkbenchSession> {
        self.create_with_clock(&PortableRuntime, input)
    }

    pub(crate) fn create_with_clock<C>(
        &self,
        clock: &C,
        input: CreateWorkbenchSessionInput,
    ) -> Result<WorkbenchSession>
    where
        C: RuntimeClock + ?Sized,
    {
        self.create_with_clock_and_identity(clock, random_session_id, input)
    }

    fn create_with_clock_and_identity<C, NewSessionId>(
        &self,
        clock: &C,
        new_session_id: NewSessionId,
        input: CreateWorkbenchSessionInput,
    ) -> Result<WorkbenchSession>
    where
        C: RuntimeClock + ?Sized,
        NewSessionId: Fn() -> String,
    {
        let transaction = self.begin_authority_transaction()?;
        transaction.require_authority_directory(self.authority_directory()?)?;
        let unix_millis = clock.try_unix_millis()?;
        let session_id = new_session_id();
        let session = WorkbenchSession::create_with_session_id_at_unix_millis(
            input,
            &session_id,
            unix_millis,
        )?;
        let mut ledger = self.load()?;
        trim_terminal_sessions(&mut ledger.sessions);
        if ledger.sessions.len() >= MAX_SESSIONS {
            return Err(anyhow!(
                "Workbench session ledger is full; finish or remove a terminal session first"
            ));
        }
        ledger
            .sessions
            .insert(session.session_id.clone(), session.clone());
        self.save(&ledger)?;
        Ok(session)
    }

    pub(crate) fn transition(
        &self,
        session_id: &str,
        action: super::events::WorkbenchSessionAction,
    ) -> Result<WorkbenchSession> {
        self.transition_with_clock(&PortableRuntime, session_id, action)
    }

    pub(crate) fn transition_with_clock<C>(
        &self,
        clock: &C,
        session_id: &str,
        action: super::events::WorkbenchSessionAction,
    ) -> Result<WorkbenchSession>
    where
        C: RuntimeClock + ?Sized,
    {
        let transaction = self.begin_authority_transaction()?;
        self.transition_for_authority_transaction_with_clock(
            &transaction,
            clock,
            session_id,
            action,
        )
    }

    fn transition_for_authority_transaction_with_clock<C>(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
        clock: &C,
        session_id: &str,
        action: super::events::WorkbenchSessionAction,
    ) -> Result<WorkbenchSession>
    where
        C: RuntimeClock + ?Sized,
    {
        transaction.require_authority_directory(self.authority_directory()?)?;
        let mut ledger = self.load()?;
        let session = ledger
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Workbench session was not found"))?;
        let unix_millis = clock.try_unix_millis()?;
        session.transition_at_unix_millis(action, unix_millis)?;
        let updated = session.clone();
        self.save(&ledger)?;
        Ok(updated)
    }

    pub(crate) fn fork(&self, session_id: &str, event_id: &str) -> Result<WorkbenchSession> {
        self.fork_with_clock(&PortableRuntime, session_id, event_id)
    }

    pub(crate) fn fork_with_clock<C>(
        &self,
        clock: &C,
        session_id: &str,
        event_id: &str,
    ) -> Result<WorkbenchSession>
    where
        C: RuntimeClock + ?Sized,
    {
        let transaction = self.begin_authority_transaction()?;
        self.fork_for_authority_transaction_with_clock(&transaction, clock, session_id, event_id)
    }

    fn fork_for_authority_transaction_with_clock<C>(
        &self,
        transaction: &WorkbenchAuthorityTransaction,
        clock: &C,
        session_id: &str,
        event_id: &str,
    ) -> Result<WorkbenchSession>
    where
        C: RuntimeClock + ?Sized,
    {
        transaction.require_authority_directory(self.authority_directory()?)?;
        let mut ledger = self.load()?;
        let parent = ledger
            .sessions
            .get(session_id)
            .ok_or_else(|| anyhow!("Workbench session was not found"))?;
        if !parent.events.iter().any(|event| event.event_id == event_id) {
            return Err(anyhow!(
                "Workbench fork event does not belong to this session"
            ));
        }
        let child_session_id = deterministic_fork_session_id(session_id, event_id);
        if let Some(existing) = ledger.sessions.get(&child_session_id) {
            if existing.parent_session_id.as_deref() == Some(session_id)
                && existing.fork_event_id.as_deref() == Some(event_id)
            {
                return Ok(existing.clone());
            }
            return Err(anyhow!(
                "Workbench fork ID collides with an unrelated session"
            ));
        }
        let unix_millis = clock.try_unix_millis()?;
        let parent = ledger
            .sessions
            .get_mut(session_id)
            .expect("Workbench parent was checked before mutable access");
        let child = parent.fork_at_event_at_unix_millis(event_id, unix_millis)?;
        ledger
            .sessions
            .insert(child.session_id.clone(), child.clone());
        self.save(&ledger)?;
        Ok(child)
    }

    fn authority_directory(&self) -> Result<&std::path::Path> {
        self.path
            .parent()
            .ok_or_else(|| anyhow!("Workbench session ledger has no parent directory"))
    }

    fn begin_authority_transaction(&self) -> Result<WorkbenchAuthorityTransaction> {
        WorkbenchProcessGrantStore::for_authority_directory(self.authority_directory()?)
            .begin_authority_transaction()
    }

    fn load(&self) -> Result<WorkbenchLedger> {
        if !self.path.exists() {
            return Ok(WorkbenchLedger::default());
        }
        let bytes = std::fs::read(&self.path)
            .with_context(|| format!("reading Workbench ledger {}", self.path.display()))?;
        let ledger: WorkbenchLedger = serde_json::from_slice(&bytes)
            .with_context(|| format!("decoding Workbench ledger {}", self.path.display()))?;
        if ledger.schema_version != WORKBENCH_LEDGER_SCHEMA_VERSION {
            return Err(anyhow!("Unsupported Workbench ledger schema version"));
        }
        if ledger.sessions.len() > MAX_SESSIONS {
            return Err(anyhow!(
                "Workbench ledger exceeds its session retention cap"
            ));
        }
        for (session_id, session) in &ledger.sessions {
            if session_id != &session.session_id {
                return Err(anyhow!(
                    "Workbench ledger session key does not match its payload"
                ));
            }
            session.validate()?;
        }
        Ok(ledger)
    }

    fn save(&self, ledger: &WorkbenchLedger) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("creating Workbench ledger directory {}", parent.display())
            })?;
        }
        let temporary = self
            .path
            .with_extension(format!("json.tmp.{}", std::process::id()));
        let bytes = serde_json::to_vec_pretty(ledger).context("encoding Workbench ledger")?;
        std::fs::write(&temporary, bytes)
            .with_context(|| format!("writing Workbench ledger {}", temporary.display()))?;
        std::fs::rename(&temporary, &self.path).with_context(|| {
            format!(
                "committing Workbench ledger {} -> {}",
                temporary.display(),
                self.path.display()
            )
        })
    }
}

fn random_session_id() -> String {
    format!("workbench:{}", Uuid::new_v4())
}

fn trim_terminal_sessions(sessions: &mut BTreeMap<String, WorkbenchSession>) {
    if sessions.len() < MAX_SESSIONS {
        return;
    }
    let mut terminal = sessions
        .iter()
        .filter(|(_, session)| {
            matches!(
                session.status,
                WorkbenchSessionStatus::Cancelled | WorkbenchSessionStatus::Completed
            )
        })
        .map(|(id, session)| (session.updated_at.clone(), id.clone()))
        .collect::<Vec<_>>();
    terminal.sort();
    for (_, id) in terminal {
        if sessions.len() < MAX_SESSIONS {
            break;
        }
        sessions.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkbenchLedger, WorkbenchStore};
    use crate::workbench_kernel::events::WorkbenchSessionAction;
    use crate::workbench_kernel::session::CreateWorkbenchSessionInput;
    use switchboard_runtime::{FixedClock, RuntimeClock, RuntimeClockError};

    #[derive(Clone, Copy, Debug)]
    struct FailingClock {
        unix_millis: i64,
    }

    impl RuntimeClock for FailingClock {
        fn unix_millis(&self) -> i64 {
            self.unix_millis
        }

        fn try_unix_millis(&self) -> Result<i64, RuntimeClockError> {
            Err(RuntimeClockError::Failed(
                "injected Workbench clock failure",
            ))
        }
    }

    fn input() -> CreateWorkbenchSessionInput {
        CreateWorkbenchSessionInput {
            workspace_digest: format!("sha256:{}", "b".repeat(64)),
            task_class: "planning".into(),
        }
    }

    fn timestamp_at(unix_millis: i64) -> String {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(unix_millis)
            .expect("valid fixed timestamp")
            .to_rfc3339()
    }

    #[test]
    fn persisted_session_ledger_rejects_unknown_fields() {
        let value = serde_json::json!({
            "schemaVersion": 1,
            "sessions": {},
            "workspacePath": "/must/not/persist"
        });
        assert!(serde_json::from_value::<WorkbenchLedger>(value).is_err());
    }

    #[test]
    fn store_persists_transition_and_rejects_corrupt_ledger() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let path = directory.path().join("workbench-sessions.json");
        let store = WorkbenchStore::at(path.clone());
        let created = store.create(input()).expect("create session");
        store
            .transition(&created.session_id, WorkbenchSessionAction::Pause)
            .expect("pause session");
        assert_eq!(
            store.get(&created.session_id).expect("load session").status,
            crate::workbench_kernel::events::WorkbenchSessionStatus::Paused
        );
        std::fs::write(&path, "{not json").expect("corrupt ledger");
        assert!(store.list().is_err());
    }

    #[test]
    fn repeated_fork_returns_the_existing_child_without_appending_another_event() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let store = WorkbenchStore::at(directory.path().join("workbench-sessions.json"));
        let parent = store.create(input()).expect("create session");
        let event_id = parent.events[0].event_id.clone();
        let first = store
            .fork(&parent.session_id, &event_id)
            .expect("fork session");
        let event_count = store
            .get(&parent.session_id)
            .expect("load parent")
            .events
            .len();
        let second = store
            .fork(&parent.session_id, &event_id)
            .expect("repeat fork");
        assert_eq!(first.session_id, second.session_id);
        assert_eq!(
            store
                .get(&parent.session_id)
                .expect("reload parent")
                .events
                .len(),
            event_count
        );
    }

    #[test]
    fn identity_provider_supplies_deterministic_session_ids() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let store = WorkbenchStore::at(directory.path().join("workbench-sessions.json"));
        let created = store
            .create_with_clock_and_identity(
                &FixedClock::new(1_700_000_000_123),
                || "workbench:00000000-0000-4000-8000-000000000001".into(),
                input(),
            )
            .expect("create session with injected identity");
        assert_eq!(
            created.session_id,
            "workbench:00000000-0000-4000-8000-000000000001"
        );
        assert_eq!(
            store
                .get(&created.session_id)
                .expect("reload session")
                .session_id,
            created.session_id
        );
    }

    #[test]
    fn fixed_clock_supplies_one_timestamp_per_lifecycle_mutation() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let store = WorkbenchStore::at(directory.path().join("workbench-sessions.json"));

        let create_millis = 1_700_000_000_123;
        let create_clock = FixedClock::new(create_millis);
        let created = store
            .create_with_clock(&create_clock, input())
            .expect("create session with fixed clock");
        let created_at = timestamp_at(create_millis);
        assert_eq!(created.created_at, created_at);
        assert_eq!(created.updated_at, created_at);
        assert_eq!(created.events[0].occurred_at, created_at);

        let transition_millis = create_millis + 1_000;
        let transition_clock = FixedClock::new(transition_millis);
        let transitioned = store
            .transition_with_clock(
                &transition_clock,
                &created.session_id,
                WorkbenchSessionAction::Pause,
            )
            .expect("transition session with fixed clock");
        let transitioned_at = timestamp_at(transition_millis);
        assert_eq!(transitioned.updated_at, transitioned_at);
        assert_eq!(
            transitioned
                .events
                .last()
                .expect("transition event")
                .occurred_at,
            transitioned_at
        );

        let fork_millis = transition_millis + 1_000;
        let fork_clock = FixedClock::new(fork_millis);
        let child = store
            .fork_with_clock(
                &fork_clock,
                &created.session_id,
                &created.events[0].event_id,
            )
            .expect("fork session with fixed clock");
        let forked_at = timestamp_at(fork_millis);
        let parent = store
            .get(&created.session_id)
            .expect("load persisted parent");
        assert_eq!(parent.updated_at, forked_at);
        assert_eq!(
            parent.events.last().expect("fork event").occurred_at,
            forked_at
        );
        assert_eq!(child.created_at, forked_at);
        assert_eq!(child.updated_at, forked_at);
        assert_eq!(child.events[0].occurred_at, forked_at);
    }

    #[test]
    fn clock_failure_happens_before_ledger_save() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let path = directory.path().join("workbench-sessions.json");
        let store = WorkbenchStore::at(path.clone());

        let create_failure = FailingClock { unix_millis: 0 };
        let error = store
            .create_with_clock(&create_failure, input())
            .expect_err("clock failure must reject creation");
        assert!(error
            .to_string()
            .contains("injected Workbench clock failure"));
        assert!(!path.exists());

        let created = store
            .create_with_clock(&FixedClock::new(1_700_000_000_123), input())
            .expect("seed persisted session");
        let persisted_before = std::fs::read(&path).expect("read seeded ledger");

        let transition_failure = FailingClock { unix_millis: 0 };
        store
            .transition_with_clock(
                &transition_failure,
                &created.session_id,
                WorkbenchSessionAction::Pause,
            )
            .expect_err("clock failure must reject transition");
        assert_eq!(
            std::fs::read(&path).expect("read ledger after failed transition"),
            persisted_before
        );

        let fork_failure = FailingClock { unix_millis: 0 };
        store
            .fork_with_clock(
                &fork_failure,
                &created.session_id,
                &created.events[0].event_id,
            )
            .expect_err("clock failure must reject fork");
        assert_eq!(
            std::fs::read(&path).expect("read ledger after failed fork"),
            persisted_before
        );
    }
}
