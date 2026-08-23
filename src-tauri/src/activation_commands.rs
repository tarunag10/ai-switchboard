use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

use crate::client_adapters;
use crate::models::{DashboardState, SwitchboardMode};
use crate::state::AppState;

static SELECTIVE_ACTIVATION_LOCK: AtomicBool = AtomicBool::new(false);

struct ActivationGuard;

impl Drop for ActivationGuard {
    fn drop(&mut self) {
        SELECTIVE_ACTIVATION_LOCK.store(false, Ordering::Release);
    }
}

pub const SELECTIVE_ACTIVATION_LIMIT: usize = 5;
const SELECTION_VERSION: u32 = 1;
const SELECTION_FILE: &str = "selective-activation.json";
const CHONKIFY_PREFERENCE_FILE: &str = "repo-pack-compression.json";
const TOOL_IDS: [&str; 10] = [
    "headroom",
    "rtk",
    "repo-intelligence",
    "token-xray",
    "ponytail",
    "caveman",
    "markitdown",
    "response-cache",
    "chonkify",
    "leanctx",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveActivationSelection {
    pub version: u32,
    pub selected_tool_ids: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveActivationToolResult {
    pub tool_id: String,
    pub state: String,
    pub scope: String,
    pub evidence_class: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveRollbackResult {
    pub tool_id: String,
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeanctxActivationSnapshot {
    pub configured: bool,
    pub enabled: bool,
    pub running: bool,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PonytailActivationSnapshot {
    pub receipt: Option<Value>,
    pub host_fingerprints: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveActivationReceipt {
    pub schema_version: u32,
    pub run_id: String,
    pub selected_tool_ids: Vec<String>,
    pub overall_status: String,
    pub results: Vec<SelectiveActivationToolResult>,
    pub updated_at: String,
    pub previous_mode: Option<SwitchboardMode>,
    pub after_mode: Option<SwitchboardMode>,
    pub previous_response_cache_enabled: Option<bool>,
    pub after_response_cache_enabled: Option<bool>,
    pub previous_chonkify_mode: Option<String>,
    pub after_chonkify_mode: Option<String>,
    pub previous_leanctx: Option<LeanctxActivationSnapshot>,
    pub after_leanctx: Option<LeanctxActivationSnapshot>,
    #[serde(default)]
    pub previous_ponytail: Option<PonytailActivationSnapshot>,
    #[serde(default)]
    pub after_ponytail: Option<PonytailActivationSnapshot>,
    /// Exact host entries created by this activation. Fingerprints prove the
    /// narrow rollback target without storing host paths or marketplace state.
    #[serde(default)]
    pub ponytail_created_hosts: BTreeMap<String, String>,
    pub owned_changes: Vec<String>,
    pub rollback_status: Option<String>,
    pub rollback_results: Vec<SelectiveRollbackResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveActivationResult {
    pub receipt: SelectiveActivationReceipt,
    pub dashboard: DashboardState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoPackCompressionPreference {
    pub schema_version: u32,
    pub requested_mode: String,
    pub effective_mode: String,
    pub blocked: bool,
    pub gate_verdict: String,
    pub evidence_class: String,
    pub stored: bool,
    pub updated_at: String,
}

fn selection_path(state: &AppState) -> PathBuf {
    state.tool_manager.tools_dir().join(SELECTION_FILE)
}

fn config_path(state: &AppState, file: &str) -> PathBuf {
    state
        .tool_manager
        .tools_dir()
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("config")
        .join(file)
}

fn chonkify_gate() -> (String, bool) {
    let evidence: serde_json::Value = match serde_json::from_str(include_str!(
        "../../fixtures/chonkify-provenance-evidence.json"
    )) {
        Ok(value) => value,
        Err(_) => {
            return (
                "blocked: provenance evidence could not be decoded".into(),
                false,
            )
        }
    };
    if evidence.get("license").and_then(serde_json::Value::as_str) != Some("MIT") {
        return ("blocked: MIT provenance is required".into(), false);
    }
    if evidence
        .get("requiredSignals")
        .and_then(serde_json::Value::as_array)
        .map_or(true, |signals| signals.is_empty())
    {
        return (
            "blocked: required provenance signals are missing".into(),
            false,
        );
    }
    let max_rate = evidence
        .get("maxWrongOmissionRatePct")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let fixtures: serde_json::Value = match serde_json::from_str(include_str!(
        "../../fixtures/chonkify-wrong-omission-fixtures.json"
    )) {
        Ok(value) => value,
        Err(_) => {
            return (
                "blocked: wrong-omission fixtures could not be decoded".into(),
                false,
            )
        }
    };
    for fixture in fixtures
        .get("fixtures")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let relevant = fixture
            .get("relevantFacts")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len) as f64;
        let wrong = fixture
            .get("wrongOmissions")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len) as f64;
        if relevant > 0.0 && (wrong / relevant) * 100.0 > max_rate {
            return (
                "blocked: wrong-omission evidence exceeds the promotion gate".into(),
                false,
            );
        }
    }
    (
        "repo_pack_eligible: MIT provenance and wrong-omission fixtures passed".into(),
        true,
    )
}

fn read_chonkify_preference(state: &AppState) -> Result<RepoPackCompressionPreference, String> {
    let (gate_verdict, eligible) = chonkify_gate();
    let path = config_path(state, CHONKIFY_PREFERENCE_FILE);
    if !path.exists() {
        return Ok(RepoPackCompressionPreference {
            schema_version: 1,
            requested_mode: "off".into(),
            effective_mode: "off".into(),
            blocked: !eligible,
            gate_verdict,
            evidence_class: "fixture-verified".into(),
            stored: false,
            updated_at: Utc::now().to_rfc3339(),
        });
    }
    let bytes = fs::read(&path).map_err(|error| format!("reading Chonkify preference: {error}"))?;
    let mut preference: RepoPackCompressionPreference = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decoding Chonkify preference: {error}"))?;
    if preference.schema_version != 1
        || !matches!(preference.requested_mode.as_str(), "off" | "chonkify")
    {
        preference.requested_mode = "off".into();
        preference.effective_mode = "off".into();
        preference.blocked = true;
    } else if !eligible || preference.requested_mode == "off" {
        preference.effective_mode = "off".into();
        preference.blocked = !eligible;
    } else {
        preference.effective_mode = "chonkify".into();
        preference.blocked = false;
    }
    preference.gate_verdict = gate_verdict;
    Ok(preference)
}

fn leanctx_snapshot(state: &AppState) -> LeanctxActivationSnapshot {
    let status = state.tool_manager.leanctx_sidecar_status();
    LeanctxActivationSnapshot {
        configured: status.configured,
        enabled: status.enabled,
        running: status.running,
        mode: status.mode,
    }
}

fn ponytail_snapshot(state: &AppState) -> PonytailActivationSnapshot {
    PonytailActivationSnapshot {
        receipt: state.tool_manager.ponytail_receipt_snapshot(),
        host_fingerprints: state.tool_manager.ponytail_host_fingerprints(),
    }
}

fn newly_created_ponytail_hosts(
    previous: &PonytailActivationSnapshot,
    after: &PonytailActivationSnapshot,
) -> BTreeMap<String, String> {
    after
        .host_fingerprints
        .iter()
        .filter(|(host_id, _)| !previous.host_fingerprints.contains_key(*host_id))
        .map(|(host_id, fingerprint)| (host_id.clone(), fingerprint.clone()))
        .collect()
}

fn set_chonkify_preference(
    state: &AppState,
    mode: &str,
) -> Result<RepoPackCompressionPreference, String> {
    if !matches!(mode, "off" | "chonkify") {
        return Err("Chonkify preference must be off or chonkify.".into());
    }
    let (gate_verdict, eligible) = chonkify_gate();
    if mode == "chonkify" && !eligible {
        return Err(gate_verdict);
    }
    let preference = RepoPackCompressionPreference {
        schema_version: 1,
        requested_mode: mode.into(),
        effective_mode: mode.into(),
        blocked: false,
        gate_verdict,
        evidence_class: "fixture-verified".into(),
        stored: true,
        updated_at: Utc::now().to_rfc3339(),
    };
    let path = config_path(state, CHONKIFY_PREFERENCE_FILE);
    fs::create_dir_all(
        path.parent()
            .ok_or("Chonkify preference has no config directory")?,
    )
    .map_err(|error| format!("creating Chonkify config directory: {error}"))?;
    let temporary = path.with_extension(format!("json.tmp.{}", std::process::id()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&preference).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("writing Chonkify preference: {error}"))?;
    fs::rename(&temporary, &path)
        .map_err(|error| format!("committing Chonkify preference: {error}"))?;
    Ok(preference)
}

#[tauri::command]
pub fn get_repo_pack_compression_preference(
    state: State<'_, AppState>,
) -> Result<RepoPackCompressionPreference, String> {
    read_chonkify_preference(&state)
}

#[tauri::command]
pub fn set_repo_pack_compression_preference(
    state: State<'_, AppState>,
    mode: String,
) -> Result<RepoPackCompressionPreference, String> {
    set_chonkify_preference(&state, &mode)
}

fn validate_ids(ids: &[String]) -> Result<Vec<String>, String> {
    if ids.len() != SELECTIVE_ACTIVATION_LIMIT {
        return Err("Choose exactly five tools.".to_string());
    }
    let mut normalized = Vec::with_capacity(ids.len());
    for id in ids {
        if !TOOL_IDS.contains(&id.as_str()) {
            return Err(format!("Unknown activation tool: {id}"));
        }
        if normalized.iter().any(|existing| existing == id) {
            return Err(format!("Duplicate activation tool: {id}"));
        }
        normalized.push(id.clone());
    }
    Ok(normalized)
}

fn receipt_path(state: &AppState) -> PathBuf {
    state
        .tool_manager
        .tools_dir()
        .join("selective-activation-receipt.json")
}

fn persist_receipt(state: &AppState, receipt: &SelectiveActivationReceipt) -> Result<(), String> {
    let path = receipt_path(state);
    let temporary = path.with_extension(format!("json.tmp.{}", std::process::id()));
    let payload = serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("encoding activation receipt: {error}"))?;
    fs::write(&temporary, payload)
        .map_err(|error| format!("writing activation receipt: {error}"))?;
    fs::rename(&temporary, &path)
        .map_err(|error| format!("committing activation receipt: {error}"))?;
    Ok(())
}

fn persist_rollback_progress(
    state: &AppState,
    receipt: &mut SelectiveActivationReceipt,
    rollback_results: &[SelectiveRollbackResult],
) -> Result<(), String> {
    receipt.rollback_status = Some("in_progress".into());
    receipt.rollback_results = rollback_results.to_vec();
    receipt.updated_at = Utc::now().to_rfc3339();
    persist_receipt(state, receipt)
}

fn read_receipt(state: &AppState) -> Result<SelectiveActivationReceipt, String> {
    let path = receipt_path(state);
    let bytes = fs::read(&path).map_err(|error| format!("reading activation receipt: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("decoding activation receipt: {error}"))
}

fn validate_rollback_request(
    receipt: &SelectiveActivationReceipt,
    run_id: &str,
) -> Result<(), String> {
    if receipt.schema_version != 2 {
        return Err("This activation receipt predates rollback ownership metadata.".into());
    }
    if receipt.run_id != run_id {
        return Err("Activation run ID does not match the stored receipt.".into());
    }
    Ok(())
}

fn result(
    tool_id: &str,
    state: &str,
    scope: &str,
    evidence_class: &str,
    detail: String,
) -> SelectiveActivationToolResult {
    SelectiveActivationToolResult {
        tool_id: tool_id.to_string(),
        state: state.to_string(),
        scope: scope.to_string(),
        evidence_class: evidence_class.to_string(),
        detail,
    }
}

fn activate_managed_addon(state: &AppState, id: &str) -> Result<String, String> {
    match id {
        "rtk" => {
            if !state.tool_manager.rtk_installed() {
                state
                    .tool_manager
                    .install_rtk()
                    .map_err(|error| error.to_string())?;
            }
            client_adapters::set_rtk_enabled(
                true,
                &state.tool_manager.rtk_entrypoint(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|error| error.to_string())?;
            Ok("RTK installed/enabled through the managed shell integration.".into())
        }
        "markitdown" => {
            if !state.tool_manager.markitdown_installed() {
                state
                    .tool_manager
                    .install_markitdown()
                    .map_err(|error| error.to_string())?;
            }
            client_adapters::enable_markitdown_integration(
                &state.tool_manager.markitdown_entrypoint(),
                &state.tool_manager.markitdown_shim_path(),
                &state.tool_manager.managed_python(),
            )
            .map_err(|error| error.to_string())?;
            state
                .tool_manager
                .set_markitdown_enabled(true)
                .map_err(|error| error.to_string())?;
            Ok("MarkItDown local converter and managed integration enabled.".into())
        }
        "ponytail" => {
            if !state.tool_manager.ponytail_receipt_exists() {
                state
                    .tool_manager
                    .install_ponytail()
                    .map_err(|error| error.to_string())?;
            } else {
                state
                    .tool_manager
                    .set_ponytail_enabled(true)
                    .map_err(|error| error.to_string())?;
            }
            Ok("Ponytail managed plugin integration enabled.".into())
        }
        "caveman" => {
            if !state.tool_manager.caveman_receipt_exists() {
                state
                    .tool_manager
                    .install_caveman()
                    .map_err(|error| error.to_string())?;
            }
            state
                .tool_manager
                .set_caveman_enabled(true)
                .map_err(|error| error.to_string())?;
            let level = state.tool_manager.caveman_level();
            client_adapters::enable_caveman_integration(&level)
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "Caveman managed guidance enabled at the {level} level."
            ))
        }
        "leanctx" => {
            let status = state.tool_manager.leanctx_sidecar_status();
            if !status.configured {
                state
                    .tool_manager
                    .install_leanctx_sidecar()
                    .map_err(|error| error.to_string())?;
            }
            let status = state
                .tool_manager
                .set_leanctx_enabled(true)
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "Leanctx shadow enabled; live provider routing remains {}.",
                if status.live_request_routing {
                    "blocked by policy"
                } else {
                    "disabled"
                }
            ))
        }
        "response-cache" => {
            if matches!(
                client_adapters::load_switchboard_mode(),
                Some(SwitchboardMode::Off | SwitchboardMode::Rtk) | None
            ) {
                return Err(
                    "Exact Response Cache requires an active Headroom-compatible mode.".into(),
                );
            }
            state
                .semantic_cache
                .set_enabled(true)
                .map_err(|error| error.to_string())?;
            Ok("Exact Response Cache enabled; semantic-v2 remains disabled.".into())
        }
        other => Err(format!("unsupported managed activation tool: {other}")),
    }
}

fn ordered_ids(selected: &[String]) -> Vec<String> {
    let order = [
        "rtk",
        "headroom",
        "ponytail",
        "caveman",
        "markitdown",
        "leanctx",
        "response-cache",
        "repo-intelligence",
        "token-xray",
        "chonkify",
    ];
    order
        .iter()
        .filter(|id| selected.iter().any(|selected_id| selected_id == *id))
        .map(|id| (*id).to_string())
        .collect()
}

fn preflight_selected_tools(state: &AppState, selected: &[String]) -> Result<(), String> {
    if selected.iter().any(|id| id == "response-cache")
        && !selected.iter().any(|id| id == "headroom")
        && matches!(
            client_adapters::load_switchboard_mode(),
            Some(SwitchboardMode::Off | SwitchboardMode::Rtk) | None
        )
    {
        return Err("Exact Response Cache requires Headroom or Full mode; select Headroom or enable a compatible mode first.".into());
    }
    if selected.iter().any(|id| id == "leanctx") {
        let status = state.tool_manager.leanctx_sidecar_status();
        if !status.configured || !status.executable_present || !status.loopback_only {
            return Err("Leanctx must be configured with an executable and loopback-only endpoint before batch activation.".into());
        }
    }
    if selected.iter().any(|id| id == "repo-intelligence")
        && crate::repo_intelligence::load_latest_summary()
            .map_err(|error| error.to_string())?
            .is_none()
    {
        return Err(
            "Repo Intelligence has no indexed repository summary; index a repository first.".into(),
        );
    }
    if selected.iter().any(|id| id == "chonkify") && !chonkify_gate().1 {
        return Err("Chonkify promotion evidence is not eligible; native deterministic packs remain active.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn activate_selected_tools(
    app: AppHandle,
    selected_tool_ids: Vec<String>,
) -> Result<SelectiveActivationResult, String> {
    let selected_tool_ids = validate_ids(&selected_tool_ids)?;
    if SELECTIVE_ACTIVATION_LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return Err("Another selective activation is already running.".to_string());
    }
    let _guard = ActivationGuard;
    let state: State<'_, AppState> = app.state();
    preflight_selected_tools(&state, &selected_tool_ids)?;
    let run_id = format!(
        "selective-{}-{}",
        Utc::now().timestamp_millis(),
        std::process::id()
    );
    let previous_mode = client_adapters::load_switchboard_mode();
    let previous_response_cache_enabled = selected_tool_ids
        .iter()
        .any(|id| id == "response-cache")
        .then(|| state.semantic_cache.enabled());
    let previous_chonkify_mode = selected_tool_ids
        .iter()
        .any(|id| id == "chonkify")
        .then(|| {
            read_chonkify_preference(&state)
                .ok()
                .map(|preference| preference.effective_mode)
        })
        .flatten();
    let previous_leanctx = selected_tool_ids
        .iter()
        .any(|id| id == "leanctx")
        .then(|| leanctx_snapshot(&state));
    let previous_ponytail = selected_tool_ids
        .iter()
        .any(|id| id == "ponytail")
        .then(|| ponytail_snapshot(&state));
    let mut owned_changes = Vec::new();
    if selected_tool_ids
        .iter()
        .any(|id| id == "headroom" || id == "rtk")
    {
        owned_changes.push("switchboard_mode".into());
    }
    if previous_response_cache_enabled == Some(false) {
        owned_changes.push("response_cache_enabled".into());
    }
    if selected_tool_ids.iter().any(|id| id == "chonkify") {
        owned_changes.push("chonkify_preference".into());
    }
    if previous_leanctx.is_some() {
        owned_changes.push("leanctx_state".into());
    }
    if previous_ponytail.is_some() {
        owned_changes.push("ponytail_ownership".into());
    }
    let mut results = Vec::new();
    let mut failed = false;
    let ordered = ordered_ids(&selected_tool_ids);
    for id in ordered {
        let activation = match id.as_str() {
            "headroom" => {
                let mode = if selected_tool_ids.iter().any(|selected| selected == "rtk") {
                    SwitchboardMode::Full
                } else {
                    SwitchboardMode::Headroom
                };
                crate::switchboard_commands::set_switchboard_mode(app.clone(), mode)
                    .await
                    .map(|_| "Headroom local mode enabled.".to_string())
                    .map_err(|error| error.to_string())
            }
            "repo-intelligence" => crate::repo_intelligence::load_latest_summary()
                .map_err(|error| error.to_string())
                .and_then(|summary| {
                    summary
                        .map(|_| "Latest local repository summary loaded.".into())
                        .ok_or_else(|| {
                            "No repository summary exists; index a repository first.".into()
                        })
                }),
            "token-xray" => {
                let _ = state.token_xray_live_update(None);
                Ok("Content-free local Token X-Ray evidence refreshed.".into())
            }
            "chonkify" => set_chonkify_preference(&state, "chonkify")
                .map(|_| "Chonkify enabled for read-only Repo Intelligence packs.".into()),
            addon => activate_managed_addon(&state, addon),
        };
        match activation {
            Ok(detail) => results.push(result(
                &id,
                if matches!(id.as_str(), "repo-intelligence" | "token-xray") {
                    "refreshed"
                } else {
                    "enabled"
                },
                if id == "caveman" {
                    "managed guidance"
                } else {
                    "local activation"
                },
                if matches!(id.as_str(), "repo-intelligence" | "token-xray") {
                    "local-evidence"
                } else {
                    "local-effect"
                },
                detail,
            )),
            Err(error) => {
                failed = true;
                results.push(result(
                    &id,
                    "failed",
                    "no additional scope",
                    "unavailable",
                    error,
                ));
                break;
            }
        }
    }
    let overall_status = if failed {
        if results.len() > 1 {
            "partial"
        } else {
            "failed"
        }
    } else {
        "succeeded"
    };
    let after_mode = client_adapters::load_switchboard_mode();
    let after_response_cache_enabled =
        previous_response_cache_enabled.map(|_| state.semantic_cache.enabled());
    let after_chonkify_mode = previous_chonkify_mode.as_ref().and_then(|_| {
        read_chonkify_preference(&state)
            .ok()
            .map(|preference| preference.effective_mode)
    });
    let after_leanctx = previous_leanctx.as_ref().map(|_| leanctx_snapshot(&state));
    let after_ponytail = previous_ponytail
        .as_ref()
        .map(|_| ponytail_snapshot(&state));
    let ponytail_created_hosts = previous_ponytail
        .as_ref()
        .zip(after_ponytail.as_ref())
        .map(|(previous, after)| newly_created_ponytail_hosts(previous, after))
        .unwrap_or_default();
    let receipt = SelectiveActivationReceipt {
        schema_version: 2,
        run_id,
        selected_tool_ids,
        overall_status: overall_status.into(),
        results,
        updated_at: Utc::now().to_rfc3339(),
        previous_mode,
        after_mode,
        previous_response_cache_enabled,
        after_response_cache_enabled,
        previous_chonkify_mode,
        after_chonkify_mode,
        previous_leanctx,
        after_leanctx,
        previous_ponytail,
        after_ponytail,
        ponytail_created_hosts,
        owned_changes,
        rollback_status: None,
        rollback_results: Vec::new(),
    };
    persist_receipt(&state, &receipt)?;
    Ok(SelectiveActivationResult {
        receipt,
        dashboard: state.dashboard(),
    })
}

#[tauri::command]
pub async fn rollback_selective_activation(
    app: AppHandle,
    run_id: String,
) -> Result<SelectiveActivationResult, String> {
    if SELECTIVE_ACTIVATION_LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return Err("Another selective activation or rollback is already running.".to_string());
    }
    let _guard = ActivationGuard;
    let state: State<'_, AppState> = app.state();
    let mut receipt = read_receipt(&state)?;
    validate_rollback_request(&receipt, &run_id)?;
    if matches!(receipt.rollback_status.as_deref(), Some("succeeded")) {
        return Ok(SelectiveActivationResult {
            receipt,
            dashboard: state.dashboard(),
        });
    }

    let mut rollback_results = Vec::new();
    let mut failures = Vec::new();
    if receipt
        .owned_changes
        .iter()
        .any(|change| change == "switchboard_mode")
    {
        if let Some(mode) = receipt.previous_mode.clone() {
            if client_adapters::load_switchboard_mode() != receipt.after_mode {
                rollback_results.push(SelectiveRollbackResult {
                    tool_id: "switchboard_mode".into(),
                    state: "blocked_external_change".into(),
                    detail: "Current Switchboard mode differs from this run's post-activation state; no overwrite was attempted.".into(),
                });
                failures.push("Switchboard mode changed after activation".into());
                // Do not restore this field; other independent owned changes may still be safe.
            } else if let Err(error) =
                crate::switchboard_commands::set_switchboard_mode(app.clone(), mode).await
            {
                failures.push(error.to_string());
                rollback_results.push(SelectiveRollbackResult {
                    tool_id: "switchboard_mode".into(),
                    state: "failed".into(),
                    detail: error.to_string(),
                });
            } else {
                rollback_results.push(SelectiveRollbackResult {
                    tool_id: "switchboard_mode".into(),
                    state: "restored".into(),
                    detail: "Previous Switchboard mode restored.".into(),
                });
            }
        }
    }
    if receipt
        .owned_changes
        .iter()
        .any(|change| change == "response_cache_enabled")
    {
        if let Some(enabled) = receipt.previous_response_cache_enabled {
            if Some(state.semantic_cache.enabled()) != receipt.after_response_cache_enabled {
                rollback_results.push(SelectiveRollbackResult {
                    tool_id: "response-cache".into(),
                    state: "blocked_external_change".into(),
                    detail: "Current exact-response cache state differs from this run's post-activation state; entries were not modified.".into(),
                });
                failures.push("Exact-response cache state changed after activation".into());
            } else if let Err(error) = state.semantic_cache.set_enabled(enabled) {
                failures.push(error.to_string());
                rollback_results.push(SelectiveRollbackResult {
                    tool_id: "response-cache".into(),
                    state: "failed".into(),
                    detail: error.to_string(),
                });
            } else {
                rollback_results.push(SelectiveRollbackResult {
                    tool_id: "response-cache".into(),
                    state: "restored".into(),
                    detail:
                        "Previous exact-response cache enabled state restored; entries preserved."
                            .into(),
                });
            }
        }
    }
    if receipt
        .owned_changes
        .iter()
        .any(|change| change == "chonkify_preference")
    {
        let mode = receipt.previous_chonkify_mode.as_deref().unwrap_or("off");
        let current_mode = read_chonkify_preference(&state)
            .ok()
            .map(|preference| preference.effective_mode);
        if current_mode != receipt.after_chonkify_mode {
            rollback_results.push(SelectiveRollbackResult {
                tool_id: "chonkify".into(),
                state: "blocked_external_change".into(),
                detail: "Current Chonkify preference differs from this run's post-activation state; no overwrite was attempted.".into(),
            });
            failures.push("Chonkify preference changed after activation".into());
        } else if let Err(error) = set_chonkify_preference(&state, mode) {
            failures.push(error.clone());
            rollback_results.push(SelectiveRollbackResult {
                tool_id: "chonkify".into(),
                state: "failed".into(),
                detail: error,
            });
        } else {
            rollback_results.push(SelectiveRollbackResult {
                tool_id: "chonkify".into(),
                state: "restored".into(),
                detail: "Previous Repo Intelligence pack compression preference restored.".into(),
            });
        }
    }
    if receipt
        .owned_changes
        .iter()
        .any(|change| change == "leanctx_state")
    {
        let current = leanctx_snapshot(&state);
        if Some(current.clone()) != receipt.after_leanctx {
            rollback_results.push(SelectiveRollbackResult {
                tool_id: "leanctx".into(),
                state: "blocked_external_change".into(),
                detail: "Current Leanctx state differs from this run's post-activation state; no overwrite was attempted.".into(),
            });
            failures.push("Leanctx state changed after activation".into());
        } else if let Some(previous) = receipt.previous_leanctx.as_ref() {
            let restore = if previous.enabled {
                Ok(())
            } else {
                state.tool_manager.set_leanctx_enabled(false).map(|_| ())
            };
            match restore {
                Ok(()) => rollback_results.push(SelectiveRollbackResult {
                    tool_id: "leanctx".into(),
                    state: "restored".into(),
                    detail: "Previous Leanctx shadow state restored; configured sidecar preserved."
                        .into(),
                }),
                Err(error) => {
                    failures.push(error.to_string());
                    rollback_results.push(SelectiveRollbackResult {
                        tool_id: "leanctx".into(),
                        state: "failed".into(),
                        detail: error.to_string(),
                    });
                }
            }
        }
    }
    if receipt
        .owned_changes
        .iter()
        .any(|change| change == "ponytail_ownership")
    {
        let previously_restored_hosts: BTreeSet<String> = receipt
            .rollback_results
            .iter()
            .filter(|result| result.state == "restored")
            .filter_map(|result| result.tool_id.strip_prefix("ponytail:").map(str::to_string))
            .collect();
        let ponytail_created_hosts = receipt.ponytail_created_hosts.clone();
        if let (Some(previous), Some(after)) = (
            receipt.previous_ponytail.clone(),
            receipt.after_ponytail.clone(),
        ) {
            let mut hosts_restored = true;
            for (host_id, expected_fingerprint) in &ponytail_created_hosts {
                let tool_id = format!("ponytail:{host_id}");
                if previously_restored_hosts.contains(host_id) {
                    rollback_results.push(SelectiveRollbackResult {
                        tool_id,
                        state: "restored".into(),
                        detail: "Ponytail plugin entry was already removed by an earlier rollback attempt.".into(),
                    });
                    continue;
                }
                let current = state
                    .tool_manager
                    .ponytail_host_fingerprints()
                    .get(host_id)
                    .cloned();
                if current.as_deref() != Some(expected_fingerprint) {
                    hosts_restored = false;
                    rollback_results.push(SelectiveRollbackResult {
                        tool_id,
                        state: "blocked_external_change".into(),
                        detail: "Current Ponytail plugin entry differs from this run's recorded post-activation fingerprint; it was preserved.".into(),
                    });
                    failures.push(format!("Ponytail {host_id} entry changed after activation"));
                    continue;
                }
                match state
                    .tool_manager
                    .remove_ponytail_host_if_unchanged(host_id, expected_fingerprint)
                {
                    Ok(()) => rollback_results.push(SelectiveRollbackResult {
                        tool_id,
                        state: "restored".into(),
                        detail: "Ponytail plugin entry created by this activation was removed; marketplace registration was preserved.".into(),
                    }),
                    Err(error) => {
                        hosts_restored = false;
                        failures.push(error.to_string());
                        rollback_results.push(SelectiveRollbackResult {
                            tool_id,
                            state: "failed".into(),
                            detail: error.to_string(),
                        });
                    }
                }
                if let Err(error) =
                    persist_rollback_progress(&state, &mut receipt, &rollback_results)
                {
                    hosts_restored = false;
                    failures.push(error.clone());
                    rollback_results.push(SelectiveRollbackResult {
                        tool_id: "ponytail".into(),
                        state: "failed".into(),
                        detail: format!("Recording Ponytail rollback progress failed: {error}"),
                    });
                }
            }
            if hosts_restored {
                match state.tool_manager.restore_ponytail_receipt_if_unchanged(
                    previous.receipt.as_ref(),
                    after.receipt.as_ref(),
                ) {
                    Ok(()) => rollback_results.push(SelectiveRollbackResult {
                        tool_id: "ponytail".into(),
                        state: "restored".into(),
                        detail: "Ponytail receipt restored; only plugin entries created by this activation were removed and marketplaces were preserved.".into(),
                    }),
                    Err(error) => {
                        failures.push(error.to_string());
                        rollback_results.push(SelectiveRollbackResult {
                            tool_id: "ponytail".into(),
                            state: "blocked_external_change".into(),
                            detail: error.to_string(),
                        });
                    }
                }
            }
        } else {
            rollback_results.push(SelectiveRollbackResult {
                tool_id: "ponytail".into(),
                state: "failed".into(),
                detail: "Ponytail rollback metadata is missing from this activation receipt."
                    .into(),
            });
            failures.push("Ponytail rollback metadata is missing".into());
        }
    }
    for tool_id in receipt.selected_tool_ids.iter().filter(|id| {
        !matches!(
            id.as_str(),
            "headroom" | "rtk" | "response-cache" | "chonkify" | "leanctx" | "ponytail"
        )
    }) {
        rollback_results.push(SelectiveRollbackResult {
            tool_id: tool_id.clone(),
            state: "preserved".into(),
            detail: "Rollback does not remove pre-existing managed tools or refresh-only evidence."
                .into(),
        });
    }
    receipt.rollback_results = rollback_results;
    receipt.rollback_status = Some(
        if failures.is_empty() {
            "succeeded"
        } else {
            "partial"
        }
        .into(),
    );
    receipt.updated_at = Utc::now().to_rfc3339();
    persist_receipt(&state, &receipt)?;
    if !failures.is_empty() {
        return Err(format!(
            "Selective activation rollback was partial: {}",
            failures.join("; ")
        ));
    }
    Ok(SelectiveActivationResult {
        receipt,
        dashboard: state.dashboard(),
    })
}

#[tauri::command]
pub fn validate_selective_activation_selection(
    selected_tool_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    validate_ids(&selected_tool_ids)
}

#[tauri::command]
pub fn get_selective_activation_selection(
    state: State<'_, AppState>,
) -> Result<Option<SelectiveActivationSelection>, String> {
    let path = selection_path(&state);
    if !path.exists() {
        return Ok(None);
    }
    let bytes =
        fs::read(&path).map_err(|error| format!("reading activation selection: {error}"))?;
    let selection: SelectiveActivationSelection = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decoding activation selection: {error}"))?;
    validate_ids(&selection.selected_tool_ids)?;
    if selection.version != SELECTION_VERSION {
        return Err(format!(
            "Unsupported activation selection version: {}",
            selection.version
        ));
    }
    Ok(Some(selection))
}

#[tauri::command]
pub fn save_selective_activation_selection(
    state: State<'_, AppState>,
    selected_tool_ids: Vec<String>,
) -> Result<SelectiveActivationSelection, String> {
    let selected_tool_ids = validate_ids(&selected_tool_ids)?;
    let selection = SelectiveActivationSelection {
        version: SELECTION_VERSION,
        selected_tool_ids,
        updated_at: Utc::now().to_rfc3339(),
    };
    let path = selection_path(&state);
    let temporary = path.with_extension("json.tmp");
    let payload = serde_json::to_vec_pretty(&selection)
        .map_err(|error| format!("encoding activation selection: {error}"))?;
    fs::write(&temporary, payload)
        .map_err(|error| format!("writing activation selection: {error}"))?;
    fs::rename(&temporary, &path)
        .map_err(|error| format!("committing activation selection: {error}"))?;
    Ok(selection)
}

#[cfg(test)]
mod tests {
    use super::{
        chonkify_gate, newly_created_ponytail_hosts, validate_ids, validate_rollback_request,
        PonytailActivationSnapshot, SelectiveActivationReceipt,
    };
    use crate::models::SwitchboardMode;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn five() -> Vec<String> {
        [
            "headroom",
            "rtk",
            "repo-intelligence",
            "token-xray",
            "ponytail",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    #[test]
    fn exact_five_selection_is_accepted() {
        assert_eq!(validate_ids(&five()).expect("valid selection").len(), 5);
    }

    #[test]
    fn invalid_selection_is_rejected() {
        let mut too_many = five();
        too_many.push("caveman".into());
        assert!(validate_ids(&too_many).is_err());

        let mut duplicate = five();
        duplicate[4] = "rtk".into();
        assert!(validate_ids(&duplicate).is_err());

        let mut unknown = five();
        unknown[4] = "experimental-router".into();
        assert!(validate_ids(&unknown).is_err());
    }

    #[test]
    fn chonkify_gate_matches_checked_in_provenance_and_fixtures() {
        let (verdict, eligible) = chonkify_gate();
        assert!(eligible);
        assert!(verdict.starts_with("repo_pack_eligible:"));
    }

    #[test]
    fn ponytail_delta_tracks_only_new_host_entries() {
        let before = PonytailActivationSnapshot {
            receipt: Some(json!({ "enabled": false })),
            host_fingerprints: BTreeMap::from([("claude-code".into(), "before".into())]),
        };
        let after = PonytailActivationSnapshot {
            receipt: Some(json!({ "enabled": true })),
            host_fingerprints: BTreeMap::from([
                ("claude-code".into(), "changed-but-preexisting".into()),
                ("codex".into(), "created".into()),
            ]),
        };
        assert_eq!(
            newly_created_ponytail_hosts(&before, &after),
            BTreeMap::from([("codex".into(), "created".into())])
        );
    }

    fn receipt() -> SelectiveActivationReceipt {
        SelectiveActivationReceipt {
            schema_version: 2,
            run_id: "run-1".into(),
            selected_tool_ids: vec!["headroom".into()],
            overall_status: "succeeded".into(),
            results: Vec::new(),
            updated_at: "now".into(),
            previous_mode: Some(SwitchboardMode::Off),
            after_mode: Some(SwitchboardMode::Headroom),
            previous_response_cache_enabled: None,
            after_response_cache_enabled: None,
            previous_chonkify_mode: None,
            after_chonkify_mode: None,
            previous_leanctx: None,
            after_leanctx: None,
            previous_ponytail: None,
            after_ponytail: None,
            ponytail_created_hosts: Default::default(),
            owned_changes: vec!["switchboard_mode".into()],
            rollback_status: None,
            rollback_results: Vec::new(),
        }
    }

    #[test]
    fn rollback_rejects_wrong_run_id_and_legacy_receipts() {
        let receipt = receipt();
        assert!(validate_rollback_request(&receipt, "other-run").is_err());
        let mut legacy = receipt;
        legacy.schema_version = 1;
        assert!(validate_rollback_request(&legacy, "run-1").is_err());
    }
}
