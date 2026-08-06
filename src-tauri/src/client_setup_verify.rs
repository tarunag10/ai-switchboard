use anyhow::{anyhow, Result};

use crate::client_codex_setup::codex_provider_block_matches;
use crate::client_connectors::planned_sidecar_spec;
use crate::client_paths::{
    grok_config_path, opencode_config_path, planned_sidecar_routing_path, headroom_rtk_hook_path,
    windsurf_config_path, zed_config_path,
};
use crate::client_provider_configs::{
    grok_provider_config_matches, opencode_provider_config_matches,
    windsurf_provider_config_matches, zed_provider_config_matches,
    HEADROOM_ANTHROPIC_BASE_URL, HEADROOM_OPENAI_BASE_URL, OPENCODE_HEADROOM_PROVIDER_ID,
};
use crate::client_setup_sidecar::planned_switchboard_sidecar_matches;
use crate::client_setup_state::{
    default_headroom_rtk_path, load_setup_state, resolve_client_shell_targets,
};
use crate::client_shell_setup::{
    claude_settings_env_matches, claude_settings_hook_matches, is_headroom_proxy_reachable,
    shell_block_contains_in_files, shell_block_contains_text_in_files,
};
use crate::models::ClientSetupVerification;

const HEADROOM_PROXY_URL: &str = "http://127.0.0.1:6767";
const GEMINI_BASE_URL_ENV_KEY: &str = "GOOGLE_GEMINI_BASE_URL";
const GEMINI_COMPAT_BASE_URL_ENV_KEY: &str = "GEMINI_BASE_URL";
const GEMINI_API_KEY_ENV_KEY: &str = "GEMINI_API_KEY";
const GEMINI_HEADROOM_API_KEY_VALUE: &str = "headroom-local";

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
        "continue" => {
            let sidecar = planned_sidecar_spec(client_id)
                .ok_or_else(|| anyhow!("Unknown planned sidecar {client_id}"))?;
            let sidecar_path = planned_sidecar_routing_path(client_id)?;
            let sidecar_ok = planned_switchboard_sidecar_matches(client_id)?;
            let provider_ok = crate::continue_provider_configs::continue_provider_config_matches()?;

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
                    "Found Continue model {} pointing to Headroom in {}.",
                    crate::continue_provider_configs::CONTINUE_HEADROOM_MODEL_NAME,
                    crate::client_paths::continue_config_path().display()
                ));
            } else {
                failures.push(format!(
                    "Continue model {} was not found in {}.",
                    crate::continue_provider_configs::CONTINUE_HEADROOM_MODEL_NAME,
                    crate::client_paths::continue_config_path().display()
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
