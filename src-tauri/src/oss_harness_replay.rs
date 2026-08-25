use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MAX_EVENTS: usize = 10_000;
const MAX_INPUT_BYTES: u64 = 10 * 1024 * 1024;
const MAX_IDENTIFIER_LENGTH: usize = 128;
const REPLAY_REFERENCE_SCHEMA_VERSION: u8 = 1;
const REPLAY_REFERENCE_LEDGER_SCHEMA_VERSION: u8 = 1;
const MAX_REPLAY_REFERENCES: usize = 64;
const REPLAY_REFERENCE_LEDGER_FILE: &str = "oss-harness-replay-references.json";

static REPLAY_REFERENCE_STORE_LOCK: Mutex<()> = Mutex::new(());

const ROUTES: [&str; 6] = [
    "ingress",
    "headroom",
    "direct_anthropic",
    "direct_openai",
    "cache",
    "switchyard_observe",
];
const OUTCOMES: [&str; 8] = [
    "success",
    "upstream_http_error",
    "connect_failure",
    "write_failure",
    "read_failure",
    "timeout",
    "client_disconnect",
    "local_rejection",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplayInput {
    schema_version: u8,
    events: Vec<ReplayEvent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplayEvent {
    event_id: String,
    task_class: String,
    route: String,
    outcome: String,
    latency_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalEvent<'a> {
    event_id: &'a str,
    task_class: &'a str,
    route: &'a str,
    outcome: &'a str,
    latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OssHarnessReplayResult {
    pub schema_version: u8,
    pub replay_mode: String,
    pub automatic_promotion: String,
    pub provider_traffic: String,
    pub event_count: usize,
    pub route_counts: BTreeMap<String, usize>,
    pub outcome_counts: BTreeMap<String, usize>,
    pub latency: OssHarnessReplayLatency,
    pub replay_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OssHarnessReplayLatency {
    pub sample_count: usize,
    pub p95_ms: Option<u64>,
}

/// A native-issued, content-free receipt for an already validated replay.
///
/// This deliberately excludes the selected source path and every replay event.
/// The `replay_digest` still identifies the validated canonical event stream;
/// `receipt_digest` protects the persisted reference metadata itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OssHarnessReplayReference {
    pub schema_version: u8,
    pub replay_id: String,
    pub validated_at: String,
    pub replay_mode: String,
    pub automatic_promotion: String,
    pub provider_traffic: String,
    pub event_count: usize,
    pub replay_digest: String,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OssHarnessReplayValidation {
    pub result: OssHarnessReplayResult,
    pub reference: OssHarnessReplayReference,
}

#[tauri::command]
pub fn replay_redacted_route_events(path: String) -> Result<OssHarnessReplayValidation, String> {
    let result = replay_redacted_route_events_from_path(Path::new(path.trim()))
        .map_err(|error| error.to_string())?;
    let (_guard, store) = locked_replay_reference_store()?;
    let reference = store.record(&result).map_err(|error| error.to_string())?;
    Ok(OssHarnessReplayValidation { result, reference })
}

#[tauri::command]
pub fn list_oss_harness_replay_references() -> Result<Vec<OssHarnessReplayReference>, String> {
    let (_guard, store) = locked_replay_reference_store()?;
    store.list().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn resolve_oss_harness_replay_reference(
    replay_id: String,
) -> Result<OssHarnessReplayReference, String> {
    resolve_oss_harness_replay_reference_for_workbench(replay_id.trim())
        .map_err(|error| error.to_string())
}

pub(crate) fn resolve_oss_harness_replay_reference_for_workbench(
    replay_id: &str,
) -> Result<OssHarnessReplayReference> {
    let (_guard, store) = locked_replay_reference_store().map_err(|error| anyhow!(error))?;
    store.resolve(replay_id)
}

fn locked_replay_reference_store(
) -> std::result::Result<(std::sync::MutexGuard<'static, ()>, ReplayReferenceStore), String> {
    let guard = REPLAY_REFERENCE_STORE_LOCK
        .lock()
        .map_err(|_| "Replay receipt ledger lock is unavailable".to_string())?;
    Ok((guard, ReplayReferenceStore::in_app_storage()))
}

fn replay_redacted_route_events_from_path(path: &Path) -> Result<OssHarnessReplayResult> {
    if path.as_os_str().is_empty() {
        return Err(anyhow!("replay file path is required"));
    }
    let metadata = std::fs::metadata(path).context("reading replay file metadata")?;
    if !metadata.is_file() {
        return Err(anyhow!("replay path must be a file"));
    }
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(anyhow!("replay input exceeds 10 MiB"));
    }
    let raw = std::fs::read_to_string(path).context("reading replay file")?;
    let value: serde_json::Value = serde_json::from_str(&raw).context("parsing replay JSON")?;
    reject_sensitive(&value, "replay")?;
    let input: ReplayInput = serde_json::from_value(value).context("decoding replay schema")?;
    if input.schema_version != 1 {
        return Err(anyhow!("replay input requires schemaVersion 1"));
    }
    if input.events.len() > MAX_EVENTS {
        return Err(anyhow!("replay input exceeds 10000 events"));
    }

    let mut event_ids = BTreeSet::new();
    let mut canonical = Vec::with_capacity(input.events.len());
    for (index, event) in input.events.iter().enumerate() {
        validate_identifier(&event.event_id, "eventId", index)?;
        validate_identifier(&event.task_class, "taskClass", index)?;
        validate_identifier(&event.route, "route", index)?;
        validate_identifier(&event.outcome, "outcome", index)?;
        if !ROUTES.contains(&event.route.as_str()) {
            return Err(anyhow!("event {index} has unsupported route"));
        }
        if !OUTCOMES.contains(&event.outcome.as_str()) {
            return Err(anyhow!("event {index} has unsupported outcome"));
        }
        if !event_ids.insert(&event.event_id) {
            return Err(anyhow!("duplicate eventId: {}", event.event_id));
        }
        canonical.push(CanonicalEvent {
            event_id: &event.event_id,
            task_class: &event.task_class,
            route: &event.route,
            outcome: &event.outcome,
            latency_ms: event.latency_ms,
        });
    }

    let mut route_counts = ROUTES
        .into_iter()
        .map(|route| (route.to_string(), 0))
        .collect::<BTreeMap<_, _>>();
    let mut outcome_counts = OUTCOMES
        .into_iter()
        .map(|outcome| (outcome.to_string(), 0))
        .collect::<BTreeMap<_, _>>();
    let mut latencies = Vec::new();
    for event in &canonical {
        *route_counts.get_mut(event.route).expect("validated route") += 1;
        *outcome_counts
            .get_mut(event.outcome)
            .expect("validated outcome") += 1;
        if let Some(latency) = event.latency_ms {
            latencies.push(latency);
        }
    }
    latencies.sort_unstable();
    let p95_ms = (!latencies.is_empty()).then(|| {
        let index = ((latencies.len() * 95).div_ceil(100)).saturating_sub(1);
        latencies[index.min(latencies.len() - 1)]
    });
    let canonical_json = serde_json::to_vec(&canonical).context("canonicalizing replay events")?;
    let digest = format!("sha256:{:x}", Sha256::digest(canonical_json));
    Ok(OssHarnessReplayResult {
        schema_version: 1,
        replay_mode: "redacted_observe_only".to_string(),
        automatic_promotion: "disabled".to_string(),
        provider_traffic: "none".to_string(),
        event_count: canonical.len(),
        route_counts,
        outcome_counts,
        latency: OssHarnessReplayLatency {
            sample_count: latencies.len(),
            p95_ms,
        },
        replay_digest: digest,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplayReferenceLedger {
    schema_version: u8,
    references: BTreeMap<String, OssHarnessReplayReference>,
}

impl Default for ReplayReferenceLedger {
    fn default() -> Self {
        Self {
            schema_version: REPLAY_REFERENCE_LEDGER_SCHEMA_VERSION,
            references: BTreeMap::new(),
        }
    }
}

struct ReplayReferenceStore {
    path: PathBuf,
}

impl ReplayReferenceStore {
    fn in_app_storage() -> Self {
        Self {
            path: crate::storage::config_file(
                &crate::storage::app_data_dir(),
                REPLAY_REFERENCE_LEDGER_FILE,
            ),
        }
    }

    #[cfg(test)]
    fn at(path: PathBuf) -> Self {
        Self { path }
    }

    fn record(&self, result: &OssHarnessReplayResult) -> Result<OssHarnessReplayReference> {
        validate_replay_result(result)?;
        let mut ledger = self.load()?;
        trim_replay_references(&mut ledger.references);
        let replay_id = format!("replay-reference-{}", Uuid::new_v4());
        let validated_at = Utc::now().to_rfc3339();
        let mut reference = OssHarnessReplayReference {
            schema_version: REPLAY_REFERENCE_SCHEMA_VERSION,
            replay_id: replay_id.clone(),
            validated_at,
            replay_mode: result.replay_mode.clone(),
            automatic_promotion: result.automatic_promotion.clone(),
            provider_traffic: result.provider_traffic.clone(),
            event_count: result.event_count,
            replay_digest: result.replay_digest.clone(),
            receipt_digest: String::new(),
        };
        reference.receipt_digest = replay_reference_digest(&reference)?;
        validate_replay_reference(&reference)?;
        ledger.references.insert(replay_id, reference.clone());
        self.save(&ledger)?;
        Ok(reference)
    }

    fn list(&self) -> Result<Vec<OssHarnessReplayReference>> {
        let mut references = self.load()?.references.into_values().collect::<Vec<_>>();
        references.sort_by(|left, right| {
            right
                .validated_at
                .cmp(&left.validated_at)
                .then_with(|| left.replay_id.cmp(&right.replay_id))
        });
        Ok(references)
    }

    fn resolve(&self, replay_id: &str) -> Result<OssHarnessReplayReference> {
        if !valid_replay_reference_id(replay_id) {
            return Err(anyhow!("Replay receipt ID is invalid"));
        }
        self.load()?
            .references
            .remove(replay_id)
            .ok_or_else(|| anyhow!("Replay receipt is unknown or unavailable"))
    }

    fn load(&self) -> Result<ReplayReferenceLedger> {
        if !self.path.exists() {
            return Ok(ReplayReferenceLedger::default());
        }
        let bytes = std::fs::read(&self.path).context("reading replay receipt ledger")?;
        let ledger: ReplayReferenceLedger =
            serde_json::from_slice(&bytes).context("decoding replay receipt ledger")?;
        if ledger.schema_version != REPLAY_REFERENCE_LEDGER_SCHEMA_VERSION {
            return Err(anyhow!("Unsupported replay receipt ledger schema version"));
        }
        if ledger.references.len() > MAX_REPLAY_REFERENCES {
            return Err(anyhow!("Replay receipt ledger exceeds its retention cap"));
        }
        for (replay_id, reference) in &ledger.references {
            if replay_id != &reference.replay_id {
                return Err(anyhow!("Replay receipt key does not match its payload"));
            }
            validate_replay_reference(reference)?;
        }
        Ok(ledger)
    }

    fn save(&self, ledger: &ReplayReferenceLedger) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).context("creating replay receipt ledger directory")?;
        }
        let temporary = self
            .path
            .with_extension(format!("json.tmp.{}", std::process::id()));
        let bytes = serde_json::to_vec_pretty(ledger).context("encoding replay receipt ledger")?;
        std::fs::write(&temporary, bytes).context("writing replay receipt ledger")?;
        std::fs::rename(&temporary, &self.path).context("committing replay receipt ledger")
    }
}

fn trim_replay_references(references: &mut BTreeMap<String, OssHarnessReplayReference>) {
    while references.len() >= MAX_REPLAY_REFERENCES {
        let oldest = references
            .values()
            .min_by(|left, right| {
                left.validated_at
                    .cmp(&right.validated_at)
                    .then_with(|| left.replay_id.cmp(&right.replay_id))
            })
            .map(|reference| reference.replay_id.clone());
        if let Some(replay_id) = oldest {
            references.remove(&replay_id);
        } else {
            break;
        }
    }
}

fn validate_replay_result(result: &OssHarnessReplayResult) -> Result<()> {
    if result.schema_version != 1
        || result.replay_mode != "redacted_observe_only"
        || result.automatic_promotion != "disabled"
        || result.provider_traffic != "none"
        || result.event_count > MAX_EVENTS
        || result.latency.sample_count > result.event_count
        || !valid_sha256_digest(&result.replay_digest)
    {
        return Err(anyhow!(
            "Replay result does not satisfy the observe-only receipt contract"
        ));
    }
    if result.route_counts.values().sum::<usize>() != result.event_count
        || result.outcome_counts.values().sum::<usize>() != result.event_count
    {
        return Err(anyhow!(
            "Replay result counters do not match its event count"
        ));
    }
    Ok(())
}

pub(crate) fn replay_reference_digest(reference: &OssHarnessReplayReference) -> Result<String> {
    let canonical = serde_json::json!({
        "schemaVersion": reference.schema_version,
        "replayId": reference.replay_id,
        "validatedAt": reference.validated_at,
        "replayMode": reference.replay_mode,
        "automaticPromotion": reference.automatic_promotion,
        "providerTraffic": reference.provider_traffic,
        "eventCount": reference.event_count,
        "replayDigest": reference.replay_digest,
    });
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).context("canonicalizing replay receipt")?)
    ))
}

pub(crate) fn validate_replay_reference(reference: &OssHarnessReplayReference) -> Result<()> {
    if reference.schema_version != REPLAY_REFERENCE_SCHEMA_VERSION
        || !valid_replay_reference_id(&reference.replay_id)
        || DateTime::parse_from_rfc3339(&reference.validated_at).is_err()
        || reference.replay_mode != "redacted_observe_only"
        || reference.automatic_promotion != "disabled"
        || reference.provider_traffic != "none"
        || reference.event_count > MAX_EVENTS
        || !valid_sha256_digest(&reference.replay_digest)
        || !valid_sha256_digest(&reference.receipt_digest)
    {
        return Err(anyhow!(
            "Replay receipt has an invalid content-free contract"
        ));
    }
    if replay_reference_digest(reference)? != reference.receipt_digest {
        return Err(anyhow!("Replay receipt digest does not match its metadata"));
    }
    Ok(())
}

fn valid_replay_reference_id(value: &str) -> bool {
    value
        .strip_prefix("replay-reference-")
        .and_then(|suffix| Uuid::parse_str(suffix).ok())
        .is_some()
}

fn valid_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .map(|digest| {
            digest.len() == 64
                && digest
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
        })
        .unwrap_or(false)
}

fn validate_identifier(value: &str, label: &str, index: usize) -> Result<()> {
    if value.trim().is_empty()
        || value.len() > MAX_IDENTIFIER_LENGTH
        || value != value.trim()
        || value.chars().any(char::is_control)
    {
        return Err(anyhow!("event {index} requires a bounded {label}"));
    }
    Ok(())
}

fn reject_sensitive(value: &serde_json::Value, path: &str) -> Result<()> {
    const FORBIDDEN: [&str; 13] = [
        "prompt",
        "messages",
        "input",
        "output",
        "response",
        "body",
        "headers",
        "authorization",
        "apikey",
        "api_key",
        "token",
        "secret",
        "credential",
    ];
    match value {
        serde_json::Value::Object(object) => {
            for (key, child) in object {
                if FORBIDDEN.contains(&key.to_ascii_lowercase().as_str()) {
                    return Err(anyhow!("sensitive field is not allowed: {path}.{key}"));
                }
                reject_sensitive(child, &format!("{path}.{key}"))?;
            }
        }
        serde_json::Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                reject_sensitive(child, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn replay_file(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("create replay fixture");
        file.write_all(contents.as_bytes())
            .expect("write replay fixture");
        file
    }

    #[test]
    fn valid_replay_is_content_free_and_deterministic() {
        let file = replay_file(
            r#"{"schemaVersion":1,"events":[{"eventId":"e-1","taskClass":"coding","route":"headroom","outcome":"success","latencyMs":100},{"eventId":"e-2","taskClass":"coding","route":"cache","outcome":"timeout","latencyMs":200}]}"#,
        );
        let first = replay_redacted_route_events_from_path(file.path()).expect("valid replay");
        let second = replay_redacted_route_events_from_path(file.path()).expect("repeat replay");
        assert_eq!(first, second);
        assert_eq!(first.event_count, 2);
        assert_eq!(first.latency.p95_ms, Some(200));
        assert_eq!(first.automatic_promotion, "disabled");
        assert_eq!(first.provider_traffic, "none");
    }

    #[test]
    fn matches_the_shared_replay_golden_output_and_digest() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/oss-harness/replay-golden.json"
        ))
        .expect("parse replay golden fixture");
        let input = serde_json::to_string(&fixture["input"]).expect("encode replay input");
        let file = replay_file(&input);
        let actual = replay_redacted_route_events_from_path(file.path()).expect("valid replay");
        let actual = serde_json::to_value(actual).expect("encode replay result");
        assert_eq!(actual, fixture["expected"]);
        assert_eq!(actual["automaticPromotion"], "disabled");
        assert_eq!(actual["providerTraffic"], "none");
        assert!(actual["replayDigest"].as_str().is_some_and(|digest| {
            digest.starts_with("sha256:") && digest.len() == 71
        }));
    }

    #[test]
    fn rejects_sensitive_duplicate_and_oversized_replays() {
        let sensitive = replay_file(
            r#"{"schemaVersion":1,"events":[{"eventId":"e-1","taskClass":"coding","route":"headroom","outcome":"success","prompt":"no"}]}"#,
        );
        assert!(replay_redacted_route_events_from_path(sensitive.path()).is_err());
        let duplicate = replay_file(
            r#"{"schemaVersion":1,"events":[{"eventId":"e-1","taskClass":"coding","route":"headroom","outcome":"success"},{"eventId":"e-1","taskClass":"coding","route":"cache","outcome":"success"}]}"#,
        );
        assert!(replay_redacted_route_events_from_path(duplicate.path()).is_err());
        let oversized = replay_file(&format!(
            "{{\"schemaVersion\":1,\"events\":[{}]}}",
            "x".repeat(MAX_INPUT_BYTES as usize)
        ));
        assert!(replay_redacted_route_events_from_path(oversized.path()).is_err());
    }

    #[test]
    fn replay_receipt_is_bounded_content_free_and_detects_tampering() {
        let source = replay_file(
            r#"{"schemaVersion":1,"events":[{"eventId":"private-event-1","taskClass":"coding","route":"headroom","outcome":"success","latencyMs":100}]}"#,
        );
        let result =
            replay_redacted_route_events_from_path(source.path()).expect("validate replay");
        let directory = tempfile::tempdir().expect("create receipt directory");
        let path = directory.path().join("replay-receipts.json");
        let store = ReplayReferenceStore::at(path.clone());

        let reference = store.record(&result).expect("record receipt");
        assert_eq!(reference.replay_mode, "redacted_observe_only");
        assert_eq!(reference.automatic_promotion, "disabled");
        assert_eq!(reference.provider_traffic, "none");
        assert_eq!(
            store
                .resolve(&reference.replay_id)
                .expect("resolve receipt"),
            reference
        );
        let persisted = std::fs::read_to_string(&path).expect("read receipt ledger");
        assert!(!persisted.contains(&source.path().display().to_string()));
        assert!(!persisted.contains("private-event-1"));
        assert!(!persisted.contains("taskClass"));

        let mut tampered: serde_json::Value =
            serde_json::from_str(&persisted).expect("decode ledger");
        tampered["references"][reference.replay_id.as_str()]["eventCount"] = serde_json::json!(99);
        std::fs::write(
            &path,
            serde_json::to_vec(&tampered).expect("encode tampered ledger"),
        )
        .expect("write tampered ledger");
        assert!(store.resolve(&reference.replay_id).is_err());
    }

    #[test]
    fn replay_receipt_retention_keeps_a_bounded_native_list() {
        let source = replay_file(
            r#"{"schemaVersion":1,"events":[{"eventId":"e-1","taskClass":"coding","route":"headroom","outcome":"success"}]}"#,
        );
        let result =
            replay_redacted_route_events_from_path(source.path()).expect("validate replay");
        let directory = tempfile::tempdir().expect("create receipt directory");
        let store = ReplayReferenceStore::at(directory.path().join("replay-receipts.json"));
        for _ in 0..=MAX_REPLAY_REFERENCES {
            store.record(&result).expect("record receipt");
        }
        assert_eq!(
            store.list().expect("list receipts").len(),
            MAX_REPLAY_REFERENCES
        );
    }
}
