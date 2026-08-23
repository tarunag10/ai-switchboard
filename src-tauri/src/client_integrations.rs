use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::client_detection::codex_home;
use crate::client_paths::{claude_settings_path, headroom_markitdown_hook_path, home_dir};
use crate::client_setup_apply::{
    ensure_claude_settings_hook, managed_block_contains_text, remove_pre_tool_use_markers,
};
use crate::client_setup_state::{is_claude_code_enabled, is_codex_enabled};
use crate::managed_files::{
    backup_if_exists, managed_marker_end, managed_marker_start, parse_json_object,
    remove_managed_block, upsert_managed_block, write_file_if_changed,
};
use crate::switchboard_identity::SwitchboardIdentitySlug;

fn markitdown_claude_md_path() -> PathBuf {
    home_dir().join(".claude").join("CLAUDE.md")
}

fn markitdown_codex_agents_path() -> PathBuf {
    codex_home().join("AGENTS.md")
}

/// Office-only nudge for Claude Code, where PDFs are already handled by the
/// PreToolUse(Read) hook.
pub(crate) fn build_markitdown_office_nudge(shim_path: &Path) -> String {
    let bin = shim_path.display();
    format!(
        "## Reading Office documents (Headroom MarkItDown)\n\
         The Read tool cannot open .docx, .doc, .pptx, .ppt, .xlsx, or .xls files.\n\
         To read one, run `{bin} <path>` via Bash and use the Markdown it prints.\n\
         (PDFs are handled automatically and need no special step.)"
    )
}

/// Codex nudge: Codex has no PreToolUse-style hook, so it covers PDF *and*
/// Office formats through the `markitdown` CLI.
pub(crate) fn build_markitdown_codex_nudge(shim_path: &Path) -> String {
    let bin = shim_path.display();
    format!(
        "## Reading documents (Headroom MarkItDown)\n\
         To read a .pdf, .docx, .doc, .pptx, .ppt, .xlsx, or .xls file, run\n\
         `{bin} <path>` in the shell and use the Markdown it prints, rather than\n\
         opening the raw file. This keeps large documents cheap to read."
    )
}

/// Enables the MarkItDown addon integration for whichever coding clients are
/// configured through Headroom: Claude Code gets the PDF Read hook plus an
/// Office nudge (managed `~/.claude/CLAUDE.md` block + scoped Bash permission);
/// Codex gets a managed `~/.codex/AGENTS.md` nudge covering PDF and Office (it
/// has no hook mechanism). Idempotent and safe to re-run.
pub fn enable_markitdown_integration(
    markitdown_entrypoint: &Path,
    markitdown_shim: &Path,
    python_path: &Path,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();

    if is_claude_code_enabled() {
        let hook_path = headroom_markitdown_hook_path();
        let hook_body = build_headroom_markitdown_hook(markitdown_entrypoint, python_path);
        let (hook_changed, hook_backup) = write_file_if_changed(&hook_path, &hook_body, true)?;
        if hook_changed {
            changed_files.push(hook_path.display().to_string());
        }
        if let Some(path) = hook_backup {
            backup_files.push(path.display().to_string());
        }

        let (settings_changed, settings_backups) =
            ensure_claude_settings_hook(&hook_path, "Read", "headroom-markitdown-read.sh")?;
        changed_files.extend(settings_changed);
        backup_files.extend(settings_backups);

        let claude_md = markitdown_claude_md_path();
        let (md_changed, md_backup) = upsert_managed_block(
            &claude_md,
            "markitdown_office",
            &build_markitdown_office_nudge(markitdown_shim),
        )?;
        if md_changed {
            changed_files.push(claude_md.display().to_string());
        }
        if let Some(path) = md_backup {
            backup_files.push(path.display().to_string());
        }

        if set_markitdown_bash_permission(markitdown_shim, true)? {
            changed_files.push(claude_settings_path().display().to_string());
        }
    }

    if is_codex_enabled() {
        let agents = markitdown_codex_agents_path();
        let (codex_changed, codex_backup) = upsert_managed_block(
            &agents,
            "markitdown",
            &build_markitdown_codex_nudge(markitdown_shim),
        )?;
        if codex_changed {
            changed_files.push(agents.display().to_string());
        }
        if let Some(path) = codex_backup {
            backup_files.push(path.display().to_string());
        }
    }

    Ok((changed_files, backup_files))
}

/// Removes every MarkItDown integration artifact for all clients (Claude Read
/// hook + script + Office nudge + Bash permission, and the Codex AGENTS.md
/// nudge), leaving any RTK hook untouched. Cleanup runs unconditionally so a
/// client that was later disconnected is still scrubbed.
pub fn disable_markitdown_integration(markitdown_shim: &Path) -> Result<bool> {
    let mut changed =
        remove_pre_tool_use_markers(&claude_settings_path(), &["headroom-markitdown-read.sh"])?;
    let hook_path = headroom_markitdown_hook_path();
    changed |= remove_markitdown_hook_if_present(&hook_path)?;
    changed |=
        remove_markitdown_cache_if_present(&std::env::temp_dir().join("headroom-markitdown"))?;
    changed |= remove_managed_block(&markitdown_claude_md_path(), "markitdown_office")?;
    changed |= set_markitdown_bash_permission(markitdown_shim, false)?;
    changed |= remove_managed_block(&markitdown_codex_agents_path(), "markitdown")?;
    Ok(changed)
}

fn remove_markitdown_hook_if_present(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(path)
        .with_context(|| format!("removing MarkItDown hook {}", path.display()))?;
    Ok(true)
}

fn remove_markitdown_cache_if_present(path: &Path) -> Result<bool> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("inspecting MarkItDown cache {}", path.display()))
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        std::fs::remove_file(path)
            .with_context(|| format!("removing MarkItDown cache {}", path.display()))?;
    } else {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("removing MarkItDown cache {}", path.display()))?;
    }
    Ok(true)
}

fn caveman_claude_md_path() -> PathBuf {
    home_dir().join(".claude").join("CLAUDE.md")
}

fn caveman_codex_agents_path() -> PathBuf {
    codex_home().join("AGENTS.md")
}

/// The exact Switchboard-owned Caveman blocks for configured clients. The
/// snapshot stores no absolute paths: client IDs are stable and the blocks are
/// narrowly limited to the managed guidance that selective rollback owns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CavemanIntegrationSnapshot {
    pub blocks: BTreeMap<String, String>,
}

fn configured_caveman_clients() -> Vec<(&'static str, PathBuf)> {
    let mut clients = Vec::new();
    if is_claude_code_enabled() {
        clients.push(("claude-code", caveman_claude_md_path()));
    }
    if is_codex_enabled() {
        clients.push(("codex", caveman_codex_agents_path()));
    }
    clients
}

fn caveman_client_path(client_id: &str) -> Option<PathBuf> {
    match client_id {
        "claude-code" => Some(caveman_claude_md_path()),
        "codex" => Some(caveman_codex_agents_path()),
        _ => None,
    }
}

fn caveman_block_range(content: &str) -> Option<(usize, usize)> {
    for slug in SwitchboardIdentitySlug::marker_prefixes() {
        let start = managed_marker_start(slug.as_str(), "caveman");
        let end = managed_marker_end(slug.as_str(), "caveman");
        if let (Some(start_index), Some(end_index)) = (content.find(&start), content.find(&end)) {
            if start_index < end_index {
                return Some((start_index, end_index + end.len()));
            }
        }
    }
    None
}

fn caveman_block_at(path: &Path) -> Result<Option<String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    Ok(caveman_block_range(&content).map(|(start, end)| content[start..end].to_string()))
}

fn replace_caveman_block(path: &Path, replacement: Option<&str>) -> Result<bool> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let Some((start, end)) = caveman_block_range(&content) else {
        if replacement.is_none() {
            return Ok(false);
        }
        return Err(anyhow!(
            "Caveman marker is missing from {} after fingerprint validation",
            path.display()
        ));
    };
    let updated = if let Some(replacement) = replacement {
        format!("{}{}{}", &content[..start], replacement, &content[end..])
    } else {
        let before = content[..start].trim_end();
        let after = content[end..].trim_start_matches('\n');
        match (before.is_empty(), after.is_empty()) {
            (true, true) => String::new(),
            (true, false) => format!("{after}\n"),
            (false, true) => format!("{before}\n"),
            (false, false) => format!("{before}\n{after}"),
        }
    };
    if updated == content {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let _ = backup_if_exists(path)?;
    std::fs::write(path, updated).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

fn caveman_block_fingerprint(block: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(block.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Terse-output guidance body keyed by level. Scoped is the conservative
/// default: terse only where short output is safe, never hiding required
/// legal, safety, or debugging detail. Aggressive asks for terseness broadly.
/// Compact Chinese is experimental and only for internal working notes.
fn build_caveman_nudge(level: &str) -> String {
    match level {
        "aggressive" => "## Terse output (Switchboard Caveman, aggressive)\n\
             Default to terse output everywhere. Lead with the answer or result; cut\n\
             preamble, restated questions, and summaries of what you just did. Prefer\n\
             fragments and short synonyms. Still include any legal, safety, or\n\
             debugging detail the task actually requires -- brevity never overrides\n\
             correctness or required disclosure."
            .to_string(),
        "compact_chinese" => {
            "## Terse output (Switchboard Caveman, compact Chinese experimental)\n\
             Use compact Chinese only for private internal planning notes, scratch\n\
             handoffs, and hidden working prompts when that reduces tokens. Keep all\n\
             user-visible replies, commit messages, PR notes, legal, safety,\n\
             debugging, and release-readiness content in the user's requested\n\
             language with complete required detail. Never translate code, commands,\n\
             file paths, identifiers, error text, secrets, citations, or quoted\n\
             source material. If compact Chinese could make verification ambiguous,\n\
             use terse English instead."
                .to_string()
        }
        _ => "## Terse output (Switchboard Caveman, scoped)\n\
             For command summaries, PR notes, and handoffs, keep output terse: lead\n\
             with the result and drop preamble and self-summaries. Do NOT shorten\n\
             legal, safety, or debugging content -- keep those complete even when the\n\
             surrounding prose is terse."
            .to_string(),
    }
}

/// Enables the Caveman addon: writes a Switchboard-owned managed guidance block
/// into the instruction file of each configured coding client (Claude Code's
/// `~/.claude/CLAUDE.md`, Codex's `~/.codex/AGENTS.md`). Pure guidance -- no
/// hook, runtime, or permission. Idempotent and safe to re-run.
pub fn enable_caveman_integration(level: &str) -> Result<(Vec<String>, Vec<String>)> {
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();
    let body = build_caveman_nudge(level);

    if is_claude_code_enabled() {
        let claude_md = caveman_claude_md_path();
        let (md_changed, md_backup) = upsert_managed_block(&claude_md, "caveman", &body)?;
        if md_changed {
            changed_files.push(claude_md.display().to_string());
        }
        if let Some(path) = md_backup {
            backup_files.push(path.display().to_string());
        }
    }

    if is_codex_enabled() {
        let agents = caveman_codex_agents_path();
        let (codex_changed, codex_backup) = upsert_managed_block(&agents, "caveman", &body)?;
        if codex_changed {
            changed_files.push(agents.display().to_string());
        }
        if let Some(path) = codex_backup {
            backup_files.push(path.display().to_string());
        }
    }

    Ok((changed_files, backup_files))
}

pub fn caveman_integration_snapshot() -> Result<CavemanIntegrationSnapshot> {
    let mut blocks = BTreeMap::new();
    for (client_id, path) in configured_caveman_clients() {
        if let Some(block) = caveman_block_at(&path)? {
            blocks.insert(client_id.to_string(), block);
        }
    }
    Ok(CavemanIntegrationSnapshot { blocks })
}

/// Restores one logical client's previous Caveman block only when the current
/// block still exactly matches the activation's post-state. This protects a
/// user edit while allowing unrelated instructions in the same file to evolve.
pub fn restore_caveman_client_if_unchanged(
    client_id: &str,
    previous_block: Option<&str>,
    after_block: Option<&str>,
) -> Result<bool> {
    let path = caveman_client_path(client_id)
        .ok_or_else(|| anyhow!("unknown Caveman client identifier: {client_id}"))?;
    let current = caveman_block_at(&path)?;
    if current.as_deref() != after_block {
        let expected = after_block.map(caveman_block_fingerprint);
        let actual = current.as_deref().map(caveman_block_fingerprint);
        bail!(
            "Caveman block changed after activation for {client_id} (expected {}, found {})",
            expected.as_deref().unwrap_or("absent"),
            actual.as_deref().unwrap_or("absent")
        );
    }
    replace_caveman_block(&path, previous_block)
}

pub fn caveman_integration_matches_level(level: &str) -> Result<bool> {
    let expected = build_caveman_nudge(level);
    if is_claude_code_enabled()
        && !managed_block_contains_text(&caveman_claude_md_path(), "caveman", &expected)?
    {
        return Ok(false);
    }
    if is_codex_enabled()
        && !managed_block_contains_text(&caveman_codex_agents_path(), "caveman", &expected)?
    {
        return Ok(false);
    }
    Ok(true)
}

/// Removes the managed Caveman block from every client instruction file. Runs
/// unconditionally so a later-disconnected client is still scrubbed.
pub fn disable_caveman_integration() -> Result<bool> {
    let mut changed = remove_managed_block(&caveman_claude_md_path(), "caveman")?;
    changed |= remove_managed_block(&caveman_codex_agents_path(), "caveman")?;
    Ok(changed)
}

/// Adds or removes a `Bash(<shim> *)` entry in `permissions.allow` so the Office
/// nudge can run `markitdown` without prompting. Returns whether settings changed.
fn set_markitdown_bash_permission(shim_path: &Path, present: bool) -> Result<bool> {
    let settings_path = claude_settings_path();
    let entry = format!("Bash({} *)", shim_path.display());

    let mut content = if settings_path.exists() {
        let raw = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("reading {}", settings_path.display()))?;
        if raw.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            Value::Object(parse_json_object(&raw, &settings_path)?)
        }
    } else if present {
        Value::Object(Default::default())
    } else {
        return Ok(false);
    };

    let root = content
        .as_object_mut()
        .ok_or_else(|| anyhow!("unable to write Claude permissions settings"))?;
    let allow = root
        .entry("permissions")
        .or_insert_with(|| Value::Object(Default::default()))
        .as_object_mut()
        .ok_or_else(|| anyhow!("permissions is not an object"))?
        .entry("allow")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| anyhow!("permissions.allow is not an array"))?;

    let already = allow.iter().any(|v| v.as_str() == Some(entry.as_str()));
    if present == already {
        return Ok(false);
    }
    if present {
        allow.push(Value::String(entry));
    } else {
        allow.retain(|v| v.as_str() != Some(entry.as_str()));
    }

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let _ = backup_if_exists(&settings_path)?;
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&content).context("serializing Claude permissions settings")?,
    )
    .with_context(|| format!("writing {}", settings_path.display()))?;
    Ok(true)
}
pub(crate) fn build_headroom_markitdown_hook(markitdown_path: &Path, python_path: &Path) -> String {
    let markitdown = shell_double_quote(&markitdown_path.to_string_lossy());
    let python = shell_double_quote(&python_path.to_string_lossy());

    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

HEADROOM_MARKITDOWN="{markitdown}"
HEADROOM_PYTHON="{python}"

if [ ! -x "$HEADROOM_MARKITDOWN" ] || [ ! -x "$HEADROOM_PYTHON" ]; then
  exit 0
fi

INPUT="$(cat)"
if [ -z "$INPUT" ]; then
  exit 0
fi

HEADROOM_MD_CACHE="${{TMPDIR:-/tmp}}/headroom-markitdown"
mkdir -p "$HEADROOM_MD_CACHE" 2>/dev/null || exit 0

HEADROOM_MARKITDOWN_BIN="$HEADROOM_MARKITDOWN" HEADROOM_MD_CACHE="$HEADROOM_MD_CACHE" "$HEADROOM_PYTHON" -c 'import json, os, sys, subprocess, hashlib
ALLOWED = {{".pdf"}}
MAX_BYTES = 25 * 1024 * 1024
try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
tool_input = data.get("tool_input")
if not isinstance(tool_input, dict):
    sys.exit(0)
fp = tool_input.get("file_path")
if not isinstance(fp, str) or not fp:
    sys.exit(0)
if os.path.splitext(fp)[1].lower() not in ALLOWED:
    sys.exit(0)
try:
    st = os.stat(fp)
except OSError:
    sys.exit(0)
if st.st_size > MAX_BYTES:
    sys.exit(0)
binpath = os.environ["HEADROOM_MARKITDOWN_BIN"]
cache = os.environ["HEADROOM_MD_CACHE"]
key = hashlib.sha256((os.path.abspath(fp) + ":" + str(st.st_mtime_ns)).encode()).hexdigest()[:16]
out = os.path.join(cache, key + ".md")
if not (os.path.exists(out) and os.path.getsize(out) > 0):
    try:
        subprocess.run([binpath, fp, "-o", out], check=True, capture_output=True, timeout=120)
    except Exception:
        sys.exit(0)
if not (os.path.exists(out) and os.path.getsize(out) > 0):
    sys.exit(0)
updated = dict(tool_input)
updated["file_path"] = out
json.dump({{"hookSpecificOutput": {{"hookEventName": "PreToolUse", "permissionDecision": "allow", "permissionDecisionReason": "Headroom MarkItDown conversion", "updatedInput": updated}}}}, sys.stdout)' <<<"$INPUT" 2>/dev/null || exit 0
"#
    )
}

fn shell_double_quote(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

#[cfg(test)]
mod tests {
    use super::{
        caveman_block_fingerprint, caveman_block_range, remove_markitdown_cache_if_present,
        remove_markitdown_hook_if_present,
    };

    #[test]
    fn orphaned_markitdown_hook_removal_reports_change() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let hook = directory.path().join("headroom-markitdown-read.sh");
        std::fs::write(&hook, "#!/bin/sh\n").expect("write hook");

        assert!(remove_markitdown_hook_if_present(&hook).expect("remove hook"));
        assert!(!hook.exists());
        assert!(!remove_markitdown_hook_if_present(&hook).expect("missing hook is a no-op"));
    }

    #[test]
    fn markitdown_cache_cleanup_is_bounded_and_idempotent() {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let cache = directory.path().join("headroom-markitdown");
        std::fs::create_dir(&cache).expect("create cache");
        std::fs::write(cache.join("converted.md"), "local document").expect("write cache entry");

        assert!(remove_markitdown_cache_if_present(&cache).expect("remove cache"));
        assert!(!cache.exists());
        assert!(!remove_markitdown_cache_if_present(&cache).expect("missing cache is a no-op"));
    }

    #[test]
    fn caveman_fingerprint_isolated_to_the_managed_block() {
        let content = concat!(
            "# User instruction\n",
            "# >>> headroom:caveman >>>\n",
            "## Terse output\n",
            "# <<< headroom:caveman <<<\n",
            "# More user instruction\n"
        );
        let (start, end) = caveman_block_range(content).expect("managed Caveman block");
        assert_eq!(
            &content[start..end],
            "# >>> headroom:caveman >>>\n## Terse output\n# <<< headroom:caveman <<<"
        );
        assert_ne!(
            caveman_block_fingerprint(&content[start..end]),
            caveman_block_fingerprint(content)
        );
    }
}
