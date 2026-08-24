//! Provider-neutral, declarative Workbench plan preset contracts.
//!
//! Presets describe plan-only capability composition. Catalog ownership,
//! native resolution, and plan construction remain outside core.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PRESET_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPlanPreset {
    pub schema_version: u32,
    pub preset_id: String,
    pub label: String,
    pub description: String,
    pub required_capability_ids: Vec<String>,
    pub evidence_source: String,
    pub routing_mode: String,
    pub execution_mode: String,
    pub provider_traffic: String,
    pub writes_enabled: bool,
}

pub fn validate_workbench_plan_preset(preset: &WorkbenchPlanPreset) -> Result<()> {
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
    let mut seen = BTreeSet::new();
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

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.len() > 128
        || value.chars().any(char::is_control)
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.')
        })
    {
        bail!("Workbench {label} must be a bounded opaque identifier");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset() -> WorkbenchPlanPreset {
        WorkbenchPlanPreset {
            schema_version: PRESET_SCHEMA_VERSION,
            preset_id: "adapter-plan-review".into(),
            label: "Adapter plan review".into(),
            description: "Review a native plan-only adapter receipt.".into(),
            required_capability_ids: vec!["router_observe".into(), "client_adapter_plan".into()],
            evidence_source: "native_router_decision_receipt".into(),
            routing_mode: "observe_only".into(),
            execution_mode: "plan_only".into(),
            provider_traffic: "none".into(),
            writes_enabled: false,
        }
    }

    #[test]
    fn preset_wire_shape_and_validation_are_stable() {
        let value = serde_json::to_value(preset()).expect("serialize preset");
        assert_eq!(value["presetId"], "adapter-plan-review");
        assert!(value.get("executionEnabled").is_none());
        validate_workbench_plan_preset(&preset()).expect("valid preset");
    }

    #[test]
    fn preset_validation_rejects_execution_or_capability_drift() {
        let mut invalid = preset();
        invalid.execution_mode = "execute".into();
        assert!(validate_workbench_plan_preset(&invalid).is_err());

        let mut invalid = preset();
        invalid
            .required_capability_ids
            .push("arbitrary_shell".into());
        assert!(validate_workbench_plan_preset(&invalid).is_err());

        let mut invalid = preset();
        invalid.required_capability_ids = vec!["client_adapter_plan".into()];
        assert!(validate_workbench_plan_preset(&invalid).is_err());
    }

    #[test]
    fn preset_validation_preserves_replay_evidence_binding() {
        let mut invalid = preset();
        invalid
            .required_capability_ids
            .push("redacted_replay".into());
        assert!(validate_workbench_plan_preset(&invalid).is_err());
    }
}
