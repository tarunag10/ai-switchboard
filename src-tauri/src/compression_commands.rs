use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::client_adapters;
use crate::models::SavingsMode;
use crate::provider_upstream_profiles::{
    clear_provider_upstream_profiles, load_provider_upstream_profiles,
    save_provider_upstream_profiles, test_provider_upstream_url,
    validate_provider_upstream_profiles, ProviderUpstreamProfilesState,
    ProviderUpstreamTestResult,
};
use crate::headroom_advanced_settings::{
    load_headroom_advanced_settings, save_headroom_advanced_settings, HeadroomAdvancedSettings,
};
use crate::state::{AppState, ContentClassCompressionStats};
use crate::switchboard_commands::repair_runtime;
use crate::tool_manager::compression_profiles::{
    all_compression_profile_definitions, clear_compression_profile, effective_savings_mode,
    load_compression_profile, preset_definition, save_compression_profile,
    CompressionProfileAdvanced, CompressionProfileId, CompressionProfileState,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionProfilePresetView {
    pub id: String,
    pub label: String,
    pub description: String,
    pub savings_mode: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionProfileView {
    pub version: u8,
    pub preset_id: String,
    pub advanced: CompressionProfileAdvanced,
    pub effective_savings_mode: String,
    pub history_compression_supported: bool,
    pub presets: Vec<CompressionProfilePresetView>,
    pub storage_path: String,
}

#[tauri::command]
pub async fn get_compression_profile() -> Result<CompressionProfileView, String> {
    Ok(build_compression_profile_view(load_compression_profile()))
}

#[tauri::command]
pub async fn set_compression_profile(
    app: AppHandle,
    preset_id: String,
    advanced: Option<CompressionProfileAdvanced>,
    restart_headroom: bool,
) -> Result<CompressionProfileView, String> {
    let Some(parsed_id) = CompressionProfileId::parse(&preset_id) else {
        return Err(format!("Unknown compression preset: {preset_id}"));
    };
    let definition = preset_definition(parsed_id);
    let mut state = CompressionProfileState {
        version: 1,
        preset_id: parsed_id,
        advanced: advanced.unwrap_or_else(|| definition.advanced.clone()),
    };
    save_compression_profile(&state).map_err(|err| err.to_string())?;
    let savings_mode = effective_savings_mode(&state);
    client_adapters::write_savings_mode(savings_mode.clone()).map_err(|err| err.to_string())?;
    if restart_headroom {
        let app_state: State<'_, AppState> = app.state();
        repair_runtime(&app_state)?;
    }
    Ok(build_compression_profile_view(state))
}

#[tauri::command]
pub async fn clear_compression_profile_command(
    app: AppHandle,
    restart_headroom: bool,
) -> Result<CompressionProfileView, String> {
    clear_compression_profile().map_err(|err| err.to_string())?;
    client_adapters::write_savings_mode(SavingsMode::Balanced).map_err(|err| err.to_string())?;
    if restart_headroom {
        let app_state: State<'_, AppState> = app.state();
        repair_runtime(&app_state)?;
    }
    Ok(build_compression_profile_view(load_compression_profile()))
}

#[tauri::command]
pub async fn get_provider_upstream_profiles() -> Result<ProviderUpstreamProfilesState, String> {
    Ok(load_provider_upstream_profiles())
}

#[tauri::command]
pub async fn set_provider_upstream_profiles(
    app: AppHandle,
    state: ProviderUpstreamProfilesState,
    restart_headroom: bool,
) -> Result<ProviderUpstreamProfilesState, String> {
    validate_provider_upstream_profiles(&state).map_err(|err| err.to_string())?;
    save_provider_upstream_profiles(&state).map_err(|err| err.to_string())?;
    if restart_headroom {
        let app_state: State<'_, AppState> = app.state();
        repair_runtime(&app_state)?;
    }
    Ok(state)
}

#[tauri::command]
pub async fn clear_provider_upstream_profiles_command(
    app: AppHandle,
    restart_headroom: bool,
) -> Result<ProviderUpstreamProfilesState, String> {
    clear_provider_upstream_profiles().map_err(|err| err.to_string())?;
    if restart_headroom {
        let app_state: State<'_, AppState> = app.state();
        repair_runtime(&app_state)?;
    }
    Ok(load_provider_upstream_profiles())
}

#[tauri::command]
pub async fn test_provider_upstream_profile(
    provider: String,
    url: String,
) -> Result<ProviderUpstreamTestResult, String> {
    Ok(test_provider_upstream_url(&provider, &url))
}

#[tauri::command]
pub async fn get_headroom_content_class_stats(
    app: AppHandle,
) -> Result<ContentClassCompressionStats, String> {
    let state: State<'_, AppState> = app.state();
    Ok(state
        .headroom_content_class_for_xray()
        .unwrap_or_default())
}

#[tauri::command]
pub async fn get_headroom_advanced_settings() -> Result<HeadroomAdvancedSettings, String> {
    Ok(load_headroom_advanced_settings())
}

#[tauri::command]
pub async fn set_headroom_advanced_settings(
    app: AppHandle,
    settings: HeadroomAdvancedSettings,
    restart_headroom: bool,
) -> Result<HeadroomAdvancedSettings, String> {
    let saved = save_headroom_advanced_settings(&settings).map_err(|err| err.to_string())?;
    if restart_headroom {
        let app_state: State<'_, AppState> = app.state();
        repair_runtime(&app_state)?;
    }
    Ok(saved)
}

fn savings_mode_label(mode: &SavingsMode) -> String {
    match mode {
        SavingsMode::Balanced => "balanced".to_string(),
        SavingsMode::Aggressive => "aggressive".to_string(),
    }
}

fn build_compression_profile_view(state: CompressionProfileState) -> CompressionProfileView {
    let savings_mode = effective_savings_mode(&state);
    CompressionProfileView {
        version: state.version,
        preset_id: state.preset_id.as_str().to_string(),
        advanced: state.advanced.clone(),
        effective_savings_mode: savings_mode_label(&savings_mode),
        history_compression_supported: crate::tool_manager::compression_profiles::history_compression_toggle_supported(),
        presets: all_compression_profile_definitions()
            .into_iter()
            .map(|preset| CompressionProfilePresetView {
                id: preset.id.as_str().to_string(),
                label: preset.label.to_string(),
                description: preset.description.to_string(),
                savings_mode: savings_mode_label(&preset.savings_mode),
            })
            .collect(),
        storage_path: crate::tool_manager::compression_profiles::compression_profile_path()
            .display()
            .to_string(),
    }
}
