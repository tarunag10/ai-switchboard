use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Local;
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::{json, Value};
use tar::Archive;

use crate::models::{RtkDailyStats, RtkTodayStats};
use crate::runtime_distribution::{download_to_path, rtk_distribution_artifact, RTK_VERSION};
use crate::tool_manager::ToolManager;

#[derive(Debug, Clone, Deserialize)]
struct RtkDailyGainOutput {
    #[serde(default)]
    summary: Option<RtkGainSummary>,
    #[serde(default)]
    daily: Vec<RtkDailyEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RtkGainSummary {
    pub total_commands: u64,
    #[serde(default)]
    pub total_input: u64,
    #[serde(default)]
    pub total_output: u64,
    pub total_saved: u64,
    pub avg_savings_pct: f64,
    #[serde(default)]
    pub total_time_ms: u64,
    pub avg_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct RtkDailyEntry {
    date: String,
    #[serde(default)]
    commands: u64,
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    saved_tokens: u64,
    savings_pct: Option<f64>,
    #[serde(default)]
    total_time_ms: u64,
    avg_time_ms: Option<u64>,
}

impl ToolManager {
    pub fn rtk_entrypoint(&self) -> std::path::PathBuf {
        self.runtime.bin_dir.join("rtk")
    }

    pub fn read_rtk_activity(&self, max_lines: usize) -> Result<Vec<String>> {
        if !self.rtk_installed() {
            return Ok(vec!["RTK is not installed yet.".into()]);
        }

        let output = Command::new(self.rtk_entrypoint())
            .arg("session")
            .current_dir(&self.runtime.root_dir)
            .output()
            .with_context(|| format!("starting {} session", self.rtk_entrypoint().display()))?;

        if !output.status.success() {
            return Err(anyhow!(
                "command failed: {} session\nstdout:\n{}\nstderr:\n{}",
                self.rtk_entrypoint().display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines: Vec<String> = stdout.lines().map(|line| line.to_string()).collect();
        if lines.len() > max_lines {
            lines = lines.split_off(lines.len() - max_lines);
        }
        Ok(lines)
    }

    fn read_rtk_receipt(&self) -> Option<Value> {
        let path = self.runtime.tools_dir.join("rtk.json");
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn rtk_installed(&self) -> bool {
        self.rtk_entrypoint().exists() && self.runtime.tools_dir.join("rtk.json").exists()
    }

    pub fn installed_rtk_version(&self) -> Option<String> {
        self.read_rtk_receipt()?
            .get("version")?
            .as_str()
            .map(|v| v.to_string())
    }

    pub fn rtk_needs_install(&self) -> bool {
        !self.rtk_entrypoint().exists()
            || self.installed_rtk_version().as_deref() != Some(RTK_VERSION)
    }

    /// Refresh an *already installed* rtk to the pinned version. Never creates a
    /// fresh install: RTK is opt-in, so a missing binary means the user has not
    /// installed it (or uninstalled it) and launch must leave it absent.
    /// Returns Ok(true) if work was done, Ok(false) if already current or absent.
    pub fn ensure_rtk_current(&self) -> Result<bool> {
        if !self.rtk_entrypoint().exists() {
            return Ok(false);
        }
        if !self.rtk_needs_install() {
            return Ok(false);
        }
        self.install_rtk()?;
        Ok(true)
    }

    fn rtk_gain_output(&self) -> Option<RtkDailyGainOutput> {
        if !self.rtk_installed() {
            return None;
        }
        let output = Command::new(self.rtk_entrypoint())
            .args(["gain", "--daily", "--format", "json"])
            .current_dir(&self.runtime.root_dir)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        serde_json::from_slice(&output.stdout).ok()
    }

    pub fn rtk_gain_summary(&self) -> Option<RtkGainSummary> {
        self.rtk_gain_output()?.summary
    }

    pub fn rtk_today_stats(&self) -> Option<RtkTodayStats> {
        let today = Local::now().date_naive().to_string();
        self.rtk_gain_output()?
            .daily
            .into_iter()
            .find(|entry| entry.date == today)
            .map(|entry| RtkTodayStats {
                date: entry.date,
                saved_tokens: entry.saved_tokens,
                commands: entry.commands,
                input_tokens: entry.input_tokens,
                output_tokens: entry.output_tokens,
                savings_pct: entry.savings_pct,
                total_time_ms: entry.total_time_ms,
                avg_time_ms: entry.avg_time_ms,
            })
    }

    pub fn rtk_daily_stats(&self) -> Option<Vec<RtkDailyStats>> {
        Some(
            self.rtk_gain_output()?
                .daily
                .into_iter()
                .map(|entry| RtkDailyStats {
                    date: entry.date,
                    saved_tokens: entry.saved_tokens,
                    commands: entry.commands,
                    input_tokens: entry.input_tokens,
                    output_tokens: entry.output_tokens,
                    savings_pct: entry.savings_pct,
                    total_time_ms: entry.total_time_ms,
                    avg_time_ms: entry.avg_time_ms,
                })
                .collect(),
        )
    }

    pub fn install_rtk(&self) -> Result<()> {
        let artifact = rtk_distribution_artifact()?;
        let archive_path = self.runtime.downloads_dir.join(format!(
            "rtk-v{}-{}-{}.tar.gz",
            RTK_VERSION,
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        download_to_path(&artifact.url, &archive_path, artifact.sha256)?;

        let extract_dir = self.runtime.downloads_dir.join("rtk-extract");
        if extract_dir.exists() {
            std::fs::remove_dir_all(&extract_dir)
                .with_context(|| format!("removing {}", extract_dir.display()))?;
        }
        std::fs::create_dir_all(&extract_dir)
            .with_context(|| format!("creating {}", extract_dir.display()))?;

        let file = std::fs::File::open(&archive_path)
            .with_context(|| format!("opening {}", archive_path.display()))?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        archive
            .unpack(&extract_dir)
            .with_context(|| format!("extracting into {}", extract_dir.display()))?;

        let extracted_binary = extract_dir.join("rtk");
        if !extracted_binary.exists() {
            bail!(
                "rtk extraction completed but {} was not found",
                extracted_binary.display()
            );
        }

        let destination = self.rtk_entrypoint();
        std::fs::copy(&extracted_binary, &destination)
            .with_context(|| format!("writing {}", destination.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = std::fs::metadata(&destination)
                .with_context(|| format!("reading {}", destination.display()))?
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&destination, permissions)
                .with_context(|| format!("chmod {}", destination.display()))?;
        }

        self.write_tool_receipt(
            "rtk",
            json!({
                "status": "healthy",
                "installedBy": "Headroom",
                "scope": "self-contained",
                "runtime": "binary",
                "entrypoint": destination,
                "source": "https://github.com/rtk-ai/rtk",
                "version": RTK_VERSION,
                "artifact": {
                    "url": artifact.url,
                    "sha256": artifact.sha256
                }
            }),
        )
    }

    /// Remove the managed rtk binary and its receipt. Shell PATH and Claude Code
    /// hook teardown is handled separately by `client_adapters::set_rtk_enabled`.
    pub fn uninstall_rtk(&self) -> Result<()> {
        let binary = self.rtk_entrypoint();
        if binary.exists() {
            std::fs::remove_file(&binary)
                .with_context(|| format!("removing {}", binary.display()))?;
        }
        let receipt = self.runtime.tools_dir.join("rtk.json");
        if receipt.exists() {
            std::fs::remove_file(&receipt)
                .with_context(|| format!("removing {}", receipt.display()))?;
        }
        Ok(())
    }
}
