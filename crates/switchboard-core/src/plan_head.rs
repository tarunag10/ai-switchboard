//! Content-free identity receipt for a durable Workbench plan head.
//!
//! This contract deliberately contains no plan payload, filesystem location,
//! runtime handle, provider request, or process authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PLAN_HEAD_RECEIPT_SCHEMA_VERSION: u32 = 1;

const PLAN_HEAD_ID_DOMAIN: &[u8] = b"ai-switchboard-workbench-plan-head-id-v1\0";
const PLAN_HEAD_RECEIPT_DOMAIN: &[u8] = b"ai-switchboard-workbench-plan-head-receipt-v1\0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanHeadReceipt {
    pub schema_version: u32,
    pub head_id: String,
    pub session_id: String,
    pub session_snapshot_digest: String,
    pub generation: u64,
    pub plan_id: String,
    pub plan_snapshot_digest: String,
    pub predecessor_head_id: Option<String>,
    pub predecessor_record_digest: Option<String>,
    pub record_digest: String,
}

/// Derives the stable, ledger-scoped identity for one plan-head generation.
pub fn plan_head_identity(
    ledger_id: &str,
    generation: u64,
    session_id: &str,
    session_snapshot_digest: &str,
    plan_snapshot_digest: &str,
    predecessor_head_id: Option<&str>,
    predecessor_record_digest: Option<&str>,
) -> String {
    let generation = generation.to_string();
    let digest = domain_digest(
        PLAN_HEAD_ID_DOMAIN,
        &[
            ledger_id.as_bytes(),
            generation.as_bytes(),
            session_id.as_bytes(),
            session_snapshot_digest.as_bytes(),
            plan_snapshot_digest.as_bytes(),
            predecessor_head_id.unwrap_or("none").as_bytes(),
            predecessor_record_digest.unwrap_or("none").as_bytes(),
        ],
    );
    format!("workbench-plan-head:{}", &digest["sha256:".len()..39])
}

/// Digests every durable receipt field except the digest itself.
pub fn plan_head_receipt_digest(receipt: &PlanHeadReceipt) -> String {
    let schema_version = receipt.schema_version.to_string();
    let generation = receipt.generation.to_string();
    domain_digest(
        PLAN_HEAD_RECEIPT_DOMAIN,
        &[
            schema_version.as_bytes(),
            receipt.head_id.as_bytes(),
            receipt.session_id.as_bytes(),
            receipt.session_snapshot_digest.as_bytes(),
            generation.as_bytes(),
            receipt.plan_id.as_bytes(),
            receipt.plan_snapshot_digest.as_bytes(),
            receipt
                .predecessor_head_id
                .as_deref()
                .unwrap_or("none")
                .as_bytes(),
            receipt
                .predecessor_record_digest
                .as_deref()
                .unwrap_or("none")
                .as_bytes(),
        ],
    )
}

fn domain_digest(domain: &[u8], fields: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn receipt() -> PlanHeadReceipt {
        let session_snapshot_digest = digest('a');
        let plan_snapshot_digest = digest('b');
        let head_id = plan_head_identity(
            "workbench-plan-head-ledger:test",
            1,
            "workbench:test",
            &session_snapshot_digest,
            &plan_snapshot_digest,
            None,
            None,
        );
        let mut receipt = PlanHeadReceipt {
            schema_version: PLAN_HEAD_RECEIPT_SCHEMA_VERSION,
            head_id,
            session_id: "workbench:test".into(),
            session_snapshot_digest,
            generation: 1,
            plan_id: "run-plan:test".into(),
            plan_snapshot_digest,
            predecessor_head_id: None,
            predecessor_record_digest: None,
            record_digest: String::new(),
        };
        receipt.record_digest = plan_head_receipt_digest(&receipt);
        receipt
    }

    #[test]
    fn identity_is_deterministic_and_generation_bound() {
        let receipt = receipt();
        let repeated = plan_head_identity(
            "workbench-plan-head-ledger:test",
            1,
            &receipt.session_id,
            &receipt.session_snapshot_digest,
            &receipt.plan_snapshot_digest,
            None,
            None,
        );
        let next_generation = plan_head_identity(
            "workbench-plan-head-ledger:test",
            2,
            &receipt.session_id,
            &receipt.session_snapshot_digest,
            &receipt.plan_snapshot_digest,
            Some(&receipt.head_id),
            Some(&receipt.record_digest),
        );

        assert_eq!(receipt.head_id, repeated);
        assert_ne!(receipt.head_id, next_generation);
    }

    #[test]
    fn receipt_digest_is_deterministic_and_tamper_evident() {
        let receipt = receipt();
        assert_eq!(receipt.record_digest, plan_head_receipt_digest(&receipt));

        let mut changed = receipt.clone();
        changed.plan_id = "run-plan:changed".into();
        assert_ne!(receipt.record_digest, plan_head_receipt_digest(&changed));
    }

    #[test]
    fn serde_contract_round_trips_and_rejects_unknown_fields() {
        let receipt = receipt();
        let encoded = serde_json::to_value(&receipt).expect("serialize plan-head receipt");
        let decoded: PlanHeadReceipt =
            serde_json::from_value(encoded.clone()).expect("deserialize plan-head receipt");
        assert_eq!(decoded, receipt);

        let mut unknown = encoded;
        unknown["prompt"] = serde_json::json!("must not be accepted");
        assert!(serde_json::from_value::<PlanHeadReceipt>(unknown).is_err());
    }
}
