//! Native Aider provider routing.
//!
//! Aider documents `~/.aider.conf.yml` with an `openai-api-base` field for
//! OpenAI-compatible endpoints. This adapter sets only that allowlisted field
//! to the local Headroom proxy. It never reads or writes API keys, `api-key`,
//! `set-env`, or other provider settings.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};

use crate::client_paths::aider_config_path;
use crate::client_provider_configs::HEADROOM_OPENAI_BASE_URL;
use crate::managed_files::backup_if_exists;

pub(crate) const AIDER_NATIVE_MARKER: &str = "ai-switchboard:aider-provider";
pub(crate) const AIDER_NATIVE_APPLY_RECORD_ID: &str = "aider-provider-routing";
pub(crate) const AIDER_NATIVE_OWNER: &str = "Aider provider routing";
pub(crate) const AIDER_OPENAI_API_BASE_KEY: &str = "openai-api-base";

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

pub(crate) fn aider_config_backup_pattern() -> String {
    let path = aider_config_path();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(".aider.conf.yml");
    format!("{file_name}.headroom-backup-*")
}

pub(crate) fn aider_apply_confirmation_phrase(marker: &str, current_state: &str) -> String {
    format!(
        "Apply {marker} to {} after reviewing {}",
        aider_config_path().display(),
        short_state_hash(current_state)
    )
}

fn read_config(path: &PathBuf) -> Result<(Mapping, String)> {
    if !path.exists() {
        return Ok((Mapping::new(), String::new()));
    }
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let value = serde_yaml::from_str::<Value>(&raw)
        .with_context(|| format!("parsing Aider YAML config {}", path.display()))?;
    let mapping = value.as_mapping().cloned().ok_or_else(|| {
        anyhow!(
            "{} must contain a YAML mapping at the top level before Switchboard can manage Aider.",
            path.display()
        )
    })?;
    Ok((mapping, raw))
}

fn managed_openai_api_base_matches(mapping: &Mapping) -> bool {
    scalar_string(mapping, AIDER_OPENAI_API_BASE_KEY).as_deref() == Some(HEADROOM_OPENAI_BASE_URL)
}

fn upsert_openai_api_base(root: &mut Mapping) -> Result<bool> {
    if managed_openai_api_base_matches(root) {
        return Ok(false);
    }

    if let Some(existing) = scalar_string(root, AIDER_OPENAI_API_BASE_KEY) {
        return Err(anyhow!(
            "{} already defines {AIDER_OPENAI_API_BASE_KEY}; refusing to overwrite an unmanaged Aider endpoint.",
            aider_config_path().display()
        ));
    }

    root.insert(
        string_key(AIDER_OPENAI_API_BASE_KEY),
        Value::String(HEADROOM_OPENAI_BASE_URL.to_string()),
    );
    Ok(true)
}

pub(crate) fn aider_next_provider_config() -> Result<(String, bool)> {
    let path = aider_config_path();
    let (mut root, raw) = read_config(&path)?;
    let changed = upsert_openai_api_base(&mut root)?;
    if !changed {
        return Ok((raw, false));
    }
    let next = serde_yaml::to_string(&Value::Mapping(root))
        .context("serializing Aider provider preview")?;
    Ok((next, true))
}

pub(crate) fn configure_aider_provider_config() -> Result<(Vec<String>, Vec<String>)> {
    let path = aider_config_path();
    let (next_config, changed) = aider_next_provider_config()?;
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
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

pub(crate) fn aider_provider_config_matches() -> Result<bool> {
    let path = aider_config_path();
    if !path.exists() {
        return Ok(false);
    }
    let (root, _) = read_config(&path)?;
    Ok(managed_openai_api_base_matches(&root))
}

pub(crate) fn remove_aider_provider_config() -> Result<Vec<String>> {
    let path = aider_config_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let (mut root, raw) = read_config(&path)?;
    if !managed_openai_api_base_matches(&root) {
        return Ok(Vec::new());
    }
    root.remove(&string_key(AIDER_OPENAI_API_BASE_KEY));
    let next = serde_yaml::to_string(&Value::Mapping(root))
        .context("serializing Aider provider cleanup")?;
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
            "ai-switchboard-aider-native-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("fixture home");
        path
    }

    fn with_aider_home<F>(label: &str, run: F)
    where
        F: FnOnce(PathBuf),
    {
        let home = fixture_home(label);
        let config = home.join(".aider.conf.yml");
        let prev_home = std::env::var_os("HOME");
        let prev_aider = std::env::var_os("AIDER_CONFIG_PATH");
        std::env::set_var("HOME", &home);
        std::env::set_var("AIDER_CONFIG_PATH", &config);
        run(config);
        match prev_aider {
            Some(value) => std::env::set_var("AIDER_CONFIG_PATH", value),
            None => std::env::remove_var("AIDER_CONFIG_PATH"),
        }
        match prev_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    #[serial_test::serial]
    fn sets_openai_api_base_without_touching_existing_keys() {
        with_aider_home("append", |config| {
            fs::write(
                &config,
                r#"openai-api-key: secret-should-stay
model: gpt-4o
"#,
            )
            .expect("seed config");

            let (next, changed) = aider_next_provider_config().expect("preview");
            assert!(changed);
            assert!(next.contains("secret-should-stay"));
            assert!(next.contains("gpt-4o"));
            assert!(next.contains(AIDER_OPENAI_API_BASE_KEY));
            assert!(next.contains(HEADROOM_OPENAI_BASE_URL));

            configure_aider_provider_config().expect("apply");
            assert!(aider_provider_config_matches().expect("matches"));
            let applied = fs::read_to_string(&config).expect("read applied");
            assert!(applied.contains("secret-should-stay"));
            assert!(!applied.contains("secret-should-stay: overwritten"));

            remove_aider_provider_config().expect("remove");
            assert!(!aider_provider_config_matches().expect("removed"));
            let restored = fs::read_to_string(&config).expect("read removed");
            assert!(restored.contains("secret-should-stay"));
            assert!(!restored.contains(HEADROOM_OPENAI_BASE_URL));
        });
    }

    #[test]
    #[serial_test::serial]
    fn refuses_conflicting_openai_api_base() {
        with_aider_home("conflict", |config| {
            fs::write(&config, "openai-api-base: https://api.openai.com/v1\n")
                .expect("seed config");

            let error = aider_next_provider_config().expect_err("conflict");
            assert!(error.to_string().contains("refusing to overwrite"));
        });
    }
}
