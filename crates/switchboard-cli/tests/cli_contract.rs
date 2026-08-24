use serde_json::{json, Value};
use switchboard_cli::{
    run_cli, EXIT_SUCCESS, EXIT_USAGE, MAX_ROUTER_INPUT_BYTES, MAX_SESSION_INPUT_BYTES,
};

const SESSION: &[u8] = include_bytes!("fixtures/session-active-v1.json");

fn run(args: &[&str], input: &[u8]) -> (u8, String, String) {
    let args = args
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    let mut input = input;
    let mut output = Vec::new();
    let mut error = Vec::new();
    let code = run_cli(&args, &mut input, &mut output, &mut error);
    (
        code,
        String::from_utf8(output).expect("stdout UTF-8"),
        String::from_utf8(error).expect("stderr UTF-8"),
    )
}

fn session_value() -> Value {
    serde_json::from_slice(SESSION).expect("session fixture")
}

fn router_request() -> Value {
    json!({
        "schemaVersion": 1,
        "request": {
            "requestedModel": "exact-model",
            "requiredFeatures": ["streaming", "tools"],
            "privacy": "prefer_local",
            "maximumCostMicrousdPerMillionInputTokens": 10_000,
            "maximumQueueLatencyMs": 100,
            "preferredEndpointId": null
        },
        "candidates": [
            {
                "id": "remote",
                "enabled": true,
                "verified": true,
                "health": "healthy",
                "privacy": "remote",
                "costMicrousdPerMillionInputTokens": 10,
                "queueLatencyMs": 2,
                "features": ["streaming", "tools"],
                "availableModels": ["exact-model"]
            },
            {
                "id": "local",
                "enabled": true,
                "verified": true,
                "health": "healthy",
                "privacy": "local",
                "costMicrousdPerMillionInputTokens": 100,
                "queueLatencyMs": 20,
                "features": ["streaming", "tools"],
                "availableModels": ["exact-model"]
            }
        ]
    })
}

#[test]
fn harness_status_is_runtime_backed_and_fail_closed() {
    let (code, output, error) = run(&["harness", "status"], b"");
    assert_eq!(code, EXIT_SUCCESS);
    assert!(error.is_empty());
    assert_eq!(
        output,
        concat!(
            "{\"contractVersion\":1,\"surface\":\"cli\",",
            "\"executionMode\":\"observe_only\",",
            "\"providerTrafficEnabled\":false,\"processStartEnabled\":false}\n"
        )
    );
}

#[test]
fn session_serialization_is_valid_and_deterministic() {
    let (first_code, first, first_error) = run(&["workbench", "session", "serialize"], SESSION);
    let compact_input = serde_json::to_vec(&session_value()).expect("compact fixture");
    let (second_code, second, second_error) =
        run(&["workbench", "session", "serialize"], &compact_input);
    assert_eq!(first_code, EXIT_SUCCESS);
    assert_eq!(second_code, EXIT_SUCCESS);
    assert!(first_error.is_empty());
    assert!(second_error.is_empty());
    assert_eq!(first, second);

    let value: Value = serde_json::from_str(&first).expect("serialized session JSON");
    assert_eq!(value["executionMode"], "plan_only");
    assert_eq!(value["providerTraffic"], "none");
    assert_eq!(value["status"], "active");
}

#[test]
fn endpoint_plan_is_deterministic_compact_and_observe_only() {
    let first_input = serde_json::to_vec_pretty(&router_request()).expect("router request");
    let (first_code, first, first_error) = run(&["router", "endpoint", "plan"], &first_input);

    let mut reordered = router_request();
    reordered["candidates"]
        .as_array_mut()
        .expect("candidate array")
        .reverse();
    let second_input = serde_json::to_vec(&reordered).expect("reordered router request");
    let (second_code, second, second_error) = run(&["router", "endpoint", "plan"], &second_input);

    assert_eq!(first_code, EXIT_SUCCESS);
    assert_eq!(second_code, EXIT_SUCCESS);
    assert!(first_error.is_empty());
    assert!(second_error.is_empty());
    assert_eq!(first, second);
    assert!(first.ends_with('\n'));
    assert!(!first[..first.len() - 1].contains('\n'));
    assert!(!first.contains("  "));

    let value: Value = serde_json::from_str(&first).expect("endpoint plan JSON");
    assert_eq!(value["strategy"], "deterministic_endpoint");
    assert_eq!(value["executionMode"], "observe_only");
    assert_eq!(value["requestedModel"], "exact-model");
    assert_eq!(value["actualModel"], "exact-model");
    assert_eq!(value["providerTrafficEnabled"], false);
    assert_eq!(value["processStartEnabled"], false);
    assert_eq!(value["endpoint"]["selectedEndpointId"], "local");
    assert_eq!(value["endpoint"]["explanations"][0]["endpointId"], "local");
    assert_eq!(value["endpoint"]["explanations"][1]["endpointId"], "remote");
}

#[test]
fn endpoint_plan_rejects_malformed_unknown_and_unsafe_input_without_echoing_values() {
    let malformed = b"{\"schemaVersion\":1,\"request\":";
    let (code, output, error) = run(&["router", "endpoint", "plan"], malformed);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("malformed"));
    assert!(!error.contains("schemaVersion"));

    let mut unknown = router_request();
    unknown["privatePrompt"] = json!("private prompt contents");
    let input = serde_json::to_vec(&unknown).expect("unknown-field request");
    let (code, output, error) = run(&["router", "endpoint", "plan"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("unsupported field"));
    assert!(!error.contains("private prompt contents"));

    let mut unknown_enum = router_request();
    unknown_enum["request"]["privacy"] = json!("private-routing-mode");
    let input = serde_json::to_vec(&unknown_enum).expect("unknown-enum request");
    let (code, output, error) = run(&["router", "endpoint", "plan"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("unsupported enum value"));
    assert!(!error.contains("private-routing-mode"));

    let mut unsafe_request = router_request();
    unsafe_request["request"]["requestedModel"] = json!("private model prompt with spaces");
    let input = serde_json::to_vec(&unsafe_request).expect("unsafe router request");
    let (code, output, error) = run(&["router", "endpoint", "plan"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("failed validation"));
    assert!(!error.contains("private model prompt"));
}

#[test]
fn endpoint_plan_requires_exactly_one_bounded_json_request() {
    let (code, output, error) = run(&["router", "endpoint", "plan"], b" \n\t");
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("JSON is required on stdin"));

    let oversized = vec![b' '; MAX_ROUTER_INPUT_BYTES + 1];
    let (code, output, error) = run(&["router", "endpoint", "plan"], &oversized);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("exceeds the 1 MiB limit"));

    let one = serde_json::to_vec(&router_request()).expect("router request");
    let mut two = one.clone();
    two.extend_from_slice(&one);
    let (code, output, error) = run(&["router", "endpoint", "plan"], &two);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("malformed"));
}

#[test]
fn session_serialization_rejects_unknown_field_without_echoing_values() {
    let mut value = session_value();
    value["prompt"] = json!("private prompt contents must never be echoed");
    let input = serde_json::to_vec(&value).expect("content-bearing session");
    let (code, output, error) = run(&["workbench", "session", "serialize"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("unsupported field"));
    assert!(!error.contains("private prompt contents"));
}

#[test]
fn session_serialization_rejects_unknown_enum_without_echoing_values() {
    let mut value = session_value();
    value["status"] = json!("totally-unknown");
    let input = serde_json::to_vec(&value).expect("status mismatch session");
    let (code, output, error) = run(&["workbench", "session", "serialize"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("unsupported enum value"));
    assert!(!error.contains("totally-unknown"));
}

#[test]
fn session_serialization_rejects_invalid_digest_and_status() {
    let mut value = session_value();
    value["workspaceDigest"] = json!("/Users/alice/private-repo");
    let input = serde_json::to_vec(&value).expect("invalid session");
    let (code, output, error) = run(&["workbench", "session", "serialize"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("failed validation"));

    let mut value = session_value();
    value["status"] = json!("completed");
    let input = serde_json::to_vec(&value).expect("status mismatch session");
    let (code, output, error) = run(&["workbench", "session", "serialize"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("failed validation"));
}

#[test]
fn session_serialization_rejects_bad_sequence_and_session_timestamps() {
    let mut value = session_value();
    value["events"][0]["sequence"] = json!(1);
    let input = serde_json::to_vec(&value).expect("invalid sequence session");
    let (code, output, error) = run(&["workbench", "session", "serialize"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("failed validation"));

    let mut value = session_value();
    value["createdAt"] = json!("private text in a timestamp field");
    let input = serde_json::to_vec(&value).expect("invalid timestamp session");
    let (code, output, error) = run(&["workbench", "session", "serialize"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("failed validation"));
    assert!(!error.contains("private text"));
}

#[test]
fn session_serialization_rejects_malformed_json_without_echoing_input() {
    let malformed = b"{\"schemaVersion\":1,\"sessionId\":\"workbench:test\",";
    let (code, output, error) = run(&["workbench", "session", "serialize"], malformed);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("malformed"));
    assert!(!error.contains("schemaVersion"));
    assert!(!error.contains("workbench:test"));
}

#[test]
fn session_serialization_rejects_control_characters_without_echoing_them() {
    let mut value = session_value();
    value["sessionId"] = json!("workbench:test\u{0007}");
    let input = serde_json::to_vec(&value).expect("control-char session");
    let (code, output, error) = run(&["workbench", "session", "serialize"], &input);
    assert_eq!(code, EXIT_USAGE);
    assert!(output.is_empty());
    assert!(error.contains("failed validation"));
    assert!(!error.contains("workbench:test"));
}

#[test]
fn session_serialization_rejects_empty_and_oversized_input() {
    let (empty_code, empty_output, empty_error) =
        run(&["workbench", "session", "serialize"], b" \n\t");
    assert_eq!(empty_code, EXIT_USAGE);
    assert!(empty_output.is_empty());
    assert!(empty_error.contains("JSON is required on stdin"));

    let oversized = vec![b' '; MAX_SESSION_INPUT_BYTES + 1];
    let (large_code, large_output, large_error) =
        run(&["workbench", "session", "serialize"], &oversized);
    assert_eq!(large_code, EXIT_USAGE);
    assert!(large_output.is_empty());
    assert!(large_error.contains("exceeds the 1 MiB limit"));
}

#[test]
fn usage_errors_are_explicit_without_reflecting_arguments() {
    let (missing_code, missing_output, missing_error) = run(&[], b"");
    assert_eq!(missing_code, EXIT_USAGE);
    assert!(missing_output.is_empty());
    assert!(missing_error.contains("a command is required"));
    assert!(missing_error.contains("switchboard harness status"));

    let private_argument = "private-token-value";
    let (unknown_code, unknown_output, unknown_error) = run(&[private_argument], b"");
    assert_eq!(unknown_code, EXIT_USAGE);
    assert!(unknown_output.is_empty());
    assert!(unknown_error.contains("unsupported command"));
    assert!(!unknown_error.contains(private_argument));
}
