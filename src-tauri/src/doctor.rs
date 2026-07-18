use std::path::Path;

use chrono::{DateTime, Utc};

use crate::client_adapters;
use crate::codex_threads;
use crate::models::{DoctorIssue, DoctorReport, DoctorSeverity, RuntimeStatus, SwitchboardMode};
use crate::repo_intelligence;
use crate::state::AppState;
use crate::switchboard_commands::{switchboard_mode_label, switchboard_mode_wants_headroom};

mod connector_issues;

use connector_issues::{
    codex_routing_doctor_issue, planned_connector_doctor_body, unrouted_managed_connector_issues,
    unverified_managed_connector_issues,
};
pub(crate) fn doctor_repair_action_restores_headroom(action: &str) -> bool {
    matches!(
        action,
        "repair_runtime" | "repair_client_setups" | "repair_codex_setup" | "repair_all"
    ) || action.starts_with("repair_client_setup:")
}

pub(crate) fn switchboard_mode_blocks_doctor_repair(
    mode: Option<&SwitchboardMode>,
    action: &str,
) -> bool {
    !switchboard_mode_wants_headroom(mode) && doctor_repair_action_restores_headroom(action)
}

pub(crate) fn infer_switchboard_mode(
    runtime: &RuntimeStatus,
    enabled_client_count: usize,
) -> (SwitchboardMode, bool, bool) {
    let rtk_enabled = runtime.rtk.installed && runtime.rtk.enabled;
    let headroom_enabled =
        runtime.running && runtime.proxy_reachable && !runtime.paused && enabled_client_count > 0;
    let mode = match (headroom_enabled, rtk_enabled) {
        (true, true) => SwitchboardMode::Full,
        (true, false) => SwitchboardMode::Headroom,
        (false, true) => SwitchboardMode::Rtk,
        (false, false) => SwitchboardMode::Off,
    };

    (mode, rtk_enabled, headroom_enabled)
}

fn off_mode_violations(runtime: &RuntimeStatus, enabled_client_count: usize) -> Vec<&'static str> {
    let mut violations = Vec::new();
    if runtime.running || runtime.proxy_reachable {
        violations.push("Headroom engine is still reachable");
    }
    if enabled_client_count > 0 {
        violations.push("managed clients are still routed");
    }
    if runtime.rtk.installed && runtime.rtk.enabled {
        violations.push("RTK is still enabled");
    }
    violations
}

fn push_off_mode_doctor_issue(
    issues: &mut Vec<DoctorIssue>,
    runtime: &RuntimeStatus,
    enabled_client_count: usize,
) {
    let violations = off_mode_violations(runtime, enabled_client_count);
    if violations.is_empty() {
        return;
    }

    issues.push(DoctorIssue {
        id: "off_mode_not_clean".to_string(),
        title: "Off mode still has active routing evidence".to_string(),
        body: format!(
            "Off mode requested, but {}. Disable routing or restart affected shells, then run Doctor again.",
            violations.join(", ")
        ),
        severity: DoctorSeverity::Warning,
        repair_action: Some("verify_off_mode".to_string()),
    });
}

fn repo_intelligence_saved_paths_missing(summary: &crate::models::RepoIntelligenceSummary) -> bool {
    let Some(metadata) = summary.index_metadata.as_ref() else {
        return false;
    };
    if metadata.file_fingerprints.is_empty() {
        return false;
    }
    let repo_root = Path::new(&summary.repo_root);
    metadata
        .file_fingerprints
        .iter()
        .all(|entry| !repo_root.join(&entry.path).exists())
}

pub(crate) fn repo_intelligence_doctor_issue(
    summary: &crate::models::RepoIntelligenceSummary,
    now: DateTime<Utc>,
) -> Option<DoctorIssue> {
    if !Path::new(&summary.repo_root).is_dir() {
        return Some(DoctorIssue {
            id: "repo_intelligence_repo_missing".to_string(),
            title: "Repo Intelligence index points to a missing folder".to_string(),
            body: format!(
                "The last indexed repo path is no longer available: {}. Repair will clear this saved index; then re-index an available local repository from the Repo Intelligence add-on card.",
                summary.repo_root
            ),
            severity: DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        });
    }

    if repo_intelligence_saved_paths_missing(summary) {
        return Some(DoctorIssue {
            id: "repo_intelligence_repo_moved".to_string(),
            title: "Repo Intelligence index no longer matches this folder".to_string(),
            body: format!(
                "The saved Repo Intelligence file map no longer matches files under {}. The repo may have moved, been replaced, or been cleaned. Repair will clear this saved index; then re-index the current local repository before copying packs or agent handoffs.",
                summary.repo_root
            ),
            severity: DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        });
    }

    let freshness = repo_intelligence::build_index_freshness_response(Some(summary));
    let indexer_health = if freshness.indexer_version.as_deref()
        == Some(repo_intelligence::current_indexer_version())
    {
        "current"
    } else {
        "version_mismatch"
    };
    if freshness.parser_health == "version_mismatch"
        || freshness.index_health == "metadata_missing"
        || indexer_health == "version_mismatch"
    {
        return Some(DoctorIssue {
            id: "repo_intelligence_index_health".to_string(),
            title: "Repo Intelligence parser/index health needs refresh".to_string(),
            body: format!(
                "The saved Repo Intelligence index for {} reports index health '{}', parser health '{}', and indexer health '{}'. Repair will clear this saved index; then re-index the current local repository so Doctor and agent handoffs use the current parser/index contract.",
                summary.repo_root, freshness.index_health, freshness.parser_health, indexer_health
            ),
            severity: DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        });
    }

    let stale = DateTime::parse_from_rfc3339(&summary.indexed_at)
        .map(|indexed_at| {
            now.signed_duration_since(indexed_at.with_timezone(&Utc))
                .num_days()
                >= 7
        })
        .unwrap_or(false);
    if stale {
        return Some(DoctorIssue {
            id: "repo_intelligence_stale".to_string(),
            title: "Repo Intelligence index is stale".to_string(),
            body: format!(
                "The last Repo Intelligence index for {} is more than 7 days old. Repair will clear the stale saved index; then re-index it before relying on context packs for agent handoff.",
                summary.repo_root
            ),
            severity: DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        });
    }

    None
}

fn repo_memory_mcp_doctor_issue(runtime: &RuntimeStatus) -> Option<DoctorIssue> {
    if runtime.repo_memory_mcp_configured == Some(false) {
        return Some(DoctorIssue {
            id: "repo_memory_mcp_not_configured".to_string(),
            title: "Repo Memory MCP is not configured".to_string(),
            body: runtime
                .repo_memory_mcp_error
                .clone()
                .unwrap_or_else(|| {
                    "Repo Memory MCP is required before supported agents can request read-only Repo Intelligence packs through MCP. Repair will install the app-managed read-only repo-memory server, then run the start/smoke check before marking it active.".to_string()
                }),
            severity: DoctorSeverity::Warning,
            repair_action: Some("install_repo_memory_mcp".to_string()),
        });
    }

    let (id, title, body) = match runtime.repo_memory_mcp_supervision_status.as_str() {
        "smoke_failed" => (
            "repo_memory_mcp_smoke_failed",
            "Repo Memory MCP smoke check failed",
            "Repo Memory MCP is configured, but the read-only smoke check failed. Repair will reinstall the app-managed descriptor, start the MCP session, and re-run the smoke contract before supported agents rely on repo context.",
        ),
        "stale_config" => (
            "repo_memory_mcp_stale_config",
            "Repo Memory MCP config is stale",
            "Repo Memory MCP was marked active, but the app-managed MCP descriptor is missing or unsafe. Repair will restore the read-only descriptor and re-run the start/smoke check.",
        ),
        "service_unhealthy" => (
            "repo_memory_mcp_service_unhealthy",
            "Repo Memory MCP service is unhealthy",
            "Repo Memory MCP is configured, but the current descriptor, script, or Node runtime evidence is not healthy. Repair will restore the app-managed read-only descriptor and re-run the start/smoke check.",
        ),
        "restart_required" | "unknown_active" | "active" => (
            "repo_memory_mcp_needs_verification",
            "Repo Memory MCP needs verification",
            "Repo Memory MCP has active session state without current app-process smoke proof. Repair will run the app-managed Prepare MCP flow before agents consume repo-memory tools.",
        ),
        _ => return None,
    };

    Some(DoctorIssue {
        id: id.to_string(),
        title: title.to_string(),
        body: body.to_string(),
        severity: DoctorSeverity::Warning,
        repair_action: Some("install_repo_memory_mcp".to_string()),
    })
}

pub(crate) fn build_doctor_report(state: &AppState) -> DoctorReport {
    let runtime = state.runtime_status();
    let codex_direct_bypass = state
        .codex_bypass
        .load(std::sync::atomic::Ordering::Acquire);
    let mut issues = Vec::new();
    if crate::proxy_intercept::latest_provider_auth_error().as_deref()
        == Some("provider_auth_scope_missing")
    {
        issues.push(DoctorIssue {
            id: "provider_auth_scope_missing".to_string(),
            title: "Provider authorization is missing Responses: Write".to_string(),
            body: "The upstream provider rejected a Codex Responses request because the active credential or its organization/project authorization is missing api.responses.write. This is not a token-compression failure. Use a project key with Responses: Write or ChatGPT/Codex OAuth; Switchboard will not read, print, or change your secret.".to_string(),
            severity: DoctorSeverity::Error,
            repair_action: None,
        });
    }
    if state.tool_manager.caveman_receipt_exists() {
        let caveman_level = state.tool_manager.caveman_level();
        match client_adapters::caveman_integration_matches_level(&caveman_level) {
            Ok(false) => issues.push(DoctorIssue {
                id: "caveman_profile_mismatch".to_string(),
                title: "Caveman profile is not active in agent guidance".to_string(),
                body: format!(
                    "Caveman is installed at `{caveman_level}`, but the managed Claude/Codex guidance does not match that profile. Use the Addons Caveman level control or Doctor repair before relying on Compact Chinese or Caveman savings in this session."
                ),
                severity: DoctorSeverity::Warning,
                repair_action: Some("set_caveman_level".to_string()),
            }),
            Err(err) => issues.push(DoctorIssue {
                id: "caveman_profile_check_failed".to_string(),
                title: "Caveman profile could not be verified".to_string(),
                body: format!(
                    "Caveman is installed, but Switchboard could not verify the managed Claude/Codex guidance: {err}"
                ),
                severity: DoctorSeverity::Warning,
                repair_action: Some("set_caveman_level".to_string()),
            }),
            Ok(true) => {}
        }
    }
    let connectors =
        client_adapters::list_client_connectors(&state.cached_clients()).unwrap_or_default();
    let managed_connectors = connectors.iter().filter(|client| {
        matches!(
            client.support_status,
            crate::models::ClientConnectorSupportStatus::Managed
        )
    });
    let enabled_clients = managed_connectors
        .clone()
        .filter(|client| client.enabled)
        .count();
    let installed_clients = managed_connectors.filter(|client| client.installed).count();
    let planned_installed = connectors
        .iter()
        .filter(|client| {
            client.installed
                && matches!(
                    client.support_status,
                    crate::models::ClientConnectorSupportStatus::Planned
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    let (inferred_mode, _rtk_ready, _headroom_ready) =
        infer_switchboard_mode(&runtime, enabled_clients);
    let desired_mode = client_adapters::load_switchboard_mode().unwrap_or(inferred_mode.clone());

    if matches!(desired_mode, SwitchboardMode::Off) {
        push_off_mode_doctor_issue(&mut issues, &runtime, enabled_clients);
    }

    if let Some(issue) = repo_memory_mcp_doctor_issue(&runtime) {
        issues.push(issue);
    }

    if desired_mode != inferred_mode {
        issues.push(DoctorIssue {
            id: "switchboard_mode_degraded".to_string(),
            title: "Requested optimization is degraded".to_string(),
            body: format!(
                "{} is requested, but {} is active. Doctor lists missing local pieces below; repair managed connector items, then keep only retained connector native-routing gates manual until their backup, verify, rollback, and Off cleanup evidence is promoted.",
                switchboard_mode_label(&desired_mode),
                switchboard_mode_label(&inferred_mode)
            ),
            severity: DoctorSeverity::Warning,
            repair_action: None,
        });
    }

    if matches!(
        desired_mode,
        SwitchboardMode::Full | SwitchboardMode::Headroom
    ) && runtime.installed
        && (!runtime.running || !runtime.proxy_reachable || runtime.auto_paused)
    {
        issues.push(DoctorIssue {
id: "headroom_runtime_unreachable".to_string(),
title: "Headroom runtime is not reachable".to_string(),
body: runtime
.startup_error_hint
.clone()
.or_else(|| runtime.startup_error.clone())
.unwrap_or_else(|| {
"The local proxy is not answering. Repair will restart the Headroom runtime and refresh switchboard status.".to_string()
}),
severity: DoctorSeverity::Error,
repair_action: Some("repair_runtime".to_string()),
});
    }

    if codex_direct_bypass
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
    {
        issues.push(DoctorIssue {
id: "codex_direct_bypass".to_string(),
                    title: "Codex is in fallback direct routing".to_string(),
                    body: "Codex is using direct routing after a fallback bypass. Oversized Codex turns now auto-route before Headroom refusal; reset this only after confirming the conversation is compact enough for optimized routing.".to_string(),
severity: DoctorSeverity::Warning,
repair_action: Some("reset_codex_bypass".to_string()),
});
    }

    if runtime.proxy_reachable && runtime.proxy_auth_status != "authenticated" {
        issues.push(DoctorIssue {
            id: "proxy_loopback_unauthenticated".to_string(),
            title: "Proxy is loopback-only, not authenticated".to_string(),
            body: format!(
                "The local proxy is bound to {} and rejects browser Origin/non-loopback Host requests, but managed clients do not yet send a per-session auth token. Treat localhost as local-process trust, not a security boundary.",
                runtime.proxy_bind_address
            ),
            severity: DoctorSeverity::Warning,
            repair_action: None,
        });
    }

    if matches!(
        desired_mode,
        SwitchboardMode::Full | SwitchboardMode::Headroom
    ) && runtime.running
        && runtime.proxy_reachable
        && runtime.kompress_enabled != Some(true)
    {
        issues.push(DoctorIssue {
            id: "headroom_native_compressor_unavailable".to_string(),
            title: "Headroom native ML compressor is not enabled".to_string(),
            body: "Headroom is reachable, but its native Kompress compressor has not reported as enabled. Provider routing remains live; Doctor recommends repairing or prefetching the managed runtime before treating native ML savings as measured.".to_string(),
            severity: DoctorSeverity::Warning,
            repair_action: Some("repair_runtime".to_string()),
        });
    }

    let codex_provider_block_matches =
        client_adapters::codex_provider_block_matches().unwrap_or(false);
    if let Some(issue) =
        codex_routing_doctor_issue(&connectors, &desired_mode, codex_provider_block_matches)
    {
        issues.push(issue);
    }
    issues.extend(unrouted_managed_connector_issues(
        &connectors,
        &desired_mode,
    ));

    if connectors
        .iter()
        .any(|client| client.client_id == "codex" && client.enabled)
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
    {
        let retagging = codex_threads::get_codex_thread_retagging_settings();
        if !matches!(
            retagging.codex_thread_retagging,
            crate::models::CodexThreadRetaggingMode::Enabled
        ) {
            issues.push(DoctorIssue {
                id: "codex_thread_retagging_opt_in_required".to_string(),
                title: "Codex history retagging needs consent".to_string(),
                body: "Codex is routed through Headroom, but Switchboard will not edit Codex SQLite history until retagging is explicitly enabled. History may appear split between native and Headroom providers; enable retagging only after reviewing the backup and restore notes.".to_string(),
                severity: DoctorSeverity::Warning,
                repair_action: None,
            });
        }
    }

    issues.extend(unverified_managed_connector_issues(&connectors));

    if !planned_installed.is_empty() {
        issues.push(DoctorIssue {
            id: "planned_connectors_detected".to_string(),
            title: "Gated coding tools detected".to_string(),
            body: planned_connector_doctor_body(&planned_installed),
            severity: DoctorSeverity::Warning,
            repair_action: None,
        });
    }

    if matches!(
        desired_mode,
        SwitchboardMode::Full | SwitchboardMode::Headroom
    ) && enabled_clients == 0
        && installed_clients == 0
    {
        issues.push(DoctorIssue {
id: "no_headroom_clients".to_string(),
title: "No clients are routed through Headroom".to_string(),
body: "No supported coding clients were detected yet. Install or open Codex, Claude Code, or a supported editor, then return to connect it.".to_string(),
severity: DoctorSeverity::Warning,
repair_action: None,
});
    }

    match repo_intelligence::load_latest_summary() {
        Ok(Some(summary)) => {
            if let Some(issue) = repo_intelligence_doctor_issue(&summary, Utc::now()) {
                issues.push(issue);
            }
        }
        Ok(None) => {}
        Err(err) => issues.push(DoctorIssue {
            id: "repo_intelligence_storage_corrupt".to_string(),
            title: "Repo Intelligence index cannot be read".to_string(),
            body: format!(
                "The saved Repo Intelligence index could not be parsed or read: {err}. Repair will clear the saved index; then re-index a local repository before copying packs or agent handoffs."
            ),
            severity: DoctorSeverity::Warning,
            repair_action: Some("clear_repo_intelligence_index".to_string()),
        }),
    }

    if matches!(desired_mode, SwitchboardMode::Full | SwitchboardMode::Rtk)
        && (!runtime.rtk.installed || !runtime.rtk.enabled)
    {
        issues.push(DoctorIssue {
id: "rtk_not_active".to_string(),
title: "RTK is not active".to_string(),
body: if runtime.rtk.installed {
"RTK is installed but turned off. Repair will enable local RTK shell compression.".to_string()
} else {
"RTK is required for the requested switchboard mode. Repair will install RTK into Headroom-managed storage and enable local shell compression.".to_string()
},
severity: DoctorSeverity::Warning,
repair_action: Some("repair_rtk_runtime".to_string()),
});
    }

    if matches!(desired_mode, SwitchboardMode::Full | SwitchboardMode::Rtk)
        && runtime.rtk.installed
        && runtime.rtk.enabled
        && (!runtime.rtk.path_configured || !runtime.rtk.hook_configured)
    {
        issues.push(DoctorIssue {
id: "rtk_integration_incomplete".to_string(),
title: "RTK integration is incomplete".to_string(),
body: "RTK is enabled, but its shell PATH export or Claude Code hook is missing. Repair will re-apply the local RTK integration.".to_string(),
severity: DoctorSeverity::Warning,
repair_action: Some("repair_rtk_integrations".to_string()),
});
    }

    let tools = state.tool_manager.list_tools();
    let tool_needs_repair = |id: &str| {
        tools.iter().find(|tool| tool.id == id).is_some_and(|tool| {
            !tool.enabled || !matches!(tool.status, crate::models::ToolStatus::Healthy)
        })
    };
    let caveman_level = state.tool_manager.caveman_level();
    let caveman_guidance_drifted = state.tool_manager.caveman_receipt_exists()
        && tools
            .iter()
            .find(|tool| tool.id == "caveman")
            .is_some_and(|tool| tool.enabled)
        && !client_adapters::caveman_integration_matches_level(&caveman_level).unwrap_or(false);
    if tool_needs_repair("caveman") || caveman_guidance_drifted {
        issues.push(DoctorIssue {
            id: "caveman_guidance_inactive".to_string(),
            title: "Caveman guidance is not active".to_string(),
            body: "Caveman should keep a managed guidance block in Claude Code and Codex instruction files. Repair will recreate its local receipt and rewrite the Switchboard-owned guidance block.".to_string(),
            severity: DoctorSeverity::Warning,
            repair_action: Some("repair_caveman_guidance".to_string()),
        });
    }
    if tool_needs_repair("ponytail") {
        issues.push(DoctorIssue {
            id: "ponytail_plugin_inactive".to_string(),
            title: "Ponytail plugin is not active".to_string(),
            body: "Ponytail should be registered with Claude Code or Codex when its add-on is enabled. Repair will re-run the plugin install for available local hosts.".to_string(),
            severity: DoctorSeverity::Warning,
            repair_action: Some("repair_ponytail_plugin".to_string()),
        });
    }

    if runtime.paused
        && !runtime.auto_paused
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
    {
        issues.push(DoctorIssue {
	id: "headroom_paused".to_string(),
	title: "Headroom engine is paused".to_string(),
	body: "The proxy is intentionally off. Use Full optimization or Headroom only to restart routing through the Headroom engine.".to_string(),
	severity: DoctorSeverity::Warning,
	repair_action: None,
	});
    }

    let status = if issues
        .iter()
        .any(|issue| matches!(issue.severity, DoctorSeverity::Error))
    {
        DoctorSeverity::Error
    } else if issues.is_empty() {
        DoctorSeverity::Ok
    } else {
        DoctorSeverity::Warning
    };

    let summary = match status {
        DoctorSeverity::Ok => {
            "No switchboard issues detected. Headroom and RTK look ready for normal use."
        }
        DoctorSeverity::Warning => "Doctor found switchboard items that may need attention.",
        DoctorSeverity::Error => "Doctor found a blocking switchboard issue.",
    }
    .to_string();

    DoctorReport {
        status,
        summary,
        issues,
    }
}

#[cfg(test)]
mod doctor_tests {
    use super::*;
    use crate::models::ClientConnectorStatus;
    use crate::models::ClientSetupResult;
    use crate::switchboard_commands::switchboard_mode_wants_rtk;

    #[test]
    fn planned_connector_doctor_body_includes_backend_metadata() {
        let body = planned_connector_doctor_body(&[ClientConnectorStatus {
            client_id: "gemini_cli".to_string(),
            name: "Gemini CLI".to_string(),
            support_status: crate::models::ClientConnectorSupportStatus::Planned,
            setup_phase: "guide".to_string(),
            setup_hint: "Manual guide only.".to_string(),
            category: "cli".to_string(),
            detection_sources: vec!["PATH: gemini".to_string(), "~/.gemini".to_string()],
            detection_evidence: vec!["Detected at /opt/homebrew/bin/gemini".to_string()],
            config_locations: vec!["~/.gemini".to_string()],
            automation_gates: vec![
                "Back up provider settings before any routing change.".to_string()
            ],
            manual_workflow: vec!["Use RTK-only mode for noisy output.".to_string()],
            config_creation_steps: vec![
                "Detect config surface".to_string(),
                "Show dry-run diff".to_string(),
                "Create backup".to_string(),
                "Apply with consent".to_string(),
                "Verify in Doctor".to_string(),
                "Rollback safely".to_string(),
                "Clean up in Off mode".to_string(),
            ],
            config_creation_step_details: Vec::new(),
            config_dry_run_preview: Some(crate::models::ClientConnectorConfigDryRunPreview {
                target: "/Users/test/.gemini".to_string(),
                marker: "mac-ai-switchboard:gemini_cli".to_string(),
                backup_path: "/Users/test/.gemini.mac-ai-switchboard.bak".to_string(),
                current_state: "No Switchboard-managed Gemini provider routing detected."
                    .to_string(),
                proposed_state:
                    "Add AI Switchboard local provider routing after explicit consent."
                        .to_string(),
                apply_blocked_reason:
                    "Gemini CLI automation is disabled until backup, verify, rollback, and Off cleanup gates pass."
                        .to_string(),
                rollback_preview:
                    "Restore the Gemini config backup or remove only the managed block.".to_string(),
                confirmation_phrase: "APPLY GEMINI CLI CONFIG".to_string(),
                writes: Vec::new(),
            }),
            automation_path: vec![
                crate::models::ClientConnectorAutomationStage {
                    id: "detect".to_string(),
                    label: "Detect config surface".to_string(),
                    status: "ready".to_string(),
                    evidence: "Gemini CLI has local detection evidence.".to_string(),
                },
                crate::models::ClientConnectorAutomationStage {
                    id: "dryRunDiff".to_string(),
                    label: "Show dry-run diff".to_string(),
                    status: "ready".to_string(),
                    evidence: "Blocked preview is ready.".to_string(),
                },
            ],
            installed: true,
            enabled: false,
            verified: false,
            setup_verification: None,
            last_configured_at: None,
        }]);

        assert!(body.contains("Gemini CLI detected"));
        assert!(body.contains("Backend checks: PATH: gemini, ~/.gemini."));
        assert!(body.contains("Config locations watched: ~/.gemini."));
        assert!(body.contains("Detection evidence: Detected at /opt/homebrew/bin/gemini."));
        assert!(body.contains("Manual workflow: Use RTK-only mode"));
        assert!(body.contains("automatic provider routing is not enabled for these tools yet"));
        assert!(body.contains("Why setup is gated"));
        assert!(body.contains("exact backups, consented apply, Doctor verification"));
        assert!(body.contains("Safe today: use RTK-only mode or Repo Intelligence packs"));
        assert!(body.contains("review provider and model settings manually"));
    }

    fn test_runtime_status(
        running: bool,
        proxy_reachable: bool,
        rtk_enabled: bool,
    ) -> RuntimeStatus {
        RuntimeStatus {
            platform: "macos".to_string(),
            support_tier: "full".to_string(),
            installed: true,
            running,
            starting: false,
            paused: false,
            auto_paused: false,
            proxy_reachable,
            proxy_bind_address: "127.0.0.1:6767".to_string(),
            proxy_auth_status: "loopback_validated_unauthenticated".to_string(),
            proxy_auth_detail: "Loopback-only test fixture.".to_string(),
            headroom_pid: if running { Some(42) } else { None },
            launch_agent_status: crate::models::LaunchAgentRuntimeStatus {
                installed: false,
                path: None,
                label: "com.tarunagarwal.mac-ai-switchboard".to_string(),
                loaded: Some(false),
                load_detail: Some(
                    "launchctl does not report test LaunchAgent as loaded.".to_string(),
                ),
                legacy_installed: false,
                legacy_path: None,
                legacy_label: "Headroom".to_string(),
                legacy_loaded: Some(false),
                legacy_load_detail: Some(
                    "launchctl does not report legacy test LaunchAgent as loaded.".to_string(),
                ),
            },
            backend_status: crate::models::BackendRuntimeStatus {
                reachable: running,
                bind_address: "127.0.0.1:6768".to_string(),
                port: 6768,
                default_port: 6768,
                fallback_range_start: 6769,
                fallback_range_end: 6790,
            },
            mcp_configured: None,
            mcp_error: None,
            repo_memory_mcp_configured: None,
            repo_memory_mcp_error: None,
            repo_memory_mcp_active: false,
            repo_memory_mcp_last_started_at: None,
            repo_memory_mcp_last_checked_at: None,
            repo_memory_mcp_supervision_status: "unknown".to_string(),
            repo_memory_mcp_service: None,
            ml_installed: None,
            kompress_enabled: None,
            headroom_learn_supported: true,
            headroom_learn_disabled_reason: None,
            startup_error: None,
            startup_error_hint: None,
            runtime_upgrade_failure: None,
            rtk: crate::models::RtkRuntimeStatus {
                installed: rtk_enabled,
                enabled: rtk_enabled,
                version: None,
                path_configured: false,
                hook_configured: false,
                total_commands: None,
                total_input: None,
                total_output: None,
                total_saved: None,
                avg_savings_pct: None,
                total_time_ms: None,
                avg_time_ms: None,
                daily: Vec::new(),
                command_families: Vec::new(),
            },
        }
    }

    fn test_connector_status(
        id: &str,
        name: &str,
        support_status: crate::models::ClientConnectorSupportStatus,
        installed: bool,
        enabled: bool,
        verified: bool,
    ) -> ClientConnectorStatus {
        ClientConnectorStatus {
            client_id: id.to_string(),
            name: name.to_string(),
            support_status,
            setup_phase: "managed".to_string(),
            setup_hint: "Automatic reversible setup.".to_string(),
            category: "cli".to_string(),
            detection_sources: vec!["test".to_string()],
            detection_evidence: vec!["detected".to_string()],
            config_locations: vec!["~/.config/test".to_string()],
            automation_gates: Vec::new(),
            manual_workflow: Vec::new(),
            config_creation_steps: Vec::new(),
            config_creation_step_details: Vec::new(),
            config_dry_run_preview: None,
            automation_path: Vec::new(),
            installed,
            enabled,
            verified,
            setup_verification: None,
            last_configured_at: None,
        }
    }

    fn test_client_setup_result(client_id: &str, verified: bool) -> ClientSetupResult {
        ClientSetupResult {
            client_id: client_id.to_string(),
            applied: true,
            already_configured: false,
            summary: "Client configuration updated to route through Headroom.".to_string(),
            changed_files: vec!["~/.zshrc".to_string()],
            backup_files: Vec::new(),
            next_steps: Vec::new(),
            verification: crate::models::ClientSetupVerification {
                client_id: client_id.to_string(),
                verified,
                proxy_reachable: true,
                checks: Vec::new(),
                failures: if verified {
                    Vec::new()
                } else {
                    vec![format!("{client_id} verification failed")]
                },
            },
        }
    }

    #[test]
    fn off_mode_violations_empty_when_runtime_clients_and_rtk_are_off() {
        let runtime = test_runtime_status(false, false, false);
        assert!(off_mode_violations(&runtime, 0).is_empty());
    }

    #[test]
    fn switchboard_mode_intent_disables_everything_in_off_mode() {
        assert!(!switchboard_mode_wants_headroom(Some(
            &SwitchboardMode::Off
        )));
        assert!(!switchboard_mode_wants_rtk(Some(&SwitchboardMode::Off)));
    }

    #[test]
    fn switchboard_mode_intent_keeps_rtk_only_without_headroom() {
        assert!(!switchboard_mode_wants_headroom(Some(
            &SwitchboardMode::Rtk
        )));
        assert!(switchboard_mode_wants_rtk(Some(&SwitchboardMode::Rtk)));
    }

    #[test]
    fn switchboard_mode_intent_defaults_to_full_optimization() {
        assert!(switchboard_mode_wants_headroom(None));
        assert!(switchboard_mode_wants_rtk(None));
    }

    #[test]
    fn off_mode_blocks_doctor_repairs_that_restore_headroom() {
        for action in [
            "repair_runtime",
            "repair_client_setups",
            "repair_client_setup:gemini_cli",
            "repair_codex_setup",
            "repair_all",
        ] {
            assert!(
                switchboard_mode_blocks_doctor_repair(Some(&SwitchboardMode::Off), action),
                "{action} should be blocked in Off mode"
            );
        }
    }

    #[test]
    fn rtk_only_blocks_doctor_repairs_that_restore_headroom() {
        for action in [
            "repair_runtime",
            "repair_client_setups",
            "repair_client_setup:opencode",
            "repair_codex_setup",
            "repair_all",
        ] {
            assert!(
                switchboard_mode_blocks_doctor_repair(Some(&SwitchboardMode::Rtk), action),
                "{action} should be blocked in RTK-only mode"
            );
        }
    }

    #[test]
    fn headroom_modes_allow_headroom_repair_actions() {
        for mode in [SwitchboardMode::Headroom, SwitchboardMode::Full] {
            for action in [
                "repair_runtime",
                "repair_client_setups",
                "repair_client_setup:windsurf",
                "repair_codex_setup",
                "repair_all",
            ] {
                assert!(
                    !switchboard_mode_blocks_doctor_repair(Some(&mode), action),
                    "{action} should be allowed in {}",
                    switchboard_mode_label(&mode)
                );
            }
        }
    }

    #[test]
    fn repair_all_actions_skip_duplicate_managed_client_repairs() {
        let issue = |id: &str, action: Option<&str>| DoctorIssue {
            id: id.to_string(),
            title: id.to_string(),
            body: id.to_string(),
            severity: DoctorSeverity::Warning,
            repair_action: action.map(str::to_string),
        };
        let report = DoctorReport {
            status: DoctorSeverity::Warning,
            summary: "repairable".to_string(),
            issues: vec![
                issue("runtime", Some("repair_runtime")),
                issue("all_clients", Some("repair_client_setups")),
                issue("gemini_a", Some("repair_client_setup:gemini_cli")),
                issue("gemini_b", Some("repair_client_setup:gemini_cli")),
                issue("opencode", Some("repair_client_setup:opencode")),
                issue("manual", None),
                issue("off_mode", Some("verify_off_mode")),
                issue("rtk_a", Some("repair_rtk_runtime")),
                issue("rtk_b", Some("repair_rtk_runtime")),
            ],
        };

        assert_eq!(
            crate::switchboard_commands::normalized_repair_all_actions(&report),
            vec![
                "repair_runtime".to_string(),
                "repair_client_setups".to_string(),
                "repair_rtk_runtime".to_string(),
            ],
        );
    }

    #[test]
    fn repair_all_failure_summary_reports_every_failed_action() {
        let failures = vec![
            "repair_client_setups: Gemini CLI verification failed".to_string(),
            "repair_rtk_runtime: rtk install failed".to_string(),
        ];

        let error = crate::switchboard_commands::summarize_doctor_repair_all_failures(&failures)
            .expect_err("failures should be reported");

        assert!(error.contains("repair_all completed with failures"));
        assert!(error.contains("repair_client_setups: Gemini CLI verification failed"));
        assert!(error.contains("repair_rtk_runtime: rtk install failed"));
    }

    #[test]
    fn repair_all_failure_summary_allows_successful_runs() {
        crate::switchboard_commands::summarize_doctor_repair_all_failures(&[])
            .expect("empty failures should pass");
    }

    #[test]
    fn doctor_client_repair_reports_failed_post_write_verification() {
        let result = ClientSetupResult {
            client_id: "gemini_cli".to_string(),
            applied: true,
            already_configured: false,
            summary: "Client configuration updated to route through Headroom.".to_string(),
            changed_files: vec!["~/.zshrc".to_string()],
            backup_files: Vec::new(),
            next_steps: Vec::new(),
            verification: crate::models::ClientSetupVerification {
                client_id: "gemini_cli".to_string(),
                verified: false,
                proxy_reachable: true,
                checks: Vec::new(),
                failures: vec![
                    "Gemini GEMINI_BASE_URL export was not found in shell profiles.".to_string(),
                    "Switchboard-managed Gemini CLI sidecar was not found.".to_string(),
                ],
            },
        };

        let error = crate::switchboard_commands::ensure_doctor_client_repair_verified(&result)
            .expect_err("Doctor repair should fail when post-write verification fails");

        assert!(error.contains("gemini_cli repair applied but verification still failed"));
        assert!(error.contains("GEMINI_BASE_URL export was not found"));
        assert!(error.contains("sidecar was not found"));
    }

    #[test]
    fn managed_client_batch_repair_attempts_all_installed_managed_connectors() {
        let connectors = vec![
            test_connector_status(
                "gemini_cli",
                "Gemini CLI",
                crate::models::ClientConnectorSupportStatus::Managed,
                true,
                false,
                false,
            ),
            test_connector_status(
                "opencode",
                "OpenCode",
                crate::models::ClientConnectorSupportStatus::Managed,
                true,
                false,
                false,
            ),
            test_connector_status(
                "cursor",
                "Cursor",
                crate::models::ClientConnectorSupportStatus::Planned,
                true,
                false,
                false,
            ),
        ];
        let mut attempted = Vec::new();

        let batch = crate::switchboard_commands::run_managed_client_repair_batch(
            &connectors,
            |connector| {
                attempted.push(connector.client_id.clone());
                if connector.client_id == "gemini_cli" {
                    Ok(test_client_setup_result(&connector.client_id, false))
                } else {
                    Ok(test_client_setup_result(&connector.client_id, true))
                }
            },
        )
        .expect("batch should run");

        assert_eq!(attempted, vec!["gemini_cli", "opencode"]);
        assert_eq!(batch.repaired, 1);
        assert_eq!(batch.failures.len(), 1);
        assert!(batch.failures[0].contains("Gemini CLI"));
        assert!(batch.failures[0].contains("verification failed"));

        let error = crate::switchboard_commands::summarize_managed_client_repair_batch(&batch)
            .expect_err("partial failure should stay visible");
        assert!(error.contains("repaired 1 managed client(s)"));
        assert!(error.contains("Gemini CLI"));
    }

    #[test]
    fn managed_client_batch_repair_reports_no_installed_supported_clients() {
        let connectors = vec![test_connector_status(
            "cursor",
            "Cursor",
            crate::models::ClientConnectorSupportStatus::Planned,
            true,
            false,
            false,
        )];

        let error = crate::switchboard_commands::run_managed_client_repair_batch(
            &connectors,
            |_connector| Ok(test_client_setup_result("cursor", true)),
        )
        .expect_err("no managed clients should fail");

        assert_eq!(error, "no installed supported clients found to repair");
    }

    #[test]
    fn non_headroom_doctor_repairs_remain_available_in_off_and_rtk_modes() {
        for mode in [SwitchboardMode::Off, SwitchboardMode::Rtk] {
            for action in [
                "verify_off_mode",
                "reset_codex_bypass",
                "repair_rtk_integrations",
                "repair_rtk_runtime",
                "repair_caveman_guidance",
                "repair_ponytail_plugin",
                "clear_repo_intelligence_index",
                "install_repo_memory_mcp",
            ] {
                assert!(
                    !switchboard_mode_blocks_doctor_repair(Some(&mode), action),
                    "{action} should remain available in {}",
                    switchboard_mode_label(&mode)
                );
            }
        }
    }

    #[test]
    fn off_mode_doctor_issue_lists_active_routing_evidence() {
        let runtime = test_runtime_status(true, true, true);
        let mut issues = Vec::new();

        push_off_mode_doctor_issue(&mut issues, &runtime, 2);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "off_mode_not_clean");
        assert!(issues[0]
            .body
            .contains("Headroom engine is still reachable"));
        assert!(issues[0].body.contains("managed clients are still routed"));
        assert!(issues[0].body.contains("RTK is still enabled"));
        assert!(matches!(issues[0].severity, DoctorSeverity::Warning));
        assert_eq!(issues[0].repair_action.as_deref(), Some("verify_off_mode"));
    }

    #[test]
    fn repo_memory_mcp_doctor_issue_is_repairable_when_unconfigured() {
        let mut runtime = test_runtime_status(true, true, true);
        runtime.mcp_configured = Some(true);
        runtime.mcp_error = None;
        runtime.repo_memory_mcp_configured = Some(false);
        runtime.repo_memory_mcp_error =
            Some("repo-memory missing from Claude MCP config".to_string());

        let issue = repo_memory_mcp_doctor_issue(&runtime).expect("repo memory issue");

        assert_eq!(issue.id, "repo_memory_mcp_not_configured");
        assert_eq!(
            issue.repair_action.as_deref(),
            Some("install_repo_memory_mcp")
        );
        assert!(issue.body.contains("repo-memory missing"));

        runtime.repo_memory_mcp_configured = Some(true);
        assert!(repo_memory_mcp_doctor_issue(&runtime).is_none());
    }

    #[test]
    fn repo_memory_mcp_doctor_issue_surfaces_failed_supervision() {
        let mut runtime = test_runtime_status(true, true, true);
        runtime.mcp_configured = Some(true);
        runtime.repo_memory_mcp_configured = Some(true);

        runtime.repo_memory_mcp_supervision_status = "smoke_failed".to_string();
        let smoke_issue = repo_memory_mcp_doctor_issue(&runtime).expect("smoke issue");
        assert_eq!(smoke_issue.id, "repo_memory_mcp_smoke_failed");
        assert_eq!(
            smoke_issue.repair_action.as_deref(),
            Some("install_repo_memory_mcp")
        );
        assert!(smoke_issue.body.contains("read-only smoke check failed"));

        runtime.repo_memory_mcp_supervision_status = "stale_config".to_string();
        let stale_issue = repo_memory_mcp_doctor_issue(&runtime).expect("stale issue");
        assert_eq!(stale_issue.id, "repo_memory_mcp_stale_config");
        assert!(stale_issue.body.contains("descriptor is missing or unsafe"));

        runtime.repo_memory_mcp_supervision_status = "service_unhealthy".to_string();
        let service_issue = repo_memory_mcp_doctor_issue(&runtime).expect("service issue");
        assert_eq!(service_issue.id, "repo_memory_mcp_service_unhealthy");
        assert!(service_issue.body.contains("descriptor, script, or Node"));
        assert_eq!(
            service_issue.repair_action.as_deref(),
            Some("install_repo_memory_mcp")
        );

        runtime.repo_memory_mcp_supervision_status = "active".to_string();
        let active_issue = repo_memory_mcp_doctor_issue(&runtime).expect("active issue");
        assert_eq!(active_issue.id, "repo_memory_mcp_needs_verification");
        assert!(active_issue
            .body
            .contains("without current app-process smoke proof"));

        runtime.repo_memory_mcp_supervision_status = "verified_active".to_string();
        assert!(repo_memory_mcp_doctor_issue(&runtime).is_none());
    }

    #[test]
    fn codex_routing_issue_repairs_detected_but_unrouted_codex() {
        let connectors = vec![test_connector_status(
            "codex",
            "Codex",
            crate::models::ClientConnectorSupportStatus::Managed,
            true,
            false,
            false,
        )];

        let issue = codex_routing_doctor_issue(&connectors, &SwitchboardMode::Full, false)
            .expect("unrouted Codex should be repairable");

        assert_eq!(issue.id, "codex_provider_mismatch");
        assert_eq!(issue.repair_action.as_deref(), Some("repair_codex_setup"));
        assert!(issue.body.contains("Codex routing is repair ready"));
        assert!(issue.body.contains("reversible managed setup"));
        assert!(issue.body.contains("OPENAI_BASE_URL"));
    }

    #[test]
    fn unrouted_managed_connector_issue_repairs_second_direct_tool() {
        let connectors = vec![
            test_connector_status(
                "codex",
                "Codex",
                crate::models::ClientConnectorSupportStatus::Managed,
                true,
                true,
                true,
            ),
            test_connector_status(
                "gemini_cli",
                "Gemini CLI",
                crate::models::ClientConnectorSupportStatus::Managed,
                true,
                false,
                false,
            ),
        ];

        let issues = unrouted_managed_connector_issues(&connectors, &SwitchboardMode::Full);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "gemini_cli_routing_not_configured");
        assert_eq!(issues[0].title, "Gemini CLI routing is repair ready");
        assert_eq!(
            issues[0].repair_action.as_deref(),
            Some("repair_client_setup:gemini_cli")
        );
        assert!(issues[0].body.contains("this managed client setup"));
        assert!(issues[0].body.contains("preserve user-owned config"));
    }

    #[test]
    fn unrouted_managed_sidecar_connector_issue_repairs_amazon_q() {
        let connectors = vec![test_connector_status(
            "amazon_q",
            "Amazon Q Developer CLI",
            crate::models::ClientConnectorSupportStatus::Managed,
            true,
            false,
            false,
        )];

        let issues = unrouted_managed_connector_issues(&connectors, &SwitchboardMode::Full);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "amazon_q_routing_not_configured");
        assert_eq!(
            issues[0].repair_action.as_deref(),
            Some("repair_client_setup:amazon_q")
        );
        assert!(issues[0].body.contains("this managed client setup"));
    }

    #[test]
    fn unrouted_planned_connector_stays_manual() {
        let connectors = vec![test_connector_status(
            "aider",
            "Aider",
            crate::models::ClientConnectorSupportStatus::Planned,
            true,
            false,
            false,
        )];

        let issues = unrouted_managed_connector_issues(&connectors, &SwitchboardMode::Full);

        assert!(issues.is_empty());
    }

    #[test]
    fn unverified_managed_connector_issue_repairs_only_that_tool() {
        let connectors = vec![
            test_connector_status(
                "gemini_cli",
                "Gemini CLI",
                crate::models::ClientConnectorSupportStatus::Managed,
                true,
                true,
                false,
            ),
            test_connector_status(
                "opencode",
                "OpenCode",
                crate::models::ClientConnectorSupportStatus::Managed,
                true,
                true,
                true,
            ),
        ];

        let issues = unverified_managed_connector_issues(&connectors);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "gemini_cli_routing_config_mismatch");
        assert_eq!(
            issues[0].repair_action.as_deref(),
            Some("repair_client_setup:gemini_cli")
        );
        assert!(issues[0].body.contains("no longer verifies"));
    }

    #[test]
    fn unverified_planned_connector_stays_manual() {
        let connectors = vec![test_connector_status(
            "cursor",
            "Cursor",
            crate::models::ClientConnectorSupportStatus::Planned,
            true,
            true,
            false,
        )];

        let issues = unverified_managed_connector_issues(&connectors);

        assert!(issues.is_empty());
    }
}
