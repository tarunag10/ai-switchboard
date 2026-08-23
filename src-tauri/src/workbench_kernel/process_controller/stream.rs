use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::super::session::validate_digest;

const STREAM_METADATA_SCHEMA_VERSION: u32 = 1;
pub(super) const MAX_CLASSIFIED_STREAM_BYTES: u64 = 64 * 1024;
const MAX_CLASSIFIED_STREAM_CHUNKS: u64 = 1_024;

const SENSITIVE_STREAM_MARKERS: [&str; 9] = [
    "authorization",
    "bearer ",
    "api_key",
    "api-key",
    "password",
    "credential",
    "secret",
    "token",
    "sk-",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FakeStreamMetadata {
    pub(super) schema_version: u32,
    pub(super) observed_chunks: u64,
    pub(super) classified_chunks: u64,
    pub(super) observed_bytes: u64,
    pub(super) classified_bytes: u64,
    pub(super) dropped_bytes: u64,
    pub(super) invalid_utf8_chunks: u64,
    pub(super) redacted_chunks: u64,
    pub(super) control_bytes: u64,
    pub(super) metadata_digest: String,
}

impl FakeStreamMetadata {
    pub(super) fn empty() -> Result<Self> {
        let mut metadata = Self {
            schema_version: STREAM_METADATA_SCHEMA_VERSION,
            observed_chunks: 0,
            classified_chunks: 0,
            observed_bytes: 0,
            classified_bytes: 0,
            dropped_bytes: 0,
            invalid_utf8_chunks: 0,
            redacted_chunks: 0,
            control_bytes: 0,
            metadata_digest: String::new(),
        };
        metadata.refresh_digest()?;
        Ok(metadata)
    }

    pub(super) fn observe(&mut self, bytes: &[u8]) -> Result<bool> {
        if bytes.is_empty() {
            return Ok(false);
        }

        self.observed_chunks = self.observed_chunks.saturating_add(1);
        self.observed_bytes = self.observed_bytes.saturating_add(bytes.len() as u64);

        let remaining_bytes = MAX_CLASSIFIED_STREAM_BYTES.saturating_sub(self.classified_bytes);
        let can_classify_chunk = self.classified_chunks < MAX_CLASSIFIED_STREAM_CHUNKS;
        let classified_length = if can_classify_chunk {
            remaining_bytes.min(bytes.len() as u64) as usize
        } else {
            0
        };
        let classified = &bytes[..classified_length];

        if classified_length > 0 {
            self.classified_chunks += 1;
            self.classified_bytes += classified_length as u64;
            let invalid_utf8 = std::str::from_utf8(classified).is_err();
            if invalid_utf8 {
                self.invalid_utf8_chunks += 1;
            }
            let control_bytes = classified
                .iter()
                .filter(|byte| byte.is_ascii_control() && !matches!(byte, b'\n' | b'\r' | b'\t'))
                .count() as u64;
            self.control_bytes = self.control_bytes.saturating_add(control_bytes);

            let normalized = String::from_utf8_lossy(classified).to_ascii_lowercase();
            if invalid_utf8
                || control_bytes > 0
                || SENSITIVE_STREAM_MARKERS
                    .iter()
                    .any(|marker| normalized.contains(marker))
            {
                self.redacted_chunks += 1;
            }
        }

        self.dropped_bytes = self
            .dropped_bytes
            .saturating_add((bytes.len() - classified_length) as u64);
        self.refresh_digest()?;
        self.validate()?;
        Ok(true)
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.schema_version != STREAM_METADATA_SCHEMA_VERSION
            || self.classified_chunks > MAX_CLASSIFIED_STREAM_CHUNKS
            || self.classified_chunks > self.observed_chunks
            || self.classified_bytes > MAX_CLASSIFIED_STREAM_BYTES
            || self.invalid_utf8_chunks > self.classified_chunks
            || self.redacted_chunks > self.classified_chunks
            || self.control_bytes > self.classified_bytes
        {
            bail!("Workbench fake process stream metadata is invalid");
        }
        if self
            .classified_bytes
            .checked_add(self.dropped_bytes)
            .ok_or_else(|| anyhow!("Workbench fake process stream byte counters overflow"))?
            != self.observed_bytes
        {
            bail!("Workbench fake process stream byte accounting is inconsistent");
        }
        validate_digest(&self.metadata_digest, "fake process stream metadata digest")?;
        if self.metadata_digest != self.expected_digest()? {
            bail!("Workbench fake process stream metadata digest does not match its counters");
        }
        Ok(())
    }

    fn refresh_digest(&mut self) -> Result<()> {
        self.metadata_digest = self.expected_digest()?;
        Ok(())
    }

    fn expected_digest(&self) -> Result<String> {
        sha256_json(&serde_json::json!({
            "schemaVersion": self.schema_version,
            "observedChunks": self.observed_chunks,
            "classifiedChunks": self.classified_chunks,
            "observedBytes": self.observed_bytes,
            "classifiedBytes": self.classified_bytes,
            "droppedBytes": self.dropped_bytes,
            "invalidUtf8Chunks": self.invalid_utf8_chunks,
            "redactedChunks": self.redacted_chunks,
            "controlBytes": self.control_bytes,
        }))
    }
}

fn sha256_json(value: &serde_json::Value) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value)?)
    ))
}
