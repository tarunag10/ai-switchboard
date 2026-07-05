use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::RepoMemoryMcpServiceStatus;
use crate::state::AppState;

pub(super) const REPO_MEMORY_MCP_SUPERVISION_INTERVAL_SECS: i64 = 15 * 60;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RepoMemoryMcpSessionState {
    pub(super) active: bool,
    pub(super) last_started_at: Option<DateTime<Utc>>,
    pub(super) last_checked_at: Option<DateTime<Utc>>,
    pub(super) supervision_status: Option<String>,
    pub(super) supervisor_pid: Option<u32>,
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
        {
            let mut session = self.repo_memory_mcp_state.lock();
            session.active = true;
            session.last_started_at = Some(Utc::now());
            session.last_checked_at = Some(Utc::now());
            session.supervision_status = Some("verified_active".to_string());
            session.supervisor_pid = Some(std::process::id());
            self.persist_repo_memory_mcp_state(&session)?;
        }
        *self.cached_runtime_status.lock() = None;
        Ok(())
    }

    pub fn stop_repo_memory_mcp(&self) -> Result<()> {
        {
            let mut session = self.repo_memory_mcp_state.lock();
            session.active = false;
            session.last_checked_at = Some(Utc::now());
            session.supervision_status = Some("stopped".to_string());
            session.supervisor_pid = None;
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
        };
        if !should_verify {
            return;
        }

        let (active, status) = match self.tool_manager.verify_repo_memory_mcp_smoke() {
            Ok(_) => (true, "verified_active".to_string()),
            Err(err) => {
                log::warn!("repo-memory MCP supervision smoke failed: {err:#}");
                (false, "smoke_failed".to_string())
            }
        };

        let mut session = self.repo_memory_mcp_state.lock();
        session.active = active;
        session.last_checked_at = Some(Utc::now());
        session.supervision_status = Some(status);
        session.supervisor_pid = if active { Some(current_pid) } else { None };
        if let Err(err) = self.persist_repo_memory_mcp_state(&session) {
            log::warn!("failed to persist repo-memory MCP supervision state: {err:#}");
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
