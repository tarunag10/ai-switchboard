//! Native Continue provider routing.
//!
//! Continue documents `~/.continue/config.yaml` with a `models` array whose
//! entries may set `provider`, `model`, and `apiBase`. This adapter adds or
//! updates one OpenAI-compatible Headroom model entry. It never reads or
//! writes `apiKey`, secrets, account state, or unrelated model entries.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};

use crate::client_paths::continue_config_path;
use crate::client_provider_configs::HEADROOM_OPENAI_BASE_URL;
use crate::managed_files::backup_if_exists;

pub(crate) const CONTINUE_NATIVE_MARKER: &str = "ai-switchboard:continue-provider";
pub(crate) const CONTINUE_NATIVE_APPLY_RECORD_ID: &str = "continue-provider-routing";
pub(crate) const CONTINUE_NATIVE_OWNER: &str = "Continue provider routing";
pub(crate) const CONTINUE_HEADROOM_MODEL_NAME: &str = "AI Switchboard";
pub(crate) const CONTINUE_HEADROOM_MODEL_ID: &str = "headroom";
pub(crate) const CONTINUE_HEADROOM_PROVIDER: &str = "openai";

fn string_key(key: &str) -> Value {
    Value::String(key.to_string())
}

fn scalar_string(mapping: &Mapping, key: &str) -> Option<String> {
    mapping
        .get(string_key(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn short_state_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn continue_config_backup_pattern() -> String {
    let path = continue_config_path();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.yaml");
    format!("{file_name}.headroom-backup-*")
}

pub(crate) fn continue_apply_confirmation_phrase(marker: &str, current_state: &str) -> String {
    format!(
        "Apply {marker} to {} after reviewing {}",
        continue_config_path().display(),
        short_state_hash(current_state)
    )
}

fn headroom_model_mapping() -> Mapping {
    let mut model = Mapping::new();
    model.insert(
        string_key("name"),
        Value::String(CONTINUE_HEADROOM_MODEL_NAME.to_string()),
    );
    model.insert(
        string_key("provider"),
        Value::String(CONTINUE_HEADROOM_PROVIDER.to_string()),
    );
    model.insert(
        string_key("model"),
        Value::String(CONTINUE_HEADROOM_MODEL_ID.to_string()),
    );
    model.insert(
        string_key("apiBase"),
        Value::String(HEADROOM_OPENAI_BASE_URL.to_string()),
    );
    model.insert(
        string_key("roles"),
        Value::Sequence(vec![
            Value::String("chat".to_string()),
            Value::String("edit".to_string()),
            Value::String("apply".to_string()),
        ]),
    );
    model
}

fn headroom_model_value() -> Value {
    Value::Mapping(headroom_model_mapping())
}

fn model_entry_matches(entry: &Value) -> bool {
    let Some(mapping) = entry.as_mapping() else {
        return false;
    };
    scalar_string(mapping, "name").as_deref() == Some(CONTINUE_HEADROOM_MODEL_NAME)
        && scalar_string(mapping, "provider").as_deref() == Some(CONTINUE_HEADROOM_PROVIDER)
        && scalar_string(mapping, "model").as_deref() == Some(CONTINUE_HEADROOM_MODEL_ID)
        && scalar_string(mapping, "apiBase").as_deref() == Some(HEADROOM_OPENAI_BASE_URL)
}

fn model_entry_conflicts(entry: &Value) -> bool {
    let Some(mapping) = entry.as_mapping() else {
        return false;
    };
    if scalar_string(mapping, "name").as_deref() != Some(CONTINUE_HEADROOM_MODEL_NAME) {
        return false;
    }
    !model_entry_matches(entry)
}

fn read_config(path: &PathBuf) -> Result<(Mapping, String)> {
    if !path.exists() {
        return Ok((Mapping::new(), String::new()));
    }
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let value = serde_yaml::from_str::<Value>(&raw)
        .with_context(|| format!("parsing Continue YAML config {}", path.display()))?;
    let mapping = value
        .as_mapping()
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "{} must contain a YAML mapping at the top level before Switchboard can manage Continue.",
                path.display()
            )
        })?;
    Ok((mapping, raw))
}

fn ensure_required_root_fields(root: &mut Mapping) {
    if !root.contains_key(string_key("name")) {
        root.insert(
            string_key("name"),
            Value::String("AI Switchboard Continue Routing".to_string()),
        );
    }
    if !root.contains_key(string_key("version")) {
        root.insert(string_key("version"), Value::String("0.0.1".to_string()));
    }
    if !root.contains_key(string_key("schema")) {
        root.insert(string_key("schema"), Value::String("v1".to_string()));
    }
}

fn upsert_headroom_model(root: &mut Mapping) -> Result<bool> {
    ensure_required_root_fields(root);
    let models = root
        .entry(string_key("models"))
        .or_insert_with(|| Value::Sequence(Vec::new()));
    let sequence = models
        .as_sequence_mut()
        .ok_or_else(|| anyhow!("Continue config models key must be an array."))?;

    if let Some(index) = sequence.iter().position(model_entry_matches) {
        if sequence[index] == headroom_model_value() {
            return Ok(false);
        }
        return Err(anyhow!(
            "Continue config contains a drifted Switchboard model entry named '{CONTINUE_HEADROOM_MODEL_NAME}'; refusing to overwrite it."
        ));
    }

    if sequence.iter().any(model_entry_conflicts) {
        return Err(anyhow!(
            "Continue config already defines a conflicting model named '{CONTINUE_HEADROOM_MODEL_NAME}'; refusing to overwrite it."
        ));
    }

    sequence.push(headroom_model_value());
    Ok(true)
}

pub(crate) fn continue_next_provider_config() -> Result<(String, bool)> {
    let path = continue_config_path();
    let (mut root, raw) = read_config(&path)?;
    let changed = upsert_headroom_model(&mut root)?;
    if !changed {
        return Ok((raw, false));
    }
    let next = serde_yaml::to_string(&Value::Mapping(root))
        .context("serializing Continue provider preview")?;
    Ok((next, true))
}

pub(crate) fn configure_continue_provider_config() -> Result<(Vec<String>, Vec<String>)> {
    let path = continue_config_path();
    let (next_config, changed) = continue_next_provider_config()?;
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let backup = backup_if_exists(&path)?;
    std::fs::write(&path, next_config).with_context(|| format!("writing {}", path.display()))?;
    Ok((
        vec![path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

pub(crate) fn continue_provider_config_matches() -> Result<bool> {
    let path = continue_config_path();
    if !path.exists() {
        return Ok(false);
    }
    let (root, _) = read_config(&path)?;
    let Some(models) = root.get(string_key("models")).and_then(Value::as_sequence) else {
        return Ok(false);
    };
    Ok(models.iter().any(model_entry_matches))
}

pub(crate) fn remove_continue_provider_config() -> Result<Vec<String>> {
    let path = continue_config_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let (mut root, raw) = read_config(&path)?;
    let Some(models) = root.get_mut(string_key("models")).and_then(Value::as_sequence_mut) else {
        return Ok(Vec::new());
    };
    let before = models.len();
    models.retain(|entry| !model_entry_matches(entry));
    if models.len() == before {
        return Ok(Vec::new());
    }

    let next = serde_yaml::to_string(&Value::Mapping(root))
        .context("serializing Continue provider cleanup")?;
    if next == raw {
        return Ok(Vec::new());
    }
    let _ = backup_if_exists(&path)?;
    std::fs::write(&path, next).with_context(|| format!("writing {}", path.display()))?;
    Ok(vec![path.display().to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_home(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "mac-ai-switchboard-continue-native-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("fixture home");
        path
    }

    fn with_continue_home<F>(label: &str, run: F)
    where
        F: FnOnce(PathBuf),
    {
        let home = fixture_home(label);
        let continue_root = home.join(".continue");
        let prev_home = std::env::var_os("HOME");
        let prev_continue = std::env::var_os("CONTINUE_PATH_ROOT");
        std::env::set_var("HOME", &home);
        std::env::set_var("CONTINUE_PATH_ROOT", &continue_root);
        run(continue_root);
        match prev_continue {
            Some(value) => std::env::set_var("CONTINUE_PATH_ROOT", value),
            None => std::env::remove_var("CONTINUE_PATH_ROOT"),
        }
        match prev_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    #[serial_test::serial]
    fn adds_headroom_model_without_touching_existing_models() {
        with_continue_home("append", |continue_root| {
        let config = continue_root.join("config.yaml");
        fs::create_dir_all(config.parent().unwrap()).expect("continue dir");
        fs::write(
            &config,
            r#"name: User Config
version: 1.0.0
schema: v1
models:
  - name: GPT-4o
    provider: openai
    model: gpt-4o
    apiKey: secret-should-stay
"#,
        )
        .expect("seed config");

        let (next, changed) = continue_next_provider_config().expect("preview");
        assert!(changed);
        assert!(next.contains("GPT-4o"));
        assert!(next.contains("secret-should-stay"));
        assert!(next.contains(CONTINUE_HEADROOM_MODEL_NAME));
        assert!(next.contains(HEADROOM_OPENAI_BASE_URL));

        configure_continue_provider_config().expect("apply");
        assert!(continue_provider_config_matches().expect("matches"));
        let applied = fs::read_to_string(&config).expect("read applied");
        assert!(applied.contains("GPT-4o"));
        assert!(applied.contains("secret-should-stay"));

        remove_continue_provider_config().expect("remove");
        assert!(!continue_provider_config_matches().expect("removed"));
        let restored = fs::read_to_string(&config).expect("read removed");
        assert!(restored.contains("GPT-4o"));
        assert!(!restored.contains(CONTINUE_HEADROOM_MODEL_NAME));
        });
    }

    #[test]
    #[serial_test::serial]
    fn refuses_conflicting_model_name() {
        with_continue_home("conflict", |continue_root| {
        let config = continue_root.join("config.yaml");
        fs::create_dir_all(config.parent().unwrap()).expect("continue dir");
        fs::write(
            &config,
            r#"name: User Config
version: 1.0.0
schema: v1
models:
  - name: AI Switchboard
    provider: openai
    model: gpt-4o
    apiBase: https://api.openai.com/v1
"#,
        )
        .expect("seed config");

        let error = continue_next_provider_config().expect_err("conflict");
        assert!(error.to_string().contains("conflicting model"));
        });
    }
}
