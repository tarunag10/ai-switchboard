use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::models::ToolStatus;
use crate::process_runner::run_command_streaming;

use super::ToolManager;

const PONYTAIL_MARKETPLACE: &str = "DietrichGebert/ponytail";
const PONYTAIL_MARKETPLACE_NAME: &str = "ponytail";
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
    /// single host. Used for both first install and re-enable.
    fn install_ponytail_into(&self, host: PluginHost) -> Result<()> {
        let cli = host.cli().context("CLI not found on PATH")?;
        // Re-adding an already-known marketplace is a benign error, so ignore it.
        let _ = self.run_ponytail_cmd(&cli, host, host.marketplace_add_args());
        self.run_ponytail_cmd(&cli, host, host.install_args())?;
        if !host.plugin_present() {
            bail!("install completed but the plugin was not registered");
        }
        Ok(())
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
        for host in hosts {
            match self.install_ponytail_into(host) {
                Ok(()) => installed_any = true,
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
            log::warn!(
                "ponytail installed for some hosts but not all: {}",
                errors.join("; ")
            );
        }
        let version =
            installed_ponytail_version().unwrap_or_else(|| PONYTAIL_DISPLAY_VERSION.into());
        self.write_tool_receipt("ponytail", json!({ "version": version, "enabled": true }))?;
        Ok(())
    }

    pub fn set_ponytail_enabled(&self, enabled: bool) -> Result<()> {
        // Guard on the receipt, not host presence: disabling on a host without a
        // disable verb (Codex) removes the plugin, so `ponytail_installed()`
        // would be false and re-enabling could never get past this check.
        if !self.ponytail_receipt_exists() {
            bail!("ponytail is not installed");
        }
        let mut errors: Vec<String> = Vec::new();
        let mut changed_any = false;
        for host in PluginHost::ALL {
            let Some(cli) = host.cli() else { continue };
            // Codex has no enable/disable verb, so enabling re-installs and
            // disabling removes. Skip disabling a host that isn't present.
            let result = if enabled {
                self.install_ponytail_into(host)
            } else if host.plugin_present() {
                self.run_ponytail_cmd(&cli, host, host.disable_args())
            } else {
                continue;
            };
            match result {
                Ok(()) => changed_any = true,
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
            json!({ "version": version, "enabled": enabled }),
        )?;
        Ok(())
    }

    pub fn uninstall_ponytail(&self) -> Result<()> {
        // No receipt means Headroom never installed it. Don't touch the user's
        // plugin config or marketplace registration (which they may own).
        if !self.ponytail_receipt_exists() {
            return Ok(());
        }
        for host in PluginHost::ALL {
            if let Some(cli) = host.cli() {
                let _ = self.run_ponytail_cmd(&cli, host, host.uninstall_args());
                let _ = self.run_ponytail_cmd(&cli, host, host.marketplace_remove_args());
            }
        }
        let receipt = self.runtime.tools_dir.join("ponytail.json");
        if receipt.exists() {
            std::fs::remove_file(&receipt)
                .with_context(|| format!("removing {}", receipt.display()))?;
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

    fn cli(self) -> Option<PathBuf> {
        match self {
            PluginHost::ClaudeCode => crate::claude_cli::detect_claude_cli(),
            PluginHost::Codex => crate::claude_cli::detect_codex_cli(),
        }
    }

    fn marketplace_add_args(self) -> &'static [&'static str] {
        &["plugin", "marketplace", "add", PONYTAIL_MARKETPLACE]
    }

    fn marketplace_remove_args(self) -> &'static [&'static str] {
        &["plugin", "marketplace", "remove", PONYTAIL_MARKETPLACE_NAME]
    }

    fn install_args(self) -> &'static [&'static str] {
        match self {
            PluginHost::ClaudeCode => {
                &["plugin", "install", PONYTAIL_PLUGIN_REF, "--scope", "user"]
            }
            PluginHost::Codex => &["plugin", "add", PONYTAIL_PLUGIN_REF],
        }
    }

    fn disable_args(self) -> &'static [&'static str] {
        match self {
            PluginHost::ClaudeCode => &["plugin", "disable", PONYTAIL_PLUGIN_REF],
            PluginHost::Codex => &["plugin", "remove", PONYTAIL_PLUGIN_REF],
        }
    }

    fn uninstall_args(self) -> &'static [&'static str] {
        match self {
            PluginHost::ClaudeCode => &["plugin", "uninstall", PONYTAIL_PLUGIN_REF],
            PluginHost::Codex => &["plugin", "remove", PONYTAIL_PLUGIN_REF],
        }
    }

    fn plugin_present(self) -> bool {
        match self {
            PluginHost::ClaudeCode => claude_ponytail_present(),
            PluginHost::Codex => codex_ponytail_present(),
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
fn claude_ponytail_present() -> bool {
    ponytail_installed_plugins()
        .and_then(|v| v.get("plugins")?.get(PONYTAIL_PLUGIN_REF).cloned())
        .and_then(|entry| entry.as_array().map(|installs| !installs.is_empty()))
        .unwrap_or(false)
}

/// Codex records installs in `~/.codex/config.toml` under a
/// `[plugins."ponytail@ponytail"]` table. Keys containing `@` are always
/// quoted, so a header substring match is reliable and avoids a TOML parse
/// dependency (matching how client_adapters edits this file).
fn codex_ponytail_present() -> bool {
    let Some(path) = dirs::home_dir().map(|h| h.join(".codex").join("config.toml")) else {
        return false;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let header = format!("[plugins.\"{PONYTAIL_PLUGIN_REF}\"]");
    text.lines().any(|line| line.trim_start() == header)
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
