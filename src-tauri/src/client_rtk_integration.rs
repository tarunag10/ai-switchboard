use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::client_claude_settings::{entry_contains_hook, remove_pre_tool_use_markers};
use crate::client_paths::{
    claude_settings_path, headroom_rtk_hook_path, resolve_default_shell_targets, rtk_codex_agents_path,
    shell_path, ALL_SHELL_FILES,
};
use crate::client_setup_apply::build_headroom_rtk_hook;
use crate::client_shell_setup::build_rtk_codex_nudge;
use crate::client_setup_state::{is_codex_enabled, load_setup_state, write_setup_state};
use crate::managed_files::{
    managed_marker_end, managed_marker_start, parse_json_object, remove_managed_block,
};
use crate::switchboard_identity::{primary_marker_prefix, SwitchboardIdentitySlug};

const CLAUDE_HOOK_ARTIFACT: &str = "claude-hook";
const CLAUDE_SETTINGS_ARTIFACT: &str = "claude-settings-hook";
const CODEX_NUDGE_ARTIFACT: &str = "codex-nudge";
const RTK_HOOK_MARKER: &str = "headroom-rtk-rewrite.sh";

/// Content-free view of the RTK artifacts that selective activation may own.
/// Logical identifiers and hashes make rollback auditable without putting a
/// home path, shell configuration, or instructions into the activation receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RtkIntegrationSnapshot {
    pub artifacts: BTreeMap<String, String>,
}

fn fingerprint(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_ref());
    format!("sha256:{:x}", hasher.finalize())
}

fn shell_artifact_id(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("RTK shell target has no valid filename: {}", path.display()))?;
    if !ALL_SHELL_FILES.contains(&name) {
        bail!("RTK shell target is not a known profile filename: {name}");
    }
    Ok(format!("shell:{name}"))
}

fn shell_path_for_artifact(id: &str) -> Result<PathBuf> {
    let name = id
        .strip_prefix("shell:")
        .ok_or_else(|| anyhow!("not an RTK shell artifact: {id}"))?;
    if !ALL_SHELL_FILES.contains(&name) {
        bail!("unknown RTK shell artifact: {id}");
    }
    Ok(shell_path(name))
}

fn managed_block(path: &Path, block_id: &str) -> Result<Option<String>> {
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
                "RTK managed block is ambiguous in {}; selective activation will not overwrite it",
                path.display()
            );
        }
        let start_index = content
            .find(&start)
            .expect("counted RTK managed marker must be present");
        let end_index = content
            .find(&end)
            .expect("counted RTK managed marker must be present");
        if start_index >= end_index {
            bail!(
                "RTK managed block markers are malformed in {}; selective activation will not overwrite it",
                path.display()
            );
        }
        matches.push(content[start_index..end_index + end.len()].to_string());
    }
    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => bail!(
            "multiple RTK marker variants are present in {}; selective activation will not overwrite them",
            path.display()
        ),
    }
}

fn canonical_managed_block(block_id: &str, body: &str) -> String {
    let start = managed_marker_start(primary_marker_prefix(), block_id);
    let end = managed_marker_end(primary_marker_prefix(), block_id);
    format!("{start}\n{body}\n{end}")
}

fn shell_block_body(managed_rtk_path: &Path) -> Result<String> {
    let bin_dir = managed_rtk_path.parent().ok_or_else(|| {
        anyhow!(
            "managed RTK path {} is missing a parent directory",
            managed_rtk_path.display()
        )
    })?;
    Ok(format!(
        "export PATH=\"{}:$PATH\"",
        shell_double_quote(&bin_dir.to_string_lossy())
    ))
}

fn canonical_rtk_shell_block(managed_rtk_path: &Path) -> Result<String> {
    Ok(canonical_managed_block(
        "managed_rtk",
        &shell_block_body(managed_rtk_path)?,
    ))
}

fn canonical_rtk_codex_block(managed_rtk_path: &Path) -> String {
    canonical_managed_block("rtk", &build_rtk_codex_nudge(managed_rtk_path))
}

fn canonical_settings_hook() -> Result<Value> {
    let hook_path = headroom_rtk_hook_path();
    let command = hook_path
        .to_str()
        .ok_or_else(|| anyhow!("RTK hook path contains invalid UTF-8: {}", hook_path.display()))?;
    Ok(json!({
        "matcher": "Bash",
        "hooks": [{
            "type": "command",
            "command": command,
        }]
    }))
}

fn rtk_settings_hook() -> Result<Option<Value>> {
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
                .filter(|entry| entry_contains_hook(entry, RTK_HOOK_MARKER))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => bail!(
            "multiple RTK Claude settings hooks are present; selective activation will not overwrite them"
        ),
    }
}

fn artifact_fingerprint(id: &str) -> Result<Option<String>> {
    match id {
        CLAUDE_HOOK_ARTIFACT => match std::fs::read(headroom_rtk_hook_path()) {
            Ok(bytes) => Ok(Some(fingerprint(bytes))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("reading managed RTK Claude hook"),
        },
        CLAUDE_SETTINGS_ARTIFACT => rtk_settings_hook()?
            .map(|entry| serde_json::to_vec(&entry).map(fingerprint))
            .transpose()
            .context("serializing managed RTK Claude settings hook"),
        CODEX_NUDGE_ARTIFACT => managed_block(&rtk_codex_agents_path(), "rtk")
            .map(|block| block.map(fingerprint)),
        id if id.starts_with("shell:") => managed_block(&shell_path_for_artifact(id)?, "managed_rtk")
            .map(|block| block.map(fingerprint)),
        _ => bail!("unknown RTK integration artifact: {id}"),
    }
}

fn expected_artifact_fingerprint(
    id: &str,
    managed_rtk_path: &Path,
    managed_python_path: &Path,
) -> Result<String> {
    match id {
        CLAUDE_HOOK_ARTIFACT => Ok(fingerprint(build_headroom_rtk_hook(
            managed_rtk_path,
            managed_python_path,
        ))),
        CLAUDE_SETTINGS_ARTIFACT => serde_json::to_vec(&canonical_settings_hook()?)
            .map(fingerprint)
            .context("serializing expected RTK Claude settings hook"),
        CODEX_NUDGE_ARTIFACT => Ok(fingerprint(canonical_rtk_codex_block(managed_rtk_path))),
        id if id.starts_with("shell:") => Ok(fingerprint(canonical_rtk_shell_block(managed_rtk_path)?)),
        _ => bail!("unknown RTK integration artifact: {id}"),
    }
}

/// Snapshots exactly the integration targets an RTK activation can mutate.
pub fn rtk_integration_snapshot() -> Result<RtkIntegrationSnapshot> {
    let mut artifacts = BTreeMap::new();
    for target in rtk_shell_targets() {
        let id = shell_artifact_id(&target)?;
        if let Some(fingerprint) = artifact_fingerprint(&id)? {
            artifacts.insert(id, fingerprint);
        }
    }
    for id in [CLAUDE_HOOK_ARTIFACT, CLAUDE_SETTINGS_ARTIFACT] {
        if let Some(fingerprint) = artifact_fingerprint(id)? {
            artifacts.insert(id.to_string(), fingerprint);
        }
    }
    if is_codex_enabled() {
        if let Some(fingerprint) = artifact_fingerprint(CODEX_NUDGE_ARTIFACT)? {
            artifacts.insert(CODEX_NUDGE_ARTIFACT.to_string(), fingerprint);
        }
    }
    Ok(RtkIntegrationSnapshot { artifacts })
}

fn rtk_shell_targets() -> Vec<PathBuf> {
    resolve_default_shell_targets()
}

fn rtk_activation_artifact_ids() -> Result<Vec<String>> {
    let mut ids = rtk_shell_targets()
        .iter()
        .map(|target| shell_artifact_id(target))
        .collect::<Result<Vec<_>>>()?;
    ids.extend([
        CLAUDE_HOOK_ARTIFACT.to_string(),
        CLAUDE_SETTINGS_ARTIFACT.to_string(),
    ]);
    if is_codex_enabled() {
        ids.push(CODEX_NUDGE_ARTIFACT.to_string());
    }
    Ok(ids)
}

/// Rejects legacy, custom, or malformed managed artifacts before an activation
/// could replace them. A selective receipt can only recreate canonical content.
pub fn validate_rtk_integration_snapshot(
    snapshot: &RtkIntegrationSnapshot,
    managed_rtk_path: &Path,
    managed_python_path: &Path,
) -> Result<()> {
    let expected_ids = rtk_activation_artifact_ids()?;
    let present = expected_ids
        .iter()
        .filter(|id| snapshot.artifacts.contains_key(*id))
        .count();
    if present != 0 && present != expected_ids.len() {
        bail!(
            "RTK has a partial managed integration; repair it from Addons before selective activation so no existing artifact is overwritten"
        );
    }
    for (id, actual) in &snapshot.artifacts {
        let expected = expected_artifact_fingerprint(id, managed_rtk_path, managed_python_path)?;
        if actual != &expected {
            bail!(
                "RTK has custom or legacy managed {id}; update it from Addons before selective activation so it can be restored safely"
            );
        }
    }
    Ok(())
}

/// Returns only artifacts introduced by this activation. Rewriting an existing
/// artifact is deliberately not owned by a selective receipt.
pub fn newly_created_rtk_artifacts(
    previous: &RtkIntegrationSnapshot,
    after: &RtkIntegrationSnapshot,
) -> BTreeMap<String, String> {
    after
        .artifacts
        .iter()
        .filter(|(id, _)| !previous.artifacts.contains_key(*id))
        .map(|(id, fingerprint)| (id.clone(), fingerprint.clone()))
        .collect()
}

/// Removes one run-created RTK artifact only if it still matches its recorded
/// post-activation fingerprint. This intentionally avoids `set_rtk_enabled`
/// because that broad helper also strips MarkItDown's Claude hook.
pub fn remove_rtk_artifact_if_unchanged(id: &str, after_fingerprint: &str) -> Result<bool> {
    let current = artifact_fingerprint(id)?;
    if current.as_deref() != Some(after_fingerprint) {
        bail!(
            "RTK {id} changed after activation (expected {}, found {})",
            after_fingerprint,
            current.as_deref().unwrap_or("absent")
        );
    }
    match id {
        CLAUDE_HOOK_ARTIFACT => std::fs::remove_file(headroom_rtk_hook_path())
            .context("removing run-created RTK Claude hook")
            .map(|_| true),
        CLAUDE_SETTINGS_ARTIFACT => {
            remove_pre_tool_use_markers(&claude_settings_path(), &[RTK_HOOK_MARKER])
        }
        CODEX_NUDGE_ARTIFACT => remove_managed_block(&rtk_codex_agents_path(), "rtk"),
        id if id.starts_with("shell:") => {
            remove_managed_block(&shell_path_for_artifact(id)?, "managed_rtk")
        }
        _ => bail!("unknown RTK integration artifact: {id}"),
    }
}

pub fn restore_rtk_disabled_if_unchanged(previous: bool, after: bool) -> Result<()> {
    let mut state = load_setup_state();
    if state.rtk_disabled != after {
        bail!(
            "RTK enabled state changed after activation (expected {}, found {})",
            after,
            state.rtk_disabled
        );
    }
    state.rtk_disabled = previous;
    write_setup_state(&state)
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
        canonical_managed_block, newly_created_rtk_artifacts, RtkIntegrationSnapshot,
    };
    use std::collections::BTreeMap;

    #[test]
    fn rtk_delta_tracks_only_new_artifacts() {
        let previous = RtkIntegrationSnapshot {
            artifacts: BTreeMap::from([("shell:.zshrc".into(), "before".into())]),
        };
        let after = RtkIntegrationSnapshot {
            artifacts: BTreeMap::from([
                ("shell:.zshrc".into(), "changed-but-preexisting".into()),
                ("claude-hook".into(), "created".into()),
            ]),
        };
        assert_eq!(
            newly_created_rtk_artifacts(&previous, &after),
            BTreeMap::from([("claude-hook".into(), "created".into())])
        );
    }

    #[test]
    fn canonical_rtk_block_is_limited_to_the_managed_marker() {
        let block = canonical_managed_block("managed_rtk", "export PATH=\"/tmp/bin:$PATH\"");
        assert!(block.starts_with("# >>> ai-switchboard:managed_rtk >>>"));
        assert!(block.ends_with("# <<< ai-switchboard:managed_rtk <<<"));
        assert!(!block.contains("# user instruction"));
    }
}
