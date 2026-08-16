use crate::models::{ClientHealth, ClientStatus, SwitchboardMode};

use super::{
    adapter_status_for_listing, coding_client_adapter, coding_client_adapter_for_version,
    ConfigPlanAction, ConsentToken, CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
};

fn detected(client_id: &str, name: &str) -> ClientStatus {
    ClientStatus {
        id: client_id.to_string(),
        name: name.to_string(),
        installed: true,
        configured: false,
        health: ClientHealth::Attention,
        notes: vec![format!("Detected {name} without reading credentials.")],
    }
}

#[test]
fn first_adapter_cohort_is_registered_behind_one_contract() {
    for id in ["claude_code", "codex", "codex_cli", "gemini_cli"] {
        let adapter = coding_client_adapter(id).unwrap_or_else(|| panic!("missing {id} adapter"));
        assert!(matches!(
            adapter.id(),
            "claude_code" | "codex" | "gemini_cli"
        ));
        assert!(!adapter.footprint().secret_values_included);
    }
    assert!(coding_client_adapter("cursor").is_none());
}

#[test]
fn plan_is_a_deterministic_secret_free_diff_and_requires_exact_consent() {
    let adapter = coding_client_adapter("claude_code").expect("Claude adapter");
    let plan = adapter.plan(SwitchboardMode::Full).expect("Claude plan");
    let repeated = adapter.plan(SwitchboardMode::Full).expect("repeat plan");

    assert_eq!(plan, repeated);
    assert_eq!(plan.action, ConfigPlanAction::ApplyManagedRouting);
    assert!(plan.reversible);
    assert!(!plan.diffs.is_empty());
    assert!(plan
        .diffs
        .iter()
        .all(|diff| !diff.before.to_lowercase().contains("token=")));
    assert!(ConsentToken::issue(&plan, "wrong phrase").is_err());
    assert!(ConsentToken::issue(&plan, &plan.confirmation_phrase).is_ok());

    let off = adapter.plan(SwitchboardMode::Off).expect("Off plan");
    assert_eq!(off.action, ConfigPlanAction::CleanupManagedRouting);
    assert_ne!(off.plan_id, plan.plan_id);
}

#[test]
fn listing_status_exposes_structured_detection_plan_and_footprint() {
    for (id, name) in [
        ("claude_code", "Claude Code"),
        ("codex", "Codex"),
        ("gemini_cli", "Gemini CLI"),
    ] {
        let status = adapter_status_for_listing(id, &detected(id, name), false)
            .expect("listing status")
            .expect("registered adapter status");
        assert_eq!(status.adapter_id, id);
        assert_eq!(status.detection.client_id, id);
        assert!(status.detection.installed);
        assert!(status.detection.lifecycle_fixture_complete);
        assert_eq!(
            status.detection.contract_version,
            CODING_CLIENT_ADAPTER_CONTRACT_VERSION
        );
        assert_eq!(status.plan.client_id, id);
        assert!(status.verification.is_none());
        assert!(!status.footprint.secret_values_included);
    }
}

#[test]
fn contract_version_mismatch_is_rejected_before_connector_work() {
    assert!(coding_client_adapter_for_version(
        "claude_code",
        CODING_CLIENT_ADAPTER_CONTRACT_VERSION
    )
    .is_ok());
    let error = coding_client_adapter_for_version(
        "claude_code",
        CODING_CLIENT_ADAPTER_CONTRACT_VERSION + 1,
    )
    .err()
    .expect("version mismatch");
    assert!(error
        .to_string()
        .contains("Unsupported CodingClientAdapter contract version"));
}
