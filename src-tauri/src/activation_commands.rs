use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use serde::{Deserialize, Serialize};
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
pub struct SelectiveActivationReceipt {
    pub schema_version: u32,
    pub run_id: String,
    pub selected_tool_ids: Vec<String>,
    pub overall_status: String,
    pub results: Vec<SelectiveActivationToolResult>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveActivationResult {
    pub receipt: SelectiveActivationReceipt,
    pub dashboard: DashboardState,
}

fn selection_path(state: &AppState) -> PathBuf {
    state.tool_manager.tools_dir().join(SELECTION_FILE)
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
    let run_id = format!(
        "selective-{}-{}",
        Utc::now().timestamp_millis(),
        std::process::id()
    );
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
            "chonkify" => Ok(
                "Chonkify remains scoped to the read-only Repo Intelligence pack preference."
                    .into(),
            ),
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
    let receipt = SelectiveActivationReceipt {
        schema_version: 1,
        run_id,
        selected_tool_ids,
        overall_status: overall_status.into(),
        results,
        updated_at: Utc::now().to_rfc3339(),
    };
    persist_receipt(&state, &receipt)?;
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
    use super::validate_ids;

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
}
