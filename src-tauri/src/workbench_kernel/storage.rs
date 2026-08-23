use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use super::events::WorkbenchSessionStatus;
use super::session::{
    deterministic_fork_session_id, CreateWorkbenchSessionInput, WorkbenchSession,
};

const WORKBENCH_LEDGER_FILE: &str = "workbench-sessions.json";
const WORKBENCH_LEDGER_SCHEMA_VERSION: u32 = 1;
const MAX_SESSIONS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    pub(crate) fn create(&self, input: CreateWorkbenchSessionInput) -> Result<WorkbenchSession> {
        let mut ledger = self.load()?;
        trim_terminal_sessions(&mut ledger.sessions);
        if ledger.sessions.len() >= MAX_SESSIONS {
            return Err(anyhow!(
                "Workbench session ledger is full; finish or remove a terminal session first"
            ));
        }
        let session = WorkbenchSession::create(input)?;
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
        let mut ledger = self.load()?;
        let session = ledger
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Workbench session was not found"))?;
        session.transition(action)?;
        let updated = session.clone();
        self.save(&ledger)?;
        Ok(updated)
    }

    pub(crate) fn fork(&self, session_id: &str, event_id: &str) -> Result<WorkbenchSession> {
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
        let parent = ledger
            .sessions
            .get_mut(session_id)
            .expect("Workbench parent was checked before mutable access");
        let child = parent.fork_at_event(event_id)?;
        ledger
            .sessions
            .insert(child.session_id.clone(), child.clone());
        self.save(&ledger)?;
        Ok(child)
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
    use super::WorkbenchStore;
    use crate::workbench_kernel::events::WorkbenchSessionAction;
    use crate::workbench_kernel::session::CreateWorkbenchSessionInput;

    fn input() -> CreateWorkbenchSessionInput {
        CreateWorkbenchSessionInput {
            workspace_digest: format!("sha256:{}", "b".repeat(64)),
            task_class: "planning".into(),
        }
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
}
