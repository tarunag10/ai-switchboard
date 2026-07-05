use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{ClaudePlanTier, LaunchExperience, RuntimeUpgradeFailure};
use crate::storage::config_file;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LaunchProfile {
    pub(super) launch_count: u64,
    pub(super) launch_experience: LaunchExperience,
    pub(super) lifetime_requests: usize,
    pub(super) lifetime_estimated_savings_usd: f64,
    pub(super) lifetime_estimated_tokens_saved: u64,
    #[serde(default)]
    pub(super) setup_wizard_complete: bool,
    #[serde(default)]
    pub(super) last_launched_app_version: Option<String>,
    #[serde(default)]
    pub(super) last_runtime_upgrade_failure: Option<RuntimeUpgradeFailure>,
    /// Highest Terms-of-Service version the user has accepted. Defaults to 0
    /// for profiles written before this field existed, so existing users are
    /// re-prompted by the acceptance gate when `REQUIRED_TERMS_VERSION` > 0.
    #[serde(default)]
    pub(super) accepted_terms_version: u32,
}

impl LaunchProfile {
    pub(super) fn load_or_create(base_dir: &Path) -> Result<(Self, PathBuf)> {
        let path = config_file(base_dir, "launch-profile.json");

        let previous = if path.exists() {
            let bytes =
                std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            serde_json::from_slice::<LaunchProfile>(&bytes)
                .with_context(|| format!("parsing {}", path.display()))?
        } else {
            LaunchProfile {
                launch_count: 0,
                launch_experience: LaunchExperience::FirstRun,
                lifetime_requests: 0,
                lifetime_estimated_savings_usd: 0.0,
                lifetime_estimated_tokens_saved: 0,
                setup_wizard_complete: false,
                last_launched_app_version: None,
                last_runtime_upgrade_failure: None,
                accepted_terms_version: 0,
            }
        };

        let mut current = previous;
        current.launch_count += 1;

        // Migrate legacy seeded demo totals to true zero-based tracking.
        if current.lifetime_requests == 138
            && (current.lifetime_estimated_savings_usd - 31.72).abs() < f64::EPSILON
            && current.lifetime_estimated_tokens_saved == 512_844
        {
            current.lifetime_requests = 0;
            current.lifetime_estimated_savings_usd = 0.0;
            current.lifetime_estimated_tokens_saved = 0;
        }

        if current.launch_count == 1 {
            current.launch_experience = LaunchExperience::FirstRun;
        } else {
            current.launch_experience = LaunchExperience::Resume;
        }

        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&current).context("serializing launch profile")?,
        )
        .with_context(|| format!("writing {}", path.display()))?;

        Ok((current, path))
    }
}

pub(super) fn persist_launch_profile(path: &Path, profile: &LaunchProfile) {
    if let Ok(bytes) = serde_json::to_vec_pretty(profile) {
        let _ = std::fs::write(path, bytes);
    }
}

/// Last classification that returned a non-Unknown tier. Persisted so the
/// pricing gate can keep applying the right thresholds when Anthropic's
/// OAuth profile transiently comes back sparse and the live classifier
/// returns Unknown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LastKnownGoodPlan {
    pub(super) plan_tier: ClaudePlanTier,
    pub(super) recorded_at: DateTime<Utc>,
}

impl LastKnownGoodPlan {
    pub(super) fn load(base_dir: &Path) -> (Option<Self>, PathBuf) {
        let path = config_file(base_dir, "last-known-good-plan.json");
        let value = if path.exists() {
            std::fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
        } else {
            None
        };
        (value, path)
    }
}

pub(super) fn persist_last_known_good_plan(path: &Path, plan: &LastKnownGoodPlan) {
    if let Ok(bytes) = serde_json::to_vec_pretty(plan) {
        let _ = std::fs::write(path, bytes);
    }
}
