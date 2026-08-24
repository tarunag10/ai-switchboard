use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use crate::client_claude_settings::{ensure_claude_settings_hook, entry_contains_hook};
use crate::client_paths::{claude_settings_path, headroom_rtk_hook_path, rtk_codex_agents_path};
use crate::client_setup_state::{
    default_headroom_managed_python_path, default_headroom_rtk_path, is_codex_enabled,
    load_setup_state,
};
use crate::managed_files::{
    find_managed_block_range, parse_json_object, remove_shell_block, upsert_managed_block,
    write_file_if_changed,
};

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
        if let Some((start_idx, end_idx)) = find_managed_block_range(&content, block_id) {
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
        if let Some((start_idx, end_idx)) = find_managed_block_range(&content, block_id) {
            if content[start_idx..end_idx].contains(expected_text) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

pub(crate) fn claude_settings_env_matches(env_key: &str, expected_value: &str) -> Result<bool> {
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

pub(crate) fn is_headroom_proxy_reachable() -> bool {
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
    let Some((start_idx, end_idx)) = find_managed_block_range(&content, block_id) else {
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

pub(crate) fn build_rtk_codex_nudge(managed_rtk_path: &Path) -> String {
    let bin = managed_rtk_path.display();
    format!(
        "## Token-saving shell commands (Headroom RTK)\n\
         Run shell commands through RTK to get compact, token-optimized output:\n\
         prefix the command with `{bin} ` (for example `{bin} git status`,\n\
         `{bin} ls -la`, `{bin} cargo build`). RTK passes through anything it\n\
         does not optimize, so it is safe to use as a prefix for any command."
    )
}

pub(crate) fn build_headroom_rtk_hook(
    managed_rtk_path: &Path,
    managed_python_path: &Path,
) -> String {
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
