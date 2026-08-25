use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::client_connector_status::connector_has_complete_lifecycle_fixture;
use crate::client_connectors::{
    connector_manifest, manifest_config_locations, manifest_detection_sources,
};
use crate::client_detection::{
    detect_aider_client, detect_amazon_q_client, detect_claude_code_client, detect_codex_client,
    detect_continue_client, detect_gemini_cli_client, detect_goose_client,
    detect_grok_cli_client, detect_opencode_client, detect_qwen_code_client,
    detect_windsurf_client, detect_zed_ai_client,
};
use crate::client_footprint::get_managed_footprint;
use crate::client_setup_apply::{apply_client_setup, disable_client_setup, verify_client_setup};
use crate::client_setup_state::{is_configured, load_setup_state};
use crate::models::{ClientSetupVerification, ClientStatus, ManagedFootprintItem, SwitchboardMode};

pub const CODING_CLIENT_ADAPTER_CONTRACT_VERSION: u32 = 1;

/// Stable, connector-neutral lifecycle contract for coding clients.
///
/// Implementations intentionally delegate to the existing setup modules. This
/// keeps the safety-critical file editing and rollback behavior in one place
/// while callers migrate away from connector-specific booleans.
pub trait CodingClientAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn detect(&self) -> DetectionResult;
    fn plan(&self, mode: SwitchboardMode) -> Result<ConfigPlan>;
    fn apply(&self, plan: &ConfigPlan, consent: ConsentToken) -> Result<ApplyReceipt>;
    fn verify(&self) -> Result<VerificationReport>;
    fn rollback(&self, receipt: &ApplyReceipt) -> Result<RollbackReport>;
    fn cleanup_off_mode(&self) -> Result<CleanupReport>;
    fn footprint(&self) -> ManagedFootprint;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub contract_version: u32,
    pub client_id: String,
    pub installed: bool,
    pub configured: bool,
    pub sources: Vec<String>,
    pub evidence: Vec<String>,
    pub config_locations: Vec<String>,
    pub lifecycle_fixture_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigPlanAction {
    ApplyManagedRouting,
    CleanupManagedRouting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDiff {
    pub target: String,
    pub before: String,
    pub after: String,
    pub managed_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPlan {
    pub plan_id: String,
    pub client_id: String,
    pub mode: SwitchboardMode,
    pub action: ConfigPlanAction,
    pub diffs: Vec<ConfigDiff>,
    pub confirmation_phrase: String,
    pub reversible: bool,
    pub evidence: Vec<String>,
}

/// Proof that the exact dry-run plan was explicitly confirmed.
///
/// The fields remain private so callers cannot construct a token without
/// matching the plan's generated phrase.
#[derive(Debug, Clone)]
pub struct ConsentToken {
    client_id: String,
    plan_id: String,
}

impl ConsentToken {
    pub fn issue(plan: &ConfigPlan, confirmation_phrase: &str) -> Result<Self> {
        if confirmation_phrase != plan.confirmation_phrase {
            return Err(anyhow!(
                "Adapter consent phrase does not match the dry-run plan."
            ));
        }
        Ok(Self {
            client_id: plan.client_id.clone(),
            plan_id: plan.plan_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerificationReport {
    pub client_id: String,
    pub verified: bool,
    pub proxy_reachable: bool,
    pub checks: Vec<String>,
    pub failures: Vec<String>,
}

impl From<ClientSetupVerification> for VerificationReport {
    fn from(value: ClientSetupVerification) -> Self {
        Self {
            client_id: value.client_id,
            verified: value.verified,
            proxy_reachable: value.proxy_reachable,
            checks: value.checks,
            failures: value.failures,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReceipt {
    pub client_id: String,
    pub plan_id: String,
    pub applied_at: String,
    pub changed_files: Vec<String>,
    pub backup_files: Vec<String>,
    pub already_configured: bool,
    pub summary: String,
    pub verification: VerificationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RollbackReport {
    pub client_id: String,
    pub plan_id: String,
    pub rolled_back: bool,
    pub actions: Vec<String>,
    pub preserved_backups: Vec<String>,
    pub verification: VerificationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupReport {
    pub client_id: String,
    pub cleaned: bool,
    pub actions: Vec<String>,
    pub verification: VerificationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedFootprint {
    pub client_id: String,
    pub secret_values_included: bool,
    pub items: Vec<ManagedFootprintItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodingClientAdapterStatus {
    pub adapter_id: String,
    pub detection: DetectionResult,
    pub plan: ConfigPlan,
    pub verification: Option<VerificationReport>,
    pub footprint: ManagedFootprint,
}

#[derive(Debug, Clone, Copy)]
enum BuiltinAdapterKind {
    ClaudeCode,
    Codex,
    Gemini,
    Opencode,
    GrokCli,
    Aider,
    Continue,
    Goose,
    QwenCode,
    AmazonQ,
    Windsurf,
    ZedAi,
}

#[derive(Debug, Clone, Copy)]
struct ExistingCodingClientAdapter {
    kind: BuiltinAdapterKind,
}

impl ExistingCodingClientAdapter {
    fn canonical_id(self) -> &'static str {
        match self.kind {
            BuiltinAdapterKind::ClaudeCode => "claude_code",
            BuiltinAdapterKind::Codex => "codex",
            BuiltinAdapterKind::Gemini => "gemini_cli",
            BuiltinAdapterKind::Opencode => "opencode",
            BuiltinAdapterKind::GrokCli => "grok_cli",
            BuiltinAdapterKind::Aider => "aider",
            BuiltinAdapterKind::Continue => "continue",
            BuiltinAdapterKind::Goose => "goose",
            BuiltinAdapterKind::QwenCode => "qwen_code",
            BuiltinAdapterKind::AmazonQ => "amazon_q",
            BuiltinAdapterKind::Windsurf => "windsurf",
            BuiltinAdapterKind::ZedAi => "zed_ai",
        }
    }

    fn detect_status(self) -> ClientStatus {
        let state = load_setup_state();
        match self.kind {
            BuiltinAdapterKind::ClaudeCode => {
                detect_claude_code_client(is_configured(&state, "claude_code"))
            }
            BuiltinAdapterKind::Codex => detect_codex_client(is_configured(&state, "codex")),
            BuiltinAdapterKind::Gemini => {
                let mut status = detect_gemini_cli_client();
                status.configured = is_configured(&state, "gemini_cli");
                status
            }
            BuiltinAdapterKind::Opencode => {
                let mut status = detect_opencode_client();
                status.configured = is_configured(&state, "opencode");
                status
            }
            BuiltinAdapterKind::GrokCli => {
                let mut status = detect_grok_cli_client();
                status.configured = is_configured(&state, "grok_cli");
                status
            }
            BuiltinAdapterKind::Aider => {
                let mut status = detect_aider_client();
                status.configured = is_configured(&state, "aider");
                status
            }
            BuiltinAdapterKind::Continue => {
                let mut status = detect_continue_client();
                status.configured = is_configured(&state, "continue");
                status
            }
            BuiltinAdapterKind::Goose => {
                let mut status = detect_goose_client();
                status.configured = is_configured(&state, "goose");
                status
            }
            BuiltinAdapterKind::QwenCode => {
                let mut status = detect_qwen_code_client();
                status.configured = is_configured(&state, "qwen_code");
                status
            }
            BuiltinAdapterKind::AmazonQ => {
                let mut status = detect_amazon_q_client();
                status.configured = is_configured(&state, "amazon_q");
                status
            }
            BuiltinAdapterKind::Windsurf => {
                let mut status = detect_windsurf_client();
                status.configured = is_configured(&state, "windsurf");
                status
            }
            BuiltinAdapterKind::ZedAi => {
                let mut status = detect_zed_ai_client();
                status.configured = is_configured(&state, "zed_ai");
                status
            }
        }
    }

    fn plan_diffs(self, action: &ConfigPlanAction) -> Vec<ConfigDiff> {
        let cleanup = matches!(action, ConfigPlanAction::CleanupManagedRouting);
        let (targets, managed_boundary) = match self.kind {
            BuiltinAdapterKind::ClaudeCode => (
                vec!["shell profile managed blocks", "~/.claude/settings.json"],
                "Switchboard-owned Claude environment and hook entries",
            ),
            BuiltinAdapterKind::Codex => (
                vec!["shell profile managed blocks", "~/.codex/config.toml"],
                "Switchboard-owned Codex shell and provider blocks",
            ),
            BuiltinAdapterKind::Gemini => (
                vec![
                    "shell profile managed blocks",
                    "Gemini routing-intent sidecar",
                ],
                "Switchboard-owned Gemini shell and sidecar blocks",
            ),
            BuiltinAdapterKind::Opencode => (
                vec![
                    "~/.config/opencode/opencode.json managed provider routing",
                    "OpenCode routing-intent sidecar",
                ],
                "Switchboard-owned OpenCode provider routing; credentials, account state, and model selection remain manual.",
            ),
            BuiltinAdapterKind::GrokCli => (
                vec![
                    "~/.grok/config.toml [endpoints].models_base_url",
                    "Grok / xAI routing-intent sidecar",
                ],
                "Switchboard manages only the documented non-secret models_base_url endpoint field; XAI_API_KEY or Grok login authentication, account state, and model selection remain manual.",
            ),
            BuiltinAdapterKind::Aider => (
                vec![
                    "~/.aider.conf.yml allowlisted openai-api-base field",
                    "Aider routing-intent sidecar",
                ],
                "Switchboard sets only the allowlisted openai-api-base provider field; API keys, set-env, wrapper/env setup, and model selection remain manual.",
            ),
            BuiltinAdapterKind::Continue => (
                vec![
                    "~/.continue/config.yaml allowlisted managed model entry",
                    "Continue routing-intent sidecar",
                ],
                "Switchboard owns one config.yaml model routing entry; provider credentials and unrelated model settings remain manual.",
            ),
            BuiltinAdapterKind::Goose => (
                vec![
                    "Goose config.yaml allowlisted OpenAI/Anthropic endpoint fields",
                    "read-only Repo Memory MCP bridge descriptor",
                    "Goose routing-intent sidecar",
                ],
                "Switchboard owns only the documented allowlisted endpoint fields and the read-only Repo Memory MCP bridge; Goose credentials, account state, and model selection remain manual.",
            ),
            BuiltinAdapterKind::QwenCode => (
                vec!["Qwen Code routing-intent sidecar"],
                "Switchboard owns only its routing-intent sidecar; Qwen account and model setup remains manual.",
            ),
            BuiltinAdapterKind::AmazonQ => (
                vec!["Amazon Q Developer CLI routing-intent sidecar"],
                "Switchboard owns only its routing-intent sidecar; AWS auth/provider/workspace state remains manual.",
            ),
            BuiltinAdapterKind::Windsurf => (
                vec!["Windsurf User settings.json managed routing"],
                "Switchboard-owned Windsurf editor settings routing; provider credentials, account state, and model selection remain manual.",
            ),
            BuiltinAdapterKind::ZedAi => (
                vec!["Zed settings.json managed assistant routing"],
                "Switchboard-owned Zed assistant settings routing; provider credentials, account state, and model selection remain manual.",
            ),
        };
        targets
            .into_iter()
            .map(|target| ConfigDiff {
                target: target.to_string(),
                before: "User configuration preserved; secret values are not read for this plan."
                    .to_string(),
                after: if cleanup {
                    "Switchboard-managed routing removed; unrelated configuration preserved."
                        .to_string()
                } else {
                    "Switchboard-managed localhost routing present after apply.".to_string()
                },
                managed_boundary: managed_boundary.to_string(),
            })
            .collect()
    }
}

impl CodingClientAdapter for ExistingCodingClientAdapter {
    fn id(&self) -> &'static str {
        self.canonical_id()
    }

    fn detect(&self) -> DetectionResult {
        detection_from_status(self.canonical_id(), &self.detect_status())
    }

    fn plan(&self, mode: SwitchboardMode) -> Result<ConfigPlan> {
        let action = if matches!(mode, SwitchboardMode::Headroom | SwitchboardMode::Full) {
            ConfigPlanAction::ApplyManagedRouting
        } else {
            ConfigPlanAction::CleanupManagedRouting
        };
        let diffs = self.plan_diffs(&action);
        let fingerprint_input = format!("{}|{:?}|{:?}", self.id(), mode, diffs);
        let digest = Sha256::digest(fingerprint_input.as_bytes());
        let plan_id = format!("{}-{:x}", self.id(), digest)[..self.id().len() + 13].to_string();
        let confirmation_phrase = format!("APPLY ADAPTER PLAN {plan_id}");
        Ok(ConfigPlan {
            plan_id,
            client_id: self.id().to_string(),
            mode,
            action,
            diffs,
            confirmation_phrase,
            reversible: true,
            evidence: vec![
                "Dry-run planning does not write files or read provider credentials.".to_string(),
                "Existing connector setup creates backups before managed edits where prior files exist."
                    .to_string(),
                "Rollback and Off cleanup delegate to the existing tested connector lifecycle."
                    .to_string(),
            ],
        })
    }

    fn apply(&self, plan: &ConfigPlan, consent: ConsentToken) -> Result<ApplyReceipt> {
        validate_plan_and_consent(self.id(), plan, &consent)?;
        if plan.action != ConfigPlanAction::ApplyManagedRouting {
            return Err(anyhow!(
                "Cleanup plans must use cleanup_off_mode instead of apply."
            ));
        }
        let result = apply_client_setup(self.id())?;
        Ok(ApplyReceipt {
            client_id: self.id().to_string(),
            plan_id: plan.plan_id.clone(),
            applied_at: Utc::now().to_rfc3339(),
            changed_files: result.changed_files,
            backup_files: result.backup_files,
            already_configured: result.already_configured,
            summary: result.summary,
            verification: result.verification.into(),
        })
    }

    fn verify(&self) -> Result<VerificationReport> {
        Ok(verify_client_setup(self.id())?.into())
    }

    fn rollback(&self, receipt: &ApplyReceipt) -> Result<RollbackReport> {
        if receipt.client_id != self.id() {
            return Err(anyhow!("Apply receipt belongs to a different adapter."));
        }
        if receipt.already_configured || receipt.changed_files.is_empty() {
            return Ok(RollbackReport {
                client_id: self.id().to_string(),
                plan_id: receipt.plan_id.clone(),
                rolled_back: true,
                actions: vec![
                    "Apply made no changes, so rollback preserved the pre-existing managed state."
                        .to_string(),
                ],
                preserved_backups: receipt.backup_files.clone(),
                verification: self.verify()?,
            });
        }

        disable_client_setup(self.id())?;
        let verification = self.verify()?;
        if verification.verified {
            return Err(anyhow!(
                "Adapter rollback completed without an error, but managed routing still verifies."
            ));
        }
        Ok(RollbackReport {
            client_id: self.id().to_string(),
            plan_id: receipt.plan_id.clone(),
            rolled_back: true,
            actions: vec![
                "Removed only Switchboard-owned routing through the existing connector rollback path."
                    .to_string(),
                "Unrelated client settings and provider credentials were preserved.".to_string(),
            ],
            preserved_backups: receipt.backup_files.clone(),
            verification,
        })
    }

    fn cleanup_off_mode(&self) -> Result<CleanupReport> {
        disable_client_setup(self.id())?;
        let verification = self.verify()?;
        if verification.verified {
            return Err(anyhow!(
                "Off-mode cleanup completed without an error, but managed routing still verifies."
            ));
        }
        Ok(CleanupReport {
            client_id: self.id().to_string(),
            cleaned: true,
            actions: vec![
                "Removed Switchboard-owned routing through the existing Off-mode cleanup path."
                    .to_string(),
            ],
            verification,
        })
    }

    fn footprint(&self) -> ManagedFootprint {
        let items = get_managed_footprint()
            .items
            .into_iter()
            .filter(|item| footprint_item_matches(self.id(), &item.id))
            .collect();
        ManagedFootprint {
            client_id: self.id().to_string(),
            secret_values_included: false,
            items,
        }
    }
}

pub(crate) fn validate_plan_and_consent(
    adapter_id: &str,
    plan: &ConfigPlan,
    consent: &ConsentToken,
) -> Result<()> {
    if plan.client_id != adapter_id || consent.client_id != adapter_id {
        return Err(anyhow!(
            "Adapter plan or consent belongs to a different client."
        ));
    }
    if consent.plan_id != plan.plan_id {
        return Err(anyhow!("Adapter consent does not match this dry-run plan."));
    }
    Ok(())
}

fn footprint_item_matches(client_id: &str, item_id: &str) -> bool {
    item_id.starts_with("shell-")
        || match client_id {
            "claude_code" => item_id.starts_with("claude-"),
            "codex" => item_id.starts_with("codex-"),
            "gemini_cli" => item_id.starts_with("gemini_cli-"),
            "opencode" => item_id.starts_with("opencode-"),
            "grok_cli" => item_id.starts_with("grok_cli-"),
            "aider" => item_id.starts_with("aider-"),
            "continue" => item_id.starts_with("continue-"),
            "goose" => item_id.starts_with("goose-"),
            "qwen_code" => item_id.starts_with("qwen_code-"),
            "amazon_q" => item_id.starts_with("amazon_q-"),
            "windsurf" => item_id.starts_with("windsurf-"),
            "zed_ai" => item_id.starts_with("zed_ai-"),
            _ => false,
        }
}

fn detection_from_status(client_id: &str, status: &ClientStatus) -> DetectionResult {
    let manifest = connector_manifest(client_id);
    DetectionResult {
        contract_version: CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
        client_id: client_id.to_string(),
        installed: status.installed,
        configured: status.configured,
        sources: manifest
            .as_ref()
            .map(manifest_detection_sources)
            .unwrap_or_default(),
        evidence: status.notes.clone(),
        config_locations: manifest_config_locations(manifest.as_ref()),
        lifecycle_fixture_complete: connector_has_complete_lifecycle_fixture(client_id),
    }
}

pub fn coding_client_adapter(client_id: &str) -> Option<Box<dyn CodingClientAdapter>> {
    if matches!(client_id, "deepseek_harness" | "dsh") {
        return Some(Box::new(
            crate::deepseek_harness::DeepSeekHarnessAdapter::default(),
        ));
    }
    let kind = match client_id {
        "claude_code" => BuiltinAdapterKind::ClaudeCode,
        "codex" | "codex_cli" => BuiltinAdapterKind::Codex,
        "gemini_cli" => BuiltinAdapterKind::Gemini,
        "opencode" => BuiltinAdapterKind::Opencode,
        "grok_cli" => BuiltinAdapterKind::GrokCli,
        "aider" => BuiltinAdapterKind::Aider,
        "continue" => BuiltinAdapterKind::Continue,
        "goose" => BuiltinAdapterKind::Goose,
        "qwen_code" => BuiltinAdapterKind::QwenCode,
        "amazon_q" => BuiltinAdapterKind::AmazonQ,
        "windsurf" => BuiltinAdapterKind::Windsurf,
        "zed_ai" => BuiltinAdapterKind::ZedAi,
        _ => return None,
    };
    Some(Box::new(ExistingCodingClientAdapter { kind }))
}

pub fn coding_client_adapter_for_version(
    client_id: &str,
    contract_version: u32,
) -> Result<Box<dyn CodingClientAdapter>> {
    if contract_version != CODING_CLIENT_ADAPTER_CONTRACT_VERSION {
        return Err(anyhow!(
            "Unsupported CodingClientAdapter contract version {contract_version}; expected {CODING_CLIENT_ADAPTER_CONTRACT_VERSION}."
        ));
    }
    coding_client_adapter(client_id)
        .ok_or_else(|| anyhow!("No CodingClientAdapter is registered for {client_id}."))
}

/// Build the structured lifecycle state used by connector listing without
/// repeating environment detection or changing any files.
pub fn adapter_status_for_listing(
    client_id: &str,
    detected: &ClientStatus,
    configured: bool,
) -> Result<Option<CodingClientAdapterStatus>> {
    let Some(adapter) = coding_client_adapter(client_id) else {
        return Ok(None);
    };
    let mut detected = detected.clone();
    detected.configured = configured;
    let detection = detection_from_status(adapter.id(), &detected);
    let verification = if configured {
        Some(adapter.verify().unwrap_or_else(|error| VerificationReport {
            client_id: adapter.id().to_string(),
            verified: false,
            proxy_reachable: false,
            checks: Vec::new(),
            failures: vec![format!("Adapter verification could not complete: {error}")],
        }))
    } else {
        None
    };
    Ok(Some(CodingClientAdapterStatus {
        adapter_id: adapter.id().to_string(),
        detection,
        plan: adapter.plan(SwitchboardMode::Full)?,
        verification,
        footprint: adapter.footprint(),
    }))
}

#[cfg(test)]
#[path = "client_adapter_contract_tests.rs"]
mod tests;
