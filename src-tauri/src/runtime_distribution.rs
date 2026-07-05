use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::tool_manager::BootstrapStepUpdate;

pub(crate) const HEADROOM_REQUIREMENTS_LOCK: &str =
    include_str!("../python/headroom-requirements.lock");
pub(crate) const HEADROOM_LINUX_REQUIREMENTS_LOCK: &str =
    include_str!("../python/headroom-linux-requirements.lock");
pub(crate) const PYTHON_STANDALONE_RELEASE: &str = "20251014";
pub(crate) const PYTHON_SHA256_MACOS_AARCH64: &str =
    "84cb7acbf75264982c8bdd818bfa1ff0f1eb76007b48a5f3e01d28633b46afdf";
pub(crate) const PYTHON_SHA256_MACOS_X86_64: &str =
    "f76a921e71e9c8954cccd00f176b7083041527b3b4223670d05bbb2f51209d3f";
pub(crate) const PYTHON_SHA256_LINUX_X86_64: &str =
    "c74addcd1b033a6e4d60ead3ab47fcc995569027e01d3061c4a934f363c4a0cf";
pub(crate) const PYTHON_SHA256_LINUX_AARCH64: &str =
    "d2a6c0d4ceea088f635b309a59d5d700a256656423225f96ddfb71d532adb1aa";
pub(crate) const HEADROOM_PINNED_VERSION: &str = "0.27.0";
pub(crate) const HEADROOM_PINNED_WHEEL_URL: &str = "https://files.pythonhosted.org/packages/10/95/928bfb645df23025fb375de19c7d57ec21a0991712236d7748ce456139e3/headroom_ai-0.27.0-cp310-abi3-macosx_11_0_arm64.whl";
pub(crate) const HEADROOM_PINNED_SHA256: &str =
    "00b54b70533c841f4702fffaf215eff84bafed7612c07a56d675ef8a1ffab543";
pub(crate) const RTK_VERSION: &str = "0.42.4";
pub(crate) const RTK_SHA256_MACOS_AARCH64: &str =
    "f223ca074a0215af002679bc1d34ca92b93e25b3e8ae16aace6e84c06e586802";
pub(crate) const RTK_SHA256_MACOS_X86_64: &str =
    "84121316867613e61925c209607f033b2113bb0ce312c267a79d3e3e8f221e49";
pub(crate) const RTK_SHA256_LINUX_AARCH64: &str =
    "cc2b91c064eb670c097c184913c8fbcb1a943d53d7fe505375e96ba0c5b6459f";
pub(crate) const RTK_SHA256_LINUX_X86_64: &str =
    "34975116da11e09e502501daf758143e0b22ed3a42a10eb67fb693a6270d9e36";

pub(crate) struct DownloadArtifact {
    pub(crate) url: String,
    pub(crate) sha256: Option<&'static str>,
}

/// Metadata for a specific headroom-ai release fetched from PyPI.
pub(crate) struct HeadroomRelease {
    pub(crate) version: String,
    pub(crate) wheel_url: String,
    pub(crate) sha256: String,
}

impl HeadroomRelease {
    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeMaintenanceKind {
    Upgrade,
    RequirementsRepair,
}

/// Outcome of [`ToolManager::atomic_upgrade_headroom`].
///
/// `InstalledPendingValidation` means install + smoke test succeeded but the
/// backup is still on disk. The caller must either commit or rollback.
pub enum UpgradeOutcome {
    InstalledPendingValidation {
        /// Last ~100 lines of pip stdout/stderr from this install. Attached
        /// to the boot-validation Sentry event when it later fails — pip
        /// can return exit 0 while leaving the venv in a broken state
        /// (skipped packages, downgraded native deps with mismatched ABI,
        /// etc.), and without the tail there's no record of what actually
        /// happened. Empty string when capture was skipped (e.g., bootstrap).
        pip_output_tail: String,
    },
    InstallFailed {
        /// True if we successfully restored the old venv + receipt.
        restored: bool,
        error: anyhow::Error,
    },
}

/// State required to perform (and roll back) an in-place upgrade — i.e. an
/// upgrade that mutates the live venv instead of rebuilding it. When
/// `previous_lock_backup` is `Some`, the dep lock has churned and the file at
/// that path is the pre-upgrade lock content, used by rollback and recovery
/// to `pip install --upgrade -r <backup>` back to the prior pin set.
pub(crate) struct InPlaceUpgradeContext {
    pub(crate) previous_version: String,
    pub(crate) previous_lock_backup: Option<PathBuf>,
}

/// Bounded ring buffer collecting pip stdout/stderr lines for post-mortem
/// diagnostics. Keeps the LAST `max_lines` (drops oldest when full) so
/// warnings, "Skipping X", "Successfully installed ..." lines that pip
/// prints near the end of a run survive. Sentry extras cap at ~16KB; 100
/// lines at the typical ~120-char pip line averages ~12KB.
pub(crate) struct PipOutputCapture {
    pub(crate) lines: std::collections::VecDeque<String>,
    max_lines: usize,
}

impl PipOutputCapture {
    pub(crate) fn new(max_lines: usize) -> Self {
        Self {
            lines: std::collections::VecDeque::with_capacity(max_lines),
            max_lines,
        }
    }

    pub(crate) fn push(&mut self, line: &str) {
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line.to_string());
    }

    pub(crate) fn into_string(self) -> String {
        let parts: Vec<String> = self.lines.into_iter().collect();
        parts.join("\n")
    }
}

/// State required to perform (and roll back) an in-place upgrade — i.e. an
/// upgrade that mutates the live venv instead of rebuilding it. When
/// `previous_lock_backup` is `Some`, the dep lock has churned and the file at
/// that path is the pre-upgrade lock content, used by rollback and recovery
/// to `pip install --upgrade -r <backup>` back to the prior pin set.

pub(crate) fn available_disk_bytes(path: &Path) -> Option<u64> {
    #[cfg(not(unix))]
    {
        let _ = path;
        return None;
    }

    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
        if ret != 0 {
            return None;
        }
        Some(stat.f_bavail as u64 * stat.f_frsize as u64)
    }
}

pub(crate) fn python_distribution_artifact() -> Result<DownloadArtifact> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok(DownloadArtifact {
            url: format!(
                "https://github.com/astral-sh/python-build-standalone/releases/download/{}/cpython-3.12.12+20251014-aarch64-apple-darwin-install_only_stripped.tar.gz",
                PYTHON_STANDALONE_RELEASE
            ),
            sha256: Some(PYTHON_SHA256_MACOS_AARCH64),
        }),
        ("macos", "x86_64") => Ok(DownloadArtifact {
            url: format!(
                "https://github.com/astral-sh/python-build-standalone/releases/download/{}/cpython-3.12.12+20251014-x86_64-apple-darwin-install_only_stripped.tar.gz",
                PYTHON_STANDALONE_RELEASE
            ),
            sha256: Some(PYTHON_SHA256_MACOS_X86_64),
        }),
        ("linux", "x86_64") => Ok(DownloadArtifact {
            url: format!(
                "https://github.com/astral-sh/python-build-standalone/releases/download/{}/cpython-3.12.12+20251014-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz",
                PYTHON_STANDALONE_RELEASE
            ),
            sha256: Some(PYTHON_SHA256_LINUX_X86_64),
        }),
        ("linux", "aarch64") => Ok(DownloadArtifact {
            url: format!(
                "https://github.com/astral-sh/python-build-standalone/releases/download/{}/cpython-3.12.12+20251014-aarch64-unknown-linux-gnu-install_only_stripped.tar.gz",
                PYTHON_STANDALONE_RELEASE
            ),
            sha256: Some(PYTHON_SHA256_LINUX_AARCH64),
        }),
        (os, arch) => bail!("unsupported Headroom managed Python target: {os}/{arch}"),
    }
}

pub(crate) fn rtk_distribution_artifact() -> Result<DownloadArtifact> {
    let (target, sha256) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => ("aarch64-apple-darwin", RTK_SHA256_MACOS_AARCH64),
        ("macos", "x86_64") => ("x86_64-apple-darwin", RTK_SHA256_MACOS_X86_64),
        ("linux", "aarch64") => ("aarch64-unknown-linux-gnu", RTK_SHA256_LINUX_AARCH64),
        ("linux", "x86_64") => ("x86_64-unknown-linux-musl", RTK_SHA256_LINUX_X86_64),
        (os, arch) => bail!("unsupported RTK target: {os}/{arch}"),
    };

    Ok(DownloadArtifact {
        url: format!(
            "https://github.com/rtk-ai/rtk/releases/download/v{}/rtk-{}.tar.gz",
            RTK_VERSION, target
        ),
        sha256: Some(sha256),
    })
}

pub(crate) fn download_to_path(
    url: &str,
    destination: &Path,
    expected_sha256: Option<&str>,
) -> Result<()> {
    download_to_path_with_progress(url, destination, expected_sha256, |_, _| {})
}

pub(crate) fn download_to_path_with_progress<F>(
    url: &str,
    destination: &Path,
    expected_sha256: Option<&str>,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64, Option<u64>),
{
    if destination.exists() {
        if let Some(expected_sha256) = expected_sha256 {
            match verify_sha256_file(destination, expected_sha256) {
                Ok(()) => return Ok(()),
                Err(_) => {
                    std::fs::remove_file(destination)
                        .with_context(|| format!("removing {}", destination.display()))?;
                }
            }
        } else {
            return Ok(());
        }
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("mac-ai-switchboard/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(30 * 60))
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .context("building download client")?;

    let tmp_path = destination.with_extension("partial");
    const MAX_ATTEMPTS: u32 = 5;
    let mut last_err = anyhow::anyhow!("no attempts made");

    for attempt in 0..MAX_ATTEMPTS {
        if attempt > 0 {
            // 2s, 4s, 8s, 16s between attempts.
            std::thread::sleep(Duration::from_secs(1u64 << attempt));
        }
        let _ = std::fs::remove_file(&tmp_path);

        let result = (|| -> Result<()> {
            let mut response = client
                .get(url)
                .send()
                .with_context(|| format!("downloading {}", url))?
                .error_for_status()
                .with_context(|| format!("downloading {}", url))?;

            let total_bytes = response.content_length();
            let mut file = std::fs::File::create(&tmp_path)
                .with_context(|| format!("creating {}", tmp_path.display()))?;
            let mut hasher = Sha256::new();
            let mut buf = vec![0u8; 64 * 1024];
            let mut downloaded: u64 = 0;
            on_progress(0, total_bytes);
            let mut last_emit = Instant::now();

            loop {
                let n = response.read(&mut buf).context("reading download body")?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n])
                    .with_context(|| format!("writing {}", tmp_path.display()))?;
                hasher.update(&buf[..n]);
                downloaded += n as u64;
                if last_emit.elapsed() >= Duration::from_millis(250) {
                    on_progress(downloaded, total_bytes);
                    last_emit = Instant::now();
                }
            }
            file.flush().context("flushing download")?;
            drop(file);
            on_progress(downloaded, total_bytes);

            if let Some(expected_sha256) = expected_sha256 {
                let actual_checksum = format!("{:x}", hasher.finalize());
                if actual_checksum != expected_sha256 {
                    bail!(
                        "checksum mismatch for {}: expected {}, got {}",
                        url,
                        expected_sha256,
                        actual_checksum
                    );
                }
            }

            std::fs::rename(&tmp_path, destination).with_context(|| {
                format!(
                    "renaming {} to {}",
                    tmp_path.display(),
                    destination.display()
                )
            })?;
            Ok(())
        })();

        match result {
            Ok(()) => return Ok(()),
            Err(e) => last_err = e,
        }
    }

    let _ = std::fs::remove_file(&tmp_path);
    Err(last_err)
}

pub(crate) fn verify_sha256_file(path: &Path, expected_sha256: &str) -> Result<()> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let actual_checksum = sha256_bytes(&bytes);
    if actual_checksum != expected_sha256 {
        bail!(
            "checksum mismatch for {}: expected {}, got {}",
            path.display(),
            expected_sha256,
            actual_checksum
        );
    }
    Ok(())
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn requirements_lock_sha(lock: &str) -> String {
    let mut hasher = Sha256::new();
    for line in lock.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        hasher.update(trimmed.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

pub(crate) fn bootstrap_requirements_lock() -> &'static str {
    bootstrap_requirements_lock_for_target(std::env::consts::OS)
}

pub(crate) fn bootstrap_requirements_lock_for_target(os: &str) -> &'static str {
    match os {
        // Linux bootstrap only needs the proxy runtime. Installing the full
        // headroom-ai[all] stack pulls optional native packages like hnswlib
        // that fail on many fresh Linux systems.
        "linux" => HEADROOM_LINUX_REQUIREMENTS_LOCK,
        _ => HEADROOM_REQUIREMENTS_LOCK,
    }
}

pub(crate) fn pip_line_to_progress(
    line: &str,
    elapsed: Duration,
    counter: &mut u32,
    base_percent: u8,
    max_percent: u8,
) -> Option<BootstrapStepUpdate> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let message = if let Some(rest) = trimmed.strip_prefix("Collecting ") {
        let spec = rest.split_whitespace().next().unwrap_or(rest);
        let pkg = spec
            .split(|c: char| matches!(c, '=' | '<' | '>' | '!' | '~' | ';' | '['))
            .next()
            .unwrap_or(spec);
        format!("Fetching {}...", pkg)
    } else if let Some(rest) = trimmed.strip_prefix("Downloading ") {
        let file = rest.split_whitespace().next().unwrap_or(rest);
        let name = file.rsplit('/').next().unwrap_or(file);
        let pkg = name.split('-').next().unwrap_or(name);
        format!("Downloading {}...", pkg)
    } else if trimmed.starts_with("Installing collected packages") {
        "Installing packages...".to_string()
    } else if let Some(rest) = trimmed.strip_prefix("Successfully installed ") {
        let count = rest.split_whitespace().count();
        format!("Installed {} packages.", count)
    } else {
        return None;
    };

    *counter = counter.saturating_add(1);
    let span = max_percent.saturating_sub(base_percent).max(1) as u32;
    let advance = (*counter).min(span.saturating_sub(1));
    let percent = (base_percent as u32 + advance).min(max_percent as u32 - 1) as u8;

    let remaining = 90_u64.saturating_sub(elapsed.as_secs()).max(5);
    Some(BootstrapStepUpdate {
        step: "Updating dependencies",
        message,
        eta_seconds: remaining,
        percent,
    })
}
