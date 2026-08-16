//! Experimental DeepSeek Harness context-plugin mapping.
//!
//! This is an adapter prototype over the stable `ContextProvider` pack. It
//! patches no dsh core and performs no repository mutation. A future supported
//! dsh plugin seam can serialize `DshContextInsertion` at the agent lifecycle
//! boundary without importing Repo Intelligence internals.

use std::collections::BTreeSet;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::context_provider::ContextPack;

pub(crate) const DSH_CONTEXT_PROTOTYPE_ID: &str =
    "ai-switchboard.repo-intelligence.experimental.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DshLifecyclePoint {
    BeforeAgentRun,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DshContextEvidence {
    pub source_estimated_tokens: u64,
    pub inserted_estimated_tokens: u64,
    pub savings_vs_full_scan_pct: f64,
    pub selected_file_count: usize,
    pub task_term_match_count: usize,
    pub ranking_evidence_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DshContextPayload {
    pub title: String,
    pub purpose: String,
    pub repo_root: String,
    pub indexed_at: String,
    pub files: Vec<String>,
    pub ranking_evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DshContextInsertion {
    pub plugin_id: &'static str,
    pub experimental: bool,
    pub lifecycle_point: DshLifecyclePoint,
    pub replay_identity: String,
    pub payload: DshContextPayload,
    pub evidence: DshContextEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DshContextOutcome {
    Inserted,
    DuplicateSkipped,
}

/// Session-local deduplication state owned by the adapter, not by dsh core.
#[derive(Default)]
pub(crate) struct DshContextPrototype {
    inserted: BTreeSet<String>,
}

impl DshContextPrototype {
    pub(crate) fn prepare(
        &mut self,
        session_id: &str,
        pack: &ContextPack,
    ) -> Result<(DshContextOutcome, DshContextInsertion)> {
        if session_id.trim().is_empty() {
            bail!("dsh context prototype requires a session identity");
        }
        if !pack.safety.read_only || pack.safety.modifies_repository {
            bail!("dsh context prototype accepts read-only context packs only");
        }

        let replay_identity = replay_identity(pack);
        let deduplication_key = format!("{session_id}:{replay_identity}");
        let outcome = if self.inserted.insert(deduplication_key) {
            DshContextOutcome::Inserted
        } else {
            DshContextOutcome::DuplicateSkipped
        };

        let files = pack
            .pack
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        let insertion = DshContextInsertion {
            plugin_id: DSH_CONTEXT_PROTOTYPE_ID,
            experimental: true,
            lifecycle_point: DshLifecyclePoint::BeforeAgentRun,
            replay_identity,
            payload: DshContextPayload {
                title: pack.pack.title.clone(),
                purpose: pack.pack.purpose.clone(),
                repo_root: pack.repo_root.clone(),
                indexed_at: pack.indexed_at.clone(),
                files,
                ranking_evidence: pack.pack.ranking.evidence.clone(),
            },
            evidence: DshContextEvidence {
                source_estimated_tokens: pack.pack.estimated_tokens,
                inserted_estimated_tokens: pack.pack.estimated_tokens,
                savings_vs_full_scan_pct: pack.pack.savings_vs_full_scan_pct,
                selected_file_count: pack.pack.files.len(),
                task_term_match_count: pack.pack.ranking.graph_task_term_match_count,
                ranking_evidence_count: pack.pack.ranking.evidence.len(),
            },
        };
        Ok((outcome, insertion))
    }
}

fn replay_identity(pack: &ContextPack) -> String {
    let mut digest = Sha256::new();
    for value in [
        DSH_CONTEXT_PROTOTYPE_ID,
        pack.repo_root.as_str(),
        pack.indexed_at.as_str(),
        pack.pack.id.as_str(),
    ] {
        digest.update(value.as_bytes());
        digest.update([0]);
    }
    for file in &pack.pack.files {
        digest.update(file.path.as_bytes());
        digest.update([0]);
        digest.update(file.estimated_tokens.to_le_bytes());
    }
    format!("sha256:{:x}", digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        RepoContextPack, RepoContextPackGraphBrief, RepoContextPackRankingMetadata,
        RepoContextPackSafety, RepoFileRole, RepoFileSignal, RepoIndexFreshnessResponse,
        RepoIndexFreshnessStatus,
    };

    fn fixture_pack() -> ContextPack {
        ContextPack {
            repo_root: "/fixture/repo".to_string(),
            indexed_at: "2026-08-16T00:00:00Z".to_string(),
            pack: RepoContextPack {
                id: "implementation".to_string(),
                title: "Implementation".to_string(),
                purpose: "Focused implementation context".to_string(),
                files: vec![RepoFileSignal {
                    path: "src/lib.rs".to_string(),
                    role: RepoFileRole::Source,
                    language: "Rust".to_string(),
                    estimated_tokens: 80,
                    include_by_default: true,
                    reasons: vec!["task term match".to_string()],
                }],
                estimated_tokens: 80,
                savings_vs_full_scan_pct: 60.0,
                ranking: RepoContextPackRankingMetadata {
                    task_terms: vec!["adapter".to_string()],
                    graph_task_term_match_count: 1,
                    reverse_dependency_hubs: Vec::new(),
                    evidence: vec!["matched adapter".to_string()],
                },
            },
            index_metadata: None,
            index_freshness: RepoIndexFreshnessResponse {
                status: RepoIndexFreshnessStatus::Fresh,
                indexed_at: Some("2026-08-16T00:00:00Z".to_string()),
                repo_root: Some("/fixture/repo".to_string()),
                label: "Fresh".to_string(),
                detail: "fixture".to_string(),
                api_available: true,
                graph_available: false,
                index_health: "current".to_string(),
                parser_health: "current".to_string(),
                indexer_version: Some("fixture".to_string()),
                parser_version: None,
                indexed_file_count: Some(1),
                skipped_file_count: Some(0),
                safety: RepoContextPackSafety {
                    read_only: true,
                    excludes_secret_like_paths: true,
                    modifies_repository: false,
                },
            },
            graph_brief: RepoContextPackGraphBrief {
                available: false,
                dependency_hub_count: 0,
                import_edge_count: 0,
                reverse_dependency_hub_count: 0,
                symbol_count: 0,
                symbol_edge_count: 0,
                graph_input_paths: Vec::new(),
            },
            safety: RepoContextPackSafety {
                read_only: true,
                excludes_secret_like_paths: true,
                modifies_repository: false,
            },
        }
    }

    #[test]
    fn maps_pack_to_experimental_plugin_payload_and_evidence() {
        let mut prototype = DshContextPrototype::default();
        let (outcome, insertion) = prototype
            .prepare("session-1", &fixture_pack())
            .expect("insertion");

        assert_eq!(outcome, DshContextOutcome::Inserted);
        assert!(insertion.experimental);
        assert_eq!(insertion.payload.files, vec!["src/lib.rs"]);
        assert_eq!(insertion.evidence.inserted_estimated_tokens, 80);
        assert_eq!(insertion.evidence.task_term_match_count, 1);
    }

    #[test]
    fn skips_duplicate_insertion_within_session() {
        let mut prototype = DshContextPrototype::default();
        let pack = fixture_pack();
        prototype.prepare("session-1", &pack).expect("first");
        let (outcome, _) = prototype.prepare("session-1", &pack).expect("second");
        assert_eq!(outcome, DshContextOutcome::DuplicateSkipped);
    }

    #[test]
    fn replay_identity_is_stable_across_sessions() {
        let mut first = DshContextPrototype::default();
        let mut replay = DshContextPrototype::default();
        let pack = fixture_pack();
        let (_, original) = first.prepare("session-1", &pack).expect("original");
        let (_, replayed) = replay.prepare("session-1", &pack).expect("replay");
        let (_, another_session) = replay.prepare("session-2", &pack).expect("new session");
        assert_eq!(original.replay_identity, replayed.replay_identity);
        assert_eq!(original.replay_identity, another_session.replay_identity);
    }

    #[test]
    fn refuses_non_read_only_context_pack() {
        let mut pack = fixture_pack();
        pack.safety.modifies_repository = true;
        assert!(DshContextPrototype::default()
            .prepare("session-1", &pack)
            .expect_err("unsafe pack")
            .to_string()
            .contains("read-only"));
    }
}
