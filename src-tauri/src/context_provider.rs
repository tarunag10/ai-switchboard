//! Stable read-only context-provider boundary for repository context packs.
//!
//! The first adapter reads the latest Repo Intelligence index. It never scans,
//! writes, clears, or mutates a repository and deliberately exposes the stable
//! names below so consumers do not import Repo Intelligence internals.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::models::RepoIntelligenceSummary;
pub(crate) use crate::models::{
    RepoContextPackResponse as ContextPack, RepoIndexFreshnessResponse as ContextFreshness,
    RepoIntelligenceManifestResponse as ContextManifest,
};

pub(crate) const REPO_INTELLIGENCE_CONTEXT_PROVIDER_ID: &str = "repo_intelligence";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Workspace {
    pub root: PathBuf,
}

impl Workspace {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextRequest {
    pub workspace: Workspace,
    pub pack_id: Option<String>,
}

/// Read-only contract for bounded context-pack consumers.
pub(crate) trait ContextProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn manifest(&self, workspace: &Workspace) -> Result<ContextManifest>;
    fn build_pack(&self, request: &ContextRequest) -> Result<ContextPack>;
    fn freshness(&self, workspace: &Workspace) -> Result<ContextFreshness>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RepoIntelligenceContextProvider;

impl RepoIntelligenceContextProvider {
    pub(crate) fn new() -> Self {
        Self
    }

    fn summary(&self, workspace: &Workspace) -> Result<RepoIntelligenceSummary> {
        let summary = crate::repo_intelligence::load_latest_summary()?
            .ok_or_else(|| anyhow!("no Repo Intelligence index is available"))?;
        validate_workspace(workspace, &summary)?;
        Ok(summary)
    }
}

impl ContextProvider for RepoIntelligenceContextProvider {
    fn id(&self) -> &'static str {
        REPO_INTELLIGENCE_CONTEXT_PROVIDER_ID
    }

    fn manifest(&self, workspace: &Workspace) -> Result<ContextManifest> {
        let summary = self.summary(workspace)?;
        let mut manifest = crate::repo_intelligence::build_manifest_response(&summary);
        // The Repo Intelligence UI also owns an explicit clear-index command.
        // That mutation is intentionally absent from this read-only provider.
        manifest
            .queries
            .retain(|query| query.id != "clear_repo_index");
        Ok(manifest)
    }

    fn build_pack(&self, request: &ContextRequest) -> Result<ContextPack> {
        let summary = self.summary(&request.workspace)?;
        crate::repo_intelligence::build_context_pack_response(&summary, request.pack_id.as_deref())
    }

    fn freshness(&self, workspace: &Workspace) -> Result<ContextFreshness> {
        let summary = self.summary(workspace)?;
        Ok(crate::repo_intelligence::build_index_freshness_response(
            Some(&summary),
        ))
    }
}

fn validate_workspace(workspace: &Workspace, summary: &RepoIntelligenceSummary) -> Result<()> {
    if paths_match(&workspace.root, Path::new(&summary.repo_root)) {
        return Ok(());
    }
    Err(anyhow!(
        "latest Repo Intelligence index is for {}, not {}",
        summary.repo_root,
        workspace.root.display()
    ))
}

fn paths_match(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString;

    use super::*;
    use crate::models::{RepoContextPack, RepoContextPackRankingMetadata};

    struct StorageEnvGuard {
        previous_home: Option<OsString>,
        previous_xdg: Option<OsString>,
    }

    impl StorageEnvGuard {
        fn isolated(root: &Path) -> Self {
            let previous_home = std::env::var_os("HOME");
            let previous_xdg = std::env::var_os("XDG_DATA_HOME");
            std::env::set_var("HOME", root);
            std::env::set_var("XDG_DATA_HOME", root);
            Self {
                previous_home,
                previous_xdg,
            }
        }
    }

    impl Drop for StorageEnvGuard {
        fn drop(&mut self) {
            match self.previous_home.take() {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match self.previous_xdg.take() {
                Some(value) => std::env::set_var("XDG_DATA_HOME", value),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }
    }

    fn summary(repo_root: &Path) -> RepoIntelligenceSummary {
        RepoIntelligenceSummary {
            indexed_at: "2026-08-16T00:00:00Z".to_string(),
            repo_root: repo_root.display().to_string(),
            indexer_version: Some("test".to_string()),
            total_files: 1,
            indexed_files: 1,
            skipped_files: 0,
            estimated_full_scan_tokens: 100,
            role_counts: BTreeMap::new(),
            index_metadata: None,
            graph: None,
            packs: vec![RepoContextPack {
                id: "implementation".to_string(),
                title: "Implementation".to_string(),
                purpose: "Focused implementation context".to_string(),
                files: Vec::new(),
                estimated_tokens: 10,
                savings_vs_full_scan_pct: 90.0,
                ranking: RepoContextPackRankingMetadata::default(),
            }],
        }
    }

    #[test]
    fn contract_is_object_safe_and_read_only() {
        let provider = RepoIntelligenceContextProvider::new();
        let provider: &dyn ContextProvider = &provider;
        assert_eq!(provider.id(), REPO_INTELLIGENCE_CONTEXT_PROVIDER_ID);
    }

    #[test]
    fn workspace_validation_rejects_a_pack_from_another_repo() {
        let indexed = summary(Path::new("/tmp/indexed-repo"));
        let err = validate_workspace(&Workspace::new("/tmp/another-repo"), &indexed)
            .expect_err("workspace mismatch");
        assert!(err
            .to_string()
            .contains("latest Repo Intelligence index is for"));
    }

    #[test]
    fn adapter_outputs_preserve_read_only_safety_contract() {
        let indexed = summary(Path::new("/tmp/indexed-repo"));
        let pack = crate::repo_intelligence::build_context_pack_response(&indexed, None)
            .expect("context pack");
        let manifest = crate::repo_intelligence::build_manifest_response(&indexed);

        assert!(pack.safety.read_only);
        assert!(!pack.safety.modifies_repository);
        assert!(manifest.safety.read_only);
        assert!(!manifest.safety.modifies_repository);
    }

    #[test]
    #[serial_test::serial]
    fn provider_serves_saved_pack_without_exposing_mutating_queries() {
        let scratch = tempfile::tempdir().expect("scratch");
        let _guard = StorageEnvGuard::isolated(scratch.path());
        let workspace_root = scratch.path().join("repo");
        std::fs::create_dir_all(&workspace_root).expect("workspace");
        let indexed = summary(&workspace_root);
        crate::repo_intelligence::save_latest_summary(&indexed).expect("save index");

        let provider = RepoIntelligenceContextProvider::new();
        let provider: &dyn ContextProvider = &provider;
        let workspace = Workspace::new(&workspace_root);
        let manifest = provider.manifest(&workspace).expect("manifest");
        let pack = provider
            .build_pack(&ContextRequest {
                workspace: workspace.clone(),
                pack_id: Some("implementation".to_string()),
            })
            .expect("pack");
        let freshness = provider.freshness(&workspace).expect("freshness");

        assert_eq!(pack.pack.id, "implementation");
        assert!(pack.safety.read_only);
        assert!(freshness.safety.read_only);
        assert!(!manifest
            .queries
            .iter()
            .any(|query| query.id == "clear_repo_index"));
    }
}
