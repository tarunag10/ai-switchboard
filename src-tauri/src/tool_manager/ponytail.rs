use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::models::ToolStatus;
use crate::ponytail_bundled::{skill_ids, verify_bundled_ponytail, PONYTAIL_SOURCE_COMMIT};
use crate::process_runner::run_command_streaming;

use super::ToolManager;

pub(super) use crate::ponytail_bundled::PONYTAIL_DISPLAY_VERSION;

const PONYTAIL_PLUGIN_REF: &str = "ponytail@ponytail";
const PONYTAIL_DELIVERY: &str = "bundled_guidance";

fn is_bundled_receipt(receipt: &Value) -> bool {
    receipt.get("delivery").and_then(Value::as_str) == Some(PONYTAIL_DELIVERY)
}

fn receipt_string_map(receipt: &Value, key: &str) -> BTreeMap<String, String> {
    receipt
        .get(key)
        .and_then(Value::as_object)
        .map(|values| {
            values
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn receipt_string_list(receipt: &Value, key: &str) -> Vec<String> {
    receipt
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn created_fingerprints(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    after
        .iter()
        .filter(|(client, _)| !before.contains_key(*client))
        .map(|(client, fingerprint)| (client.clone(), fingerprint.clone()))
        .collect()
}

impl ToolManager {
    fn current_ponytail_guidance(&self) -> Result<BTreeMap<String, String>> {
        crate::client_integrations::ponytail_integration_fingerprints()
    }

    fn bundled_receipt(
        enabled: bool,
        owned_clients: &BTreeMap<String, String>,
        legacy_cleanup_pending: &[String],
    ) -> Value {
        json!({
            "version": PONYTAIL_DISPLAY_VERSION,
            "enabled": enabled,
            "delivery": PONYTAIL_DELIVERY,
            "sourceCommit": PONYTAIL_SOURCE_COMMIT,
            "bundledResources": skill_ids(),
            "activeProfile": "ponytail",
            "commandsExposed": false,
            "ownedClients": owned_clients,
            "legacyCleanupPending": legacy_cleanup_pending,
        })
    }

    fn receipt_owned_clients(receipt: &Value) -> BTreeMap<String, String> {
        receipt_string_map(receipt, "ownedClients")
    }

    fn legacy_cleanup_hosts(receipt: &Value) -> Vec<LegacyPluginHost> {
        let key = if is_bundled_receipt(receipt) {
            "legacyCleanupPending"
        } else {
            "ownedHosts"
        };
        receipt_string_list(receipt, key)
            .into_iter()
            .filter_map(|host| LegacyPluginHost::from_label(&host))
            .collect()
    }

    fn cleanup_legacy_hosts(&self, hosts: &[LegacyPluginHost]) -> Result<()> {
        let mut failures = Vec::new();
        for host in hosts {
            if host.plugin_fingerprint().is_none() {
                continue;
            }
            let Some(cli) = host.cli() else {
                failures.push(format!("{} CLI is unavailable", host.label()));
                continue;
            };
            let label = host.label();
            if let Err(error) = run_command_streaming(
                &cli,
                host.uninstall_args(),
                &self.runtime.root_dir,
                &mut |line: &str| log::info!("legacy ponytail cleanup [{label}]: {line}"),
            ) {
                failures.push(format!("{label}: {error:#}"));
            } else if host.plugin_fingerprint().is_some() {
                failures.push(format!("{label}: plugin entry remains after removal"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            bail!(
                "legacy Ponytail plugin cleanup is pending: {}",
                failures.join("; ")
            )
        }
    }

    fn migrate_legacy_receipt(&self, enabled: bool, legacy: &Value) -> Result<()> {
        verify_bundled_ponytail()?;
        let existing = self.current_ponytail_guidance()?;
        if !existing.is_empty() {
            bail!("Ponytail managed guidance exists without bundled ownership metadata");
        }
        let owned_clients = if enabled {
            crate::client_integrations::enable_ponytail_integration()?;
            self.current_ponytail_guidance()?
        } else {
            BTreeMap::new()
        };
        let legacy_hosts = Self::legacy_cleanup_hosts(legacy);
        let pending: Vec<String> = legacy_hosts
            .iter()
            .map(|host| host.label().to_string())
            .collect();
        if let Err(error) = self.write_tool_receipt(
            "ponytail",
            Self::bundled_receipt(enabled, &owned_clients, &pending),
        ) {
            if enabled {
                let _ = crate::client_integrations::disable_ponytail_integration_if_unchanged(
                    &owned_clients,
                );
            }
            return Err(error);
        }
        self.cleanup_legacy_hosts(&legacy_hosts)?;
        self.write_tool_receipt(
            "ponytail",
            Self::bundled_receipt(enabled, &owned_clients, &[]),
        )
    }

    fn finish_pending_legacy_cleanup(&self, receipt: &Value) -> Result<Value> {
        let hosts = Self::legacy_cleanup_hosts(receipt);
        if hosts.is_empty() {
            return Ok(receipt.clone());
        }
        self.cleanup_legacy_hosts(&hosts)?;
        let enabled = receipt
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let owned = Self::receipt_owned_clients(receipt);
        let completed = Self::bundled_receipt(enabled, &owned, &[]);
        self.write_tool_receipt("ponytail", completed.clone())?;
        Ok(completed)
    }

    pub fn smoke_test_ponytail(&self) -> Result<()> {
        let Some(receipt) = self.ponytail_receipt_snapshot() else {
            return Ok(());
        };
        verify_bundled_ponytail()?;
        if !is_bundled_receipt(&receipt) {
            bail!("Ponytail still has a legacy marketplace receipt");
        }
        if !receipt_string_list(&receipt, "legacyCleanupPending").is_empty() {
            bail!("Ponytail legacy plugin cleanup is pending");
        }
        let expected = Self::receipt_owned_clients(&receipt);
        let current = self.current_ponytail_guidance()?;
        if current != expected {
            bail!("Ponytail managed guidance differs from its ownership receipt");
        }
        let enabled = receipt
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if enabled && !crate::client_integrations::ponytail_integration_matches()? {
            bail!("Ponytail is enabled but its bundled managed guidance is missing or stale");
        }
        if !enabled && !current.is_empty() {
            bail!("Ponytail is disabled but managed guidance remains active");
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn ponytail_installed(&self) -> bool {
        self.ponytail_receipt_exists()
    }

    pub fn ponytail_registered_hosts(&self) -> Vec<String> {
        self.current_ponytail_guidance()
            .unwrap_or_default()
            .into_keys()
            .map(|client| match client.as_str() {
                "claude-code" => "Claude Code".to_string(),
                "codex" => "Codex".to_string(),
                _ => client,
            })
            .collect()
    }

    /// Content-free fingerprints of Switchboard-owned managed guidance blocks.
    /// The method name remains for schema-4 selective receipt compatibility.
    pub fn ponytail_host_fingerprints(&self) -> BTreeMap<String, String> {
        self.current_ponytail_guidance().unwrap_or_default()
    }

    pub fn legacy_ponytail_host_fingerprints(&self) -> BTreeMap<String, String> {
        LegacyPluginHost::ALL
            .into_iter()
            .filter_map(|host| {
                host.plugin_fingerprint()
                    .map(|fingerprint| (host.id().to_string(), fingerprint))
            })
            .collect()
    }

    pub fn ponytail_receipt_snapshot(&self) -> Option<Value> {
        self.read_tool_receipt("ponytail")
    }

    pub fn ponytail_receipt_exists(&self) -> bool {
        self.runtime.tools_dir.join("ponytail.json").exists()
    }

    pub fn ponytail_requires_legacy_migration(&self) -> bool {
        self.ponytail_receipt_snapshot()
            .as_ref()
            .is_some_and(|receipt| {
                !is_bundled_receipt(receipt)
                    || !receipt_string_list(receipt, "legacyCleanupPending").is_empty()
            })
    }

    pub fn install_ponytail(&self) -> Result<()> {
        verify_bundled_ponytail()?;
        if self.ponytail_receipt_exists() {
            return self.set_ponytail_enabled(true);
        }
        let before = self.current_ponytail_guidance()?;
        if !before.is_empty() {
            bail!("Ponytail managed guidance exists without an ownership receipt");
        }
        crate::client_integrations::enable_ponytail_integration()?;
        let after = self.current_ponytail_guidance()?;
        if let Err(error) =
            self.write_tool_receipt("ponytail", Self::bundled_receipt(true, &after, &[]))
        {
            let _ = crate::client_integrations::disable_ponytail_integration_if_unchanged(&after);
            return Err(error);
        }
        Ok(())
    }

    pub fn set_ponytail_enabled(&self, enabled: bool) -> Result<()> {
        let Some(receipt) = self.ponytail_receipt_snapshot() else {
            bail!("ponytail is not installed");
        };
        verify_bundled_ponytail()?;
        if !is_bundled_receipt(&receipt) {
            return self.migrate_legacy_receipt(enabled, &receipt);
        }
        let receipt = self.finish_pending_legacy_cleanup(&receipt)?;
        let expected = Self::receipt_owned_clients(&receipt);
        let current = self.current_ponytail_guidance()?;
        if current != expected {
            bail!("Ponytail managed guidance changed after activation; it was preserved");
        }

        if enabled {
            crate::client_integrations::enable_ponytail_integration()?;
            let after = self.current_ponytail_guidance()?;
            let created = created_fingerprints(&current, &after);
            if let Err(error) =
                self.write_tool_receipt("ponytail", Self::bundled_receipt(true, &after, &[]))
            {
                let _ =
                    crate::client_integrations::disable_ponytail_integration_if_unchanged(&created);
                return Err(error);
            }
        } else {
            crate::client_integrations::disable_ponytail_integration_if_unchanged(&expected)?;
            if let Err(error) = self.write_tool_receipt(
                "ponytail",
                Self::bundled_receipt(false, &BTreeMap::new(), &[]),
            ) {
                let _ = crate::client_integrations::enable_ponytail_integration();
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn uninstall_ponytail(&self) -> Result<()> {
        let Some(receipt) = self.ponytail_receipt_snapshot() else {
            return Ok(());
        };
        if !is_bundled_receipt(&receipt) {
            self.migrate_legacy_receipt(false, &receipt)?;
        }
        let receipt = self
            .ponytail_receipt_snapshot()
            .context("Ponytail receipt disappeared during migration")?;
        let receipt = self.finish_pending_legacy_cleanup(&receipt)?;
        let expected = Self::receipt_owned_clients(&receipt);
        let current = self.current_ponytail_guidance()?;
        if current != expected {
            bail!("Ponytail managed guidance changed after activation; it was preserved");
        }
        let was_enabled = receipt
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        crate::client_integrations::disable_ponytail_integration_if_unchanged(&expected)?;
        let receipt_path = self.runtime.tools_dir.join("ponytail.json");
        if let Err(error) = std::fs::remove_file(&receipt_path) {
            if was_enabled {
                let _ = crate::client_integrations::enable_ponytail_integration();
            }
            return Err(error).with_context(|| format!("removing {}", receipt_path.display()));
        }
        Ok(())
    }

    pub fn remove_ponytail_host_if_unchanged(
        &self,
        host_id: &str,
        expected_fingerprint: &str,
    ) -> Result<()> {
        crate::client_integrations::remove_ponytail_client_if_unchanged(
            host_id,
            expected_fingerprint,
        )
    }

    pub fn remove_legacy_ponytail_host_if_unchanged(
        &self,
        host_id: &str,
        expected_fingerprint: &str,
    ) -> Result<()> {
        let host = LegacyPluginHost::from_id(host_id)
            .with_context(|| format!("unknown legacy Ponytail host identifier: {host_id}"))?;
        if host.plugin_fingerprint().as_deref() != Some(expected_fingerprint) {
            bail!("legacy Ponytail plugin entry changed after activation for {host_id}");
        }
        let cli = host
            .cli()
            .context("legacy Ponytail host CLI is unavailable")?;
        let label = host.label();
        run_command_streaming(
            &cli,
            host.uninstall_args(),
            &self.runtime.root_dir,
            &mut |line: &str| log::info!("legacy ponytail rollback [{label}]: {line}"),
        )?;
        if host.plugin_fingerprint().is_some() {
            bail!("legacy Ponytail plugin entry remains after rollback for {host_id}");
        }
        Ok(())
    }

    pub fn restore_ponytail_receipt_if_unchanged(
        &self,
        previous_receipt: Option<&Value>,
        after_receipt: Option<&Value>,
    ) -> Result<()> {
        if self.ponytail_receipt_snapshot().as_ref() != after_receipt {
            bail!("Ponytail managed receipt changed after activation");
        }
        let receipt_path = self.runtime.tools_dir.join("ponytail.json");
        if let Some(previous_receipt) = previous_receipt {
            self.write_tool_receipt("ponytail", previous_receipt.clone())?;
        } else if receipt_path.exists() {
            std::fs::remove_file(&receipt_path)
                .with_context(|| format!("removing {}", receipt_path.display()))?;
        }
        Ok(())
    }

    pub(super) fn ponytail_status(&self) -> ToolStatus {
        let Some(receipt) = self.ponytail_receipt_snapshot() else {
            return ToolStatus::NotInstalled;
        };
        if verify_bundled_ponytail().is_err()
            || !is_bundled_receipt(&receipt)
            || !receipt_string_list(&receipt, "legacyCleanupPending").is_empty()
        {
            return ToolStatus::Degraded;
        }
        let expected = Self::receipt_owned_clients(&receipt);
        let Ok(current) = self.current_ponytail_guidance() else {
            return ToolStatus::Degraded;
        };
        if current != expected {
            return ToolStatus::Degraded;
        }
        let enabled = receipt
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if !enabled {
            return if current.is_empty() {
                ToolStatus::Healthy
            } else {
                ToolStatus::Degraded
            };
        }
        match crate::client_integrations::ponytail_integration_matches() {
            Ok(true) if !expected.is_empty() => ToolStatus::Healthy,
            Ok(_) | Err(_) => ToolStatus::Degraded,
        }
    }

    pub(super) fn installed_ponytail_version(&self) -> Option<String> {
        self.ponytail_receipt_snapshot().and_then(|receipt| {
            receipt
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LegacyPluginHost {
    ClaudeCode,
    Codex,
}

impl LegacyPluginHost {
    const ALL: [Self; 2] = [Self::ClaudeCode, Self::Codex];

    fn from_id(value: &str) -> Option<Self> {
        match value {
            "claude-code" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    fn from_label(value: &str) -> Option<Self> {
        match value {
            "Claude Code" => Some(Self::ClaudeCode),
            "Codex" => Some(Self::Codex),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
        }
    }

    fn cli(self) -> Option<std::path::PathBuf> {
        match self {
            Self::ClaudeCode => crate::claude_cli::detect_claude_cli(),
            Self::Codex => crate::claude_cli::detect_codex_cli(),
        }
    }

    fn uninstall_args(self) -> &'static [&'static str] {
        match self {
            Self::ClaudeCode => &["plugin", "uninstall", PONYTAIL_PLUGIN_REF],
            Self::Codex => &["plugin", "remove", PONYTAIL_PLUGIN_REF],
        }
    }

    fn plugin_fingerprint(self) -> Option<String> {
        match self {
            Self::ClaudeCode => claude_legacy_plugin_entry().map(|entry| json_fingerprint(&entry)),
            Self::Codex => codex_legacy_plugin_block().map(text_fingerprint),
        }
    }
}

fn claude_legacy_plugin_entry() -> Option<Value> {
    let path = dirs::home_dir()?
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");
    let value: Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let entry = value.get("plugins")?.get(PONYTAIL_PLUGIN_REF)?.clone();
    entry
        .as_array()
        .is_some_and(|entries| !entries.is_empty())
        .then_some(entry)
}

fn codex_legacy_plugin_block() -> Option<String> {
    let path = dirs::home_dir()?.join(".codex").join("config.toml");
    let text = std::fs::read_to_string(path).ok()?;
    let header = format!("[plugins.\"{PONYTAIL_PLUGIN_REF}\"]");
    let mut offset = 0;
    let mut start = None;
    for segment in text.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        if start.is_some() && line.trim_start().starts_with('[') {
            return start.map(|start| text[start..offset].to_string());
        }
        if start.is_none() && line.trim_start() == header {
            start = Some(offset);
        }
        offset += segment.len();
    }
    start.map(|start| text[start..].to_string())
}

fn json_fingerprint(value: &Value) -> String {
    let payload = serde_json::to_vec(value).expect("serializing legacy Ponytail entry");
    bytes_fingerprint(&payload)
}

fn text_fingerprint(value: String) -> String {
    bytes_fingerprint(value.as_bytes())
}

fn bytes_fingerprint(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::{
        codex_legacy_plugin_block, created_fingerprints, is_bundled_receipt, receipt_string_map,
        LegacyPluginHost,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn bundled_receipt_detection_keeps_legacy_migration_explicit() {
        assert!(is_bundled_receipt(
            &json!({ "delivery": "bundled_guidance" })
        ));
        assert!(!is_bundled_receipt(&json!({ "version": "latest" })));
    }

    #[test]
    fn owned_client_maps_are_content_free_and_created_delta_is_exact() {
        let receipt = json!({ "ownedClients": { "codex": "sha256:one" } });
        assert_eq!(receipt_string_map(&receipt, "ownedClients").len(), 1);
        let before = BTreeMap::from([("codex".to_string(), "sha256:one".to_string())]);
        let after = BTreeMap::from([
            ("claude-code".to_string(), "sha256:two".to_string()),
            ("codex".to_string(), "sha256:one".to_string()),
        ]);
        assert_eq!(
            created_fingerprints(&before, &after),
            BTreeMap::from([("claude-code".to_string(), "sha256:two".to_string())])
        );
    }

    #[test]
    fn legacy_host_ids_and_labels_remain_receipt_compatible() {
        assert_eq!(
            LegacyPluginHost::from_label("Claude Code"),
            Some(LegacyPluginHost::ClaudeCode)
        );
        assert_eq!(
            LegacyPluginHost::from_id("codex"),
            Some(LegacyPluginHost::Codex)
        );
        assert!(LegacyPluginHost::from_label("unknown").is_none());
        let _ = codex_legacy_plugin_block();
    }
}
