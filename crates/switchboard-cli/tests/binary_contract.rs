use std::io::Write;
use std::process::{Command, Stdio};

const SESSION: &[u8] = include_bytes!("fixtures/session-active-v1.json");

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
    let mut child = Command::new(env!("CARGO_BIN_EXE_switchboard"))
        .args(["workbench", "session", "serialize"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Switchboard CLI");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(SESSION)
        .expect("write session fixture");
    let output = child.wait_with_output().expect("wait for Switchboard CLI");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let serialized: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("serialized session JSON");
    assert_eq!(serialized["sessionId"], "workbench:test");
    assert_eq!(serialized["executionMode"], "plan_only");
    assert_eq!(serialized["providerTraffic"], "none");
}
