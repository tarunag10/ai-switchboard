use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::super::events::validate_identifier;
use super::super::session::validate_digest;
use super::receipt::{sha256_json, WorkbenchFakeProcessReceipt};

const REGISTRY_SCHEMA_VERSION: u32 = 1;
pub(super) const MAX_FAKE_RUNS: usize = 64;
pub(super) const MAX_RETIRED_FAKE_RUNS: usize = 4_096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct WorkbenchFakeProcessRegistry {
    pub(super) schema_version: u32,
    pub(super) owner_epoch: String,
    pub(super) next_registered_sequence: u64,
    pub(super) runs: BTreeMap<String, WorkbenchFakeProcessReceipt>,
    pub(super) retired_runs: BTreeMap<String, String>,
    pub(super) registry_digest: String,
}

impl WorkbenchFakeProcessRegistry {
    pub(super) fn empty(owner_epoch: &str) -> Result<Self> {
        validate_identifier(owner_epoch, "fake process owner epoch")?;
        let mut registry = Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            owner_epoch: owner_epoch.into(),
            next_registered_sequence: 0,
            runs: BTreeMap::new(),
            retired_runs: BTreeMap::new(),
            registry_digest: String::new(),
        };
        registry.refresh_digest()?;
        Ok(registry)
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.schema_version != REGISTRY_SCHEMA_VERSION
            || self.runs.len() > MAX_FAKE_RUNS
            || self.retired_runs.len() > MAX_RETIRED_FAKE_RUNS
        {
            bail!("Workbench fake process registry is unsupported or exceeds its capacity");
        }
        validate_identifier(&self.owner_epoch, "fake process owner epoch")?;
        let mut sequences = BTreeSet::new();
        for (run_id, receipt) in &self.runs {
            if run_id != &receipt.process_run_id {
                bail!("Workbench fake process registry key does not match its receipt");
            }
            if receipt.registered_sequence >= self.next_registered_sequence
                || !sequences.insert(receipt.registered_sequence)
            {
                bail!("Workbench fake process registry sequence metadata is invalid");
            }
            receipt.validate()?;
        }
        for (run_id, binding_digest) in &self.retired_runs {
            validate_identifier(run_id, "retired process run ID")?;
            validate_digest(binding_digest, "retired process binding digest")?;
            if self.runs.contains_key(run_id) {
                bail!("Workbench fake process run cannot be both live and retired");
            }
        }
        validate_digest(&self.registry_digest, "fake process registry digest")?;
        if self.registry_digest != self.expected_digest()? {
            bail!("Workbench fake process registry digest does not match its content");
        }
        Ok(())
    }

    pub(super) fn refresh_digest(&mut self) -> Result<()> {
        self.registry_digest = self.expected_digest()?;
        Ok(())
    }

    fn expected_digest(&self) -> Result<String> {
        sha256_json(&serde_json::json!({
            "schemaVersion": self.schema_version,
            "ownerEpoch": &self.owner_epoch,
            "nextRegisteredSequence": self.next_registered_sequence,
            "runs": &self.runs,
            "retiredRuns": &self.retired_runs,
        }))
    }
}

pub(super) fn load_registry(
    path: &PathBuf,
    owner_epoch: &str,
) -> Result<(WorkbenchFakeProcessRegistry, Option<Vec<u8>>)> {
    if !path.exists() {
        return Ok((WorkbenchFakeProcessRegistry::empty(owner_epoch)?, None));
    }
    let bytes = fs::read(path)
        .with_context(|| format!("reading Workbench fake process registry {}", path.display()))?;
    let registry: WorkbenchFakeProcessRegistry =
        serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "decoding Workbench fake process registry {}",
                path.display()
            )
        })?;
    registry.validate()?;
    Ok((registry, Some(bytes)))
}
