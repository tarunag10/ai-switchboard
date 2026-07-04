use tauri::State;

use crate::codex_threads;
use crate::message_logging;
use crate::models::{
    CodexDbRestoreResult, CodexThreadRetaggingSettings, MessageLoggingSettings, PurgeResult,
};
use crate::state::AppState;

#[tauri::command]
pub fn get_message_logging_settings() -> MessageLoggingSettings {
    message_logging::load_settings()
}

#[tauri::command]
pub fn set_message_logging_settings(
    settings: MessageLoggingSettings,
) -> Result<MessageLoggingSettings, String> {
    message_logging::save_settings(&settings).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn enable_full_message_logging(hours: u32) -> Result<MessageLoggingSettings, String> {
    let settings = MessageLoggingSettings::enabled_for(hours);
    message_logging::save_settings(&settings).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn disable_full_message_logging() -> Result<MessageLoggingSettings, String> {
    let settings = MessageLoggingSettings {
        full_message_logging: false,
        full_message_logging_expires_at: None,
        message_log_retention_hours: 24,
    };
    message_logging::save_settings(&settings).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn purge_message_logs(state: State<'_, AppState>) -> PurgeResult {
    state.purge_message_logs()
}

#[tauri::command]
pub fn get_codex_thread_retagging_settings() -> CodexThreadRetaggingSettings {
    codex_threads::get_codex_thread_retagging_settings()
}

#[tauri::command]
pub fn set_codex_thread_retagging_settings(
    settings: CodexThreadRetaggingSettings,
) -> Result<CodexThreadRetaggingSettings, String> {
    codex_threads::set_codex_thread_retagging_settings(settings).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn restore_codex_thread_db_backup(path: String) -> Result<CodexDbRestoreResult, String> {
    codex_threads::restore_codex_thread_db_backup(&path).map_err(|err| err.to_string())
}
