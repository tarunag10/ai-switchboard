use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_EVENTS: usize = 10_000;
const MAX_INPUT_BYTES: u64 = 10 * 1024 * 1024;
const MAX_IDENTIFIER_LENGTH: usize = 128;

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

#[tauri::command]
pub fn replay_redacted_route_events(path: String) -> Result<OssHarnessReplayResult, String> {
    replay_redacted_route_events_from_path(Path::new(path.trim()))
        .map_err(|error| error.to_string())
}

fn replay_redacted_route_events_from_path(path: &Path) -> Result<OssHarnessReplayResult> {
    if path.as_os_str().is_empty() {
        return Err(anyhow!("replay file path is required"));
    }
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("reading replay file metadata: {}", path.display()))?;
    if !metadata.is_file() {
        return Err(anyhow!("replay path must be a file"));
    }
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(anyhow!("replay input exceeds 10 MiB"));
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading replay file: {}", path.display()))?;
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
}
