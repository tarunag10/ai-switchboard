use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::client_paths::{
    grok_config_path, opencode_config_path, windsurf_config_path, zed_config_path,
    GROK_CONFIG_FILE, OPENCODE_CONFIG_FILE, WINDSURF_CONFIG_FILE, ZED_CONFIG_FILE,
};
use crate::managed_files::{backup_if_exists, parse_json_object};

pub(super) const HEADROOM_ANTHROPIC_BASE_URL: &str = "http://127.0.0.1:6767";
pub(super) const HEADROOM_OPENAI_BASE_URL: &str = "http://127.0.0.1:6767/v1";
pub(super) const OPENCODE_HEADROOM_PROVIDER_ID: &str = "headroom";
pub(super) const WINDSURF_MARKER_PREFIX: &str = "headroom:windsurf";
pub(super) const ZED_MARKER_PREFIX: &str = "headroom:zed";
pub(super) const GROK_MARKER_PREFIX: &str = "headroom:grok";
pub(super) const GROK_HEADROOM_BASE_URL: &str = "http://127.0.0.1:6767/v1";

fn opencode_headroom_provider_value() -> Value {
    serde_json::json!({
        "npm": "@ai-sdk/openai",
        "name": "Mac AI Switchboard",
        "options": {
            "baseURL": HEADROOM_OPENAI_BASE_URL
        },
        "models": {
            "headroom": {
                "name": "Headroom Router"
            }
        }
    })
}

fn short_state_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn opencode_apply_confirmation_phrase(marker: &str, current_state: &str) -> String {
    format!(
        "Apply {marker} to {} after reviewing {}",
        opencode_config_path().display(),
        short_state_hash(current_state)
    )
}

pub(super) fn opencode_config_backup_pattern() -> String {
    let path = opencode_config_path();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(OPENCODE_CONFIG_FILE);
    format!("{}.headroom-backup-*", file_name)
}

pub(super) fn opencode_next_provider_config() -> Result<(Value, bool)> {
    let path = opencode_config_path();
    let mut root = if path.exists() {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        parse_json_object(&raw, &path)?
    } else {
        serde_json::Map::new()
    };
    let provider_value = root
        .entry("provider".to_string())
        .or_insert_with(|| Value::Object(Default::default()));
    if !provider_value.is_object() {
        return Err(anyhow!(
            "{} provider key must be an object before Switchboard can manage OpenCode.",
            path.display()
        ));
    }
    let provider = provider_value
        .as_object_mut()
        .ok_or_else(|| anyhow!("unable to write OpenCode provider settings"))?;
    let next = opencode_headroom_provider_value();
    let changed = match provider.get(OPENCODE_HEADROOM_PROVIDER_ID) {
        Some(existing) if existing == &next => false,
        _ => {
            provider.insert(OPENCODE_HEADROOM_PROVIDER_ID.to_string(), next);
            true
        }
    };
    Ok((Value::Object(root), changed))
}

pub(super) fn configure_opencode_provider_config() -> Result<(Vec<String>, Vec<String>)> {
    let path = opencode_config_path();
    let (next_config, changed) = opencode_next_provider_config()?;
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let backup = backup_if_exists(&path)?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&next_config).context("serializing OpenCode provider config")?,
    )
    .with_context(|| format!("writing {}", path.display()))?;

    Ok((
        vec![path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

pub(super) fn opencode_provider_config_matches() -> Result<bool> {
    let path = opencode_config_path();
    if !path.exists() {
        return Ok(false);
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let root = parse_json_object(&raw, &path)?;
    let provider = root
        .get("provider")
        .and_then(|value| value.as_object())
        .and_then(|providers| providers.get(OPENCODE_HEADROOM_PROVIDER_ID));
    Ok(provider == Some(&opencode_headroom_provider_value()))
}

pub(super) fn remove_opencode_provider_config() -> Result<()> {
    let path = opencode_config_path();
    if !path.exists() {
        return Ok(());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut root = parse_json_object(&raw, &path)?;
    let Some(provider_obj) = root
        .get_mut("provider")
        .and_then(|value| value.as_object_mut())
    else {
        return Ok(());
    };
    match provider_obj.get(OPENCODE_HEADROOM_PROVIDER_ID) {
        Some(existing) if existing == &opencode_headroom_provider_value() => {}
        _ => return Ok(()),
    }
    provider_obj.remove(OPENCODE_HEADROOM_PROVIDER_ID);
    if provider_obj.is_empty() {
        root.remove("provider");
    }
    let _ = backup_if_exists(&path)?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&Value::Object(root))
            .context("serializing OpenCode provider cleanup")?,
    )
    .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub(super) fn windsurf_config_backup_pattern() -> String {
    let path = windsurf_config_path();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(WINDSURF_CONFIG_FILE);
    format!("{}.headroom-backup-*", file_name)
}

pub(super) fn windsurf_apply_confirmation_phrase(marker: &str, current_state: &str) -> String {
    format!(
        "Apply {marker} to {} after reviewing {}",
        windsurf_config_path().display(),
        short_state_hash(current_state)
    )
}

pub(super) fn windsurf_next_provider_config() -> Result<(Value, bool)> {
    let path = windsurf_config_path();
    let mut root = if path.exists() {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        parse_json_object(&raw, &path)?
    } else {
        serde_json::Map::new()
    };

    let mut changed = false;
    changed |= set_json_string(
        &mut root,
        &format!("// >>> {WINDSURF_MARKER_PREFIX} >>>"),
        "Managed by Mac AI Switchboard for Windsurf.",
    );
    changed |= set_json_string(&mut root, "anthropic.baseUrl", HEADROOM_ANTHROPIC_BASE_URL);
    changed |= set_json_string(
        &mut root,
        &format!("// <<< {WINDSURF_MARKER_PREFIX} <<<"),
        "End of managed block.",
    );

    Ok((Value::Object(root), changed))
}

pub(super) fn configure_windsurf_provider_config() -> Result<(Vec<String>, Vec<String>)> {
    let path = windsurf_config_path();
    let (next_config, changed) = windsurf_next_provider_config()?;
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let backup = backup_if_exists(&path)?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&next_config).context("serializing Windsurf provider config")?,
    )
    .with_context(|| format!("writing {}", path.display()))?;

    Ok((
        vec![path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

pub(super) fn windsurf_provider_config_matches() -> Result<bool> {
    let path = windsurf_config_path();
    if !path.exists() {
        return Ok(false);
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let root = parse_json_object(&raw, &path)?;
    let start_marker = format!("// >>> {WINDSURF_MARKER_PREFIX} >>>");
    let end_marker = format!("// <<< {WINDSURF_MARKER_PREFIX} <<<");
    Ok(root.get(&start_marker).is_some()
        && root.get(&end_marker).is_some()
        && root.get("anthropic.baseUrl").and_then(|v| v.as_str())
            == Some(HEADROOM_ANTHROPIC_BASE_URL))
}

pub(super) fn remove_windsurf_provider_config() -> Result<Vec<String>> {
    let path = windsurf_config_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut root = parse_json_object(&raw, &path)?;

    let start_marker = format!("// >>> {WINDSURF_MARKER_PREFIX} >>>");
    let end_marker = format!("// <<< {WINDSURF_MARKER_PREFIX} <<<");
    let mut changed = false;
    changed |= root.remove(&start_marker).is_some();
    changed |=
        remove_json_key_if_matches(&mut root, "anthropic.baseUrl", HEADROOM_ANTHROPIC_BASE_URL);
    changed |= root.remove(&end_marker).is_some();

    if !changed {
        return Ok(Vec::new());
    }

    let _ = backup_if_exists(&path)?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&Value::Object(root))
            .context("serializing Windsurf config for connector removal")?,
    )
    .with_context(|| format!("writing {}", path.display()))?;

    Ok(vec![path.display().to_string()])
}

pub(super) fn zed_config_backup_pattern() -> String {
    let path = zed_config_path();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(ZED_CONFIG_FILE);
    format!("{}.headroom-backup-*", file_name)
}

pub(super) fn zed_apply_confirmation_phrase(marker: &str, current_state: &str) -> String {
    format!(
        "Apply {marker} to {} after reviewing {}",
        zed_config_path().display(),
        short_state_hash(current_state)
    )
}

pub(super) fn zed_next_provider_config() -> Result<(Value, bool)> {
    let path = zed_config_path();
    let mut root = if path.exists() {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        parse_json_object(&raw, &path)?
    } else {
        serde_json::Map::new()
    };

    let mut changed = false;
    changed |= set_json_string(
        &mut root,
        &format!("// >>> {ZED_MARKER_PREFIX} >>>"),
        "Managed by Mac AI Switchboard for Zed.",
    );
    changed |= set_json_string(&mut root, "anthropic.baseUrl", HEADROOM_ANTHROPIC_BASE_URL);
    changed |= set_json_string(
        &mut root,
        &format!("// <<< {ZED_MARKER_PREFIX} <<<"),
        "End of managed block.",
    );

    Ok((Value::Object(root), changed))
}

pub(super) fn configure_zed_provider_config() -> Result<(Vec<String>, Vec<String>)> {
    let path = zed_config_path();
    let (next_config, changed) = zed_next_provider_config()?;
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let backup = backup_if_exists(&path)?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&next_config).context("serializing Zed provider config")?,
    )
    .with_context(|| format!("writing {}", path.display()))?;

    Ok((
        vec![path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

pub(super) fn zed_provider_config_matches() -> Result<bool> {
    let path = zed_config_path();
    if !path.exists() {
        return Ok(false);
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let root = parse_json_object(&raw, &path)?;
    let start_marker = format!("// >>> {ZED_MARKER_PREFIX} >>>");
    let end_marker = format!("// <<< {ZED_MARKER_PREFIX} <<<");
    Ok(root.get(&start_marker).is_some()
        && root.get(&end_marker).is_some()
        && root.get("anthropic.baseUrl").and_then(|v| v.as_str())
            == Some(HEADROOM_ANTHROPIC_BASE_URL))
}

pub(super) fn remove_zed_provider_config() -> Result<Vec<String>> {
    let path = zed_config_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut root = parse_json_object(&raw, &path)?;

    let start_marker = format!("// >>> {ZED_MARKER_PREFIX} >>>");
    let end_marker = format!("// <<< {ZED_MARKER_PREFIX} <<<");
    let mut changed = false;
    changed |= root.remove(&start_marker).is_some();
    changed |=
        remove_json_key_if_matches(&mut root, "anthropic.baseUrl", HEADROOM_ANTHROPIC_BASE_URL);
    changed |= root.remove(&end_marker).is_some();

    if !changed {
        return Ok(Vec::new());
    }

    let _ = backup_if_exists(&path)?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&Value::Object(root))
            .context("serializing Zed config for connector removal")?,
    )
    .with_context(|| format!("writing {}", path.display()))?;

    Ok(vec![path.display().to_string()])
}

/// Grok Build's documented native endpoint surface.  The CLI reads
/// `[endpoints].models_base_url` from `~/.grok/config.toml` and then speaks the
/// OpenAI-compatible `/v1` API.  We intentionally manage only this non-secret
/// endpoint field: auth.json, API keys, account state, model selection, and
/// every other Grok setting stay outside Switchboard's read/write boundary.
fn grok_endpoint_line() -> String {
    format!("models_base_url = \"{GROK_HEADROOM_BASE_URL}\"")
}

fn grok_marker_start() -> String {
    format!("# >>> {GROK_MARKER_PREFIX} >>>")
}

fn grok_marker_end() -> String {
    format!("# <<< {GROK_MARKER_PREFIX} <<<")
}

pub(super) fn grok_config_backup_pattern() -> String {
    format!("{}.headroom-backup-*", GROK_CONFIG_FILE)
}

pub(super) fn grok_apply_confirmation_phrase(marker: &str, current_state: &str) -> String {
    format!(
        "Apply {marker} to {} after reviewing {}",
        grok_config_path().display(),
        short_state_hash(current_state)
    )
}

fn grok_section_bounds(lines: &[&str]) -> Option<(usize, usize)> {
    let start = lines.iter().position(|line| {
        let trimmed = line.trim();
        trimmed == "[endpoints]"
    })?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| {
            let trimmed = line.trim();
            trimmed.starts_with('[') && !trimmed.starts_with("#")
        })
        .map(|(index, _)| index)
        .unwrap_or(lines.len());
    Some((start, end))
}

fn grok_endpoint_line_in_section(lines: &[&str], start: usize, end: usize) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .take(end.saturating_sub(start + 1))
        .find(|(_, line)| {
            let trimmed = line.trim_start();
            trimmed.starts_with("models_base_url")
                && trimmed
                    .get("models_base_url".len()..)
                    .is_some_and(|rest| rest.trim_start().starts_with('='))
        })
        .map(|(index, _)| index)
}

fn grok_managed_block_present(raw: &str) -> bool {
    raw.contains(&grok_marker_start()) && raw.contains(&grok_marker_end())
}

fn grok_managed_block_contains_endpoint(raw: &str) -> bool {
    let start_marker = grok_marker_start();
    let end_marker = grok_marker_end();
    let Some(start) = raw.find(&start_marker) else {
        return false;
    };
    let Some(end_relative) = raw[start..].find(&end_marker) else {
        return false;
    };
    raw[start..start + end_relative].contains(&grok_endpoint_line())
}

/// Insert or update the single allowlisted endpoint field while leaving all
/// unrelated TOML bytes untouched. Existing non-Switchboard endpoint values
/// are refused rather than overwritten.
pub(super) fn grok_next_provider_config() -> Result<(String, bool)> {
    let path = grok_config_path();
    let raw = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };

    let start_marker = grok_marker_start();
    let end_marker = grok_marker_end();
    let endpoint_line = grok_endpoint_line();
    if grok_managed_block_present(&raw) {
        if grok_managed_block_contains_endpoint(&raw) {
            return Ok((raw, false));
        }
        return Err(anyhow!(
            "{} contains a drifted Switchboard Grok endpoint marker; refusing to overwrite it.",
            path.display()
        ));
    }

    let mut lines = raw.lines().collect::<Vec<_>>();
    let had_trailing_newline = raw.ends_with('\n');
    if let Some((section_start, section_end)) = grok_section_bounds(&lines) {
        if let Some(endpoint_index) =
            grok_endpoint_line_in_section(&lines, section_start, section_end)
        {
            let existing = lines[endpoint_index].trim();
            if existing != endpoint_line {
                return Err(anyhow!(
                    "{} already defines [endpoints].models_base_url; refusing to overwrite an unmanaged Grok endpoint.",
                    path.display()
                ));
            }
            return Err(anyhow!(
                "{} defines an unmarked Grok endpoint; add the Switchboard marker manually before native routing can be managed.",
                path.display()
            ));
        }

        let block = vec![
            start_marker.as_str(),
            endpoint_line.as_str(),
            end_marker.as_str(),
        ];
        lines.splice(section_end..section_end, block);
        let mut next = lines.join("\n");
        if had_trailing_newline || !next.ends_with('\n') {
            next.push('\n');
        }
        return Ok((next, true));
    }

    let mut next = raw;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    if !next.is_empty() {
        next.push('\n');
    }
    next.push_str("[endpoints]\n");
    next.push_str(&start_marker);
    next.push('\n');
    next.push_str(&endpoint_line);
    next.push('\n');
    next.push_str(&end_marker);
    next.push('\n');
    Ok((next, true))
}

pub(super) fn configure_grok_provider_config() -> Result<(Vec<String>, Vec<String>)> {
    let path = grok_config_path();
    let (next_config, changed) = grok_next_provider_config()?;
    if !changed {
        return Ok((Vec::new(), Vec::new()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let backup = backup_if_exists(&path)?;
    std::fs::write(&path, next_config).with_context(|| format!("writing {}", path.display()))?;
    Ok((
        vec![path.display().to_string()],
        backup
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    ))
}

pub(super) fn grok_provider_config_matches() -> Result<bool> {
    let path = grok_config_path();
    if !path.exists() {
        return Ok(false);
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(grok_managed_block_contains_endpoint(&raw))
}

pub(super) fn remove_grok_provider_config() -> Result<Vec<String>> {
    let path = grok_config_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let start_marker = grok_marker_start();
    let end_marker = grok_marker_end();
    let Some(start) = raw.find(&start_marker) else {
        return Ok(Vec::new());
    };
    let Some(end_relative) = raw[start..].find(&end_marker) else {
        return Err(anyhow!(
            "{} contains an incomplete Switchboard Grok marker; refusing cleanup.",
            path.display()
        ));
    };
    let end = start + end_relative + end_marker.len();
    let block = &raw[start..end];
    if !block.contains(&grok_endpoint_line()) {
        return Err(anyhow!(
            "{} contains a drifted Switchboard Grok marker; refusing cleanup.",
            path.display()
        ));
    }
    let mut next = String::with_capacity(raw.len());
    next.push_str(&raw[..start]);
    next.push_str(&raw[end..]);
    // If Switchboard created the [endpoints] table solely for this field,
    // remove that now-empty table too. Preserve the table when it still has
    // any user-owned key or comment.
    let mut next_lines = next.lines().collect::<Vec<_>>();
    if let Some((section_start, section_end)) = grok_section_bounds(&next_lines) {
        let has_user_content = next_lines[section_start + 1..section_end]
            .iter()
            .any(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'));
        if !has_user_content {
            next_lines.drain(section_start..section_end);
            next = next_lines.join("\n");
            if raw.ends_with('\n') && !next.ends_with('\n') {
                next.push('\n');
            }
        }
    }
    // Keep the user's existing newline convention stable while removing the
    // managed block's surrounding blank line when present.
    let next = next.replace("\n\n\n", "\n\n");
    if next == raw {
        return Ok(Vec::new());
    }
    let _ = backup_if_exists(&path)?;
    std::fs::write(&path, next).with_context(|| format!("writing {}", path.display()))?;
    Ok(vec![path.display().to_string()])
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
