//! Provider-neutral, content-free process-start grant receipts.
//!
//! This module contains only the serialized grant contract and its integrity
//! rules. Grant issuance, UUID and clock selection, authority locking,
//! persistence, and process execution remain platform-adapter concerns.

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PROCESS_START_CAPABILITY_ID: &str = "adapter_process_start";
pub const PROCESS_START_GRANT_TTL_SECONDS: i64 = 15 * 60;
pub const GRANT_SCHEMA_VERSION: u32 = 1;
pub const GRANTED: &str = "granted";
pub const EXPIRED: &str = "expired";
pub const REVOKED: &str = "revoked";

const MAX_IDENTIFIER_LENGTH: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkbenchProcessStartGrant {
    pub schema_version: u32,
    pub grant_id: String,
    pub session_id: String,
    pub plan_id: String,
    pub process_run_id: String,
    pub capability_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub status: String,
    pub revoked_at: Option<String>,
    pub execution_enabled: bool,
    pub provider_traffic: String,
    pub writes_enabled: bool,
    pub receipt_digest: String,
}

/// Validates the complete persisted grant contract, including its receipt hash.
pub fn validate_process_start_grant(grant: &WorkbenchProcessStartGrant) -> Result<()> {
    if grant.schema_version != GRANT_SCHEMA_VERSION {
        bail!("Workbench process grant schema is unsupported");
    }
    for (value, label) in [
        (&grant.grant_id, "process grant ID"),
        (&grant.session_id, "session ID"),
        (&grant.plan_id, "plan ID"),
        (&grant.process_run_id, "process run ID"),
    ] {
        validate_identifier(value, label)?;
    }
    if grant.capability_id != PROCESS_START_CAPABILITY_ID
        || !matches!(grant.status.as_str(), GRANTED | EXPIRED | REVOKED)
        || grant.execution_enabled
        || grant.provider_traffic != "none"
        || grant.writes_enabled
    {
        bail!("Workbench process grant violates the non-executing boundary");
    }

    let issued_at = parse_timestamp(&grant.issued_at, "issue")?;
    let expires_at = parse_timestamp(&grant.expires_at, "expiry")?;
    if expires_at.signed_duration_since(issued_at)
        != Duration::seconds(PROCESS_START_GRANT_TTL_SECONDS)
    {
        bail!("Workbench process grant expiry must use the native fixed policy");
    }
    match (grant.status.as_str(), grant.revoked_at.as_deref()) {
        (GRANTED | EXPIRED, None) => {}
        (REVOKED, Some(revoked_at)) => {
            let revoked_at = parse_timestamp(revoked_at, "revoke")?;
            if revoked_at < issued_at || revoked_at > expires_at {
                bail!("Workbench process grant revoke time is outside its validity window");
            }
        }
        _ => bail!("Workbench process grant revoke state is invalid"),
    }
    if grant.receipt_digest != process_start_grant_digest(grant)? {
        bail!("Workbench process grant receipt digest does not match its content");
    }
    Ok(())
}

impl WorkbenchProcessStartGrant {
    pub fn validate(&self) -> Result<()> {
        validate_process_start_grant(self)
    }

    pub fn effective_state_at(&self, now: DateTime<Utc>) -> Result<&'static str> {
        let issued_at = parse_timestamp(&self.issued_at, "issue")?;
        let expires_at = parse_timestamp(&self.expires_at, "expiry")?;
        Ok(if self.status == REVOKED {
            REVOKED
        } else if self.status == EXPIRED || now < issued_at || now >= expires_at {
            EXPIRED
        } else {
            "active"
        })
    }

    pub fn require_active_at(&self, now: DateTime<Utc>) -> Result<()> {
        self.validate()?;
        if self.effective_state_at(now)? != "active" {
            bail!("Workbench process grant is not active");
        }
        Ok(())
    }
}

/// Computes the exact receipt digest used by the persisted grant wire contract.
pub fn process_start_grant_digest(grant: &WorkbenchProcessStartGrant) -> Result<String> {
    let canonical = serde_json::json!({
        "schemaVersion": grant.schema_version,
        "grantId": &grant.grant_id,
        "sessionId": &grant.session_id,
        "planId": &grant.plan_id,
        "processRunId": &grant.process_run_id,
        "capabilityId": &grant.capability_id,
        "issuedAt": &grant.issued_at,
        "expiresAt": &grant.expires_at,
        "status": &grant.status,
        "revokedAt": &grant.revoked_at,
        "executionEnabled": grant.execution_enabled,
        "providerTraffic": &grant.provider_traffic,
        "writesEnabled": grant.writes_enabled,
    });
    let bytes = serde_json::to_vec(&canonical).context("canonicalizing Workbench process grant")?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.len() > MAX_IDENTIFIER_LENGTH
        || value.chars().any(char::is_control)
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.')
        })
    {
        bail!("Workbench {label} must be a bounded opaque identifier");
    }
    Ok(())
}

fn parse_timestamp(value: &str, label: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow!("Workbench process grant {label} time is invalid"))
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn grant() -> WorkbenchProcessStartGrant {
        let mut grant = WorkbenchProcessStartGrant {
            schema_version: GRANT_SCHEMA_VERSION,
            grant_id: "process-grant:test".into(),
            session_id: "workbench:test".into(),
            plan_id: "run-plan:test".into(),
            process_run_id: "process-run:test".into(),
            capability_id: PROCESS_START_CAPABILITY_ID.into(),
            issued_at: "2026-08-23T00:00:00Z".into(),
            expires_at: "2026-08-23T00:15:00Z".into(),
            status: GRANTED.into(),
            revoked_at: None,
            execution_enabled: false,
            provider_traffic: "none".into(),
            writes_enabled: false,
            receipt_digest: String::new(),
        };
        grant.receipt_digest = process_start_grant_digest(&grant).expect("digest grant");
        grant
    }

    #[test]
    fn digest_is_deterministic_and_binds_content() {
        let original = grant();
        assert_eq!(
            process_start_grant_digest(&original).expect("digest"),
            process_start_grant_digest(&original).expect("digest")
        );
        let mut changed = original.clone();
        changed.plan_id = "run-plan:changed".into();
        assert_ne!(
            process_start_grant_digest(&original).expect("digest"),
            process_start_grant_digest(&changed).expect("digest")
        );
    }

    #[test]
    fn wire_contract_rejects_unknown_fields() {
        let mut value = serde_json::to_value(grant()).expect("serialize grant");
        value["prompt"] = serde_json::json!("must not persist");
        assert!(serde_json::from_value::<WorkbenchProcessStartGrant>(value).is_err());
    }

    #[test]
    fn validation_rejects_invalid_digest_and_expiry() {
        let mut invalid = grant();
        invalid.receipt_digest = digest('f');
        assert!(invalid.validate().is_err());

        let mut invalid = grant();
        invalid.expires_at = "2026-08-23T00:16:00Z".into();
        invalid.receipt_digest = process_start_grant_digest(&invalid).expect("digest grant");
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn validation_preserves_plan_only_flags_and_status_rules() {
        let mut invalid = grant();
        invalid.execution_enabled = true;
        invalid.receipt_digest = process_start_grant_digest(&invalid).expect("digest grant");
        assert!(invalid.validate().is_err());

        let mut revoked = grant();
        revoked.status = REVOKED.into();
        revoked.revoked_at = Some("2026-08-23T00:01:00Z".into());
        revoked.receipt_digest = process_start_grant_digest(&revoked).expect("digest grant");
        revoked.validate().expect("valid revoked receipt");
        assert_eq!(
            revoked.effective_state_at(Utc::now()).expect("state"),
            REVOKED
        );
    }
}
