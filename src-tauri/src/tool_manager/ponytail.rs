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

fn preserve_rollback_failure<T>(
    primary: anyhow::Error,
    rollback: Result<T>,
    action: &str,
) -> anyhow::Error {
    match rollback {
        Ok(_) => primary,
        Err(rollback_error) => {
            anyhow::anyhow!("{primary:#}; compensating {action} also failed: {rollback_error:#}")
        }
    }
}

fn rollback_ponytail_repair(
    created: &BTreeMap<String, String>,
    refresh: &crate::client_integrations::PonytailGuidanceRefresh,
) -> Result<()> {
    let mut failures = Vec::new();
    if let Err(error) =
        crate::client_integrations::disable_ponytail_integration_if_unchanged(created)
    {
        failures.push(format!("removing newly created guidance: {error:#}"));
    }
    if let Err(error) = crate::client_integrations::restore_ponytail_guidance_refresh(refresh) {
        failures.push(format!("restoring the previous profile: {error:#}"));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        bail!("Ponytail repair rollback failed: {}", failures.join("; "))
    }
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
            if host.plugin_fingerprint()?.is_none() {
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
            } else if host.plugin_fingerprint()?.is_some() {
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
                return Err(preserve_rollback_failure(
                    error,
                    crate::client_integrations::disable_ponytail_integration_if_unchanged(
                        &owned_clients,
                    ),
                    "managed-guidance removal",
                ));
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

    /// Fallible counterpart used by ownership-sensitive transactions. Unlike
    /// status-only callers, activation must never treat an unreadable client
    /// file as proof that no pre-existing guidance exists.
    pub fn ponytail_host_fingerprints_checked(&self) -> Result<BTreeMap<String, String>> {
        self.current_ponytail_guidance()
    }

    pub fn ponytail_host_fingerprint(&self, client_id: &str) -> Result<Option<String>> {
        crate::client_integrations::ponytail_integration_fingerprint(client_id)
    }

    pub fn legacy_ponytail_host_fingerprint(&self, host_id: &str) -> Result<Option<String>> {
        LegacyPluginHost::from_id(host_id)
            .with_context(|| format!("unknown legacy Ponytail host identifier: {host_id}"))?
            .plugin_fingerprint()
    }

    pub fn ponytail_receipt_snapshot(&self) -> Option<Value> {
        self.read_tool_receipt("ponytail")
    }

    pub fn ponytail_receipt_snapshot_checked(&self) -> Result<Option<Value>> {
        self.read_tool_receipt_checked("ponytail")
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
            return Err(preserve_rollback_failure(
                error,
                crate::client_integrations::disable_ponytail_integration_if_unchanged(&after),
                "managed-guidance removal",
            ));
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
                return Err(preserve_rollback_failure(
                    error,
                    crate::client_integrations::disable_ponytail_integration_if_unchanged(&created),
                    "new managed-guidance removal",
                ));
            }
        } else {
            let removed =
                crate::client_integrations::disable_ponytail_integration_if_unchanged(&expected)?;
            if let Err(error) = self.write_tool_receipt(
                "ponytail",
                Self::bundled_receipt(false, &BTreeMap::new(), &[]),
            ) {
                return Err(preserve_rollback_failure(
                    error,
                    crate::client_integrations::restore_ponytail_removal(&removed),
                    "exact receipt-owned guidance restoration",
                ));
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
        let removed =
            crate::client_integrations::disable_ponytail_integration_if_unchanged(&expected)?;
        let receipt_path = self.runtime.tools_dir.join("ponytail.json");
        if let Err(error) = std::fs::remove_file(&receipt_path) {
            return Err(preserve_rollback_failure(
                anyhow::Error::new(error).context(format!("removing {}", receipt_path.display())),
                crate::client_integrations::restore_ponytail_removal(&removed),
                "exact receipt-owned guidance restoration",
            ));
        }
        Ok(())
    }

    pub fn repair_ponytail(&self) -> Result<()> {
        let Some(receipt) = self.ponytail_receipt_snapshot() else {
            return self.install_ponytail();
        };
        if !is_bundled_receipt(&receipt) || self.ponytail_requires_legacy_migration() {
            return self.set_ponytail_enabled(true);
        }
        verify_bundled_ponytail()?;
        let expected = Self::receipt_owned_clients(&receipt);
        let before = self.current_ponytail_guidance()?;
        for (client_id, current_fingerprint) in &before {
            if expected.get(client_id) != Some(current_fingerprint) {
                bail!(
                    "Ponytail managed guidance changed outside its ownership receipt for {client_id}; it was preserved"
                );
            }
        }

        let refresh =
            crate::client_integrations::refresh_ponytail_guidance_if_unchanged(&expected)?;
        if let Err(error) = crate::client_integrations::enable_ponytail_integration() {
            return Err(preserve_rollback_failure(
                error,
                crate::client_integrations::restore_ponytail_guidance_refresh(&refresh),
                "previous-profile restoration",
            ));
        }
        let after = self.current_ponytail_guidance()?;
        let created = created_fingerprints(&before, &after);
        let still_missing: Vec<&String> = expected
            .keys()
            .filter(|client_id| !after.contains_key(*client_id))
            .collect();
        if !still_missing.is_empty() {
            return Err(preserve_rollback_failure(
                anyhow::anyhow!(
                    "Ponytail cannot restore disconnected clients: {}",
                    still_missing
                        .into_iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                rollback_ponytail_repair(&created, &refresh),
                "Ponytail repair rollback",
            ));
        }
        if let Err(error) =
            self.write_tool_receipt("ponytail", Self::bundled_receipt(true, &after, &[]))
        {
            return Err(preserve_rollback_failure(
                error,
                rollback_ponytail_repair(&created, &refresh),
                "Ponytail repair rollback",
            ));
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
        .map(|_| ())
    }

    pub fn remove_legacy_ponytail_host_if_unchanged(
        &self,
        host_id: &str,
        expected_fingerprint: &str,
    ) -> Result<()> {
        let host = LegacyPluginHost::from_id(host_id)
            .with_context(|| format!("unknown legacy Ponytail host identifier: {host_id}"))?;
        if host.plugin_fingerprint()?.as_deref() != Some(expected_fingerprint) {
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
        if host.plugin_fingerprint()?.is_some() {
            bail!("legacy Ponytail plugin entry remains after rollback for {host_id}");
        }
        Ok(())
    }

    pub fn restore_ponytail_receipt_if_unchanged(
        &self,
        previous_receipt: Option<&Value>,
        after_receipt: Option<&Value>,
    ) -> Result<()> {
        let receipt_path = self.runtime.tools_dir.join("ponytail.json");
        let serialize = |receipt: &Value| {
            serde_json::to_vec_pretty(receipt).context("serializing managed Ponytail receipt")
        };
        match (previous_receipt, after_receipt) {
            (Some(previous), Some(after)) => {
                crate::managed_files::atomic_write_bytes_if_unchanged(
                    &receipt_path,
                    &serialize(after)?,
                    &serialize(previous)?,
                )?;
            }
            (None, Some(after)) => {
                crate::managed_files::atomic_remove_file_if_unchanged(
                    &receipt_path,
                    &serialize(after)?,
                )?;
            }
            (Some(previous), None) => {
                crate::managed_files::atomic_write_bytes_if_absent(
                    &receipt_path,
                    &serialize(previous)?,
                )?;
            }
            (None, None) => {}
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

    fn plugin_fingerprint(self) -> Result<Option<String>> {
        match self {
            Self::ClaudeCode => claude_legacy_plugin_entry()
                .map(|entry| entry.map(|entry| json_fingerprint(&entry))),
            Self::Codex => codex_legacy_plugin_block().map(|block| block.map(text_fingerprint)),
        }
    }
}

fn claude_legacy_plugin_entry() -> Result<Option<Value>> {
    let path = dirs::home_dir()
        .context("home directory is unavailable")?
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let value: Value =
        serde_json::from_slice(&bytes).with_context(|| format!("decoding {}", path.display()))?;
    let root = value
        .as_object()
        .with_context(|| format!("{} must contain a JSON object", path.display()))?;
    let Some(plugins) = root.get("plugins") else {
        return Ok(None);
    };
    let plugins = plugins
        .as_object()
        .with_context(|| format!("{}.plugins must be a JSON object", path.display()))?;
    let Some(entry) = plugins.get(PONYTAIL_PLUGIN_REF).cloned() else {
        return Ok(None);
    };
    let entries = entry.as_array().with_context(|| {
        format!(
            "{}.plugins.{PONYTAIL_PLUGIN_REF} must be an array",
            path.display()
        )
    })?;
    Ok((!entries.is_empty()).then_some(entry))
}

fn codex_legacy_plugin_block() -> Result<Option<String>> {
    let path = dirs::home_dir()
        .context("home directory is unavailable")?
        .join(".codex")
        .join("config.toml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let header = format!("[plugins.\"{PONYTAIL_PLUGIN_REF}\"]");
    if text.lines().filter(|line| line.trim() == header).count() > 1 {
        bail!(
            "duplicate legacy Ponytail plugin blocks in {}",
            path.display()
        );
    }
    let mut offset = 0;
    let mut start = None;
    for segment in text.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        if start.is_some() && line.trim_start().starts_with('[') {
            return Ok(start.map(|start| text[start..offset].to_string()));
        }
        if line.trim_start() == header {
            start = Some(offset);
        }
        offset += segment.len();
    }
    Ok(start.map(|start| text[start..].to_string()))
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
