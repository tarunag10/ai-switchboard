//! Provider-neutral contracts shared by Switchboard surfaces.
//!
//! This crate deliberately has no filesystem, process, network, Tauri, or
//! platform dependencies. It is the first extraction boundary for the
//! cross-platform Router/Workbench architecture; platform adapters belong in
//! higher-level crates.

use serde::{Deserialize, Serialize};

pub mod plan_head;
pub mod presets;
pub mod process_admission;
pub mod process_grant;
pub mod process_run_spec;
pub mod router;
pub mod workbench;

pub use workbench::{WorkbenchEventKind, WorkbenchSessionAction, WorkbenchSessionStatus};

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

    #[test]
    fn workbench_enum_wire_contract_matches_the_existing_api() {
        let event_kinds = [
            (WorkbenchEventKind::Started, "started"),
            (WorkbenchEventKind::Attached, "attached"),
            (WorkbenchEventKind::Checkpoint, "checkpoint"),
            (WorkbenchEventKind::Paused, "paused"),
            (WorkbenchEventKind::Resumed, "resumed"),
            (WorkbenchEventKind::Cancelled, "cancelled"),
            (WorkbenchEventKind::Completed, "completed"),
            (WorkbenchEventKind::Forked, "forked"),
        ];
        for (kind, wire_value) in event_kinds {
            let encoded = serde_json::to_string(&kind).expect("serialize Workbench event kind");
            assert_eq!(encoded, format!("\"{wire_value}\""));
            let decoded: WorkbenchEventKind =
                serde_json::from_str(&encoded).expect("deserialize Workbench event kind");
            assert_eq!(decoded, kind);
        }

        let statuses = [
            (WorkbenchSessionStatus::Active, "active"),
            (WorkbenchSessionStatus::Paused, "paused"),
            (WorkbenchSessionStatus::Cancelled, "cancelled"),
            (WorkbenchSessionStatus::Completed, "completed"),
        ];
        for (status, wire_value) in statuses {
            let encoded = serde_json::to_string(&status).expect("serialize Workbench status");
            assert_eq!(encoded, format!("\"{wire_value}\""));
            let decoded: WorkbenchSessionStatus =
                serde_json::from_str(&encoded).expect("deserialize Workbench status");
            assert_eq!(decoded, status);
        }

        for (wire_value, expected) in [
            ("pause", WorkbenchSessionAction::Pause),
            ("resume", WorkbenchSessionAction::Resume),
            ("cancel", WorkbenchSessionAction::Cancel),
            ("complete", WorkbenchSessionAction::Complete),
        ] {
            let encoded = format!("\"{wire_value}\"");
            let decoded: WorkbenchSessionAction =
                serde_json::from_str(&encoded).expect("deserialize Workbench action");
            assert_eq!(
                std::mem::discriminant(&decoded),
                std::mem::discriminant(&expected)
            );
        }

        assert!(serde_json::from_str::<WorkbenchEventKind>("\"unknown\"").is_err());
        assert!(serde_json::from_str::<WorkbenchSessionStatus>("\"unknown\"").is_err());
        assert!(serde_json::from_str::<WorkbenchSessionAction>("\"unknown\"").is_err());
    }
}
