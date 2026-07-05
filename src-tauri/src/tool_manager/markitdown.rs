use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::process_runner::{run_command_streaming, run_command_with_timeout};
use crate::tool_manager::{
    run_pip_install_with_retries_streaming, ToolManager, HEADROOM_SMOKE_TEST_TIMEOUT,
};

pub const MARKITDOWN_PINNED_VERSION: &str = "0.1.6";

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
}
