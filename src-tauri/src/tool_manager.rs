use std::fs::OpenOptions;
use std::net::TcpListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use parking_lot::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tar::Archive;

use crate::backend_port::{self, AllForeign, SelectedFallback};
use crate::headroom_learn::{
    claude_project_memory_file, read_headroom_learn_metadata_from_path, HeadroomLearnMetadata,
    HeadroomLearnProjectSummary,
};
use crate::models::{ManagedTool, RepoMemoryMcpServiceStatus, ToolStatus};
#[cfg(test)]
use crate::process_runner::path_with_binary_dir;
use crate::process_runner::{
    build_command, exit_status_signal, run_command, run_command_streaming,
    run_command_with_timeout, CommandFailure,
};
pub(crate) use crate::runtime_distribution::{
    available_disk_bytes, bootstrap_requirements_lock, download_to_path,
    download_to_path_with_progress, pip_line_to_progress, python_distribution_artifact,
    requirements_lock_sha, rtk_distribution_artifact, HeadroomRelease, InPlaceUpgradeContext,
    PipOutputCapture, RuntimeMaintenanceKind, UpgradeOutcome, HEADROOM_PINNED_SHA256,
    HEADROOM_PINNED_VERSION, HEADROOM_PINNED_WHEEL_URL, RTK_VERSION,
};
#[cfg(test)]
use crate::runtime_distribution::{
    bootstrap_requirements_lock_for_target, sha256_bytes, verify_sha256_file,
};

/// Pinned headroom-ai version. Upgrade logic is disabled; this exact version
/// will be installed if the currently-installed version differs.
///
/// 0.20.x–0.24.x upstream shipped a maturin/Rust-native wheel that was both
/// per-Python-version and per-platform (e.g. `cp312-cp312-macosx_11_0_arm64`,
/// upstream #355). Starting with 0.25.0 the native module is built against the
/// CPython stable ABI (abi3, upstream #516), so a single `cp310-abi3` wheel per
/// platform now covers every CPython >= 3.10 — the pin below
/// (`cp310-abi3-macosx_11_0_arm64`) installs cleanly on our bundled cp312 and
/// stays valid if `PYTHON_STANDALONE_RELEASE` later moves to 3.13+. Only the
/// per-platform axis still matters: if Linux is ever added to the release matrix
/// (release-macos.yml builds macOS arm64 only today), re-pick the matching
/// `*-manylinux_*` abi3 wheel from
/// https://pypi.org/pypi/headroom-ai/<version>/json and add a per-platform
/// wheel-picker (mirroring `python_distribution_artifact`).
const HEADROOM_SMOKE_TEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Upper bound on the one-time `learn --verbosity` baseline seed run before
/// proxy start. Typical runs are a few seconds (a ~100MB transcript project
/// seeds in ~3s); the cap only trips on pathological corpora, after which the
/// proxy starts anyway and seeding retries next launch.
const HEADROOM_BASELINE_SEED_TIMEOUT: Duration = Duration::from_secs(30);
/// Index of pre-built wheels for sdist-only PyPI packages (e.g. hnswlib).
/// GitHub's expanded_assets endpoint serves HTML anchors pip can consume via --find-links.
const VENDOR_WHEELS_INDEX_URL: &str =
    "https://github.com/gglucass/headroom-desktop/releases/expanded_assets/vendor-wheels-v1";
// headroom binds on the backend port chosen at spawn time (default 6768);
// the intercept layer on 6767 forwards to it. The backend port is dynamic
// because something else on the machine (e.g. rapportd) can claim 6768 at
// login — see `backend_port` for the selection logic.
fn headroom_proxy_port() -> String {
    backend_port::get().to_string()
}
const HEADROOM_PROXY_URL: &str = "http://127.0.0.1:6767";
const MCP_METHOD_CLAUDE_CLI: &str = "claude_cli";
const MCP_METHOD_FALLBACK_JSON: &str = "fallback_json";
const MCP_METHOD_DIRECT_CLAUDE_JSON: &str = "direct_claude_json";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum McpInstallMethod {
    ClaudeCli,
    FallbackJson,
    DirectClaudeJson,
}

impl McpInstallMethod {
    fn as_str(self) -> &'static str {
        match self {
            McpInstallMethod::ClaudeCli => MCP_METHOD_CLAUDE_CLI,
            McpInstallMethod::FallbackJson => MCP_METHOD_FALLBACK_JSON,
            McpInstallMethod::DirectClaudeJson => MCP_METHOD_DIRECT_CLAUDE_JSON,
        }
    }
}
const HEADROOM_STARTUP_POLL_MS: u64 = 250;
const HEADROOM_STARTUP_TIMEOUT_MS: u64 = 300_000;

/// Full-file SHA-256 values of historical headroom-requirements.lock shipments
/// whose pinned versions are byte-for-byte identical to the current lock.
/// Receipts holding one of these shas are treated as up-to-date; the receipt is
/// silently migrated to the comment-insensitive sha on next launch so the
/// entry can be dropped the next time the lock file actually changes.
///
/// When modifying the lock, re-evaluate: compare the stripped (comments and
/// blank lines removed) form of each legacy lock against the stripped current
/// lock. Drop any entry that no longer matches — those users need a real
/// reinstall.
const LEGACY_REQUIREMENTS_LOCK_SHAS: &[&str] = &[
    // The 0.25.0 and 0.26.0 bundles shared one lock byte-for-byte (identical
    // requires_dist). 0.27.0 actually moves pins (tree-sitter-language-pack
    // 1.8.1 -> 0.13.0 plus the spreadsheet extra: et-xmlfile/openpyxl/xlrd), so
    // the stripped lock sha changes. The 0.25.0/0.26.0 cohort's lock receipt is
    // now genuinely stale and SHOULD trigger a reinstall — do not whitelist the
    // old sha here. The list stays empty until a future no-op cosmetic lock edit
    // (comments/blank lines only, no pin moves) needs to be treated as
    // up-to-date.
];

pub const CAVEMAN_LEVEL_SCOPED: &str = "scoped";
pub const CAVEMAN_LEVEL_AGGRESSIVE: &str = "aggressive";
pub const CAVEMAN_LEVEL_COMPACT_CHINESE: &str = "compact_chinese";
const REPO_MEMORY_DISPLAY_VERSION: &str = "1";
const REPO_MEMORY_MCP_NAME: &str = "repo-memory";
const REPO_MEMORY_MCP_SMOKE_TEST_TIMEOUT: Duration = Duration::from_secs(10);

mod caveman;
mod headroom_receipt;
mod markitdown;
mod ponytail;
mod proxy_runtime;
mod repo_memory;
mod rtk;

#[cfg(test)]
use headroom_receipt::parse_major_minor_patch;
use headroom_receipt::{receipt_requires_atomic_rebuild, ATOMIC_REBUILD_FLOOR_VERSION};
#[allow(unused_imports)]
pub use proxy_runtime::running_proxy_argv;
pub use proxy_runtime::running_proxy_matches_expected_args;
use proxy_runtime::{
    apply_savings_mode_env, diagnose_proxy_port, extract_required_pydantic_core_version,
    format_all_foreign_bail, headroom_entrypoint_startup_args, headroom_python_startup_args,
    is_local_proxy_reachable, parse_pid_from_lsof_detail, reclaim_orphan_proxy,
    repair_console_script_interpreter, sanitize_log_variant, PortState,
};
#[cfg(test)]
use proxy_runtime::{
    format_already_running_bail, probe_backend_readyz_ok, proxy_argv_contains_expected_flags_for,
    redact_sensitive, wait_for_port_free,
};
pub(crate) use proxy_runtime::{newest_proxy_log_path, tail_log_file};
use repo_memory::{
    claude_code_has_headroom_mcp_server, claude_code_has_mcp_server, resolve_command_path,
    script_path as repo_memory_script_path, write_headroom_to_claude_json,
    write_mcp_server_to_claude_json,
};
pub use rtk::RtkGainSummary;

#[derive(Debug, Clone)]
pub struct BootstrapStepUpdate {
    pub step: &'static str,
    pub message: String,
    pub eta_seconds: u64,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedRuntime {
    pub root_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub python_dir: PathBuf,
    pub venv_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub downloads_dir: PathBuf,
}

impl ManagedRuntime {
    pub fn bootstrap_root(base_dir: &Path) -> Self {
        let root_dir = base_dir.join("headroom");
        let runtime_dir = root_dir.join("runtime");
        let bin_dir = root_dir.join("bin");
        let python_dir = runtime_dir.join("python");
        let venv_dir = runtime_dir.join("venv");
        let tools_dir = root_dir.join("tools");
        let downloads_dir = root_dir.join("downloads");

        Self {
            root_dir,
            runtime_dir,
            bin_dir,
            python_dir,
            venv_dir,
            tools_dir,
            downloads_dir,
        }
    }

    pub fn ensure_layout(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root_dir)
            .with_context(|| format!("creating {}", self.root_dir.display()))?;
        std::fs::create_dir_all(&self.runtime_dir)
            .with_context(|| format!("creating {}", self.runtime_dir.display()))?;
        std::fs::create_dir_all(&self.bin_dir)
            .with_context(|| format!("creating {}", self.bin_dir.display()))?;
        std::fs::create_dir_all(&self.tools_dir)
            .with_context(|| format!("creating {}", self.tools_dir.display()))?;
        std::fs::create_dir_all(&self.downloads_dir)
            .with_context(|| format!("creating {}", self.downloads_dir.display()))?;
        Ok(())
    }

    pub fn standalone_python(&self) -> PathBuf {
        self.python_dir.join("bin").join("python3")
    }

    pub fn managed_python(&self) -> PathBuf {
        self.venv_dir.join("bin").join("python3")
    }

    pub fn managed_pip(&self) -> PathBuf {
        self.venv_dir.join("bin").join("pip")
    }

    pub fn ready_flag(&self) -> PathBuf {
        self.venv_dir.join("READY")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root_dir.join("logs")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedToolManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub runtime: String,
    pub source_url: String,
    pub version: String,
    pub checksum: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct ToolManager {
    runtime: ManagedRuntime,
    manifests: Vec<ManagedToolManifest>,
    log_marker_cache: Arc<Mutex<Option<ToolLogMarkerCache>>>,
}

#[derive(Debug, Clone)]
struct ToolLogMarkerCache {
    tool_id: String,
    path: PathBuf,
    modified: std::time::SystemTime,
    result: Option<bool>,
}

/// Result of a best-effort kompress model prefetch.
pub enum KompressPrefetchOutcome {
    /// Model successfully downloaded and cached.
    Downloaded,
    /// Subprocess exited non-zero. `cause` is a coarse category plus the last
    /// meaningful line of `kompress-prefetch.log`, suitable for Sentry.
    Failed { cause: String },
}

/// Build a short, Sentry-friendly cause from the tail of the prefetch log.
/// The leading `[category]` keeps related failures grouped; the trailing line
/// carries the specific error for triage.
fn summarize_kompress_prefetch_failure(log_path: &Path) -> String {
    let contents = std::fs::read_to_string(log_path).unwrap_or_default();
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(40);
    let tail = lines[start..].join("\n");

    let category = classify_kompress_prefetch_failure(&tail);
    let detail: String = tail
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .next_back()
        .unwrap_or("(no output in kompress-prefetch.log)")
        .chars()
        .take(200)
        .collect();

    format!("[{category}] {detail}")
}

/// Bucket a prefetch-log tail into a coarse, stable failure category.
fn classify_kompress_prefetch_failure(tail: &str) -> &'static str {
    let t = tail.to_lowercase();
    if t.is_empty() {
        "no output"
    } else if t.contains("sigabrt") || t.contains("aborted") {
        "native abort"
    } else if t.contains("no space left") || t.contains("disk full") || t.contains("errno 28") {
        "disk full"
    } else if t.contains("connection")
        || t.contains("timed out")
        || t.contains("timeout")
        || t.contains("name resolution")
        || t.contains("failed to resolve")
        || t.contains("max retries exceeded")
        || t.contains("ssl")
        || t.contains("httperror")
    {
        "network"
    } else if t.contains("permission denied") {
        "permission denied"
    } else {
        "other"
    }
}

impl ToolManager {
    pub fn new(runtime: ManagedRuntime) -> Self {
        let rtk_checksum = rtk_distribution_artifact()
            .ok()
            .and_then(|artifact| artifact.sha256.map(str::to_owned));
        let manifests = vec![
            ManagedToolManifest {
                id: "headroom".into(),
                name: "Headroom".into(),
                description: "Default optimizer stage for every supported client.".into(),
                runtime: "python".into(),
                source_url: "https://pypi.org/project/headroom-ai/".into(),
                version: HEADROOM_PINNED_VERSION.into(),
                checksum: None,
                required: true,
            },
            ManagedToolManifest {
                id: "rtk".into(),
                name: "RTK".into(),
                description:
                    "Token-optimized shell command proxy for your coding agent and your terminal.".into(),
                runtime: "binary".into(),
                source_url: "https://github.com/rtk-ai/rtk".into(),
                version: RTK_VERSION.into(),
                checksum: rtk_checksum,
                required: false,
            },
            ManagedToolManifest {
                id: "markitdown".into(),
                name: "MarkItDown".into(),
                description:
                    "Converts PDF and Office documents to Markdown so they cost far fewer tokens when your agent reads them."
                        .into(),
                runtime: "python".into(),
                source_url: "https://github.com/microsoft/markitdown".into(),
                version: markitdown::MARKITDOWN_PINNED_VERSION.into(),
                checksum: None,
                required: false,
            },
            ManagedToolManifest {
                id: "ponytail".into(),
                name: "Ponytail".into(),
                description:
                    "Plugin that nudges the agent to write the least code possible. Installs into Claude Code and Codex. Requires their CLI and Node.js on PATH."
                        .into(),
                runtime: "plugin".into(),
                source_url: "https://github.com/DietrichGebert/ponytail".into(),
                version: ponytail::PONYTAIL_DISPLAY_VERSION.into(),
                checksum: None,
                required: false,
            },
            ManagedToolManifest {
                id: "caveman".into(),
                name: "Caveman".into(),
                description:
                    "Managed guidance block that nudges the agent toward terse output. Writes a Switchboard-owned block into Claude Code and Codex instruction files."
                        .into(),
                runtime: "plugin".into(),
                source_url: "https://github.com/mac-ai-switchboard/caveman".into(),
                version: caveman::CAVEMAN_DISPLAY_VERSION.into(),
                checksum: None,
                required: false,
            },
        ];

        Self {
            runtime,
            manifests,
            log_marker_cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn list_tools(&self) -> Vec<ManagedTool> {
        self.manifests
            .iter()
            .map(|manifest| ManagedTool {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                description: manifest.description.clone(),
                runtime: manifest.runtime.clone(),
                required: manifest.required,
                enabled: self.tool_enabled(&manifest.id),
                status: self.detect_status(&manifest.id),
                source_url: manifest.source_url.clone(),
                version: if manifest.id == "headroom" {
                    self.installed_headroom_version()
                        .unwrap_or_else(|| manifest.version.clone())
                } else if manifest.id == "ponytail" {
                    self.installed_ponytail_version()
                        .unwrap_or_else(|| manifest.version.clone())
                } else {
                    manifest.version.clone()
                },
                checksum: manifest.checksum.clone(),
                metadata: if manifest.id == "caveman" {
                    Some(json!({ "level": self.caveman_level() }))
                } else {
                    None
                },
            })
            .collect()
    }

    pub fn python_runtime_installed(&self) -> bool {
        self.runtime.ready_flag().exists() && self.runtime.managed_python().exists()
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.runtime.logs_dir()
    }

    pub fn headroom_entrypoint(&self) -> PathBuf {
        self.runtime.venv_dir.join("bin").join("headroom")
    }

    pub fn managed_python(&self) -> PathBuf {
        self.runtime.managed_python()
    }

    /// Seed the output-shaper savings baseline by mining the user's Claude Code
    /// transcripts once. The proxy's `/stats` `output_shaping` estimate stays
    /// `available: false` until this baseline exists, so without it the
    /// dashboard would never show an output-reduction number. Heuristic-only
    /// (no `--llm-judge`), so it needs no API key or network, and writes the
    /// baseline into `~/.headroom/output_savings.json` (the same `workspace_dir`
    /// the proxy's recorder reads).
    ///
    /// Targets a single transcript-rich project rather than `--all`: upstream's
    /// `_run_verbosity` writes the ledger *inside* its per-project loop
    /// (last-project-wins), so `--apply --all` overwrites the baseline with
    /// whatever project sorts last — often a near-empty one. We instead pick the
    /// project with the most transcript bytes and pass its real path via
    /// `--project`. Baseline strata (model / turn kind / size / tools) are
    /// project-independent, so one busy project yields a usable baseline. (A
    /// proper cross-project aggregate belongs upstream; tracked separately.)
    ///
    /// Best-effort and idempotent: skips when a baseline is already present.
    ///
    /// MUST run *before* the proxy starts. The proxy's `SavingsRecorder` loads
    /// the baseline once at boot and, on its periodic flush, writes its
    /// in-memory ledger back to disk — so a baseline written after the proxy is
    /// running is both invisible (never reloaded) and eventually clobbered by an
    /// empty-baseline flush. Seeding first means the recorder boots with the
    /// real baseline and the number shows without an app relaunch. Synchronous
    /// but bounded by `HEADROOM_BASELINE_SEED_TIMEOUT`; callers already run it on
    /// a background thread, so the one-time ~3s scan never blocks the UI.
    ///
    /// The learned verbosity level it also writes is intentionally ignored: the
    /// proxy spawn pins `HEADROOM_VERBOSITY_LEVEL=2`, the manual-override tier.
    pub fn seed_verbosity_baseline_if_needed(&self) {
        if verbosity_baseline_present() {
            log::debug!("verbosity baseline seeding skipped: baseline already present");
            return;
        }
        let entrypoint = self.headroom_entrypoint();
        if !entrypoint.exists() {
            log::debug!(
                "verbosity baseline seeding skipped: entrypoint not yet installed at {}",
                entrypoint.display()
            );
            return;
        }
        log::info!("seeding output-shaper verbosity baseline (no baseline present yet)");
        let Some(project_cwd) = busiest_claude_project_cwd() else {
            log::info!("verbosity baseline seeding skipped: no Claude transcripts found");
            return;
        };
        let args = [
            "learn",
            "--verbosity",
            "--apply",
            "--project",
            project_cwd.as_str(),
        ];
        // Bounded so a pathological transcript corpus can never hang launch:
        // typical runs are a few seconds; the cap only trips on outliers, after
        // which we proceed and retry next launch.
        match run_command_with_timeout(
            &entrypoint,
            &args,
            &self.runtime.root_dir,
            HEADROOM_BASELINE_SEED_TIMEOUT,
        ) {
            Ok(()) => log::info!("seeded output-shaper verbosity baseline from {project_cwd}"),
            Err(err) => log::info!("verbosity baseline seeding failed: {err:#}"),
        }
    }

    pub fn headroom_learn_log_path(&self, project_path: &str) -> PathBuf {
        let logs_dir = self.runtime.logs_dir();
        let project_name = Path::new(project_path)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("project");
        let safe_name: String = project_name
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        let mut hasher = Sha256::new();
        hasher.update(project_path.as_bytes());
        let digest = format!("{:x}", hasher.finalize());
        let short_hash = &digest[..12];
        logs_dir.join(format!("headroom-learn-{safe_name}-{short_hash}.log"))
    }

    pub fn headroom_learn_last_run_at(&self, project_path: &str) -> Option<String> {
        let path = self.headroom_learn_log_path(project_path);
        if let Ok(modified) = std::fs::metadata(path).and_then(|meta| meta.modified()) {
            let timestamp: DateTime<Utc> = modified.into();
            return Some(timestamp.to_rfc3339());
        }

        self.headroom_learn_metadata(project_path)
            .and_then(|metadata| metadata.learned_at)
    }

    /// Bundled metadata used to populate a `ClaudeCodeProject` row. Reads
    /// CLAUDE.md + MEMORY.md once instead of three times, which collapses
    /// 6 file reads per project down to 2 during the project list scan.
    pub fn headroom_learn_project_summary(
        &self,
        project_path: &str,
    ) -> HeadroomLearnProjectSummary {
        let metadata = self.headroom_learn_metadata(project_path);
        let log_last_run_at = std::fs::metadata(self.headroom_learn_log_path(project_path))
            .and_then(|meta| meta.modified())
            .ok()
            .map(|m| {
                let t: DateTime<Utc> = m.into();
                t.to_rfc3339()
            });
        HeadroomLearnProjectSummary {
            last_run_at: log_last_run_at
                .or_else(|| metadata.as_ref().and_then(|m| m.learned_at.clone())),
            has_persisted_learnings: metadata.is_some(),
            pattern_count: metadata.and_then(|m| m.pattern_count),
        }
    }

    /// `reclaim_healthy_orphan`: forwarded to `reclaim_orphan_proxy` so an
    /// upgrade boot validation replaces even a still-healthy old proxy squatting
    /// on 6768. Pass `false` for normal launch (leave a live backend alone).
    pub fn start_headroom_background(&self, reclaim_healthy_orphan: bool) -> Result<Child> {
        let mut allow_repair = true;
        'attempt: loop {
            let python = self.managed_python();
            if !python.exists() {
                bail!("headroom managed python not found at {}", python.display());
            }

            let entrypoint = self.headroom_entrypoint();

            let mut failures: Vec<HeadroomStartupFailure> = Vec::new();
            let logs_dir = self.runtime.logs_dir();
            std::fs::create_dir_all(&logs_dir)
                .with_context(|| format!("creating {}", logs_dir.display()))?;

            // Pre-flight: 6768 may already be held. Three cases:
            //   * Free → spawn on 6768.
            //   * HeadroomRunning → an orphaned proxy from a prior session is
            //     squatting on the port (a healthy one would have satisfied
            //     `is_headroom_proxy_reachable` upstream, so `ensure_headroom_running`
            //     would never have reached the spawn path). Reclaim the port by
            //     terminating it, then spawn the fresh runtime. Only bail if it
            //     turns out to be genuinely health-serving or we can't free it.
            //   * ForeignOccupant → try to fall back to a port in
            //     6769..=6790. Only bail if every fallback is also taken.
            // The chosen port is stored in `backend_port` so the intercept,
            // health probes, and spawn args all pick it up.
            let initial_state = diagnose_proxy_port(backend_port::DEFAULT_BACKEND_PORT);
            match initial_state {
                PortState::Free => {
                    backend_port::set(backend_port::DEFAULT_BACKEND_PORT);
                }
                PortState::HeadroomRunning => {
                    reclaim_orphan_proxy(
                        backend_port::DEFAULT_BACKEND_PORT,
                        reclaim_healthy_orphan,
                    )?;
                    backend_port::set(backend_port::DEFAULT_BACKEND_PORT);
                }
                PortState::ForeignOccupant(detail) => {
                    let pid = parse_pid_from_lsof_detail(&detail);
                    let try_bind = |port: u16| TcpListener::bind(("127.0.0.1", port)).is_ok();
                    match backend_port::select_fallback(detail.clone(), pid, try_bind) {
                        Ok(SelectedFallback {
                            port,
                            original_occupant,
                            original_pid,
                        }) => {
                            backend_port::set(port);
                            log::warn!(
                                "[backend_port] {} held by {}; falling back to {}",
                                backend_port::DEFAULT_BACKEND_PORT,
                                original_occupant,
                                port,
                            );
                            sentry::with_scope(
                                |scope| {
                                    scope.set_tag("flow", "backend_port_fallback");
                                    scope.set_tag(
                                        "occupant_cmd",
                                        original_occupant
                                            .split(" pid ")
                                            .next()
                                            .unwrap_or("unknown"),
                                    );
                                    scope.set_extra(
                                        "original_port",
                                        backend_port::DEFAULT_BACKEND_PORT.into(),
                                    );
                                    scope.set_extra("chosen_port", port.into());
                                    if let Some(p) = original_pid {
                                        scope.set_extra("occupant_pid", p.into());
                                    }
                                },
                                || {
                                    sentry::capture_message(
                                        &format!(
                                            "backend_port_fallback: {} held by {}, using {}",
                                            backend_port::DEFAULT_BACKEND_PORT,
                                            original_occupant,
                                            port,
                                        ),
                                        sentry::Level::Info,
                                    );
                                },
                            );
                        }
                        Err(AllForeign {
                            original_occupant,
                            fallback_range,
                            ..
                        }) => {
                            bail!(
                                "{}",
                                format_all_foreign_bail(
                                    backend_port::DEFAULT_BACKEND_PORT,
                                    &original_occupant,
                                    fallback_range,
                                )
                            );
                        }
                    }
                }
            }

            // Construct spawn variants AFTER pre-flight so `--port` reflects any
            // fallback chosen above. The arg helpers read `backend_port::get()`
            // eagerly; building them earlier bakes in the stale default and the
            // proxy ends up trying to bind the foreign-held port.
            // Use the console_scripts entrypoint when available to avoid the Python
            // -m double-import RuntimeWarning. Fall back to -m if missing.
            let savings_mode = crate::client_adapters::load_savings_mode();
            repair_console_script_interpreter(&entrypoint, &python)?;
            let startup_variants: Vec<(PathBuf, Vec<String>)> = if entrypoint.exists() {
                vec![
                    (entrypoint, headroom_entrypoint_startup_args(&savings_mode)),
                    (python.clone(), headroom_python_startup_args()),
                ]
            } else {
                vec![(python.clone(), headroom_python_startup_args())]
            };

            for (executable, args) in &startup_variants {
                let variant = if args.is_empty() {
                    "default".to_string()
                } else {
                    sanitize_log_variant(&args.join("-"))
                };
                let log_path = logs_dir.join(format!("headroom-{variant}.log"));
                let log_file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .with_context(|| format!("opening {}", log_path.display()))?;

                // Wrap with `nice` so headroom yields CPU to foreground apps
                // (Claude Code, terminal, etc.) when the machine is contended.
                // On idle systems headroom still runs at full speed.
                let mut command = Command::new("/usr/bin/nice");
                command
                    .arg("-n")
                    .arg("5")
                    .arg(executable)
                    .args(args)
                    .current_dir(&self.runtime.root_dir)
                    .process_group(0)
                    .env("PYTHONNOUSERSITE", "1")
                    .env("PYTHONUNBUFFERED", "1")
                    .env("PYTHONFAULTHANDLER", "1")
                    .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
                    .env("PIP_NO_INPUT", "1")
                    // Force huggingface_hub off the native `hf_xet` downloader.
                    // Its Rust extension can SIGABRT ("Fatal Python error: Aborted")
                    // inside xet_get while pulling kompress-int8.onnx during
                    // eager_load_compressors, killing the interpreter before it
                    // binds the port (Sentry: never opened port within timeout).
                    // The SIGABRT is uncatchable in Python; disabling xet falls
                    // back to the stable HTTPS download path.
                    .env("HF_HUB_DISABLE_XET", "1")
                    .env("HEADROOM_SDK", "headroom-desktop-proxy")
                    .env("HEADROOM_HTTP2", "false")
                    // Disable the HTTP/1.1 keep-alive pool for the upstream
                    // (proxy -> api.anthropic.com) client. Claude Code cancels
                    // streaming requests constantly (ESC, aborted tool calls,
                    // subagent cancellations), which can leave a pooled TLS
                    // connection desynced; reusing it surfaces as
                    // "SSLV3_ALERT_BAD_RECORD_MAC" on the next request. The
                    // proxy's retry path does not catch SSL/RemoteProtocolError,
                    // so the raw error leaks back to the client. Fresh
                    // connection per request avoids reuse of a poisoned socket.
                    .env("HEADROOM_MAX_KEEPALIVE", "0")
                    // Optimization mode. Always token: maximize raw-token savings via
                    // prior-turn compression. (Cache mode and the auth-based auto-switch
                    // were removed; cache mode contributed no measurable savings over
                    // Claude Code's native prefix caching.)
                    .env("HEADROOM_MODE", "token")
                    // Enable plain user-message text compression in addition to
                    // tool results. Primary motive is Codex/OpenAI: with a 0.5
                    // read-discount and 0.0 write-penalty, compressing user text
                    // clears the force-compress threshold and yields real savings.
                    // This is the only desktop-side lever (the `headroom proxy`
                    // entrypoint reads this env only; it exposes no CLI flag), and
                    // it is process-global on the single shared proxy. Anthropic
                    // blast radius is small: its 0.9 read-discount means already-
                    // cached content almost never busts the frozen prefix, and
                    // protect_recent/min_tokens guard the just-typed prompt.
                    .env("HEADROOM_COMPRESS_USER_MESSAGES", "1")
                    // Output-token shaping (new in headroom-ai 0.27.0). The proxy
                    // never emits output tokens, so this works request-side: it
                    // appends a byte-stable verbosity instruction to the TAIL of
                    // the system prompt (after the cache_control breakpoint, so the
                    // provider prefix cache is preserved) and lowers an
                    // already-present output_config.effort on mechanically-classified
                    // turns. Off by default upstream; enabled here. Effort router
                    // and mechanical-effort use upstream defaults (on, "low"). The
                    // shaper only ever lowers an effort the client already sent and
                    // never toggles thinking.type, so it cannot 400 a model that
                    // lacks effort support.
                    .env("HEADROOM_OUTPUT_SHAPER", "1")
                    // Pin the steering level explicitly. An explicit env is the
                    // manual-override tier in the shaper's level resolution, so it
                    // wins over the per-user learned level written to verbosity.json
                    // by the baseline-seeding `learn --verbosity` run. That keeps
                    // steering uniform/predictable across users while the seeded
                    // baseline still feeds the /stats savings estimate. Level 2 =
                    // skip pre/postamble, don't restate in-context code/tool output.
                    .env("HEADROOM_VERBOSITY_LEVEL", "2");
                apply_savings_mode_env(&mut command, &savings_mode);
                let mut child = command
                    .stdin(Stdio::null())
                    .stdout(Stdio::from(
                        log_file
                            .try_clone()
                            .with_context(|| format!("cloning {}", log_path.display()))?,
                    ))
                    .stderr(Stdio::from(log_file))
                    .spawn()
                    .with_context(|| {
                        format!(
                            "starting headroom background process: {} {}",
                            executable.display(),
                            args.join(" ")
                        )
                    })?;

                let mut startup_ok = false;
                let mut reason: Option<String> = None;

                let startup_polls = (HEADROOM_STARTUP_TIMEOUT_MS / HEADROOM_STARTUP_POLL_MS).max(1);
                for _ in 0..startup_polls {
                    thread::sleep(Duration::from_millis(HEADROOM_STARTUP_POLL_MS));
                    if is_local_proxy_reachable() {
                        startup_ok = true;
                        break;
                    }

                    match child.try_wait() {
                        Ok(Some(status)) => {
                            reason = Some(format!(
                                "exited with status {} before opening port {}",
                                status,
                                headroom_proxy_port()
                            ));
                            break;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            reason = Some(format!("wait check failed: {}", err));
                            break;
                        }
                    }
                }

                if startup_ok {
                    return Ok(child);
                }

                // Timeout path (process still alive, port never opened): send SIGABRT
                // so PYTHONFAULTHANDLER=1 dumps all-thread tracebacks to the log file
                // before the process dies. Skip if the process already exited on its own.
                if reason.is_none() {
                    let _ = Command::new("/bin/kill")
                        .arg("-ABRT")
                        .arg(child.id().to_string())
                        .status();
                    thread::sleep(Duration::from_millis(500));
                }

                let _ = child.kill();
                let _ = child.wait();

                let reason = reason.unwrap_or_else(|| {
                    format!(
                        "never opened port {} within {}ms",
                        headroom_proxy_port(),
                        HEADROOM_STARTUP_TIMEOUT_MS
                    )
                });
                failures.push(HeadroomStartupFailure {
                    program: executable.display().to_string(),
                    args: args.iter().map(|s| s.to_string()).collect(),
                    log_path: log_path.display().to_string(),
                    log_tail: tail_log_file(&log_path, 80),
                    reason,
                });
            }

            // All variants failed. If the proxy crashed because the venv has a
            // pydantic / pydantic-core skew (e.g. a partial upgrade left
            // pydantic-core ahead of pydantic), pin pydantic-core back to the
            // version pydantic asks for and retry once. The error message itself
            // tells us the required version — see extract_required_pydantic_core_version.
            if allow_repair {
                if let Some(target) = failures
                    .iter()
                    .find_map(|f| extract_required_pydantic_core_version(&f.log_tail))
                {
                    log::warn!(
                        "headroom proxy failed with pydantic-core/pydantic skew; \
                     reinstalling pydantic-core=={target} and retrying"
                    );
                    match self.repair_pydantic_core(&target) {
                        Ok(()) => {
                            log::warn!("pydantic-core repair succeeded; retrying headroom startup");
                            allow_repair = false;
                            continue 'attempt;
                        }
                        Err(repair_err) => {
                            log::error!("pydantic-core repair failed: {repair_err:#}");
                        }
                    }
                }
            }

            let last = failures
                .pop()
                .expect("at least one startup variant attempted");
            let prior_summary = if failures.is_empty() {
                String::new()
            } else {
                let joined = failures
                    .iter()
                    .map(|f| format!("{} {} {}", f.program, f.args.join(" "), f.reason))
                    .collect::<Vec<_>>()
                    .join("; ");
                format!(" (prior attempts: {})", joined)
            };
            return Err(anyhow::Error::from(last).context(format!(
                "unable to keep headroom running in background{}",
                prior_summary
            )));
        }
    }

    pub fn latest_tool_log_path(&self, tool_id: &str) -> Option<PathBuf> {
        let logs_dir = self.runtime.logs_dir();
        let entries = std::fs::read_dir(&logs_dir).ok()?;
        let prefix = format!("{tool_id}-");
        let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with(&prefix) && name.ends_with(".log"))
                    .unwrap_or(false)
            })
            .filter_map(|path| {
                let modified = std::fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .ok()?;
                Some((modified, path))
            })
            .collect();

        candidates.sort_by_key(|(modified, _)| *modified);
        candidates.last().map(|(_, path)| path.clone())
    }

    pub fn read_headroom_log_tail(&self, max_lines: usize) -> Result<Vec<String>> {
        self.read_tool_log_tail("headroom", max_lines)
    }

    pub fn read_tool_log_tail(&self, tool_id: &str, max_lines: usize) -> Result<Vec<String>> {
        let Some(path) = self.latest_tool_log_path(tool_id) else {
            return Ok(Vec::new());
        };

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let lines = content
            .lines()
            .rev()
            .take(max_lines)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|line| line.to_string())
            .collect();
        Ok(lines)
    }

    fn latest_tool_log_marker_state(
        &self,
        tool_id: &str,
        enabled_markers: &[&str],
        disabled_markers: &[&str],
    ) -> Option<bool> {
        let path = self.latest_tool_log_path(tool_id)?;
        self.scan_file_for_marker_state_cached(tool_id, &path, enabled_markers, disabled_markers)
    }

    fn scan_file_for_marker_state_cached(
        &self,
        cache_key: &str,
        path: &Path,
        enabled_markers: &[&str],
        disabled_markers: &[&str],
    ) -> Option<bool> {
        let modified = std::fs::metadata(path).ok()?.modified().ok()?;

        {
            let cache = self.log_marker_cache.lock();
            if let Some(cached) = cache.as_ref() {
                if cached.tool_id == cache_key && cached.path == path && cached.modified == modified
                {
                    return cached.result;
                }
            }
        }

        let content = std::fs::read_to_string(path).ok()?;

        let mut result: Option<bool> = None;
        for line in content.lines().rev() {
            let lowered = line.to_ascii_lowercase();
            if enabled_markers
                .iter()
                .any(|marker| lowered.contains(marker))
            {
                result = Some(true);
                break;
            }
            if disabled_markers
                .iter()
                .any(|marker| lowered.contains(marker))
            {
                result = Some(false);
                break;
            }
        }

        let mut cache = self.log_marker_cache.lock();
        *cache = Some(ToolLogMarkerCache {
            tool_id: cache_key.to_string(),
            path: path.to_path_buf(),
            modified,
            result,
        });

        result
    }

    pub fn headroom_mcp_configured(&self) -> Option<bool> {
        self.read_headroom_receipt()?
            .get("mcp")?
            .get("configured")?
            .as_bool()
    }

    pub fn headroom_mcp_error(&self) -> Option<String> {
        self.read_headroom_receipt()?
            .get("mcp")?
            .get("error")?
            .as_str()
            .map(|value| value.to_string())
    }

    pub fn headroom_mcp_install_method(&self) -> Option<String> {
        self.read_headroom_receipt()?
            .get("mcp")?
            .get("installMethod")?
            .as_str()
            .map(|value| value.to_string())
    }

    pub fn headroom_ml_installed(&self) -> Option<bool> {
        self.read_headroom_receipt()?
            .get("ml")?
            .get("installed")?
            .as_bool()
    }

    pub fn headroom_kompress_enabled(&self) -> Option<bool> {
        // The `headroom` Python package attaches a RotatingFileHandler to its
        // `headroom` root logger with `propagate = False` (see helpers.py:
        // `_setup_file_logging`). Proxy-logger INFO lines — including the
        // `Kompress: ENABLED/not installed/disabled` startup markers — go to
        // `~/.headroom/logs/proxy.log` only, never to the stderr stream that
        // our Tauri-spawned log captures. Probe that file first; fall back to
        // the spawn-time tool log (covers older headroom versions that do
        // propagate to stderr).
        // Positive markers: the startup `Kompress: ENABLED` line (cache hit at
        // eager-preload) AND the lazy-load success lines emitted on first use
        // when the model was downloaded after a cold-cache startup. The scan
        // returns the most recent marker, so a lazy load flips the status to
        // enabled without waiting for a backend restart.
        const KOMPRESS_ENABLED_MARKERS: &[&str] = &[
            "kompress: enabled",
            "kompress onnx loaded",
            "kompress pytorch loaded",
        ];
        const KOMPRESS_DISABLED_MARKERS: &[&str] =
            &["kompress: not installed", "kompress: disabled"];
        if let Some(path) = headroom_propagated_proxy_log_path() {
            if let Some(state) = self.scan_file_for_marker_state_cached(
                "headroom-proxy-log",
                &path,
                KOMPRESS_ENABLED_MARKERS,
                KOMPRESS_DISABLED_MARKERS,
            ) {
                return Some(state);
            }
        }

        self.latest_tool_log_marker_state(
            "headroom",
            KOMPRESS_ENABLED_MARKERS,
            KOMPRESS_DISABLED_MARKERS,
        )
    }

    /// True if the Kompress model snapshot is already present in the
    /// HuggingFace hub cache (`$HOME/.cache/huggingface/hub/
    /// models--chopratejas--kompress-v2-base/snapshots/<rev>`). Used as the
    /// prefetch idempotency guard so we never re-download an existing model.
    pub fn kompress_model_cached(&self) -> bool {
        let Some(home) = dirs::home_dir() else {
            return false;
        };
        let snapshots = home
            .join(".cache")
            .join("huggingface")
            .join("hub")
            .join("models--chopratejas--kompress-v2-base")
            .join("snapshots");
        std::fs::read_dir(&snapshots)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
    }

    /// Download the Kompress model (~260MB) into the HF cache by running the
    /// bundled venv python's loader with network enabled. Blocks until the
    /// download finishes — call this on a background thread. Output is captured
    /// to `logs/kompress-prefetch.log`.
    ///
    /// This front-loads the download the proxy would otherwise do lazily on
    /// first request, so a fresh install has ML compression ready before any
    /// traffic. It is best-effort: on failure the proxy's own lazy-load path
    /// still downloads on first use. On a non-zero exit the returned
    /// [`KompressPrefetchOutcome::Failed`] carries a short, Sentry-friendly
    /// cause read from the tail of the prefetch log.
    pub fn prefetch_kompress_model(&self) -> Result<KompressPrefetchOutcome> {
        let python = self.managed_python();
        if !python.exists() {
            bail!("headroom managed python not found at {}", python.display());
        }
        let logs_dir = self.runtime.logs_dir();
        std::fs::create_dir_all(&logs_dir)
            .with_context(|| format!("creating {}", logs_dir.display()))?;
        let log_path = logs_dir.join("kompress-prefetch.log");
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("opening {}", log_path.display()))?;

        let status = Command::new(&python)
            .arg("-c")
            .arg(
                "from headroom.transforms.kompress_compressor import KompressCompressor; \
                 KompressCompressor().preload(allow_download=True)",
            )
            .current_dir(&self.runtime.root_dir)
            .env("PYTHONNOUSERSITE", "1")
            .env("PYTHONUNBUFFERED", "1")
            // Same xet guard as the proxy spawn: the native hf_xet downloader
            // can SIGABRT mid-pull; the HTTPS fallback is stable.
            .env("HF_HUB_DISABLE_XET", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::from(
                log_file
                    .try_clone()
                    .with_context(|| format!("cloning {}", log_path.display()))?,
            ))
            .stderr(Stdio::from(log_file))
            .status()
            .with_context(|| format!("running kompress prefetch via {}", python.display()))?;

        if status.success() {
            Ok(KompressPrefetchOutcome::Downloaded)
        } else {
            Ok(KompressPrefetchOutcome::Failed {
                cause: summarize_kompress_prefetch_failure(&log_path),
            })
        }
    }

    fn read_headroom_receipt(&self) -> Option<Value> {
        let path = self.runtime.tools_dir.join("headroom.json");
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    fn headroom_learn_metadata(&self, project_path: &str) -> Option<HeadroomLearnMetadata> {
        let mut candidates = self
            .headroom_learn_memory_paths(project_path)
            .into_iter()
            .filter_map(|path| read_headroom_learn_metadata_from_path(&path))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| right.sort_key.cmp(&left.sort_key));
        candidates
            .into_iter()
            .next()
            .map(|candidate| candidate.metadata)
    }

    fn headroom_learn_memory_paths(&self, project_path: &str) -> Vec<PathBuf> {
        vec![
            Path::new(project_path).join("CLAUDE.md"),
            claude_project_memory_file(project_path),
        ]
    }

    /// Returns the installed Headroom version from the tool receipt, if any.
    pub fn installed_headroom_version(&self) -> Option<String> {
        self.read_headroom_receipt()?
            .get("version")?
            .as_str()
            .map(|v| v.to_string())
    }

    fn installed_requirements_lock_sha(&self) -> Option<String> {
        self.read_headroom_receipt()?
            .get("artifact")?
            .get("requirementsLockSha256")?
            .as_str()
            .map(|v| v.to_string())
    }

    /// Returns the pinned release if the installed version differs from the pin.
    pub fn check_headroom_upgrade(&self) -> Option<HeadroomRelease> {
        let installed = self.installed_headroom_version()?;
        if installed == HEADROOM_PINNED_VERSION {
            return None;
        }
        Some(HeadroomRelease {
            version: HEADROOM_PINNED_VERSION.into(),
            wheel_url: HEADROOM_PINNED_WHEEL_URL.into(),
            sha256: HEADROOM_PINNED_SHA256.into(),
        })
    }

    /// Returns true if the compiled requirements lock differs from what was
    /// used during the last headroom install.
    ///
    /// As a side effect, if the stored sha is a known legacy value whose
    /// pinned versions are byte-identical to the current lock, rewrites the
    /// receipt with the new-format sha and returns false. This avoids a
    /// purely cosmetic reinstall on the 0.2.50 → 0.3.0 jump.
    pub fn requirements_are_stale(&self) -> bool {
        let Some(stored) = self.installed_requirements_lock_sha() else {
            return true;
        };
        let current = requirements_lock_sha(bootstrap_requirements_lock());
        if stored == current {
            return false;
        }
        if LEGACY_REQUIREMENTS_LOCK_SHAS.contains(&stored.as_str()) {
            if let Err(err) = self.write_requirements_lock_sha_to_receipt(&current) {
                log::warn!("failed to migrate legacy requirementsLockSha256: {err}");
            }
            return false;
        }
        true
    }

    fn write_requirements_lock_sha_to_receipt(&self, sha: &str) -> Result<()> {
        let receipt_path = self.runtime.tools_dir.join("headroom.json");
        let bytes = std::fs::read(&receipt_path)
            .with_context(|| format!("reading {}", receipt_path.display()))?;
        let mut receipt: Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", receipt_path.display()))?;
        if let Some(artifact) = receipt.get_mut("artifact").and_then(|a| a.as_object_mut()) {
            artifact.insert("requirementsLockSha256".into(), json!(sha));
        } else {
            return Ok(());
        }
        std::fs::write(&receipt_path, serde_json::to_vec(&receipt)?)
            .with_context(|| format!("writing {}", receipt_path.display()))?;
        Ok(())
    }

    pub fn repair_stale_requirements_with_progress<F>(&self, mut progress: F) -> Result<()>
    where
        F: FnMut(BootstrapStepUpdate),
    {
        let requirements_lock = bootstrap_requirements_lock();
        let lock_path = self.write_headroom_requirements_lock(requirements_lock)?;

        progress(BootstrapStepUpdate {
            step: "Repairing dependencies",
            message: "Repairing Headroom's bundled dependencies.".into(),
            eta_seconds: 60,
            percent: 40,
        });

        let deps_start = Instant::now();
        let progress_ref = std::cell::RefCell::new(&mut progress);
        let mut dep_counter: u32 = 0;
        run_pip_install_with_retries_streaming(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--find-links",
                VENDOR_WHEELS_INDEX_URL,
                "--extra-index-url",
                "https://pypi.org/simple",
                "--upgrade",
                "--requirement",
                lock_path.to_string_lossy().as_ref(),
            ],
            &self.runtime.root_dir,
            |line| {
                if let Some(update) =
                    pip_line_to_progress(line, deps_start.elapsed(), &mut dep_counter, 40, 82)
                {
                    if let Ok(mut cb) = progress_ref.try_borrow_mut() {
                        (cb)(BootstrapStepUpdate {
                            step: "Repairing dependencies",
                            message: update.message,
                            eta_seconds: update.eta_seconds,
                            percent: update.percent,
                        });
                    }
                }
            },
        )
        .context("repairing stale headroom requirements")?;

        progress(BootstrapStepUpdate {
            step: "Configuring integrations",
            message: "Setting up Headroom MCP integration.".into(),
            eta_seconds: 5,
            percent: 88,
        });

        let mcp_install = match self.install_headroom_mcp() {
            Ok(method) => json!({
                "configured": true,
                "proxyUrl": HEADROOM_PROXY_URL,
                "installMethod": method.as_str(),
            }),
            Err(err) => {
                log::info!("headroom MCP setup skipped during repair: {err:#}");
                json!({ "configured": false, "proxyUrl": HEADROOM_PROXY_URL, "error": err.to_string() })
            }
        };

        self.update_headroom_receipt_after_requirements_repair(
            requirements_lock_sha(requirements_lock),
            mcp_install,
        )?;

        progress(BootstrapStepUpdate {
            step: "Repair complete",
            message: "Headroom dependency repair finished.".into(),
            eta_seconds: 0,
            percent: 95,
        });

        Ok(())
    }

    pub fn bootstrap_all(&self) -> Result<ManagedRuntime> {
        self.bootstrap_all_with_progress(|_| {})
    }

    pub fn bootstrap_all_with_progress<F>(&self, mut progress: F) -> Result<ManagedRuntime>
    where
        F: FnMut(BootstrapStepUpdate),
    {
        progress(BootstrapStepUpdate {
            step: "Preparing install",
            message: "Setting up managed directories.".into(),
            eta_seconds: 3,
            percent: 5,
        });
        self.runtime.ensure_layout()?;

        if !self.runtime.standalone_python().exists() {
            progress(BootstrapStepUpdate {
                step: "Downloading Python",
                message: "Fetching pinned standalone Python runtime.".into(),
                eta_seconds: 75,
                percent: 18,
            });
            self.install_python_distribution(|update| progress(update))?;
        } else {
            progress(BootstrapStepUpdate {
                step: "Python runtime",
                message: "Pinned Python runtime already available locally.".into(),
                eta_seconds: 3,
                percent: 18,
            });
        }

        if !self.runtime.managed_python().exists() {
            progress(BootstrapStepUpdate {
                step: "Creating environment",
                message: "Creating isolated Headroom virtual environment.".into(),
                eta_seconds: 25,
                percent: 35,
            });
            self.create_managed_venv()?;
        } else {
            progress(BootstrapStepUpdate {
                step: "Environment",
                message: "Isolated runtime already present.".into(),
                eta_seconds: 3,
                percent: 35,
            });
        }

        progress(BootstrapStepUpdate {
            step: "Installing Headroom",
            message: "Installing Headroom and required dependencies.".into(),
            eta_seconds: 95,
            percent: 58,
        });
        self.install_headroom()?;

        // RTK is opt-in: bootstrap no longer installs it. Users add it from the
        // Addons tab, which calls install_addon("rtk").
        progress(BootstrapStepUpdate {
            step: "Finalizing",
            message: "Writing managed runtime receipts and completion markers.".into(),
            eta_seconds: 6,
            percent: 90,
        });
        self.write_ready_flag()?;
        self.write_bootstrap_receipt()?;
        progress(BootstrapStepUpdate {
            step: "Install complete",
            message: "Headroom runtime installed successfully.".into(),
            eta_seconds: 0,
            percent: 100,
        });
        Ok(self.runtime.clone())
    }

    fn install_python_distribution<F>(&self, mut emit_step: F) -> Result<()>
    where
        F: FnMut(BootstrapStepUpdate),
    {
        let archive_path = self.runtime.downloads_dir.join("python-standalone.tar.gz");
        let artifact = python_distribution_artifact()?;
        // Sub-progress maps the download to bootstrap percents 18..=34 (next
        // step starts at 35). Keeps the progress bar moving on slow networks
        // so users don't assume the app has frozen.
        let started_at = Instant::now();
        download_to_path_with_progress(
            &artifact.url,
            &archive_path,
            artifact.sha256,
            |downloaded, total| {
                let downloaded_mb = downloaded as f64 / 1_048_576.0;
                let (message, percent, eta_seconds) = match total {
                    Some(total) if total > 0 => {
                        let total_mb = total as f64 / 1_048_576.0;
                        let frac = (downloaded as f64 / total as f64).clamp(0.0, 1.0);
                        let percent = (18.0 + frac * 16.0).round().clamp(18.0, 34.0) as u8;
                        let elapsed = started_at.elapsed().as_secs_f64().max(0.1);
                        let rate = downloaded as f64 / elapsed;
                        let remaining = (total.saturating_sub(downloaded)) as f64;
                        let eta = if rate > 1.0 {
                            (remaining / rate).ceil() as u64
                        } else {
                            75
                        };
                        (
                            format!(
                                "Downloading Python runtime: {:.1} / {:.1} MB",
                                downloaded_mb, total_mb
                            ),
                            percent,
                            eta,
                        )
                    }
                    _ => (
                        format!("Downloading Python runtime: {:.1} MB", downloaded_mb),
                        18,
                        75,
                    ),
                };
                emit_step(BootstrapStepUpdate {
                    step: "Downloading Python",
                    message,
                    eta_seconds,
                    percent,
                });
            },
        )?;

        let file = std::fs::File::open(&archive_path)
            .with_context(|| format!("opening {}", archive_path.display()))?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        archive
            .unpack(&self.runtime.runtime_dir)
            .with_context(|| format!("extracting into {}", self.runtime.runtime_dir.display()))?;

        if !self.runtime.standalone_python().exists() {
            bail!(
                "standalone python extraction completed but {} was not found",
                self.runtime.standalone_python().display()
            );
        }

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            let python = self.runtime.standalone_python();
            if let Ok(metadata) = std::fs::metadata(&python) {
                let mut perms = metadata.permissions();
                if perms.mode() & 0o111 == 0 {
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&python, perms);
                }
            }
        }

        // Strip the quarantine attribute from the extracted runtime so macOS
        // Gatekeeper doesn't scan it on first execution (which can hang the
        // machine for 20-30 seconds).
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("xattr")
                .args([
                    "-rd",
                    "com.apple.quarantine",
                    self.runtime.runtime_dir.to_string_lossy().as_ref(),
                ])
                .output();
        }

        Ok(())
    }

    fn create_managed_venv(&self) -> Result<()> {
        run_python_command(
            &self.runtime.standalone_python(),
            &[
                "-m",
                "venv",
                self.runtime.venv_dir.to_string_lossy().as_ref(),
            ],
            &self.runtime.root_dir,
        )
        .context("creating Headroom-managed virtualenv")?;

        run_python_command(
            &self.runtime.managed_python(),
            &["-m", "pip", "--version"],
            &self.runtime.root_dir,
        )
        .context("verifying Headroom-managed pip is available")?;

        Ok(())
    }

    /// Bootstrap path: installs the pinned headroom release.
    fn install_headroom(&self) -> Result<()> {
        // Bootstrap path runs at first launch where there is no boot
        // validation yet — no caller will read the captured pip output, so
        // skip the buffer to avoid allocating it.
        self.install_headroom_release(
            &HeadroomRelease {
                version: HEADROOM_PINNED_VERSION.into(),
                wheel_url: HEADROOM_PINNED_WHEEL_URL.into(),
                sha256: HEADROOM_PINNED_SHA256.into(),
            },
            |_| {},
            None,
        )
    }

    fn install_headroom_release<F>(
        &self,
        release: &HeadroomRelease,
        mut progress: F,
        pip_capture: Option<&std::cell::RefCell<PipOutputCapture>>,
    ) -> Result<()>
    where
        F: FnMut(BootstrapStepUpdate),
    {
        let requirements_lock = bootstrap_requirements_lock();
        let lock_path = self.write_headroom_requirements_lock(requirements_lock)?;
        let wheel_path = self
            .runtime
            .downloads_dir
            .join(format!("headroom_ai-{}-py3-none-any.whl", release.version));

        progress(BootstrapStepUpdate {
            step: "Downloading update",
            message: "Fetching Headroom engine update bundle.".into(),
            eta_seconds: 15,
            percent: 40,
        });

        // Try direct wheel download (with retries). If it fails, fall back to PyPI index.
        let use_wheel =
            match download_to_path(&release.wheel_url, &wheel_path, Some(&release.sha256)) {
                Ok(()) => true,
                Err(download_err) => {
                    log::warn!(
                    "headroom wheel download failed (will fall back to pip index): {download_err}"
                );
                    false
                }
            };

        progress(BootstrapStepUpdate {
            step: "Updating dependencies",
            message: "Updating Headroom's bundled dependencies.".into(),
            eta_seconds: 90,
            percent: 55,
        });

        // Stream pip's stdout/stderr and translate noteworthy lines into
        // user-facing step updates so the progress UI actually changes
        // during the ~60-90s dependency install instead of staring at a
        // single "Updating dependencies" frame. Also funnel each line into
        // the diagnostic capture so a later boot-validation failure can
        // forensic the pip run that produced the broken venv.
        let deps_start = std::time::Instant::now();
        let deps_progress_ref = std::cell::RefCell::new(&mut progress);
        let mut dep_counter: u32 = 0;
        run_pip_install_with_retries_streaming(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--find-links",
                VENDOR_WHEELS_INDEX_URL,
                "--extra-index-url",
                "https://pypi.org/simple",
                "--upgrade",
                "--requirement",
                lock_path.to_string_lossy().as_ref(),
            ],
            &self.runtime.root_dir,
            |line| {
                if let Some(cap) = pip_capture {
                    cap.borrow_mut().push(line);
                }
                if let Some(update) =
                    pip_line_to_progress(line, deps_start.elapsed(), &mut dep_counter, 55, 80)
                {
                    if let Ok(mut cb) = deps_progress_ref.try_borrow_mut() {
                        (cb)(update);
                    }
                }
            },
        )
        .context("installing locked Headroom dependencies into Headroom-managed virtualenv")?;

        progress(BootstrapStepUpdate {
            step: "Applying update",
            message: "Applying the Headroom engine update.".into(),
            eta_seconds: 15,
            percent: 80,
        });

        let headroom_spec = format!("headroom-ai=={}", release.version);
        let headroom_arg = if use_wheel {
            wheel_path.to_string_lossy().into_owned()
        } else {
            headroom_spec.clone()
        };
        run_pip_install_with_retries_streaming(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--extra-index-url",
                "https://pypi.org/simple",
                "--no-deps",
                &headroom_arg,
            ],
            &self.runtime.root_dir,
            |line| {
                if let Some(cap) = pip_capture {
                    cap.borrow_mut().push(line);
                }
            },
        )
        .with_context(|| {
            if use_wheel {
                "installing verified Headroom wheel into Headroom-managed virtualenv".into()
            } else {
                format!("installing {headroom_spec} from PyPI into Headroom-managed virtualenv")
            }
        })?;

        // Ad-hoc sign every native extension pip just dropped into the venv.
        // PyPI wheels are unsigned; some EDR tooling stalls or blocks on
        // first-execution of unsigned binaries. Best-effort — failures are
        // logged and ignored, the smoke test downstream is the real gate.
        self.ad_hoc_sign_venv_natives();

        progress(BootstrapStepUpdate {
            step: "Configuring integrations",
            message: "Setting up Headroom MCP integration.".into(),
            eta_seconds: 5,
            percent: 90,
        });

        let mcp_install = match self.install_headroom_mcp() {
            Ok(method) => json!({
                "configured": true,
                "proxyUrl": HEADROOM_PROXY_URL,
                "installMethod": method.as_str(),
            }),
            Err(err) => {
                log::info!("headroom MCP setup skipped: {err:#}");
                json!({
                    "configured": false,
                    "proxyUrl": HEADROOM_PROXY_URL,
                    "error": err.to_string()
                })
            }
        };

        self.write_tool_receipt(
            "headroom",
            json!({
                "status": "healthy",
                "installedBy": "Headroom",
                "scope": "self-contained",
                "runtime": "python",
                "pythonExecutable": self.runtime.managed_python(),
                "pipExecutable": self.runtime.managed_pip(),
                "entrypoint": self.runtime.venv_dir.join("bin").join("headroom"),
                "source": self.manifests[0].source_url,
                "version": release.version,
                "artifact": {
                    "url": release.wheel_url,
                    "sha256": release.sha256,
                    "requirementsLockSha256": requirements_lock_sha(requirements_lock)
                },
                "mcp": mcp_install,
                "ml": {
                    "installed": true,
                    "engine": "kompress"
                }
            }),
        )
    }

    fn update_headroom_receipt_after_requirements_repair(
        &self,
        requirements_lock_sha256: String,
        mcp_install: Value,
    ) -> Result<()> {
        let receipt_path = self.runtime.tools_dir.join("headroom.json");
        if let Ok(bytes) = std::fs::read(&receipt_path) {
            if let Ok(mut receipt) = serde_json::from_slice::<Value>(&bytes) {
                if let Some(artifact) = receipt.get_mut("artifact").and_then(|a| a.as_object_mut())
                {
                    artifact.insert(
                        "requirementsLockSha256".into(),
                        json!(requirements_lock_sha256),
                    );
                }
                receipt["mcp"] = mcp_install;
                std::fs::write(&receipt_path, serde_json::to_vec(&receipt)?)
                    .with_context(|| format!("writing {}", receipt_path.display()))?;
            }
        }
        Ok(())
    }

    /// Cheap post-install sanity check: can the new venv import the top-level
    /// headroom package and its proxy entrypoint? Catches import errors, syntax
    /// errors, and missing transitive dependencies introduced by a new version
    /// before we try to actually boot the proxy.
    ///
    /// If the failure is a pydantic / pydantic-core skew (pip's `--upgrade -r
    /// lock` left the two out of sync), reinstall pydantic-core at the version
    /// pydantic asks for and retry the smoke test once. Mirrors the
    /// proxy-startup repair in `start_headroom_proxy_with_repair` so the same
    /// recoverable failure doesn't fail an in-flight upgrade and force a
    /// rollback.
    pub fn smoke_test_headroom(&self) -> Result<()> {
        match self.smoke_test_headroom_with_timeout(HEADROOM_SMOKE_TEST_TIMEOUT) {
            Ok(()) => Ok(()),
            Err(err) => {
                let target = err
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<CommandFailure>())
                    .and_then(|f| extract_required_pydantic_core_version(&f.stderr));
                let Some(target) = target else {
                    return Err(err);
                };
                log::warn!(
                    "smoke test failed with pydantic-core/pydantic skew; \
                     reinstalling pydantic-core=={target} and retrying"
                );
                if let Err(repair_err) = self.repair_pydantic_core(&target) {
                    log::error!("pydantic-core repair failed: {repair_err:#}");
                    return Err(err);
                }
                self.smoke_test_headroom_with_timeout(HEADROOM_SMOKE_TEST_TIMEOUT)
            }
        }
    }

    fn smoke_test_headroom_with_timeout(&self, timeout: Duration) -> Result<()> {
        let python = self.runtime.managed_python();
        if let Err(err) = run_command_with_timeout(
            &python,
            &["-c", "import headroom; import headroom.proxy.server"],
            &self.runtime.root_dir,
            timeout,
        )
        .with_context(|| format!("running smoke test with {}", python.display()))
        {
            return Err(anyhow::Error::new(CommandFailure {
                program: python.display().to_string(),
                args: vec![
                    "-c".into(),
                    "import headroom; import headroom.proxy.server".into(),
                ],
                stdout: err
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<CommandFailure>())
                    .map(|failure| failure.stdout.clone())
                    .unwrap_or_default(),
                stderr: err
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<CommandFailure>())
                    .map(|failure| failure.stderr.clone())
                    .unwrap_or_else(|| format!("{err:#}")),
                exit_code: err
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<CommandFailure>())
                    .and_then(|failure| failure.exit_code),
                signal: err
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<CommandFailure>())
                    .and_then(|failure| failure.signal),
            }))
            .context("Headroom smoke test failed — the new version cannot be imported");
        }
        Ok(())
    }

    /// Apply an ad-hoc codesign signature to every native extension (.so /
    /// .dylib) under the venv's site-packages. PyPI wheels arrive unsigned,
    /// and some endpoint protection (EDR) tooling either blocks unsigned
    /// freshly-extracted binaries outright or makes them slower to load on
    /// first execution. An ad-hoc signature (`codesign --force --sign -`)
    /// satisfies macOS Gatekeeper's "signed" check and clears at least one
    /// class of EDR heuristic without us shipping a Developer ID at runtime.
    ///
    /// Best-effort: failures are logged and ignored. The install must not
    /// fail because codesign couldn't sign one file — the smoke test that
    /// follows is the real gate.
    fn ad_hoc_sign_venv_natives(&self) -> usize {
        if !cfg!(target_os = "macos") {
            return 0;
        }
        let site_packages = self
            .runtime
            .venv_dir
            .join("lib")
            .join("python3.12")
            .join("site-packages");
        if !site_packages.exists() {
            return 0;
        }
        let mut native_paths: Vec<PathBuf> = Vec::new();
        if let Err(err) = collect_native_extensions(&site_packages, &mut native_paths) {
            log::warn!(
                "ad-hoc codesign skipped: failed to walk {}: {err:#}",
                site_packages.display()
            );
            return 0;
        }
        if native_paths.is_empty() {
            return 0;
        }
        let total = native_paths.len();
        // One codesign invocation can accept many file arguments; ARG_MAX
        // (~256KB on macOS) is well above what we'd hit even with 1000+
        // long paths, so we avoid the per-file fork-exec overhead.
        let output = Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .args(&native_paths)
            .output();
        match output {
            Ok(out) if out.status.success() => {
                log::info!("ad-hoc codesign signed {total} venv native extensions");
                total
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                log::warn!(
                    "ad-hoc codesign exited {:?} for {total} files: {}",
                    out.status.code(),
                    stderr.trim()
                );
                0
            }
            Err(err) => {
                log::warn!("ad-hoc codesign failed to spawn: {err:#}");
                0
            }
        }
    }

    fn venv_backup_dir(&self) -> PathBuf {
        let mut dir = self.runtime.venv_dir.clone();
        let file_name = format!(
            "{}.backup",
            dir.file_name().and_then(|n| n.to_str()).unwrap_or("venv")
        );
        dir.set_file_name(file_name);
        dir
    }

    fn headroom_receipt_path(&self) -> PathBuf {
        self.runtime.tools_dir.join("headroom.json")
    }

    fn headroom_receipt_backup_path(&self) -> PathBuf {
        self.runtime.tools_dir.join("headroom.json.backup")
    }

    fn upgrade_marker_path(&self) -> PathBuf {
        self.runtime.runtime_dir.join("upgrade.in_progress.json")
    }

    fn write_upgrade_marker(
        &self,
        target_version: &str,
        in_place_previous_version: Option<&str>,
        in_place_previous_lock_backup: Option<&Path>,
    ) -> Result<()> {
        let marker = self.upgrade_marker_path();
        let mut body = json!({
            "target_version": target_version,
            "started_at": Utc::now().to_rfc3339(),
        });
        if let Some(previous) = in_place_previous_version {
            body["in_place"] = json!(true);
            body["previous_version"] = json!(previous);
        }
        if let Some(backup) = in_place_previous_lock_backup {
            body["previous_lock_backup"] = json!(backup);
        }
        std::fs::write(&marker, serde_json::to_vec_pretty(&body)?)
            .with_context(|| format!("writing {}", marker.display()))?;
        Ok(())
    }

    /// Read the in-progress upgrade marker and, if it records an in-place
    /// upgrade, return (previous_version, target_version, previous_lock_backup).
    /// Returns None for missing markers and for full-venv-rebuild markers.
    fn read_in_place_marker(&self) -> Option<(String, String, Option<PathBuf>)> {
        let bytes = std::fs::read(self.upgrade_marker_path()).ok()?;
        let body: Value = serde_json::from_slice(&bytes).ok()?;
        if body.get("in_place").and_then(|v| v.as_bool()) != Some(true) {
            return None;
        }
        let previous = body.get("previous_version")?.as_str()?.to_string();
        let target = body.get("target_version")?.as_str()?.to_string();
        let lock_backup = body
            .get("previous_lock_backup")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        Some((previous, target, lock_backup))
    }

    fn clear_upgrade_marker(&self) {
        let _ = std::fs::remove_file(self.upgrade_marker_path());
    }

    /// Inspect disk state for the signature of an interrupted previous upgrade
    /// and restore the backup venv as the live venv if so.
    ///
    /// Interrupted = upgrade marker file present. The backup venv is treated
    /// as the canonical "old, working" one; the live venv (if any) is whatever
    /// partial state was left behind. Safe to call at every upgrade entry.
    ///
    /// Returns true if recovery was performed.
    pub fn recover_from_interrupted_upgrade(&self) -> bool {
        let marker = self.upgrade_marker_path();
        if !marker.exists() {
            return false;
        }

        // In-place recovery: pip install is atomic per-package, so the venv
        // may be in a mixed state (some packages at target pins, others at
        // previous pins). Restore deps from the lock snapshot (if the
        // interrupted upgrade took that path) and force-reinstall the prior
        // headroom-ai so the next launch starts from a known-good state.
        // `check_headroom_upgrade` will then retry the swap fresh.
        if let Some((previous_version, _target, previous_lock_backup)) = self.read_in_place_marker()
        {
            log::info!(
                "recover_from_interrupted_upgrade: in-place upgrade was in progress; \
                 reinstalling previous headroom-ai {previous_version}"
            );
            if let Some(ref backup) = previous_lock_backup {
                let _ = self.pip_restore_deps_from_backup(backup);
                let _ = std::fs::copy(backup, self.active_lock_path());
                let _ = std::fs::remove_file(backup);
            }
            let _ = self.pip_force_reinstall_headroom_version(&previous_version);
            let receipt_backup = self.headroom_receipt_backup_path();
            let receipt_path = self.headroom_receipt_path();
            if receipt_backup.exists() {
                let _ = std::fs::copy(&receipt_backup, &receipt_path);
                let _ = std::fs::remove_file(&receipt_backup);
            }
            self.clear_upgrade_marker();
            return true;
        }

        let backup_dir = self.venv_backup_dir();
        let venv_dir = &self.runtime.venv_dir;
        let receipt_backup = self.headroom_receipt_backup_path();
        let receipt_path = self.headroom_receipt_path();

        log::info!(
            "recover_from_interrupted_upgrade: found stale marker at {}; restoring backup",
            marker.display()
        );

        if backup_dir.exists() {
            // The live venv (if present) is a partial/unknown new install.
            // Blow it away and put the backup back in its place.
            if venv_dir.exists() {
                if let Err(err) = std::fs::remove_dir_all(venv_dir) {
                    log::error!(
                        "recover_from_interrupted_upgrade: failed to remove partial venv at {}: {err}",
                        venv_dir.display()
                    );
                    // Leave everything in place; clearing the marker would be
                    // worse than leaving it for a later manual intervention.
                    return false;
                }
            }
            if let Err(err) = std::fs::rename(&backup_dir, venv_dir) {
                log::error!(
                    "recover_from_interrupted_upgrade: failed to restore venv from {}: {err}",
                    backup_dir.display()
                );
                return false;
            }
            if receipt_backup.exists() {
                let _ = std::fs::copy(&receipt_backup, &receipt_path);
                let _ = std::fs::remove_file(&receipt_backup);
            }
        } else {
            // No backup to restore from. Rare — the user (or a script) deleted
            // the backup dir while the marker was still live. Best we can do
            // is clear the marker so we don't loop on this state.
            log::warn!(
                "recover_from_interrupted_upgrade: no backup at {}; clearing marker",
                backup_dir.display()
            );
        }
        self.clear_upgrade_marker();
        true
    }

    /// Atomic runtime upgrade. Moves the current venv aside, creates a fresh
    /// venv at the original path, installs the new release, runs a smoke test.
    ///
    /// On success: returns `InstalledPendingValidation` — the backup is **still
    /// on disk** and the caller must call either [`commit_headroom_upgrade`] (if
    /// the new proxy boots) or [`rollback_headroom_upgrade`] (if it doesn't).
    ///
    /// On failure in any install step: rolls back internally, restoring the
    /// previous venv + receipt byte-for-byte, and returns `InstallFailed`.
    ///
    /// `force_rebuild` skips the in-place upgrade attempt and goes straight
    /// to the move-aside-and-rebuild path. Used by the user-facing "Retry
    /// with full rebuild" recovery flow when an in-place upgrade installed
    /// cleanly but boot validation failed (typically an ABI mismatch in
    /// native deps that pip can't detect).
    pub fn atomic_upgrade_headroom<F>(
        &self,
        release: &HeadroomRelease,
        mut progress: F,
        force_rebuild: bool,
    ) -> UpgradeOutcome
    where
        F: FnMut(BootstrapStepUpdate),
    {
        progress(BootstrapStepUpdate {
            step: "Preparing update",
            message: "Checking for previous upgrade state.".into(),
            eta_seconds: 2,
            percent: 5,
        });

        // If a prior upgrade was interrupted (process killed between
        // move-aside and success-commit), the backup is the REAL venv.
        // Restore it before doing anything destructive.
        let _recovered = self.recover_from_interrupted_upgrade();

        // In-place path: mutate the live venv rather than rebuilding it.
        // Covers both the wheel-only case (lock unchanged) and the lock-churn
        // case (`pip install --upgrade -r lock` reinstalls only the pins that
        // actually differ). Skipped when `force_rebuild` is set (user-
        // initiated recovery from a botched in-place upgrade) or when
        // `prepare_in_place_upgrade` decides the receipt isn't safe to
        // mutate in-place.
        if !force_rebuild {
            if let Some(ctx) = self.prepare_in_place_upgrade() {
                return self.in_place_upgrade_headroom(release, ctx, progress);
            }
        }

        let venv_dir = self.runtime.venv_dir.clone();
        let backup_dir = self.venv_backup_dir();
        let receipt_path = self.headroom_receipt_path();
        let receipt_backup = self.headroom_receipt_backup_path();

        // Best-effort: purge any leftover backup from a cleanly-completed
        // previous upgrade. recover_from_interrupted_upgrade above has
        // already handled any backup that belongs to an in-flight upgrade.
        if backup_dir.exists() {
            if let Err(err) = std::fs::remove_dir_all(&backup_dir) {
                return UpgradeOutcome::InstallFailed {
                    restored: false,
                    error: anyhow!(
                        "failed to remove stale venv backup at {}: {err}",
                        backup_dir.display()
                    ),
                };
            }
        }
        let _ = std::fs::remove_file(&receipt_backup);

        // Disk-space pre-check: building a fresh venv doubles space usage
        // momentarily. Refuse if less than 1 GB is free on the root volume.
        if let Some(avail) = available_disk_bytes(&self.runtime.root_dir) {
            const ONE_GB: u64 = 1_024 * 1_024 * 1_024;
            if avail < ONE_GB {
                return UpgradeOutcome::InstallFailed {
                    restored: false,
                    error: anyhow!(
                        "insufficient disk space for runtime upgrade: {} MB free, 1024 MB required",
                        avail / (1024 * 1024)
                    ),
                };
            }
        }

        // Move current venv + receipt aside. Write the in-progress marker
        // FIRST so that if we're killed between the rename and
        // commit/rollback, the next launch can recognize and recover.
        let had_live_venv = venv_dir.exists();
        if had_live_venv {
            if let Err(err) = self.write_upgrade_marker(&release.version, None, None) {
                return UpgradeOutcome::InstallFailed {
                    restored: false,
                    error: err.context("writing upgrade-in-progress marker"),
                };
            }
            if let Err(err) = std::fs::rename(&venv_dir, &backup_dir) {
                self.clear_upgrade_marker();
                return UpgradeOutcome::InstallFailed {
                    restored: false,
                    error: anyhow!("failed to move {} aside: {err}", venv_dir.display()),
                };
            }
        }
        let had_receipt = receipt_path.exists();
        if had_receipt {
            if let Err(err) = std::fs::copy(&receipt_path, &receipt_backup) {
                let restored = self.restore_venv_from_backup(had_live_venv);
                return UpgradeOutcome::InstallFailed {
                    restored,
                    error: anyhow!("failed to snapshot {}: {err}", receipt_path.display()),
                };
            }
        }

        progress(BootstrapStepUpdate {
            step: "Creating environment",
            message: "Creating isolated Headroom virtual environment.".into(),
            eta_seconds: 20,
            percent: 15,
        });

        if let Err(err) = self.create_managed_venv() {
            let restored = self.rollback_partial_upgrade(had_live_venv, had_receipt);
            return UpgradeOutcome::InstallFailed {
                restored,
                error: err.context("creating replacement Headroom virtualenv"),
            };
        }

        // install_headroom_release emits its own granular progress from ~40-90%.
        let pip_capture = std::cell::RefCell::new(PipOutputCapture::new(100));
        if let Err(err) = self.install_headroom_release(release, &mut progress, Some(&pip_capture))
        {
            let restored = self.rollback_partial_upgrade(had_live_venv, had_receipt);
            return UpgradeOutcome::InstallFailed {
                restored,
                error: err,
            };
        }

        progress(BootstrapStepUpdate {
            step: "Verifying install",
            message: "Running Headroom import smoke test.".into(),
            eta_seconds: 3,
            percent: 95,
        });

        if let Err(err) = self.smoke_test_headroom() {
            let restored = self.rollback_partial_upgrade(had_live_venv, had_receipt);
            return UpgradeOutcome::InstallFailed {
                restored,
                error: err,
            };
        }

        // Re-stamp the READY flag on the fresh venv. Without this,
        // `python_runtime_installed()` returns false (the flag lives inside
        // venv_dir, which was replaced during the swap), which would make
        // `ensure_headroom_running()` early-return without spawning the
        // new proxy — silently breaking boot validation.
        if let Err(err) = self.write_ready_flag() {
            let restored = self.rollback_partial_upgrade(had_live_venv, had_receipt);
            return UpgradeOutcome::InstallFailed {
                restored,
                error: err.context("writing READY flag on upgraded venv"),
            };
        }

        progress(BootstrapStepUpdate {
            step: "Install complete",
            message: "Install finished. Verifying Headroom boot…".into(),
            eta_seconds: 0,
            percent: 97,
        });

        UpgradeOutcome::InstalledPendingValidation {
            pip_output_tail: pip_capture.into_inner().into_string(),
        }
    }

    /// Tear down the new venv and restore the previous one. Called by the
    /// `state.rs` upgrade coordinator when boot validation fails.
    /// Idempotent — no-op if no backup exists.
    pub fn rollback_headroom_upgrade(&self) -> Result<()> {
        // In-place rollback: no venv backup. Restore deps from the lock
        // snapshot (if the upgrade touched the lock), then pip-reinstall the
        // previous headroom-ai and restore the receipt.
        if let Some((previous_version, _target, previous_lock_backup)) = self.read_in_place_marker()
        {
            if let Some(ref backup) = previous_lock_backup {
                self.pip_restore_deps_from_backup(backup).with_context(|| {
                    format!(
                        "rollback failed — could not restore dependencies from {}",
                        backup.display()
                    )
                })?;
                let _ = std::fs::copy(backup, self.active_lock_path());
                let _ = std::fs::remove_file(backup);
            }
            self.pip_force_reinstall_headroom_version(&previous_version)
                .with_context(|| {
                    format!(
                        "rollback failed — could not reinstall previous Headroom version {previous_version}"
                    )
                })?;
            let receipt_backup = self.headroom_receipt_backup_path();
            let receipt_path = self.headroom_receipt_path();
            if receipt_backup.exists() {
                std::fs::copy(&receipt_backup, &receipt_path)
                    .with_context(|| format!("restoring {}", receipt_path.display()))?;
                let _ = std::fs::remove_file(&receipt_backup);
            }
            self.clear_upgrade_marker();
            return Ok(());
        }

        let backup_dir = self.venv_backup_dir();
        if !backup_dir.exists() {
            return Ok(());
        }
        let had_live_venv = true; // by definition, if we have a backup
        let had_receipt = self.headroom_receipt_backup_path().exists();
        let restored = self.rollback_partial_upgrade(had_live_venv, had_receipt);
        if !restored {
            bail!(
                "rollback failed — venv.backup is present but could not be restored to {}",
                self.runtime.venv_dir.display()
            );
        }
        Ok(())
    }

    /// Returns true if the bundled requirements lock's dep pins differ from
    /// what was installed (ignoring comment/whitespace churn). Conservative:
    /// if we can't determine the installed sha, assume it differs.
    fn lock_pins_differ_from_installed(&self) -> bool {
        let Some(stored) = self.installed_requirements_lock_sha() else {
            return true;
        };
        let current = requirements_lock_sha(bootstrap_requirements_lock());
        stored != current && !LEGACY_REQUIREMENTS_LOCK_SHAS.contains(&stored.as_str())
    }

    fn active_lock_path(&self) -> PathBuf {
        self.runtime
            .downloads_dir
            .join("headroom-requirements.lock")
    }

    fn lock_backup_path(&self) -> PathBuf {
        self.runtime
            .downloads_dir
            .join("headroom-requirements.lock.backup")
    }

    /// Prepare to upgrade the runtime in place (no venv rebuild). Returns
    /// `None` when the caller should fall back to the full atomic rebuild:
    /// either there is no prior install to upgrade, the previously-installed
    /// version is below `ATOMIC_REBUILD_FLOOR_VERSION` (in-place pip across
    /// that delta leaves stale native libs), or the lock churned but the
    /// active lock file is missing on disk so we can't safely snapshot for
    /// rollback.
    ///
    /// When `Some`, the caller owns `previous_lock_backup` (if set): on
    /// success, `commit_headroom_upgrade` deletes it; on failure, rollback
    /// uses it to restore the prior pin set.
    fn prepare_in_place_upgrade(&self) -> Option<InPlaceUpgradeContext> {
        let previous_version = self.installed_headroom_version()?;
        if receipt_requires_atomic_rebuild(&previous_version) {
            log::info!(
                "prepare_in_place_upgrade: receipt {} predates atomic-rebuild floor {:?}; \
                 forcing full venv rebuild",
                previous_version,
                ATOMIC_REBUILD_FLOOR_VERSION
            );
            return None;
        }
        let previous_lock_backup = if self.lock_pins_differ_from_installed() {
            let active = self.active_lock_path();
            if !active.exists() {
                return None;
            }
            let backup = self.lock_backup_path();
            let _ = std::fs::remove_file(&backup);
            std::fs::copy(&active, &backup).ok()?;
            Some(backup)
        } else {
            None
        };
        Some(InPlaceUpgradeContext {
            previous_version,
            previous_lock_backup,
        })
    }

    /// In-place upgrade: mutate the live venv rather than rebuilding it.
    /// When `ctx.previous_lock_backup` is set, runs
    /// `pip install --upgrade -r lock` first so only churned dep pins are
    /// reinstalled (pip skips packages already at the pinned version). Then
    /// force-reinstalls the new `headroom-ai` wheel.
    ///
    /// On any failure, attempts to restore the prior version and (if
    /// applicable) the prior lock via the same pip mechanism.
    fn in_place_upgrade_headroom<F>(
        &self,
        release: &HeadroomRelease,
        ctx: InPlaceUpgradeContext,
        mut progress: F,
    ) -> UpgradeOutcome
    where
        F: FnMut(BootstrapStepUpdate),
    {
        let receipt_path = self.headroom_receipt_path();
        let receipt_backup = self.headroom_receipt_backup_path();

        // Purge stale receipt backup from any cleanly-completed prior upgrade.
        let _ = std::fs::remove_file(&receipt_backup);

        // Snapshot the receipt so rollback can restore the old artifact pointers.
        if receipt_path.exists() {
            if let Err(err) = std::fs::copy(&receipt_path, &receipt_backup) {
                if let Some(ref p) = ctx.previous_lock_backup {
                    let _ = std::fs::remove_file(p);
                }
                return UpgradeOutcome::InstallFailed {
                    restored: true,
                    error: anyhow!("failed to snapshot {}: {err}", receipt_path.display()),
                };
            }
        }

        // Write marker so an interrupted upgrade can be recovered on next launch.
        if let Err(err) = self.write_upgrade_marker(
            &release.version,
            Some(&ctx.previous_version),
            ctx.previous_lock_backup.as_deref(),
        ) {
            let _ = std::fs::remove_file(&receipt_backup);
            if let Some(ref p) = ctx.previous_lock_backup {
                let _ = std::fs::remove_file(p);
            }
            return UpgradeOutcome::InstallFailed {
                restored: true,
                error: err.context("writing upgrade-in-progress marker"),
            };
        }

        // Bounded ring buffer collecting pip stdout/stderr across both
        // install steps. Attached to the boot-validation Sentry event when
        // it later fails — pip can return exit 0 while leaving the venv
        // broken (skipped packages, ABI-mismatched native deps), and the
        // tail is the only forensic record of what pip actually did.
        let pip_capture = std::cell::RefCell::new(PipOutputCapture::new(100));

        // Dep-lock upgrade (only when pins changed).
        if ctx.previous_lock_backup.is_some() {
            progress(BootstrapStepUpdate {
                step: "Updating dependencies",
                message: "Updating Headroom's bundled dependencies.".into(),
                eta_seconds: 45,
                percent: 15,
            });

            let requirements_lock = bootstrap_requirements_lock();
            let lock_path = match self.write_headroom_requirements_lock(requirements_lock) {
                Ok(p) => p,
                Err(err) => {
                    let restored = self.rollback_in_place_upgrade_inner(&ctx);
                    return UpgradeOutcome::InstallFailed {
                        restored,
                        error: err,
                    };
                }
            };

            let deps_start = std::time::Instant::now();
            let deps_progress_ref = std::cell::RefCell::new(&mut progress);
            let mut dep_counter: u32 = 0;
            if let Err(err) = run_pip_install_with_retries_streaming(
                &self.runtime.managed_python(),
                &[
                    "-m",
                    "pip",
                    "install",
                    "--timeout",
                    "180",
                    "--retries",
                    "10",
                    "--find-links",
                    VENDOR_WHEELS_INDEX_URL,
                    "--extra-index-url",
                    "https://pypi.org/simple",
                    "--upgrade",
                    "--requirement",
                    lock_path.to_string_lossy().as_ref(),
                ],
                &self.runtime.root_dir,
                |line| {
                    pip_capture.borrow_mut().push(line);
                    if let Some(update) =
                        pip_line_to_progress(line, deps_start.elapsed(), &mut dep_counter, 15, 55)
                    {
                        if let Ok(mut cb) = deps_progress_ref.try_borrow_mut() {
                            (cb)(update);
                        }
                    }
                },
            ) {
                let restored = self.rollback_in_place_upgrade_inner(&ctx);
                return UpgradeOutcome::InstallFailed {
                    restored,
                    error: err.context("upgrading Headroom's bundled dependencies in place"),
                };
            }
        }

        progress(BootstrapStepUpdate {
            step: "Downloading update",
            message: "Fetching Headroom engine update bundle.".into(),
            eta_seconds: 10,
            percent: 60,
        });

        let wheel_path = self
            .runtime
            .downloads_dir
            .join(format!("headroom_ai-{}-py3-none-any.whl", release.version));
        let use_wheel =
            match download_to_path(&release.wheel_url, &wheel_path, Some(&release.sha256)) {
                Ok(()) => true,
                Err(download_err) => {
                    log::warn!(
                    "headroom wheel download failed (will fall back to pip index): {download_err}"
                );
                    false
                }
            };

        progress(BootstrapStepUpdate {
            step: "Applying update",
            message: "Installing the new Headroom wheel.".into(),
            eta_seconds: 10,
            percent: 75,
        });

        let headroom_spec = format!("headroom-ai=={}", release.version);
        let headroom_arg = if use_wheel {
            wheel_path.to_string_lossy().into_owned()
        } else {
            headroom_spec.clone()
        };
        if let Err(err) = run_pip_install_with_retries_streaming(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--extra-index-url",
                "https://pypi.org/simple",
                "--no-deps",
                "--force-reinstall",
                &headroom_arg,
            ],
            &self.runtime.root_dir,
            |line| {
                pip_capture.borrow_mut().push(line);
            },
        ) {
            let restored = self.rollback_in_place_upgrade_inner(&ctx);
            let context_msg = if use_wheel {
                "installing verified Headroom wheel into Headroom-managed virtualenv"
            } else {
                "installing headroom-ai from PyPI into Headroom-managed virtualenv"
            };
            return UpgradeOutcome::InstallFailed {
                restored,
                error: err.context(context_msg),
            };
        }

        // Ad-hoc sign every native extension pip just dropped in. Failures
        // are logged and ignored; the smoke test below is the real gate.
        self.ad_hoc_sign_venv_natives();

        progress(BootstrapStepUpdate {
            step: "Verifying install",
            message: "Running Headroom import smoke test.".into(),
            eta_seconds: 3,
            percent: 85,
        });

        if let Err(err) = self.smoke_test_headroom() {
            let restored = self.rollback_in_place_upgrade_inner(&ctx);
            return UpgradeOutcome::InstallFailed {
                restored,
                error: err,
            };
        }

        // Optional addon: an in-place upgrade keeps the venv, so markitdown
        // should still run. Warn-only — a broken optional addon must not fail
        // the core Headroom upgrade.
        if let Err(err) = self.smoke_test_markitdown() {
            log::warn!("markitdown smoke test failed after upgrade: {err:#}");
        }
        if let Err(err) = self.smoke_test_ponytail() {
            log::warn!("ponytail smoke test failed after upgrade: {err:#}");
        }

        progress(BootstrapStepUpdate {
            step: "Configuring integrations",
            message: "Setting up Headroom MCP integration.".into(),
            eta_seconds: 5,
            percent: 92,
        });

        let mcp_install = match self.install_headroom_mcp() {
            Ok(method) => json!({
                "configured": true,
                "proxyUrl": HEADROOM_PROXY_URL,
                "installMethod": method.as_str(),
            }),
            Err(err) => {
                log::info!("headroom MCP setup skipped: {err:#}");
                json!({
                    "configured": false,
                    "proxyUrl": HEADROOM_PROXY_URL,
                    "error": err.to_string(),
                })
            }
        };

        if let Err(err) = self.update_headroom_receipt_after_in_place_upgrade(release, mcp_install)
        {
            let restored = self.rollback_in_place_upgrade_inner(&ctx);
            return UpgradeOutcome::InstallFailed {
                restored,
                error: err,
            };
        }

        progress(BootstrapStepUpdate {
            step: "Install complete",
            message: "Install finished. Verifying Headroom boot…".into(),
            eta_seconds: 0,
            percent: 97,
        });

        UpgradeOutcome::InstalledPendingValidation {
            pip_output_tail: pip_capture.into_inner().into_string(),
        }
    }

    fn pip_force_reinstall_headroom_version(&self, version: &str) -> Result<()> {
        let spec = format!("headroom-ai=={version}");
        run_pip_install_with_retries(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--extra-index-url",
                "https://pypi.org/simple",
                "--no-deps",
                "--force-reinstall",
                &spec,
            ],
            &self.runtime.root_dir,
        )
        .with_context(|| format!("reinstalling Headroom version {version}"))
    }

    /// Recover from a pydantic / pydantic-core version skew by reinstalling
    /// pydantic-core at the version pydantic wants. Triggered when the proxy
    /// log shows the SystemError pydantic raises during import. `--no-deps`
    /// keeps the rest of the venv untouched.
    fn repair_pydantic_core(&self, target_version: &str) -> Result<()> {
        // Reinstall pydantic itself first (no version pin) to rewrite its
        // dist-info. A failed prior upgrade can leave two `pydantic-X.Y.dist-info`
        // dirs in site-packages; `importlib.metadata.metadata('pydantic')` then
        // returns either one non-deterministically, producing flip-flopping
        // "requires N.N.N" errors across attempts. Force-reinstalling pydantic
        // collapses the duplicates so the next pin we apply actually matches
        // what pydantic asks for.
        run_pip_install_with_retries(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--extra-index-url",
                "https://pypi.org/simple",
                "--no-deps",
                "--force-reinstall",
                "pydantic",
            ],
            &self.runtime.root_dir,
        )
        .with_context(|| "reinstalling pydantic to clear duplicate dist-info")?;

        let spec = format!("pydantic-core=={target_version}");
        run_pip_install_with_retries(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--extra-index-url",
                "https://pypi.org/simple",
                "--no-deps",
                "--force-reinstall",
                &spec,
            ],
            &self.runtime.root_dir,
        )
        .with_context(|| format!("reinstalling pydantic-core=={target_version}"))
    }

    /// Restore deps from `previous_lock_backup` via
    /// `pip install --upgrade -r <backup>` — packages already at the old pin
    /// are skipped by pip, only packages that were actually churned by the
    /// failed upgrade get reinstalled.
    fn pip_restore_deps_from_backup(&self, backup_lock: &Path) -> Result<()> {
        run_pip_install_with_retries(
            &self.runtime.managed_python(),
            &[
                "-m",
                "pip",
                "install",
                "--timeout",
                "180",
                "--retries",
                "10",
                "--find-links",
                VENDOR_WHEELS_INDEX_URL,
                "--extra-index-url",
                "https://pypi.org/simple",
                "--upgrade",
                "--requirement",
                backup_lock.to_string_lossy().as_ref(),
            ],
            &self.runtime.root_dir,
        )
        .with_context(|| {
            format!(
                "restoring Headroom dependencies from {}",
                backup_lock.display()
            )
        })
    }

    fn rollback_in_place_upgrade_inner(&self, ctx: &InPlaceUpgradeContext) -> bool {
        // Restore deps first so headroom-ai lands on a consistent dep set.
        let deps_ok = match ctx.previous_lock_backup.as_deref() {
            Some(backup) => {
                let ok = self.pip_restore_deps_from_backup(backup).is_ok();
                let active = self.active_lock_path();
                let _ = std::fs::copy(backup, &active);
                let _ = std::fs::remove_file(backup);
                ok
            }
            None => true,
        };
        let wheel_ok = self
            .pip_force_reinstall_headroom_version(&ctx.previous_version)
            .is_ok();
        let receipt_backup = self.headroom_receipt_backup_path();
        let receipt_path = self.headroom_receipt_path();
        let receipt_ok = if receipt_backup.exists() {
            let copy_ok = std::fs::copy(&receipt_backup, &receipt_path).is_ok();
            let _ = std::fs::remove_file(&receipt_backup);
            copy_ok
        } else {
            true
        };
        self.clear_upgrade_marker();
        deps_ok && wheel_ok && receipt_ok
    }

    fn update_headroom_receipt_after_in_place_upgrade(
        &self,
        release: &HeadroomRelease,
        mcp_install: Value,
    ) -> Result<()> {
        let receipt_path = self.headroom_receipt_path();
        let bytes = std::fs::read(&receipt_path)
            .with_context(|| format!("reading {}", receipt_path.display()))?;
        let mut receipt: Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", receipt_path.display()))?;
        receipt["version"] = json!(release.version);
        if let Some(artifact) = receipt.get_mut("artifact").and_then(|a| a.as_object_mut()) {
            artifact.insert("url".into(), json!(release.wheel_url));
            artifact.insert("sha256".into(), json!(release.sha256));
            artifact.insert(
                "requirementsLockSha256".into(),
                json!(requirements_lock_sha(bootstrap_requirements_lock())),
            );
        }
        receipt["mcp"] = mcp_install;
        std::fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt)?)
            .with_context(|| format!("writing {}", receipt_path.display()))?;
        Ok(())
    }

    /// Finalize a successful atomic upgrade. Deletes the backup venv and
    /// receipt snapshot. Non-fatal if cleanup fails — a future upgrade's
    /// "purge stale backup" step will clean up whatever we left behind.
    pub fn commit_headroom_upgrade(&self) -> Result<()> {
        let backup_dir = self.venv_backup_dir();
        if backup_dir.exists() {
            if let Err(err) = std::fs::remove_dir_all(&backup_dir) {
                log::warn!(
                    "commit_headroom_upgrade: non-fatal: failed to remove {}: {err}",
                    backup_dir.display()
                );
            }
        }
        let _ = std::fs::remove_file(self.headroom_receipt_backup_path());
        let _ = std::fs::remove_file(self.lock_backup_path());
        // Clear the in-progress marker last, so a mid-commit crash (e.g.,
        // between the remove_dir_all of the backup and the marker cleanup)
        // still looks like an interrupted upgrade on the next launch and
        // triggers recovery rather than a potentially-unsafe purge.
        self.clear_upgrade_marker();
        Ok(())
    }

    /// Restore both venv + receipt from their backups. Used from the atomic
    /// upgrade failure path and from the post-boot-validation rollback path.
    /// Returns true if the restore succeeded.
    fn rollback_partial_upgrade(&self, had_live_venv: bool, had_receipt: bool) -> bool {
        // Remove any partial new venv.
        if self.runtime.venv_dir.exists() {
            if let Err(err) = std::fs::remove_dir_all(&self.runtime.venv_dir) {
                log::error!(
                    "rollback: failed to remove partial venv at {}: {err}",
                    self.runtime.venv_dir.display()
                );
                return false;
            }
        }
        let venv_restored = self.restore_venv_from_backup(had_live_venv);
        if !venv_restored {
            return false;
        }
        if had_receipt {
            let receipt_path = self.headroom_receipt_path();
            let receipt_backup = self.headroom_receipt_backup_path();
            if let Err(err) = std::fs::copy(&receipt_backup, &receipt_path) {
                log::error!(
                    "rollback: failed to restore {}: {err}",
                    receipt_path.display()
                );
                return false;
            }
            let _ = std::fs::remove_file(&receipt_backup);
        }
        // Rollback complete — clear the marker so we don't trigger recovery
        // on the next launch.
        self.clear_upgrade_marker();
        true
    }

    fn restore_venv_from_backup(&self, had_live_venv: bool) -> bool {
        if !had_live_venv {
            return true;
        }
        let backup_dir = self.venv_backup_dir();
        if !backup_dir.exists() {
            return true;
        }
        match std::fs::rename(&backup_dir, &self.runtime.venv_dir) {
            Ok(()) => true,
            Err(err) => {
                log::error!(
                    "rollback: failed to restore venv from {}: {err}",
                    backup_dir.display()
                );
                false
            }
        }
    }

    /// Runs MCP install if the receipt shows it is not configured, or was
    /// configured via the legacy `~/.claude/mcp.json` fallback (which Claude
    /// Code ≥2.x ignores). Safe to call at every launch — no-ops when the
    /// server is already registered via `claude mcp add` or direct json write.
    ///
    /// If the install fails with a Python `ModuleNotFoundError`/`ImportError`
    /// in stderr, the venv is missing one or more pinned dependencies despite
    /// the receipt's `requirementsLockSha256` saying otherwise (seen on
    /// upgrades from very-old desktop versions where a partial install left
    /// the receipt stamped but the venv incomplete). Self-heal by running the
    /// requirements repair, which re-installs the full lock file and retries
    /// MCP install internally.
    pub fn ensure_mcp_configured(&self) -> Result<()> {
        if self.headroom_mcp_configured() == Some(true)
            && matches!(
                self.headroom_mcp_install_method().as_deref(),
                Some(MCP_METHOD_CLAUDE_CLI) | Some(MCP_METHOD_DIRECT_CLAUDE_JSON)
            )
        {
            return Ok(());
        }
        let method = match self.install_headroom_mcp() {
            Ok(method) => method,
            Err(err) if looks_like_corrupt_venv_error(&err) => {
                log::warn!(
                    "MCP install hit a Python import error; running requirements repair: {err:#}"
                );
                sentry::capture_message(
                    "MCP install hit corrupt-venv signal; auto-running requirements repair",
                    sentry::Level::Info,
                );
                self.repair_stale_requirements_with_progress(|_| {})
                    .context("auto-repairing venv after MCP install import error")?;
                // repair_stale_requirements_with_progress runs install_headroom_mcp
                // and writes the mcp section of the receipt itself, so we're done.
                return Ok(());
            }
            Err(err) => return Err(err),
        };
        let receipt_path = self.runtime.tools_dir.join("headroom.json");
        if let Ok(bytes) = std::fs::read(&receipt_path) {
            if let Ok(mut receipt) = serde_json::from_slice::<Value>(&bytes) {
                receipt["mcp"] = json!({
                    "configured": true,
                    "proxyUrl": HEADROOM_PROXY_URL,
                    "installMethod": method.as_str(),
                });
                let _ = std::fs::write(&receipt_path, serde_json::to_vec(&receipt)?);
            }
        }
        Ok(())
    }

    fn install_headroom_mcp(&self) -> Result<McpInstallMethod> {
        let entrypoint = self.headroom_entrypoint();
        let detected_claude = crate::claude_cli::detect_claude_cli();

        // GUI apps launched from Finder/Dock inherit a minimal PATH that
        // excludes /opt/homebrew/bin, /usr/local/bin, ~/.claude/local/bin,
        // etc. Without augmentation, `shutil.which("claude")` inside the
        // Python CLI returns None and it falls back to writing
        // ~/.claude/mcp.json — a legacy path Claude Code ≥2.x does not read.
        let run_install = |force: bool| -> Result<(std::process::Output, Vec<&'static str>)> {
            let mut args: Vec<&'static str> =
                vec!["mcp", "install", "--proxy-url", HEADROOM_PROXY_URL];
            if force {
                args.push("--force");
            }
            let mut cmd = build_command(&entrypoint, &args[..], &self.runtime.root_dir);
            if let Some(claude_path) = detected_claude.as_ref() {
                if let Some(dir) = claude_path.parent() {
                    let existing = std::env::var("PATH").unwrap_or_default();
                    let augmented = if existing.is_empty() {
                        dir.display().to_string()
                    } else {
                        format!("{}:{}", dir.display(), existing)
                    };
                    cmd.env("PATH", augmented);
                }
            }
            let output = cmd
                .output()
                .with_context(|| format!("starting {} {}", entrypoint.display(), args.join(" ")))
                .context("configuring Headroom MCP integration")?;
            Ok((output, args))
        };

        // Always pass --force so that stale entrypoints left over from a
        // previous Headroom version (e.g. venv python3 path → headroom CLI)
        // are overwritten without a separate retry. --force is a no-op when
        // the config is already correct or absent. Desktop owns this config.
        let (output, args) = run_install(true)?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let exit_code = output.status.code();
            let signal = exit_status_signal(&output.status);

            // If the Python CLI exited non-zero only because no supported tool
            // (claude, codex, etc.) was detected on PATH, it wrote nothing --
            // but the direct JSON write path below can still configure the
            // integration. Fall through instead of surfacing a Sentry warning.
            if !stdout.contains("not detected on this system") {
                let detected = detected_claude
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "<not detected>".into());
                sentry::with_scope(
                    |scope| {
                        scope.set_extra("claude_cli_detected", detected.clone().into());
                        scope.set_extra(
                            "exit_code",
                            exit_code
                                .map(|c| c.into())
                                .unwrap_or(serde_json::Value::Null),
                        );
                        scope.set_extra(
                            "signal",
                            signal.map(|s| s.into()).unwrap_or(serde_json::Value::Null),
                        );
                        scope.set_extra(
                            "stdout_tail",
                            stdout[stdout.char_indices().rev().nth(2047).map_or(0, |(i, _)| i)..]
                                .into(),
                        );
                        scope.set_extra(
                            "stderr_tail",
                            stderr[stderr.char_indices().rev().nth(2047).map_or(0, |(i, _)| i)..]
                                .into(),
                        );
                    },
                    || {
                        sentry::capture_message(
                            "Headroom MCP install exited non-zero",
                            sentry::Level::Warning,
                        );
                    },
                );
                return Err(anyhow::Error::new(CommandFailure {
                    program: entrypoint.display().to_string(),
                    args: args.iter().map(|s| s.to_string()).collect(),
                    stdout,
                    stderr,
                    exit_code,
                    signal,
                }))
                .context("configuring Headroom MCP integration");
            }
        }

        // Ground truth: did Claude Code actually see the server? The Python
        // CLI's fallback branch writes ~/.claude/mcp.json (legacy, ignored by
        // Claude Code ≥2.x) and exits 0, so the subprocess succeeding is not
        // a reliable proxy for "integration works". Read the file Claude Code
        // actually reads and confirm the registration landed there.
        if claude_code_has_headroom_mcp_server() {
            return Ok(McpInstallMethod::ClaudeCli);
        }

        // The Python CLI couldn't find `claude` (e.g. GUI launch with bare
        // PATH) and wrote ~/.claude/mcp.json instead. Write the entry
        // directly to ~/.claude.json, which is what Claude Code ≥2.x reads.
        if let Ok(()) = write_headroom_to_claude_json(&entrypoint, HEADROOM_PROXY_URL) {
            if claude_code_has_headroom_mcp_server() {
                return Ok(McpInstallMethod::DirectClaudeJson);
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detected = detected_claude
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<not detected>".into());
        sentry::with_scope(
            |scope| {
                scope.set_extra("claude_cli_detected", detected.clone().into());
                scope.set_extra(
                    "stdout_tail",
                    stdout[stdout.char_indices().rev().nth(511).map_or(0, |(i, _)| i)..].into(),
                );
                scope.set_extra(
                    "stderr_tail",
                    stderr[stderr.char_indices().rev().nth(511).map_or(0, |(i, _)| i)..].into(),
                );
            },
            || {
                sentry::capture_message(
                    "Headroom MCP install exited 0 but Claude Code does not see the server \
                     (fell back to ~/.claude/mcp.json which Claude Code ≥2.x ignores).",
                    sentry::Level::Warning,
                );
            },
        );
        Ok(McpInstallMethod::FallbackJson)
    }

    fn write_headroom_requirements_lock(&self, contents: &str) -> Result<PathBuf> {
        let lock_path = self
            .runtime
            .downloads_dir
            .join("headroom-requirements.lock");
        std::fs::write(&lock_path, contents)
            .with_context(|| format!("writing {}", lock_path.display()))?;
        Ok(lock_path)
    }

    fn write_bootstrap_receipt(&self) -> Result<()> {
        let receipt = self.runtime.root_dir.join("bootstrap-receipt.json");
        std::fs::write(
            &receipt,
            serde_json::to_vec_pretty(&json!({
                "managedBy": "Headroom",
                "runtime": "python",
                "scope": "self-contained",
                "downloadsDir": self.runtime.downloads_dir,
                "managedBinDir": self.runtime.bin_dir,
                "pythonDistribution": self.runtime.standalone_python(),
                "managedPython": self.runtime.managed_python(),
                "managedPip": self.runtime.managed_pip(),
                "toolsDir": self.runtime.tools_dir
            }))
            .context("serializing bootstrap receipt")?,
        )
        .with_context(|| format!("writing {}", receipt.display()))?;
        Ok(())
    }

    fn write_ready_flag(&self) -> Result<()> {
        let ready_flag = self.runtime.ready_flag();
        std::fs::write(
            &ready_flag,
            json!({
                "managedPython": self.runtime.managed_python(),
                "managedPip": self.runtime.managed_pip(),
                "scope": "self-contained"
            })
            .to_string(),
        )
        .with_context(|| format!("writing {}", ready_flag.display()))?;
        Ok(())
    }

    fn write_tool_receipt(&self, tool_id: &str, payload: serde_json::Value) -> Result<()> {
        let path = self.runtime.tools_dir.join(format!("{tool_id}.json"));
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&payload).context("serializing managed tool receipt")?,
        )
        .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    fn read_tool_receipt(&self, tool_id: &str) -> Option<Value> {
        let path = self.runtime.tools_dir.join(format!("{tool_id}.json"));
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Optional tools persist an `enabled` flag in their receipt. Required core
    /// tools (headroom, rtk) are always enabled. Missing flag defaults to true.
    fn tool_enabled(&self, tool_id: &str) -> bool {
        self.read_tool_receipt(tool_id)
            .and_then(|receipt| receipt.get("enabled").and_then(Value::as_bool))
            .unwrap_or(true)
    }

    pub fn install_repo_memory_mcp(&self) -> Result<()> {
        let script = repo_memory_script_path("repo-intelligence.mjs")?;
        let node_command = resolve_command_path("node")
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "node".to_string());
        let command = format!("{} {} --mcp-serve", node_command, script.display());
        write_mcp_server_to_claude_json(
            REPO_MEMORY_MCP_NAME,
            json!({
                "command": node_command,
                "args": [script, "--mcp-serve"],
                "env": { "MAC_AI_SWITCHBOARD_REPO_MEMORY_READ_ONLY": "1" },
            }),
        )?;
        self.write_tool_receipt(
            "repo-memory",
            json!({
                "version": REPO_MEMORY_DISPLAY_VERSION,
                "mcp": {
                    "configured": true,
                    "serverName": REPO_MEMORY_MCP_NAME,
                    "readOnly": true,
                    "transport": "stdio",
                    "command": command,
                    "descriptorPath": self.runtime.tools_dir.join("repo-memory.json"),
                },
            }),
        )?;
        Ok(())
    }

    pub fn repo_memory_mcp_service_status(&self) -> Option<RepoMemoryMcpServiceStatus> {
        let descriptor_path = self.runtime.tools_dir.join("repo-memory.json");
        let script = repo_memory_script_path("repo-intelligence.mjs").ok()?;
        let descriptor_present = descriptor_path.exists();
        let script_present = script.exists();
        let node_command = resolve_command_path("node")
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "node".to_string());
        let node_available = resolve_command_path("node").is_some();
        let healthy = descriptor_present && script_present && node_available;
        let mut issues = Vec::new();
        if !descriptor_present {
            issues.push("descriptor_missing".to_string());
        }
        if !script_present {
            issues.push("script_missing".to_string());
        }
        if !node_available {
            issues.push("node_missing".to_string());
        }
        Some(RepoMemoryMcpServiceStatus {
            managed_by_app: true,
            read_only: true,
            transport: "stdio".to_string(),
            command: format!("{} {} --mcp-serve", node_command, script.display()),
            descriptor_path: descriptor_path.display().to_string(),
            descriptor_present,
            script_path: script.display().to_string(),
            script_present,
            node_available,
            healthy,
            issues,
        })
    }

    pub fn ensure_repo_memory_mcp_configured(&self) -> Result<()> {
        if self.repo_memory_mcp_configured() == Some(true) {
            return Ok(());
        }
        self.install_repo_memory_mcp()
    }

    pub fn verify_repo_memory_mcp_smoke(&self) -> Result<()> {
        let script = repo_memory_script_path("check-repo-memory-mcp.mjs")?;
        let cwd = script
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let script_arg = script.to_string_lossy().to_string();
        run_command_with_timeout(
            Path::new("node"),
            &[script_arg.as_str()],
            &cwd,
            REPO_MEMORY_MCP_SMOKE_TEST_TIMEOUT,
        )
        .context("verifying repo-memory MCP read-only smoke contract")?;
        Ok(())
    }

    pub fn repo_memory_mcp_configured(&self) -> Option<bool> {
        if !self.runtime.tools_dir.join("repo-memory.json").exists() {
            return Some(false);
        }
        Some(claude_code_has_mcp_server(REPO_MEMORY_MCP_NAME))
    }

    pub fn repo_memory_mcp_error(&self) -> Option<String> {
        match self.repo_memory_mcp_configured() {
            Some(true) => None,
            Some(false) => Some("repo-memory missing from Claude MCP config".to_string()),
            None => Some("Repo Memory MCP configuration could not be verified".to_string()),
        }
    }

    fn detect_status(&self, tool_id: &str) -> ToolStatus {
        if tool_id == "caveman" {
            // Pure receipt-backed guidance tool: presence of the receipt means
            // installed. Managed-block drift is surfaced by Doctor, not here.
            return if self.caveman_receipt_exists() {
                ToolStatus::Healthy
            } else {
                ToolStatus::NotInstalled
            };
        }
        if tool_id == "ponytail" {
            return self.ponytail_status();
        }
        let installed_path = self.runtime.tools_dir.join(format!("{tool_id}.json"));
        if installed_path.exists() && self.python_runtime_installed() {
            ToolStatus::Healthy
        } else {
            ToolStatus::NotInstalled
        }
    }
}

fn headroom_propagated_proxy_log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = PathBuf::from(home)
        .join(".headroom")
        .join("logs")
        .join("proxy.log");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Recursively collect every `.so` / `.dylib` under `dir`. Used by
/// `ad_hoc_sign_venv_natives` to enumerate the native extensions pip
/// dropped into the venv. Symlinks are followed via `read_dir`'s default
/// behavior on macOS, but `file_type` is checked so we don't recurse into
/// non-directories. Errors propagate so the caller can log + skip.
fn collect_native_extensions(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_native_extensions(&path, out)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "so" || ext == "dylib" {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// Hash a requirements lock file ignoring comments and blank lines, so that
/// header/comment churn does not force a full `pip install` on upgrade.
fn run_python_command(python: &Path, args: &[&str], cwd: &Path) -> Result<()> {
    run_command(python, args, cwd)
}

/// Path to the output-shaper savings ledger. Mirrors headroom's
/// `workspace_dir()` default of `~/.headroom` (neither the proxy nor the
/// seeding run sets `HEADROOM_WORKSPACE_DIR`, so both resolve here).
fn output_savings_ledger_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".headroom")
            .join("output_savings.json"),
    )
}

/// True once the verbosity baseline has been seeded (non-empty sample count).
/// The persisted baseline does not carry a `total_samples` field — that is a
/// computed property of the in-memory model. On disk the total observation
/// count lives in `baseline.glob.n` (the global accumulator), so that is what
/// we gate on. The proxy reports the savings estimate as unavailable until a
/// baseline with observations exists.
fn verbosity_baseline_present() -> bool {
    let Some(path) = output_savings_ledger_path() else {
        return false;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|json| {
            json.get("baseline")
                .and_then(|b| b.get("glob"))
                .and_then(|g| g.get("n"))
                .and_then(|n| n.as_u64())
        })
        .is_some_and(|n| n > 0)
}

/// Real project root (the transcript `cwd`) of the Claude Code project with the
/// most transcript bytes under `~/.claude/projects`. Reading `cwd` from a
/// transcript avoids lossily decoding the mangled `~/.claude/projects` dir name
/// — it is exactly the path headroom's plugin resolves to, so `--project <cwd>`
/// matches. Returns `None` when no non-empty transcript exists.
fn busiest_claude_project_cwd() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let projects_dir = PathBuf::from(home).join(".claude").join("projects");

    let mut best: Option<(u64, PathBuf)> = None;
    for entry in std::fs::read_dir(&projects_dir).ok()?.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let mut bytes = 0u64;
        if let Ok(files) = std::fs::read_dir(&dir) {
            for f in files.flatten() {
                let p = f.path();
                if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    if let Ok(meta) = f.metadata() {
                        bytes += meta.len();
                    }
                }
            }
        }
        if bytes > 0 && best.as_ref().is_none_or(|(b, _)| bytes > *b) {
            best = Some((bytes, dir));
        }
    }

    project_cwd_from_transcript_dir(&best?.1)
}

/// Pull the `cwd` field from the first transcript line that has one. Reads at
/// most a few lines so a multi-hundred-MB transcript dir stays cheap.
fn project_cwd_from_transcript_dir(dir: &Path) -> Option<String> {
    use std::io::{BufRead, BufReader};
    for f in std::fs::read_dir(dir).ok()?.flatten() {
        let p = f.path();
        if p.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(file) = std::fs::File::open(&p) else {
            continue;
        };
        for line in BufReader::new(file).lines().take(50).map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(cwd) = v.get("cwd").and_then(|c| c.as_str()) {
                    if !cwd.is_empty() {
                        return Some(cwd.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Runs a pip install invocation with retries on transient failures.
///
/// pip's own `--retries` flag only covers connection establishment, not
/// mid-stream read timeouts, so a single TCP stall during a wheel download
/// can fail the whole bootstrap (see Sentry bootstrap_failed reports). We
/// retry the full invocation; pip's cachecontrol layer persists partial
/// responses so retries resume cheaply instead of redownloading from zero.
fn run_pip_install_with_retries(python: &Path, args: &[&str], cwd: &Path) -> Result<()> {
    run_pip_install_with_retries_streaming(python, args, cwd, |_| {})
}

/// Translate a pip stdout/stderr line into a progress update, or None for
/// noise. Counter-based monotonic advance inside `[base_percent, max_percent-1]`:
/// we don't know the final dep count up-front, so each interesting line nudges
/// the bar forward and it saturates just below the parent step's ceiling.
// Compact representation of a pip-install failure for log/Sentry. The full
// CommandFailure Display dumps program + args + stdout + stderr, which the
// 400-char Sentry cap eats before any stderr lines appear. Pip's actual
// reason lives on stderr, so prefer the tail of stderr (or stdout if stderr
// is empty) plus exit code.
fn compact_pip_failure(err: &anyhow::Error) -> String {
    const TAIL_BUDGET: usize = 300;
    let Some(failure) = err.chain().find_map(|c| c.downcast_ref::<CommandFailure>()) else {
        return err.to_string();
    };
    let source = if !failure.stderr.trim().is_empty() {
        failure.stderr.as_str()
    } else {
        failure.stdout.as_str()
    };
    let trimmed = source.trim_end();
    let tail = if trimmed.len() > TAIL_BUDGET {
        let start = trimmed.len() - TAIL_BUDGET;
        let aligned = trimmed[start..]
            .find('\n')
            .map(|i| start + i + 1)
            .unwrap_or(start);
        &trimmed[aligned..]
    } else {
        trimmed
    };
    let exit = failure
        .exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "signal".into());
    format!("exit={exit}; stderr tail: {tail}")
}

/// Streaming variant of `run_pip_install_with_retries`. Each line emitted by
/// pip on stdout/stderr is piped through `on_line` as it arrives, so callers
/// can translate noteworthy pip events ("Collecting X", "Downloading Y",
/// "Installing collected packages", "Successfully installed") into
/// user-facing progress updates instead of staring at a static message for
/// the 60–90 seconds a large pip install takes.
fn run_pip_install_with_retries_streaming<F>(
    python: &Path,
    args: &[&str],
    cwd: &Path,
    mut on_line: F,
) -> Result<()>
where
    F: FnMut(&str),
{
    const MAX_ATTEMPTS: u32 = 3;
    const BACKOFFS_SECS: &[u64] = &[2, 5];
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match run_command_streaming(python, args, cwd, &mut on_line) {
            Ok(()) => return Ok(()),
            Err(err) => {
                if attempt < MAX_ATTEMPTS {
                    log::info!(
                        "pip install attempt {}/{} failed (will retry): {}",
                        attempt,
                        MAX_ATTEMPTS,
                        err
                    );
                } else {
                    log::warn!(
                        "pip install attempt {}/{} failed (final): {}",
                        attempt,
                        MAX_ATTEMPTS,
                        compact_pip_failure(&err)
                    );
                }
                last_err = Some(err);
                if attempt < MAX_ATTEMPTS {
                    let idx = (attempt as usize - 1).min(BACKOFFS_SECS.len() - 1);
                    std::thread::sleep(std::time::Duration::from_secs(BACKOFFS_SECS[idx]));
                }
            }
        }
    }
    Err(last_err.expect("at least one attempt was made"))
}

/// Returns true when an `anyhow::Error` from a `headroom <subcommand>` shell-out
/// looks like the venv is missing pinned dependencies — i.e. Python died with
/// `ModuleNotFoundError` or `ImportError` before the CLI could run. This is the
/// recovery signal for a partial install that left the receipt's
/// `requirementsLockSha256` stamped but the venv contents incomplete.
fn looks_like_corrupt_venv_error(err: &anyhow::Error) -> bool {
    let Some(failure) = err.downcast_ref::<CommandFailure>() else {
        return false;
    };
    let stderr = failure.stderr.as_str();
    stderr.contains("ModuleNotFoundError") || stderr.contains("ImportError")
}

/// Structured error emitted when the headroom proxy subprocess fails to open
/// its port. Capture sites downcast to pull the log tail into Sentry `extra`
/// fields, which are not subject to the 8KB message cap.
#[derive(Debug)]
pub struct HeadroomStartupFailure {
    pub program: String,
    pub args: Vec<String>,
    pub log_path: String,
    pub log_tail: String,
    pub reason: String,
}

impl std::fmt::Display for HeadroomStartupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} (log: {}){}",
            self.program,
            self.args.join(" "),
            self.reason,
            self.log_path,
            if self.log_tail.is_empty() {
                String::new()
            } else {
                format!("\n--- log tail ---\n{}\n--- end log ---", self.log_tail)
            }
        )
    }
}

impl std::error::Error for HeadroomStartupFailure {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use chrono::Local;
    use serde_json::{json, Value};

    use super::{
        bootstrap_requirements_lock_for_target, classify_kompress_prefetch_failure,
        extract_required_pydantic_core_version, format_all_foreign_bail,
        format_already_running_bail, headroom_entrypoint_startup_args,
        headroom_python_startup_args, looks_like_corrupt_venv_error, parse_major_minor_patch,
        parse_pid_from_lsof_detail, path_with_binary_dir, probe_backend_readyz_ok,
        proxy_argv_contains_expected_flags_for, receipt_requires_atomic_rebuild,
        reclaim_orphan_proxy, redact_sensitive, repair_console_script_interpreter,
        requirements_lock_sha, rtk_distribution_artifact, run_command, sanitize_log_variant,
        sha256_bytes, summarize_kompress_prefetch_failure, verify_sha256_file, wait_for_port_free,
        CommandFailure, HeadroomRelease, ManagedRuntime, PipOutputCapture, ToolManager,
        UpgradeOutcome, ATOMIC_REBUILD_FLOOR_VERSION, CAVEMAN_LEVEL_COMPACT_CHINESE,
        CAVEMAN_LEVEL_SCOPED, RTK_VERSION,
    };
    use crate::backend_port;
    use crate::models::SavingsMode;
    use crate::port_conflict;
    use std::net::TcpListener;

    struct AppStorageEnvGuard {
        prev_xdg: Option<std::ffi::OsString>,
        _temp_dir: std::path::PathBuf,
    }

    impl AppStorageEnvGuard {
        fn new() -> Self {
            let prev_xdg = std::env::var_os("XDG_DATA_HOME");
            let temp_dir = unique_temp_dir("app-storage");
            std::env::set_var("XDG_DATA_HOME", &temp_dir);
            Self {
                prev_xdg,
                _temp_dir: temp_dir,
            }
        }
    }

    impl Drop for AppStorageEnvGuard {
        fn drop(&mut self) {
            match self.prev_xdg.take() {
                Some(value) => std::env::set_var("XDG_DATA_HOME", value),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }
    }

    #[test]
    fn path_with_binary_dir_prepends_parent() {
        let path =
            path_with_binary_dir(&PathBuf::from("/Users/x/.nvm/versions/node/v22/bin/codex"));
        assert!(path.starts_with("/Users/x/.nvm/versions/node/v22/bin:"));
        // A bare binary name has no usable parent; PATH is left unchanged.
        let existing = std::env::var("PATH").unwrap_or_default();
        assert_eq!(path_with_binary_dir(&PathBuf::from("codex")), existing);
    }

    #[test]
    fn repair_console_script_interpreter_rewrites_legacy_python_path() {
        let root = unique_temp_dir("repair-console-script");
        let bin = root.join("venv").join("bin");
        fs::create_dir_all(&bin).unwrap();
        let entrypoint = bin.join("headroom");
        let python = bin.join("python3");
        fs::write(&python, "#!/bin/sh\n").unwrap();
        fs::write(
            &entrypoint,
            "#!/bin/sh\n'''exec' \"/Users/t/Library/Application Support/Headroom/headroom/runtime/venv/bin/python3\" \"$0\" \"$@\"\n' '''\nimport sys\n",
        )
        .unwrap();

        repair_console_script_interpreter(&entrypoint, &python).unwrap();

        let updated = fs::read_to_string(&entrypoint).unwrap();
        assert!(updated.contains(&format!("'''exec' \"{}\"", python.display())));
        assert!(!updated.contains("Application Support/Headroom/headroom/runtime/venv/bin/python3"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn classify_kompress_prefetch_failure_buckets_known_causes() {
        assert_eq!(classify_kompress_prefetch_failure(""), "no output");
        assert_eq!(
            classify_kompress_prefetch_failure("python3 abort trap: 6 (SIGABRT)"),
            "native abort"
        );
        assert_eq!(
            classify_kompress_prefetch_failure("OSError: [Errno 28] No space left on device"),
            "disk full"
        );
        assert_eq!(
            classify_kompress_prefetch_failure(
                "requests.exceptions.ConnectionError: Max retries exceeded with url"
            ),
            "network"
        );
        assert_eq!(
            classify_kompress_prefetch_failure("PermissionError: [Errno 13] Permission denied"),
            "permission denied"
        );
        assert_eq!(
            classify_kompress_prefetch_failure("ValueError: something unexpected"),
            "other"
        );
    }

    #[test]
    fn summarize_kompress_prefetch_failure_uses_last_meaningful_line() {
        let dir = std::env::temp_dir().join(format!(
            "kompress-prefetch-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let log = dir.join("kompress-prefetch.log");
        fs::write(
            &log,
            "Downloading model...\nTraceback (most recent call last):\n  File x\nConnectionError: Max retries exceeded\n\n",
        )
        .unwrap();

        let cause = summarize_kompress_prefetch_failure(&log);
        assert_eq!(cause, "[network] ConnectionError: Max retries exceeded");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn summarize_kompress_prefetch_failure_handles_missing_log() {
        let cause = summarize_kompress_prefetch_failure(&PathBuf::from("/no/such/prefetch.log"));
        assert_eq!(cause, "[no output] (no output in kompress-prefetch.log)");
    }

    #[test]
    fn redact_sensitive_strips_anthropic_keys() {
        let line = "POST /v1/messages x-api-key: sk-ant-api03-AbCdEf-12_34 done";
        let out = redact_sensitive(line);
        assert!(!out.contains("sk-ant-"), "leak: {out}");
        assert!(out.contains("[REDACTED]"));
        assert!(out.contains("done"));
    }

    #[test]
    fn redact_sensitive_strips_bearer_tokens() {
        let line = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload.sig trailing";
        let out = redact_sensitive(line);
        assert!(!out.contains("eyJhbGciOiJIUzI1NiJ9"), "leak: {out}");
        assert!(out.contains("[REDACTED]"));
        assert!(out.contains("trailing"));
    }

    #[test]
    fn redact_sensitive_passes_through_clean_lines() {
        let line = "2026-05-03T20:31:34Z proxy started on 127.0.0.1:6767";
        assert_eq!(redact_sensitive(line), line);
    }

    #[test]
    fn redact_sensitive_ignores_short_bearer_word() {
        // "Bearer" followed by something too short to be a real token shouldn't
        // be redacted — we don't want to nuke unrelated prose.
        let line = "the Bearer of the message is fine";
        assert_eq!(redact_sensitive(line), line);
    }

    fn cmd_failure_with_stderr(stderr: &str) -> anyhow::Error {
        anyhow::Error::new(CommandFailure {
            program: "/runtime/venv/bin/headroom".into(),
            args: vec!["mcp".into(), "install".into()],
            stdout: String::new(),
            stderr: stderr.into(),
            exit_code: Some(1),
            signal: None,
        })
    }

    #[test]
    fn looks_like_corrupt_venv_error_matches_module_not_found() {
        let err = cmd_failure_with_stderr(
            "Traceback (most recent call last):\n\
             ...\n\
             ModuleNotFoundError: No module named 'opentelemetry'\n",
        );
        assert!(looks_like_corrupt_venv_error(&err));
    }

    #[test]
    fn looks_like_corrupt_venv_error_matches_import_error() {
        let err = cmd_failure_with_stderr(
            "Traceback (most recent call last):\n\
             ImportError: cannot import name 'X' from partially initialized module 'Y'\n",
        );
        assert!(looks_like_corrupt_venv_error(&err));
    }

    #[test]
    fn looks_like_corrupt_venv_error_ignores_unrelated_failures() {
        let err = cmd_failure_with_stderr("error: invalid --proxy-url\n");
        assert!(!looks_like_corrupt_venv_error(&err));
    }

    #[test]
    fn looks_like_corrupt_venv_error_ignores_non_command_errors() {
        let err = anyhow::anyhow!("some other failure with ModuleNotFoundError in the message");
        // Only CommandFailure errors carry the structured stderr we trust as a
        // corrupt-venv signal — a bare anyhow message could be anything.
        assert!(!looks_like_corrupt_venv_error(&err));
    }

    #[test]
    fn looks_like_corrupt_venv_error_survives_anyhow_context() {
        use anyhow::Context as _;
        let err = cmd_failure_with_stderr("ModuleNotFoundError: No module named 'opentelemetry'\n");
        let wrapped = Err::<(), _>(err)
            .context("configuring Headroom MCP integration")
            .unwrap_err();
        assert!(looks_like_corrupt_venv_error(&wrapped));
    }

    #[test]
    fn requirements_lock_sha_ignores_comments_and_blank_lines() {
        let a = "# header one\n\nabsl-py==2.4.0\naiohttp==3.13.5\n";
        let b = "# header two — different\naiohttp==3.13.5\nabsl-py==2.4.0\n";
        let c = "absl-py==2.4.0\naiohttp==3.13.6\n";
        // Same pinned versions, different comments/whitespace → same hash.
        assert_eq!(requirements_lock_sha(a), requirements_lock_sha(a));
        // Order still matters (pip resolution order), so (a) and (b) differ.
        assert_ne!(requirements_lock_sha(a), requirements_lock_sha(b));
        // A real version bump changes the hash.
        assert_ne!(requirements_lock_sha(a), requirements_lock_sha(c));
        // Adding/removing a comment or blank line must not change the hash.
        let a_more_comments =
            "# header one\n# extra note\n\n\nabsl-py==2.4.0\n# inline\naiohttp==3.13.5\n";
        assert_eq!(
            requirements_lock_sha(a),
            requirements_lock_sha(a_more_comments)
        );
    }

    #[test]
    fn run_command_failure_carries_structured_output() {
        let tmp = std::env::temp_dir();
        let err = run_command(
            std::path::Path::new("/bin/sh"),
            &["-c", "echo hi-out; echo hi-err 1>&2; exit 7"],
            &tmp,
        )
        .expect_err("command should have failed");

        let failure = err
            .chain()
            .find_map(|e| e.downcast_ref::<CommandFailure>())
            .expect("CommandFailure should be in the error chain");

        assert_eq!(failure.exit_code, Some(7));
        assert!(
            failure.stdout.contains("hi-out"),
            "stdout: {}",
            failure.stdout
        );
        assert!(
            failure.stderr.contains("hi-err"),
            "stderr: {}",
            failure.stderr
        );
        assert_eq!(failure.program, "/bin/sh");
    }

    #[test]
    fn managed_python_paths_live_inside_headroom_root() {
        let root = std::env::temp_dir().join("headroom-tool-manager-test");
        let runtime = ManagedRuntime::bootstrap_root(&root);

        assert!(runtime.managed_python().starts_with(&runtime.root_dir));
        assert!(runtime.standalone_python().starts_with(&runtime.root_dir));
        assert!(runtime.managed_pip().starts_with(&runtime.root_dir));
        assert!(runtime.bin_dir.starts_with(&runtime.root_dir));
    }

    #[test]
    fn kompress_marker_scan_treats_lazy_load_as_enabled() {
        let (root, runtime, manager) = seed_test_runtime("kompress-marker");
        fs::create_dir_all(runtime.logs_dir()).expect("logs dir");
        let enabled: &[&str] = &[
            "kompress: enabled",
            "kompress onnx loaded",
            "kompress pytorch loaded",
        ];
        let disabled: &[&str] = &["kompress: not installed", "kompress: disabled"];

        // Cold-cache startup logs "not installed"; a later first-use lazy load
        // logs "Kompress ONNX loaded". The most-recent marker (lazy load) wins,
        // so the desktop reports enabled without a restart.
        let log = runtime.logs_dir().join("kompress-lazy.log");
        fs::write(
            &log,
            "2026-06-12 10:00:00 - headroom.proxy - INFO - Kompress: not installed (pip install headroom-ai[ml])\n\
             2026-06-12 10:05:00 - headroom.proxy - INFO - Kompress ONNX loaded: chopratejas/kompress-v2-base backend=onnx\n",
        )
        .expect("write lazy log");
        assert_eq!(
            manager.scan_file_for_marker_state_cached("k-lazy", &log, enabled, disabled),
            Some(true),
            "a lazy-load line after a not-installed line should report enabled"
        );

        // A pure cold-cache log (no lazy load yet) still reports disabled.
        let log2 = runtime.logs_dir().join("kompress-cold.log");
        fs::write(
            &log2,
            "2026-06-12 10:00:00 - headroom.proxy - INFO - Kompress: not installed (pip install headroom-ai[ml])\n",
        )
        .expect("write cold log");
        assert_eq!(
            manager.scan_file_for_marker_state_cached("k-cold", &log2, enabled, disabled),
            Some(false),
            "a not-installed line with no later load should report disabled"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_distribution_artifact_is_pinned_to_current_release_with_checksum() {
        let artifact = rtk_distribution_artifact().expect("supported RTK target");

        assert!(artifact.url.contains(&format!("/v{RTK_VERSION}/")));
        assert!(
            artifact.sha256.is_some(),
            "RTK artifact checksum should be pinned"
        );
    }

    #[test]
    fn tool_manifest_exposes_platform_rtk_checksum() {
        let root = std::env::temp_dir().join("headroom-tool-manager-manifest-test");
        let runtime = ManagedRuntime::bootstrap_root(&root);
        let manager = ToolManager::new(runtime);

        let rtk = manager
            .list_tools()
            .into_iter()
            .find(|tool| tool.id == "rtk")
            .expect("rtk manifest should exist");
        assert_eq!(rtk.version, RTK_VERSION);
        assert!(rtk.checksum.is_some(), "RTK checksum should be exposed");
    }

    #[test]
    fn caveman_compact_chinese_level_round_trips() {
        let (_root, _runtime, manager) = seed_test_runtime("caveman-compact-chinese");

        manager.install_caveman().expect("install caveman");
        manager
            .set_caveman_level(CAVEMAN_LEVEL_COMPACT_CHINESE)
            .expect("set compact chinese");

        let caveman = manager
            .list_tools()
            .into_iter()
            .find(|tool| tool.id == "caveman")
            .expect("caveman manifest should exist");
        assert_eq!(manager.caveman_level(), CAVEMAN_LEVEL_COMPACT_CHINESE);
        assert_eq!(
            caveman.metadata.as_ref().and_then(|meta| meta.get("level")),
            Some(&json!(CAVEMAN_LEVEL_COMPACT_CHINESE))
        );
    }

    #[test]
    fn caveman_unknown_level_falls_back_to_scoped() {
        let (_root, _runtime, manager) = seed_test_runtime("caveman-unknown-level");

        manager.install_caveman().expect("install caveman");
        manager
            .set_caveman_level("translate_everything")
            .expect("set unknown level");

        assert_eq!(manager.caveman_level(), CAVEMAN_LEVEL_SCOPED);
    }

    #[test]
    fn rtk_installed_requires_binary_and_receipt() {
        let (root, runtime, manager) = seed_test_runtime("rtk-installed");

        assert!(!manager.rtk_installed(), "no binary or receipt yet");

        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        assert!(
            !manager.rtk_installed(),
            "binary alone should not count as installed"
        );

        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");
        assert!(manager.rtk_installed(), "binary + receipt should count");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn installed_rtk_version_reads_receipt() {
        let (root, runtime, manager) = seed_test_runtime("rtk-version");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": "0.37.2-test" }))
            .expect("rtk receipt");

        assert_eq!(
            manager.installed_rtk_version().as_deref(),
            Some("0.37.2-test")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_needs_install_true_when_binary_missing() {
        let (root, _runtime, manager) = seed_test_runtime("rtk-needs-install-missing");
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");
        assert!(manager.rtk_needs_install());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_needs_install_true_when_version_stale() {
        let (root, runtime, manager) = seed_test_runtime("rtk-needs-install-stale");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": "0.0.1-old" }))
            .expect("rtk receipt");
        assert!(manager.rtk_needs_install());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_needs_install_false_when_current() {
        let (root, runtime, manager) = seed_test_runtime("rtk-needs-install-current");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");
        assert!(!manager.rtk_needs_install());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ensure_rtk_current_is_noop_when_already_current() {
        let (root, runtime, manager) = seed_test_runtime("rtk-ensure-current-noop");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nexit 0\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");
        let did_work = manager.ensure_rtk_current().expect("ensure_rtk_current");
        assert!(!did_work, "should skip install when already current");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ensure_rtk_current_is_noop_when_binary_absent() {
        // RTK is opt-in: a missing binary means uninstalled/never-installed, so
        // launch must not create a fresh install.
        let (root, _runtime, manager) = seed_test_runtime("rtk-ensure-current-absent");
        let did_work = manager.ensure_rtk_current().expect("ensure_rtk_current");
        assert!(!did_work, "should not install rtk when binary is absent");
        assert!(!manager.rtk_installed(), "rtk must remain uninstalled");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn read_rtk_activity_reports_not_installed_when_missing() {
        let (root, _runtime, manager) = seed_test_runtime("rtk-not-installed");

        let lines = manager
            .read_rtk_activity(10)
            .expect("not-installed fallback");
        assert_eq!(lines, vec!["RTK is not installed yet.".to_string()]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn read_rtk_activity_returns_last_lines_from_session_output() {
        let (root, runtime, manager) = seed_test_runtime("rtk-activity");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nif [ \"$1\" = \"session\" ]; then\n  printf 'line-1\\nline-2\\nline-3\\nline-4\\n';\n  exit 0\nfi\nexit 9\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");

        let lines = manager.read_rtk_activity(2).expect("session output");
        assert_eq!(lines, vec!["line-3".to_string(), "line-4".to_string()]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn read_rtk_activity_surfaces_session_failures() {
        let (root, runtime, manager) = seed_test_runtime("rtk-activity-fail");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nif [ \"$1\" = \"session\" ]; then\n  echo 'session stdout';\n  echo 'session stderr' 1>&2;\n  exit 7\nfi\nexit 9\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");

        let err = manager
            .read_rtk_activity(10)
            .expect_err("failing session should surface an error");
        let msg = err.to_string();
        assert!(msg.contains("session stdout"), "stdout preserved: {msg}");
        assert!(msg.contains("session stderr"), "stderr preserved: {msg}");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_today_stats_returns_matching_daily_row() {
        let (root, runtime, manager) = seed_test_runtime("rtk-today");
        let today = Local::now().date_naive().to_string();
        let script = format!(
            "#!/usr/bin/env bash\nif [ \"$1\" = \"gain\" ]; then\n  cat <<'EOF'\n{{\"daily\":[{{\"date\":\"1999-01-01\",\"commands\":1,\"saved_tokens\":2}},{{\"date\":\"{today}\",\"commands\":7,\"saved_tokens\":1234}}]}}\nEOF\n  exit 0\nfi\nexit 9\n",
        );
        write_executable(&runtime.bin_dir.join("rtk"), &script);
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");

        let stats = manager.rtk_today_stats().expect("today stats");
        assert_eq!(stats.date, today);
        assert_eq!(stats.commands, 7);
        assert_eq!(stats.saved_tokens, 1234);
        assert_eq!(stats.input_tokens, 0);
        assert_eq!(stats.output_tokens, 0);
        assert_eq!(stats.savings_pct, None);
        assert_eq!(stats.total_time_ms, 0);
        assert_eq!(stats.avg_time_ms, None);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_daily_stats_returns_all_daily_rows() {
        let (root, runtime, manager) = seed_test_runtime("rtk-daily");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nif [ \"$1\" = \"gain\" ]; then\n  echo '{\"daily\":[{\"date\":\"2026-06-24\",\"commands\":2,\"input_tokens\":900,\"output_tokens\":600,\"saved_tokens\":300,\"savings_pct\":33.3,\"total_time_ms\":1200,\"avg_time_ms\":600},{\"date\":\"2026-06-25\",\"commands\":7,\"input_tokens\":2000,\"output_tokens\":766,\"saved_tokens\":1234,\"savings_pct\":61.7,\"total_time_ms\":3500,\"avg_time_ms\":500}]}';\n  exit 0\nfi\nexit 9\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");

        let stats = manager.rtk_daily_stats().expect("daily stats");
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].date, "2026-06-24");
        assert_eq!(stats[0].commands, 2);
        assert_eq!(stats[0].input_tokens, 900);
        assert_eq!(stats[0].output_tokens, 600);
        assert_eq!(stats[0].saved_tokens, 300);
        assert_eq!(stats[0].savings_pct, Some(33.3));
        assert_eq!(stats[0].total_time_ms, 1200);
        assert_eq!(stats[0].avg_time_ms, Some(600));
        assert_eq!(stats[1].date, "2026-06-25");
        assert_eq!(stats[1].commands, 7);
        assert_eq!(stats[1].input_tokens, 2000);
        assert_eq!(stats[1].output_tokens, 766);
        assert_eq!(stats[1].saved_tokens, 1234);
        assert_eq!(stats[1].savings_pct, Some(61.7));
        assert_eq!(stats[1].total_time_ms, 3500);
        assert_eq!(stats[1].avg_time_ms, Some(500));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_gain_summary_preserves_lifetime_token_and_timing_totals() {
        let (root, runtime, manager) = seed_test_runtime("rtk-summary");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nif [ \"$1\" = \"gain\" ]; then\n  echo '{\"summary\":{\"total_commands\":12,\"total_input\":1500,\"total_output\":600,\"total_saved\":900,\"avg_savings_pct\":60.0,\"total_time_ms\":3500,\"avg_time_ms\":292},\"daily\":[]}';\n  exit 0\nfi\nexit 9\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");

        let summary = manager.rtk_gain_summary().expect("summary");
        assert_eq!(summary.total_commands, 12);
        assert_eq!(summary.total_input, 1500);
        assert_eq!(summary.total_output, 600);
        assert_eq!(summary.total_saved, 900);
        assert_eq!(summary.avg_savings_pct, 60.0);
        assert_eq!(summary.total_time_ms, 3500);
        assert_eq!(summary.avg_time_ms, Some(292));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_today_stats_returns_none_when_today_absent() {
        let (root, runtime, manager) = seed_test_runtime("rtk-today-missing");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nif [ \"$1\" = \"gain\" ]; then\n  echo '{\"daily\":[{\"date\":\"1999-01-01\",\"commands\":1,\"saved_tokens\":2}]}';\n  exit 0\nfi\nexit 9\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");

        assert!(manager.rtk_today_stats().is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_today_stats_returns_none_on_command_failure() {
        let (root, runtime, manager) = seed_test_runtime("rtk-today-fail");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nif [ \"$1\" = \"gain\" ]; then\n  echo 'boom' 1>&2;\n  exit 4\nfi\nexit 9\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");

        assert!(manager.rtk_today_stats().is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rtk_today_stats_returns_none_on_invalid_json() {
        let (root, runtime, manager) = seed_test_runtime("rtk-today-invalid-json");
        write_executable(
            &runtime.bin_dir.join("rtk"),
            "#!/usr/bin/env bash\nif [ \"$1\" = \"gain\" ]; then\n  echo 'not-json';\n  exit 0\nfi\nexit 9\n",
        );
        manager
            .write_tool_receipt("rtk", serde_json::json!({ "version": RTK_VERSION }))
            .expect("rtk receipt");

        assert!(manager.rtk_today_stats().is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bootstrap_all_installs_into_temp_root_when_enabled() {
        if std::env::var("HEADROOM_RUN_NETWORK_TESTS").is_err() {
            return;
        }

        let root = std::env::temp_dir().join(format!("headroom-e2e-{}", uuid::Uuid::new_v4()));
        let runtime = ManagedRuntime::bootstrap_root(&root);
        let manager = ToolManager::new(runtime.clone());

        manager.bootstrap_all().expect("bootstrap succeeds");

        assert!(runtime.managed_python().exists());
        assert!(runtime.tools_dir.join("headroom.json").exists());
        assert!(runtime.bin_dir.join("rtk").exists());
    }

    #[test]
    fn proxy_argv_matches_when_all_expected_flags_present() {
        std::env::remove_var("HEADROOM_FULL_MESSAGE_LOGGING");
        let argv = "/usr/bin/nice -n 5 /Users/x/headroom proxy --port 6768 --log-messages \
                    --learn --no-memory-tools --no-memory-context --memory-db-path /tmp/m.db";
        assert!(proxy_argv_contains_expected_flags_for(
            argv,
            &SavingsMode::Balanced
        ));
    }

    #[test]
    fn proxy_argv_mismatch_when_core_flags_missing() {
        let argv = "/Users/x/headroom proxy --port 6768";
        assert!(!proxy_argv_contains_expected_flags_for(
            argv,
            &SavingsMode::Balanced
        ));
    }

    #[test]
    fn proxy_argv_requires_log_messages_only_during_explicit_opt_in() {
        std::env::set_var("HEADROOM_FULL_MESSAGE_LOGGING", "1");
        let argv = "headroom proxy --port 6768 --learn --no-memory-tools \
                    --no-memory-context --memory-db-path /tmp/m.db";
        assert!(!proxy_argv_contains_expected_flags_for(
            argv,
            &SavingsMode::Balanced
        ));
        std::env::remove_var("HEADROOM_FULL_MESSAGE_LOGGING");
    }

    #[test]
    fn proxy_argv_mismatch_when_learn_missing() {
        let argv = "headroom proxy --port 6768 --log-messages --no-memory-tools \
                    --no-memory-context --memory-db-path /tmp/m.db";
        assert!(!proxy_argv_contains_expected_flags_for(
            argv,
            &SavingsMode::Balanced
        ));
    }

    #[test]
    fn proxy_argv_match_does_not_get_fooled_by_negated_flag_substring() {
        // `--no-learn` contains `--learn` as a substring; whitespace tokenizing
        // ensures we don't false-positive on it.
        let argv = "headroom proxy --port 6768 --log-messages --no-learn \
                    --no-memory-tools --no-memory-context --memory-db-path /tmp/m.db";
        assert!(!proxy_argv_contains_expected_flags_for(
            argv,
            &SavingsMode::Balanced
        ));
    }

    #[test]
    fn proxy_argv_match_works_for_python_module_invocation() {
        let argv = "/Users/x/venv/bin/python3 -m headroom.proxy.server --port 6768 \
                    --no-http2 --log-messages --learn --no-memory-tools --no-memory-context \
                    --memory-db-path /tmp/m.db";
        assert!(proxy_argv_contains_expected_flags_for(
            argv,
            &SavingsMode::Balanced
        ));
    }

    #[test]
    fn aggressive_proxy_argv_requires_tool_result_interception() {
        let balanced_argv = "headroom proxy --port 6768 --log-messages --learn \
                             --no-memory-tools --no-memory-context --memory-db-path /tmp/m.db";
        let aggressive_argv = "headroom proxy --port 6768 --log-messages \
                               --intercept-tool-results --learn --no-memory-tools \
                               --no-memory-context --memory-db-path /tmp/m.db";

        assert!(!proxy_argv_contains_expected_flags_for(
            balanced_argv,
            &SavingsMode::Aggressive
        ));
        assert!(proxy_argv_contains_expected_flags_for(
            aggressive_argv,
            &SavingsMode::Aggressive
        ));
    }

    #[test]
    fn sanitize_log_variant_replaces_path_separators() {
        let raw = "proxy---memory-db-path-/Users/x/Library/Application Support/Headroom/memory.db";
        let cleaned = sanitize_log_variant(raw);
        assert!(
            !cleaned.contains('/'),
            "expected no slashes, got: {cleaned}"
        );
        assert!(!cleaned.contains('\\'));
        assert!(cleaned.contains("memory-db-path"));
    }

    #[test]
    fn sanitize_log_variant_truncates_long_input() {
        let raw = "a".repeat(500);
        let cleaned = sanitize_log_variant(&raw);
        assert_eq!(cleaned.len(), 80);
    }

    #[test]
    fn sanitize_log_variant_keeps_short_safe_input_unchanged() {
        let raw = "proxy---port-6768---log-messages---learn";
        let cleaned = sanitize_log_variant(raw);
        assert_eq!(cleaned, raw);
    }

    #[test]
    fn parse_pid_from_lsof_detail_extracts_numeric_pid() {
        assert_eq!(parse_pid_from_lsof_detail("rapportd pid 594"), Some(594));
        assert_eq!(
            parse_pid_from_lsof_detail("python3.12 pid 1073"),
            Some(1073)
        );
        assert_eq!(
            parse_pid_from_lsof_detail("Google Chrome Helper pid 4242"),
            Some(4242)
        );
    }

    #[test]
    fn parse_pid_from_lsof_detail_returns_none_for_unknown_or_malformed() {
        assert_eq!(parse_pid_from_lsof_detail("unknown process"), None);
        assert_eq!(parse_pid_from_lsof_detail(""), None);
        assert_eq!(
            parse_pid_from_lsof_detail("rapportd pid not-a-number"),
            None
        );
        // Missing the " pid " separator entirely.
        assert_eq!(parse_pid_from_lsof_detail("rapportd 594"), None);
    }

    /// Round-trip: the bail string produced by `format_all_foreign_bail` must
    /// be matched by `port_conflict::is_port_conflict` (so the persistent-
    /// conflict marker keeps tracking it) AND the occupant must be parseable
    /// by `port_conflict::parse_occupant` (so analytics/Sentry get the
    /// process name and pid).
    #[test]
    fn all_foreign_bail_round_trips_through_port_conflict_helpers() {
        let bail = format_all_foreign_bail(6768, "rapportd pid 594", (6769, 6790));
        assert!(
            port_conflict::is_port_conflict(&bail),
            "bail must match is_port_conflict so the marker keeps tracking; got: {bail}"
        );
        let (cmd, pid) = port_conflict::parse_occupant(&bail);
        assert_eq!(cmd.as_deref(), Some("rapportd"), "bail: {bail}");
        assert_eq!(pid, Some(594), "bail: {bail}");
    }

    /// Mirror round-trip for the unknown-occupant path (lsof returned nothing
    /// useful). `parse_occupant` should return None/None instead of inventing
    /// a fake cmd from "unknown process".
    #[test]
    fn all_foreign_bail_with_unknown_occupant_round_trips() {
        let bail = format_all_foreign_bail(6768, "unknown process", (6769, 6790));
        assert!(port_conflict::is_port_conflict(&bail));
        let (cmd, pid) = port_conflict::parse_occupant(&bail);
        assert!(cmd.is_none(), "got cmd: {cmd:?} from bail: {bail}");
        assert!(pid.is_none(), "got pid: {pid:?} from bail: {bail}");
    }

    /// The "stale headroom proxy holding the port" bail must NOT trigger
    /// the foreign-process port-conflict path — those are separate
    /// fingerprints in Sentry. Verifies the boundary stays intact.
    #[test]
    fn already_running_bail_is_not_classified_as_foreign_conflict() {
        let bail = format_already_running_bail(6768);
        assert!(
            !port_conflict::is_port_conflict(&bail),
            "stale-proxy bail must not match foreign-port classifier; got: {bail}"
        );
        // But the lib.rs port-conflict-failure classifier (which fingerprints
        // both shapes the same way) still catches it via its second condition.
        assert!(crate::is_port_conflict_failure(&bail));
    }

    #[test]
    fn wait_for_port_free_detects_release() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(
            !wait_for_port_free(port, Duration::from_millis(200)),
            "port held by a live listener must not report free"
        );
        drop(listener);
        assert!(
            wait_for_port_free(port, Duration::from_secs(2)),
            "port must report free shortly after the listener is dropped"
        );
    }

    /// Reproduces the upgrade-rollback scenario from the Sentry report: a
    /// *healthy* orphaned proxy (answers /readyz) squatting on the backend
    /// port. Normal launch (`force_unhealthy_too=false`) must leave it alone
    /// and bail; an upgrade boot validation (`true`) must reclaim it anyway so
    /// the new venv can bind. Ignored by default — spawns a child process,
    /// binds a port, and kills the process. Run locally:
    /// `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored reclaim_orphan`
    #[test]
    #[ignore]
    fn reclaim_orphan_proxy_respects_upgrade_override() {
        let port = {
            let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            l.local_addr().unwrap().port()
        };

        // Stand-in for a live old-version proxy: answers 200 on every path so
        // `/readyz` reads healthy. argv is `python3 -c ...`, which deliberately
        // does NOT match `stop_headroom`'s pattern-kill — exactly the orphan
        // that survives into the spawn pre-flight.
        let script = r#"
import http.server, socketserver, sys
class S(socketserver.TCPServer):
    allow_reuse_address = True
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200); self.end_headers(); self.wfile.write(b'ok')
    def log_message(self, *a):
        pass
S(('127.0.0.1', int(sys.argv[1])), H).serve_forever()
"#;

        let mut child = std::process::Command::new("/usr/bin/python3")
            .arg("-c")
            .arg(script)
            .arg(port.to_string())
            .spawn()
            .expect("spawn stand-in proxy");

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !probe_backend_readyz_ok(port) {
            assert!(
                std::time::Instant::now() < deadline,
                "stand-in proxy never became healthy on port {port}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }

        // Normal launch: a healthy occupant is left alone, reclaim bails.
        assert!(
            reclaim_orphan_proxy(port, false).is_err(),
            "force=false must bail on a healthy occupant"
        );
        assert!(
            probe_backend_readyz_ok(port),
            "force=false must NOT kill the healthy occupant"
        );

        // Upgrade validation: the healthy old proxy is reclaimed regardless.
        assert!(
            reclaim_orphan_proxy(port, true).is_ok(),
            "force=true must reclaim even a healthy occupant"
        );
        assert!(
            wait_for_port_free(port, Duration::from_secs(3)),
            "force=true must free the port"
        );

        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn probe_backend_readyz_ok_false_when_nothing_listening() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert!(
            !probe_backend_readyz_ok(port),
            "no server listening means /readyz is not healthy"
        );
    }

    #[test]
    fn extract_required_pydantic_core_version_pulls_pin_from_systemerror() {
        let log = "Traceback (most recent call last):\n  File \"<frozen runpy>\", line 189, in _run_module_as_main\n  ...\nSystemError: The installed pydantic-core version (2.46.3) is incompatible with the current pydantic version, which requires 2.41.5. If you encounter this error, make sure that you haven't upgraded pydantic-core manually.\n";
        assert_eq!(
            extract_required_pydantic_core_version(log),
            Some("2.41.5".into())
        );
    }

    #[test]
    fn extract_required_pydantic_core_version_returns_none_on_unrelated_traceback() {
        let log = "Traceback (most recent call last):\n  File \"x.py\", line 1, in <module>\nImportError: No module named 'foo'\n";
        assert!(extract_required_pydantic_core_version(log).is_none());
    }

    #[test]
    fn extract_required_pydantic_core_version_returns_none_when_marker_missing_version() {
        // Future-proof: if pydantic ever changes the message format and there's
        // no version after "which requires ", we must not return an empty pin.
        let log = "pydantic-core mismatch: which requires nothing useful here";
        assert!(extract_required_pydantic_core_version(log).is_none());
    }

    #[test]
    fn managed_headroom_startup_uses_supported_proxy_args() {
        std::env::set_var("HEADROOM_FULL_MESSAGE_LOGGING", "0");
        let _app_storage = AppStorageEnvGuard::new();
        crate::message_logging::save_settings(&crate::models::MessageLoggingSettings::default())
            .expect("disable message logging for startup test");
        backend_port::reset_for_tests();
        let default_port = backend_port::DEFAULT_BACKEND_PORT.to_string();
        let entrypoint_args = headroom_entrypoint_startup_args(&SavingsMode::Balanced);
        assert!(entrypoint_args.starts_with(&[
            "proxy".to_string(),
            "--port".to_string(),
            default_port.clone(),
        ]));
        assert!(!entrypoint_args.contains(&"--log-messages".to_string()));
        assert!(entrypoint_args.contains(&"--learn".to_string()));
        assert!(entrypoint_args.contains(&"--no-memory-tools".to_string()));
        assert!(entrypoint_args.contains(&"--no-memory-context".to_string()));
        assert!(entrypoint_args.contains(&"--memory-db-path".to_string()));

        let python_args = headroom_python_startup_args();
        assert_eq!(
            python_args,
            vec![
                "-m".to_string(),
                "headroom.proxy.server".to_string(),
                "--port".to_string(),
                default_port,
                "--no-http2".to_string(),
            ]
        );
        // The python -m fallback must not pass learn flags; argparse on
        // headroom.proxy.server doesn't define them and would exit 2.
        assert!(!python_args.contains(&"--learn".to_string()));
        assert!(!python_args.contains(&"--no-memory-tools".to_string()));
        assert!(!python_args.contains(&"--no-memory-context".to_string()));
        assert!(!python_args.contains(&"--memory-db-path".to_string()));

        backend_port::reset_for_tests();
    }

    #[test]
    fn full_message_logging_startup_args_require_explicit_opt_in() {
        std::env::set_var("HEADROOM_FULL_MESSAGE_LOGGING", "1");
        let entrypoint_args = headroom_entrypoint_startup_args(&SavingsMode::Balanced);
        let python_args = headroom_python_startup_args();
        std::env::remove_var("HEADROOM_FULL_MESSAGE_LOGGING");

        assert!(entrypoint_args.contains(&"--log-messages".to_string()));
        assert!(python_args.contains(&"--log-messages".to_string()));
    }

    #[test]
    fn aggressive_startup_args_enable_tool_result_interception() {
        let balanced_args = headroom_entrypoint_startup_args(&SavingsMode::Balanced);
        let aggressive_args = headroom_entrypoint_startup_args(&SavingsMode::Aggressive);

        assert!(!balanced_args.contains(&"--intercept-tool-results".to_string()));
        assert!(aggressive_args.contains(&"--intercept-tool-results".to_string()));
    }

    /// Regression: `start_headroom_background` previously built `startup_variants`
    /// before pre-flight ran, so when fallback called `backend_port::set(6769)`
    /// the variants still spawned with `--port 6768` and both failed with
    /// EADDRINUSE. The arg helpers read the atomic at call time, so as long as
    /// the helpers are invoked AFTER fallback has updated the atomic, the
    /// chosen fallback port flows through.
    #[test]
    fn startup_args_reflect_fallback_port_set_after_default() {
        backend_port::reset_for_tests();
        backend_port::set(6770);

        let entrypoint_args = headroom_entrypoint_startup_args(&SavingsMode::Balanced);
        let port_idx = entrypoint_args
            .iter()
            .position(|a| a == "--port")
            .expect("entrypoint args contain --port");
        assert_eq!(entrypoint_args[port_idx + 1], "6770");

        let python_args = headroom_python_startup_args();
        let port_idx = python_args
            .iter()
            .position(|a| a == "--port")
            .expect("python args contain --port");
        assert_eq!(python_args[port_idx + 1], "6770");

        backend_port::reset_for_tests();
    }

    #[test]
    fn linux_bootstrap_requirements_skip_optional_memory_and_ml_packages() {
        let linux_requirements = bootstrap_requirements_lock_for_target("linux");

        assert!(linux_requirements.contains("ast-grep-cli=="));
        assert!(!linux_requirements.contains("hnswlib=="));
        assert!(linux_requirements.contains("opentelemetry-api=="));
        assert!(!linux_requirements.contains("torch=="));
        assert!(!linux_requirements.contains("sentence-transformers=="));
        assert!(linux_requirements.contains("mcp=="));
        assert!(linux_requirements.contains("onnxruntime=="));
        assert!(linux_requirements.contains("transformers=="));
    }

    #[test]
    fn verify_sha256_file_accepts_matching_content_and_rejects_mismatches() {
        let root = unique_temp_dir("headroom-sha256");
        fs::create_dir_all(&root).expect("create root");
        let artifact = root.join("artifact.bin");
        fs::write(&artifact, b"headroom").expect("write artifact");

        let checksum = sha256_bytes(b"headroom");
        verify_sha256_file(&artifact, &checksum).expect("matching checksum");

        let err = verify_sha256_file(&artifact, "not-the-right-checksum")
            .expect_err("mismatched checksum should fail");
        assert!(err.to_string().contains("checksum mismatch"));

        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    fn write_executable(path: &std::path::Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, body).expect("write script");
        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("chmod");
    }

    fn seed_test_runtime(prefix: &str) -> (PathBuf, ManagedRuntime, ToolManager) {
        let root = unique_temp_dir(prefix);
        let runtime = ManagedRuntime::bootstrap_root(&root);
        runtime.ensure_layout().expect("layout");
        fs::create_dir_all(&runtime.venv_dir).expect("venv dir");
        fs::write(runtime.venv_dir.join("marker"), b"live-v1").expect("marker");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            br#"{"version":"0.0.1"}"#,
        )
        .expect("receipt");
        let manager = ToolManager::new(runtime.clone());
        (root, runtime, manager)
    }

    struct TestHome {
        root: PathBuf,
        previous_home: Option<std::ffi::OsString>,
    }

    impl TestHome {
        fn new(prefix: &str) -> Self {
            let root = unique_temp_dir(prefix);
            fs::create_dir_all(&root).expect("create home");
            let previous_home = std::env::var_os("HOME");
            std::env::set_var("HOME", &root);
            Self {
                root,
                previous_home,
            }
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            match self.previous_home.take() {
                Some(home) => std::env::set_var("HOME", home),
                None => std::env::remove_var("HOME"),
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    #[serial_test::serial]
    fn install_repo_memory_mcp_writes_receipt_and_server() {
        let home = TestHome::new("repo-memory-install-home");
        let (root, runtime, manager) = seed_test_runtime("repo-memory-install");

        manager
            .install_repo_memory_mcp()
            .expect("install repo memory mcp");

        let receipt: Value = serde_json::from_slice(
            &fs::read(runtime.tools_dir.join("repo-memory.json")).expect("read receipt"),
        )
        .expect("parse receipt");
        assert_eq!(receipt["mcp"]["configured"], true);
        assert_eq!(receipt["mcp"]["readOnly"], true);
        assert_eq!(receipt["mcp"]["transport"], "stdio");
        assert!(receipt["mcp"]["command"]
            .as_str()
            .expect("receipt command")
            .contains("repo-intelligence.mjs"));
        assert!(receipt["mcp"]["descriptorPath"]
            .as_str()
            .expect("descriptor path")
            .ends_with("repo-memory.json"));
        let claude: Value =
            serde_json::from_slice(&fs::read(home.root.join(".claude.json")).expect("read claude"))
                .expect("parse claude");
        assert_eq!(
            claude["mcpServers"]["repo-memory"]["env"]["MAC_AI_SWITCHBOARD_REPO_MEMORY_READ_ONLY"],
            "1"
        );
        assert_eq!(manager.repo_memory_mcp_configured(), Some(true));
        assert!(manager.repo_memory_mcp_error().is_none());
        let service = manager
            .repo_memory_mcp_service_status()
            .expect("repo memory service status");
        assert!(service.managed_by_app);
        assert!(service.read_only);
        assert_eq!(service.transport, "stdio");
        assert!(service.command.contains("repo-intelligence.mjs"));
        assert!(service.descriptor_path.ends_with("repo-memory.json"));
        assert!(service.descriptor_present);
        assert!(service.script_path.ends_with("repo-intelligence.mjs"));
        assert!(service.script_present);
        assert!(service.node_available);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[serial_test::serial]
    fn repo_memory_mcp_config_detects_missing_server_separately_from_headroom() {
        let home = TestHome::new("repo-memory-missing-home");
        let (_root, runtime, manager) = seed_test_runtime("repo-memory-missing");
        fs::write(
            runtime.tools_dir.join("repo-memory.json"),
            br#"{"version":"1","mcp":{"configured":true,"serverName":"repo-memory"}}"#,
        )
        .expect("write receipt");
        fs::write(
            home.root.join(".claude.json"),
            br#"{"mcpServers":{"headroom":{"command":"headroom"}}}"#,
        )
        .expect("write claude json");

        assert_eq!(manager.repo_memory_mcp_configured(), Some(false));
        assert!(manager
            .repo_memory_mcp_error()
            .expect("repo memory error")
            .contains("repo-memory missing"));
    }

    #[test]
    fn tool_enabled_reads_receipt_flag_and_defaults_true() {
        let (_root, runtime, manager) = seed_test_runtime("tool-enabled");
        // No receipt -> default enabled.
        assert!(manager.tool_enabled("markitdown"));

        fs::write(
            runtime.tools_dir.join("markitdown.json"),
            br#"{"version":"0.1.6","enabled":false}"#,
        )
        .expect("receipt");
        assert!(!manager.tool_enabled("markitdown"));

        fs::write(
            runtime.tools_dir.join("markitdown.json"),
            br#"{"version":"0.1.6","enabled":true}"#,
        )
        .expect("receipt");
        assert!(manager.tool_enabled("markitdown"));
    }

    #[test]
    fn commit_headroom_upgrade_removes_backup() {
        let (root, runtime, manager) = seed_test_runtime("commit-backup");
        let backup = manager.venv_backup_dir();
        fs::create_dir_all(&backup).expect("backup dir");
        fs::write(backup.join("old-marker"), b"old").expect("old marker");
        fs::write(
            manager.headroom_receipt_backup_path(),
            br#"{"version":"0.0.0"}"#,
        )
        .expect("receipt backup");

        manager.commit_headroom_upgrade().expect("commit ok");

        assert!(!backup.exists(), "backup should be removed");
        assert!(!manager.headroom_receipt_backup_path().exists());
        assert!(
            runtime.venv_dir.join("marker").exists(),
            "live venv untouched"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn commit_headroom_upgrade_is_noop_without_backup() {
        let (root, _runtime, manager) = seed_test_runtime("commit-noop");
        manager.commit_headroom_upgrade().expect("noop ok");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn collect_native_extensions_walks_recursively_and_filters_by_extension() {
        let root = unique_temp_dir("collect-natives");
        let sp = root.join("site-packages");
        let pkg = sp.join("torch").join("_C");
        fs::create_dir_all(&pkg).expect("nested dirs");
        // Should be collected:
        fs::write(sp.join("mmh3.cpython-312-darwin.so"), b"").expect("so file");
        fs::write(pkg.join("libtorch_python.dylib"), b"").expect("dylib file");
        fs::write(
            sp.join("hnswlib").join("hnswlib.cpython-312-darwin.so"),
            b"",
        )
        .or_else(|_| {
            fs::create_dir_all(sp.join("hnswlib")).and_then(|_| {
                fs::write(
                    sp.join("hnswlib").join("hnswlib.cpython-312-darwin.so"),
                    b"",
                )
            })
        })
        .expect("nested so");
        // Should NOT be collected:
        fs::write(sp.join("README.md"), b"docs").expect("md file");
        fs::write(sp.join("module.py"), b"code").expect("py file");
        fs::write(pkg.join("_C.pyi"), b"stubs").expect("pyi file");

        let mut paths = Vec::new();
        super::collect_native_extensions(&sp, &mut paths).expect("walk ok");
        paths.sort();

        assert_eq!(paths.len(), 3, "expected 3 native files, got {paths:?}");
        assert!(paths
            .iter()
            .any(|p| p.ends_with("mmh3.cpython-312-darwin.so")));
        assert!(paths.iter().any(|p| p.ends_with("libtorch_python.dylib")));
        assert!(paths
            .iter()
            .any(|p| p.ends_with("hnswlib.cpython-312-darwin.so")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ad_hoc_sign_venv_natives_returns_zero_when_site_packages_missing() {
        // Fresh venv dir with no lib/python3.12/site-packages subtree — the
        // helper must silently return 0 rather than error. This is the path
        // exercised on every install before pip has populated the venv.
        let (root, _runtime, manager) = seed_test_runtime("codesign-no-sitepackages");
        assert_eq!(manager.ad_hoc_sign_venv_natives(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn commit_headroom_upgrade_removes_lock_backup() {
        let (root, _runtime, manager) = seed_test_runtime("commit-lock-backup");
        let lock_backup = manager.lock_backup_path();
        fs::write(&lock_backup, b"old-lock==1.0\n").expect("seed lock backup");

        manager.commit_headroom_upgrade().expect("commit ok");

        assert!(
            !lock_backup.exists(),
            "in-place lock backup should be removed on commit"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn write_and_read_in_place_marker_roundtrip() {
        let (root, _runtime, manager) = seed_test_runtime("marker-roundtrip");
        let lock_backup = manager.lock_backup_path();
        fs::write(&lock_backup, b"old-lock\n").expect("seed lock backup");

        manager
            .write_upgrade_marker("0.11.0", Some("0.10.8"), Some(&lock_backup))
            .expect("marker");

        let (prev, target, backup) = manager
            .read_in_place_marker()
            .expect("marker should parse as in-place");
        assert_eq!(prev, "0.10.8");
        assert_eq!(target, "0.11.0");
        assert_eq!(backup.as_deref(), Some(lock_backup.as_path()));

        // Atomic-rebuild markers (no previous_version) must not parse as in-place.
        manager
            .write_upgrade_marker("0.11.0", None, None)
            .expect("atomic marker");
        assert!(manager.read_in_place_marker().is_none());

        // Wheel-only shape (previous_version but no lock backup).
        manager
            .write_upgrade_marker("0.10.8", Some("0.10.7"), None)
            .expect("wheel-only marker");
        let (prev, target, backup) = manager.read_in_place_marker().expect("parse");
        assert_eq!(prev, "0.10.7");
        assert_eq!(target, "0.10.8");
        assert!(backup.is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn update_headroom_receipt_after_in_place_upgrade_rewrites_artifact() {
        // Guards the legacy-sha migration path. When LEGACY_REQUIREMENTS_LOCK_SHAS
        // is empty (current state after the 0.19.0 lock regen), there is no
        // legacy fixture to inject — re-enable when a future cosmetic-only lock
        // change re-populates the list.
        if super::LEGACY_REQUIREMENTS_LOCK_SHAS.is_empty() {
            return;
        }
        let (root, runtime, manager) = seed_test_runtime("receipt-rewrite");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "0.10.4",
                "artifact": {
                    "url": "https://old.example/headroom_ai-0.10.4.whl",
                    "sha256": "oldoldold",
                    "requirementsLockSha256": super::LEGACY_REQUIREMENTS_LOCK_SHAS[0],
                },
                "mcp": { "configured": false },
            }))
            .unwrap(),
        )
        .expect("seed receipt");

        let release = HeadroomRelease {
            version: "0.10.8".into(),
            wheel_url: "https://new.example/headroom_ai-0.10.8.whl".into(),
            sha256: "newnewnew".into(),
        };
        let mcp = serde_json::json!({ "configured": true, "proxyUrl": "http://127.0.0.1:6767" });
        manager
            .update_headroom_receipt_after_in_place_upgrade(&release, mcp.clone())
            .expect("receipt update ok");

        let receipt: serde_json::Value = serde_json::from_slice(
            &fs::read(runtime.tools_dir.join("headroom.json")).expect("read receipt"),
        )
        .expect("parse receipt");
        assert_eq!(receipt["version"], "0.10.8");
        assert_eq!(receipt["artifact"]["url"], release.wheel_url);
        assert_eq!(receipt["artifact"]["sha256"], release.sha256);
        assert_eq!(
            receipt["artifact"]["requirementsLockSha256"],
            requirements_lock_sha(super::bootstrap_requirements_lock()),
            "legacy sha must be migrated to the comment-insensitive form"
        );
        assert_eq!(receipt["mcp"], mcp);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rollback_headroom_upgrade_restores_from_backup() {
        // Simulate state after a boot-validation failure: a NEW venv is live
        // at venv_dir, the previous one is at venv_dir.backup, and the old
        // receipt is snapshotted.
        let (root, runtime, manager) = seed_test_runtime("rollback");
        let backup = manager.venv_backup_dir();

        // "Move" the current live venv to backup and create a fake "new" venv.
        fs::rename(&runtime.venv_dir, &backup).expect("move aside");
        fs::create_dir_all(&runtime.venv_dir).expect("new venv dir");
        fs::write(runtime.venv_dir.join("new-marker"), b"new").expect("new marker");
        fs::copy(
            runtime.tools_dir.join("headroom.json"),
            manager.headroom_receipt_backup_path(),
        )
        .expect("snapshot receipt");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            br#"{"version":"9.9.9"}"#,
        )
        .expect("new receipt");

        manager
            .rollback_headroom_upgrade()
            .expect("rollback succeeds");

        // The live venv should now be the original (contains "marker", not "new-marker").
        assert!(
            runtime.venv_dir.join("marker").exists(),
            "restored marker present"
        );
        assert!(
            !runtime.venv_dir.join("new-marker").exists(),
            "new venv wiped"
        );
        assert!(!backup.exists(), "backup consumed");
        let receipt = fs::read(runtime.tools_dir.join("headroom.json")).expect("receipt");
        assert!(
            String::from_utf8_lossy(&receipt).contains("0.0.1"),
            "receipt restored to previous: {}",
            String::from_utf8_lossy(&receipt)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rollback_headroom_upgrade_is_noop_without_backup() {
        let (root, _runtime, manager) = seed_test_runtime("rollback-noop");
        manager.rollback_headroom_upgrade().expect("noop ok");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recover_from_interrupted_upgrade_restores_backup_as_live() {
        // Simulate an interrupted upgrade: marker present, venv.backup has
        // the real old venv, venv has some partial new content.
        let (root, runtime, manager) = seed_test_runtime("interrupted");
        let backup = manager.venv_backup_dir();

        // Move original venv aside (as atomic_upgrade would).
        fs::rename(&runtime.venv_dir, &backup).expect("move aside");
        // Simulate a partial new venv left by an interrupted pip install.
        fs::create_dir_all(&runtime.venv_dir).expect("partial venv");
        fs::write(runtime.venv_dir.join("partial-marker"), b"interrupted").expect("partial");
        // Marker file and receipt backup (written by atomic_upgrade).
        manager
            .write_upgrade_marker("0.8.2", None, None)
            .expect("marker");
        fs::copy(
            runtime.tools_dir.join("headroom.json"),
            manager.headroom_receipt_backup_path(),
        )
        .expect("receipt snapshot");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            br#"{"version":"9.9.9-partial"}"#,
        )
        .expect("new receipt");

        let recovered = manager.recover_from_interrupted_upgrade();
        assert!(recovered, "recovery should fire");

        // The live venv should be the restored original.
        assert!(
            runtime.venv_dir.join("marker").exists(),
            "original restored"
        );
        assert!(
            !runtime.venv_dir.join("partial-marker").exists(),
            "partial new venv discarded"
        );
        assert!(!backup.exists(), "backup consumed");
        assert!(
            !manager.upgrade_marker_path().exists(),
            "marker cleared after recovery"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recover_from_interrupted_upgrade_is_noop_without_marker() {
        let (root, _runtime, manager) = seed_test_runtime("interrupted-noop");
        assert!(!manager.recover_from_interrupted_upgrade());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recover_from_interrupted_upgrade_handles_wheel_only_marker() {
        // In-place marker without a lock backup (wheel-only interrupted
        // upgrade). The pip reinstall inside recovery fails harmlessly without
        // a real python; we only assert the file-manipulation side: receipt
        // restored, marker cleared.
        let (root, runtime, manager) = seed_test_runtime("recover-wheel-only");
        manager
            .write_upgrade_marker("0.10.8", Some("0.10.7"), None)
            .expect("marker");
        fs::write(
            manager.headroom_receipt_backup_path(),
            br#"{"version":"0.10.7"}"#,
        )
        .expect("receipt snapshot");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            br#"{"version":"0.10.8-partial"}"#,
        )
        .expect("partial receipt");

        assert!(manager.recover_from_interrupted_upgrade());

        assert!(
            !manager.upgrade_marker_path().exists(),
            "marker cleared after recovery"
        );
        assert!(
            !manager.headroom_receipt_backup_path().exists(),
            "receipt backup consumed"
        );
        let receipt: serde_json::Value = serde_json::from_slice(
            &fs::read(runtime.tools_dir.join("headroom.json")).expect("read receipt"),
        )
        .expect("parse receipt");
        assert_eq!(receipt["version"], "0.10.7", "receipt restored to previous");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recover_from_interrupted_upgrade_handles_in_place_marker_with_lock_backup() {
        // In-place marker with a lock backup. Recovery should: copy the lock
        // backup back to the active lock path, remove the backup, restore the
        // receipt, and clear the marker. Pip calls fail harmlessly without a
        // real python.
        let (root, runtime, manager) = seed_test_runtime("recover-lock-backup");
        let active_lock = manager.active_lock_path();
        let lock_backup = manager.lock_backup_path();
        fs::write(&active_lock, b"new-lock==2.0\n").expect("seed active lock");
        fs::write(&lock_backup, b"old-lock==1.0\n").expect("seed lock backup");

        manager
            .write_upgrade_marker("0.11.0", Some("0.10.8"), Some(&lock_backup))
            .expect("marker");
        fs::write(
            manager.headroom_receipt_backup_path(),
            br#"{"version":"0.10.8"}"#,
        )
        .expect("receipt snapshot");

        assert!(manager.recover_from_interrupted_upgrade());

        assert!(!manager.upgrade_marker_path().exists(), "marker cleared");
        assert!(!lock_backup.exists(), "lock backup consumed");
        assert_eq!(
            fs::read(&active_lock).expect("read active lock"),
            b"old-lock==1.0\n",
            "active lock rolled back to snapshot content"
        );
        assert!(!manager.headroom_receipt_backup_path().exists());
        let receipt: serde_json::Value = serde_json::from_slice(
            &fs::read(runtime.tools_dir.join("headroom.json")).expect("read receipt"),
        )
        .expect("parse receipt");
        assert_eq!(receipt["version"], "0.10.8");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn atomic_upgrade_purges_stale_backup_and_reports_failure_without_python() {
        // Without a real standalone python available, the upgrade will fail.
        // We still want to verify that a stale backup from a previous aborted
        // upgrade is removed before the attempt, and that the live venv is
        // preserved or restored byte-for-byte after the failure.
        let (root, runtime, manager) = seed_test_runtime("atomic-stale");

        // Pre-seed a stale backup (simulating a previous aborted upgrade).
        let stale_backup = manager.venv_backup_dir();
        fs::create_dir_all(&stale_backup).expect("stale backup");
        fs::write(stale_backup.join("stale-marker"), b"stale").expect("stale marker");

        // Fake release — bogus URL ensures download/install would fail even
        // if we somehow reached that step.
        let release = HeadroomRelease {
            version: "0.0.0-test".into(),
            wheel_url: "https://example.invalid/headroom.whl".into(),
            sha256: "deadbeef".into(),
        };

        let outcome = manager.atomic_upgrade_headroom(&release, |_| {}, true);

        match outcome {
            UpgradeOutcome::InstallFailed { .. } => {}
            UpgradeOutcome::InstalledPendingValidation { .. } => {
                panic!("unexpected success without python");
            }
        }

        // Live venv is still present with its original content. Depending on
        // which preflight failed, the old venv may never have been moved.
        assert!(
            runtime.venv_dir.join("marker").exists(),
            "original marker preserved or restored"
        );
        // Stale backup purged (either consumed during restore or cleaned at start).
        assert!(!stale_backup.exists(), "stale backup removed");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn requirements_are_stale_recognizes_legacy_sha_and_migrates_receipt() {
        if super::LEGACY_REQUIREMENTS_LOCK_SHAS.is_empty() {
            return;
        }
        let (root, runtime, manager) = seed_test_runtime("legacy-sha-migrate");
        let legacy_sha = super::LEGACY_REQUIREMENTS_LOCK_SHAS[0];
        let receipt_path = runtime.tools_dir.join("headroom.json");
        fs::write(
            &receipt_path,
            serde_json::to_vec(&serde_json::json!({
                "version": "0.2.50",
                "artifact": { "requirementsLockSha256": legacy_sha },
            }))
            .unwrap(),
        )
        .expect("receipt");

        assert!(
            !manager.requirements_are_stale(),
            "legacy sha should be treated as current"
        );

        let receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(&receipt_path).expect("receipt read")).expect("json");
        assert_eq!(
            receipt["artifact"]["requirementsLockSha256"],
            requirements_lock_sha(super::bootstrap_requirements_lock()),
            "receipt should be migrated to the new-format sha"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn requirements_are_stale_flags_unknown_sha() {
        let (root, runtime, manager) = seed_test_runtime("unknown-sha");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "0.2.45",
                "artifact": { "requirementsLockSha256": "deadbeef".repeat(8) },
            }))
            .unwrap(),
        )
        .expect("receipt");

        assert!(
            manager.requirements_are_stale(),
            "unknown sha must force a reinstall"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_in_place_skips_lock_snapshot_when_sha_matches() {
        let (root, runtime, manager) = seed_test_runtime("in-place-current");
        let current_sha = requirements_lock_sha(super::bootstrap_requirements_lock());
        // Receipt must be ≥ ATOMIC_REBUILD_FLOOR_VERSION or `prepare_in_place_upgrade`
        // forces an atomic rebuild before reaching the lock-snapshot logic.
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "0.20.0",
                "artifact": { "requirementsLockSha256": current_sha },
            }))
            .unwrap(),
        )
        .expect("receipt");

        let ctx = manager
            .prepare_in_place_upgrade()
            .expect("eligible for in-place");
        assert_eq!(ctx.previous_version, "0.20.0");
        assert!(
            ctx.previous_lock_backup.is_none(),
            "lock unchanged => no snapshot"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_in_place_skips_lock_snapshot_when_stored_sha_is_legacy() {
        if super::LEGACY_REQUIREMENTS_LOCK_SHAS.is_empty() {
            return;
        }
        let (root, runtime, manager) = seed_test_runtime("in-place-legacy");
        let legacy_sha = super::LEGACY_REQUIREMENTS_LOCK_SHAS[0];
        // Receipt must be ≥ ATOMIC_REBUILD_FLOOR_VERSION; this test exercises
        // the legacy-sha path, not the version-floor path.
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "0.20.0",
                "artifact": { "requirementsLockSha256": legacy_sha },
            }))
            .unwrap(),
        )
        .expect("receipt");

        let ctx = manager
            .prepare_in_place_upgrade()
            .expect("eligible for in-place");
        assert_eq!(ctx.previous_version, "0.20.0");
        assert!(ctx.previous_lock_backup.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_in_place_snapshots_lock_when_pins_differ() {
        let (root, runtime, manager) = seed_test_runtime("in-place-lock-churn");
        // Receipt must be ≥ ATOMIC_REBUILD_FLOOR_VERSION; this test is about
        // the lock-snapshot path, not the version-floor path (covered by
        // `receipt_requires_atomic_rebuild_below_floor`).
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "0.20.0",
                "artifact": { "requirementsLockSha256": "deadbeef".repeat(8) },
            }))
            .unwrap(),
        )
        .expect("receipt");
        fs::write(manager.active_lock_path(), b"old-lock-content==1.0\n")
            .expect("seed active lock");

        let ctx = manager
            .prepare_in_place_upgrade()
            .expect("eligible for in-place");
        assert_eq!(ctx.previous_version, "0.20.0");
        let backup = ctx
            .previous_lock_backup
            .as_ref()
            .expect("lock changed => snapshot taken");
        assert_eq!(
            fs::read(backup).expect("backup readable"),
            b"old-lock-content==1.0\n"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_in_place_falls_back_to_atomic_when_lock_missing() {
        // Lock pins differ AND the active lock is missing on disk => caller
        // should fall through to the full atomic rebuild so rollback stays
        // safe. Receipt must be ≥ ATOMIC_REBUILD_FLOOR_VERSION so the
        // version-floor early-return doesn't pre-empt the assertion target.
        let (root, runtime, manager) = seed_test_runtime("in-place-no-lock-on-disk");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "0.20.0",
                "artifact": { "requirementsLockSha256": "deadbeef".repeat(8) },
            }))
            .unwrap(),
        )
        .expect("receipt");
        // no active lock written
        assert!(manager.prepare_in_place_upgrade().is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_in_place_skipped_without_installed_version() {
        let (root, runtime, manager) = seed_test_runtime("in-place-no-receipt");
        fs::remove_file(runtime.tools_dir.join("headroom.json")).expect("drop receipt");
        assert!(manager.prepare_in_place_upgrade().is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_in_place_falls_back_to_atomic_when_receipt_predates_floor() {
        // 0.8.2 (shipped in headroom-desktop 0.2.50-rc.1 and the fallback
        // version on every Sentry boot-validation stall observed for 0.3.6)
        // is below the 0.10.0 floor. Force the rebuild even when the lock
        // snapshot is takeable.
        let (root, runtime, manager) = seed_test_runtime("in-place-pre-floor");
        let current_sha = requirements_lock_sha(super::bootstrap_requirements_lock());
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "0.8.2",
                "artifact": { "requirementsLockSha256": current_sha },
            }))
            .unwrap(),
        )
        .expect("receipt");
        fs::write(manager.active_lock_path(), b"old-lock-content==1.0\n")
            .expect("seed active lock");
        assert!(
            manager.prepare_in_place_upgrade().is_none(),
            "0.8.2 receipt must force atomic rebuild even when lock snapshot is takeable"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repair_stale_requirements_updates_receipt_and_emits_progress() {
        let (root, runtime, manager) = seed_test_runtime("repair-requirements");
        write_executable(&runtime.managed_python(), "#!/bin/sh\nexit 0\n");
        write_executable(&manager.headroom_entrypoint(), "#!/bin/sh\nexit 0\n");
        fs::write(
            runtime.tools_dir.join("headroom.json"),
            br#"{
                "version":"0.8.2",
                "artifact":{"requirementsLockSha256":"stale"},
                "mcp":{"configured":false}
            }"#,
        )
        .expect("seed receipt");

        let mut steps = Vec::new();
        manager
            .repair_stale_requirements_with_progress(|step| steps.push(step.step.to_string()))
            .expect("repair succeeds");

        assert!(steps.iter().any(|step| step == "Repairing dependencies"));
        assert!(steps.iter().any(|step| step == "Configuring integrations"));
        assert!(steps.iter().any(|step| step == "Repair complete"));

        let receipt = fs::read(runtime.tools_dir.join("headroom.json")).expect("receipt");
        let receipt: serde_json::Value = serde_json::from_slice(&receipt).expect("receipt json");
        assert_eq!(
            receipt["artifact"]["requirementsLockSha256"],
            requirements_lock_sha(super::bootstrap_requirements_lock())
        );
        assert_eq!(receipt["mcp"]["configured"], true);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_test_headroom_succeeds_with_executable_python() {
        let (root, runtime, manager) = seed_test_runtime("smoke-ok");
        write_executable(&runtime.managed_python(), "#!/bin/sh\nexit 0\n");

        manager
            .smoke_test_headroom_with_timeout(super::HEADROOM_SMOKE_TEST_TIMEOUT)
            .expect("smoke test succeeds");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_test_headroom_returns_command_failure_output_on_nonzero_exit() {
        let (root, runtime, manager) = seed_test_runtime("smoke-fail");
        write_executable(
            &runtime.managed_python(),
            "#!/bin/sh\necho failure-stdout\necho failure-stderr >&2\nexit 7\n",
        );

        let err = manager
            .smoke_test_headroom_with_timeout(super::HEADROOM_SMOKE_TEST_TIMEOUT)
            .expect_err("smoke test should fail");
        let failure = err
            .chain()
            .find_map(|cause| cause.downcast_ref::<CommandFailure>())
            .expect("command failure");
        assert_eq!(failure.exit_code, Some(7));
        assert!(failure.stdout.contains("failure-stdout"));
        assert!(failure.stderr.contains("failure-stderr"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_test_markitdown_is_noop_when_not_installed() {
        let (root, _runtime, manager) = seed_test_runtime("markitdown-smoke-absent");
        manager
            .smoke_test_markitdown_with_timeout(Duration::from_secs(2))
            .expect("no-op when markitdown is absent");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_test_ponytail_is_noop_when_not_installed() {
        let (root, _runtime, manager) = seed_test_runtime("ponytail-smoke-absent");
        manager
            .smoke_test_ponytail()
            .expect("no-op when ponytail receipt is absent");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ponytail_disabled_receipt_reports_installed_not_missing() {
        // A receipt with enabled:false means the user disabled it via the app.
        // On hosts without a disable verb the plugin is gone, but the card must
        // still show "installed" (Enable), not "not installed" (Install).
        let (root, runtime, manager) = seed_test_runtime("ponytail-disabled");
        fs::write(
            runtime.tools_dir.join("ponytail.json"),
            br#"{"version":"latest","enabled":false}"#,
        )
        .expect("receipt");
        assert!(matches!(
            manager.detect_status("ponytail"),
            crate::models::ToolStatus::Healthy
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn uninstall_ponytail_is_noop_without_receipt() {
        // Cleanup must not touch plugin/marketplace config Headroom never wrote.
        let (root, _runtime, manager) = seed_test_runtime("ponytail-uninstall-noreceipt");
        manager
            .uninstall_ponytail()
            .expect("no-op when ponytail receipt is absent");
        let _ = fs::remove_dir_all(root);
    }

    /// End-to-end round trip against the real `claude`/`codex` plugin CLIs:
    /// install, confirm both presence checks + smoke test flip on, then
    /// uninstall and confirm they flip off. Ignored by default — it needs at
    /// least one CLI on PATH plus network, and mutates the real ~/.claude and
    /// ~/.codex plugin config. Run locally:
    /// `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored ponytail_install_roundtrip`
    #[test]
    #[ignore]
    fn ponytail_install_roundtrip() {
        let (root, _runtime, manager) = seed_test_runtime("ponytail-roundtrip");

        if crate::claude_cli::detect_claude_cli().is_none()
            && crate::claude_cli::detect_codex_cli().is_none()
        {
            eprintln!("skipping ponytail_install_roundtrip: no claude/codex CLI on PATH");
            let _ = fs::remove_dir_all(&root);
            return;
        }

        // Capture every result and always run uninstall before asserting, so a
        // failed assertion never leaves the plugin behind on the real machine.
        let install = manager.install_ponytail();
        let installed = manager.ponytail_installed();
        let smoke_while_installed = manager.smoke_test_ponytail();
        let uninstall = manager.uninstall_ponytail();
        let gone = !manager.ponytail_installed();
        let _ = fs::remove_dir_all(&root);

        install.expect("install_ponytail should succeed");
        assert!(
            installed,
            "ponytail_installed() should be true after install"
        );
        smoke_while_installed.expect("smoke_test_ponytail should pass while installed");
        uninstall.expect("uninstall_ponytail should succeed");
        assert!(gone, "ponytail_installed() should be false after uninstall");
    }

    #[test]
    fn smoke_test_markitdown_succeeds_when_entrypoint_runs() {
        let (root, runtime, manager) = seed_test_runtime("markitdown-smoke-ok");
        fs::write(
            runtime.tools_dir.join("markitdown.json"),
            br#"{"version":"0.1.6","enabled":true}"#,
        )
        .expect("receipt");
        write_executable(&manager.markitdown_entrypoint(), "#!/bin/sh\nexit 0\n");

        manager
            .smoke_test_markitdown_with_timeout(super::HEADROOM_SMOKE_TEST_TIMEOUT)
            .expect("smoke test succeeds");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_test_markitdown_fails_on_nonzero_exit() {
        let (root, runtime, manager) = seed_test_runtime("markitdown-smoke-fail");
        fs::write(
            runtime.tools_dir.join("markitdown.json"),
            br#"{"version":"0.1.6","enabled":true}"#,
        )
        .expect("receipt");
        write_executable(&manager.markitdown_entrypoint(), "#!/bin/sh\nexit 3\n");

        manager
            .smoke_test_markitdown_with_timeout(super::HEADROOM_SMOKE_TEST_TIMEOUT)
            .expect_err("smoke test should fail");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_test_headroom_repairs_pydantic_core_skew_and_retries() {
        let (root, runtime, manager) = seed_test_runtime("smoke-pydantic-skew");
        let state_file = root.join("smoke-attempts");
        let pip_log = root.join("pip-args");
        let script = format!(
            r#"#!/bin/sh
case "$1" in
  -c)
    if [ -f '{state}' ]; then
      exit 0
    fi
    touch '{state}'
    cat >&2 <<'EOF'
Traceback (most recent call last):
  File "<string>", line 1, in <module>
SystemError: The installed pydantic-core version (2.41.5) is incompatible with the current pydantic version, which requires 2.46.3. If you encounter this error, make sure that you haven't upgraded pydantic-core manually.
EOF
    exit 1
    ;;
  -m)
    echo "$@" >> '{pip_log}'
    exit 0
    ;;
esac
exit 0
"#,
            state = state_file.display(),
            pip_log = pip_log.display(),
        );
        write_executable(&runtime.managed_python(), &script);

        manager
            .smoke_test_headroom()
            .expect("repair should let smoke retry succeed");

        let pip_args = fs::read_to_string(&pip_log).expect("pip log written");
        assert!(
            pip_args.contains("pydantic-core==2.46.3"),
            "expected repair to install pydantic-core==2.46.3, got: {pip_args}"
        );
        // pydantic itself must also be force-reinstalled to collapse any
        // duplicate dist-info dirs that cause the flip-flop skew.
        let pydantic_invocations = pip_args
            .lines()
            .filter(|line| {
                line.contains("--force-reinstall")
                    && line.split_whitespace().any(|tok| tok == "pydantic")
            })
            .count();
        assert_eq!(
            pydantic_invocations, 1,
            "expected exactly one force-reinstall of pydantic, got: {pip_args}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_test_headroom_does_not_repair_unrelated_failures() {
        let (root, runtime, manager) = seed_test_runtime("smoke-unrelated-fail");
        let state_file = root.join("attempts");
        let script = format!(
            "#!/bin/sh\necho >> '{state}'\necho boom >&2\nexit 1\n",
            state = state_file.display(),
        );
        write_executable(&runtime.managed_python(), &script);

        let err = manager
            .smoke_test_headroom()
            .expect_err("smoke should fail without retry");
        let failure = err
            .chain()
            .find_map(|cause| cause.downcast_ref::<CommandFailure>())
            .expect("command failure");
        assert_eq!(failure.exit_code, Some(1));

        let attempts = fs::read_to_string(&state_file).expect("attempts log");
        assert_eq!(
            attempts.lines().count(),
            1,
            "non-skew failures should not retry"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_test_headroom_times_out() {
        let (root, runtime, manager) = seed_test_runtime("smoke-timeout");
        write_executable(&runtime.managed_python(), "#!/bin/sh\nsleep 1\n");

        let err = manager
            .smoke_test_headroom_with_timeout(Duration::from_millis(100))
            .expect_err("smoke test should time out");
        let failure = err
            .chain()
            .find_map(|cause| cause.downcast_ref::<CommandFailure>())
            .expect("command failure");
        assert_eq!(failure.exit_code, None);
        assert!(failure.stderr.contains("command timed out after 100ms"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pip_output_capture_keeps_last_n_lines() {
        let mut cap = PipOutputCapture::new(3);
        cap.push("first");
        cap.push("second");
        cap.push("third");
        // Buffer is now full; the next push must evict "first".
        cap.push("fourth");
        cap.push("fifth");
        let out = cap.into_string();
        // We keep the LAST 3 lines because the tail (warnings, "Successfully
        // installed", "Skipping X") is the diagnostically interesting part.
        assert_eq!(out, "third\nfourth\nfifth");
    }

    #[test]
    fn pip_output_capture_handles_empty_and_partial_fill() {
        let cap = PipOutputCapture::new(10);
        assert_eq!(cap.into_string(), "");

        let mut cap = PipOutputCapture::new(10);
        cap.push("only line");
        assert_eq!(cap.into_string(), "only line");

        let mut cap = PipOutputCapture::new(10);
        cap.push("a");
        cap.push("b");
        assert_eq!(cap.into_string(), "a\nb");
    }
}
