use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

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
