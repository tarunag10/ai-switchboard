//! Native-owned, declarative Workbench plan presets.
//!
//! These presets only compose existing plan-only capabilities and their
//! receipt-backed evidence sources. They do not save routing policy, launch a
//! provider, or add a second Router implementation.

use super::events::validate_identifier;
use anyhow::{anyhow, Result};

pub(crate) use switchboard_core::presets::{
    validate_workbench_plan_preset, WorkbenchPlanPreset, PRESET_SCHEMA_VERSION,
};

pub(crate) fn all_workbench_plan_presets() -> Vec<WorkbenchPlanPreset> {
    vec![
        WorkbenchPlanPreset {
            schema_version: PRESET_SCHEMA_VERSION,
            preset_id: "adapter-plan-review".into(),
            label: "Adapter plan review".into(),
            description: "Inspect one native Router receipt with an existing reversible client-adapter dry run.".into(),
            required_capability_ids: vec!["router_observe".into(), "client_adapter_plan".into()],
            evidence_source: "native_router_decision_receipt".into(),
            routing_mode: "observe_only".into(),
            execution_mode: "plan_only".into(),
            provider_traffic: "none".into(),
            writes_enabled: false,
        },
        WorkbenchPlanPreset {
            schema_version: PRESET_SCHEMA_VERSION,
            preset_id: "evidence-review".into(),
            label: "Router and replay review".into(),
            description: "Inspect separately native-validated Router and redacted replay receipts before an adapter dry run.".into(),
            required_capability_ids: vec![
                "router_observe".into(),
                "redacted_replay".into(),
                "client_adapter_plan".into(),
            ],
            evidence_source: "native_router_and_replay_receipts".into(),
            routing_mode: "observe_only".into(),
            execution_mode: "plan_only".into(),
            provider_traffic: "none".into(),
            writes_enabled: false,
        },
    ]
}

pub(crate) fn resolve_workbench_plan_preset(preset_id: &str) -> Result<WorkbenchPlanPreset> {
    validate_identifier(preset_id, "preset ID")?;
    all_workbench_plan_presets()
        .into_iter()
        .find(|preset| preset.preset_id == preset_id)
        .ok_or_else(|| anyhow!("Workbench plan preset is unknown"))
}

#[cfg(test)]
mod tests {
    use super::{
        all_workbench_plan_presets, resolve_workbench_plan_preset, validate_workbench_plan_preset,
    };

    #[test]
    fn presets_only_compose_existing_plan_only_capabilities() {
        let presets = all_workbench_plan_presets();
        assert_eq!(presets.len(), 2);
        assert!(presets.iter().all(|preset| {
            validate_workbench_plan_preset(preset).is_ok()
                && preset.routing_mode == "observe_only"
                && preset.execution_mode == "plan_only"
                && preset.provider_traffic == "none"
                && !preset.writes_enabled
        }));
    }

    #[test]
    fn preset_resolution_is_native_and_fail_closed() {
        assert_eq!(
            resolve_workbench_plan_preset("evidence-review")
                .expect("resolve preset")
                .evidence_source,
            "native_router_and_replay_receipts"
        );
        assert!(resolve_workbench_plan_preset("manual-route").is_err());
    }
}
