use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Local;
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::{json, Value};
use tar::Archive;

use crate::models::{RtkCommandFamilyStats, RtkDailyStats, RtkTodayStats};
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

#[derive(Debug, Clone)]
pub(crate) struct RtkHistoryRow {
    pub(crate) timestamp: String,
    pub(crate) original_cmd: String,
    pub(crate) rtk_cmd: String,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) saved_tokens: u64,
    pub(crate) exec_time_ms: u64,
}

#[derive(Debug, Default)]
struct RtkCommandFamilyAccumulator {
    commands: u64,
    input_tokens: u64,
    output_tokens: u64,
    saved_tokens: u64,
    total_time_ms: u64,
    last_observed_at: Option<String>,
}

fn saturating_sqlite_u64(value: i64) -> u64 {
    value.max(0) as u64
}

/// Return a non-sensitive command family from an RTK command row. RTK stores
/// full shell commands for its own local history; Switchboard deliberately
/// retains only the first executable token and strips path components.
pub(crate) fn command_family(original_cmd: &str, rtk_cmd: &str) -> Option<String> {
    let candidate = original_cmd
        .split_whitespace()
        .find(|token| !token.contains('=') && *token != "env" && *token != "command")
        .or_else(|| {
            rtk_cmd
                .split_whitespace()
                .skip_while(|token| token.starts_with("rtk"))
                .find(|token| !token.contains('=') && *token != "env" && *token != "command")
        })?;

    let token = candidate
        .trim_matches(|character: char| character == '\'' || character == '"' || character == '`')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default();
    if token.is_empty() || token.len() > 64 {
        return None;
    }
    let lowercase = token.to_ascii_lowercase();
    if lowercase.starts_with("sk-")
        || lowercase.starts_with("sk_")
        || lowercase.starts_with("xai-")
        || lowercase.starts_with("ghp_")
        || lowercase.starts_with("github_pat_")
        || lowercase.starts_with("bearer")
        || (token.len() >= 32
            && token
                .chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .count()
                >= 28)
    {
        return None;
    }
    if !token.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | ':' | '@')
    }) {
        return None;
    }
    Some(token.to_string())
}

pub(crate) fn aggregate_rtk_command_families(
    rows: impl IntoIterator<Item = RtkHistoryRow>,
) -> Vec<RtkCommandFamilyStats> {
    let mut by_family = BTreeMap::<String, RtkCommandFamilyAccumulator>::new();
    for row in rows {
        let Some(family) = command_family(&row.original_cmd, &row.rtk_cmd) else {
            continue;
        };
        let entry = by_family.entry(family).or_default();
        entry.commands = entry.commands.saturating_add(1);
        entry.input_tokens = entry.input_tokens.saturating_add(row.input_tokens);
        entry.output_tokens = entry.output_tokens.saturating_add(row.output_tokens);
        entry.saved_tokens = entry.saved_tokens.saturating_add(row.saved_tokens);
        entry.total_time_ms = entry.total_time_ms.saturating_add(row.exec_time_ms);
        if entry
            .last_observed_at
            .as_ref()
            .is_none_or(|timestamp| timestamp < &row.timestamp)
        {
            entry.last_observed_at = Some(row.timestamp);
        }
    }

    by_family
        .into_iter()
        .map(|(family, aggregate)| {
            let savings_pct = if aggregate.input_tokens > 0 {
                Some(aggregate.saved_tokens as f64 / aggregate.input_tokens as f64 * 100.0)
            } else {
                None
            };
            let avg_time_ms =
                (aggregate.commands > 0).then(|| aggregate.total_time_ms / aggregate.commands);
            RtkCommandFamilyStats {
                family,
                commands: aggregate.commands,
                input_tokens: aggregate.input_tokens,
                output_tokens: aggregate.output_tokens,
                saved_tokens: aggregate.saved_tokens,
                savings_pct,
                total_time_ms: aggregate.total_time_ms,
                avg_time_ms,
                last_observed_at: aggregate.last_observed_at,
            }
        })
        .collect()
}

fn rtk_history_db_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(path) = std::env::var("RTK_DATABASE_PATH") {
        if !path.trim().is_empty() {
            paths.push(PathBuf::from(path));
        }
    }
    if let Some(home) = dirs::home_dir() {
        #[cfg(target_os = "macos")]
        paths.push(home.join("Library/Application Support/rtk/history.db"));
        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            if !data_home.trim().is_empty() {
                paths.push(PathBuf::from(data_home).join("rtk/history.db"));
            }
        }
        paths.push(home.join(".local/share/rtk/history.db"));
    }
    paths
}

pub(crate) fn read_rtk_command_families_from_db(path: &Path) -> Option<Vec<RtkCommandFamilyStats>> {
    use rusqlite::{Connection, OpenFlags};

    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    let mut statement = connection
        .prepare(
            "SELECT timestamp, original_cmd, rtk_cmd, input_tokens, output_tokens, saved_tokens, exec_time_ms FROM commands ORDER BY id",
        )
        .ok()?;
    let rows = statement
        .query_map([], |row| {
            Ok(RtkHistoryRow {
                timestamp: row.get::<_, String>(0)?,
                original_cmd: row.get::<_, String>(1)?,
                rtk_cmd: row.get::<_, String>(2)?,
                input_tokens: saturating_sqlite_u64(row.get::<_, i64>(3)?),
                output_tokens: saturating_sqlite_u64(row.get::<_, i64>(4)?),
                saved_tokens: saturating_sqlite_u64(row.get::<_, i64>(5)?),
                exec_time_ms: saturating_sqlite_u64(row.get::<_, i64>(6)?),
            })
        })
        .ok()?
        .filter_map(Result::ok);
    Some(aggregate_rtk_command_families(rows))
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

    /// Read RTK's local history database without writing to it. Only
    /// first-token command families and aggregate token/timing metrics leave
    /// this method; command arguments and project paths never enter app state.
    pub fn rtk_command_families(&self) -> Vec<RtkCommandFamilyStats> {
        if !self.rtk_installed() {
            return Vec::new();
        }
        rtk_history_db_candidates()
            .into_iter()
            .find_map(|path| read_rtk_command_families_from_db(&path))
            .unwrap_or_default()
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
