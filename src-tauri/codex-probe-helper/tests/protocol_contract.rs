use codex_probe_helper::{
    decode_preparation_request, decode_preparation_response, encode_preparation_request,
    encode_preparation_response, preparation_request_from_host,
    prepare_shape_consistent_non_executing_response, HostPreparationProjection, ProtocolError,
    FRAME_PREFIX_BYTES, MAX_FRAME_BYTES, MAX_IDENTIFIER_BYTES, MAX_PAYLOAD_BYTES,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const D0: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const D1: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const D2: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const D3: &str = "sha256:3333333333333333333333333333333333333333333333333333333333333333";
const D4: &str = "sha256:4444444444444444444444444444444444444444444444444444444444444444";
const D5: &str = "sha256:5555555555555555555555555555555555555555555555555555555555555555";
const D6: &str = "sha256:6666666666666666666666666666666666666666666666666666666666666666";
const D7: &str = "sha256:7777777777777777777777777777777777777777777777777777777777777777";
const D8: &str = "sha256:8888888888888888888888888888888888888888888888888888888888888888";
const D9: &str = "sha256:9999999999999999999999999999999999999999999999999999999999999999";
const DA: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DB: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DC: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DD: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const DE: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const DF: &str = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const PREPARED_AT: &str = "2026-08-24T03:40:00+00:00";

fn bounded_digest(domain: &[u8], values: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for value in values {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

struct HostFixture {
    binding_digest: String,
    request_id: String,
    request_digest: String,
    receipt_id: String,
    receipt_digest: String,
}

impl HostFixture {
    fn new() -> Self {
        let binding_digest = bounded_digest(
            b"ai-switchboard-codex-helper-launch-binding-v1\0",
            &[
                "session-1",
                D0,
                D1,
                "plan-1",
                D2,
                "process-run-1",
                D3,
                "grant-1",
                D4,
                "admission-1",
                D5,
                "attempt-1",
                D6,
                D7,
                D8,
                "macos-restricted-helper-v1",
                D9,
                DA,
                DB,
                "codex-version-probe-v1",
                "separately-signed-nested-helper-required",
                "collected-npm-schema-v2",
                "2",
                DC,
                "candidate-1",
                DD,
                DF,
                DE,
            ],
        );
        let request_id = format!(
            "codex-helper-request:{}",
            binding_digest.trim_start_matches("sha256:")
        );
        let request_digest = bounded_digest(
            b"ai-switchboard-codex-helper-launch-request-v1\0",
            &[
                "1",
                request_id.as_str(),
                binding_digest.as_str(),
                "prepared_no_process",
                "10000",
                "none",
            ],
        );
        let receipt_identity = bounded_digest(
            b"ai-switchboard-codex-helper-launch-preparation-receipt-id-v1\0",
            &[request_id.as_str(), request_digest.as_str(), PREPARED_AT],
        );
        let receipt_id = format!(
            "codex-helper-receipt:{}",
            receipt_identity.trim_start_matches("sha256:")
        );
        let receipt_digest = bounded_digest(
            b"ai-switchboard-codex-helper-launch-preparation-receipt-v1\0",
            &[
                "1",
                receipt_id.as_str(),
                request_id.as_str(),
                request_digest.as_str(),
                PREPARED_AT,
                "validated_no_process",
                "000000",
                "none",
            ],
        );
        Self {
            binding_digest,
            request_id,
            request_digest,
            receipt_id,
            receipt_digest,
        }
    }

    fn projection(&self) -> HostPreparationProjection<'_> {
        HostPreparationProjection {
            request_id: &self.request_id,
            request_digest: &self.request_digest,
            binding_digest: &self.binding_digest,
            preparation_receipt_id: &self.receipt_id,
            preparation_receipt_digest: &self.receipt_digest,
            prepared_at: PREPARED_AT,
            session_id: "session-1",
            session_snapshot_digest: D0,
            workspace_digest: D1,
            plan_id: "plan-1",
            plan_snapshot_digest: D2,
            process_run_id: "process-run-1",
            process_run_spec_digest: D3,
            grant_id: "grant-1",
            grant_receipt_digest: D4,
            admission_id: "admission-1",
            admission_receipt_digest: D5,
            attempt_id: "attempt-1",
            probe_plan_digest: DC,
            candidate_id: "candidate-1",
            launcher_chain_identity_digest: DD,
            preflight_identity_digest: DE,
            containment_identity_digest: DF,
            host_instance_identity_digest: D6,
            boot_session_identity_digest: D7,
            os_build_identity_digest: D8,
            helper_code_identity_digest: D9,
            helper_entitlements_identity_digest: DA,
            enforcement_policy_identity_digest: DB,
        }
    }
}

fn sample_request() -> codex_probe_helper::PreparationRequestFrame {
    let fixture = HostFixture::new();
    preparation_request_from_host(fixture.projection()).expect("consistent host transcript")
}

fn sample_response() -> codex_probe_helper::PreparationResponseFrame {
    prepare_shape_consistent_non_executing_response(&sample_request()).expect("shape response")
}

fn frame_payload(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(FRAME_PREFIX_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn payload_string(frame: &[u8]) -> String {
    std::str::from_utf8(&frame[FRAME_PREFIX_BYTES..])
        .expect("UTF-8 test frame")
        .to_owned()
}

fn json_from_frame(frame: &[u8]) -> Value {
    serde_json::from_slice(&frame[FRAME_PREFIX_BYTES..]).expect("valid framed JSON")
}

fn frame_from_json(value: &Value) -> Vec<u8> {
    frame_payload(&serde_json::to_vec(value).expect("serializable test JSON"))
}

fn replace_in_frame(frame: &[u8], from: &str, to: &str) -> Vec<u8> {
    let payload = payload_string(frame);
    let replaced = payload.replacen(from, to, 1);
    assert_ne!(payload, replaced, "test replacement must match");
    frame_payload(replaced.as_bytes())
}

fn append_unknown_field(frame: &[u8], field: &str) -> Vec<u8> {
    let mut payload = payload_string(frame);
    assert_eq!(payload.pop(), Some('}'));
    payload.push_str(&format!(",\"{field}\":\"forbidden\"}}"));
    frame_payload(payload.as_bytes())
}

#[test]
fn deterministic_round_trip_stays_shape_only_with_headroom() {
    let request = sample_request();
    let first_request_frame = encode_preparation_request(&request).expect("request frame");
    let second_request_frame = encode_preparation_request(&request).expect("request frame");
    assert_eq!(first_request_frame, second_request_frame);
    assert_eq!(
        decode_preparation_request(&first_request_frame).expect("decoded request"),
        request
    );

    let response = sample_response();
    let first_response_frame = encode_preparation_response(&response).expect("response frame");
    let second_response_frame = encode_preparation_response(&response).expect("response frame");
    assert_eq!(first_response_frame, second_response_frame);
    assert_eq!(
        decode_preparation_response(&first_response_frame).expect("decoded response"),
        response
    );

    assert!(first_request_frame.len() <= MAX_FRAME_BYTES - 1_024);
    assert!(first_response_frame.len() <= MAX_FRAME_BYTES - 1_024);
    assert!(request.manual_opt_in_required);
    assert!(!request.runnable);
    assert!(!request.supported);
    assert!(!request.process_start_enabled);
    assert!(!request.execution_reserved);
    assert!(!request.user_workspace_writes_enabled);
    assert!(!response.host_authenticated);
    assert!(!response.freshness_verified);
    assert!(!response.launch_authority_verified);
    assert!(!response.helper_invoked);
    assert!(!response.process_started);
    assert!(!response.execution_reserved);
    assert!(!response.runnable);
    assert!(!response.supported);
    assert!(!response.user_workspace_writes_enabled);
}

#[test]
fn fabricated_but_self_consistent_transcript_is_shape_valid_not_authenticated() {
    let request = sample_request();
    request.validate_shape().expect("self-consistent shape");
    let response = prepare_shape_consistent_non_executing_response(&request).expect("response");
    assert!(!response.host_authenticated);
    assert!(!response.freshness_verified);
    assert!(!response.launch_authority_verified);
}

#[test]
fn framing_rejects_short_zero_oversize_truncation_trailing_and_invalid_utf8() {
    assert_eq!(
        decode_preparation_request(&[0, 0, 0]),
        Err(ProtocolError::FrameTooShort)
    );
    assert_eq!(
        decode_preparation_request(&[0, 0, 0, 0]),
        Err(ProtocolError::FrameLengthZero)
    );
    let oversized_prefix = ((MAX_PAYLOAD_BYTES + 1) as u32).to_be_bytes();
    assert_eq!(
        decode_preparation_request(&oversized_prefix),
        Err(ProtocolError::FrameLengthTooLarge)
    );
    let mut truncated = encode_preparation_request(&sample_request()).expect("request frame");
    truncated.pop();
    assert_eq!(
        decode_preparation_request(&truncated),
        Err(ProtocolError::FrameLengthMismatch)
    );
    let mut trailing = encode_preparation_request(&sample_request()).expect("request frame");
    trailing.push(b' ');
    assert_eq!(
        decode_preparation_request(&trailing),
        Err(ProtocolError::FrameLengthMismatch)
    );
    assert_eq!(
        decode_preparation_request(&frame_payload(&[0xff])),
        Err(ProtocolError::InvalidUtf8)
    );
}

#[test]
fn maximum_payload_boundary_is_bounded_before_json_parsing() {
    let maximum = frame_payload(&vec![b' '; MAX_PAYLOAD_BYTES]);
    assert_eq!(maximum.len(), MAX_FRAME_BYTES);
    assert_eq!(
        decode_preparation_request(&maximum),
        Err(ProtocolError::JsonRejected)
    );
    let over = frame_payload(&vec![b' '; MAX_PAYLOAD_BYTES + 1]);
    assert_eq!(
        decode_preparation_request(&over),
        Err(ProtocolError::FrameLengthTooLarge)
    );
}

#[test]
fn request_schema_rejects_malformed_missing_duplicate_null_wrong_and_unknown_fields() {
    let frame = encode_preparation_request(&sample_request()).expect("request frame");
    assert_eq!(
        decode_preparation_request(&frame_payload(b"{")),
        Err(ProtocolError::JsonRejected)
    );

    let mut missing = json_from_frame(&frame);
    missing.as_object_mut().expect("object").remove("requestId");
    assert_eq!(
        decode_preparation_request(&frame_from_json(&missing)),
        Err(ProtocolError::JsonRejected)
    );
    let mut null = json_from_frame(&frame);
    null["requestId"] = Value::Null;
    assert_eq!(
        decode_preparation_request(&frame_from_json(&null)),
        Err(ProtocolError::JsonRejected)
    );
    let mut wrong = json_from_frame(&frame);
    wrong["requestId"] = Value::Bool(false);
    assert_eq!(
        decode_preparation_request(&frame_from_json(&wrong)),
        Err(ProtocolError::JsonRejected)
    );
    assert_eq!(
        decode_preparation_request(&replace_in_frame(
            &frame,
            "\"requestId\":",
            "\"requestId\":\"duplicate\",\"requestId\":"
        )),
        Err(ProtocolError::JsonRejected)
    );

    for forbidden in [
        "path",
        "executable",
        "command",
        "argument",
        "arguments",
        "argv",
        "env",
        "environment",
        "stdin",
        "shell",
        "cwd",
        "workingDirectory",
        "prompt",
        "credential",
        "headers",
        "pid",
        "pgid",
        "stdout",
        "stderr",
        "output",
    ] {
        assert_eq!(
            decode_preparation_request(&append_unknown_field(&frame, forbidden)),
            Err(ProtocolError::JsonRejected),
            "accepted forbidden field {forbidden}"
        );
    }
}

#[test]
fn response_schema_is_equally_closed() {
    let frame = encode_preparation_response(&sample_response()).expect("response frame");
    let mut missing = json_from_frame(&frame);
    missing.as_object_mut().expect("object").remove("state");
    assert_eq!(
        decode_preparation_response(&frame_from_json(&missing)),
        Err(ProtocolError::JsonRejected)
    );
    let mut null = json_from_frame(&frame);
    null["state"] = Value::Null;
    assert_eq!(
        decode_preparation_response(&frame_from_json(&null)),
        Err(ProtocolError::JsonRejected)
    );
    let mut wrong = json_from_frame(&frame);
    wrong["hostAuthenticated"] = Value::String("false".into());
    assert_eq!(
        decode_preparation_response(&frame_from_json(&wrong)),
        Err(ProtocolError::JsonRejected)
    );
    assert_eq!(
        decode_preparation_response(&replace_in_frame(
            &frame,
            "\"state\":",
            "\"state\":\"shape_consistent_no_process\",\"state\":"
        )),
        Err(ProtocolError::JsonRejected)
    );
    assert_eq!(
        decode_preparation_response(&append_unknown_field(&frame, "output")),
        Err(ProtocolError::JsonRejected)
    );
}

#[test]
fn canonical_wire_rejects_reordered_whitespace_and_alternative_escape_forms() {
    let frame = encode_preparation_request(&sample_request()).expect("request frame");
    let reordered = frame_from_json(&json_from_frame(&frame));
    assert_ne!(reordered, frame);
    assert_eq!(
        decode_preparation_request(&reordered),
        Err(ProtocolError::NonCanonicalJson)
    );

    let mut whitespace = payload_string(&frame);
    whitespace.push(' ');
    assert_eq!(
        decode_preparation_request(&frame_payload(whitespace.as_bytes())),
        Err(ProtocolError::NonCanonicalJson)
    );

    let escaped = replace_in_frame(
        &frame,
        "codex-helper-request:",
        "\\u0063odex-helper-request:",
    );
    assert_eq!(
        decode_preparation_request(&escaped),
        Err(ProtocolError::NonCanonicalJson)
    );
}

#[test]
fn fixed_protocol_discriminants_and_version_are_closed() {
    let frame = encode_preparation_request(&sample_request()).expect("request frame");
    assert_eq!(
        decode_preparation_request(&replace_in_frame(
            &frame,
            "\"protocolVersion\":1",
            "\"protocolVersion\":2"
        )),
        Err(ProtocolError::UnsupportedProtocolVersion)
    );
    for (from, to) in [
        ("prepare_no_process", "execute"),
        ("codex_version_probe_v1", "arbitrary_command"),
        ("preparation_only", "launch"),
        ("separately_signed_nested_helper_required", "in_process"),
        ("collected_npm_schema_v2", "direct"),
        ("macos_restricted_helper_v1", "unrestricted"),
    ] {
        assert_eq!(
            decode_preparation_request(&replace_in_frame(&frame, from, to)),
            Err(ProtocolError::JsonRejected),
            "accepted open discriminant {to}"
        );
    }
}

#[test]
fn identifiers_digests_and_prepared_at_are_strictly_bounded() {
    let mut empty = sample_request();
    empty.session_id.clear();
    assert_eq!(
        empty.validate_shape(),
        Err(ProtocolError::InvalidIdentifier)
    );
    let mut oversized = sample_request();
    oversized.candidate_id = "a".repeat(MAX_IDENTIFIER_BYTES + 1);
    assert_eq!(
        oversized.validate_shape(),
        Err(ProtocolError::InvalidIdentifier)
    );
    for invalid in [" leading", "trailing ", "contains/slash", "non-ascii-é"] {
        let mut request = sample_request();
        request.attempt_id = invalid.into();
        assert_eq!(
            request.validate_shape(),
            Err(ProtocolError::InvalidIdentifier),
            "accepted identifier {invalid:?}"
        );
    }
    for invalid in [
        "",
        "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "md5:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sha256:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
    ] {
        let mut request = sample_request();
        request.workspace_digest = invalid.into();
        assert_eq!(
            request.validate_shape(),
            Err(ProtocolError::InvalidDigest),
            "accepted digest {invalid:?}"
        );
    }
    let mut timestamp = sample_request();
    timestamp.prepared_at = "not-a-time".into();
    assert_eq!(
        timestamp.validate_shape(),
        Err(ProtocolError::InvalidPreparedAt)
    );
}

#[test]
fn every_upstream_binding_field_is_recomputed_or_rejected() {
    macro_rules! assert_identifier_tamper {
        ($field:ident) => {{
            let mut request = sample_request();
            request.$field = "different-valid-id".into();
            assert_eq!(
                request.validate_shape(),
                Err(ProtocolError::HostTranscriptMismatch),
                "unverified identifier {}",
                stringify!($field)
            );
        }};
    }
    macro_rules! assert_digest_tamper {
        ($field:ident) => {{
            let mut request = sample_request();
            request.$field = if request.$field == D0 { D1 } else { D0 }.into();
            assert_eq!(
                request.validate_shape(),
                Err(ProtocolError::HostTranscriptMismatch),
                "unverified digest {}",
                stringify!($field)
            );
        }};
    }
    assert_identifier_tamper!(session_id);
    assert_identifier_tamper!(plan_id);
    assert_identifier_tamper!(process_run_id);
    assert_identifier_tamper!(grant_id);
    assert_identifier_tamper!(admission_id);
    assert_identifier_tamper!(attempt_id);
    assert_identifier_tamper!(candidate_id);
    assert_digest_tamper!(session_snapshot_digest);
    assert_digest_tamper!(workspace_digest);
    assert_digest_tamper!(plan_snapshot_digest);
    assert_digest_tamper!(process_run_spec_digest);
    assert_digest_tamper!(grant_receipt_digest);
    assert_digest_tamper!(admission_receipt_digest);
    assert_digest_tamper!(host_instance_identity_digest);
    assert_digest_tamper!(boot_session_identity_digest);
    assert_digest_tamper!(os_build_identity_digest);
    assert_digest_tamper!(helper_code_identity_digest);
    assert_digest_tamper!(helper_entitlements_identity_digest);
    assert_digest_tamper!(enforcement_policy_identity_digest);
    assert_digest_tamper!(probe_plan_digest);
    assert_digest_tamper!(launcher_chain_identity_digest);
    assert_digest_tamper!(containment_identity_digest);
    assert_digest_tamper!(preflight_identity_digest);

    for mutate in [
        |request: &mut codex_probe_helper::PreparationRequestFrame| {
            request.binding_digest = D0.into()
        },
        |request: &mut codex_probe_helper::PreparationRequestFrame| {
            request.request_id = "different-valid-id".into()
        },
        |request: &mut codex_probe_helper::PreparationRequestFrame| {
            request.request_digest = D0.into()
        },
        |request: &mut codex_probe_helper::PreparationRequestFrame| {
            request.preparation_receipt_id = "different-valid-id".into()
        },
        |request: &mut codex_probe_helper::PreparationRequestFrame| {
            request.preparation_receipt_digest = D0.into()
        },
        |request: &mut codex_probe_helper::PreparationRequestFrame| {
            request.prepared_at = "2026-08-24T03:41:00+00:00".into()
        },
    ] {
        let mut request = sample_request();
        mutate(&mut request);
        assert_eq!(
            request.validate_shape(),
            Err(ProtocolError::HostTranscriptMismatch)
        );
    }
}

#[test]
fn request_policy_flags_and_schema_drift_fail_closed() {
    macro_rules! assert_flag_rejected {
        ($field:ident, $value:expr) => {{
            let mut request = sample_request();
            request.$field = $value;
            assert_eq!(
                request.validate_shape(),
                Err(ProtocolError::FixedPolicyViolation),
                "accepted policy flag {}",
                stringify!($field)
            );
        }};
    }
    assert_flag_rejected!(manual_opt_in_required, false);
    assert_flag_rejected!(runnable, true);
    assert_flag_rejected!(supported, true);
    assert_flag_rejected!(process_start_enabled, true);
    assert_flag_rejected!(execution_reserved, true);
    assert_flag_rejected!(user_workspace_writes_enabled, true);

    let mut host_schema = sample_request();
    host_schema.host_contract_schema_version += 1;
    assert_eq!(
        host_schema.validate_shape(),
        Err(ProtocolError::FixedPolicyViolation)
    );
    let mut collection_schema = sample_request();
    collection_schema.collection_receipt_schema_version += 1;
    assert_eq!(
        collection_schema.validate_shape(),
        Err(ProtocolError::FixedPolicyViolation)
    );
}

#[test]
fn outer_frame_and_response_bindings_are_tamper_evident() {
    let mut request = sample_request();
    request.frame_digest = D0.into();
    assert_eq!(
        request.validate_shape(),
        Err(ProtocolError::FrameDigestMismatch)
    );

    let response = sample_response();
    macro_rules! assert_response_tamper {
        ($field:ident, $value:expr) => {{
            let mut tampered = response.clone();
            tampered.$field = $value.into();
            assert_eq!(
                tampered.validate_shape(),
                Err(ProtocolError::FrameDigestMismatch),
                "unbound response field {}",
                stringify!($field)
            );
        }};
    }
    assert_response_tamper!(request_frame_digest, D0);
    assert_response_tamper!(request_id, "different-request");
    assert_response_tamper!(request_digest, D1);
    assert_response_tamper!(binding_digest, D2);
    assert_response_tamper!(preparation_receipt_id, "different-preparation");
    assert_response_tamper!(preparation_receipt_digest, D3);
    assert_response_tamper!(attempt_id, "different-attempt");
}

#[test]
fn every_response_authority_or_execution_flag_is_permanently_false() {
    let response = sample_response();
    macro_rules! assert_response_flag_rejected {
        ($field:ident) => {{
            let mut tampered = response.clone();
            tampered.$field = true;
            assert_eq!(
                tampered.validate_shape(),
                Err(ProtocolError::FixedPolicyViolation),
                "accepted true response flag {}",
                stringify!($field)
            );
        }};
    }
    assert_response_flag_rejected!(host_authenticated);
    assert_response_flag_rejected!(freshness_verified);
    assert_response_flag_rejected!(launch_authority_verified);
    assert_response_flag_rejected!(helper_invoked);
    assert_response_flag_rejected!(process_started);
    assert_response_flag_rejected!(execution_reserved);
    assert_response_flag_rejected!(runnable);
    assert_response_flag_rejected!(supported);
    assert_response_flag_rejected!(user_workspace_writes_enabled);
}

#[test]
fn concatenated_frames_are_never_accepted_as_one_message() {
    let frame = encode_preparation_request(&sample_request()).expect("request frame");
    let concatenated = [frame.as_slice(), frame.as_slice()].concat();
    assert_eq!(
        decode_preparation_request(&concatenated),
        Err(ProtocolError::FrameLengthMismatch)
    );
}
