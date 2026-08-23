use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::client_detection::codex_home;
use crate::client_paths::{claude_settings_path, headroom_markitdown_hook_path, home_dir};
use crate::client_setup_apply::{
    ensure_claude_settings_hook, entry_contains_hook, managed_block_contains_text,
    remove_pre_tool_use_markers,
};
use crate::client_setup_state::{is_claude_code_enabled, is_codex_enabled};
use crate::managed_files::{
    atomic_write_bytes_if_absent, atomic_write_bytes_if_unchanged, backup_if_exists,
    managed_marker_end, managed_marker_start, parse_json_object, remove_managed_block,
    upsert_managed_block, write_file_if_changed,
};
use crate::switchboard_identity::{primary_marker_prefix, SwitchboardIdentitySlug};

fn markitdown_claude_md_path() -> PathBuf {
    home_dir().join(".claude").join("CLAUDE.md")
}

fn markitdown_codex_agents_path() -> PathBuf {
    codex_home().join("AGENTS.md")
}

const MARKITDOWN_HOOK_MARKER: &str = "headroom-markitdown-read.sh";
const MARKITDOWN_CLAUDE_HOOK_ARTIFACT: &str = "claude-hook";
const MARKITDOWN_CLAUDE_SETTINGS_ARTIFACT: &str = "claude-settings-hook";
const MARKITDOWN_CLAUDE_OFFICE_ARTIFACT: &str = "claude-office-nudge";
const MARKITDOWN_CLAUDE_PERMISSION_ARTIFACT: &str = "claude-bash-permission";
const MARKITDOWN_CODEX_NUDGE_ARTIFACT: &str = "codex-nudge";

/// Content-free fingerprints of the individual MarkItDown integration
/// artifacts that can be created by selective activation. Logical IDs avoid
/// recording paths, instruction contents, or permission text in receipts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarkitdownIntegrationSnapshot {
    pub artifacts: BTreeMap<String, String>,
}

fn markitdown_fingerprint(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_ref());
    format!("sha256:{:x}", hasher.finalize())
}

fn canonical_markitdown_block(block_id: &str, body: &str) -> String {
    let start = managed_marker_start(primary_marker_prefix(), block_id);
    let end = managed_marker_end(primary_marker_prefix(), block_id);
    format!("{start}\n{body}\n{end}")
}

fn markitdown_managed_block(path: &Path, block_id: &str) -> Result<Option<String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let mut matches = Vec::new();
    for slug in SwitchboardIdentitySlug::marker_prefixes() {
        let start = managed_marker_start(slug.as_str(), block_id);
        let end = managed_marker_end(slug.as_str(), block_id);
        let starts = content.match_indices(&start).count();
        let ends = content.match_indices(&end).count();
        if starts == 0 && ends == 0 {
            continue;
        }
        if starts != 1 || ends != 1 {
            bail!(
                "MarkItDown managed block is ambiguous in {}; selective activation will not overwrite it",
                path.display()
            );
        }
        let start_index = content
            .find(&start)
            .expect("counted MarkItDown marker must exist");
        let end_index = content
            .find(&end)
            .expect("counted MarkItDown marker must exist");
        if start_index >= end_index {
            bail!(
                "MarkItDown managed block markers are malformed in {}; selective activation will not overwrite it",
                path.display()
            );
        }
        matches.push(content[start_index..end_index + end.len()].to_string());
    }
    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => bail!(
            "multiple MarkItDown marker variants are present in {}; selective activation will not overwrite them",
            path.display()
        ),
    }
}

fn markitdown_settings_hook() -> Result<Option<Value>> {
    let settings_path = claude_settings_path();
    let raw = match std::fs::read_to_string(&settings_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("reading {}", settings_path.display()))
        }
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let root = parse_json_object(&raw, &settings_path)?;
    let matches = root
        .get("hooks")
        .and_then(Value::as_object)
        .and_then(|hooks| hooks.get("PreToolUse"))
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| entry_contains_hook(entry, MARKITDOWN_HOOK_MARKER))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => bail!(
            "multiple MarkItDown Claude settings hooks are present; selective activation will not overwrite them"
        ),
    }
}

fn markitdown_permission_entry(shim_path: &Path) -> Result<Option<Value>> {
    let settings_path = claude_settings_path();
    let raw = match std::fs::read_to_string(&settings_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("reading {}", settings_path.display()))
        }
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let root = parse_json_object(&raw, &settings_path)?;
    let expected = format!("Bash({} *)", shim_path.display());
    let matches = root
        .get("permissions")
        .and_then(Value::as_object)
        .and_then(|permissions| permissions.get("allow"))
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| entry.as_str() == Some(expected.as_str()))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => bail!(
            "multiple MarkItDown Claude Bash permissions are present; selective activation will not overwrite them"
        ),
    }
}

fn markitdown_artifact_fingerprint(id: &str, shim_path: &Path) -> Result<Option<String>> {
    match id {
        MARKITDOWN_CLAUDE_HOOK_ARTIFACT => match std::fs::read(headroom_markitdown_hook_path()) {
            Ok(bytes) => Ok(Some(markitdown_fingerprint(bytes))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("reading managed MarkItDown Claude hook"),
        },
        MARKITDOWN_CLAUDE_SETTINGS_ARTIFACT => markitdown_settings_hook()?
            .map(|entry| serde_json::to_vec(&entry).map(markitdown_fingerprint))
            .transpose()
            .context("serializing MarkItDown Claude settings hook"),
        MARKITDOWN_CLAUDE_OFFICE_ARTIFACT => {
            markitdown_managed_block(&markitdown_claude_md_path(), "markitdown_office")
                .map(|block| block.map(markitdown_fingerprint))
        }
        MARKITDOWN_CLAUDE_PERMISSION_ARTIFACT => markitdown_permission_entry(shim_path)?
            .map(|entry| serde_json::to_vec(&entry).map(markitdown_fingerprint))
            .transpose()
            .context("serializing MarkItDown Claude Bash permission"),
        MARKITDOWN_CODEX_NUDGE_ARTIFACT => {
            markitdown_managed_block(&markitdown_codex_agents_path(), "markitdown")
                .map(|block| block.map(markitdown_fingerprint))
        }
        _ => bail!("unknown MarkItDown integration artifact: {id}"),
    }
}

fn expected_markitdown_artifact_fingerprint(
    id: &str,
    markitdown_entrypoint: &Path,
    markitdown_shim: &Path,
    python_path: &Path,
) -> Result<String> {
    match id {
        MARKITDOWN_CLAUDE_HOOK_ARTIFACT => Ok(markitdown_fingerprint(
            build_headroom_markitdown_hook(markitdown_entrypoint, python_path),
        )),
        MARKITDOWN_CLAUDE_SETTINGS_ARTIFACT => {
            let hook_path = headroom_markitdown_hook_path();
            let command = hook_path
                .to_str()
                .context("MarkItDown hook path contains invalid UTF-8")?;
            serde_json::to_vec(&serde_json::json!({
                "matcher": "Read",
                "hooks": [{ "type": "command", "command": command }]
            }))
            .map(markitdown_fingerprint)
            .context("serializing expected MarkItDown Claude settings hook")
        }
        MARKITDOWN_CLAUDE_OFFICE_ARTIFACT => {
            Ok(markitdown_fingerprint(canonical_markitdown_block(
                "markitdown_office",
                &build_markitdown_office_nudge(markitdown_shim),
            )))
        }
        MARKITDOWN_CLAUDE_PERMISSION_ARTIFACT => serde_json::to_vec(&Value::String(format!(
            "Bash({} *)",
            markitdown_shim.display()
        )))
        .map(markitdown_fingerprint)
        .context("serializing expected MarkItDown Claude Bash permission"),
        MARKITDOWN_CODEX_NUDGE_ARTIFACT => Ok(markitdown_fingerprint(canonical_markitdown_block(
            "markitdown",
            &build_markitdown_codex_nudge(markitdown_shim),
        ))),
        _ => bail!("unknown MarkItDown integration artifact: {id}"),
    }
}

const MARKITDOWN_ARTIFACT_IDS: [&str; 5] = [
    MARKITDOWN_CLAUDE_HOOK_ARTIFACT,
    MARKITDOWN_CLAUDE_SETTINGS_ARTIFACT,
    MARKITDOWN_CLAUDE_OFFICE_ARTIFACT,
    MARKITDOWN_CLAUDE_PERMISSION_ARTIFACT,
    MARKITDOWN_CODEX_NUDGE_ARTIFACT,
];

/// Snapshots all managed MarkItDown artifacts, including ones whose client was
/// later disconnected, so an external change can be preserved rather than
/// silently overwritten during a future selective activation.
pub fn markitdown_integration_snapshot(
    markitdown_shim: &Path,
) -> Result<MarkitdownIntegrationSnapshot> {
    let mut artifacts = BTreeMap::new();
    for id in MARKITDOWN_ARTIFACT_IDS {
        if let Some(fingerprint) = markitdown_artifact_fingerprint(id, markitdown_shim)? {
            artifacts.insert(id.to_string(), fingerprint);
        }
    }
    Ok(MarkitdownIntegrationSnapshot { artifacts })
}

/// Reject custom, legacy, malformed, and partial integration artifacts before
/// activation can replace any of them. Absent artifacts are created only for
/// currently configured clients and are tracked individually in the receipt.
pub fn validate_markitdown_integration_snapshot(
    snapshot: &MarkitdownIntegrationSnapshot,
    markitdown_entrypoint: &Path,
    markitdown_shim: &Path,
    python_path: &Path,
) -> Result<()> {
    for (id, actual) in &snapshot.artifacts {
        let expected = expected_markitdown_artifact_fingerprint(
            id,
            markitdown_entrypoint,
            markitdown_shim,
            python_path,
        )?;
        if actual != &expected {
            bail!(
                "MarkItDown {id} has custom or legacy content; repair it from Addons before selective activation so existing configuration is preserved"
            );
        }
    }
    Ok(())
}

pub fn newly_created_markitdown_artifacts(
    previous: &MarkitdownIntegrationSnapshot,
    after: &MarkitdownIntegrationSnapshot,
) -> BTreeMap<String, String> {
    after
        .artifacts
        .iter()
        .filter(|(id, _)| !previous.artifacts.contains_key(*id))
        .map(|(id, fingerprint)| (id.clone(), fingerprint.clone()))
        .collect()
}

/// Removes one run-created MarkItDown artifact only if its current logical
/// content still matches the exact post-activation fingerprint. This never
/// invokes the broad disable helper, which is intended for explicit Addons
/// cleanup and can remove pre-existing integration entries.
pub fn remove_markitdown_artifact_if_unchanged(
    id: &str,
    after_fingerprint: &str,
    markitdown_shim: &Path,
) -> Result<bool> {
    let current = markitdown_artifact_fingerprint(id, markitdown_shim)?;
    if current.as_deref() != Some(after_fingerprint) {
        bail!(
            "MarkItDown {id} changed after activation (expected {}, found {})",
            after_fingerprint,
            current.as_deref().unwrap_or("absent")
        );
    }
    match id {
        MARKITDOWN_CLAUDE_HOOK_ARTIFACT => std::fs::remove_file(headroom_markitdown_hook_path())
            .context("removing run-created MarkItDown Claude hook")
            .map(|_| true),
        MARKITDOWN_CLAUDE_SETTINGS_ARTIFACT => {
            remove_pre_tool_use_markers(&claude_settings_path(), &[MARKITDOWN_HOOK_MARKER])
        }
        MARKITDOWN_CLAUDE_OFFICE_ARTIFACT => {
            remove_managed_block(&markitdown_claude_md_path(), "markitdown_office")
        }
        MARKITDOWN_CLAUDE_PERMISSION_ARTIFACT => {
            set_markitdown_bash_permission(markitdown_shim, false)
        }
        MARKITDOWN_CODEX_NUDGE_ARTIFACT => {
            remove_managed_block(&markitdown_codex_agents_path(), "markitdown")
        }
        _ => bail!("unknown MarkItDown integration artifact: {id}"),
    }
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

/// A non-sensitive observation of a single Switchboard-owned Caveman block.
/// `level` is present only for canonical guidance that this build can recreate;
/// custom or legacy content is recorded as an opaque fingerprint and is never
/// overwritten by the rollback-capable activation flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CavemanManagedBlockSnapshot {
    pub level: Option<String>,
    pub fingerprint: String,
}

/// Switchboard-owned Caveman blocks for configured clients. The receipt uses
/// logical client IDs and canonical levels/fingerprints only—never paths,
/// whole instruction files, or arbitrary user-authored prompt text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CavemanIntegrationSnapshot {
    pub blocks: BTreeMap<String, CavemanManagedBlockSnapshot>,
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

fn canonical_caveman_block(level: &str) -> String {
    let start = managed_marker_start(primary_marker_prefix(), "caveman");
    let end = managed_marker_end(primary_marker_prefix(), "caveman");
    format!("{start}\n{}\n{end}", build_caveman_nudge(level))
}

fn caveman_block_snapshot(block: &str) -> CavemanManagedBlockSnapshot {
    let level = ["scoped", "aggressive", "compact_chinese"]
        .into_iter()
        .find(|level| canonical_caveman_block(level) == block)
        .map(str::to_string);
    CavemanManagedBlockSnapshot {
        level,
        fingerprint: caveman_block_fingerprint(block),
    }
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
            blocks.insert(client_id.to_string(), caveman_block_snapshot(&block));
        }
    }
    Ok(CavemanIntegrationSnapshot { blocks })
}

/// Restores one logical client's previous Caveman block only when the current
/// block still exactly matches the activation's post-state. This protects a
/// user edit while allowing unrelated instructions in the same file to evolve.
pub fn restore_caveman_client_if_unchanged(
    client_id: &str,
    previous_block: Option<&CavemanManagedBlockSnapshot>,
    after_block: Option<&CavemanManagedBlockSnapshot>,
) -> Result<bool> {
    let path = caveman_client_path(client_id)
        .ok_or_else(|| anyhow!("unknown Caveman client identifier: {client_id}"))?;
    let current = caveman_block_at(&path)?
        .as_deref()
        .map(caveman_block_snapshot);
    if current.as_ref() != after_block {
        let expected = after_block.map(|block| block.fingerprint.as_str());
        let actual = current.as_ref().map(|block| block.fingerprint.as_str());
        bail!(
            "Caveman block changed after activation for {client_id} (expected {}, found {})",
            expected.as_deref().unwrap_or("absent"),
            actual.as_deref().unwrap_or("absent")
        );
    }
    let replacement = match previous_block {
        Some(previous_block) => {
            let level = previous_block
                .level
                .as_deref()
                .context("Caveman block predates the canonical rollback contract")?;
            Some(canonical_caveman_block(level))
        }
        None => None,
    };
    replace_caveman_block(&path, replacement.as_deref())
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

fn configured_ponytail_clients() -> Vec<(&'static str, PathBuf)> {
    configured_caveman_clients()
}

fn all_ponytail_clients() -> [(&'static str, PathBuf); 2] {
    [
        ("claude-code", caveman_claude_md_path()),
        ("codex", caveman_codex_agents_path()),
    ]
}

fn ponytail_client_path(client_id: &str) -> Option<PathBuf> {
    caveman_client_path(client_id)
}

fn ponytail_block_range(content: &str) -> Result<Option<(usize, usize)>> {
    let mut found = None;
    for slug in SwitchboardIdentitySlug::marker_prefixes() {
        let start = managed_marker_start(slug.as_str(), "ponytail");
        let end = managed_marker_end(slug.as_str(), "ponytail");
        let starts: Vec<_> = content.match_indices(&start).collect();
        let ends: Vec<_> = content.match_indices(&end).collect();
        if starts.is_empty() && ends.is_empty() {
            continue;
        }
        if starts.len() != 1 || ends.len() != 1 || starts[0].0 >= ends[0].0 {
            bail!("Ponytail managed block markers are ambiguous or malformed");
        }
        if found.is_some() {
            bail!("multiple Ponytail managed blocks were found");
        }
        found = Some((starts[0].0, ends[0].0 + end.len()));
    }
    Ok(found)
}

fn ponytail_block_at(path: &Path) -> Result<Option<String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    Ok(ponytail_block_range(&content)?.map(|(start, end)| content[start..end].to_string()))
}

fn ponytail_block_fingerprint(block: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(block.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn build_ponytail_nudge() -> Result<String> {
    Ok(format!(
        "## Ponytail (AI Switchboard bundled profile)\n\
         Source: DietrichGebert/ponytail 4.9.0 at commit {} (MIT). AI Switchboard\n\
         applies the reviewed core guidance below without marketplace installation,\n\
         lifecycle hooks, runtime downloads, or automatic updates.\n\n{}",
        crate::ponytail_bundled::PONYTAIL_SOURCE_COMMIT,
        crate::ponytail_bundled::core_guidance()?
    ))
}

fn canonical_ponytail_block() -> Result<String> {
    let start = managed_marker_start(primary_marker_prefix(), "ponytail");
    let end = managed_marker_end(primary_marker_prefix(), "ponytail");
    Ok(format!("{start}\n{}\n{end}", build_ponytail_nudge()?))
}

#[derive(Debug, Default)]
pub struct PonytailGuidanceRefresh {
    previous_blocks: BTreeMap<String, String>,
    refreshed_fingerprints: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
pub struct PonytailRemovalSnapshot {
    removed_blocks: BTreeMap<String, String>,
}

impl PonytailRemovalSnapshot {
    fn is_empty(&self) -> bool {
        self.removed_blocks.is_empty()
    }
}

fn replace_ponytail_block_if_unchanged(
    client_id: &str,
    expected_fingerprint: &str,
    replacement_block: &str,
) -> Result<String> {
    let path = ponytail_client_path(client_id)
        .ok_or_else(|| anyhow!("unknown Ponytail client identifier: {client_id}"))?;
    let existing =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let (start, end) =
        ponytail_block_range(&existing)?.context("Ponytail managed block is no longer present")?;
    let previous_block = existing[start..end].to_string();
    if ponytail_block_fingerprint(&previous_block) != expected_fingerprint {
        bail!("Ponytail managed block changed after activation for {client_id}");
    }
    if previous_block == replacement_block {
        return Ok(previous_block);
    }
    let mut replacement = String::with_capacity(
        existing.len() + replacement_block.len().saturating_sub(previous_block.len()),
    );
    replacement.push_str(&existing[..start]);
    replacement.push_str(replacement_block);
    replacement.push_str(&existing[end..]);
    backup_if_exists(&path)?;
    atomic_write_bytes_if_unchanged(&path, existing.as_bytes(), replacement.as_bytes())?;
    Ok(previous_block)
}

pub fn restore_ponytail_guidance_refresh(refresh: &PonytailGuidanceRefresh) -> Result<()> {
    let mut failures = Vec::new();
    for (client_id, previous_block) in refresh.previous_blocks.iter().rev() {
        let Some(expected_fingerprint) = refresh.refreshed_fingerprints.get(client_id) else {
            failures.push(format!("missing refreshed fingerprint for {client_id}"));
            continue;
        };
        if let Err(error) =
            replace_ponytail_block_if_unchanged(client_id, expected_fingerprint, previous_block)
        {
            failures.push(format!("{client_id}: {error:#}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        bail!(
            "restoring the previous Ponytail managed profile failed: {}",
            failures.join("; ")
        )
    }
}

pub fn refresh_ponytail_guidance_if_unchanged(
    expected: &BTreeMap<String, String>,
) -> Result<PonytailGuidanceRefresh> {
    let replacement = canonical_ponytail_block()?;
    let replacement_fingerprint = ponytail_block_fingerprint(&replacement);
    let mut refresh = PonytailGuidanceRefresh::default();
    for (client_id, expected_fingerprint) in expected {
        let path = ponytail_client_path(client_id)
            .ok_or_else(|| anyhow!("unknown Ponytail client identifier: {client_id}"))?;
        let Some(current_block) = ponytail_block_at(&path)? else {
            continue;
        };
        if ponytail_block_fingerprint(&current_block) != *expected_fingerprint {
            let primary = anyhow!(
                "Ponytail managed block changed after activation for {client_id}; it was preserved"
            );
            return match restore_ponytail_guidance_refresh(&refresh) {
                Ok(()) => Err(primary),
                Err(rollback) => Err(anyhow!(
                    "{primary:#}; compensating profile restoration also failed: {rollback:#}"
                )),
            };
        }
        if current_block == replacement {
            continue;
        }
        match replace_ponytail_block_if_unchanged(client_id, expected_fingerprint, &replacement) {
            Ok(previous_block) => {
                refresh
                    .previous_blocks
                    .insert(client_id.clone(), previous_block);
                refresh
                    .refreshed_fingerprints
                    .insert(client_id.clone(), replacement_fingerprint.clone());
            }
            Err(primary) => {
                return match restore_ponytail_guidance_refresh(&refresh) {
                    Ok(()) => Err(primary),
                    Err(rollback) => Err(anyhow!(
                        "{primary:#}; compensating profile restoration also failed: {rollback:#}"
                    )),
                };
            }
        }
    }
    Ok(refresh)
}

pub fn ponytail_integration_fingerprints() -> Result<BTreeMap<String, String>> {
    let mut fingerprints = BTreeMap::new();
    for (client_id, path) in all_ponytail_clients() {
        if let Some(block) = ponytail_block_at(&path)? {
            fingerprints.insert(client_id.to_string(), ponytail_block_fingerprint(&block));
        }
    }
    Ok(fingerprints)
}

pub fn ponytail_integration_fingerprint(client_id: &str) -> Result<Option<String>> {
    let path = ponytail_client_path(client_id)
        .ok_or_else(|| anyhow!("unknown Ponytail client identifier: {client_id}"))?;
    ponytail_block_at(&path).map(|block| block.as_deref().map(ponytail_block_fingerprint))
}

pub fn ponytail_registered_clients() -> Result<Vec<String>> {
    Ok(ponytail_integration_fingerprints()?.into_keys().collect())
}

/// Installs the immutable app-bundled Ponytail core profile into the same
/// Switchboard-owned instruction-file boundary used by Caveman. Existing
/// non-canonical managed content is preserved and blocks activation.
pub fn enable_ponytail_integration() -> Result<(Vec<String>, Vec<String>)> {
    let clients = configured_ponytail_clients();
    if clients.is_empty() {
        bail!("Ponytail needs a configured Claude Code or Codex client");
    }
    let canonical = canonical_ponytail_block()?;
    for (_, path) in &clients {
        if let Some(existing) = ponytail_block_at(path)? {
            if existing != canonical {
                bail!(
                    "Ponytail managed guidance differs in {}; it was preserved",
                    path.display()
                );
            }
        }
    }

    let body = build_ponytail_nudge()?;
    let mut changed_files = Vec::new();
    let mut backup_files = Vec::new();
    let canonical_fingerprint = ponytail_block_fingerprint(&canonical);
    let mut created_blocks: Vec<(String, String)> = Vec::new();
    for (client_id, path) in clients {
        let result = upsert_managed_block(&path, "ponytail", &body);
        let (changed, backup) = match result {
            Ok(result) => result,
            Err(error) => {
                let mut cleanup_errors = Vec::new();
                for (created_client, expected_fingerprint) in created_blocks.iter().rev() {
                    if let Err(cleanup) =
                        remove_ponytail_client_if_unchanged(created_client, expected_fingerprint)
                    {
                        cleanup_errors.push(cleanup.to_string());
                    }
                }
                if cleanup_errors.is_empty() {
                    return Err(error)
                        .context("writing bundled Ponytail guidance; partial writes rolled back");
                }
                bail!(
                    "writing bundled Ponytail guidance failed: {error:#}; rollback also failed: {}",
                    cleanup_errors.join("; ")
                );
            }
        };
        if changed {
            changed_files.push(path.display().to_string());
            created_blocks.push((client_id.to_string(), canonical_fingerprint.clone()));
        }
        if let Some(backup) = backup {
            backup_files.push(backup.display().to_string());
        }
    }
    Ok((changed_files, backup_files))
}

pub fn ponytail_integration_matches() -> Result<bool> {
    let clients = configured_ponytail_clients();
    if clients.is_empty() {
        return Ok(false);
    }
    let canonical = canonical_ponytail_block()?;
    for (_, path) in clients {
        if ponytail_block_at(&path)?.as_deref() != Some(canonical.as_str()) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn disable_ponytail_integration() -> Result<bool> {
    let canonical_fingerprint = ponytail_block_fingerprint(&canonical_ponytail_block()?);
    let current = ponytail_integration_fingerprints()?;
    if current
        .values()
        .any(|value| value != &canonical_fingerprint)
    {
        bail!("Ponytail managed guidance changed after activation; it was preserved");
    }
    disable_ponytail_integration_if_unchanged(&current).map(|snapshot| !snapshot.is_empty())
}

pub fn disable_ponytail_integration_if_unchanged(
    expected: &BTreeMap<String, String>,
) -> Result<PonytailRemovalSnapshot> {
    for (client_id, fingerprint) in expected {
        let path = ponytail_client_path(client_id)
            .ok_or_else(|| anyhow!("unknown Ponytail client identifier: {client_id}"))?;
        let current = ponytail_block_at(&path)?
            .as_deref()
            .map(ponytail_block_fingerprint);
        if current.as_deref() != Some(fingerprint.as_str()) {
            bail!("Ponytail managed guidance changed after activation for {client_id}");
        }
    }

    let mut removed = PonytailRemovalSnapshot::default();
    for (client_id, expected_fingerprint) in expected {
        match remove_ponytail_client_if_unchanged(client_id, expected_fingerprint) {
            Ok(previous_block) => {
                removed
                    .removed_blocks
                    .insert(client_id.clone(), previous_block);
            }
            Err(error) => {
                if let Err(rollback) = restore_ponytail_removal(&removed) {
                    bail!(
                        "removing Ponytail guidance failed: {error:#}; restoring partial removals also failed: {rollback:#}"
                    );
                } else {
                    return Err(error)
                        .context("removing Ponytail guidance; partial removals restored");
                }
            }
        }
    }
    Ok(removed)
}

fn restore_ponytail_client_if_absent(client_id: &str, block: &str) -> Result<()> {
    let path = ponytail_client_path(client_id)
        .ok_or_else(|| anyhow!("unknown Ponytail client identifier: {client_id}"))?;
    let (existing, was_absent) = match std::fs::read_to_string(&path) {
        Ok(existing) => (existing, false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (String::new(), true),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    if let Some(current) =
        ponytail_block_range(&existing)?.map(|(start, end)| existing[start..end].to_string())
    {
        if current == block {
            return Ok(());
        }
        bail!("Ponytail managed guidance changed during compensation for {client_id}");
    }
    let replacement = if existing.trim().is_empty() {
        format!("{block}\n")
    } else {
        format!("{}\n{block}\n", existing.trim_end())
    };
    backup_if_exists(&path)?;
    if was_absent {
        atomic_write_bytes_if_absent(&path, replacement.as_bytes())?;
    } else {
        atomic_write_bytes_if_unchanged(&path, existing.as_bytes(), replacement.as_bytes())?;
    }
    Ok(())
}

pub fn restore_ponytail_removal(snapshot: &PonytailRemovalSnapshot) -> Result<()> {
    let mut failures = Vec::new();
    for (client_id, block) in &snapshot.removed_blocks {
        if let Err(error) = restore_ponytail_client_if_absent(client_id, block) {
            failures.push(format!("{client_id}: {error:#}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        bail!(
            "restoring exact Ponytail receipt-owned guidance failed: {}",
            failures.join("; ")
        )
    }
}

pub fn remove_ponytail_client_if_unchanged(
    client_id: &str,
    expected_fingerprint: &str,
) -> Result<String> {
    let path = ponytail_client_path(client_id)
        .ok_or_else(|| anyhow!("unknown Ponytail client identifier: {client_id}"))?;
    let existing =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let (start, end) =
        ponytail_block_range(&existing)?.context("Ponytail managed block is no longer present")?;
    let block = &existing[start..end];
    let actual = ponytail_block_fingerprint(&block);
    if actual != expected_fingerprint {
        bail!("Ponytail managed block changed after activation for {client_id}");
    }
    let tail = existing[end..].trim_start_matches('\n');
    let mut replacement = String::with_capacity(existing.len());
    replacement.push_str(existing[..start].trim_end());
    if !replacement.is_empty() && !tail.is_empty() {
        replacement.push('\n');
    }
    replacement.push_str(tail);
    if !replacement.is_empty() && !replacement.ends_with('\n') {
        replacement.push('\n');
    }
    backup_if_exists(&path)?;
    atomic_write_bytes_if_unchanged(&path, existing.as_bytes(), replacement.as_bytes())?;
    Ok(block.to_string())
}

#[cfg(test)]
mod ponytail_bundled_tests {
    use super::{canonical_ponytail_block, ponytail_block_range};

    #[test]
    fn bundled_ponytail_block_is_attributed_and_content_complete() {
        let block = canonical_ponytail_block().expect("canonical Ponytail block");
        assert!(block.contains("DietrichGebert/ponytail 4.9.0"));
        assert!(block.contains("The shortest path to done is the right path."));
        assert!(block.contains("ai-switchboard:ponytail"));
    }

    #[test]
    fn ponytail_block_parser_rejects_ambiguous_markers() {
        let canonical = canonical_ponytail_block().expect("canonical Ponytail block");
        assert_eq!(
            ponytail_block_range(&canonical).unwrap(),
            Some((0, canonical.len()))
        );
        assert!(ponytail_block_range(&format!("{canonical}\n{canonical}")).is_err());
    }
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
        canonical_caveman_block, caveman_block_fingerprint, caveman_block_range,
        caveman_block_snapshot, newly_created_markitdown_artifacts,
        remove_markitdown_cache_if_present, remove_markitdown_hook_if_present,
        MarkitdownIntegrationSnapshot,
    };
    use std::collections::BTreeMap;

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
    fn markitdown_delta_tracks_only_new_managed_artifacts() {
        let previous = MarkitdownIntegrationSnapshot {
            artifacts: BTreeMap::from([("claude-hook".into(), "before".into())]),
        };
        let after = MarkitdownIntegrationSnapshot {
            artifacts: BTreeMap::from([
                ("claude-hook".into(), "changed-but-preexisting".into()),
                ("codex-nudge".into(), "created".into()),
            ]),
        };
        assert_eq!(
            newly_created_markitdown_artifacts(&previous, &after),
            BTreeMap::from([("codex-nudge".into(), "created".into())])
        );
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

    #[test]
    fn caveman_snapshot_keeps_only_canonical_level_and_fingerprint() {
        let canonical = canonical_caveman_block("scoped");
        assert_eq!(
            caveman_block_snapshot(&canonical).level.as_deref(),
            Some("scoped")
        );
        assert!(caveman_block_snapshot("# custom caveman marker")
            .level
            .is_none());
    }
}
