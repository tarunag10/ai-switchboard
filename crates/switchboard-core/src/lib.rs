//! Provider-neutral contracts shared by Switchboard surfaces.
//!
//! This crate deliberately has no filesystem, process, network, Tauri, or
//! platform dependencies. It is the first extraction boundary for the
//! cross-platform Router/Workbench architecture; platform adapters belong in
//! higher-level crates.

use serde::{Deserialize, Serialize};

pub const CORE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    ObserveOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningStrategy {
    DeterministicEndpoint,
    ObserveOnlyShadow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessSurface {
    Cli,
    Desktop,
    Workbench,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessStatus {
    pub contract_version: u32,
    pub surface: HarnessSurface,
    pub execution_mode: ExecutionMode,
    pub provider_traffic_enabled: bool,
    pub process_start_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchEventKind {
    Started,
    Attached,
    Checkpoint,
    Paused,
    Resumed,
    Cancelled,
    Completed,
    Forked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchSessionStatus {
    Active,
    Paused,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchSessionAction {
    Pause,
    Resume,
    Cancel,
    Complete,
}

impl HarnessStatus {
    pub fn local_preview(surface: HarnessSurface) -> Self {
        Self {
            contract_version: CORE_CONTRACT_VERSION,
            surface,
            execution_mode: ExecutionMode::ObserveOnly,
            provider_traffic_enabled: false,
            process_start_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_preview_is_fail_closed_and_serializable() {
        let status = HarnessStatus::local_preview(HarnessSurface::Cli);
        assert_eq!(status.contract_version, CORE_CONTRACT_VERSION);
        assert_eq!(status.execution_mode, ExecutionMode::ObserveOnly);
        assert!(!status.provider_traffic_enabled);
        assert!(!status.process_start_enabled);

        let json = serde_json::to_value(status).expect("serialize core status");
        assert_eq!(json["surface"], "cli");
        assert_eq!(json["executionMode"], "observe_only");
    }
}
