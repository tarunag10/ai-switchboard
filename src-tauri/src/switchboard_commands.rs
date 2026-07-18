use serde_json::json;
use tauri::{AppHandle, Manager, State};

use crate::models::{
    ClientConnectorStatus, ClientSetupResult, DoctorReport, SavingsMode, SwitchboardMode,
    SwitchboardState,
};
use crate::state::AppState;
use crate::{analytics, client_adapters, doctor, local_mode, optimization, repo_intelligence};

pub(crate) fn switchboard_mode_label(mode: &SwitchboardMode) -> &'static str {
    match mode {
        SwitchboardMode::Off => "Off",
        SwitchboardMode::Rtk => "RTK only",
        SwitchboardMode::Headroom => "Headroom only",
        SwitchboardMode::Full => "Full optimization",
    }
}

pub(crate) fn saved_switchboard_mode_wants_headroom() -> bool {
    switchboard_mode_wants_headroom(client_adapters::load_switchboard_mode().as_ref())
}

pub(crate) fn saved_switchboard_mode_wants_rtk() -> bool {
    switchboard_mode_wants_rtk(client_adapters::load_switchboard_mode().as_ref())
}

pub(crate) fn switchboard_mode_wants_headroom(mode: Option<&SwitchboardMode>) -> bool {
    matches!(
        mode,
        Some(SwitchboardMode::Headroom | SwitchboardMode::Full) | None
    )
}

pub(crate) fn switchboard_mode_wants_rtk(mode: Option<&SwitchboardMode>) -> bool {
    matches!(
        mode,
        Some(SwitchboardMode::Rtk | SwitchboardMode::Full) | None
    )
}

fn build_switchboard_state(state: &AppState) -> Result<SwitchboardState, String> {
    let runtime = state.runtime_status();
    let clients = client_adapters::list_client_connectors(&state.cached_clients())
        .map_err(|err| err.to_string())?;
    let enabled_clients: Vec<ClientConnectorStatus> = clients
        .iter()
        .filter(|client| client.enabled)
        .cloned()
        .collect();
    let (inferred_mode, rtk_enabled, headroom_enabled) =
        doctor::infer_switchboard_mode(&runtime, enabled_clients.len());
    let desired_mode = client_adapters::load_switchboard_mode().unwrap_or(inferred_mode.clone());
    let savings_mode = client_adapters::load_savings_mode();
    let effective_mode = inferred_mode;
    let needs_attention = desired_mode != effective_mode;
    let codex_direct_bypass = state
        .codex_bypass
        .load(std::sync::atomic::Ordering::Acquire);
    let summary = if needs_attention {
        format!(
            "{} requested, but {} is currently active. Run Doctor to repair the missing local pieces.",
            switchboard_mode_label(&desired_mode),
            switchboard_mode_label(&effective_mode)
        )
    } else if codex_direct_bypass
        && matches!(
            desired_mode,
            SwitchboardMode::Full | SwitchboardMode::Headroom
        )
    {
        "Codex is in fallback direct routing. Oversized turns auto-route before Headroom refusal; reset only after confirming the conversation is compact enough for optimized routing."
            .to_string()
    } else {
        match desired_mode {
            SwitchboardMode::Full => {
                "Headroom proxy routing and RTK command compression are both active."
            }
            SwitchboardMode::Headroom => {
                "LLM traffic is routed through Headroom. RTK command compression is off."
            }
            SwitchboardMode::Rtk => {
                "RTK command compression is active. No coding client is routed through Headroom."
            }
            SwitchboardMode::Off => "No optimization layer is active right now.",
        }
        .to_string()
    };
    let local_only = local_mode::enabled();

    Ok(SwitchboardState {
        mode: desired_mode.clone(),
        desired_mode,
        effective_mode,
        savings_mode,
        needs_attention,
        local_only,
        remote_services_enabled: !local_only,
        runtime,
        clients,
        enabled_clients,
        rtk_enabled,
        headroom_enabled,
        summary,
    })
}

#[tauri::command]
pub async fn get_switchboard_state(state: State<'_, AppState>) -> Result<SwitchboardState, String> {
    build_switchboard_state(&state)
}

#[tauri::command]
pub fn get_doctor_report(state: State<'_, AppState>) -> DoctorReport {
    doctor::build_doctor_report(&state)
}

fn repair_runtime(state: &AppState) -> Result<(), String> {
    state.stop_headroom();
    state.set_runtime_auto_paused(false);
    state.resume_runtime().map_err(|err| err.to_string())?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

pub(crate) fn ensure_doctor_client_repair_verified(
    result: &ClientSetupResult,
) -> Result<(), String> {
    if result.verification.verified {
        return Ok(());
    }

    let details = if result.verification.failures.is_empty() {
        "post-repair verification returned no passing evidence".to_string()
    } else {
        result.verification.failures.join("; ")
    };
    Err(format!(
        "{} repair applied but verification still failed: {details}",
        result.client_id
    ))
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ManagedClientRepairBatch {
    pub(crate) repaired: usize,
    pub(crate) failures: Vec<String>,
}

pub(crate) fn summarize_managed_client_repair_batch(
    batch: &ManagedClientRepairBatch,
) -> Result<(), String> {
    if batch.repaired == 0 && batch.failures.is_empty() {
        return Err("no installed supported clients found to repair".to_string());
    }
    if batch.failures.is_empty() {
        return Ok(());
    }

    let prefix = if batch.repaired == 0 {
        "managed client repair failed".to_string()
    } else {
        format!(
            "repaired {} managed client(s), but some repairs failed",
            batch.repaired
        )
    };
    Err(format!("{prefix}: {}", batch.failures.join(" | ")))
}

pub(crate) fn run_managed_client_repair_batch<F>(
    connectors: &[ClientConnectorStatus],
    mut repair: F,
) -> Result<ManagedClientRepairBatch, String>
where
    F: FnMut(&ClientConnectorStatus) -> Result<ClientSetupResult, String>,
{
    let mut batch = ManagedClientRepairBatch {
        repaired: 0,
        failures: Vec::new(),
    };
    let mut saw_installed_managed = false;

    for connector in connectors.iter().filter(|connector| {
        connector.installed
            && matches!(
                connector.support_status,
                crate::models::ClientConnectorSupportStatus::Managed
            )
    }) {
        saw_installed_managed = true;
        match repair(connector).and_then(|result| {
            ensure_doctor_client_repair_verified(&result)?;
            Ok(result)
        }) {
            Ok(_) => batch.repaired += 1,
            Err(err) => batch.failures.push(format!("{}: {}", connector.name, err)),
        }
    }

    if !saw_installed_managed {
        return Err("no installed supported clients found to repair".to_string());
    }

    Ok(batch)
}

fn repair_client_setups(state: &AppState) -> Result<(), String> {
    state
        .codex_bypass
        .store(false, std::sync::atomic::Ordering::Release);
    state.resume_runtime().map_err(|err| err.to_string())?;
    let connectors = client_adapters::list_client_connectors(&state.cached_clients())
        .map_err(|err| err.to_string())?;
    let batch = run_managed_client_repair_batch(&connectors, |connector| {
        client_adapters::apply_client_setup(&connector.client_id).map_err(|err| err.to_string())
    })?;
    if batch.repaired > 0 {
        state.invalidate_runtime_status_cache();
    }
    summarize_managed_client_repair_batch(&batch)
}

fn repair_managed_client_setup(state: &AppState, client_id: &str) -> Result<(), String> {
    if client_id.trim().is_empty() {
        return Err("client id is required for managed client repair".to_string());
    }
    state
        .codex_bypass
        .store(false, std::sync::atomic::Ordering::Release);
    state.resume_runtime().map_err(|err| err.to_string())?;
    let connectors = client_adapters::list_client_connectors(&state.cached_clients())
        .map_err(|err| err.to_string())?;
    let connector = connectors
        .iter()
        .find(|connector| connector.client_id == client_id)
        .ok_or_else(|| format!("managed client not found: {client_id}"))?;
    if !connector.installed {
        return Err(format!("{} is not installed", connector.name));
    }
    if !matches!(
        connector.support_status,
        crate::models::ClientConnectorSupportStatus::Managed
    ) {
        return Err(format!("{} is not a managed connector", connector.name));
    }
    let result =
        client_adapters::apply_client_setup(&connector.client_id).map_err(|err| err.to_string())?;
    ensure_doctor_client_repair_verified(&result)?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

fn repair_codex_setup(state: &AppState) -> Result<(), String> {
    state
        .codex_bypass
        .store(false, std::sync::atomic::Ordering::Release);
    state.resume_runtime().map_err(|err| err.to_string())?;
    let result = client_adapters::apply_client_setup("codex").map_err(|err| err.to_string())?;
    ensure_doctor_client_repair_verified(&result)?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

fn repair_rtk_integrations(state: &AppState) -> Result<(), String> {
    client_adapters::set_rtk_enabled(
        true,
        &state.tool_manager.rtk_entrypoint(),
        &state.tool_manager.managed_python(),
    )
    .map_err(|err| err.to_string())?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

fn repair_rtk_runtime(state: &AppState) -> Result<(), String> {
    if !state.tool_manager.rtk_installed() {
        state
            .tool_manager
            .install_rtk()
            .map_err(|err| err.to_string())?;
    }
    repair_rtk_integrations(state)
}

fn repair_caveman_guidance(state: &AppState) -> Result<(), String> {
    if !state.tool_manager.caveman_receipt_exists() {
        state
            .tool_manager
            .install_caveman()
            .map_err(|err| err.to_string())?;
    } else {
        state
            .tool_manager
            .set_caveman_enabled(true)
            .map_err(|err| err.to_string())?;
    }
    client_adapters::enable_caveman_integration(&state.tool_manager.caveman_level())
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn repair_ponytail_plugin(state: &AppState) -> Result<(), String> {
    if state.tool_manager.list_tools().iter().any(|tool| {
        tool.id == "ponytail" && !matches!(tool.status, crate::models::ToolStatus::Healthy)
    }) {
        state
            .tool_manager
            .install_ponytail()
            .map_err(|err| err.to_string())?;
    } else {
        state
            .tool_manager
            .set_ponytail_enabled(true)
            .map_err(|err| err.to_string())?;
    }
    let hosts = state.tool_manager.ponytail_registered_hosts();
    let _ = state.record_ponytail_attribution(&hosts);
    Ok(())
}

pub(crate) fn clear_repo_intelligence_index() -> Result<(), String> {
    repo_intelligence::clear_latest_summary()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn repair_repo_memory_mcp(state: &AppState) -> Result<(), String> {
    state
        .tool_manager
        .install_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    state
        .start_repo_memory_mcp()
        .map_err(|err| err.to_string())?;
    state.invalidate_runtime_status_cache();
    Ok(())
}

pub(crate) fn normalized_repair_all_actions(report: &DoctorReport) -> Vec<String> {
    let has_all_client_repair = report
        .issues
        .iter()
        .any(|issue| issue.repair_action.as_deref() == Some("repair_client_setups"));
    let mut actions = Vec::new();
    let mut repaired_client_ids = Vec::new();

    for action in report
        .issues
        .iter()
        .filter_map(|issue| issue.repair_action.as_deref())
    {
        if action == "verify_off_mode" {
            continue;
        }

        if action.starts_with("repair_client_setup:") {
            if has_all_client_repair {
                continue;
            }
            let client_id = action
                .strip_prefix("repair_client_setup:")
                .unwrap_or_default()
                .to_string();
            if repaired_client_ids.iter().any(|seen| seen == &client_id) {
                continue;
            }
            repaired_client_ids.push(client_id);
        }

        if actions.iter().any(|seen| seen == action) {
            continue;
        }
        actions.push(action.to_string());
    }

    actions
}

pub(crate) fn summarize_doctor_repair_all_failures(failures: &[String]) -> Result<(), String> {
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "repair_all completed with failures: {}",
            failures.join(" | ")
        ))
    }
}

fn run_single_doctor_repair_action(state: &AppState, action: &str) -> Result<(), String> {
    match action {
        "reset_codex_bypass" => {
            state
                .codex_bypass
                .store(false, std::sync::atomic::Ordering::Release);
            state.invalidate_runtime_status_cache();
            Ok(())
        }
        "repair_runtime" => repair_runtime(state),
        "repair_client_setups" => repair_client_setups(state),
        action if action.starts_with("repair_client_setup:") => {
            let client_id = action
                .strip_prefix("repair_client_setup:")
                .unwrap_or_default();
            repair_managed_client_setup(state, client_id)
        }
        "repair_codex_setup" => repair_codex_setup(state),
        "repair_rtk_integrations" => repair_rtk_integrations(state),
        "repair_rtk_runtime" => repair_rtk_runtime(state),
        "repair_caveman_guidance" => repair_caveman_guidance(state),
        "repair_ponytail_plugin" => repair_ponytail_plugin(state),
        "clear_repo_intelligence_index" => clear_repo_intelligence_index(),
        "install_repo_memory_mcp" => repair_repo_memory_mcp(state),
        other => Err(format!("unknown doctor repair action: {other}")),
    }
}

#[tauri::command]
pub async fn run_doctor_repair(
    state: State<'_, AppState>,
    action: String,
) -> Result<DoctorReport, String> {
    let saved_mode = client_adapters::load_switchboard_mode();
    if doctor::switchboard_mode_blocks_doctor_repair(saved_mode.as_ref(), action.as_str()) {
        let mode_label = saved_mode
            .as_ref()
            .map(switchboard_mode_label)
            .unwrap_or("current mode");
        return Err(format!(
            "{mode_label} is requested, so Doctor will not run {action} because it can restore Headroom routing. Choose Headroom only or Full optimization first."
        ));
    }

    match action.as_str() {
        "verify_off_mode" => Ok(doctor::build_doctor_report(&state)),
        "repair_all" => {
            let report = doctor::build_doctor_report(&state);
            let mut failures = Vec::new();
            for action in normalized_repair_all_actions(&report) {
                if let Err(err) = run_single_doctor_repair_action(&state, &action) {
                    failures.push(format!("{action}: {err}"));
                }
            }
            summarize_doctor_repair_all_failures(&failures)?;
            Ok(doctor::build_doctor_report(&state))
        }
        other => {
            run_single_doctor_repair_action(&state, other)?;
            Ok(doctor::build_doctor_report(&state))
        }
    }
}

#[tauri::command]
pub async fn set_switchboard_mode(
    app: AppHandle,
    mode: SwitchboardMode,
) -> Result<SwitchboardState, String> {
    let state: tauri::State<'_, AppState> = app.state();
    client_adapters::write_switchboard_mode(mode.clone()).map_err(|err| err.to_string())?;
    if matches!(mode, SwitchboardMode::Full) {
        let full_policy = optimization::action_policy::OptimizationActionPolicy::default();
        optimization::action_policy::save_action_policy(&full_policy)?;
    }

    match mode {
        SwitchboardMode::Off => {
            client_adapters::set_rtk_enabled(
                false,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state.set_runtime_paused(true);
            state.set_runtime_auto_paused(false);
            state
                .codex_bypass
                .store(true, std::sync::atomic::Ordering::Release);
            state.stop_headroom();
            // Off means no optimization layer, including the separate local
            // exact-response cache. Keep the database for explicit user clear
            // and auditability, but do not serve cached responses in Off mode.
            state
                .semantic_cache
                .set_enabled(false)
                .map_err(|err| err.to_string())?;
            client_adapters::clear_client_setups().map_err(|err| err.to_string())?;
            analytics::track_event(&app, "switchboard_mode_off", None);
        }
        SwitchboardMode::Rtk => {
            client_adapters::set_rtk_enabled(
                true,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state.set_runtime_paused(true);
            state.set_runtime_auto_paused(false);
            state
                .codex_bypass
                .store(true, std::sync::atomic::Ordering::Release);
            state.stop_headroom();
            state
                .semantic_cache
                .set_enabled(false)
                .map_err(|err| err.to_string())?;
            client_adapters::clear_client_setups().map_err(|err| err.to_string())?;
            analytics::track_event(&app, "switchboard_mode_rtk", None);
        }
        SwitchboardMode::Headroom => {
            client_adapters::set_rtk_enabled(
                false,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state
                .codex_bypass
                .store(false, std::sync::atomic::Ordering::Release);
            state.resume_runtime().map_err(|err| err.to_string())?;
            client_adapters::restore_client_setups();
            analytics::track_event(&app, "switchboard_mode_headroom", None);
        }
        SwitchboardMode::Full => {
            client_adapters::set_rtk_enabled(
                true,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|err| err.to_string())?;
            state
                .codex_bypass
                .store(false, std::sync::atomic::Ordering::Release);
            state.resume_runtime().map_err(|err| err.to_string())?;
            client_adapters::restore_client_setups();
            analytics::track_event(&app, "switchboard_mode_full", None);
        }
    }

    state.invalidate_runtime_status_cache();
    build_switchboard_state(&state)
}

#[tauri::command]
pub async fn set_savings_mode(
    app: AppHandle,
    mode: SavingsMode,
) -> Result<SwitchboardState, String> {
    let state: tauri::State<'_, AppState> = app.state();
    client_adapters::write_savings_mode(mode.clone()).map_err(|err| err.to_string())?;
    if !state.runtime_status().paused {
        repair_runtime(&state)?;
    }
    state.invalidate_runtime_status_cache();
    analytics::track_event(
        &app,
        "switchboard_savings_mode_changed",
        Some(json!({ "mode": format!("{mode:?}").to_ascii_lowercase() })),
    );
    build_switchboard_state(&state)
}

/// Debug-only: force the proxy intercept's bypass flag on/off so a developer
/// can manually exercise the gated path (Python proxy stopped, traffic routed
/// direct to api.anthropic.com) without crossing the real disable threshold.
/// Compiled out of release builds.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn debug_force_proxy_bypass(state: State<'_, AppState>, on: bool) -> Result<bool, String> {
    log::debug!("[debug_force_proxy_bypass] requested on={on}");
    state
        .proxy_bypass
        .store(on, std::sync::atomic::Ordering::Release);
    log::debug!(
        "[debug_force_proxy_bypass] stored bypass={}",
        state
            .proxy_bypass
            .load(std::sync::atomic::Ordering::Acquire)
    );
    if on {
        state.stop_headroom();
        log::debug!("[debug_force_proxy_bypass] stop_headroom complete");
    } else {
        // Recover from any auto-pause / client teardown that may have run
        // while bypass was active (the watchdog's give-up path or the
        // pricing gate's `disable_client_setup` call).
        client_adapters::restore_client_setups();
        state.set_runtime_paused(false);
        state
            .ensure_headroom_running()
            .map_err(|err| err.to_string())?;
    }
    Ok(state
        .proxy_bypass
        .load(std::sync::atomic::Ordering::Acquire))
}
