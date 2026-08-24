use std::io::Write;
use std::process::{Command, Stdio};

const SESSION: &[u8] = include_bytes!("fixtures/session-active-v1.json");

fn router_request() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion": 1,
        "request": {
            "requestedModel": "exact-model",
            "requiredFeatures": ["streaming"],
            "privacy": "require_local",
            "maximumCostMicrousdPerMillionInputTokens": 1_000,
            "maximumQueueLatencyMs": 100,
            "preferredEndpointId": null
        },
        "candidates": [{
            "id": "local",
            "enabled": true,
            "verified": true,
            "health": "healthy",
            "privacy": "local",
            "costMicrousdPerMillionInputTokens": 100,
            "queueLatencyMs": 20,
            "features": ["streaming"],
            "availableModels": ["exact-model"]
        }]
    }))
    .expect("router request")
}

fn run_with_stdin(args: &[&str], input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_switchboard"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Switchboard CLI");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input)
        .expect("write CLI stdin");
    child.wait_with_output().expect("wait for Switchboard CLI")
}

#[test]
fn binary_exposes_fail_closed_harness_status() {
    let output = Command::new(env!("CARGO_BIN_EXE_switchboard"))
        .args(["harness", "status"])
        .output()
        .expect("run Switchboard CLI");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout UTF-8"),
        concat!(
            "{\"contractVersion\":1,\"surface\":\"cli\",",
            "\"executionMode\":\"observe_only\",",
            "\"providerTrafficEnabled\":false,\"processStartEnabled\":false}\n"
        )
    );
}

#[test]
fn binary_serializes_a_valid_session_from_stdin() {
    let output = run_with_stdin(&["workbench", "session", "serialize"], SESSION);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let serialized: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("serialized session JSON");
    assert_eq!(serialized["sessionId"], "workbench:test");
    assert_eq!(serialized["executionMode"], "plan_only");
    assert_eq!(serialized["providerTraffic"], "none");
}

#[test]
fn binary_plans_an_observe_only_endpoint_route_from_stdin() {
    let output = run_with_stdin(&["router", "endpoint", "plan"], &router_request());
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let plan: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("endpoint plan JSON");
    assert_eq!(plan["strategy"], "deterministic_endpoint");
    assert_eq!(plan["executionMode"], "observe_only");
    assert_eq!(plan["requestedModel"], "exact-model");
    assert_eq!(plan["actualModel"], "exact-model");
    assert_eq!(plan["providerTrafficEnabled"], false);
    assert_eq!(plan["processStartEnabled"], false);
    assert_eq!(plan["endpoint"]["selectedEndpointId"], "local");
}

#[test]
fn binary_router_errors_are_stable_and_content_free() {
    let private = b"{\"schemaVersion\":1,\"privatePrompt\":\"do not echo this\"}";
    let output = run_with_stdin(&["router", "endpoint", "plan"], private);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).expect("stderr UTF-8");
    assert!(error.contains("unsupported field"));
    assert!(!error.contains("do not echo this"));
    assert!(!error.contains("privatePrompt"));
}
