use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::models::ToolStatus;
use crate::process_runner::run_command_streaming;

use super::ToolManager;

const PONYTAIL_MARKETPLACE: &str = "DietrichGebert/ponytail";
const PONYTAIL_PLUGIN_REF: &str = "ponytail@ponytail";
pub(super) const PONYTAIL_DISPLAY_VERSION: &str = "latest";

impl ToolManager {
    /// Ponytail is a Claude Code plugin, not a binary we own, so "smoke test"
    /// means confirming it is still registered in Claude Code's plugin registry.
    /// No-op when our receipt says it was never installed.
    pub fn smoke_test_ponytail(&self) -> Result<()> {
        if !self.runtime.tools_dir.join("ponytail.json").exists() {
            return Ok(());
        }
        if !PluginHost::ALL.iter().any(|host| host.plugin_present()) {
            bail!("ponytail receipt exists but the plugin is no longer registered with any host");
        }
        Ok(())
    }

    /// A ponytail install is genuine only when our receipt exists AND at least
    /// one host (Claude Code or Codex) still has the plugin registered, so a
    /// user who removes it via `/plugin` doesn't leave the card stuck on
    /// "Enabled".
    #[cfg(test)]
    pub fn ponytail_installed(&self) -> bool {
        self.runtime.tools_dir.join("ponytail.json").exists()
            && PluginHost::ALL.iter().any(|host| host.plugin_present())
    }

    pub fn ponytail_registered_hosts(&self) -> Vec<String> {
        PluginHost::ALL
            .iter()
            .copied()
            .filter(|host| host.plugin_present())
            .map(|host| host.label().to_string())
            .collect()
    }

    /// Fingerprints of the exact Ponytail plugin entries currently configured
    /// by each supported host. The map intentionally contains no paths or
    /// marketplace metadata: those belong to the user and are never rollback
    /// targets.
    pub fn ponytail_host_fingerprints(&self) -> BTreeMap<String, String> {
        PluginHost::ALL
            .iter()
            .copied()
            .filter_map(|host| {
                host.plugin_fingerprint()
                    .map(|fingerprint| (host.id().to_string(), fingerprint))
            })
            .collect()
    }

    /// The managed Ponytail receipt is Switchboard-owned state. A selective
    /// rollback may restore it only when it still exactly matches the receipt
    /// written by that activation.
    pub fn ponytail_receipt_snapshot(&self) -> Option<Value> {
        self.read_tool_receipt("ponytail")
    }

    pub fn ponytail_receipt_exists(&self) -> bool {
        self.runtime.tools_dir.join("ponytail.json").exists()
    }

    fn run_ponytail_cmd(&self, cli: &Path, host: PluginHost, args: &[&str]) -> Result<()> {
        let label = host.label();
        run_command_streaming(cli, args, &self.runtime.root_dir, &mut |line: &str| {
            log::info!("ponytail [{label}]: {line}")
        })
    }

    /// Registers the marketplace (best-effort) and installs the plugin into a
    /// single host. The return value records whether this operation created a
    /// plugin entry that Switchboard may later remove.
    fn install_ponytail_into(&self, host: PluginHost) -> Result<bool> {
        let cli = host.cli().context("CLI not found on PATH")?;
        let already_present = host.plugin_present();
        // Re-adding an already-known marketplace is a benign error, so ignore it.
        let _ = self.run_ponytail_cmd(&cli, host, host.marketplace_add_args());
        self.run_ponytail_cmd(&cli, host, host.install_args())?;
        if !host.plugin_present() {
            bail!("install completed but the plugin was not registered");
        }
        Ok(!already_present)
    }

    fn uninstall_ponytail_plugin(&self, host: PluginHost) -> Result<()> {
        let cli = host.cli().context("CLI not found on PATH")?;
        self.run_ponytail_cmd(&cli, host, host.uninstall_args())
    }

    fn owned_hosts(&self) -> Vec<String> {
        self.read_tool_receipt("ponytail")
            .and_then(|receipt| receipt.get("ownedHosts").cloned())
            .and_then(|hosts| hosts.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|host| host.as_str().map(str::to_string))
            .collect()
    }

    pub fn install_ponytail(&self) -> Result<()> {
        let hosts: Vec<PluginHost> = PluginHost::ALL
            .into_iter()
            .filter(|host| host.cli().is_some())
            .collect();
        if hosts.is_empty() {
            bail!(
                "Neither the Claude Code CLI ('claude') nor the Codex CLI ('codex') was found on PATH. Install one, then try again."
            );
        }
        let mut errors: Vec<String> = Vec::new();
        let mut installed_any = false;
        let mut owned_hosts = Vec::new();
        for host in hosts {
            match self.install_ponytail_into(host) {
                Ok(owned) => {
                    installed_any = true;
                    if owned {
                        owned_hosts.push(host.label().to_string());
                    }
                }
                Err(err) => errors.push(format!("{}: {err:#}", host.label())),
            }
        }
        if !installed_any {
            bail!(
                "installing the ponytail plugin failed: {}",
                errors.join("; ")
            );
        }
        if !errors.is_empty() {
            let mut rollback_errors = Vec::new();
            for host in PluginHost::ALL
                .into_iter()
                .filter(|host| owned_hosts.iter().any(|owned| owned == host.label()))
            {
                if let Err(err) = self.uninstall_ponytail_plugin(host) {
                    rollback_errors.push(format!("{}: {err:#}", host.label()));
                }
            }
            if rollback_errors.is_empty() {
                bail!(
                    "ponytail installation was rolled back after host failure: {}",
                    errors.join("; ")
                );
            }
            bail!(
                "ponytail installation failed and rollback was incomplete: {}; cleanup: {}",
                errors.join("; "),
                rollback_errors.join("; ")
            );
        }
        let version =
            installed_ponytail_version().unwrap_or_else(|| PONYTAIL_DISPLAY_VERSION.into());
        self.write_tool_receipt(
            "ponytail",
            json!({ "version": version, "enabled": true, "ownedHosts": owned_hosts }),
        )?;
        Ok(())
    }

    pub fn set_ponytail_enabled(&self, enabled: bool) -> Result<()> {
        // Guard on the receipt, not host presence: a disabled app-owned plugin
        // may be absent from hosts that do not expose a separate disable verb.
        if !self.ponytail_receipt_exists() {
            bail!("ponytail is not installed");
        }
        let mut errors: Vec<String> = Vec::new();
        let mut changed_any = false;
        let mut owned_hosts = self.owned_hosts();
        for host in PluginHost::ALL {
            // Enabling re-installs where needed; disabling removes only plugin
            // entries whose ownership is recorded in our receipt.
            let owns_plugin = owned_hosts.iter().any(|owned| owned == host.label());
            let result = if enabled {
                self.install_ponytail_into(host)
            } else if owns_plugin && host.plugin_present() {
                self.uninstall_ponytail_plugin(host).map(|()| false)
            } else {
                continue;
            };
            match result {
                Ok(owned) => {
                    changed_any = true;
                    if enabled && owned && !owned_hosts.iter().any(|value| value == host.label()) {
                        owned_hosts.push(host.label().to_string());
                    }
                }
                Err(err) => errors.push(format!("{}: {err:#}", host.label())),
            }
        }
        if !changed_any && !errors.is_empty() {
            bail!("toggling ponytail failed: {}", errors.join("; "));
        }
        let version =
            installed_ponytail_version().unwrap_or_else(|| PONYTAIL_DISPLAY_VERSION.into());
        self.write_tool_receipt(
            "ponytail",
            json!({ "version": version, "enabled": enabled, "ownedHosts": owned_hosts }),
        )?;
        Ok(())
    }

    pub fn uninstall_ponytail(&self) -> Result<()> {
        // No receipt means Headroom never installed it. Don't touch the user's
        // plugin config or marketplace registration (which they may own).
        if !self.ponytail_receipt_exists() {
            return Ok(());
        }
        let owned_hosts = self.owned_hosts();
        for host in PluginHost::ALL
            .into_iter()
            .filter(|host| owned_hosts.iter().any(|owned| owned == host.label()))
        {
            if let Err(err) = self.uninstall_ponytail_plugin(host) {
                log::warn!(
                    "ponytail plugin cleanup failed for {}: {err:#}",
                    host.label()
                );
            }
        }
        // Marketplace ownership is not observable through the supported CLI
        // contract. Never remove a marketplace registration that may predate
        // Switchboard; plugin ownership is the only cleanup boundary we can
        // prove from the receipt.
        let receipt = self.runtime.tools_dir.join("ponytail.json");
        if receipt.exists() {
            std::fs::remove_file(&receipt)
                .with_context(|| format!("removing {}", receipt.display()))?;
        }
        Ok(())
    }

    /// Removes a single plugin entry only when the host's current entry still
    /// has the exact fingerprint captured after activation. This deliberately
    /// leaves marketplace registrations untouched.
    pub fn remove_ponytail_host_if_unchanged(
        &self,
        host_id: &str,
        expected_fingerprint: &str,
    ) -> Result<()> {
        let host = PluginHost::from_id(host_id)
            .with_context(|| format!("unknown Ponytail host identifier: {host_id}"))?;
        let current = host
            .plugin_fingerprint()
            .context("Ponytail plugin entry is no longer present")?;
        if current != expected_fingerprint {
            bail!(
                "Ponytail plugin entry for {} changed after activation",
                host.label()
            );
        }
        self.uninstall_ponytail_plugin(host)?;
        if host.plugin_fingerprint().is_some() {
            bail!(
                "Ponytail plugin entry for {} remains after removal",
                host.label()
            );
        }
        Ok(())
    }

    /// Restores only the Switchboard-owned Ponytail receipt. Host plugin
    /// entries must be rolled back separately, first, with their own exact
    /// fingerprints. JSON equality intentionally ignores formatting changes
    /// while rejecting any content change.
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
        let Some(receipt) = self.read_tool_receipt("ponytail") else {
            return ToolStatus::NotInstalled;
        };
        // Intentionally disabled via the app: the plugin may be gone from
        // hosts that lack a disable verb (Codex), but the receipt means it's
        // still installed -- report Healthy so the card shows Enable, not Install.
        let enabled = receipt
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if !enabled {
            return ToolStatus::Healthy;
        }
        // Enabled per our receipt: require it still be registered with a host,
        // so a manual `/plugin` removal surfaces as not-installed.
        if PluginHost::ALL.iter().any(|host| host.plugin_present()) {
            ToolStatus::Healthy
        } else {
            ToolStatus::NotInstalled
        }
    }

    pub(super) fn installed_ponytail_version(&self) -> Option<String> {
        installed_ponytail_version()
    }
}

/// Ponytail ships a marketplace plugin that both Claude Code and Codex can
/// install through their own `<cli> plugin ...` managers. Their verbs differ
/// (Claude has enable/disable/install/uninstall; Codex only add/remove), so
/// each host carries its own argument vectors.
#[derive(Clone, Copy)]
enum PluginHost {
    ClaudeCode,
    Codex,
}

impl PluginHost {
    const ALL: [PluginHost; 2] = [PluginHost::ClaudeCode, PluginHost::Codex];

    fn label(self) -> &'static str {
        match self {
            PluginHost::ClaudeCode => "Claude Code",
            PluginHost::Codex => "Codex",
        }
    }

    fn id(self) -> &'static str {
        match self {
            PluginHost::ClaudeCode => "claude-code",
            PluginHost::Codex => "codex",
        }
    }

    fn from_id(value: &str) -> Option<Self> {
        match value {
            "claude-code" => Some(PluginHost::ClaudeCode),
            "codex" => Some(PluginHost::Codex),
            _ => None,
        }
    }

    fn cli(self) -> Option<PathBuf> {
        match self {
            PluginHost::ClaudeCode => crate::claude_cli::detect_claude_cli(),
            PluginHost::Codex => crate::claude_cli::detect_codex_cli(),
        }
    }

    fn marketplace_add_args(self) -> &'static [&'static str] {
        &["plugin", "marketplace", "add", PONYTAIL_MARKETPLACE]
    }

    fn install_args(self) -> &'static [&'static str] {
        match self {
            PluginHost::ClaudeCode => {
                &["plugin", "install", PONYTAIL_PLUGIN_REF, "--scope", "user"]
            }
            PluginHost::Codex => &["plugin", "add", PONYTAIL_PLUGIN_REF],
        }
    }

    fn uninstall_args(self) -> &'static [&'static str] {
        match self {
            PluginHost::ClaudeCode => &["plugin", "uninstall", PONYTAIL_PLUGIN_REF],
            PluginHost::Codex => &["plugin", "remove", PONYTAIL_PLUGIN_REF],
        }
    }

    fn plugin_present(self) -> bool {
        self.plugin_fingerprint().is_some()
    }

    fn plugin_fingerprint(self) -> Option<String> {
        match self {
            PluginHost::ClaudeCode => claude_ponytail_fingerprint(),
            PluginHost::Codex => codex_ponytail_fingerprint(),
        }
    }
}

fn ponytail_installed_plugins() -> Option<Value> {
    let path = dirs::home_dir()?
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Claude Code records installs in `~/.claude/plugins/installed_plugins.json`
/// under `plugins["ponytail@ponytail"]` as a non-empty array of install records.
fn claude_ponytail_fingerprint() -> Option<String> {
    claude_ponytail_entry(ponytail_installed_plugins()?).map(|entry| json_fingerprint(&entry))
}

/// Codex records installs in `~/.codex/config.toml` under a
/// `[plugins."ponytail@ponytail"]` table. Keys containing `@` are always
/// quoted, so a header substring match is reliable and avoids a TOML parse
/// dependency (matching how client_adapters edits this file).
fn codex_ponytail_fingerprint() -> Option<String> {
    let Some(path) = dirs::home_dir().map(|h| h.join(".codex").join("config.toml")) else {
        return None;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return None;
    };
    codex_ponytail_block(&text).map(text_fingerprint)
}

fn claude_ponytail_entry(plugins: Value) -> Option<Value> {
    let entry = plugins.get("plugins")?.get(PONYTAIL_PLUGIN_REF)?.clone();
    let has_installs = entry
        .as_array()
        .is_some_and(|installs| !installs.is_empty());
    has_installs.then_some(entry)
}

fn codex_ponytail_block(text: &str) -> Option<&str> {
    let header = format!("[plugins.\"{PONYTAIL_PLUGIN_REF}\"]");
    let mut offset = 0;
    let mut start = None;
    for segment in text.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        if start.is_some() && line.trim_start().starts_with('[') {
            return start.map(|start| &text[start..offset]);
        }
        if start.is_none() && line.trim_start() == header {
            start = Some(offset);
        }
        offset += segment.len();
    }
    start.map(|start| &text[start..])
}

fn json_fingerprint(value: &Value) -> String {
    let payload = serde_json::to_vec(value).expect("serializing JSON value for fingerprint");
    bytes_fingerprint(&payload)
}

fn text_fingerprint(value: &str) -> String {
    bytes_fingerprint(value.as_bytes())
}

fn bytes_fingerprint(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    format!("sha256:{:x}", hasher.finalize())
}

fn installed_ponytail_version() -> Option<String> {
    let plugins = ponytail_installed_plugins()?;
    let installs = plugins
        .get("plugins")?
        .get(PONYTAIL_PLUGIN_REF)?
        .as_array()?;
    installs
        .first()?
        .get("version")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{claude_ponytail_entry, codex_ponytail_block, text_fingerprint, PluginHost};
    use serde_json::json;

    #[test]
    fn ponytail_hosts_have_stable_receipt_identifiers() {
        assert_eq!(PluginHost::ClaudeCode.id(), "claude-code");
        assert_eq!(PluginHost::Codex.id(), "codex");
        assert!(PluginHost::from_id("unknown").is_none());
    }

    #[test]
    fn claude_fingerprint_ignores_an_empty_plugin_array() {
        assert!(claude_ponytail_entry(json!({
            "plugins": { "ponytail@ponytail": [] }
        }))
        .is_none());
    }

    #[test]
    fn codex_fingerprint_targets_only_the_ponytail_table() {
        let config = concat!(
            "[plugins.\"ponytail@ponytail\"]\n",
            "enabled = true\n",
            "[plugins.\"another@plugin\"]\n",
            "enabled = true\n"
        );
        assert_eq!(
            codex_ponytail_block(config),
            Some("[plugins.\"ponytail@ponytail\"]\nenabled = true\n")
        );
        assert_ne!(
            text_fingerprint(codex_ponytail_block(config).unwrap()),
            text_fingerprint(config)
        );
    }
}
