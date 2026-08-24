use std::{
    fs,
    sync::{Arc, Barrier},
    thread,
};

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use uuid::Uuid;

use super::router_shadow::*;

const DECIDED_AT: &str = "2026-08-24T00:00:00.000Z";
const POLICY_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ROUTE_DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const MODEL_DIGEST: &str =
    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const CANDIDATE_MODEL_DIGEST: &str =
    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const ENDPOINT_DIGEST: &str =
    "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("test timestamp")
        .with_timezone(&Utc)
}

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn model(identity_digest: &str) -> RouterShadowModelIdentity {
    RouterShadowModelIdentity {
        provenance: RouterShadowModelProvenance::VerifiedEndpointCatalog,
        identity_digest: identity_digest.into(),
    }
}

fn decision_draft() -> RouterShadowDecisionDraft {
    RouterShadowDecisionDraft {
        source: RouterShadowSource::NativeProxyDirect,
        client_class: RouterShadowClientClass::Codex,
        request_class: RouterShadowRequestClass::OpenAiResponses,
        policy_digest: POLICY_DIGEST.into(),
        route_plan_digest: ROUTE_DIGEST.into(),
        decision_stage: RouterShadowDecisionStage::Observe,
        routing_mode: RouterShadowMode::ObserveOnly,
        task_class: RouterShadowTaskClass::CodeGeneration,
        task_classification_source: RouterShadowTaskClassificationSource::FixedRules,
        requested_model: model(MODEL_DIGEST),
        proposed_model: Some(model(CANDIDATE_MODEL_DIGEST)),
        proposed_endpoint_identity_digest: Some(ENDPOINT_DIGEST.into()),
        required_features: vec![
            RouterShadowFeature::Vision,
            RouterShadowFeature::Streaming,
            RouterShadowFeature::Tools,
        ],
        streaming_intent: RouterShadowStreamingIntent::Streaming,
    }
}

fn completion_draft(router_run_id: &str) -> RouterShadowCompletionDraft {
    RouterShadowCompletionDraft {
        router_run_id: router_run_id.into(),
        actual_transport: RouterShadowTransport::DirectOpenAi,
        forwarded_model: model(MODEL_DIGEST),
        provider_reported_model: Some(RouterShadowModelIdentity {
            provenance: RouterShadowModelProvenance::ProviderResponseCatalogMatch,
            identity_digest: MODEL_DIGEST.into(),
        }),
        upstream_started: true,
        response_headers_received: true,
        delivery_completed: true,
        status_code: Some(200),
        transport_outcome: RouterShadowOutcome::Completed,
        observed_monotonic_latency_ms: Some(1_250),
        cost_evidence_state: RouterShadowCostEvidenceState::ProviderReported,
        provider_billed_cost_microunits: Some(42),
        failure_class: None,
    }
}

fn insert_at(
    connection: &mut Connection,
    index: u128,
    decided_at: DateTime<Utc>,
) -> RouterShadowDecisionV1 {
    insert_router_shadow_decision_at_for_tests(
        connection,
        decision_draft(),
        decided_at,
        uuid(index * 2 + 1),
        uuid(index * 2 + 2),
    )
    .expect("insert Router shadow decision")
}

fn complete_at(
    connection: &mut Connection,
    decision: &RouterShadowDecisionV1,
    completed_at: DateTime<Utc>,
) -> Result<RouterShadowCompletionV1, RouterShadowStoreError> {
    complete_router_shadow_run_at_for_tests(
        connection,
        completion_draft(&decision.router_run_id),
        completed_at,
        uuid(3),
    )
}

fn sha256(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

#[test]
fn canonical_json_and_sha256_vectors_are_fixed() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let decision = insert_router_shadow_decision_at_for_tests(
        &mut connection,
        decision_draft(),
        timestamp(DECIDED_AT),
        uuid(1),
        uuid(2),
    )
    .expect("insert fixed decision");
    let expected_decision_json = concat!(
        r#"{"schemaVersion":1,"generationId":1,"decisionId":"router-shadow-decision-00000000-0000-0000-0000-000000000001","routerRunId":"router-run-00000000-0000-0000-0000-000000000002","decidedAt":"2026-08-24T00:00:00.000Z","source":"native_proxy_direct","clientClass":"codex","requestClass":"openai_responses","policyDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","routePlanDigest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","decisionStage":"observe","routingMode":"observe_only","taskClass":"code_generation","taskClassificationSource":"fixed_rules","requestedModel":{"provenance":"verified_endpoint_catalog","identityDigest":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"},"proposedModel":{"provenance":"verified_endpoint_catalog","identityDigest":"sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"},"proposedEndpointIdentityDigest":"sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","requiredFeatures":["streaming","tools","vision"],"streamingIntent":"streaming","executionModelInvariant":"preserve_requested","routeMutationApplied":false,"promotionEligible":false,"privacy":{"requestBodyRetained":false,"responseBodyRetained":false,"promptRetained":false,"headersRetained":false,"credentialsRetained":false,"toolPayloadsRetained":false,"filesystemPathsRetained":false,"endpointUrlsRetained":false,"providerRequestIdsRetained":false,"requestBodyHashRetained":false}}"#,
    )
    .replace(
        "\"generationId\":1,",
        "\"generationId\":1,\"generationDigest\":\"sha256:4ca57ebabbd2d8a378a9992faccbf2a439897c45fca72af2b26021496fc10e37\",",
    );
    assert_eq!(decision.canonical_json_for_tests(), expected_decision_json);
    assert_eq!(
        decision.canonical_digest,
        "sha256:6de77d89089d90749196acea51c0edeae02db9080dee15b5101b353e00bcf51d"
    );
    assert_eq!(decision.canonical_digest, sha256(&expected_decision_json));
    let (generation_schema, generation_created_at, generation_digest): (i64, String, String) =
        connection
            .query_row(
                "SELECT schema_version, created_at, canonical_digest \
                 FROM router_shadow_generations WHERE generation_id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load fixed generation");
    let expected_generation_json =
        r#"{"schemaVersion":1,"generationId":1,"createdAt":"2026-08-24T00:00:00.000Z"}"#;
    assert_eq!(generation_schema, 1);
    assert_eq!(generation_created_at, DECIDED_AT);
    assert_eq!(
        generation_digest,
        "sha256:4ca57ebabbd2d8a378a9992faccbf2a439897c45fca72af2b26021496fc10e37"
    );
    assert_eq!(generation_digest, sha256(expected_generation_json));

    let completion = complete_at(
        &mut connection,
        &decision,
        timestamp("2026-08-24T00:00:01.000Z"),
    )
    .expect("complete fixed decision");
    let expected_completion_json = concat!(
        r#"{"schemaVersion":1,"completionId":"router-shadow-completion-00000000-0000-0000-0000-000000000003","routerRunId":"router-run-00000000-0000-0000-0000-000000000002","completedAt":"2026-08-24T00:00:01.000Z","actualTransport":"direct_open_ai","forwardedModel":{"provenance":"verified_endpoint_catalog","identityDigest":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"},"providerReportedModel":{"provenance":"provider_response_catalog_match","identityDigest":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"},"modelPreserved":true,"upstreamStarted":true,"responseHeadersReceived":true,"deliveryCompleted":true,"statusCode":200,"transportOutcome":"completed","latencyMs":1250,"clockEvidence":"monotonic_observed","costEvidenceState":"provider_reported","providerBilledCostMicrounits":42,"qualityEvidenceState":"not_collected","reworkEvidenceState":"not_collected","failureClass":null,"decisionDigest":"DECISION_DIGEST_VECTOR","policyDigest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","routePlanDigest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#,
    )
    .replace(
        "DECISION_DIGEST_VECTOR",
        "sha256:6de77d89089d90749196acea51c0edeae02db9080dee15b5101b353e00bcf51d",
    );
    assert_eq!(
        completion.canonical_json_for_tests(),
        expected_completion_json
    );
    assert_eq!(
        completion.canonical_digest,
        "sha256:c24c27c9f5b225df6e8417330eb16e5a68ca31d88a3bcb80547a28129652ba7f"
    );
    assert_eq!(
        completion.canonical_digest,
        sha256(&expected_completion_json)
    );
}

#[test]
fn decision_round_trip_is_content_free_and_store_identified() {
    let sentinel = "sk-live-secret prompt /Users/example/repository Authorization: Bearer";
    let mut connection = Connection::open_in_memory().expect("open database");
    let decision =
        insert_router_shadow_decision(&mut connection, decision_draft()).expect("insert decision");
    let loaded =
        load_router_shadow_decision(&connection, &decision.router_run_id).expect("load decision");

    assert_eq!(loaded, decision);
    assert!(decision.decision_id.starts_with("router-shadow-decision-"));
    assert!(decision.router_run_id.starts_with("router-run-"));
    assert_eq!(decision.route_mutation_applied, false);
    assert_eq!(decision.promotion_eligible, false);
    assert_eq!(
        decision.required_features,
        vec![
            RouterShadowFeature::Streaming,
            RouterShadowFeature::Tools,
            RouterShadowFeature::Vision,
        ]
    );
    let persisted: String = connection
        .query_row(
            "SELECT group_concat(value, '|') FROM (\
             SELECT decision_id AS value FROM router_shadow_decisions UNION ALL \
             SELECT router_run_id FROM router_shadow_decisions UNION ALL \
             SELECT requested_model_json FROM router_shadow_decisions UNION ALL \
             SELECT proposed_model_json FROM router_shadow_decisions UNION ALL \
             SELECT required_features_json FROM router_shadow_decisions UNION ALL \
             SELECT privacy_json FROM router_shadow_decisions)",
            [],
            |row| row.get(0),
        )
        .expect("read persisted text");
    assert!(!persisted.contains(sentinel));
    assert!(!persisted.contains("repository"));
    assert!(!persisted.contains("Authorization"));
    assert!(!persisted.contains("/Users/"));
}

#[test]
fn privacy_attestation_is_closed_and_all_false() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let decision = insert_at(&mut connection, 1, timestamp(DECIDED_AT));
    let privacy = serde_json::to_value(&decision.privacy).expect("privacy JSON");
    let fields = privacy.as_object().expect("privacy object");
    assert_eq!(fields.len(), 10);
    assert!(fields
        .values()
        .all(|value| value == &serde_json::Value::Bool(false)));

    let with_unknown = r#"{"requestBodyRetained":false,"responseBodyRetained":false,"promptRetained":false,"headersRetained":false,"credentialsRetained":false,"toolPayloadsRetained":false,"filesystemPathsRetained":false,"endpointUrlsRetained":false,"providerRequestIdsRetained":false,"requestBodyHashRetained":false,"headroom":false}"#;
    assert!(serde_json::from_str::<RouterShadowPrivacyV1>(with_unknown).is_err());
}

#[test]
fn closed_json_types_reject_unknown_or_raw_routing_fields() {
    let model_with_name = format!(
        r#"{{"provenance":"verified_endpoint_catalog","identityDigest":"{MODEL_DIGEST}","model":"gpt-secret"}}"#
    );
    assert!(serde_json::from_str::<RouterShadowModelIdentity>(&model_with_name).is_err());
    assert!(serde_json::from_str::<RouterShadowModelIdentity>(
        r#"{"provenance":"provider_runtime","identityDigest":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}"#
    )
    .is_err());
    assert!(serde_json::from_str::<RouterShadowPrivacyV1>(r#"{"cache":false}"#).is_err());
}

#[test]
fn generations_rotate_without_evicting_old_receipts() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let now = timestamp(DECIDED_AT);
    let mut first = None;
    let mut last = None;
    for index in 0..257 {
        let decision = insert_at(&mut connection, index + 10, now);
        first.get_or_insert_with(|| decision.clone());
        last = Some(decision);
    }

    assert_eq!(first.expect("first decision").generation_id, 1);
    assert_eq!(last.expect("last decision").generation_id, 2);
    let counts: Vec<(i64, i64)> = {
        let mut statement = connection
            .prepare(
                "SELECT generation_id, COUNT(*) FROM router_shadow_decisions \
                 GROUP BY generation_id ORDER BY generation_id",
            )
            .expect("prepare counts");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query counts")
            .collect::<Result<_, _>>()
            .expect("collect counts")
    };
    assert_eq!(counts, vec![(1, 256), (2, 1)]);
}

#[test]
fn concurrent_generation_boundary_is_atomic_and_survives_reopen() {
    let directory = tempdir().expect("temporary directory");
    let database = directory.path().join("router-shadow.sqlite");
    {
        let mut connection = Connection::open(&database).expect("open database");
        let now = timestamp(DECIDED_AT);
        for index in 0..255 {
            insert_at(&mut connection, index + 1_000, now);
        }
    }
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|worker| {
            let database = database.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut connection = Connection::open(database).expect("open worker database");
                barrier.wait();
                insert_router_shadow_decision_at_for_tests(
                    &mut connection,
                    decision_draft(),
                    timestamp(DECIDED_AT),
                    uuid(10_000 + worker * 2),
                    uuid(10_001 + worker * 2),
                )
            })
        })
        .collect();
    barrier.wait();
    for handle in handles {
        handle
            .join()
            .expect("join worker")
            .expect("insert worker decision");
    }

    let connection = Connection::open(&database).expect("reopen database");
    let generations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM router_shadow_generations",
            [],
            |row| row.get(0),
        )
        .expect("count generations");
    let first: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM router_shadow_decisions WHERE generation_id = 1",
            [],
            |row| row.get(0),
        )
        .expect("count first generation");
    let second: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM router_shadow_decisions WHERE generation_id = 2",
            [],
            |row| row.get(0),
        )
        .expect("count second generation");
    assert_eq!((generations, first, second), (2, 256, 1));
}

#[test]
fn completion_can_target_an_older_generation() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let now = timestamp(DECIDED_AT);
    let first = insert_at(&mut connection, 1, now);
    for index in 1..257 {
        insert_at(&mut connection, index + 1, now);
    }
    assert_eq!(first.generation_id, 1);
    let completion = complete_at(&mut connection, &first, now + Duration::milliseconds(1))
        .expect("complete old generation");
    assert_eq!(completion.router_run_id, first.router_run_id);
}

#[test]
fn exactly_one_concurrent_completion_wins() {
    let directory = tempdir().expect("temporary directory");
    let database = directory.path().join("router-shadow.sqlite");
    let decision = {
        let mut connection = Connection::open(&database).expect("open database");
        insert_at(&mut connection, 1, timestamp(DECIDED_AT))
    };
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|worker| {
            let database = database.clone();
            let barrier = Arc::clone(&barrier);
            let run_id = decision.router_run_id.clone();
            thread::spawn(move || {
                let mut connection = Connection::open(database).expect("open worker database");
                barrier.wait();
                complete_router_shadow_run_at_for_tests(
                    &mut connection,
                    completion_draft(&run_id),
                    timestamp("2026-08-24T00:00:01.000Z"),
                    uuid(20_000 + worker),
                )
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("join worker"))
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(RouterShadowStoreError::DuplicateCompletion)))
            .count(),
        1
    );
}

#[test]
fn model_mismatch_persists_an_unverified_integrity_terminal() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let decision = insert_at(&mut connection, 1, timestamp(DECIDED_AT));
    let mut draft = completion_draft(&decision.router_run_id);
    draft.forwarded_model = model(CANDIDATE_MODEL_DIGEST);
    let completion = complete_router_shadow_run_at_for_tests(
        &mut connection,
        draft,
        timestamp("2026-08-24T00:00:01.000Z"),
        uuid(3),
    )
    .expect("persist integrity terminal");

    assert_eq!(completion.model_preserved, false);
    assert_eq!(
        completion.transport_outcome,
        RouterShadowOutcome::IntegrityFailure
    );
    assert_eq!(
        completion.failure_class,
        Some(RouterShadowFailureClass::Integrity)
    );
    assert!(!completion.integrity_verified());
    assert_eq!(
        load_router_shadow_completion(&connection, &decision.router_run_id)
            .expect("load completion"),
        Some(completion)
    );
}

#[test]
fn clock_rollback_is_an_explicit_unverified_terminal() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let decided_at = timestamp(DECIDED_AT);
    let decision = insert_at(&mut connection, 1, decided_at);
    let mut draft = completion_draft(&decision.router_run_id);
    draft.observed_monotonic_latency_ms = None;
    let completion = complete_router_shadow_run_at_for_tests(
        &mut connection,
        draft,
        decided_at - Duration::milliseconds(1),
        uuid(3),
    )
    .expect("persist clock anomaly");

    assert_eq!(
        completion.transport_outcome,
        RouterShadowOutcome::ClockAnomaly
    );
    assert_eq!(
        completion.clock_evidence,
        RouterShadowClockEvidence::ClockRollbackDetected
    );
    assert_eq!(completion.latency_ms, 0);
    assert_eq!(completion.upstream_started, false);
    assert!(!completion.integrity_verified());
}

#[test]
fn clock_rollback_and_model_mismatch_remain_explicit() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let decided_at = timestamp(DECIDED_AT);
    let decision = insert_at(&mut connection, 1, decided_at);
    let mut draft = completion_draft(&decision.router_run_id);
    draft.forwarded_model = model(CANDIDATE_MODEL_DIGEST);
    draft.observed_monotonic_latency_ms = None;
    let completion = complete_router_shadow_run_at_for_tests(
        &mut connection,
        draft,
        decided_at - Duration::milliseconds(1),
        uuid(3),
    )
    .expect("persist combined anomaly");
    assert_eq!(completion.model_preserved, false);
    assert_eq!(
        completion.transport_outcome,
        RouterShadowOutcome::ClockAnomaly
    );
    assert!(!completion.integrity_verified());
}

#[test]
fn completion_window_is_exclusive_at_twenty_four_hours() {
    let now = timestamp(DECIDED_AT);
    let mut accepted_connection = Connection::open_in_memory().expect("open database");
    let accepted = insert_at(&mut accepted_connection, 1, now);
    let mut accepted_draft = completion_draft(&accepted.router_run_id);
    accepted_draft.observed_monotonic_latency_ms = None;
    assert!(complete_router_shadow_run_at_for_tests(
        &mut accepted_connection,
        accepted_draft,
        now + Duration::hours(24) - Duration::milliseconds(1),
        uuid(3),
    )
    .is_ok());

    let mut expired_connection = Connection::open_in_memory().expect("open database");
    let expired = insert_at(&mut expired_connection, 2, now);
    let mut expired_draft = completion_draft(&expired.router_run_id);
    expired_draft.observed_monotonic_latency_ms = None;
    assert!(matches!(
        complete_router_shadow_run_at_for_tests(
            &mut expired_connection,
            expired_draft,
            now + Duration::hours(24),
            uuid(4),
        ),
        Err(RouterShadowStoreError::Expired)
    ));
    assert_eq!(
        load_router_shadow_completion(&expired_connection, &expired.router_run_id)
            .expect("load absent completion"),
        None
    );
}

#[test]
fn latency_uses_monotonic_evidence_or_bounded_wall_fallback() {
    let now = timestamp(DECIDED_AT);
    let mut monotonic_connection = Connection::open_in_memory().expect("open database");
    let monotonic = insert_at(&mut monotonic_connection, 1, now);
    let completion = complete_at(
        &mut monotonic_connection,
        &monotonic,
        now + Duration::seconds(10),
    )
    .expect("complete monotonic run");
    assert_eq!(completion.latency_ms, 1_250);
    assert_eq!(
        completion.clock_evidence,
        RouterShadowClockEvidence::MonotonicObserved
    );

    let mut wall_connection = Connection::open_in_memory().expect("open database");
    let wall = insert_at(&mut wall_connection, 2, now);
    let mut draft = completion_draft(&wall.router_run_id);
    draft.observed_monotonic_latency_ms = None;
    let completion = complete_router_shadow_run_at_for_tests(
        &mut wall_connection,
        draft,
        now + Duration::milliseconds(1_234),
        uuid(4),
    )
    .expect("complete wall-clock run");
    assert_eq!(completion.latency_ms, 1_234);
    assert_eq!(
        completion.clock_evidence,
        RouterShadowClockEvidence::WallClockObserved
    );
}

#[test]
fn corrupt_or_unknown_decision_fields_fail_closed() {
    for (column, replacement) in [
        ("source", "unknown_source"),
        ("canonical_digest", POLICY_DIGEST),
        (
            "requested_model_json",
            r#"{"provenance":"verified_endpoint_catalog","identityDigest":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","raw":"secret"}"#,
        ),
    ] {
        let mut connection = Connection::open_in_memory().expect("open database");
        let decision = insert_at(&mut connection, 1, timestamp(DECIDED_AT));
        connection
            .execute_batch("DROP TRIGGER router_shadow_decisions_no_update")
            .expect("drop test trigger");
        let statement =
            format!("UPDATE router_shadow_decisions SET {column} = ?1 WHERE router_run_id = ?2");
        connection
            .execute(&statement, params![replacement, decision.router_run_id])
            .expect("corrupt decision");
        assert!(load_router_shadow_decision(&connection, &decision.router_run_id).is_err());
    }
}

#[test]
fn corrupt_completion_binding_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let decision = insert_at(&mut connection, 1, timestamp(DECIDED_AT));
    complete_at(
        &mut connection,
        &decision,
        timestamp("2026-08-24T00:00:01.000Z"),
    )
    .expect("complete decision");
    connection
        .execute_batch("DROP TRIGGER router_shadow_completions_no_update")
        .expect("drop test trigger");
    connection
        .execute(
            "UPDATE router_shadow_completions SET decision_digest = ?1 WHERE router_run_id = ?2",
            params![POLICY_DIGEST, decision.router_run_id],
        )
        .expect("corrupt completion");
    assert!(load_router_shadow_completion(&connection, &decision.router_run_id).is_err());
}

#[test]
fn every_router_shadow_table_is_append_only() {
    let mut connection = Connection::open_in_memory().expect("open database");
    let decision = insert_at(&mut connection, 1, timestamp(DECIDED_AT));
    complete_at(
        &mut connection,
        &decision,
        timestamp("2026-08-24T00:00:01.000Z"),
    )
    .expect("complete decision");

    for statement in [
        "UPDATE router_shadow_generations SET created_at = created_at WHERE generation_id = 1",
        "DELETE FROM router_shadow_generations WHERE generation_id = 1",
        "UPDATE router_shadow_decisions SET decided_at = decided_at",
        "DELETE FROM router_shadow_decisions",
        "UPDATE router_shadow_completions SET completed_at = completed_at",
        "DELETE FROM router_shadow_completions",
    ] {
        assert!(connection.execute(statement, []).is_err(), "{statement}");
    }
}

#[test]
fn incomplete_decision_survives_database_reopen() {
    let directory = tempdir().expect("temporary directory");
    let database = directory.path().join("router-shadow.sqlite");
    let decision = {
        let mut connection = Connection::open(&database).expect("open database");
        insert_at(&mut connection, 1, timestamp(DECIDED_AT))
    };
    let connection = Connection::open(&database).expect("reopen database");
    assert_eq!(
        load_router_shadow_decision(&connection, &decision.router_run_id).expect("load decision"),
        decision
    );
    assert_eq!(
        load_router_shadow_completion(&connection, &decision.router_run_id)
            .expect("load completion"),
        None
    );
    assert!(fs::metadata(database).expect("database metadata").len() > 0);
}

#[test]
fn invalid_completion_invariants_are_rejected_without_a_partial_row() {
    let cases = [
        (false, true, Some(200), RouterShadowOutcome::Completed, None),
        (true, false, Some(200), RouterShadowOutcome::Completed, None),
        (
            false,
            false,
            None,
            RouterShadowOutcome::HttpError,
            Some(RouterShadowFailureClass::HttpStatus),
        ),
    ];
    for (headers, delivery, status, outcome, failure) in cases {
        let mut connection = Connection::open_in_memory().expect("open database");
        let decision = insert_at(&mut connection, 1, timestamp(DECIDED_AT));
        let mut draft = completion_draft(&decision.router_run_id);
        draft.response_headers_received = headers;
        draft.delivery_completed = delivery;
        draft.status_code = status;
        draft.transport_outcome = outcome;
        draft.failure_class = failure;
        assert!(complete_router_shadow_run_at_for_tests(
            &mut connection,
            draft,
            timestamp("2026-08-24T00:00:01.000Z"),
            uuid(3),
        )
        .is_err());
        assert_eq!(
            load_router_shadow_completion(&connection, &decision.router_run_id)
                .expect("load absent completion"),
            None
        );
    }
}

#[test]
fn decision_semantics_require_verified_models_and_complete_proposals() {
    let now = timestamp(DECIDED_AT);

    let mut unverified_connection = Connection::open_in_memory().expect("open database");
    let mut unverified = decision_draft();
    unverified.requested_model.provenance =
        RouterShadowModelProvenance::ProviderResponseCatalogMatch;
    assert!(insert_router_shadow_decision_at_for_tests(
        &mut unverified_connection,
        unverified,
        now,
        uuid(1),
        uuid(2),
    )
    .is_err());

    let mut incomplete_connection = Connection::open_in_memory().expect("open database");
    let mut incomplete = decision_draft();
    incomplete.proposed_endpoint_identity_digest = None;
    assert!(insert_router_shadow_decision_at_for_tests(
        &mut incomplete_connection,
        incomplete,
        now,
        uuid(3),
        uuid(4),
    )
    .is_err());

    let mut classification_connection = Connection::open_in_memory().expect("open database");
    let mut classification = decision_draft();
    classification.task_class = RouterShadowTaskClass::Unclassified;
    assert!(insert_router_shadow_decision_at_for_tests(
        &mut classification_connection,
        classification,
        now,
        uuid(5),
        uuid(6),
    )
    .is_err());
}

#[test]
fn completion_rejects_transport_provenance_and_latency_drift() {
    let now = timestamp(DECIDED_AT);

    let mut transport_connection = Connection::open_in_memory().expect("open database");
    let transport_decision = insert_at(&mut transport_connection, 1, now);
    let mut wrong_transport = completion_draft(&transport_decision.router_run_id);
    wrong_transport.actual_transport = RouterShadowTransport::DirectAnthropic;
    assert!(complete_router_shadow_run_at_for_tests(
        &mut transport_connection,
        wrong_transport,
        now + Duration::seconds(1),
        uuid(3),
    )
    .is_err());

    let mut provenance_connection = Connection::open_in_memory().expect("open database");
    let provenance_decision = insert_at(&mut provenance_connection, 2, now);
    let mut wrong_provenance = completion_draft(&provenance_decision.router_run_id);
    wrong_provenance
        .provider_reported_model
        .as_mut()
        .expect("provider model")
        .provenance = RouterShadowModelProvenance::VerifiedEndpointCatalog;
    assert!(complete_router_shadow_run_at_for_tests(
        &mut provenance_connection,
        wrong_provenance,
        now + Duration::seconds(1),
        uuid(4),
    )
    .is_err());

    let mut latency_connection = Connection::open_in_memory().expect("open database");
    let latency_decision = insert_at(&mut latency_connection, 3, now);
    let mut excessive_latency = completion_draft(&latency_decision.router_run_id);
    excessive_latency.observed_monotonic_latency_ms = Some(24 * 60 * 60 * 1_000);
    assert!(complete_router_shadow_run_at_for_tests(
        &mut latency_connection,
        excessive_latency,
        now + Duration::seconds(1),
        uuid(5),
    )
    .is_err());
}

#[test]
fn monotonic_latency_survives_backward_and_forward_wall_clock_jumps() {
    let now = timestamp(DECIDED_AT);

    let mut backward_connection = Connection::open_in_memory().expect("open database");
    let backward_decision = insert_at(&mut backward_connection, 1, now);
    let backward = complete_router_shadow_run_at_for_tests(
        &mut backward_connection,
        completion_draft(&backward_decision.router_run_id),
        now - Duration::hours(2),
        uuid(3),
    )
    .expect("complete across backward wall jump");
    assert_eq!(backward.transport_outcome, RouterShadowOutcome::Completed);
    assert_eq!(backward.latency_ms, 1_250);
    assert_eq!(
        backward.clock_evidence,
        RouterShadowClockEvidence::MonotonicObserved
    );

    let mut forward_connection = Connection::open_in_memory().expect("open database");
    let forward_decision = insert_at(&mut forward_connection, 2, now);
    let forward = complete_router_shadow_run_at_for_tests(
        &mut forward_connection,
        completion_draft(&forward_decision.router_run_id),
        now + Duration::hours(48),
        uuid(4),
    )
    .expect("complete across forward wall jump");
    assert_eq!(forward.transport_outcome, RouterShadowOutcome::Completed);
    assert_eq!(forward.latency_ms, 1_250);
    assert_eq!(
        forward.clock_evidence,
        RouterShadowClockEvidence::MonotonicObserved
    );
}

#[test]
fn delivered_redirect_statuses_are_non_success_http_terminals() {
    for (index, status) in [301, 307, 308].into_iter().enumerate() {
        let mut connection = Connection::open_in_memory().expect("open database");
        let now = timestamp(DECIDED_AT);
        let decision = insert_at(&mut connection, index as u128 + 1, now);
        let mut redirect = completion_draft(&decision.router_run_id);
        redirect.status_code = Some(status);
        redirect.delivery_completed = true;
        redirect.transport_outcome = RouterShadowOutcome::HttpError;
        redirect.failure_class = Some(RouterShadowFailureClass::HttpStatus);
        let completion = complete_router_shadow_run_at_for_tests(
            &mut connection,
            redirect,
            now + Duration::seconds(1),
            uuid(index as u128 + 100),
        )
        .expect("persist redirect terminal");
        assert_eq!(completion.status_code, Some(status));
        assert_eq!(completion.transport_outcome, RouterShadowOutcome::HttpError);
        assert!(completion.integrity_verified());
    }
}

#[test]
fn generation_corruption_or_orphaning_fails_closed() {
    let now = timestamp(DECIDED_AT);

    let mut corrupt_connection = Connection::open_in_memory().expect("open database");
    let corrupt_decision = insert_at(&mut corrupt_connection, 1, now);
    corrupt_connection
        .execute_batch("DROP TRIGGER router_shadow_generations_no_update")
        .expect("drop generation update trigger");
    corrupt_connection
        .execute(
            "UPDATE router_shadow_generations SET created_at = ?1 WHERE generation_id = 1",
            params!["2026-08-24T00:00:01.000Z"],
        )
        .expect("corrupt generation timestamp");
    assert!(
        load_router_shadow_decision(&corrupt_connection, &corrupt_decision.router_run_id).is_err()
    );
    assert!(insert_router_shadow_decision_at_for_tests(
        &mut corrupt_connection,
        decision_draft(),
        now + Duration::seconds(2),
        uuid(20),
        uuid(21),
    )
    .is_err());
    assert!(
        load_router_shadow_decision(&corrupt_connection, &corrupt_decision.router_run_id).is_err()
    );

    let mut orphan_connection = Connection::open_in_memory().expect("open database");
    let orphan_decision = insert_at(&mut orphan_connection, 2, now);
    orphan_connection
        .execute_batch(
            "DROP TRIGGER router_shadow_generations_no_delete; PRAGMA foreign_keys = OFF;",
        )
        .expect("disable test-only generation protections");
    orphan_connection
        .execute(
            "DELETE FROM router_shadow_generations WHERE generation_id = 1",
            [],
        )
        .expect("orphan decision");
    assert!(
        load_router_shadow_decision(&orphan_connection, &orphan_decision.router_run_id).is_err()
    );
    assert!(insert_router_shadow_decision_at_for_tests(
        &mut orphan_connection,
        decision_draft(),
        now + Duration::seconds(2),
        uuid(22),
        uuid(23),
    )
    .is_err());
    assert!(
        load_router_shadow_decision(&orphan_connection, &orphan_decision.router_run_id).is_err()
    );
}
