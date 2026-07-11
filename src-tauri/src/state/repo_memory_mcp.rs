use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::RepoMemoryMcpServiceStatus;
use crate::state::AppState;

pub(super) const REPO_MEMORY_MCP_SUPERVISION_INTERVAL_SECS: i64 = 15 * 60;
pub(super) const REPO_MEMORY_MCP_STALE_AFTER_SECS: i64 =
    REPO_MEMORY_MCP_SUPERVISION_INTERVAL_SECS * 2;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RepoMemoryMcpSessionState {
    pub(super) active: bool,
    pub(super) last_started_at: Option<DateTime<Utc>>,
    pub(super) last_checked_at: Option<DateTime<Utc>>,
    pub(super) supervision_status: Option<String>,
    pub(super) supervisor_pid: Option<u32>,
    #[serde(default)]
    pub(super) child_pid: Option<u32>,
    #[serde(default)]
    pub(super) restart_count: u32,
    #[serde(default)]
    pub(super) last_restart_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(super) last_exit_at: Option<DateTime<Utc>>,
}

impl RepoMemoryMcpSessionState {
    pub(super) fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_slice(&bytes).context("parsing repo-memory MCP session state")
    }
}

impl AppState {
    pub fn start_repo_memory_mcp(&self) -> Result<()> {
        self.tool_manager.ensure_repo_memory_mcp_configured()?;
        if let Err(err) = self.tool_manager.verify_repo_memory_mcp_smoke() {
            self.stop_repo_memory_mcp_child();
            {
                let mut session = self.repo_memory_mcp_state.lock();
                session.active = false;
                session.last_checked_at = Some(Utc::now());
                session.supervision_status = Some("smoke_failed".to_string());
                session.supervisor_pid = None;
                self.persist_repo_memory_mcp_state(&session)?;
            }
            *self.cached_runtime_status.lock() = None;
            return Err(err);
        }
        let (child_pid, _spawned) = match self.ensure_repo_memory_mcp_child() {
            Ok(result) => result,
            Err(err) => {
                self.record_repo_memory_mcp_failure("launch_failed");
                return Err(err);
            }
        };
        {
            let mut session = self.repo_memory_mcp_state.lock();
            session.active = true;
            session.last_started_at = Some(Utc::now());
            session.last_checked_at = Some(Utc::now());
            session.supervision_status = Some("verified_active".to_string());
            session.supervisor_pid = Some(std::process::id());
            session.child_pid = Some(child_pid);
            self.persist_repo_memory_mcp_state(&session)?;
        }
        *self.cached_runtime_status.lock() = None;
        Ok(())
    }

    pub fn stop_repo_memory_mcp(&self) -> Result<()> {
        self.stop_repo_memory_mcp_child();
        {
            let mut session = self.repo_memory_mcp_state.lock();
            session.active = false;
            session.last_checked_at = Some(Utc::now());
            session.supervision_status = Some("stopped".to_string());
            session.supervisor_pid = None;
            session.child_pid = None;
            self.persist_repo_memory_mcp_state(&session)?;
        }
        *self.cached_runtime_status.lock() = None;
        Ok(())
    }

    fn persist_repo_memory_mcp_state(&self, session: &RepoMemoryMcpSessionState) -> Result<()> {
        let serialized = serde_json::to_vec_pretty(session)
            .context("serializing repo-memory MCP session state")?;
        std::fs::write(&self.repo_memory_mcp_state_path, serialized)
            .with_context(|| format!("writing {}", self.repo_memory_mcp_state_path.display()))?;
        Ok(())
    }

    pub(super) fn record_repo_memory_mcp_supervision(&self, status: &str) {
        let mut session = self.repo_memory_mcp_state.lock();
        if session.supervision_status.as_deref() == Some(status)
            && session.last_checked_at.is_some()
        {
            return;
        }
        session.last_checked_at = Some(Utc::now());
        session.supervision_status = Some(status.to_string());
        if let Err(err) = self.persist_repo_memory_mcp_state(&session) {
            log::warn!("failed to persist repo-memory MCP supervision state: {err:#}");
        }
    }

    pub(super) fn supervise_repo_memory_mcp_if_due(&self, configured: Option<bool>) {
        let now = Utc::now();
        let current_pid = std::process::id();
        let should_verify = {
            let session = self.repo_memory_mcp_state.lock();
            repo_memory_mcp_supervision_due(&session, configured, current_pid, now)
        } || self.repo_memory_mcp_child_needs_restart(current_pid);
        if !should_verify {
            return;
        }

        let (active, status, child_pid, restarted) =
            match self.tool_manager.verify_repo_memory_mcp_smoke() {
                Ok(_) => match self.ensure_repo_memory_mcp_child() {
                    Ok((child_pid, restarted)) => (
                        true,
                        "verified_active".to_string(),
                        Some(child_pid),
                        restarted,
                    ),
                    Err(err) => {
                        log::warn!("repo-memory MCP supervision restart failed: {err:#}");
                        (false, "launch_failed".to_string(), None, false)
                    }
                },
                Err(err) => {
                    log::warn!("repo-memory MCP supervision smoke failed: {err:#}");
                    self.stop_repo_memory_mcp_child();
                    (false, "smoke_failed".to_string(), None, false)
                }
            };

        let mut session = self.repo_memory_mcp_state.lock();
        session.active = active;
        session.last_checked_at = Some(Utc::now());
        session.supervision_status = Some(status);
        session.supervisor_pid = if active { Some(current_pid) } else { None };
        session.child_pid = child_pid;
        if restarted {
            session.restart_count = session.restart_count.saturating_add(1);
            session.last_restart_at = Some(now);
        }
        if !active {
            session.last_exit_at = Some(now);
        }
        if let Err(err) = self.persist_repo_memory_mcp_state(&session) {
            log::warn!("failed to persist repo-memory MCP supervision state: {err:#}");
        }
    }

    fn ensure_repo_memory_mcp_child(&self) -> Result<(u32, bool)> {
        let mut process = self.repo_memory_mcp_process.lock();
        if let Some(existing) = process.as_mut() {
            match existing.try_wait() {
                Ok(None) => return Ok((existing.id(), false)),
                Ok(Some(_)) | Err(_) => *process = None,
            }
        }
        let child = self.tool_manager.spawn_repo_memory_mcp()?;
        let child_pid = child.id();
        *process = Some(child);
        Ok((child_pid, true))
    }

    fn repo_memory_mcp_child_needs_restart(&self, current_pid: u32) -> bool {
        let session = self.repo_memory_mcp_state.lock().clone();
        if !session.active || session.supervisor_pid != Some(current_pid) {
            return session.active && session.supervisor_pid != Some(current_pid);
        }
        let mut process = self.repo_memory_mcp_process.lock();
        match process.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => false,
                Ok(Some(_)) | Err(_) => {
                    *process = None;
                    drop(process);
                    let mut session = self.repo_memory_mcp_state.lock();
                    session.child_pid = None;
                    session.last_exit_at = Some(Utc::now());
                    session.supervision_status = Some("restart_required".to_string());
                    if let Err(err) = self.persist_repo_memory_mcp_state(&session) {
                        log::warn!("failed to persist repo-memory MCP exit state: {err:#}");
                    }
                    true
                }
            },
            None => true,
        }
    }

    fn stop_repo_memory_mcp_child(&self) {
        let Some(mut child) = self.repo_memory_mcp_process.lock().take() else {
            return;
        };
        if let Err(err) = child.kill() {
            log::debug!("repo-memory MCP child was already stopped: {err}");
        }
        let _ = child.wait();
    }

    fn record_repo_memory_mcp_failure(&self, status: &str) {
        let mut session = self.repo_memory_mcp_state.lock();
        session.active = false;
        session.last_checked_at = Some(Utc::now());
        session.last_exit_at = Some(Utc::now());
        session.supervision_status = Some(status.to_string());
        session.supervisor_pid = None;
        session.child_pid = None;
        if let Err(err) = self.persist_repo_memory_mcp_state(&session) {
            log::warn!("failed to persist repo-memory MCP failure state: {err:#}");
        }
    }
}

pub(super) fn repo_memory_mcp_supervision_status(
    session: &RepoMemoryMcpSessionState,
    configured: Option<bool>,
    current_pid: u32,
    service: Option<&RepoMemoryMcpServiceStatus>,
) -> String {
    if configured == Some(true) && !repo_memory_mcp_service_healthy(service) {
        return "service_unhealthy".to_string();
    }

    if session.active
        && session.supervision_status.as_deref() == Some("verified_active")
        && session
            .last_checked_at
            .map(|checked| {
                Utc::now().signed_duration_since(checked).num_seconds()
                    > REPO_MEMORY_MCP_STALE_AFTER_SECS
            })
            .unwrap_or(false)
    {
        return "stale_health".to_string();
    }

    match (session.active, configured) {
        (true, Some(true)) => {
            if session.supervision_status.as_deref() == Some("verified_active")
                && session.supervisor_pid == Some(current_pid)
            {
                "verified_active".to_string()
            } else if session.supervision_status.as_deref() == Some("verified_active") {
                "restart_required".to_string()
            } else {
                session
                    .supervision_status
                    .clone()
                    .unwrap_or_else(|| "active".to_string())
            }
        }
        (true, Some(false)) => "stale_config".to_string(),
        (true, None) => "unknown_active".to_string(),
        (false, Some(true)) if session.supervision_status.as_deref() == Some("smoke_failed") => {
            "smoke_failed".to_string()
        }
        (false, Some(true)) if session.supervision_status.as_deref() == Some("launch_failed") => {
            "launch_failed".to_string()
        }
        (false, Some(true)) => "configured".to_string(),
        (false, Some(false)) => "needs_attention".to_string(),
        (false, None) => "unknown".to_string(),
    }
}

pub(super) fn repo_memory_mcp_service_healthy(
    service: Option<&RepoMemoryMcpServiceStatus>,
) -> bool {
    let Some(service) = service else {
        return false;
    };
    service.managed_by_app
        && service.read_only
        && service.healthy
        && service.issues.is_empty()
        && service.descriptor_present
        && service.script_present
        && service.node_available
}

pub(super) fn repo_memory_mcp_supervision_due(
    session: &RepoMemoryMcpSessionState,
    configured: Option<bool>,
    current_pid: u32,
    now: DateTime<Utc>,
) -> bool {
    if configured != Some(true)
        || !session.active
        || session.supervision_status.as_deref() != Some("verified_active")
    {
        return false;
    }

    if session.supervisor_pid != Some(current_pid) {
        return true;
    }

    match session.last_checked_at {
        Some(last_checked_at) => {
            now.signed_duration_since(last_checked_at).num_seconds()
                >= REPO_MEMORY_MCP_SUPERVISION_INTERVAL_SECS
        }
        None => true,
    }
}
