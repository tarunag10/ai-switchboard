//! Native-owned, declarative Workbench plan presets.
//!
//! These presets only compose existing plan-only capabilities and their
//! receipt-backed evidence sources. They do not save routing policy, launch a
//! provider, or add a second Router implementation.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use super::events::validate_identifier;

const PRESET_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkbenchPlanPreset {
    pub(crate) schema_version: u32,
    pub(crate) preset_id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) required_capability_ids: Vec<String>,
    pub(crate) evidence_source: String,
    pub(crate) routing_mode: String,
    pub(crate) execution_mode: String,
    pub(crate) provider_traffic: String,
    pub(crate) writes_enabled: bool,
}

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

pub(crate) fn validate_workbench_plan_preset(preset: &WorkbenchPlanPreset) -> Result<()> {
    if preset.schema_version != PRESET_SCHEMA_VERSION {
        bail!("Workbench plan preset schema is unsupported");
    }
    validate_identifier(&preset.preset_id, "preset ID")?;
    if preset.label.trim().is_empty()
        || preset.label.len() > 96
        || preset.description.trim().is_empty()
        || preset.description.len() > 256
        || !matches!(
            preset.evidence_source.as_str(),
            "native_router_decision_receipt" | "native_router_and_replay_receipts"
        )
        || preset.routing_mode != "observe_only"
        || preset.execution_mode != "plan_only"
        || preset.provider_traffic != "none"
        || preset.writes_enabled
    {
        bail!("Workbench plan preset violates the plan-only boundary");
    }
    if preset.required_capability_ids.is_empty() || preset.required_capability_ids.len() > 10 {
        bail!("Workbench plan preset has an invalid capability set");
    }
    let mut seen = std::collections::BTreeSet::new();
    for capability_id in &preset.required_capability_ids {
        validate_identifier(capability_id, "preset capability ID")?;
        if !matches!(
            capability_id.as_str(),
            "repo_context" | "redacted_replay" | "router_observe" | "client_adapter_plan"
        ) || !seen.insert(capability_id)
        {
            bail!("Workbench plan preset has an unsupported capability set");
        }
    }
    if !preset
        .required_capability_ids
        .iter()
        .any(|capability_id| capability_id == "router_observe")
    {
        bail!("Workbench plan preset requires native Router evidence");
    }
    let requires_replay = preset
        .required_capability_ids
        .iter()
        .any(|capability_id| capability_id == "redacted_replay");
    if requires_replay != (preset.evidence_source == "native_router_and_replay_receipts") {
        bail!("Workbench plan preset replay evidence does not match its capabilities");
    }
    Ok(())
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
