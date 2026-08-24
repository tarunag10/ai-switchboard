//! Read-only command surface for shared Switchboard contracts.
//!
//! Production commands use standard input and output only. They do not open
//! files, start processes, access a network, resolve providers, or mutate a
//! Workbench session.

use std::io::{Read, Write};

use chrono::DateTime;
use switchboard_core::router::{build_endpoint_route_plan, EndpointRoutePlanInput};
use switchboard_core::workbench::WorkbenchSession;
use switchboard_core::{ExecutionMode, HarnessSurface};
use switchboard_runtime::{PortableRuntime, RuntimeAdapter};

pub const MAX_SESSION_INPUT_BYTES: usize = 1024 * 1024;
pub const MAX_ROUTER_INPUT_BYTES: usize = 1024 * 1024;
pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_INTERNAL: u8 = 1;
pub const EXIT_USAGE: u8 = 2;

pub const USAGE: &str = "Switchboard CLI

Usage:
  switchboard harness status
  switchboard router endpoint plan
  switchboard workbench session serialize
  switchboard --help

Safety:
  harness status reports the fail-closed portable runtime contract.
  router endpoint plan reads one bounded content-free endpoint request from
  stdin and writes a deterministic observe-only plan without provider traffic.
  workbench session serialize reads one bounded JSON document from stdin,
  validates the content-free Workbench session, and writes deterministic JSON.
  No command starts a process, accesses a provider, or reads or writes files.";

pub fn run_cli<R: Read, W: Write, E: Write>(
    args: &[String],
    input: &mut R,
    output: &mut W,
    error: &mut E,
) -> u8 {
    match args {
        [flag] if flag == "--help" || flag == "-h" || flag == "help" => {
            write_text(output, error, USAGE)
        }
        [command, subcommand] if command == "harness" && subcommand == "status" => {
            write_harness_status(output, error)
        }
        [command, subject, action]
            if command == "router" && subject == "endpoint" && action == "plan" =>
        {
            plan_endpoint_route(input, output, error)
        }
        [command, subject, action]
            if command == "workbench" && subject == "session" && action == "serialize" =>
        {
            serialize_workbench_session(input, output, error)
        }
        [] => usage_error(error, "a command is required"),
        [command, ..] if command == "harness" => {
            usage_error(error, "expected `switchboard harness status`")
        }
        [command, ..] if command == "router" => {
            usage_error(error, "expected `switchboard router endpoint plan`")
        }
        [command, ..] if command == "workbench" => {
            usage_error(error, "expected `switchboard workbench session serialize`")
        }
        _ => usage_error(error, "unsupported command"),
    }
}

fn write_harness_status<W: Write, E: Write>(output: &mut W, error: &mut E) -> u8 {
    write_harness_status_with_runtime(&PortableRuntime, HarnessSurface::Cli, output, error)
}

fn write_harness_status_with_runtime<R, W, E>(
    runtime: &R,
    surface: HarnessSurface,
    output: &mut W,
    error: &mut E,
) -> u8
where
    R: RuntimeAdapter,
    W: Write,
    E: Write,
{
    let status = runtime.harness_status(surface);
    if status.surface != surface
        || status.execution_mode != ExecutionMode::ObserveOnly
        || status.provider_traffic_enabled
        || status.process_start_enabled
    {
        return internal_error(error, "portable runtime did not remain fail-closed");
    }

    let encoded = match serde_json::to_vec(&status) {
        Ok(encoded) => encoded,
        Err(_) => return internal_error(error, "failed to encode harness status"),
    };
    write_json(output, error, &encoded)
}

fn serialize_workbench_session<R: Read, W: Write, E: Write>(
    input: &mut R,
    output: &mut W,
    error: &mut E,
) -> u8 {
    let bytes = match read_bounded(input, MAX_SESSION_INPUT_BYTES) {
        Ok(bytes) => bytes,
        Err(ReadFailure::TooLarge) => {
            return invalid_input(error, "Workbench session input exceeds the 1 MiB limit")
        }
        Err(ReadFailure::Input) => {
            return internal_error(error, "failed to read Workbench session from stdin")
        }
    };
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return invalid_input(error, "Workbench session JSON is required on stdin");
    }

    let session = match serde_json::from_slice::<WorkbenchSession>(&bytes) {
        Ok(session) => session,
        Err(parse_error) => return invalid_workbench_session_input(error, parse_error),
    };
    if validate_session_for_serialization(&session).is_err() {
        return invalid_workbench_session_validation(error);
    }

    let encoded = match serde_json::to_vec(&session) {
        Ok(encoded) => encoded,
        Err(_) => return internal_error(error, "failed to encode Workbench session"),
    };
    write_json(output, error, &encoded)
}

fn plan_endpoint_route<R: Read, W: Write, E: Write>(
    input: &mut R,
    output: &mut W,
    error: &mut E,
) -> u8 {
    let bytes = match read_bounded(input, MAX_ROUTER_INPUT_BYTES) {
        Ok(bytes) => bytes,
        Err(ReadFailure::TooLarge) => {
            return invalid_input(error, "Router endpoint plan input exceeds the 1 MiB limit")
        }
        Err(ReadFailure::Input) => {
            return internal_error(error, "failed to read Router endpoint plan from stdin")
        }
    };
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return invalid_input(error, "Router endpoint plan JSON is required on stdin");
    }

    let input = match serde_json::from_slice::<EndpointRoutePlanInput>(&bytes) {
        Ok(input) => input,
        Err(parse_error) => return invalid_router_endpoint_plan_input(error, parse_error),
    };
    let plan = match build_endpoint_route_plan(&input) {
        Ok(plan) => plan,
        Err(_) => return invalid_router_endpoint_plan_validation(error),
    };
    let encoded = match serde_json::to_vec(&plan) {
        Ok(encoded) => encoded,
        Err(_) => return internal_error(error, "failed to encode Router endpoint plan"),
    };
    write_json(output, error, &encoded)
}

fn validate_session_for_serialization(session: &WorkbenchSession) -> Result<(), ()> {
    session.validate().map_err(|_| ())?;
    for value in [&session.created_at, &session.updated_at] {
        if value.len() > 64 || DateTime::parse_from_rfc3339(value).is_err() {
            return Err(());
        }
    }
    Ok(())
}

enum WorkbenchSessionInputFailure {
    MalformedJson,
    UnsupportedField,
    UnsupportedEnumValue,
    ValidationFailed,
}

fn workbench_session_input_failure_message(failure: WorkbenchSessionInputFailure) -> &'static str {
    match failure {
        WorkbenchSessionInputFailure::MalformedJson => "Workbench session JSON is malformed",
        WorkbenchSessionInputFailure::UnsupportedField => {
            "Workbench session JSON contains an unsupported field"
        }
        WorkbenchSessionInputFailure::UnsupportedEnumValue => {
            "Workbench session JSON contains an unsupported enum value"
        }
        WorkbenchSessionInputFailure::ValidationFailed => {
            "Workbench session JSON failed validation"
        }
    }
}

fn classify_workbench_session_parse_error(
    error: &serde_json::Error,
) -> WorkbenchSessionInputFailure {
    let message = error.to_string();
    if message.contains("unknown field") {
        WorkbenchSessionInputFailure::UnsupportedField
    } else if message.contains("unknown variant") || message.contains("invalid value") {
        WorkbenchSessionInputFailure::UnsupportedEnumValue
    } else if message.contains("missing field") {
        WorkbenchSessionInputFailure::ValidationFailed
    } else {
        match error.classify() {
            serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
                WorkbenchSessionInputFailure::MalformedJson
            }
            serde_json::error::Category::Data | serde_json::error::Category::Io => {
                WorkbenchSessionInputFailure::ValidationFailed
            }
        }
    }
}

fn invalid_workbench_session_input<E: Write>(error: &mut E, parse_error: serde_json::Error) -> u8 {
    let message = workbench_session_input_failure_message(classify_workbench_session_parse_error(
        &parse_error,
    ));
    invalid_input(error, message)
}

fn invalid_workbench_session_validation<E: Write>(error: &mut E) -> u8 {
    invalid_input(
        error,
        workbench_session_input_failure_message(WorkbenchSessionInputFailure::ValidationFailed),
    )
}

enum RouterEndpointPlanInputFailure {
    MalformedJson,
    UnsupportedField,
    UnsupportedEnumValue,
    ValidationFailed,
}

fn router_endpoint_plan_input_failure_message(
    failure: RouterEndpointPlanInputFailure,
) -> &'static str {
    match failure {
        RouterEndpointPlanInputFailure::MalformedJson => "Router endpoint plan JSON is malformed",
        RouterEndpointPlanInputFailure::UnsupportedField => {
            "Router endpoint plan JSON contains an unsupported field"
        }
        RouterEndpointPlanInputFailure::UnsupportedEnumValue => {
            "Router endpoint plan JSON contains an unsupported enum value"
        }
        RouterEndpointPlanInputFailure::ValidationFailed => {
            "Router endpoint plan JSON failed validation"
        }
    }
}

fn classify_router_endpoint_plan_parse_error(
    error: &serde_json::Error,
) -> RouterEndpointPlanInputFailure {
    let message = error.to_string();
    if message.contains("unknown field") {
        RouterEndpointPlanInputFailure::UnsupportedField
    } else if message.contains("unknown variant") || message.contains("invalid value") {
        RouterEndpointPlanInputFailure::UnsupportedEnumValue
    } else if message.contains("missing field") {
        RouterEndpointPlanInputFailure::ValidationFailed
    } else {
        match error.classify() {
            serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
                RouterEndpointPlanInputFailure::MalformedJson
            }
            serde_json::error::Category::Data | serde_json::error::Category::Io => {
                RouterEndpointPlanInputFailure::ValidationFailed
            }
        }
    }
}

fn invalid_router_endpoint_plan_input<E: Write>(
    error: &mut E,
    parse_error: serde_json::Error,
) -> u8 {
    let message = router_endpoint_plan_input_failure_message(
        classify_router_endpoint_plan_parse_error(&parse_error),
    );
    invalid_input(error, message)
}

fn invalid_router_endpoint_plan_validation<E: Write>(error: &mut E) -> u8 {
    invalid_input(
        error,
        router_endpoint_plan_input_failure_message(
            RouterEndpointPlanInputFailure::ValidationFailed,
        ),
    )
}

enum ReadFailure {
    TooLarge,
    Input,
}

fn read_bounded<R: Read>(input: &mut R, maximum_bytes: usize) -> Result<Vec<u8>, ReadFailure> {
    let mut bytes = Vec::new();
    input
        .take((maximum_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ReadFailure::Input)?;
    if bytes.len() > maximum_bytes {
        return Err(ReadFailure::TooLarge);
    }
    Ok(bytes)
}

fn write_json<W: Write, E: Write>(output: &mut W, error: &mut E, encoded: &[u8]) -> u8 {
    if output
        .write_all(encoded)
        .and_then(|_| output.write_all(b"\n"))
        .is_err()
    {
        return internal_error(error, "failed to write command output");
    }
    EXIT_SUCCESS
}

fn write_text<W: Write, E: Write>(output: &mut W, error: &mut E, text: &str) -> u8 {
    if writeln!(output, "{text}").is_err() {
        return internal_error(error, "failed to write command output");
    }
    EXIT_SUCCESS
}

fn usage_error<E: Write>(error: &mut E, message: &str) -> u8 {
    let _ = writeln!(error, "error: {message}\n\n{USAGE}");
    EXIT_USAGE
}

fn invalid_input<E: Write>(error: &mut E, message: &str) -> u8 {
    let _ = writeln!(error, "error: {message}");
    EXIT_USAGE
}

fn internal_error<E: Write>(error: &mut E, message: &str) -> u8 {
    let _ = writeln!(error, "error: {message}");
    EXIT_INTERNAL
}

#[cfg(test)]
mod tests {
    use super::*;
    use switchboard_runtime::{RuntimeCapabilities, RuntimeClock};

    #[derive(Clone, Copy, Debug)]
    struct FakeRuntime {
        provider_transport: bool,
        process_start: bool,
    }

    impl RuntimeClock for FakeRuntime {
        fn unix_millis(&self) -> i64 {
            1_725_000_123_456
        }
    }

    impl RuntimeAdapter for FakeRuntime {
        fn capabilities(&self) -> RuntimeCapabilities {
            RuntimeCapabilities {
                filesystem: false,
                process_start: self.process_start,
                provider_transport: self.provider_transport,
                secret_store: false,
            }
        }
    }

    fn run_harness_status(
        runtime: &impl RuntimeAdapter,
        surface: HarnessSurface,
    ) -> (u8, String, String) {
        let mut output = Vec::new();
        let mut error = Vec::new();
        let code = write_harness_status_with_runtime(runtime, surface, &mut output, &mut error);
        (
            code,
            String::from_utf8(output).expect("stdout UTF-8"),
            String::from_utf8(error).expect("stderr UTF-8"),
        )
    }

    #[test]
    fn harness_status_uses_injected_runtime_and_preserves_cli_contract() {
        let runtime = FakeRuntime {
            provider_transport: false,
            process_start: false,
        };

        let (code, output, error) = run_harness_status(&runtime, HarnessSurface::Cli);
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
    fn harness_status_rejects_provider_or_process_enabled_capabilities() {
        for runtime in [
            FakeRuntime {
                provider_transport: true,
                process_start: false,
            },
            FakeRuntime {
                provider_transport: false,
                process_start: true,
            },
        ] {
            let (code, output, error) = run_harness_status(&runtime, HarnessSurface::Cli);
            assert_eq!(code, EXIT_INTERNAL);
            assert!(output.is_empty());
            assert!(error.contains("did not remain fail-closed"));
        }
    }
}
