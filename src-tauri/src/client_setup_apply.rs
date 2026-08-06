use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::client_connector_status::MANAGED_CLIENT_SPECS;
use crate::client_connectors::{
    planned_connector_has_implemented_setup, planned_sidecar_spec, PlannedSidecarSpec,
    PLANNED_SIDECAR_SPECS,
};
use crate::client_detection::codex_home;
use crate::client_paths::{
    all_shell_paths, claude_settings_candidates, claude_settings_path, codex_config_toml_path,
    grok_config_path, headroom_rtk_hook_path, home_dir, opencode_config_path,
    planned_sidecar_routing_path, rtk_codex_agents_path, serialize_paths, windsurf_config_path,
    zed_config_path, SWITCHBOARD_ROUTING_FILE,
};
use crate::client_provider_configs::{
    configure_grok_provider_config, configure_opencode_provider_config,
    configure_windsurf_provider_config, configure_zed_provider_config,
    grok_provider_config_matches, opencode_provider_config_matches,
    remove_grok_provider_config, remove_opencode_provider_config,
    remove_windsurf_provider_config, remove_zed_provider_config,
    windsurf_provider_config_matches, zed_provider_config_matches,
    GROK_MARKER_PREFIX, HEADROOM_ANTHROPIC_BASE_URL, HEADROOM_OPENAI_BASE_URL,
    OPENCODE_HEADROOM_PROVIDER_ID,
};
use crate::client_codex_setup::{
    configure_codex_provider_block, codex_provider_block_matches, disable_codex_cli,
    disable_codex_gui, CODEX_ROOT_BLOCK_ID,
};
use crate::client_setup_state::{
    default_headroom_managed_python_path, default_headroom_rtk_path, is_codex_enabled,
    load_setup_state, normalized_setup_id, resolve_client_shell_targets,
    resolve_client_shell_targets_for_cleanup, write_setup_state, ClientSetupState,
};
use crate::managed_files::{
    backup_if_exists, managed_block_updated_content, managed_marker_end, managed_marker_start,
    marker_block_contains, parse_json_object, remove_managed_block, remove_shell_block,
    strip_marker_block, upsert_managed_block, write_file_if_changed,
};
use crate::models::{
    ClientSetupResult, ClientSetupVerification, ManagedConfigApplyPreview,
    ManagedConfigApplyResult, ManagedRollbackExecutionStatus,
};

// Raw proxy base — use provider-specific constants below when configuring client endpoints.
const HEADROOM_PROXY_URL: &str = "http://127.0.0.1:6767";
pub(crate) const GEMINI_BASE_URL_ENV_KEY: &str = "GOOGLE_GEMINI_BASE_URL";
const GEMINI_COMPAT_BASE_URL_ENV_KEY: &str = "GEMINI_BASE_URL";
const GEMINI_API_KEY_ENV_KEY: &str = "GEMINI_API_KEY";
const GEMINI_HEADROOM_API_KEY_VALUE: &str = "headroom-local";
pub(crate) const CURSOR_MARKER_PREFIX: &str = "headroom:cursor";
pub(crate) const CURSOR_SIDECAR_APPLY_RECORD_ID: &str = "cursor-sidecar-routing";
pub(crate) const CURSOR_SIDECAR_OWNER: &str = "Cursor routing sidecar";
pub(crate) const GOOSE_SIDECAR_APPLY_RECORD_ID: &str = "goose-sidecar-routing";
pub(crate) const GOOSE_SIDECAR_OWNER: &str = "Goose routing-intent sidecar";
pub(crate) const GROK_SIDECAR_APPLY_RECORD_ID: &str = "grok-sidecar-routing";
pub(crate) const GROK_SIDECAR_OWNER: &str = "Grok / xAI CLI routing-intent sidecar";
const MARKER_PREFIX: &str = "headroom";

pub fn apply_client_setup(client_id: &str) -> Result<ClientSetupResult> {
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();
    let mut state = load_setup_state();
    let state_id = normalized_setup_id(client_id).to_string();

    match client_id {
        "claude_code" => {
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let mut rtk_updates = ensure_rtk_integrations_for_targets(
                &default_headroom_rtk_path(),
                &default_headroom_managed_python_path(),
                &shell_targets,
            )?;
            let env_block = format!("export ANTHROPIC_BASE_URL={}", HEADROOM_ANTHROPIC_BASE_URL);
            let mut updates = configure_shell_block(&shell_targets, "claude_code", &env_block)?;
            let mut claude_updates =
                configure_claude_settings_env("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
            let mut legacy_updates = remove_legacy_vscode_base_url_keys()?;
            updates.0.append(&mut rtk_updates.0);
            updates.1.append(&mut rtk_updates.1);
            updates.0.append(&mut claude_updates.0);
            updates.1.append(&mut claude_updates.1);
            updates.0.append(&mut legacy_updates.0);
            updates.1.append(&mut legacy_updates.1);
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
            state
                .managed_shell_files
                .insert(state_id.clone(), serialize_paths(&shell_targets));
        }
        "vscode" => {
            let updates = configure_vscode_settings()?;
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        "codex" | "codex_cli" => {
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let env_block = format!("export OPENAI_BASE_URL={}", HEADROOM_OPENAI_BASE_URL);
            let mut updates = configure_shell_block(&shell_targets, "codex_cli", &env_block)?;
            let mut toml_updates = configure_codex_provider_block()?;
            updates.0.append(&mut toml_updates.0);
            updates.1.append(&mut toml_updates.1);
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
            state
                .managed_shell_files
                .insert(state_id.clone(), serialize_paths(&shell_targets));
            // Pull existing native threads into the headroom-provider menu so the
            // Codex history list stays whole once it routes through Headroom.
            let _ = crate::codex_threads::retag_codex_thread_providers("openai", "headroom");
        }
        "gemini_cli" => {
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let env_block = format!(
                "export {GEMINI_BASE_URL_ENV_KEY}={HEADROOM_PROXY_URL}\nexport {GEMINI_COMPAT_BASE_URL_ENV_KEY}={HEADROOM_PROXY_URL}\nexport {GEMINI_API_KEY_ENV_KEY}={GEMINI_HEADROOM_API_KEY_VALUE}"
            );
            let mut updates = configure_shell_block(&shell_targets, "gemini_cli", &env_block)?;
            let (changed, backup) = configure_planned_switchboard_sidecar(client_id)?;
            if changed {
                updates.0.push(
                    planned_sidecar_routing_path(client_id)?
                        .display()
                        .to_string(),
                );
            }
            if let Some(backup) = backup {
                updates.1.push(backup.display().to_string());
            }
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
            state
                .managed_shell_files
                .insert(state_id.clone(), serialize_paths(&shell_targets));
        }
        "opencode" => {
            let mut updates = configure_opencode_provider_config()?;
            let (changed, backup) = configure_planned_switchboard_sidecar(client_id)?;
            if changed {
                updates.0.push(
                    planned_sidecar_routing_path(client_id)?
                        .display()
                        .to_string(),
                );
            }
            if let Some(backup) = backup {
                updates.1.push(backup.display().to_string());
            }
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        "grok_cli" => {
            let mut updates = configure_grok_provider_config()?;
            let (sidecar_changed, sidecar_backup) =
                configure_planned_switchboard_sidecar(client_id)?;
            if sidecar_changed {
                updates.0.push(
                    planned_sidecar_routing_path(client_id)?
                        .display()
                        .to_string(),
                );
            }
            if let Some(backup) = sidecar_backup {
                updates.1.push(backup.display().to_string());
            }
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        "goose" => {
            let (changed, backups) =
                crate::goose_provider_configs::configure_goose_provider_config()?;
            changed_files.extend(changed);
            backup_files.extend(backups);
            let (sidecar_changed, sidecar_backup) =
                configure_planned_switchboard_sidecar(client_id)?;
            if sidecar_changed {
                changed_files.push(
                    planned_sidecar_routing_path(client_id)?
                        .display()
                        .to_string(),
                );
            }
            if let Some(backup) = sidecar_backup {
                backup_files.push(backup.display().to_string());
            }
        }
        "windsurf" => {
            let updates = configure_windsurf_provider_config()?;
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        "zed_ai" => {
            let updates = configure_zed_provider_config()?;
            changed_files.extend(updates.0);
            backup_files.extend(updates.1);
        }
        other if planned_sidecar_spec(other).is_some() => {
            if !planned_connector_has_implemented_setup(other) {
                return Err(anyhow!(
                    "Automatic setup is not supported yet for {other}. Use the guided workflow until backup, verify, rollback, and Off mode coverage are promoted."
                ));
            }
            let (changed, backup) = configure_planned_switchboard_sidecar(other)?;
            if changed {
                changed_files.push(planned_sidecar_routing_path(other)?.display().to_string());
            }
            if let Some(backup) = backup {
                backup_files.push(backup.display().to_string());
            }
        }
        other => return Err(anyhow!("Automatic setup is not supported yet for {other}.",)),
    }

    let configured_at = Utc::now().to_rfc3339();
    state.configured_clients.insert(state_id, configured_at);
    write_setup_state(&state)?;

    let already_configured = changed_files.is_empty();
    let summary = if let Some(sidecar) = planned_sidecar_spec(client_id) {
        if sidecar.id == "goose" && already_configured {
            "Goose provider routing and Repo Memory MCP bridge were already configured.".to_string()
        } else if sidecar.id == "goose" {
            "Goose provider routing and Repo Memory MCP bridge were configured; credentials and account state remain manual."
                .to_string()
        } else if sidecar.id == "grok_cli" && already_configured {
            "Grok / xAI native endpoint routing and Switchboard sidecar were already present."
                .to_string()
        } else if sidecar.id == "grok_cli" {
            "Grok / xAI native endpoint routing and Switchboard sidecar were written; credentials, account, and model selection remain manual.".to_string()
        } else if already_configured {
            format!("{} Switchboard sidecar was already present.", sidecar.name)
        } else {
            format!(
                "{} Switchboard sidecar written for reversible routing intent.",
                sidecar.name
            )
        }
    } else if already_configured {
        "Client was already configured for Headroom.".to_string()
    } else {
        "Client configuration updated to route through Headroom.".to_string()
    };

    let verification = verify_client_setup(client_id)?;

    Ok(ClientSetupResult {
        client_id: client_id.to_string(),
        applied: true,
        already_configured,
        summary,
        changed_files,
        backup_files,
        next_steps: client_setup_next_steps(client_id),
        verification,
    })
}
fn client_setup_next_steps(client_id: &str) -> Vec<String> {
    if normalized_setup_id(client_id) == "goose" {
        return vec![
            "Prepare the Repo Memory MCP handoff from Mode Inspector.".into(),
            "Keep Goose credentials, account state, and model selection configured manually; only the allowlisted provider endpoint fields are managed.".into(),
        ];
    }

    if normalized_setup_id(client_id) == "grok_cli" {
        return vec![
            "Keep XAI_API_KEY or Grok login authentication configured manually; Switchboard never stores credentials.".into(),
            "Run one Grok prompt and verify activity appears in Headroom.".into(),
        ];
    }

    vec![
        "Restart your terminal/editor session to pick up environment changes.".into(),
        format!(
            "Run one {} prompt and verify activity appears in Headroom.",
            match normalized_setup_id(client_id) {
                "codex_cli" => "Codex",
                "gemini_cli" => "Gemini CLI",
                "opencode" => "OpenCode",
                "cursor" => "Cursor",
                "grok_cli" => "Grok / xAI CLI",
                "aider" => "Aider",
                "continue" => "Continue",
                "qwen_code" => "Qwen Code",
                "amazon_q" => "Amazon Q Developer CLI",
                "windsurf" => "Windsurf",
                "zed_ai" => "Zed AI",
                _ => "Claude Code",
            }
        ),
    ]
}
pub fn verify_client_setup(client_id: &str) -> Result<ClientSetupVerification> {
    let mut checks = Vec::new();
    let mut failures = Vec::new();

    match client_id {
        "claude_code" => {
            let state = load_setup_state();
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let shell_ok = shell_block_contains_in_files(
                &shell_targets,
                "claude_code",
                "ANTHROPIC_BASE_URL",
                HEADROOM_ANTHROPIC_BASE_URL,
            )?;
            let rtk_path_ok =
                shell_block_contains_text_in_files(&shell_targets, "managed_rtk", "export PATH=")?;
            let claude_settings_ok =
                claude_settings_env_matches("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
            let rtk_hook_ok = claude_settings_hook_matches("headroom-rtk-rewrite.sh")?
                && headroom_rtk_hook_path().exists();

            if shell_ok {
                checks.push(
                    "Found Claude Code ANTHROPIC_BASE_URL export in managed shell block.".into(),
                );
            }
            if rtk_path_ok {
                checks.push("Found Headroom-managed RTK PATH export in shell profiles.".into());
            }
            if claude_settings_ok {
                checks.push(
                    "Found ~/.claude/settings.json env.ANTHROPIC_BASE_URL pointing to Headroom."
                        .into(),
                );
            }
            if rtk_hook_ok {
                checks.push(
                    "Found Headroom-managed RTK Claude hook in ~/.claude/settings.json.".into(),
                );
            }
            if !shell_ok && !claude_settings_ok {
                failures.push(
                    "Claude Code ANTHROPIC_BASE_URL was not found in shell blocks or ~/.claude/settings.json."
                        .into(),
                );
            }
            // RTK is a separate, opt-in integration (`set_rtk_enabled` tears it
            // down without touching ANTHROPIC_BASE_URL routing). Its wiring is
            // only ever added when the managed binary exists on disk (see
            // `ensure_rtk_integrations_for_targets`), so its absence must not
            // fail Claude Code verification when RTK isn't installed or the user
            // disabled it — routing is what "connected" means here.
            let rtk_required = !state.rtk_disabled && default_headroom_rtk_path().exists();
            if rtk_required && !rtk_path_ok {
                failures.push(
                    "Headroom-managed RTK PATH export was not found in shell profiles.".into(),
                );
            }
            if rtk_required && !rtk_hook_ok {
                failures.push(
                    "Headroom-managed RTK Claude hook was not found in ~/.claude/settings.json."
                        .into(),
                );
            }
        }
        "vscode" => {
            let mut delegated = verify_client_setup("claude_code")?;
            delegated.client_id = "vscode".to_string();
            return Ok(delegated);
        }
        "codex" | "codex_cli" => {
            let state = load_setup_state();
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let shell_ok = shell_block_contains_in_files(
                &shell_targets,
                "codex_cli",
                "OPENAI_BASE_URL",
                HEADROOM_OPENAI_BASE_URL,
            )?;
            let toml_ok = codex_provider_block_matches()?;

            if shell_ok {
                checks.push("Found Codex OPENAI_BASE_URL export in managed shell block.".into());
            }
            if toml_ok {
                checks
                    .push("Found Headroom-managed provider block in ~/.codex/config.toml.".into());
            }
            if toml_ok && !shell_ok {
                checks.push(
                    "Codex shell OPENAI_BASE_URL export was not found; config.toml provider routing is active."
                        .into(),
                );
            }
            if !toml_ok {
                failures.push(
                    "Headroom-managed provider block was not found in ~/.codex/config.toml.".into(),
                );
            }
            if !shell_ok && !toml_ok {
                failures
                    .push("Codex OPENAI_BASE_URL export was not found in shell profiles.".into());
            }
        }
        "goose" => {
            if crate::goose_provider_configs::goose_provider_config_matches()? {
                checks.push(
                    "Found Switchboard-managed Goose provider endpoint configuration.".into(),
                );
            } else {
                failures.push("Switchboard-managed Goose provider endpoint configuration was not found or does not match.".into());
            }
            if planned_switchboard_sidecar_matches(client_id)? {
                checks.push(
                    "Found Switchboard-managed Goose Repo Memory MCP bridge metadata.".into(),
                );
            }
        }
        "gemini_cli" => {
            let state = load_setup_state();
            let shell_targets = resolve_client_shell_targets(&state, client_id)?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {client_id}"))?;
            let sidecar_path = planned_sidecar_routing_path(client_id)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(client_id)?;
            let google_base_ok = shell_block_contains_in_files(
                &shell_targets,
                "gemini_cli",
                GEMINI_BASE_URL_ENV_KEY,
                HEADROOM_PROXY_URL,
            )?;
            let compat_base_ok = shell_block_contains_in_files(
                &shell_targets,
                "gemini_cli",
                GEMINI_COMPAT_BASE_URL_ENV_KEY,
                HEADROOM_PROXY_URL,
            )?;
            let api_key_ok = shell_block_contains_in_files(
                &shell_targets,
                "gemini_cli",
                GEMINI_API_KEY_ENV_KEY,
                GEMINI_HEADROOM_API_KEY_VALUE,
            )?;

            if sidecar_ok {
                checks.push(format!(
                    "Found Switchboard-managed {} sidecar at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            } else {
                failures.push(format!(
                    "Switchboard-managed {} sidecar was not found at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            }
            if google_base_ok {
                checks.push(format!(
                    "Found Gemini {} export pointing to Headroom.",
                    GEMINI_BASE_URL_ENV_KEY
                ));
            } else {
                failures.push(format!(
                    "Gemini {} export was not found in shell profiles.",
                    GEMINI_BASE_URL_ENV_KEY
                ));
            }
            if compat_base_ok {
                checks.push(format!(
                    "Found Gemini compatibility {} export pointing to Headroom.",
                    GEMINI_COMPAT_BASE_URL_ENV_KEY
                ));
            } else {
                failures.push(format!(
                    "Gemini compatibility {} export was not found in shell profiles.",
                    GEMINI_COMPAT_BASE_URL_ENV_KEY
                ));
            }
            if api_key_ok {
                checks.push(format!(
                    "Found Gemini {} export for local Headroom proxy auth.",
                    GEMINI_API_KEY_ENV_KEY
                ));
            } else {
                failures.push(format!(
                    "Gemini {} export was not found in shell profiles.",
                    GEMINI_API_KEY_ENV_KEY
                ));
            }
        }
        "opencode" => {
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {client_id}"))?;
            let sidecar_path = planned_sidecar_routing_path(client_id)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(client_id)?;
            let provider_ok = opencode_provider_config_matches()?;

            if sidecar_ok {
                checks.push(format!(
                    "Found Switchboard-managed {} sidecar at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            } else {
                failures.push(format!(
                    "Switchboard-managed {} sidecar was not found at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            }
            if provider_ok {
                checks.push(format!(
                    "Found OpenCode provider {} pointing to Headroom.",
                    OPENCODE_HEADROOM_PROVIDER_ID
                ));
            } else {
                failures.push(format!(
                    "OpenCode provider {} was not found in {}.",
                    OPENCODE_HEADROOM_PROVIDER_ID,
                    opencode_config_path().display()
                ));
            }
        }
        "grok_cli" => {
            let provider_ok = grok_provider_config_matches()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {client_id}"))?;
            let sidecar_path = planned_sidecar_routing_path(client_id)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(client_id)?;
            if provider_ok {
                checks.push(format!(
                    "Found Grok / xAI native endpoint routing in {} pointing to Headroom.",
                    grok_config_path().display()
                ));
                checks.push(
                    "Grok provider, model, account, and credential values remain user-controlled; Switchboard manages only [endpoints].models_base_url.".into(),
                );
            } else {
                failures.push(format!(
                    "Switchboard-managed Grok endpoint routing was not found in {}.",
                    grok_config_path().display()
                ));
            }
            if sidecar_ok {
                checks.push(format!(
                    "Found Switchboard-managed {} sidecar at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            } else {
                failures.push(format!(
                    "Switchboard-managed {} sidecar was not found at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            }
        }
        "windsurf" => {
            let provider_ok = windsurf_provider_config_matches()?;
            if provider_ok {
                checks.push(format!(
                    "Found Windsurf managed routing config in {}.",
                    windsurf_config_path().display()
                ));
            } else {
                failures.push(format!(
                    "Windsurf managed routing config was not found in {}.",
                    windsurf_config_path().display()
                ));
            }
        }
        "zed_ai" => {
            let provider_ok = zed_provider_config_matches()?;
            if provider_ok {
                checks.push(format!(
                    "Found Zed managed routing config in {}.",
                    zed_config_path().display()
                ));
            } else {
                failures.push(format!(
                    "Zed managed routing config was not found in {}.",
                    zed_config_path().display()
                ));
            }
        }
        other if planned_sidecar_spec(other).is_some() => {
            let sidecar = planned_sidecar_spec(other)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {other}"))?;
            let sidecar_path = planned_sidecar_routing_path(other)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(other)?;

            if sidecar_ok {
                checks.push(format!(
                    "Found Switchboard-managed {} sidecar at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            } else {
                failures.push(format!(
                    "Switchboard-managed {} sidecar was not found at {}.",
                    sidecar.name,
                    sidecar_path.display()
                ));
            }
        }
        other => return Err(anyhow!("Verification is not supported yet for {other}.",)),
    }

    // Proxy reachability is transient runtime state — the runtime warm-up
    // can finish after this verification runs. Surface it via the
    // `proxy_reachable` field, but don't fail `verified` on it. `verified`
    // attests only to "we wrote everything we needed to write".
    let proxy_reachable = is_headroom_proxy_reachable();
    if proxy_reachable {
        checks.push("Headroom proxy is reachable on 127.0.0.1:6767.".into());
    }

    Ok(ClientSetupVerification {
        client_id: client_id.to_string(),
        verified: failures.is_empty(),
        proxy_reachable,
        checks,
        failures,
    })
}
fn build_planned_switchboard_sidecar_body(spec: &PlannedSidecarSpec) -> String {
    if spec.id == "goose" {
        return format!(
            "Managed by AI Switchboard.\n\
             Purpose: reversible Goose Repo Memory MCP bridge marker alongside allowlisted native endpoint routing.\n\
             Reference proxy base: {HEADROOM_OPENAI_BASE_URL}\n\
             Boundary: native setup writes only documented non-secret OpenAI/Anthropic endpoint fields; account state, secrets, provider credentials, and model selection remain manual.\n\
             Additional Goose provider fields remain gated until their documented schema and reversible lifecycle are proven."
        );
    }

    format!(
        "Managed by AI Switchboard.\n\
         Purpose: reversible {} routing-intent sidecar while active provider config support remains gated.\n\
         Proxy base: {HEADROOM_OPENAI_BASE_URL}\n\
         Boundary: this file does not mutate account state, secrets, or undocumented provider config.\n\
         Next promotion gate: replace this sidecar with a documented {} config edit after dry-run, backup, verify, rollback, and Off cleanup pass.",
        spec.name, spec.name
    )
}

pub(crate) fn configure_planned_switchboard_sidecar(client_id: &str) -> Result<(bool, Option<PathBuf>)> {
    let spec = planned_sidecar_spec(client_id)
        .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
    let path = planned_sidecar_routing_path(client_id)?;
    upsert_managed_block(
        &path,
        spec.id,
        &build_planned_switchboard_sidecar_body(spec),
    )
}

fn cursor_sidecar_confirmation_phrase(current_state: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(current_state.as_bytes());
    let hash = hasher
        .finalize()
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "Apply {CURSOR_MARKER_PREFIX} sidecar to {} after reviewing {hash}",
        planned_sidecar_routing_path("cursor")?.display()
    ))
}

pub(crate) fn preview_cursor_sidecar_apply() -> Result<ManagedConfigApplyPreview> {
    let spec =
        planned_sidecar_spec("cursor").ok_or_else(|| anyhow!("Cursor sidecar is unavailable."))?;
    let path = planned_sidecar_routing_path("cursor")?;
    let current_state = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };
    let proposed_state = managed_block_updated_content(
        &current_state,
        spec.id,
        &build_planned_switchboard_sidecar_body(spec),
    );
    Ok(ManagedConfigApplyPreview {
        record_id: CURSOR_SIDECAR_APPLY_RECORD_ID.to_string(),
        owner: CURSOR_SIDECAR_OWNER.to_string(),
        target_path: path.display().to_string(),
        marker: CURSOR_MARKER_PREFIX.to_string(),
        backup_path: format!("{}.headroom-backup-*", SWITCHBOARD_ROUTING_FILE),
        status: ManagedRollbackExecutionStatus::Ready,
        confirmation_phrase: cursor_sidecar_confirmation_phrase(&current_state)?,
        current_state,
        proposed_state,
        rollback_preview: "Remove only the Switchboard-owned Cursor sidecar block through Rollback Center; Cursor settings, accounts, models, and extension storage remain untouched.".to_string(),
        blocked_reason: None,
        evidence: vec![
            "Cursor provider settings schema is not allowlisted; this preview targets only the Switchboard-owned sidecar.".to_string(),
            "Preview does not read Cursor settings.json, globalStorage, credentials, account state, or model selection.".to_string(),
            "Apply creates a sibling backup when a sidecar already exists, writes only the managed marker block, verifies it, and supports rollback and Off cleanup.".to_string(),
        ],
    })
}

fn sidecar_apply_confirmation_phrase(client_id: &str, current_state: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(current_state.as_bytes());
    let hash = hasher
        .finalize()
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "Apply headroom:{client_id} sidecar to {} after reviewing {hash}",
        planned_sidecar_routing_path(client_id)?.display()
    ))
}

pub(crate) fn preview_provider_sidecar_apply(
    record_id: &str,
    client_id: &str,
    owner: &str,
) -> Result<ManagedConfigApplyPreview> {
    let spec = planned_sidecar_spec(client_id)
        .ok_or_else(|| anyhow!("{owner} sidecar is unavailable."))?;
    let path = planned_sidecar_routing_path(client_id)?;
    let current_state = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };
    let proposed_state = managed_block_updated_content(
        &current_state,
        spec.id,
        &build_planned_switchboard_sidecar_body(spec),
    );
    Ok(ManagedConfigApplyPreview {
        record_id: record_id.to_string(),
        owner: owner.to_string(),
        target_path: path.display().to_string(),
        marker: format!("headroom:{client_id}"),
        backup_path: format!("{}.headroom-backup-*", SWITCHBOARD_ROUTING_FILE),
        status: ManagedRollbackExecutionStatus::Ready,
        confirmation_phrase: sidecar_apply_confirmation_phrase(client_id, &current_state)?,
        current_state,
        proposed_state,
        rollback_preview: format!("Remove only the Switchboard-owned {owner} block through Rollback Center; provider, model, credentials, and account state remain untouched."),
        blocked_reason: None,
        evidence: vec![
            format!("{owner} native provider schema is not allowlisted; this preview targets only the Switchboard-owned sidecar."),
            "Preview does not read credentials, account state, provider configuration, or model selection.".to_string(),
            "Apply is state-bound to this preview, creates a sibling backup when needed, re-reads the managed marker, and supports rollback and Off cleanup.".to_string(),
        ],
    })
}

pub(crate) fn execute_provider_sidecar_apply(
    record_id: &str,
    client_id: &str,
    owner: &str,
    confirmation_phrase: &str,
) -> Result<ManagedConfigApplyResult> {
    let preview = preview_provider_sidecar_apply(record_id, client_id, owner)?;
    if confirmation_phrase != preview.confirmation_phrase {
        return Err(anyhow!(
            "Managed config apply confirmation phrase does not match."
        ));
    }
    let path = planned_sidecar_routing_path(client_id)?;
    let (changed, backup) = configure_planned_switchboard_sidecar(client_id)?;
    if !planned_switchboard_sidecar_matches(client_id)? {
        return Err(anyhow!("{owner} verification failed after apply."));
    }
    let mut state = load_setup_state();
    state.configured_clients.insert(
        normalized_setup_id(client_id).to_string(),
        Utc::now().to_rfc3339(),
    );
    write_setup_state(&state)?;
    Ok(ManagedConfigApplyResult {
        record_id: record_id.to_string(), owner: owner.to_string(), target_path: path.display().to_string(),
        changed, backup_path: backup.map(|path| path.display().to_string()), marker: format!("headroom:{client_id}"),
        verification: vec![
            "Exact state-bound confirmation phrase matched the dry-run preview.".to_string(),
            format!("Only the Switchboard-owned {owner} sidecar was written; provider, model, credentials, and account state were not read or changed."),
            "Managed sidecar marker was re-read from disk after apply; Rollback Center and Off mode remove only the managed block.".to_string(),
        ],
    })
}

pub(crate) fn planned_switchboard_sidecar_matches(client_id: &str) -> Result<bool> {
    let spec = planned_sidecar_spec(client_id)
        .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
    let path = planned_sidecar_routing_path(client_id)?;
    if !path.exists() {
        return Ok(false);
    }

    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let expected_purpose = if spec.id == "goose" {
        "reversible Goose Repo Memory MCP bridge marker".to_string()
    } else {
        format!("reversible {} routing-intent sidecar", spec.name)
    };

    Ok(content.contains(&format!("# >>> headroom:{} >>>", spec.id))
        && content.contains(&format!("# <<< headroom:{} <<<", spec.id))
        && content.contains(HEADROOM_OPENAI_BASE_URL)
        && content.contains(&expected_purpose))
}
pub fn disable_client_setup(client_id: &str) -> Result<()> {
    let mut state = load_setup_state();

    match client_id {
        "codex" | "codex_cli" => {
            disable_codex_cli()?;
            disable_codex_gui()?;
            // Hand the threads back to the native-provider menu so the full
            // history stays visible once Codex no longer routes through Headroom.
            let _ = crate::codex_threads::retag_codex_thread_providers("headroom", "openai");
        }
        "codex_gui" => {
            disable_codex_gui()?;
        }
        "claude_code" => {
            let shell_targets = resolve_client_shell_targets_for_cleanup(&state, client_id)?;
            remove_shell_block(&shell_targets, "claude_code")?;
            // Also drop the managed_rtk PATH block so `rtk` isn't exported from
            // shell profiles after quit — otherwise the user's next shell still
            // has Headroom binaries shadowing whatever's on PATH.
            remove_shell_block(&shell_targets, "managed_rtk")?;
            remove_claude_settings_env("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
            let _ = remove_legacy_vscode_base_url_keys()?;
            // Strip the PreToolUse hook entry and delete the hook script so CC
            // behaves exactly as it did before Headroom was launched.
            for settings_path in claude_settings_candidates() {
                let _ = strip_headroom_hook_from_settings(&settings_path);
            }
            let hook_path = headroom_rtk_hook_path();
            if hook_path.exists() {
                let _ = std::fs::remove_file(&hook_path);
            }
        }
        "vscode" => remove_vscode_connector_keys()?,
        "gemini_cli" => {
            let shell_targets = resolve_client_shell_targets_for_cleanup(&state, client_id)?;
            remove_shell_block(&shell_targets, "gemini_cli")?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "opencode" => {
            remove_opencode_provider_config()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "goose" => {
            let _ = crate::goose_provider_configs::remove_goose_provider_config()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "grok_cli" => {
            let _ = remove_grok_provider_config()?;
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {client_id}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(client_id)?, sidecar.id)?;
        }
        "windsurf" => {
            remove_windsurf_provider_config()?;
        }
        "zed_ai" => {
            remove_zed_provider_config()?;
        }
        other if planned_sidecar_spec(other).is_some() => {
            let sidecar = planned_sidecar_spec(other)
                .ok_or_else(|| anyhow!("No Switchboard sidecar is configured for {other}."))?;
            let _ = remove_managed_block(&planned_sidecar_routing_path(other)?, sidecar.id)?;
        }
        other => {
            return Err(anyhow!(
                "Automatic setup disable is not supported yet for {other}.",
            ))
        }
    }

    match client_id {
        "codex" | "codex_cli" => {
            state.configured_clients.remove("codex");
            state.configured_clients.remove("codex_cli");
            state.configured_clients.remove("codex_gui");
            state.remembered_clients.remove("codex");
            state.remembered_clients.remove("codex_cli");
            state.remembered_clients.remove("codex_gui");
            state.managed_shell_files.remove("codex");
            state.managed_shell_files.remove("codex_cli");
            state.managed_shell_files.remove("codex_gui");
            state.remembered_shell_files.remove("codex");
            state.remembered_shell_files.remove("codex_cli");
            state.remembered_shell_files.remove("codex_gui");
        }
        _ => {
            let state_id = normalized_setup_id(client_id);
            state.configured_clients.remove(state_id);
            state.remembered_clients.remove(state_id);
            state.managed_shell_files.remove(state_id);
            state.remembered_shell_files.remove(state_id);
        }
    }
    write_setup_state(&state)?;
    Ok(())
}

pub fn clear_client_setups() -> Result<()> {
    // Capture snapshot before disabling. We re-apply it afterwards because
    // disable_client_setup also clears remembered_clients as a side effect,
    // which would otherwise erase the snapshot we need for restore_client_setups.
    let pre = load_setup_state();
    let snapshot_clients = pre.configured_clients.clone();
    let snapshot_shell_files = pre.managed_shell_files.clone();

    for spec in MANAGED_CLIENT_SPECS {
        let _ = disable_client_setup(spec.id);
    }
    let _ = disable_client_setup("codex_gui");
    for spec in PLANNED_SIDECAR_SPECS {
        if pre.configured_clients.contains_key(spec.id) {
            let _ = disable_client_setup(spec.id);
        }
    }

    // Re-save the remembered snapshot so restore_client_setups works on next launch.
    if !snapshot_clients.is_empty() {
        let mut state = load_setup_state();
        state.remembered_clients = snapshot_clients;
        state.remembered_shell_files = snapshot_shell_files;
        write_setup_state(&state)?;
    }

    Ok(())
}
/// Remove the PreToolUse entry pointing at `headroom-rtk-rewrite.sh`. Drops
/// the `PreToolUse` array if it becomes empty, and the `hooks` object if it
/// has no remaining event arrays. Returns true if the file was modified.
pub(crate) fn strip_headroom_hook_from_settings(settings_path: &Path) -> Result<bool> {
    remove_pre_tool_use_markers(
        settings_path,
        &["headroom-rtk-rewrite.sh", "headroom-markitdown-read.sh"],
    )
}

/// Removes every PreToolUse hook entry whose command contains one of `markers`,
/// pruning empty `PreToolUse`/`hooks` containers. Returns whether the file changed.
pub(crate) fn remove_pre_tool_use_markers(settings_path: &Path, markers: &[&str]) -> Result<bool> {
    if !settings_path.exists() {
        return Ok(false);
    }

    let raw = std::fs::read_to_string(settings_path)
        .with_context(|| format!("reading {}", settings_path.display()))?;
    if raw.trim().is_empty() {
        return Ok(false);
    }
    let mut root = parse_json_object(&raw, settings_path)?;

    let Some(hooks_val) = root.get_mut("hooks") else {
        return Ok(false);
    };
    let Some(hooks_obj) = hooks_val.as_object_mut() else {
        return Ok(false);
    };

    let mut changed = false;

    if let Some(pre_tool_use) = hooks_obj
        .get_mut("PreToolUse")
        .and_then(|value| value.as_array_mut())
    {
        let before = pre_tool_use.len();
        pre_tool_use.retain(|entry| {
            !markers
                .iter()
                .any(|marker| entry_contains_hook(entry, marker))
        });
        if pre_tool_use.len() != before {
            changed = true;
        }
        if pre_tool_use.is_empty() {
            hooks_obj.remove("PreToolUse");
        }
    }

    if hooks_obj.is_empty() {
        root.remove("hooks");
    }

    if !changed {
        return Ok(false);
    }

    let _ = backup_if_exists(settings_path)?;
    std::fs::write(
        settings_path,
        serde_json::to_vec_pretty(&Value::Object(root))
            .context("serializing Claude settings for hook cleanup")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok(true)
}

/// Re-applies setup for all clients that were active at the last pause or quit.
pub fn restore_client_setups() {
    let state = load_setup_state();
    let to_restore: Vec<String> = state.remembered_clients.keys().cloned().collect();
    for client_id in to_restore {
        let _ = apply_client_setup(&client_id);
    }
}
pub(crate) fn configure_shell_block(
    shell_targets: &[PathBuf],
    block_id: &str,
    block_body: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut changed = Vec::new();
    let mut backups = Vec::new();

    for file in shell_targets {
        let (did_change, backup) = upsert_managed_block(&file, block_id, block_body)?;
        if did_change {
            changed.push(file.display().to_string());
            if let Some(path) = backup {
                backups.push(path.display().to_string());
            }
        }
    }

    Ok((changed, backups))
}

fn configure_vscode_settings() -> Result<(Vec<String>, Vec<String>)> {
    let (mut changed_files, mut backup_files) =
        configure_claude_settings_env("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
    let (legacy_changed, legacy_backups) = remove_legacy_vscode_base_url_keys()?;
    changed_files.extend(legacy_changed);
    backup_files.extend(legacy_backups);
    Ok((changed_files, backup_files))
}

fn remove_vscode_connector_keys() -> Result<()> {
    remove_claude_settings_env("ANTHROPIC_BASE_URL", HEADROOM_ANTHROPIC_BASE_URL)?;
    let _ = remove_legacy_vscode_base_url_keys()?;
    Ok(())
}

fn set_json_string(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    expected_value: &str,
) -> bool {
    let next = Value::String(expected_value.to_string());
    match obj.get(key) {
        Some(existing) if existing == &next => false,
        _ => {
            obj.insert(key.to_string(), next);
            true
        }
    }
}

fn remove_json_key_if_matches(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    expected_value: &str,
) -> bool {
    match obj.get(key) {
        Some(Value::String(value)) if value == expected_value => obj.remove(key).is_some(),
        _ => false,
    }
}

fn configure_claude_settings_env(
    env_key: &str,
    env_value: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let settings_path = claude_settings_path();
    let mut content = if settings_path.exists() {
        let raw = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("reading {}", settings_path.display()))?;
        Value::Object(parse_json_object(&raw, &settings_path)?)
    } else {
        Value::Object(Default::default())
    };

    if !content.is_object() {
        content = Value::Object(Default::default());
    }

    let Some(root) = content.as_object_mut() else {
        return Err(anyhow!("unable to write Claude settings"));
    };

    if !root
        .get("env")
        .map(|value| value.is_object())
        .unwrap_or(false)
    {
        root.insert("env".into(), Value::Object(Default::default()));
    }

    let Some(env_obj) = root.get_mut("env").and_then(|value| value.as_object_mut()) else {
        return Err(anyhow!("unable to write Claude env settings"));
    };

    let changed = set_json_string(env_obj, env_key, env_value);
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let backup = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&content).context("serializing Claude settings")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok((
        vec![settings_path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

pub(crate) fn ensure_claude_settings_hook(
    hook_path: &Path,
    matcher: &str,
    marker: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let settings_path = claude_settings_path();
    let mut content = if settings_path.exists() {
        let raw = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("reading {}", settings_path.display()))?;
        Value::Object(parse_json_object(&raw, &settings_path)?)
    } else {
        Value::Object(Default::default())
    };

    if !content.is_object() {
        content = Value::Object(Default::default());
    }

    let hook_command = hook_path
        .to_str()
        .ok_or_else(|| anyhow!("hook path contains invalid UTF-8: {}", hook_path.display()))?;
    let already_present = claude_hook_present_in_value(&content, hook_command);
    if already_present {
        return Ok((Vec::new(), Vec::new()));
    }

    let Some(root) = content.as_object_mut() else {
        return Err(anyhow!("unable to write Claude hook settings"));
    };

    if !root
        .get("hooks")
        .map(|value| value.is_object())
        .unwrap_or(false)
    {
        root.insert("hooks".into(), Value::Object(Default::default()));
    }

    let Some(hooks_obj) = root
        .get_mut("hooks")
        .and_then(|value| value.as_object_mut())
    else {
        return Err(anyhow!("unable to write Claude hooks settings"));
    };
    if !hooks_obj
        .get("PreToolUse")
        .map(|value| value.is_array())
        .unwrap_or(false)
    {
        hooks_obj.insert("PreToolUse".into(), Value::Array(Vec::new()));
    }

    let Some(pre_tool_use) = hooks_obj
        .get_mut("PreToolUse")
        .and_then(|value| value.as_array_mut())
    else {
        return Err(anyhow!("unable to write Claude PreToolUse hooks"));
    };

    pre_tool_use.retain(|entry| !entry_contains_hook(entry, marker));
    pre_tool_use.push(serde_json::json!({
        "matcher": matcher,
        "hooks": [{
            "type": "command",
            "command": hook_command
        }]
    }));

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let backup = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&content).context("serializing Claude hook settings")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok((
        vec![settings_path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

fn remove_claude_settings_env(env_key: &str, expected_value: &str) -> Result<()> {
    let settings_path = claude_settings_path();
    if !settings_path.exists() {
        return Ok(());
    }

    let raw = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("reading {}", settings_path.display()))?;
    let mut root = parse_json_object(&raw, &settings_path)?;
    let mut changed = false;

    if let Some(Value::Object(env_obj)) = root.get_mut("env") {
        changed |= remove_json_key_if_matches(env_obj, env_key, expected_value);
        if env_obj.is_empty() {
            root.remove("env");
            changed = true;
        }
    }

    if !changed {
        return Ok(());
    }

    let _ = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&Value::Object(root))
            .context("serializing Claude settings for connector removal")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok(())
}

pub(crate) fn claude_hook_present_in_value(content: &Value, hook_path: &str) -> bool {
    content
        .get("hooks")
        .and_then(|value| value.get("PreToolUse"))
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|hooks| hooks.as_array())
                    .map(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("command")
                                .and_then(|command| command.as_str())
                                .map(|command| command == hook_path)
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

pub(crate) fn entry_contains_hook(entry: &Value, hook_fragment: &str) -> bool {
    entry
        .get("hooks")
        .and_then(|hooks| hooks.as_array())
        .map(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("command")
                    .and_then(|command| command.as_str())
                    .map(|command| command.contains(hook_fragment))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn remove_legacy_vscode_base_url_keys() -> Result<(Vec<String>, Vec<String>)> {
    let settings_path = home_dir()
        .join("Library")
        .join("Application Support")
        .join("Code")
        .join("User")
        .join("settings.json");
    if !settings_path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let raw = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("reading {}", settings_path.display()))?;
    let mut obj = parse_json_object(&raw, &settings_path)?;

    let mut changed = false;
    changed |= remove_json_key_if_matches(&mut obj, "openai.baseUrl", HEADROOM_PROXY_URL);
    changed |= remove_json_key_if_matches(&mut obj, "anthropic.baseUrl", HEADROOM_PROXY_URL);
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    let backup = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&Value::Object(obj))
            .context("serializing VS Code settings for legacy key cleanup")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;

    Ok((
        vec![settings_path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}


pub(crate) fn shell_block_contains_in_files(
    shell_targets: &[PathBuf],
    block_id: &str,
    var_name: &str,
    expected_value: &str,
) -> Result<bool> {
    for file in shell_targets {
        if !file.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let start = format!("# >>> headroom:{block_id} >>>");
        let end = format!("# <<< headroom:{block_id} <<<");

        if let (Some(start_idx), Some(end_idx)) = (content.find(&start), content.find(&end)) {
            let block = &content[start_idx..end_idx];
            let expected_line = format!("export {var_name}={expected_value}");
            if block.contains(&expected_line) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

pub(crate) fn shell_block_contains_text_in_files(
    shell_targets: &[PathBuf],
    block_id: &str,
    expected_text: &str,
) -> Result<bool> {
    for file in shell_targets {
        if !file.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let start = format!("# >>> headroom:{block_id} >>>");
        let end = format!("# <<< headroom:{block_id} <<<");

        if let (Some(start_idx), Some(end_idx)) = (content.find(&start), content.find(&end)) {
            if content[start_idx..end_idx].contains(expected_text) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn claude_settings_env_matches(env_key: &str, expected_value: &str) -> Result<bool> {
    let path = claude_settings_path();
    if !path.exists() {
        return Ok(false);
    }

    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let content: Value = Value::Object(parse_json_object(&raw, &path)?);
    Ok(matches!(
        content.get("env").and_then(|env| env.get(env_key)),
        Some(Value::String(value)) if value == expected_value
    ))
}

pub(crate) fn claude_settings_hook_matches(hook_fragment: &str) -> Result<bool> {
    let path = claude_settings_path();
    if !path.exists() {
        return Ok(false);
    }

    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let content: Value = Value::Object(parse_json_object(&raw, &path)?);

    Ok(content
        .get("hooks")
        .and_then(|hooks| hooks.get("PreToolUse"))
        .and_then(|hooks| hooks.as_array())
        .map(|entries| {
            entries
                .iter()
                .any(|entry| entry_contains_hook(entry, hook_fragment))
        })
        .unwrap_or(false))
}

fn is_headroom_proxy_reachable() -> bool {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    ["127.0.0.1", "localhost"].iter().any(|host| {
        client
            .get(format!("http://{host}:6767/readyz"))
            .send()
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    })
}

pub(crate) fn managed_block_contains_text(
    file_path: &Path,
    block_id: &str,
    expected_text: &str,
) -> Result<bool> {
    if !file_path.exists() {
        return Ok(false);
    }
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;
    let start = format!("# >>> headroom:{block_id} >>>");
    let end = format!("# <<< headroom:{block_id} <<<");
    let (Some(start_idx), Some(end_idx)) = (content.find(&start), content.find(&end)) else {
        return Ok(false);
    };
    Ok(content[start_idx..end_idx].contains(expected_text))
}

pub(crate) fn shell_double_quote(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

pub(crate) fn ensure_rtk_integrations_for_targets(
    managed_rtk_path: &Path,
    managed_python_path: &Path,
    shell_targets: &[PathBuf],
) -> Result<(Vec<String>, Vec<String>)> {
    if load_setup_state().rtk_disabled || !managed_rtk_path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();

    let mut path_updates = ensure_managed_rtk_on_path(managed_rtk_path, shell_targets)?;
    let mut hook_updates = ensure_claude_code_rtk_hook(managed_rtk_path, managed_python_path)?;
    changed_files.append(&mut path_updates.0);
    backup_files.append(&mut path_updates.1);
    changed_files.append(&mut hook_updates.0);
    backup_files.append(&mut hook_updates.1);

    if is_codex_enabled() {
        let agents = rtk_codex_agents_path();
        let (codex_changed, codex_backup) =
            upsert_managed_block(&agents, "rtk", &build_rtk_codex_nudge(managed_rtk_path))?;
        if codex_changed {
            changed_files.push(agents.display().to_string());
        }
        if let Some(path) = codex_backup {
            backup_files.push(path.display().to_string());
        }
    }

    Ok((changed_files, backup_files))
}

fn ensure_managed_rtk_on_path(
    rtk_path: &Path,
    shell_targets: &[PathBuf],
) -> Result<(Vec<String>, Vec<String>)> {
    let managed_bin_dir = rtk_path.parent().ok_or_else(|| {
        anyhow!(
            "managed RTK path {} is missing a parent directory",
            rtk_path.display()
        )
    })?;
    let path_value = shell_double_quote(&managed_bin_dir.to_string_lossy());
    configure_shell_block(
        shell_targets,
        "managed_rtk",
        &format!("export PATH=\"{path_value}:$PATH\""),
    )
}

fn ensure_claude_code_rtk_hook(
    managed_rtk_path: &Path,
    managed_python_path: &Path,
) -> Result<(Vec<String>, Vec<String>)> {
    let hook_path = headroom_rtk_hook_path();
    let hook_body = build_headroom_rtk_hook(managed_rtk_path, managed_python_path);
    let (hook_changed, hook_backup) = write_file_if_changed(&hook_path, &hook_body, true)?;
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();

    if hook_changed {
        changed_files.push(hook_path.display().to_string());
    }
    if let Some(path) = hook_backup {
        backup_files.push(path.display().to_string());
    }

    let (settings_changed, settings_backups) =
        ensure_claude_settings_hook(&hook_path, "Bash", "headroom-rtk-rewrite.sh")?;
    changed_files.extend(settings_changed);
    backup_files.extend(settings_backups);

    Ok((changed_files, backup_files))
}

fn build_rtk_codex_nudge(managed_rtk_path: &Path) -> String {
    let bin = managed_rtk_path.display();
    format!(
        "## Token-saving shell commands (Headroom RTK)\n\
         Run shell commands through RTK to get compact, token-optimized output:\n\
         prefix the command with `{bin} ` (for example `{bin} git status`,\n\
         `{bin} ls -la`, `{bin} cargo build`). RTK passes through anything it\n\
         does not optimize, so it is safe to use as a prefix for any command."
    )
}

pub(crate) fn build_headroom_rtk_hook(managed_rtk_path: &Path, managed_python_path: &Path) -> String {
    let rtk = shell_double_quote(&managed_rtk_path.to_string_lossy());
    let python = shell_double_quote(&managed_python_path.to_string_lossy());

    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

HEADROOM_RTK="{rtk}"
HEADROOM_PYTHON="{python}"

if [ ! -x "$HEADROOM_RTK" ] || [ ! -x "$HEADROOM_PYTHON" ]; then
  exit 0
fi

INPUT="$(cat)"
if [ -z "$INPUT" ]; then
  exit 0
fi

CMD="$("$HEADROOM_PYTHON" -c 'import json, sys; data = json.load(sys.stdin); cmd = data.get("tool_input", {{}}).get("command", ""); print(cmd if isinstance(cmd, str) else "")' <<<"$INPUT" 2>/dev/null || true)"
if [ -z "$CMD" ]; then
  exit 0
fi

REWRITTEN="$("$HEADROOM_RTK" rewrite "$CMD" 2>/dev/null || true)"
if [ -z "$REWRITTEN" ] || [ "$CMD" = "$REWRITTEN" ]; then
  exit 0
fi

if [ "${{REWRITTEN%% *}}" = "rtk" ]; then
  REWRITTEN="$HEADROOM_RTK${{REWRITTEN#rtk}}"
fi

FIRST_TOKEN="${{REWRITTEN%% *}}"
case "$FIRST_TOKEN" in
  /*)
    [ -x "$FIRST_TOKEN" ] || exit 0
    ;;
  *)
    command -v "$FIRST_TOKEN" >/dev/null 2>&1 || exit 0
    ;;
esac

HEADROOM_RTK_REWRITTEN="$REWRITTEN" "$HEADROOM_PYTHON" -c 'import json, os, sys; data = json.load(sys.stdin); tool_input = data.get("tool_input"); 
if not isinstance(tool_input, dict):
    sys.exit(0)
updated = dict(tool_input)
updated["command"] = os.environ["HEADROOM_RTK_REWRITTEN"]
json.dump({{"hookSpecificOutput": {{"hookEventName": "PreToolUse", "permissionDecision": "allow", "permissionDecisionReason": "Headroom RTK auto-rewrite", "updatedInput": updated}}}}, sys.stdout)' <<<"$INPUT" 2>/dev/null || exit 0
"#
    )
}

pub use crate::client_codex_setup::{codex_provider_block_matches, CODEX_ROOT_BLOCK_ID};
