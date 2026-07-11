//! Content-free Agent Memory metadata for session preparation and handoff.
//!
//! This module is intentionally stricter than the inspector contract: source
//! paths and instruction text are useful locally in the inspector, but never
//! belong in a copyable agent handoff. Secret-blocked sources are excluded
//! before the manifest is assembled.

use std::collections::BTreeMap;

use crate::models::{AgentMemorySessionManifest, AgentMemorySessionSource};

use super::discovery::{discover, AgentMemoryTarget};
use super::preview::preview_compaction;
use super::secret_scan::SecretScanStatus;

pub fn build_session_manifest(
    repo_path: String,
    target: AgentMemoryTarget,
) -> Result<AgentMemorySessionManifest, String> {
    let root = std::fs::canonicalize(&repo_path)
        .map_err(|error| format!("Could not resolve repository path: {error}"))?;
    if !root.is_dir() {
        return Err("Repository path must be a directory".to_string());
    }

    let discovered = discover(Some(&root))?;
    let preview = preview_compaction(root.display().to_string(), target)?;
    let preview_by_id: BTreeMap<_, _> = preview
        .sources
        .iter()
        .map(|source| (source.source_id.as_str(), source))
        .collect();
    let excluded_secret_source_count = discovered
        .iter()
        .filter(|item| matches_target(item.source.agent, target))
        .filter(|item| matches!(item.source.secret_scan.status, SecretScanStatus::Blocked))
        .count();

    // A safe preview needs a complete clear selection and actual before/after
    // token estimates for every selected source. Do not manufacture a delta
    // from snapshot-only fields.
    let safe_preview_available = !preview.blocked_by_secrets
        && !preview.sources.is_empty()
        && preview
            .sources
            .iter()
            .all(|source| source.before_tokens >= source.after_tokens);
    let mut sources: Vec<_> = discovered
        .iter()
        .filter(|item| matches_target(item.source.agent, target))
        .filter(|item| item.content.is_some())
        .filter(|item| matches!(item.source.secret_scan.status, SecretScanStatus::Clear))
        .filter_map(|item| {
            let preview_source = preview_by_id.get(item.source.id.as_str())?;
            Some(AgentMemorySessionSource {
                ordinal: 0,
                agent: target_key(item.source.agent).to_string(),
                scope: item.source.scope.clone(),
                managed_by_switchboard: item.source.managed_by_switchboard,
                estimated_tokens_before: preview_source.before_tokens,
                estimated_tokens_after: safe_preview_available
                    .then_some(preview_source.after_tokens),
            })
        })
        .collect();

    // Global/repo shared stable instructions are placed first. The ordering is
    // explicit metadata, not a claim that any source content was exported.
    sources.sort_by(|left, right| {
        scope_rank(&left.scope)
            .cmp(&scope_rank(&right.scope))
            .then_with(|| left.agent.cmp(&right.agent))
            .then_with(|| {
                left.estimated_tokens_before
                    .cmp(&right.estimated_tokens_before)
            })
    });
    for (index, source) in sources.iter_mut().enumerate() {
        source.ordinal = index + 1;
    }

    let (estimated_tokens_before, estimated_tokens_after, estimated_tokens_saved) =
        if safe_preview_available {
            (
                Some(preview.before_tokens),
                Some(preview.after_tokens),
                Some(preview.before_tokens.saturating_sub(preview.after_tokens)),
            )
        } else {
            (None, None, None)
        };
    let mut warnings = Vec::new();
    if excluded_secret_source_count > 0 {
        warnings.push(format!(
            "{excluded_secret_source_count} secret-blocked memory source(s) were excluded from this handoff."
        ));
    }
    if !safe_preview_available {
        warnings.push(
            "No complete safe before/after preview is available; memory savings were not attributed."
                .to_string(),
        );
    }

    Ok(AgentMemorySessionManifest {
        schema_version: 1,
        kind: "mac_ai_switchboard.agent_memory_session_manifest".to_string(),
        target: target_key(target).to_string(),
        stable_memory_first: true,
        source_count: sources.len(),
        excluded_secret_source_count,
        safe_preview_available,
        estimated_tokens_before,
        estimated_tokens_after,
        estimated_tokens_saved,
        sources,
        warnings,
    })
}

fn matches_target(source: AgentMemoryTarget, requested: AgentMemoryTarget) -> bool {
    source == requested
        || matches!(source, AgentMemoryTarget::Shared)
            && matches!(
                requested,
                AgentMemoryTarget::Codex | AgentMemoryTarget::Claude | AgentMemoryTarget::Shared
            )
}

fn target_key(target: AgentMemoryTarget) -> &'static str {
    match target {
        AgentMemoryTarget::Codex => "codex",
        AgentMemoryTarget::Claude => "claude",
        AgentMemoryTarget::Shared => "shared",
        AgentMemoryTarget::RepoMemoryMcp => "repo_memory_mcp",
    }
}

fn scope_rank(scope: &str) -> u8 {
    match scope {
        "global" => 0,
        "repo" => 1,
        "nested" => 2,
        _ => 3,
    }
}

#[cfg(test)]
fn safe_preview_counts(manifest: &AgentMemorySessionManifest) -> Option<(u64, u64)> {
    if !manifest.safe_preview_available {
        return None;
    }
    let (Some(before), Some(after)) = (
        manifest.estimated_tokens_before,
        manifest.estimated_tokens_after,
    ) else {
        return None;
    };
    (before > after).then_some((before, after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn session_manifest_is_content_and_path_free_and_orders_stable_memory_first() {
        let temp = tempdir().unwrap();
        std::fs::write(
            temp.path().join("AGENTS.md"),
            "Global guidance.\nGlobal guidance.\n",
        )
        .unwrap();
        std::fs::create_dir_all(temp.path().join("nested")).unwrap();
        std::fs::write(temp.path().join("nested/AGENTS.md"), "Nested guidance.\n").unwrap();

        let manifest =
            build_session_manifest(temp.path().display().to_string(), AgentMemoryTarget::Codex)
                .unwrap();
        let json = serde_json::to_string(&manifest).unwrap();

        assert!(manifest.stable_memory_first);
        assert!(manifest.safe_preview_available);
        assert!(manifest
            .sources
            .windows(2)
            .all(|pair| scope_rank(&pair[0].scope) <= scope_rank(&pair[1].scope)));
        assert!(!json.contains("AGENTS.md"));
        assert!(!json.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!json.contains("Global guidance"));
        assert!(safe_preview_counts(&manifest).is_some());
    }

    #[test]
    fn secret_blocked_source_is_excluded_and_disables_savings_preview() {
        let temp = tempdir().unwrap();
        std::fs::write(temp.path().join("AGENTS.md"), "Keep tests focused.\n").unwrap();
        std::fs::write(temp.path().join("nested.md"), "unrelated\n").unwrap();
        std::fs::create_dir_all(temp.path().join("nested")).unwrap();
        std::fs::write(
            temp.path().join("nested/AGENTS.md"),
            "API_KEY=sk-abcdefghijklmnopqrstuvwxyz\n",
        )
        .unwrap();

        let manifest =
            build_session_manifest(temp.path().display().to_string(), AgentMemoryTarget::Codex)
                .unwrap();
        let json = serde_json::to_string(&manifest).unwrap();

        assert_eq!(manifest.excluded_secret_source_count, 1);
        assert!(!manifest.safe_preview_available);
        assert!(manifest.source_count >= 1);
        assert!(!json.contains("nested/AGENTS.md"));
        assert!(!json.contains("sk-"));
        assert!(safe_preview_counts(&manifest).is_none());
    }
}
