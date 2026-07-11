use chrono::{DateTime, Utc};
use serde::Serialize;

use super::dedup::compacted_token_estimate;
use super::discovery::{discover, AgentMemoryTarget};
use super::rollback::{apply_confirmation_phrase, is_eligible_managed_memory};
use super::secret_scan::{SecretScanResult, SecretScanStatus};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemoryCompactionPreview {
    pub schema_version: u8,
    pub generated_at: DateTime<Utc>,
    pub repo_path: String,
    pub agent: AgentMemoryTarget,
    pub blocked_by_secrets: bool,
    pub write_performed: bool,
    /// Shown before apply so the caller can require deliberate, exact consent.
    pub confirmation_phrase: String,
    pub apply_eligible: bool,
    pub apply_blocked_reason: Option<String>,
    // Compatibility summary for the initial inspector. `sources` retains the
    // complete source-by-source preview for later multi-file UI treatment.
    pub source_path: Option<String>,
    pub before_tokens: u64,
    pub after_tokens: u64,
    pub duplicate_tokens_removed: u64,
    pub secret_scan: SecretScanResult,
    pub diff: Option<String>,
    pub summary: String,
    pub sources: Vec<AgentMemoryCompactionSourcePreview>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemoryCompactionSourcePreview {
    pub source_id: String,
    pub source_path: String,
    pub before_tokens: u64,
    pub after_tokens: u64,
    pub estimated_tokens_saved: u64,
    pub duplicate_line_count: usize,
    pub blank_lines_collapsed: usize,
    pub app_managed_blocks_preserved: bool,
    pub eligible_for_apply: bool,
    pub apply_blocked_reason: Option<String>,
    /// Content-free diff summary. Actual text stays local in the original file
    /// until a separately designed, confirmed apply flow exists.
    pub diff_summary: Vec<String>,
}

pub fn preview_compaction(
    repo_path: String,
    agent: AgentMemoryTarget,
) -> Result<AgentMemoryCompactionPreview, String> {
    let root = std::fs::canonicalize(&repo_path)
        .map_err(|error| format!("Could not resolve repository path: {error}"))?;
    if !root.is_dir() {
        return Err("Repository path must be a directory".to_string());
    }
    let items = discover(Some(&root))?;
    let mut sources = Vec::new();
    let mut warnings = Vec::new();
    let mut blocked_by_secrets = false;
    for item in items {
        if !matches_target(item.source.agent, agent) {
            continue;
        }
        let Some(content) = item.content else {
            continue;
        };
        if matches!(item.source.secret_scan.status, SecretScanStatus::Blocked) {
            blocked_by_secrets = true;
            warnings.push(format!(
                "Compaction blocked for {} because secret scanning found {} potential secret(s).",
                item.source.source_path, item.source.secret_scan.finding_count
            ));
            continue;
        }
        let before_tokens = item.source.estimated_tokens;
        let after_tokens = compacted_token_estimate(&content);
        let duplicate_line_count = duplicate_line_count(&content);
        let blank_lines_collapsed = collapsed_blank_lines(&content);
        let eligible_for_apply = is_eligible_managed_memory(&content);
        sources.push(AgentMemoryCompactionSourcePreview {
            source_id: item.source.id,
            source_path: item.source.source_path,
            before_tokens,
            after_tokens,
            estimated_tokens_saved: before_tokens.saturating_sub(after_tokens),
            duplicate_line_count,
            blank_lines_collapsed,
            app_managed_blocks_preserved: true,
            eligible_for_apply,
            apply_blocked_reason: (!eligible_for_apply).then(|| {
                "User-managed source: only canonical Switchboard-managed memory blocks may be edited."
                    .to_string()
            }),
            diff_summary: diff_summary(duplicate_line_count, blank_lines_collapsed),
        });
    }
    let before_tokens = sources.iter().map(|source| source.before_tokens).sum();
    let after_tokens = sources.iter().map(|source| source.after_tokens).sum();
    let duplicate_tokens_removed = sources
        .iter()
        .map(|source| source.estimated_tokens_saved)
        .sum();
    let source_path = sources.first().map(|source| source.source_path.clone());
    let secret_scan = if blocked_by_secrets {
        SecretScanResult {
            status: SecretScanStatus::Blocked,
            reason: Some(
                "At least one selected source contains potential credential material.".to_string(),
            ),
            finding_count: warnings.len(),
            categories: vec!["credential_material".to_string()],
            affected_line_numbers: vec![],
        }
    } else {
        SecretScanResult {
            status: SecretScanStatus::Clear,
            reason: None,
            finding_count: 0,
            categories: vec![],
            affected_line_numbers: vec![],
        }
    };
    let diff = (!blocked_by_secrets).then(|| {
        sources
            .iter()
            .flat_map(|source| source.diff_summary.iter().cloned())
            .collect::<Vec<_>>()
            .join("\n")
    });
    let summary = if blocked_by_secrets {
        "Preview is blocked by secret safety checks. No memory contents were read into the response.".to_string()
    } else {
        format!(
            "Read-only structural preview across {} source(s); no files were changed.",
            sources.len()
        )
    };
    let apply_eligible = !blocked_by_secrets
        && !sources.is_empty()
        && sources.iter().all(|source| source.eligible_for_apply)
        && sources
            .iter()
            .any(|source| source.before_tokens > source.after_tokens);
    let apply_blocked_reason = (!apply_eligible).then(|| {
        if blocked_by_secrets {
            "Secret safety checks blocked apply.".to_string()
        } else if sources.is_empty() {
            "No readable agent-memory sources are available to apply.".to_string()
        } else if sources.iter().any(|source| !source.eligible_for_apply) {
            "One or more selected sources are user-managed or noneligible; Switchboard will not edit them."
                .to_string()
        } else {
            "No safe structural compaction was identified.".to_string()
        }
    });
    Ok(AgentMemoryCompactionPreview {
        schema_version: 1,
        generated_at: Utc::now(),
        repo_path: root.display().to_string(),
        agent,
        blocked_by_secrets,
        write_performed: false,
        confirmation_phrase: apply_confirmation_phrase(agent),
        apply_eligible,
        apply_blocked_reason,
        source_path,
        before_tokens,
        after_tokens,
        duplicate_tokens_removed,
        secret_scan,
        diff,
        summary,
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

fn duplicate_line_count(content: &str) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    content
        .lines()
        .filter_map(|line| {
            let line = super::dedup::normalize_line(line);
            (!line.is_empty() && !seen.insert(line)).then_some(())
        })
        .count()
}

fn collapsed_blank_lines(content: &str) -> usize {
    let mut previous_blank = false;
    let mut count = 0;
    for line in content.lines() {
        let blank = line.trim().is_empty();
        if blank && previous_blank {
            count += 1;
        }
        previous_blank = blank;
    }
    count
}

fn diff_summary(duplicates: usize, blanks: usize) -> Vec<String> {
    let mut summary = Vec::new();
    if duplicates > 0 {
        summary.push(format!(
            "Would remove {duplicates} repeated instruction line(s)."
        ));
    }
    if blanks > 0 {
        summary.push(format!("Would collapse {blanks} adjacent blank line(s)."));
    }
    if summary.is_empty() {
        summary.push("No safe structural compaction was identified.".to_string());
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn preview_is_non_mutating_and_preserves_managed_blocks() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("AGENTS.md");
        let input = "<!-- switchboard:managed -->\nKeep it.\nKeep it.\n\n\n";
        std::fs::write(&path, input).unwrap();
        let preview =
            preview_compaction(temp.path().display().to_string(), AgentMemoryTarget::Codex)
                .unwrap();
        assert!(!preview.write_performed);
        assert!(preview.sources[0].app_managed_blocks_preserved);
        assert_eq!(std::fs::read_to_string(path).unwrap(), input);
    }

    #[test]
    fn secret_blocks_preview() {
        let temp = tempdir().unwrap();
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "API_KEY=sk-abcdefghijklmnopqrstuvwxyz\n",
        )
        .unwrap();
        let preview =
            preview_compaction(temp.path().display().to_string(), AgentMemoryTarget::Claude)
                .unwrap();
        assert!(preview.blocked_by_secrets);
        assert!(preview.sources.iter().all(
            |source| source.source_path != temp.path().join("CLAUDE.md").display().to_string()
        ));
    }
}
