//! Fail-closed DeepSeek Harness plugin maturity audit.
//!
//! The current upstream snapshot is still a developer preview. Documented
//! internal seams are useful research evidence, but they are not treated as a
//! stable third-party compatibility promise or as end-to-end Switchboard proof.

use std::collections::BTreeSet;

pub(crate) const DSH_MATURITY_AUDIT_VERSION: &str = "0.1.0-rc.5";
pub(crate) const DSH_MATURITY_AUDIT_SHA: &str = "47f943859bef60e4160492346772ded9b24f765a";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DshPromotionSurface {
    RepoIntelligenceInjection,
    RequestMetadata,
    ToolResultOptimization,
    PromptSegmentClassification,
    SwitchboardRouteDecision,
    SavingsEvidence,
}

impl DshPromotionSurface {
    const ALL: [Self; 6] = [
        Self::RepoIntelligenceInjection,
        Self::RequestMetadata,
        Self::ToolResultOptimization,
        Self::PromptSegmentClassification,
        Self::SwitchboardRouteDecision,
        Self::SavingsEvidence,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::RepoIntelligenceInjection => "repo_intelligence_injection",
            Self::RequestMetadata => "request_metadata",
            Self::ToolResultOptimization => "tool_result_optimization",
            Self::PromptSegmentClassification => "prompt_segment_classification",
            Self::SwitchboardRouteDecision => "switchboard_route_decision",
            Self::SavingsEvidence => "savings_evidence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DshSurfaceEvidence {
    pub surface: DshPromotionSurface,
    /// A public upstream extension point exists at the audited snapshot.
    pub documented_upstream_seam: bool,
    /// Upstream promises a versioned, compatibility-stable schema/lifecycle.
    pub stable_versioned_contract: bool,
    /// An upstream fixture exercises third-party use of that exact contract.
    pub upstream_compatibility_fixture: bool,
    /// Switchboard proves the real dsh lifecycle, including rollback/failure.
    pub switchboard_end_to_end_fixture: bool,
    /// Evidence attributes behavior/savings without payload or secret capture.
    pub content_free_evidence: bool,
}

impl DshSurfaceEvidence {
    fn complete(&self) -> bool {
        self.documented_upstream_seam
            && self.stable_versioned_contract
            && self.upstream_compatibility_fixture
            && self.switchboard_end_to_end_fixture
            && self.content_free_evidence
    }

    pub(crate) fn missing_evidence(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.documented_upstream_seam {
            missing.push("documented_upstream_seam");
        }
        if !self.stable_versioned_contract {
            missing.push("stable_versioned_contract");
        }
        if !self.upstream_compatibility_fixture {
            missing.push("upstream_compatibility_fixture");
        }
        if !self.switchboard_end_to_end_fixture {
            missing.push("switchboard_end_to_end_fixture");
        }
        if !self.content_free_evidence {
            missing.push("content_free_evidence");
        }
        missing
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DshMaturityEvidence {
    pub upstream_version: String,
    pub upstream_sha: String,
    pub developer_preview: bool,
    pub advertises_breaking_changes: bool,
    pub stable_release_or_tag: bool,
    pub versioned_plugin_compatibility_policy: bool,
    pub surfaces: Vec<DshSurfaceEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DshMaturityDecision {
    Experimental,
    /// Evidence is complete enough to begin a separate, manual promotion
    /// review. This never changes the adapter's maturity automatically.
    ManualPromotionReviewRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DshMaturityAudit {
    pub decision: DshMaturityDecision,
    pub blockers: Vec<String>,
}

pub(crate) fn audit_maturity(evidence: &DshMaturityEvidence) -> DshMaturityAudit {
    let mut blockers = Vec::new();
    if evidence.upstream_version != DSH_MATURITY_AUDIT_VERSION
        || evidence.upstream_sha != DSH_MATURITY_AUDIT_SHA
    {
        blockers.push("upstream snapshot differs from the reviewed adapter pin".to_string());
    }
    if evidence.developer_preview {
        blockers.push("upstream still declares developer-preview status".to_string());
    }
    if evidence.advertises_breaking_changes {
        blockers.push("upstream still warns of compatibility-breaking changes".to_string());
    }
    if !evidence.stable_release_or_tag {
        blockers.push("no stable upstream release or tag is published".to_string());
    }
    if !evidence.versioned_plugin_compatibility_policy {
        blockers.push("no versioned plugin compatibility policy is published".to_string());
    }

    let expected = DshPromotionSurface::ALL
        .into_iter()
        .collect::<BTreeSet<_>>();
    let observed = evidence
        .surfaces
        .iter()
        .map(|surface| surface.surface)
        .collect::<BTreeSet<_>>();
    if observed != expected || observed.len() != evidence.surfaces.len() {
        blockers
            .push("promotion evidence must contain each required surface exactly once".to_string());
    }
    for surface in &evidence.surfaces {
        if !surface.complete() {
            let missing = surface.missing_evidence();
            blockers.push(format!(
                "{} is missing {}",
                surface.surface.label(),
                missing.join(", ")
            ));
        }
    }

    DshMaturityAudit {
        decision: if blockers.is_empty() {
            DshMaturityDecision::ManualPromotionReviewRequired
        } else {
            DshMaturityDecision::Experimental
        },
        blockers,
    }
}

/// Evidence observed from official upstream at the pinned rc.5 snapshot.
pub(crate) fn current_upstream_evidence() -> DshMaturityEvidence {
    let surface = |surface, documented_upstream_seam| DshSurfaceEvidence {
        surface,
        documented_upstream_seam,
        stable_versioned_contract: false,
        upstream_compatibility_fixture: false,
        switchboard_end_to_end_fixture: false,
        content_free_evidence: false,
    };
    DshMaturityEvidence {
        upstream_version: DSH_MATURITY_AUDIT_VERSION.to_string(),
        upstream_sha: DSH_MATURITY_AUDIT_SHA.to_string(),
        developer_preview: true,
        advertises_breaking_changes: true,
        stable_release_or_tag: false,
        versioned_plugin_compatibility_policy: false,
        surfaces: vec![
            // agent.inject() is documented, but the local before_agent_run
            // mapping is only a Switchboard prototype, not an upstream hook.
            surface(DshPromotionSurface::RepoIntelligenceInjection, true),
            // agent/request exists, but no stable metadata mutation schema is
            // promised to third-party plugins.
            surface(DshPromotionSurface::RequestMetadata, true),
            // tools/* and an internal result pruner exist, but no supported
            // external optimization ownership contract is published.
            surface(DshPromotionSurface::ToolResultOptimization, true),
            // Prompt sections exist; a stable typed classification vocabulary
            // for external optimizers does not.
            surface(DshPromotionSurface::PromptSegmentClassification, false),
            // Request interception and LLM adapters exist, but there is no
            // stable Switchboard route-decision/fallback exchange contract.
            surface(DshPromotionSurface::SwitchboardRouteDecision, false),
            // Token metering exists, but not transformation-attributed,
            // content-free Switchboard savings evidence.
            surface(DshPromotionSurface::SavingsEvidence, false),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_snapshot_matches_the_fail_closed_adapter_pin() {
        assert_eq!(
            DSH_MATURITY_AUDIT_VERSION,
            crate::deepseek_harness::DSH_SUPPORTED_VERSION
        );
        assert_eq!(
            DSH_MATURITY_AUDIT_SHA,
            crate::deepseek_harness::DSH_SUPPORTED_UPSTREAM_SHA
        );
    }

    #[test]
    fn pinned_rc5_snapshot_remains_experimental_with_all_six_surface_gaps() {
        let evidence = current_upstream_evidence();
        let audit = audit_maturity(&evidence);

        assert_eq!(audit.decision, DshMaturityDecision::Experimental);
        for surface in DshPromotionSurface::ALL {
            assert!(audit
                .blockers
                .iter()
                .any(|blocker| blocker.starts_with(surface.label())));
        }
        assert!(audit
            .blockers
            .iter()
            .any(|blocker| blocker.contains("developer-preview")));
    }

    #[test]
    fn unaudited_upstream_version_or_sha_fails_closed() {
        let mut evidence = complete_evidence();
        evidence.upstream_version = "0.2.0".to_string();
        evidence.upstream_sha = "unreviewed".to_string();

        let audit = audit_maturity(&evidence);
        assert_eq!(audit.decision, DshMaturityDecision::Experimental);
        assert!(audit
            .blockers
            .iter()
            .any(|blocker| blocker.contains("differs from the reviewed adapter pin")));
    }

    #[test]
    fn a_missing_or_duplicate_surface_cannot_satisfy_the_gate() {
        let mut evidence = complete_evidence();
        evidence.surfaces.pop();
        evidence.surfaces.push(evidence.surfaces[0].clone());

        let audit = audit_maturity(&evidence);
        assert_eq!(audit.decision, DshMaturityDecision::Experimental);
        assert!(audit
            .blockers
            .iter()
            .any(|blocker| blocker.contains("each required surface exactly once")));
    }

    #[test]
    fn complete_evidence_only_opens_manual_review() {
        let audit = audit_maturity(&complete_evidence());
        assert_eq!(
            audit.decision,
            DshMaturityDecision::ManualPromotionReviewRequired
        );
        assert!(audit.blockers.is_empty());
    }

    fn complete_evidence() -> DshMaturityEvidence {
        DshMaturityEvidence {
            upstream_version: DSH_MATURITY_AUDIT_VERSION.to_string(),
            upstream_sha: DSH_MATURITY_AUDIT_SHA.to_string(),
            developer_preview: false,
            advertises_breaking_changes: false,
            stable_release_or_tag: true,
            versioned_plugin_compatibility_policy: true,
            surfaces: DshPromotionSurface::ALL
                .into_iter()
                .map(|surface| DshSurfaceEvidence {
                    surface,
                    documented_upstream_seam: true,
                    stable_versioned_contract: true,
                    upstream_compatibility_fixture: true,
                    switchboard_end_to_end_fixture: true,
                    content_free_evidence: true,
                })
                .collect(),
        }
    }
}
