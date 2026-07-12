//! Native Goose provider routing.
//!
//! Goose's current (2026) config contract is deliberately small and stable:
//! `config.yaml` stores `active_provider`, a `providers` map whose entries
//! contain `enabled`, `model`, and `configured`, and non-secret provider
//! endpoint parameters.  This adapter only edits the documented endpoint
//! parameters for OpenAI and Anthropic.  It never reads or writes
//! `secrets.yaml`, keychain state, credentials, account state, or model values.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};

use crate::client_paths::home_dir;
use crate::client_provider_configs::{HEADROOM_ANTHROPIC_BASE_URL, HEADROOM_OPENAI_BASE_URL};
use crate::managed_files::backup_if_exists;

pub(crate) const GOOSE_NATIVE_MARKER: &str = "headroom:goose-provider";
pub(crate) const GOOSE_NATIVE_APPLY_RECORD_ID: &str = "goose-provider-routing";
pub(crate) const GOOSE_NATIVE_OWNER: &str = "Goose provider routing";
pub(crate) const GOOSE_CONFIG_FILE: &str = "config.yaml";
const OPENAI_BASE_PATH: &str = "v1/chat/completions";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GooseProviderKind {
    OpenAi,
    Anthropic,
}

impl GooseProviderKind {
    fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "openai" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            _ => None,
        }
    }

    fn endpoint_keys(self) -> &'static [&'static str] {
        match self {
            Self::OpenAi => &["OPENAI_BASE_URL", "OPENAI_BASE_PATH"],
            Self::Anthropic => &["ANTHROPIC_HOST"],
        }
    }

    fn provider_name(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GooseProviderConfigPreview {
    pub path: PathBuf,
    pub current_state: String,
    pub proposed_state: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub changed: bool,
    pub blocked_reason: Option<String>,
    pub evidence: Vec<String>,
}

/// Resolve the current Goose config path without guessing over user state.
///
/// The upstream Goose source uses etcetera's macOS strategy, which resolves to
/// `~/Library/Application Support/Block/goose/config.yaml`. Older CLI builds
/// and Linux installs use `~/.config/goose/config.yaml`; an existing file wins
/// so upgrades do not strand a previously configured Goose installation. The
/// official `GOOSE_PATH_ROOT` fixture override is also honoured.
pub(crate) fn goose_config_path() -> PathBuf {
    if let Some(root) = std::env::var_os("GOOSE_PATH_ROOT") {
        return PathBuf::from(root).join("config").join(GOOSE_CONFIG_FILE);
    }

    let home = home_dir();
    let candidates = [
        home.join("Library")
            .join("Application Support")
            .join("Block")
            .join("goose")
            .join(GOOSE_CONFIG_FILE),
        home.join(".config").join("goose").join(GOOSE_CONFIG_FILE),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| {
            home_dir()
                .join("Library")
                .join("Application Support")
                .join("Block")
                .join("goose")
                .join(GOOSE_CONFIG_FILE)
        })
}

pub(crate) fn goose_config_backup_pattern() -> String {
    let path = goose_config_path();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(GOOSE_CONFIG_FILE);
    format!("{}.headroom-backup-*", file_name)
}

pub(crate) fn goose_apply_confirmation_phrase(current_state: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(current_state.as_bytes());
    let digest = hasher.finalize();
    let hash = digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "Apply {GOOSE_NATIVE_MARKER} to {} after reviewing {hash}",
        goose_config_path().display()
    )
}

fn empty_config() -> Value {
    Value::Mapping(Mapping::new())
}

fn read_config(path: &Path) -> Result<(Value, String)> {
    if !path.exists() {
        return Ok((empty_config(), String::new()));
    }
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let value = serde_yaml::from_str::<Value>(&raw)
        .with_context(|| format!("parsing Goose YAML config {}", path.display()))?;
    if !value.is_mapping() {
        return Err(anyhow!(
            "{} must contain a YAML mapping at the top level before Switchboard can manage Goose.",
            path.display()
        ));
    }
    Ok((value, raw))
}

fn mapping(value: &Value) -> Result<&Mapping> {
    value
        .as_mapping()
        .ok_or_else(|| anyhow!("Goose config must be a YAML mapping."))
}

fn mapping_mut(value: &mut Value) -> Result<&mut Mapping> {
    value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("Goose config must be a YAML mapping."))
}

fn string_key(key: &str) -> Value {
    Value::String(key.to_string())
}

fn scalar_string(mapping: &Mapping, key: &str) -> Option<String> {
    mapping
        .get(string_key(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn active_provider_and_model(root: &Mapping) -> (Option<String>, Option<String>) {
    let provider = scalar_string(root, "active_provider");
    let model = provider.as_deref().and_then(|provider_name| {
        root.get(string_key("providers"))
            .and_then(Value::as_mapping)
            .and_then(|providers| providers.get(string_key(provider_name)))
            .and_then(Value::as_mapping)
            .and_then(|entry| scalar_string(entry, "model"))
    });
    (provider, model)
}

fn env_override_reason(kind: GooseProviderKind) -> Option<String> {
    if std::env::var_os("GOOSE_PROVIDER").is_some() {
        return Some(
            "GOOSE_PROVIDER is set in the environment; Goose will ignore active_provider from config.yaml until that override is removed.".to_string(),
        );
    }

    let keys = match kind {
        GooseProviderKind::OpenAi => {
            &["OPENAI_HOST", "OPENAI_BASE_URL", "OPENAI_BASE_PATH"] as &[&str]
        }
        GooseProviderKind::Anthropic => &["ANTHROPIC_HOST"] as &[&str],
    };
    keys.iter().find_map(|key| {
        std::env::var_os(key).map(|_| {
            format!(
                "{key} is set in the environment; Goose will ignore the persisted provider endpoint until that override is removed."
            )
        })
    })
}

fn managed_route_matches(root: &Mapping, kind: GooseProviderKind) -> bool {
    match kind {
        GooseProviderKind::OpenAi => {
            scalar_string(root, "OPENAI_BASE_URL").as_deref() == Some(HEADROOM_OPENAI_BASE_URL)
                && scalar_string(root, "OPENAI_BASE_PATH").as_deref() == Some(OPENAI_BASE_PATH)
        }
        GooseProviderKind::Anthropic => {
            scalar_string(root, "ANTHROPIC_HOST").as_deref() == Some(HEADROOM_ANTHROPIC_BASE_URL)
        }
    }
}

fn user_owned_endpoint_keys_present(root: &Mapping, kind: GooseProviderKind) -> Vec<String> {
    let managed = managed_route_matches(root, kind);
    kind.endpoint_keys()
        .iter()
        .filter(|key| root.contains_key(string_key(key)) && !managed)
        .map(|key| (*key).to_string())
        .collect()
}

fn set_managed_route(root: &mut Mapping, kind: GooseProviderKind) {
    match kind {
        GooseProviderKind::OpenAi => {
            root.insert(
                string_key("OPENAI_BASE_URL"),
                Value::String(HEADROOM_OPENAI_BASE_URL.to_string()),
            );
            root.insert(
                string_key("OPENAI_BASE_PATH"),
                Value::String(OPENAI_BASE_PATH.to_string()),
            );
        }
        GooseProviderKind::Anthropic => {
            root.insert(
                string_key("ANTHROPIC_HOST"),
                Value::String(HEADROOM_ANTHROPIC_BASE_URL.to_string()),
            );
        }
    }
}

fn remove_managed_route(root: &mut Mapping, kind: GooseProviderKind) -> bool {
    if !managed_route_matches(root, kind) {
        return false;
    }
    let mut changed = false;
    for key in kind.endpoint_keys() {
        changed |= root.remove(string_key(key)).is_some();
    }
    changed
}

/// Replace only the documented, non-secret endpoint fields in Goose's config.
/// Existing endpoint values are treated as user-owned and block the write;
/// this guarantees Off mode can remove the Switchboard fields without needing
/// to guess or restore a value that the user supplied.
pub(crate) fn preview_goose_provider_config() -> Result<GooseProviderConfigPreview> {
    let path = goose_config_path();
    let (current, raw) = read_config(&path)?;
    let root = mapping(&current)?;
    let (provider, model) = active_provider_and_model(root);
    let mut proposed = current.clone();
    let mut blocked_reason = None;
    let mut evidence = vec![
        "Goose schema verified from the upstream Config contract: active_provider, providers.<name>.model, and documented non-secret endpoint parameters.".to_string(),
        "Credentials, secrets.yaml, keychain state, account state, and provider model values are never read or written.".to_string(),
    ];

    let kind = provider.as_deref().and_then(GooseProviderKind::from_name);
    if provider.is_none() {
        blocked_reason = Some(
            "Goose config has no active_provider; Switchboard will not guess a provider or model."
                .to_string(),
        );
    } else if kind.is_none() {
        blocked_reason = Some(format!(
            "Goose provider '{}' is not yet allowlisted for Switchboard endpoint routing; supported native schemas are openai and anthropic.",
            provider.as_deref().unwrap_or_default()
        ));
    } else if let Some(kind) = kind {
        if let Some(reason) = env_override_reason(kind) {
            blocked_reason = Some(reason);
        } else {
            let user_owned = user_owned_endpoint_keys_present(root, kind);
            if !user_owned.is_empty() {
                blocked_reason = Some(format!(
                    "Goose endpoint keys {} already contain user-owned values; Switchboard will not overwrite them. Use the existing Switchboard sidecar or clear them manually after reviewing a backup.",
                    user_owned.join(", ")
                ));
            } else if !managed_route_matches(root, kind) {
                set_managed_route(mapping_mut(&mut proposed)?, kind);
                evidence.push(format!(
                    "Allowlisted {} endpoint fields are the only proposed native writes: {}.",
                    kind.provider_name(),
                    kind.endpoint_keys().join(", ")
                ));
            } else {
                evidence.push(
                    "The allowlisted Switchboard endpoint values are already present; no write is needed.".to_string(),
                );
            }
        }
    }

    let proposed_raw = if blocked_reason.is_some() {
        raw.clone()
    } else {
        serde_yaml::to_string(&proposed).context("serializing Goose provider preview")?
    };
    let changed = proposed_raw != raw;
    Ok(GooseProviderConfigPreview {
        path,
        current_state: redact_yaml_for_display(&raw),
        proposed_state: redact_yaml_for_display(&proposed_raw),
        provider,
        model,
        changed,
        blocked_reason,
        evidence,
    })
}

pub(crate) fn configure_goose_provider_config() -> Result<(Vec<String>, Vec<String>)> {
    let preview = preview_goose_provider_config()?;
    if let Some(reason) = preview.blocked_reason {
        return Err(anyhow!(
            "Goose native provider routing is blocked: {reason}"
        ));
    }
    if !preview.changed {
        return Ok((Vec::new(), Vec::new()));
    }

    let (mut next, _) = read_config(&preview.path)?;
    let provider = preview
        .provider
        .as_deref()
        .and_then(GooseProviderKind::from_name)
        .ok_or_else(|| anyhow!("Goose active provider changed before apply."))?;
    set_managed_route(mapping_mut(&mut next)?, provider);
    let serialized = serde_yaml::to_string(&next).context("serializing Goose provider config")?;
    if let Some(parent) = preview.path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let backup = backup_if_exists(&preview.path)?;
    std::fs::write(&preview.path, serialized)
        .with_context(|| format!("writing {}", preview.path.display()))?;
    Ok((
        vec![preview.path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

pub(crate) fn goose_provider_config_matches() -> Result<bool> {
    let path = goose_config_path();
    let (current, _) = read_config(&path)?;
    let root = mapping(&current)?;
    let Some(provider) = scalar_string(root, "active_provider") else {
        return Ok(false);
    };
    let Some(kind) = GooseProviderKind::from_name(&provider) else {
        return Ok(false);
    };
    Ok(managed_route_matches(root, kind))
}

/// Off mode removes only exact Switchboard endpoint values. User-owned values
/// and unsupported provider settings are left untouched.
pub(crate) fn remove_goose_provider_config() -> Result<Vec<String>> {
    let path = goose_config_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let (mut current, _) = read_config(&path)?;
    let root = mapping(&current)?;
    let Some(provider) = scalar_string(root, "active_provider") else {
        return Ok(Vec::new());
    };
    let Some(kind) = GooseProviderKind::from_name(&provider) else {
        return Ok(Vec::new());
    };
    if !managed_route_matches(root, kind) {
        return Ok(Vec::new());
    }
    if !remove_managed_route(mapping_mut(&mut current)?, kind) {
        return Ok(Vec::new());
    }
    let serialized =
        serde_yaml::to_string(&current).context("serializing Goose provider cleanup")?;
    let _ = backup_if_exists(&path)?;
    std::fs::write(&path, serialized).with_context(|| format!("writing {}", path.display()))?;
    Ok(vec![path.display().to_string()])
}

fn redact_yaml_for_display(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "{}\n".to_string();
    }
    let Ok(mut value) = serde_yaml::from_str::<Value>(raw) else {
        return "<unavailable: invalid Goose YAML>".to_string();
    };
    redact_value(&mut value, None);
    serde_yaml::to_string(&value).unwrap_or_else(|_| "<unavailable>".to_string())
}

fn redact_value(value: &mut Value, key: Option<&str>) {
    if key.is_some_and(is_secret_key) {
        *value = Value::String("<redacted>".to_string());
        return;
    }
    match value {
        Value::Mapping(mapping) => {
            for (key, child) in mapping.iter_mut() {
                let key = key.as_str();
                redact_value(child, key);
            }
        }
        Value::Sequence(sequence) => {
            for child in sequence {
                redact_value(child, None);
            }
        }
        _ => {}
    }
}

fn is_secret_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    ["KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL", "AUTH"]
        .iter()
        .any(|part| upper.contains(part))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;

    fn write_config(root: &Path, content: &str) {
        let path = root.join("config").join(GOOSE_CONFIG_FILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn with_fixture(content: &str, test: impl FnOnce(&Path)) {
        let root = tempfile::tempdir().unwrap();
        let previous_root = std::env::var_os("GOOSE_PATH_ROOT");
        let override_keys = [
            "GOOSE_PROVIDER",
            "OPENAI_HOST",
            "OPENAI_BASE_URL",
            "OPENAI_BASE_PATH",
            "ANTHROPIC_HOST",
        ];
        let previous_overrides = override_keys
            .iter()
            .map(|key| (*key, std::env::var_os(key)))
            .collect::<Vec<_>>();
        for key in override_keys {
            std::env::remove_var(key);
        }
        std::env::set_var("GOOSE_PATH_ROOT", root.path());
        write_config(root.path(), content);
        test(root.path());
        match previous_root {
            Some(value) => std::env::set_var("GOOSE_PATH_ROOT", value),
            None => std::env::remove_var("GOOSE_PATH_ROOT"),
        }
        for (key, value) in previous_overrides {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    #[serial]
    fn openai_preview_is_allowlisted_and_redacts_secrets() {
        with_fixture(
            "active_provider: openai\nproviders:\n  openai:\n    enabled: true\n    model: gpt-4o\n    configured: true\nOPENAI_API_KEY: should-not-display\nproject: keep-me\n",
            |_root| {
                let preview = preview_goose_provider_config().unwrap();
                assert!(preview.blocked_reason.is_none());
                assert!(preview.changed);
                assert!(preview.proposed_state.contains(HEADROOM_OPENAI_BASE_URL));
                assert!(preview.proposed_state.contains(OPENAI_BASE_PATH));
                assert!(!preview.current_state.contains("should-not-display"));
                assert!(preview.proposed_state.contains("keep-me"));
                assert_eq!(preview.provider.as_deref(), Some("openai"));
                assert_eq!(preview.model.as_deref(), Some("gpt-4o"));
            },
        );
    }

    #[test]
    #[serial]
    fn apply_verify_and_off_preserve_provider_model_and_unmanaged_values() {
        with_fixture(
            "active_provider: anthropic\nproviders:\n  anthropic:\n    enabled: true\n    model: claude-sonnet-4\n    configured: true\nkeep: true\n",
            |root| {
                let preview = preview_goose_provider_config().unwrap();
                let (changed, backups) = configure_goose_provider_config().unwrap();
                assert_eq!(changed.len(), 1);
                assert_eq!(backups.len(), 1);
                assert!(goose_provider_config_matches().unwrap());
                let after = fs::read_to_string(root.join("config").join(GOOSE_CONFIG_FILE)).unwrap();
                assert!(after.contains("active_provider: anthropic"));
                assert!(after.contains("model: claude-sonnet-4"));
                assert!(after.contains("keep: true"));
                assert!(after.contains(HEADROOM_ANTHROPIC_BASE_URL));
                assert_eq!(preview.confirmation_phrase(), goose_apply_confirmation_phrase(&preview.current_state));

                let removed = remove_goose_provider_config().unwrap();
                assert_eq!(removed.len(), 1);
                assert!(!goose_provider_config_matches().unwrap());
                let cleaned = fs::read_to_string(root.join("config").join(GOOSE_CONFIG_FILE)).unwrap();
                assert!(cleaned.contains("active_provider: anthropic"));
                assert!(cleaned.contains("model: claude-sonnet-4"));
                assert!(cleaned.contains("keep: true"));
                assert!(!cleaned.contains(HEADROOM_ANTHROPIC_BASE_URL));
            },
        );
    }

    #[test]
    #[serial]
    fn unsupported_provider_and_user_endpoint_are_blocked_without_writes() {
        with_fixture(
            "active_provider: openrouter\nproviders:\n  openrouter:\n    enabled: true\n    model: openrouter/auto\n    configured: true\n",
            |root| {
                let path = root.join("config").join(GOOSE_CONFIG_FILE);
                let before = fs::read_to_string(&path).unwrap();
                let preview = preview_goose_provider_config().unwrap();
                assert!(preview.blocked_reason.is_some());
                assert_eq!(fs::read_to_string(&path).unwrap(), before);
            },
        );

        with_fixture(
            "active_provider: openai\nproviders:\n  openai:\n    enabled: true\n    model: gpt-4o\n    configured: true\nOPENAI_BASE_URL: https://proxy.example.test/v1\n",
            |root| {
                let path = root.join("config").join(GOOSE_CONFIG_FILE);
                let before = fs::read_to_string(&path).unwrap();
                let preview = preview_goose_provider_config().unwrap();
                assert!(preview.blocked_reason.is_some());
                assert_eq!(fs::read_to_string(&path).unwrap(), before);
            },
        );
    }

    #[test]
    #[serial]
    fn env_provider_override_is_blocked() {
        with_fixture(
            "active_provider: openai\nproviders:\n  openai:\n    model: gpt-4o\n",
            |_root| {
                let previous = std::env::var_os("GOOSE_PROVIDER");
                std::env::set_var("GOOSE_PROVIDER", "anthropic");
                let preview = preview_goose_provider_config().unwrap();
                assert!(preview
                    .blocked_reason
                    .as_deref()
                    .unwrap()
                    .contains("GOOSE_PROVIDER"));
                match previous {
                    Some(value) => std::env::set_var("GOOSE_PROVIDER", value),
                    None => std::env::remove_var("GOOSE_PROVIDER"),
                }
            },
        );
    }

    trait PreviewPhrase {
        fn confirmation_phrase(&self) -> String;
    }

    impl PreviewPhrase for GooseProviderConfigPreview {
        fn confirmation_phrase(&self) -> String {
            goose_apply_confirmation_phrase(&self.current_state)
        }
    }
}
