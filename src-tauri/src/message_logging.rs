use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use crate::models::{MessageLoggingSettings, PurgeResult};
use crate::storage::{app_data_dir, config_file};

const SETTINGS_FILE: &str = "message-logging.json";
const DEFAULT_RETENTION_HOURS: u32 = 24;

pub fn load_settings() -> MessageLoggingSettings {
    load_settings_from(&app_data_dir()).unwrap_or_default()
}

pub fn save_settings(settings: &MessageLoggingSettings) -> Result<MessageLoggingSettings> {
    save_settings_to(&app_data_dir(), settings)
}

pub fn full_message_logging_active() -> bool {
    if std::env::var("HEADROOM_FULL_MESSAGE_LOGGING")
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
    {
        return true;
    }
    load_settings().active_now(Utc::now())
}

pub fn purge_message_logs(activity_facts_path: &Path) -> PurgeResult {
    let mut removed_paths = Vec::new();
    if activity_facts_path.exists() {
        match std::fs::remove_file(activity_facts_path) {
            Ok(()) => removed_paths.push(activity_facts_path.display().to_string()),
            Err(error) => {
                return PurgeResult {
                    purged: false,
                    removed_paths,
                    notes: vec![format!(
                        "Failed to remove {}: {error}",
                        activity_facts_path.display()
                    )],
                };
            }
        }
    }
    PurgeResult {
        purged: true,
        removed_paths,
        notes: vec![
            "Persisted Activity feed facts were reset.".to_string(),
            "Live proxy memory is not changed; restart the runtime after disabling full message logging."
                .to_string(),
        ],
    }
}

pub fn redact_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_text(&text)),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_value).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if key_is_sensitive(&key) {
                        (key, Value::String("[REDACTED]".to_string()))
                    } else {
                        (key, redact_value(value))
                    }
                })
                .collect(),
        ),
        other => other,
    }
}

pub fn redact_text(input: &str) -> String {
    let mut output = input.to_string();
    for token in redactable_tokens(input) {
        output = output.replace(&token, "[REDACTED]");
    }
    output
}

fn load_settings_from(base_dir: &Path) -> Result<MessageLoggingSettings> {
    let path = config_file(base_dir, SETTINGS_FILE);
    if !path.exists() {
        return Ok(MessageLoggingSettings::default());
    }
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let settings = serde_json::from_slice::<MessageLoggingSettings>(&bytes)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(settings.normalized(Utc::now()))
}

fn save_settings_to(
    base_dir: &Path,
    settings: &MessageLoggingSettings,
) -> Result<MessageLoggingSettings> {
    let path = config_file(base_dir, SETTINGS_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let normalized = settings.normalized(Utc::now());
    std::fs::write(&path, serde_json::to_vec_pretty(&normalized)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(normalized)
}

fn key_is_sensitive(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("authorization")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("access_key")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.ends_with(".p8")
        || lower.ends_with(".pem")
        || lower.ends_with(".p12")
}

fn redactable_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for raw in input.split(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | ';')) {
        let token = raw.trim_matches(|ch: char| matches!(ch, ':' | '=' | ')' | ']' | '}'));
        if token.starts_with("sk-ant-")
            || token.starts_with("sk-proj-")
            || token.starts_with("ghp_")
            || token.starts_with("github_pat_")
            || token.ends_with(".p8")
            || token.ends_with(".pem")
            || token.ends_with(".p12")
        {
            tokens.push(token.to_string());
        }
    }

    for marker in [
        "BEGIN PRIVATE KEY",
        "AWS_SECRET_ACCESS_KEY",
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
    ] {
        if input.contains(marker) {
            tokens.push(marker.to_string());
        }
    }

    for prefix in [
        "Authorization: Bearer ",
        "authorization: bearer ",
        "Bearer ",
        "bearer ",
    ] {
        let mut rest = input;
        while let Some(idx) = rest.find(prefix) {
            let after = &rest[idx + prefix.len()..];
            let token: String = after
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || "-_.~+/=".contains(*ch))
                .collect();
            if token.len() >= 8 {
                tokens.push(format!("{prefix}{token}"));
            }
            rest = &after[token.len()..];
        }
    }
    tokens
}

impl Default for MessageLoggingSettings {
    fn default() -> Self {
        Self {
            full_message_logging: false,
            full_message_logging_expires_at: None,
            message_log_retention_hours: DEFAULT_RETENTION_HOURS,
        }
    }
}

impl MessageLoggingSettings {
    pub fn active_now(&self, now: DateTime<Utc>) -> bool {
        self.full_message_logging
            && self
                .full_message_logging_expires_at
                .map(|expires_at| expires_at > now)
                .unwrap_or(false)
    }

    pub fn normalized(&self, now: DateTime<Utc>) -> Self {
        let retention = self
            .message_log_retention_hours
            .clamp(1, DEFAULT_RETENTION_HOURS);
        let expires_at = self.full_message_logging_expires_at;
        let full_message_logging =
            self.full_message_logging && expires_at.map(|expiry| expiry > now).unwrap_or(false);
        Self {
            full_message_logging,
            full_message_logging_expires_at: if full_message_logging {
                expires_at
            } else {
                None
            },
            message_log_retention_hours: retention,
        }
    }

    pub fn enabled_for(hours: u32) -> Self {
        let hours = hours.clamp(1, DEFAULT_RETENTION_HOURS);
        Self {
            full_message_logging: true,
            full_message_logging_expires_at: Some(Utc::now() + Duration::hours(hours.into())),
            message_log_retention_hours: hours,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    #[test]
    fn full_message_logging_defaults_off() {
        let temp = TempDir::new().unwrap();
        let settings = load_settings_from(temp.path()).unwrap();
        assert!(!settings.full_message_logging);
        assert!(!settings.active_now(Utc::now()));
        assert_eq!(settings.message_log_retention_hours, 24);
    }

    #[test]
    fn expired_settings_normalize_to_off() {
        let settings = MessageLoggingSettings {
            full_message_logging: true,
            full_message_logging_expires_at: Some(
                Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            ),
            message_log_retention_hours: 99,
        };
        let normalized = settings.normalized(Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap());
        assert!(!normalized.full_message_logging);
        assert!(normalized.full_message_logging_expires_at.is_none());
        assert_eq!(normalized.message_log_retention_hours, 24);
    }

    #[test]
    fn redacts_fake_secrets_in_nested_payloads() {
        let payload = serde_json::json!({
            "headers": {"Authorization": "Bearer abcdefghijklmnop"},
            "messages": [{
                "content": "token sk-ant-test ghp_abcdef github_pat_abcdef sk-proj-test BEGIN PRIVATE KEY file AuthKey_123.p8"
            }],
            "OPENAI_API_KEY": "sk-proj-direct"
        });
        let redacted = serde_json::to_string(&redact_value(payload)).unwrap();
        assert!(!redacted.contains("sk-ant-test"));
        assert!(!redacted.contains("ghp_abcdef"));
        assert!(!redacted.contains("github_pat_abcdef"));
        assert!(!redacted.contains("sk-proj-test"));
        assert!(!redacted.contains("BEGIN PRIVATE KEY"));
        assert!(!redacted.contains("abcdefghijklmnop"));
        assert!(!redacted.contains("AuthKey_123.p8"));
        assert!(redacted.contains("[REDACTED]"));
    }
}
