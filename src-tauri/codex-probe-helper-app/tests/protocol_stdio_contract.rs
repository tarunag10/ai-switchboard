mod support;

use std::process::{Command, Output};
use std::time::Duration;

use codex_probe_helper::{
    decode_preparation_response, encode_preparation_request, preparation_request_from_host,
    prepare_shape_consistent_non_executing_response, HostPreparationProjection, FRAME_PREFIX_BYTES,
    MAX_FRAME_BYTES,
};
use sha2::{Digest, Sha256};
use support::run_command;

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
const FIXED_ERROR: &str = "Error: HelperFailure\n";

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

fn valid_frame() -> Vec<u8> {
    let fixture = HostFixture::new();
    let request = preparation_request_from_host(fixture.projection()).expect("valid fixture");
    encode_preparation_request(&request).expect("encoded request")
}

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ai-switchboard-codex-probe"))
}

fn run(input: &[u8]) -> Output {
    run_command(&mut command(), Some(input), Duration::ZERO)
}

fn assert_fixed_rejection(input: &[u8]) {
    let output = run(input);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "error must not produce stdout");
    assert_eq!(
        String::from_utf8(output.stderr).expect("UTF-8 stderr"),
        FIXED_ERROR
    );
}

fn raw_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(FRAME_PREFIX_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

#[test]
fn one_valid_closed_frame_returns_the_existing_no_process_response() {
    let fixture = HostFixture::new();
    let request = preparation_request_from_host(fixture.projection()).expect("valid fixture");
    let frame = encode_preparation_request(&request).expect("encoded request");
    let output = run(&frame);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let actual = decode_preparation_response(&output.stdout).expect("valid response");
    let expected =
        prepare_shape_consistent_non_executing_response(&request).expect("expected response");
    assert_eq!(actual, expected);
    assert!(!actual.host_authenticated);
    assert!(!actual.freshness_verified);
    assert!(!actual.launch_authority_verified);
    assert!(!actual.helper_invoked);
    assert!(!actual.process_started);
    assert!(!actual.execution_reserved);
    assert!(!actual.runnable);
    assert!(!actual.supported);
    assert!(!actual.user_workspace_writes_enabled);
}

#[test]
fn helper_does_not_exit_until_the_host_closes_stdin() {
    let frame = valid_frame();
    let close_delay = Duration::from_millis(200);
    let output = run_command(&mut command(), Some(&frame), close_delay);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    decode_preparation_response(&output.stdout).expect("valid response");
}

#[test]
fn empty_partial_zero_oversized_and_truncated_frames_are_rejected() {
    assert_fixed_rejection(&[]);
    assert_fixed_rejection(&[0, 0]);
    assert_fixed_rejection(&[0, 0, 0, 0]);
    assert_fixed_rejection(&((MAX_FRAME_BYTES - FRAME_PREFIX_BYTES + 1) as u32).to_be_bytes());

    let mut truncated = valid_frame();
    truncated.pop();
    assert_fixed_rejection(&truncated);
}

#[test]
fn trailing_and_concatenated_frames_are_rejected() {
    let frame = valid_frame();
    let mut trailing = frame.clone();
    trailing.push(b'x');
    assert_fixed_rejection(&trailing);

    let concatenated = [frame.as_slice(), frame.as_slice()].concat();
    assert_fixed_rejection(&concatenated);
}

#[test]
fn malformed_protocol_data_is_rejected_without_reflection_or_leakage() {
    assert_fixed_rejection(&raw_frame(b"{"));

    let sentinel = b"PRIVATE_PROMPT_TOKEN_do_not_echo";
    let output = run(&raw_frame(sentinel));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&output.stderr), FIXED_ERROR);
    assert!(!output
        .stderr
        .windows(sentinel.len())
        .any(|window| window == sentinel));
}

#[test]
fn arguments_do_not_create_an_input_or_control_surface() {
    let frame = valid_frame();
    let mut command = command();
    command.arg("--version").arg("PRIVATE_ARGUMENT_do_not_read");
    let output = run_command(&mut command, Some(&frame), Duration::ZERO);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    decode_preparation_response(&output.stdout).expect("valid response");
}

#[test]
fn inherited_environment_values_do_not_create_a_protocol_surface() {
    let fixture = HostFixture::new();
    let request = preparation_request_from_host(fixture.projection()).expect("valid fixture");
    let frame = encode_preparation_request(&request).expect("encoded request");
    let mut command = command();
    command
        .env("OPENAI_API_KEY", "PRIVATE_OPENAI_KEY_do_not_read")
        .env("ANTHROPIC_API_KEY", "PRIVATE_ANTHROPIC_KEY_do_not_read")
        .env("RUST_BACKTRACE", "1");
    let output = run_command(&mut command, Some(&frame), Duration::ZERO);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let actual = decode_preparation_response(&output.stdout).expect("valid response");
    let expected =
        prepare_shape_consistent_non_executing_response(&request).expect("expected response");
    assert_eq!(actual, expected);
}

#[test]
fn supervisor_rejects_a_child_that_exits_before_the_stdin_close_signal() {
    let result = std::panic::catch_unwind(|| {
        let mut exits_immediately = Command::new("/usr/bin/true");
        let _ = run_command(
            &mut exits_immediately,
            Some(b"held-open-input"),
            Duration::from_millis(200),
        );
    });
    assert!(result.is_err());
}
