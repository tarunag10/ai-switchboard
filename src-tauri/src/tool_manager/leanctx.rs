use std::fs::OpenOptions;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::ToolManager;

pub const LEANCTX_DISPLAY_VERSION: &str = "guided-sidecar-v1";
const LEANCTX_HEALTH_TIMEOUT: Duration = Duration::from_millis(800);
const LEANCTX_START_TIMEOUT: Duration = Duration::from_secs(4);
const LEANCTX_CAPABILITIES_PATH: &str = "/capabilities";
const REQUIRED_LEANCTX_CAPABILITY: &str = "headroom_compressor_seam";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeanctxReceipt {
    version: String,
    executable: PathBuf,
    base_url: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default)]
    last_health: Option<String>,
    #[serde(default)]
    last_error: Option<String>,
    #[serde(default)]
    promotion: LeanctxPromotionEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeanctxPromotionEvidence {
    capability_version: Option<String>,
    capability_version_ok: bool,
    protected_content_ok: bool,
    fail_open_ok: bool,
    shadow_contract_ok: bool,
    live_promotion_allowed: bool,
    status: String,
    reasons: Vec<String>,
}

impl Default for LeanctxPromotionEvidence {
    fn default() -> Self {
        Self {
            capability_version: None,
            capability_version_ok: false,
            protected_content_ok: false,
            fail_open_ok: false,
            shadow_contract_ok: false,
            live_promotion_allowed: false,
            status: "blocked".into(),
            reasons: vec!["promotion evidence is unavailable".into()],
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeanctxCapabilitiesResponse {
    version: Option<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    protected_content: Option<bool>,
    fail_open: Option<bool>,
    mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeanctxSidecarStatus {
    pub configured: bool,
    pub enabled: bool,
    pub running: bool,
    pub executable_present: bool,
    pub loopback_only: bool,
    pub base_url: Option<String>,
    pub version: Option<String>,
    pub mode: String,
    pub health: String,
    pub error: Option<String>,
    pub ownership: &'static str,
    pub live_request_routing: bool,
    pub promotion: LeanctxPromotionStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeanctxPromotionStatus {
    pub status: String,
    pub capability_version: Option<String>,
    pub capability_version_ok: bool,
    pub protected_content_ok: bool,
    pub fail_open_ok: bool,
    pub shadow_contract_ok: bool,
    pub live_promotion_allowed: bool,
    pub reasons: Vec<String>,
}

fn default_mode() -> String {
    "off".into()
}

fn receipt_path(manager: &ToolManager) -> PathBuf {
    manager.runtime.tools_dir.join("leanctx.json")
}

fn log_path(manager: &ToolManager) -> PathBuf {
    manager.runtime.logs_dir().join("leanctx-sidecar.log")
}

fn parse_loopback_url(raw: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(raw.trim()).context("LEANCTX_BASE_URL is not a valid URL")?;
    if url.scheme() != "http" {
        bail!("LEANCTX_BASE_URL must use http:// for the local guided sidecar");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("LEANCTX_BASE_URL must include a host"))?;
    let loopback = host == "localhost"
        || host
            .parse::<IpAddr>()
            .map(|address| address.is_loopback())
            .unwrap_or(false);
    if !loopback {
        bail!("LEANCTX_BASE_URL must point to localhost or a loopback IP");
    }
    Ok(url)
}

fn parse_args_json(raw: Option<String>) -> Result<Vec<String>> {
    let Some(raw) = raw.filter(|value| !value.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    let args: Vec<String> = serde_json::from_str(&raw)
        .context("LEANCTX_ARGS_JSON must be a JSON array of argument strings")?;
    if args.len() > 32 || args.iter().any(|arg| arg.len() > 2048) {
        bail!("LEANCTX_ARGS_JSON exceeds the bounded sidecar argument limits");
    }
    Ok(args)
}

fn config_from_environment() -> Result<LeanctxReceipt> {
    let executable = std::env::var_os("LEANCTX_EXECUTABLE")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Set LEANCTX_EXECUTABLE to the local leanctx executable before installing the guided profile."))?;
    if !executable.is_absolute() {
        bail!("LEANCTX_EXECUTABLE must be an absolute path");
    }
    if !executable.is_file() {
        bail!("LEANCTX_EXECUTABLE does not point to a file");
    }
    let base_url = std::env::var("LEANCTX_BASE_URL")
        .context("Set LEANCTX_BASE_URL to the local leanctx HTTP endpoint before installing the guided profile")?;
    parse_loopback_url(&base_url)?;
    let args = parse_args_json(std::env::var("LEANCTX_ARGS_JSON").ok())?;
    Ok(LeanctxReceipt {
        version: std::env::var("LEANCTX_VERSION")
            .unwrap_or_else(|_| LEANCTX_DISPLAY_VERSION.into()),
        executable,
        base_url,
        args,
        enabled: false,
        mode: "off".into(),
        last_health: None,
        last_error: None,
        promotion: LeanctxPromotionEvidence::default(),
    })
}

fn read_receipt(manager: &ToolManager) -> Option<LeanctxReceipt> {
    manager
        .read_tool_receipt("leanctx")
        .and_then(|value| serde_json::from_value(value).ok())
}

fn write_receipt(manager: &ToolManager, receipt: &LeanctxReceipt) -> Result<()> {
    manager.write_tool_receipt("leanctx", serde_json::to_value(receipt)?)
}

fn health_url(base_url: &str) -> Result<reqwest::Url> {
    let mut url = parse_loopback_url(base_url)?;
    url.set_path("/health");
    url.set_query(None);
    Ok(url)
}

fn probe_health(base_url: &str) -> Result<()> {
    let response = reqwest::blocking::Client::builder()
        .timeout(LEANCTX_HEALTH_TIMEOUT)
        .build()
        .context("building leanctx health client")?
        .get(health_url(base_url)?)
        .send()
        .context("checking leanctx /health")?;
    if !response.status().is_success() {
        bail!("leanctx /health returned HTTP {}", response.status());
    }
    Ok(())
}

fn capabilities_url(base_url: &str) -> Result<reqwest::Url> {
    let mut url = parse_loopback_url(base_url)?;
    url.set_path(LEANCTX_CAPABILITIES_PATH);
    url.set_query(None);
    Ok(url)
}

fn evaluate_promotion_capabilities(
    capabilities: LeanctxCapabilitiesResponse,
) -> LeanctxPromotionEvidence {
    let mut evidence = LeanctxPromotionEvidence {
        capability_version: capabilities.version.clone(),
        ..LeanctxPromotionEvidence::default()
    };
    evidence.capability_version_ok = capabilities
        .version
        .as_deref()
        .is_some_and(|version| !version.trim().is_empty())
        && capabilities
            .capabilities
            .iter()
            .any(|capability| capability == REQUIRED_LEANCTX_CAPABILITY);
    if !evidence.capability_version_ok {
        evidence.reasons.push(format!(
            "version and {REQUIRED_LEANCTX_CAPABILITY} capability evidence are required"
        ));
    }
    evidence.protected_content_ok = capabilities.protected_content == Some(true);
    if !evidence.protected_content_ok {
        evidence
            .reasons
            .push("protected-content handling evidence is required".into());
    }
    evidence.fail_open_ok = capabilities.fail_open == Some(true);
    if !evidence.fail_open_ok {
        evidence
            .reasons
            .push("fail-open evidence is required".into());
    }
    evidence.shadow_contract_ok = capabilities.mode.as_deref() == Some("shadow");
    if !evidence.shadow_contract_ok {
        evidence
            .reasons
            .push("sidecar must explicitly report shadow mode".into());
    }
    evidence.live_promotion_allowed = evidence.capability_version_ok
        && evidence.protected_content_ok
        && evidence.fail_open_ok
        && evidence.shadow_contract_ok;
    evidence.status = if evidence.live_promotion_allowed {
        "eligible-for-review".into()
    } else {
        "blocked".into()
    };
    evidence
}

fn probe_promotion_evidence(base_url: &str) -> LeanctxPromotionEvidence {
    let client = match reqwest::blocking::Client::builder()
        .timeout(LEANCTX_HEALTH_TIMEOUT)
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return LeanctxPromotionEvidence {
                reasons: vec![format!("capability client unavailable: {err}")],
                ..LeanctxPromotionEvidence::default()
            };
        }
    };
    let url = match capabilities_url(base_url) {
        Ok(url) => url,
        Err(err) => {
            return LeanctxPromotionEvidence {
                reasons: vec![format!("capability URL invalid: {err}")],
                ..LeanctxPromotionEvidence::default()
            };
        }
    };
    let response = match client.get(url).send() {
        Ok(response) if response.status().is_success() => response,
        Ok(response) => {
            return LeanctxPromotionEvidence {
                reasons: vec![format!(
                    "capability endpoint returned HTTP {}",
                    response.status()
                )],
                ..LeanctxPromotionEvidence::default()
            };
        }
        Err(err) => {
            return LeanctxPromotionEvidence {
                reasons: vec![format!("capability evidence unavailable: {err}")],
                ..LeanctxPromotionEvidence::default()
            };
        }
    };
    match response.json::<LeanctxCapabilitiesResponse>() {
        Ok(capabilities) => evaluate_promotion_capabilities(capabilities),
        Err(err) => LeanctxPromotionEvidence {
            reasons: vec![format!("capability evidence is not valid JSON: {err}")],
            ..LeanctxPromotionEvidence::default()
        },
    }
}

fn child_running(child: &mut Child) -> bool {
    match child.try_wait() {
        Ok(None) => true,
        Ok(Some(_)) | Err(_) => false,
    }
}

fn stop_child_process_group(child: &mut Child) {
    #[cfg(unix)]
    {
        let process_group = -(child.id() as libc::pid_t);
        // The sidecar is started in its own process group. Terminate the
        // complete group so a user-supplied wrapper cannot leave a descendant
        // listening after Disable, Uninstall, or app exit.
        unsafe {
            let _ = libc::kill(process_group, libc::SIGTERM);
        }
        for _ in 0..20 {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                Err(_) => break,
            }
        }
        unsafe {
            let _ = libc::kill(process_group, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

impl ToolManager {
    pub fn install_leanctx_sidecar(&self) -> Result<LeanctxSidecarStatus> {
        self.runtime.ensure_layout()?;
        let receipt = config_from_environment()?;
        write_receipt(self, &receipt)?;
        Ok(self.leanctx_sidecar_status())
    }

    pub fn set_leanctx_enabled(&self, enabled: bool) -> Result<LeanctxSidecarStatus> {
        let mut receipt = read_receipt(self).ok_or_else(|| {
            anyhow::anyhow!("Install the guided leanctx profile before enabling it")
        })?;
        if enabled {
            parse_loopback_url(&receipt.base_url)?;
            if !receipt.executable.is_file() {
                bail!("The configured leanctx executable is missing; repair the guided profile");
            }
            self.start_leanctx(&receipt)?;
            receipt.enabled = true;
            receipt.mode = "shadow".into();
            receipt.last_health = Some("healthy".into());
            receipt.last_error = None;
            receipt.promotion = probe_promotion_evidence(&receipt.base_url);
        } else {
            self.stop_leanctx();
            receipt.enabled = false;
            receipt.mode = "off".into();
            receipt.last_health = None;
            receipt.promotion = LeanctxPromotionEvidence {
                reasons: vec!["leanctx is disabled".into()],
                ..LeanctxPromotionEvidence::default()
            };
        }
        write_receipt(self, &receipt)?;
        Ok(self.leanctx_sidecar_status())
    }

    pub fn uninstall_leanctx_sidecar(&self) -> Result<LeanctxSidecarStatus> {
        self.stop_leanctx();
        let path = receipt_path(self);
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        }
        Ok(self.leanctx_sidecar_status())
    }

    pub fn stop_leanctx(&self) {
        let mut process = self.leanctx_process.lock();
        if let Some(mut child) = process.take() {
            stop_child_process_group(&mut child);
        }
    }

    pub fn leanctx_sidecar_status(&self) -> LeanctxSidecarStatus {
        let receipt = read_receipt(self);
        let mut process = self.leanctx_process.lock();
        let running = process.as_mut().map(child_running).unwrap_or(false);
        let executable_present = receipt
            .as_ref()
            .map(|config| config.executable.is_file())
            .unwrap_or(false);
        LeanctxSidecarStatus {
            configured: receipt.is_some(),
            enabled: receipt
                .as_ref()
                .map(|config| config.enabled)
                .unwrap_or(false),
            running,
            executable_present,
            loopback_only: receipt
                .as_ref()
                .and_then(|config| parse_loopback_url(&config.base_url).ok())
                .is_some(),
            base_url: receipt.as_ref().map(|config| config.base_url.clone()),
            version: receipt.as_ref().map(|config| config.version.clone()),
            mode: receipt
                .as_ref()
                .map(|config| config.mode.clone())
                .unwrap_or_else(|| "off".into()),
            health: receipt
                .as_ref()
                .and_then(|config| config.last_health.clone())
                .unwrap_or_else(|| "not-checked".into()),
            error: receipt
                .as_ref()
                .and_then(|config| config.last_error.clone()),
            ownership: "Switchboard-managed sidecar; Headroom remains the sole provider proxy",
            live_request_routing: false,
            promotion: LeanctxPromotionStatus {
                status: receipt
                    .as_ref()
                    .map(|config| config.promotion.status.clone())
                    .unwrap_or_else(|| "blocked".into()),
                capability_version: receipt
                    .as_ref()
                    .and_then(|config| config.promotion.capability_version.clone()),
                capability_version_ok: receipt
                    .as_ref()
                    .map(|config| config.promotion.capability_version_ok)
                    .unwrap_or(false),
                protected_content_ok: receipt
                    .as_ref()
                    .map(|config| config.promotion.protected_content_ok)
                    .unwrap_or(false),
                fail_open_ok: receipt
                    .as_ref()
                    .map(|config| config.promotion.fail_open_ok)
                    .unwrap_or(false),
                shadow_contract_ok: receipt
                    .as_ref()
                    .map(|config| config.promotion.shadow_contract_ok)
                    .unwrap_or(false),
                live_promotion_allowed: receipt
                    .as_ref()
                    .map(|config| config.promotion.live_promotion_allowed)
                    .unwrap_or(false),
                reasons: receipt
                    .as_ref()
                    .map(|config| config.promotion.reasons.clone())
                    .unwrap_or_else(|| vec!["leanctx is not configured".into()]),
            },
        }
    }

    fn start_leanctx(&self, receipt: &LeanctxReceipt) -> Result<()> {
        {
            let mut process = self.leanctx_process.lock();
            if let Some(child) = process.as_mut() {
                if child_running(child) {
                    drop(process);
                    return probe_health(&receipt.base_url);
                }
                *process = None;
            }
        }

        std::fs::create_dir_all(self.runtime.logs_dir())?;
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path(self))
            .context("opening leanctx sidecar log")?;
        let log_err = log.try_clone().context("cloning leanctx sidecar log")?;
        let mut command = Command::new(&receipt.executable);
        command
            .args(&receipt.args)
            .current_dir(&self.runtime.root_dir)
            .env("LEANCTX_SWITCHBOARD_MODE", "shadow")
            .env("LEANCTX_BASE_URL", &receipt.base_url)
            .env("PYTHONNOUSERSITE", "1")
            .env("PYTHONUNBUFFERED", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command.spawn().with_context(|| {
            format!(
                "starting leanctx sidecar at {}",
                receipt.executable.display()
            )
        })?;
        *self.leanctx_process.lock() = Some(child);

        let deadline = Instant::now() + LEANCTX_START_TIMEOUT;
        loop {
            match probe_health(&receipt.base_url) {
                Ok(()) => return Ok(()),
                Err(err) if Instant::now() < deadline => {
                    if !self
                        .leanctx_process
                        .lock()
                        .as_mut()
                        .map(child_running)
                        .unwrap_or(false)
                    {
                        self.stop_leanctx();
                        bail!("leanctx sidecar exited before health check: {err:#}");
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(err) => {
                    self.stop_leanctx();
                    bail!("leanctx sidecar did not become healthy: {err:#}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_url_rejects_remote_hosts() {
        assert!(parse_loopback_url("http://example.com:6791").is_err());
        assert!(parse_loopback_url("http://127.0.0.1:6791").is_ok());
    }

    #[test]
    fn args_are_json_and_bounded() {
        assert_eq!(parse_args_json(None).unwrap(), Vec::<String>::new());
        assert_eq!(
            parse_args_json(Some(r#"["serve","--shadow"]"#.into())).unwrap(),
            vec!["serve", "--shadow"]
        );
        assert!(parse_args_json(Some(r#"{"not":"an array"}"#.into())).is_err());
    }

    #[test]
    fn promotion_is_blocked_when_evidence_is_missing() {
        let evidence = evaluate_promotion_capabilities(LeanctxCapabilitiesResponse {
            version: None,
            capabilities: Vec::new(),
            protected_content: None,
            fail_open: None,
            mode: None,
        });
        assert_eq!(evidence.status, "blocked");
        assert!(!evidence.live_promotion_allowed);
        assert!(evidence
            .reasons
            .iter()
            .any(|reason| reason.contains("protected-content")));
    }

    #[test]
    fn complete_evidence_is_only_eligible_for_review() {
        let evidence = evaluate_promotion_capabilities(LeanctxCapabilitiesResponse {
            version: Some("0.1.0".into()),
            capabilities: vec![REQUIRED_LEANCTX_CAPABILITY.into()],
            protected_content: Some(true),
            fail_open: Some(true),
            mode: Some("shadow".into()),
        });
        assert_eq!(evidence.status, "eligible-for-review");
        assert!(evidence.live_promotion_allowed);
        assert!(evidence.shadow_contract_ok);
    }
}
