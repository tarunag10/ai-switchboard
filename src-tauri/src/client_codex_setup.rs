use std::process::Command;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::client_detection::codex_home;
use crate::client_paths::{all_shell_paths, codex_config_toml_path};
use crate::client_provider_configs::HEADROOM_OPENAI_BASE_URL;
use crate::managed_files::{
    backup_if_exists, managed_marker_end, managed_marker_start, marker_block_contains,
    remove_shell_block, strip_marker_block,
};

const MARKER_PREFIX: &str = "headroom";

pub(crate) fn disable_codex_cli() -> Result<()> {
    remove_codex_provider_block()?;
    let _ = remove_codex_toml_key("openai_base_url", HEADROOM_OPENAI_BASE_URL);
    let shell_targets = all_shell_paths();
    let _ = remove_shell_block(&shell_targets, "codex_cli");
    let _ = remove_shell_block(&shell_targets, "codex");
    Ok(())
}

pub(crate) fn disable_codex_gui() -> Result<()> {
    clear_legacy_codex_gui_launch_env()?;
    Ok(())
}

fn clear_legacy_codex_gui_launch_env() -> Result<()> {
    remove_launchctl_env(&["OPENAI_BASE_URL", "OPENAI_API_BASE"])?;
    Ok(())
}

// The managed Codex config is split across two marker blocks so each lands in
// the correct TOML scope. `model_provider`/`openai_base_url` are root keys: a
// bare key belongs to the most recently opened `[table]` above it, so appending
// them at end-of-file (as a naive text upsert does) silently absorbs them into
// whatever table the user's config happens to end in (e.g. `[features]`, whose
// values must be booleans), producing
// `invalid type: string "headroom", expected a boolean in features`. The root
// keys therefore go in a block at the *top* of the file (nothing above ⇒ root
// scope), and the `[model_providers.headroom]` table goes in a block at the
// *end*. `requires_openai_auth` is emitted only for ChatGPT-OAuth users: the
// flag is what makes Codex render the account menu (profile/email/plan/usage),
// but it also forces Codex to demand an OpenAI OAuth login (issue #406), which
// would break users authenticated with an OpenAI API key. See
// `codex_uses_chatgpt_auth`.
pub const CODEX_ROOT_BLOCK_ID: &str = "codex_cli";
const CODEX_TABLE_BLOCK_ID: &str = "codex_cli_provider";

fn codex_root_keys_body() -> String {
    format!(
        "model_provider = \"headroom\"\n\
         openai_base_url = \"{base}\"",
        base = HEADROOM_OPENAI_BASE_URL,
    )
}

/// Whether Codex is authenticated via ChatGPT OAuth (rather than an OpenAI API
/// key), read from `~/.codex/auth.json`. Drives whether the managed provider
/// block carries `requires_openai_auth = true` (see [`codex_provider_table_body`]).
fn codex_uses_chatgpt_auth() -> bool {
    let path = codex_home().join("auth.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    let Some(obj) = value.as_object() else {
        return false;
    };
    // Codex records the active method explicitly; trust it when present.
    if let Some(mode) = obj.get("auth_mode").and_then(Value::as_str) {
        return mode.eq_ignore_ascii_case("chatgpt");
    }
    // Older auth.json files predate `auth_mode`: infer ChatGPT mode from the
    // presence of an OAuth account id.
    obj.get("tokens")
        .and_then(Value::as_object)
        .and_then(|tokens| tokens.get("account_id"))
        .and_then(Value::as_str)
        .is_some_and(|id| !id.is_empty())
}

fn codex_provider_table_body(requires_openai_auth: bool) -> String {
    let mut body = format!(
        "[model_providers.headroom]\n\
         name = \"Headroom persistent proxy\"\n\
         base_url = \"{base}\"\n\
         supports_websockets = true",
        base = HEADROOM_OPENAI_BASE_URL,
    );
    if requires_openai_auth {
        body.push_str("\nrequires_openai_auth = true");
    }
    body
}

fn codex_marker_block(block_id: &str, body: &str) -> String {
    format!(
        "{}\n{body}\n{}\n",
        managed_marker_start(MARKER_PREFIX, block_id),
        managed_marker_end(MARKER_PREFIX, block_id)
    )
}

/// Remove every Headroom-managed artifact from Codex `config.toml` text: both
/// managed marker blocks, plus any orphan root keys an older (buggy) build may
/// have left absorbed into a preceding table. Leaves all other content intact.
fn strip_codex_managed_toml(content: &str) -> String {
    let without_blocks = strip_marker_block(
        &strip_marker_block(content, CODEX_ROOT_BLOCK_ID),
        CODEX_TABLE_BLOCK_ID,
    );
    let openai_orphan_prefix = "openai_base_url = \"http://127.0.0.1:";
    strip_legacy_codex_headroom_provider_table(&without_blocks)
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !(trimmed == "model_provider = \"headroom\""
                || (trimmed.starts_with(openai_orphan_prefix) && trimmed.ends_with("/v1\"")))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_legacy_codex_headroom_provider_table(content: &str) -> String {
    let mut out = Vec::new();
    let mut dropping = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[model_providers.headroom]" {
            dropping = true;
            continue;
        }
        if dropping && trimmed.starts_with('[') && trimmed.ends_with(']') {
            dropping = false;
        }
        if !dropping {
            out.push(line);
        }
    }
    out.join("\n")
}

/// Reconstruct `config.toml` with the managed root keys pinned to the top and
/// the provider table appended at the end, around the user's other content.
fn render_codex_config(existing: &str) -> String {
    let mid = strip_codex_managed_toml(existing);
    let mid = mid.trim();

    let mut out = codex_marker_block(CODEX_ROOT_BLOCK_ID, &codex_root_keys_body());
    if !mid.is_empty() {
        out.push('\n');
        out.push_str(mid);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&codex_marker_block(
        CODEX_TABLE_BLOCK_ID,
        &codex_provider_table_body(codex_uses_chatgpt_auth()),
    ));
    out
}

pub(crate) fn configure_codex_provider_block() -> Result<(Vec<String>, Vec<String>)> {
    let path = codex_config_toml_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let existing = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };

    let updated = render_codex_config(&existing);
    if updated == existing {
        return Ok((Vec::new(), Vec::new()));
    }

    let backup = backup_if_exists(&path)?;
    std::fs::write(&path, &updated).with_context(|| format!("writing {}", path.display()))?;

    let mut backup_files = Vec::new();
    if let Some(backup_path) = backup {
        backup_files.push(backup_path.display().to_string());
    }
    Ok((vec![path.display().to_string()], backup_files))
}

pub fn codex_provider_block_matches() -> Result<bool> {
    let path = codex_config_toml_path();
    if !path.exists() {
        return Ok(false);
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let base_url = format!("base_url = \"{}\"", HEADROOM_OPENAI_BASE_URL);
    let openai_base = format!("openai_base_url = \"{}\"", HEADROOM_OPENAI_BASE_URL);
    let root_ok = marker_block_contains(
        &content,
        CODEX_ROOT_BLOCK_ID,
        "model_provider = \"headroom\"",
    ) && marker_block_contains(&content, CODEX_ROOT_BLOCK_ID, &openai_base);
    let table_ok = marker_block_contains(&content, CODEX_TABLE_BLOCK_ID, &base_url);
    Ok(root_ok && table_ok)
}


fn remove_codex_provider_block() -> Result<()> {
    let path = codex_config_toml_path();
    if !path.exists() {
        return Ok(());
    }
    let existing =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let stripped = strip_codex_managed_toml(&existing);
    let normalized = {
        let trimmed = stripped.trim();
        if trimmed.is_empty() {
            String::new()
        } else {
            format!("{trimmed}\n")
        }
    };
    if normalized == existing {
        return Ok(());
    }
    let _ = backup_if_exists(&path)?;
    std::fs::write(&path, &normalized).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn remove_codex_toml_key(key: &str, expected_value: &str) -> Result<()> {
    let path = codex_config_toml_path();
    if !path.exists() {
        return Ok(());
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let target_line = format!("{key} = \"{expected_value}\"");
    let filtered: Vec<&str> = content
        .lines()
        .filter(|l| l.trim() != target_line)
        .collect();
    if filtered.len() == content.lines().count() {
        return Ok(());
    }
    let _ = backup_if_exists(&path)?;
    let mut result = filtered.join("\n");
    if !result.ends_with('\n') && !result.is_empty() {
        result.push('\n');
    }
    std::fs::write(&path, result).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
fn remove_launchctl_env(keys: &[&str]) -> Result<()> {
    for key in keys {
        let _ = run_launchctl(&["unsetenv", key]);
    }
    Ok(())
}

fn run_launchctl(args: &[&str]) -> Result<std::process::Output> {
    let output = Command::new("launchctl")
        .args(args)
        .output()
        .with_context(|| format!("running launchctl {}", args.join(" ")))?;
    Ok(output)
}

