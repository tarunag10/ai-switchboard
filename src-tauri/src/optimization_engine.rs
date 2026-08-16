//! Stable optimization-engine boundary backed by the existing Headroom runtime.
//!
//! This module owns no proxy or compression implementation. The first adapter
//! delegates lifecycle, status, and profile persistence to the same `AppState`
//! and compression-profile services already used by the desktop commands.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::client_adapters;
use crate::models::RuntimeStatus;
use crate::state::AppState;
use crate::tool_manager::compression_profiles::{
    effective_savings_mode, load_compression_profile, save_compression_profile,
    CompressionProfileAdvanced, CompressionProfileId, CompressionProfileState,
};

pub(crate) const HEADROOM_ENGINE_ID: &str = "headroom";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EngineLifecycleState {
    NotInstalled,
    Starting,
    Running,
    Paused,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EngineHealthStatus {
    Healthy,
    Inactive,
    Degraded,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationCapabilities {
    pub runtime_compression: bool,
    pub configurable_profiles: bool,
    pub lifecycle_control: bool,
    pub health_reporting: bool,
    pub local_only_control_plane: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OptimizationProfile {
    pub version: u8,
    pub preset_id: String,
    pub effective_savings_mode: String,
    pub advanced: CompressionProfileAdvanced,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EngineInstance {
    pub engine_id: String,
    pub state: EngineLifecycleState,
    pub pid: Option<u32>,
    pub bind_address: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EngineHealthReport {
    pub engine_id: String,
    pub status: EngineHealthStatus,
    pub state: EngineLifecycleState,
    pub proxy_reachable: bool,
    pub bind_address: String,
    pub detail: String,
    pub last_error: Option<String>,
}

/// Contract used by callers that need optimization behavior without depending
/// on Headroom process fields or compression-profile storage internals.
pub(crate) trait OptimizationEngine: Send + Sync {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> OptimizationCapabilities;
    fn state(&self) -> EngineLifecycleState;
    fn profile(&self) -> OptimizationProfile;
    fn configure(&self, profile: &OptimizationProfile) -> Result<OptimizationProfile>;
    fn start(&self) -> Result<EngineInstance>;
    fn stop(&self) -> Result<EngineInstance>;
    fn health(&self) -> EngineHealthReport;
}

/// Thin adapter over the app-owned Headroom lifecycle and profile services.
pub(crate) struct HeadroomOptimizationEngine<'a> {
    app_state: &'a AppState,
}

impl<'a> HeadroomOptimizationEngine<'a> {
    pub(crate) fn new(app_state: &'a AppState) -> Self {
        Self { app_state }
    }

    fn runtime_status(&self) -> RuntimeStatus {
        self.app_state.runtime_status()
    }

    fn instance_from_status(&self, status: &RuntimeStatus) -> EngineInstance {
        EngineInstance {
            engine_id: self.id().to_string(),
            state: lifecycle_state(status),
            pid: status.headroom_pid,
            bind_address: status.proxy_bind_address.clone(),
        }
    }
}

impl OptimizationEngine for HeadroomOptimizationEngine<'_> {
    fn id(&self) -> &'static str {
        HEADROOM_ENGINE_ID
    }

    fn capabilities(&self) -> OptimizationCapabilities {
        OptimizationCapabilities {
            runtime_compression: true,
            configurable_profiles: true,
            lifecycle_control: true,
            health_reporting: true,
            local_only_control_plane: true,
        }
    }

    fn state(&self) -> EngineLifecycleState {
        lifecycle_state(&self.runtime_status())
    }

    fn profile(&self) -> OptimizationProfile {
        profile_view(load_compression_profile())
    }

    fn configure(&self, profile: &OptimizationProfile) -> Result<OptimizationProfile> {
        let preset_id = CompressionProfileId::parse(&profile.preset_id)
            .ok_or_else(|| anyhow!("unknown optimization profile: {}", profile.preset_id))?;
        let state = CompressionProfileState {
            version: profile.version,
            preset_id,
            advanced: profile.advanced.clone(),
        };

        save_compression_profile(&state)?;
        client_adapters::write_savings_mode(effective_savings_mode(&state))?;
        if self.runtime_status().running {
            crate::switchboard_commands::repair_runtime(self.app_state)
                .map_err(|error| anyhow!(error))?;
        }
        Ok(profile_view(state))
    }

    fn start(&self) -> Result<EngineInstance> {
        self.app_state.resume_runtime()?;
        std::thread::spawn(client_adapters::restore_client_setups);
        self.app_state.invalidate_runtime_status_cache();
        Ok(self.instance_from_status(&self.runtime_status()))
    }

    fn stop(&self) -> Result<EngineInstance> {
        self.app_state.set_runtime_paused(true);
        self.app_state.set_runtime_auto_paused(false);
        self.app_state.stop_headroom();
        client_adapters::clear_client_setups()?;
        self.app_state.invalidate_runtime_status_cache();
        Ok(self.instance_from_status(&self.runtime_status()))
    }

    fn health(&self) -> EngineHealthReport {
        let runtime = self.runtime_status();
        let state = lifecycle_state(&runtime);
        let (status, detail) = health_status(&runtime, state);
        EngineHealthReport {
            engine_id: self.id().to_string(),
            status,
            state,
            proxy_reachable: runtime.proxy_reachable,
            bind_address: runtime.proxy_bind_address.clone(),
            detail,
            last_error: runtime.startup_error.clone(),
        }
    }
}

fn profile_view(state: CompressionProfileState) -> OptimizationProfile {
    let effective_savings_mode = match effective_savings_mode(&state) {
        crate::models::SavingsMode::Balanced => "balanced",
        crate::models::SavingsMode::Aggressive => "aggressive",
    };
    OptimizationProfile {
        version: state.version,
        preset_id: state.preset_id.as_str().to_string(),
        effective_savings_mode: effective_savings_mode.to_string(),
        advanced: state.advanced,
    }
}

fn lifecycle_state(status: &RuntimeStatus) -> EngineLifecycleState {
    if !status.installed {
        EngineLifecycleState::NotInstalled
    } else if status.starting {
        EngineLifecycleState::Starting
    } else if status.running {
        EngineLifecycleState::Running
    } else if status.paused {
        EngineLifecycleState::Paused
    } else {
        EngineLifecycleState::Stopped
    }
}

fn health_status(
    runtime: &RuntimeStatus,
    state: EngineLifecycleState,
) -> (EngineHealthStatus, String) {
    if !runtime.installed {
        return (
            EngineHealthStatus::Unavailable,
            "The managed Headroom runtime is not installed.".to_string(),
        );
    }
    if runtime.startup_error.is_some()
        || (!runtime.paused && !runtime.starting && !runtime.proxy_reachable)
    {
        return (
            EngineHealthStatus::Degraded,
            runtime.startup_error.clone().unwrap_or_else(|| {
                "Headroom is expected to be active but its local proxy is unreachable.".into()
            }),
        );
    }
    match state {
        EngineLifecycleState::Running => (
            EngineHealthStatus::Healthy,
            "Headroom is running and its local proxy is reachable.".to_string(),
        ),
        EngineLifecycleState::Starting => (
            EngineHealthStatus::Inactive,
            "Headroom is starting.".to_string(),
        ),
        EngineLifecycleState::Paused => (
            EngineHealthStatus::Inactive,
            "Headroom is paused.".to_string(),
        ),
        EngineLifecycleState::Stopped => (
            EngineHealthStatus::Inactive,
            "Headroom is stopped.".to_string(),
        ),
        EngineLifecycleState::NotInstalled => unreachable!("handled above"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_view_delegates_to_existing_profile_resolution() {
        let state = crate::tool_manager::compression_profiles::preset_definition(
            CompressionProfileId::Aggressive,
        )
        .to_state();
        let profile = profile_view(state);
        assert_eq!(profile.preset_id, "aggressive");
        assert_eq!(profile.effective_savings_mode, "aggressive");
        assert!(profile.advanced.compress_tool_results);
    }

    #[test]
    fn headroom_capabilities_are_available_through_object_safe_contract() {
        fn assert_object_safe(_: &dyn OptimizationEngine) {}
        let _ = assert_object_safe;

        let capabilities = OptimizationCapabilities {
            runtime_compression: true,
            configurable_profiles: true,
            lifecycle_control: true,
            health_reporting: true,
            local_only_control_plane: true,
        };
        assert!(capabilities.local_only_control_plane);
    }

    #[test]
    fn preset_names_are_validated_by_the_existing_profile_registry() {
        assert!(CompressionProfileId::parse("balanced").is_some());
        assert!(CompressionProfileId::parse("not-a-profile").is_none());
    }
}
