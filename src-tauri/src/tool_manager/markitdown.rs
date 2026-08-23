use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::process_runner::{run_command_streaming, run_command_with_timeout};
use crate::tool_manager::{
    run_pip_install_with_retries_streaming, ToolManager, HEADROOM_SMOKE_TEST_TIMEOUT,
};

pub const MARKITDOWN_PINNED_VERSION: &str = "0.1.6";

/// Content-free MarkItDown runtime ownership proof for selective activation
/// receipts. It covers only managed executable, shim, and receipt artifacts;
/// their paths and contents remain inside the local ToolManager boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarkitdownInstallationSnapshot {
    pub entrypoint_fingerprint: Option<String>,
    pub shim_fingerprint: Option<String>,
    pub receipt_fingerprint: Option<String>,
}

impl MarkitdownInstallationSnapshot {
    pub fn is_absent(&self) -> bool {
        self.entrypoint_fingerprint.is_none()
            && self.shim_fingerprint.is_none()
            && self.receipt_fingerprint.is_none()
    }

    pub fn is_complete(&self) -> bool {
        self.entrypoint_fingerprint.is_some()
            && self.shim_fingerprint.is_some()
            && self.receipt_fingerprint.is_some()
    }
}

fn artifact_fingerprint(path: &std::path::Path) -> Result<Option<String>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(fingerprint(bytes))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

fn symlink_fingerprint(path: &std::path::Path) -> Result<Option<String>> {
    match std::fs::read_link(path) {
        Ok(target) => Ok(Some(fingerprint(target.as_os_str().as_encoded_bytes()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading symlink {}", path.display())),
    }
}

fn fingerprint(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_ref());
    format!("sha256:{:x}", hasher.finalize())
}

impl ToolManager {
    /// Verifies the managed `markitdown` console script actually executes (its
    /// base converters and their dependencies import). No-op when the addon
    /// isn't installed, so it can be called unconditionally from a smoke pass.
    pub fn smoke_test_markitdown(&self) -> Result<()> {
        self.smoke_test_markitdown_with_timeout(HEADROOM_SMOKE_TEST_TIMEOUT)
    }

    pub(super) fn smoke_test_markitdown_with_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<()> {
        if !self.markitdown_installed() {
            return Ok(());
        }
        let bin = self.markitdown_entrypoint();
        run_command_with_timeout(&bin, &["--help"], &self.runtime.root_dir, timeout)
            .with_context(|| format!("running markitdown smoke test with {}", bin.display()))?;
        Ok(())
    }

    pub fn markitdown_entrypoint(&self) -> PathBuf {
        self.runtime.venv_dir.join("bin").join("markitdown")
    }

    /// Symlink in the Headroom-managed bin dir. The Office nudge and the Bash
    /// permission both reference this absolute path, so it works whether or not
    /// the bin dir is on PATH (RTK, which exports it, is now opt-in).
    pub fn markitdown_shim_path(&self) -> PathBuf {
        self.runtime.bin_dir.join("markitdown")
    }

    fn markitdown_receipt_path(&self) -> PathBuf {
        self.runtime.tools_dir.join("markitdown.json")
    }

    pub fn markitdown_receipt_snapshot(&self) -> Option<serde_json::Value> {
        self.read_tool_receipt("markitdown")
    }

    pub fn markitdown_installation_snapshot(&self) -> Result<MarkitdownInstallationSnapshot> {
        Ok(MarkitdownInstallationSnapshot {
            entrypoint_fingerprint: artifact_fingerprint(&self.markitdown_entrypoint())?,
            shim_fingerprint: symlink_fingerprint(&self.markitdown_shim_path())?,
            receipt_fingerprint: artifact_fingerprint(&self.markitdown_receipt_path())?,
        })
    }

    /// Reject partial managed state before selective activation could write
    /// integrations that reference an absent or replaced runtime.
    pub fn validate_markitdown_installation_snapshot(
        &self,
        snapshot: &MarkitdownInstallationSnapshot,
    ) -> Result<()> {
        if snapshot.is_absent() || snapshot.is_complete() {
            return Ok(());
        }
        bail!(
            "MarkItDown has a partial managed runtime; repair it from Addons before selective activation so existing artifacts are preserved"
        )
    }

    fn ensure_markitdown_shim(&self) -> Result<()> {
        let shim = self.markitdown_shim_path();
        if shim.exists() || shim.symlink_metadata().is_ok() {
            let _ = std::fs::remove_file(&shim);
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(self.markitdown_entrypoint(), &shim)
            .with_context(|| format!("symlinking markitdown shim {}", shim.display()))?;
        Ok(())
    }

    pub fn markitdown_installed(&self) -> bool {
        self.runtime.tools_dir.join("markitdown.json").exists()
            && self.markitdown_entrypoint().exists()
    }

    pub fn install_markitdown(&self) -> Result<()> {
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
                &format!("markitdown[all]=={MARKITDOWN_PINNED_VERSION}"),
            ],
            &self.runtime.root_dir,
            |line| log::info!("markitdown pip: {line}"),
        )?;
        if !self.markitdown_entrypoint().exists() {
            bail!(
                "markitdown install completed but {} was not found",
                self.markitdown_entrypoint().display()
            );
        }
        run_command_with_timeout(
            &self.markitdown_entrypoint(),
            &["--help"],
            &self.runtime.root_dir,
            HEADROOM_SMOKE_TEST_TIMEOUT,
        )
        .context("markitdown installed but failed its smoke test")?;
        self.ensure_markitdown_shim()?;
        self.write_tool_receipt(
            "markitdown",
            json!({ "version": MARKITDOWN_PINNED_VERSION, "enabled": true }),
        )?;
        Ok(())
    }

    pub fn set_markitdown_enabled(&self, enabled: bool) -> Result<()> {
        if !self.markitdown_installed() {
            bail!("markitdown is not installed");
        }
        self.write_tool_receipt(
            "markitdown",
            json!({ "version": MARKITDOWN_PINNED_VERSION, "enabled": enabled }),
        )?;
        Ok(())
    }

    pub fn uninstall_markitdown(&self) -> Result<()> {
        let _ = run_command_streaming(
            &self.runtime.managed_python(),
            &["-m", "pip", "uninstall", "-y", "markitdown"],
            &self.runtime.root_dir,
            &mut |line: &str| log::info!("markitdown pip uninstall: {line}"),
        );
        let shim = self.markitdown_shim_path();
        if shim.symlink_metadata().is_ok() {
            let _ = std::fs::remove_file(&shim);
        }
        let receipt = self.runtime.tools_dir.join("markitdown.json");
        if receipt.exists() {
            std::fs::remove_file(&receipt)
                .with_context(|| format!("removing {}", receipt.display()))?;
        }
        Ok(())
    }

    /// Restores only the Switchboard-owned MarkItDown receipt after an
    /// integration rollback has proven the current receipt still belongs to
    /// this activation. The runtime itself is deliberately preserved here.
    pub fn restore_markitdown_receipt_if_unchanged(
        &self,
        previous_receipt: Option<&serde_json::Value>,
        after_receipt: Option<&serde_json::Value>,
    ) -> Result<()> {
        if self.markitdown_receipt_snapshot().as_ref() != after_receipt {
            bail!("MarkItDown managed receipt changed after activation");
        }
        let receipt_path = self.markitdown_receipt_path();
        if let Some(previous_receipt) = previous_receipt {
            self.write_tool_receipt("markitdown", previous_receipt.clone())?;
        } else if receipt_path.exists() {
            std::fs::remove_file(&receipt_path)
                .with_context(|| format!("removing {}", receipt_path.display()))?;
        }
        Ok(())
    }

    /// Removes a MarkItDown runtime created by one selective activation only
    /// when every owned runtime artifact still matches its post-activation
    /// fingerprint. The normal broad uninstall remains separate for explicit
    /// user-driven Addons cleanup.
    pub fn uninstall_markitdown_if_unchanged(
        &self,
        after: &MarkitdownInstallationSnapshot,
    ) -> Result<()> {
        if !after.is_complete() {
            bail!("MarkItDown selective rollback has no complete runtime ownership metadata");
        }
        let current = self.markitdown_installation_snapshot()?;
        if &current != after {
            bail!(
                "MarkItDown runtime changed after activation; its executable, shim, and receipt were preserved"
            );
        }
        run_command_streaming(
            &self.runtime.managed_python(),
            &["-m", "pip", "uninstall", "-y", "markitdown"],
            &self.runtime.root_dir,
            &mut |line: &str| log::info!("markitdown pip uninstall: {line}"),
        )?;
        if self.markitdown_entrypoint().exists() {
            bail!("markitdown uninstall completed but its managed entrypoint remains");
        }
        let shim = self.markitdown_shim_path();
        if shim.symlink_metadata().is_ok() {
            std::fs::remove_file(&shim).with_context(|| format!("removing {}", shim.display()))?;
        }
        let receipt = self.markitdown_receipt_path();
        if receipt.exists() {
            std::fs::remove_file(&receipt)
                .with_context(|| format!("removing {}", receipt.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MarkitdownInstallationSnapshot;

    #[test]
    fn installation_snapshot_rejects_partial_runtime_state() {
        let absent = MarkitdownInstallationSnapshot {
            entrypoint_fingerprint: None,
            shim_fingerprint: None,
            receipt_fingerprint: None,
        };
        let partial = MarkitdownInstallationSnapshot {
            entrypoint_fingerprint: Some("sha256:entrypoint".into()),
            shim_fingerprint: None,
            receipt_fingerprint: Some("sha256:receipt".into()),
        };
        let complete = MarkitdownInstallationSnapshot {
            entrypoint_fingerprint: Some("sha256:entrypoint".into()),
            shim_fingerprint: Some("sha256:shim".into()),
            receipt_fingerprint: Some("sha256:receipt".into()),
        };
        assert!(absent.is_absent());
        assert!(!partial.is_absent());
        assert!(!partial.is_complete());
        assert!(complete.is_complete());
    }
}
