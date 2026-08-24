//! Read-only command surface for shared Switchboard contracts.
//!
//! Production commands use standard input and output only. They do not open
//! files, start processes, access a network, resolve providers, or mutate a
//! Workbench session.

use std::io::{Read, Write};

use chrono::DateTime;
use switchboard_core::workbench::WorkbenchSession;
use switchboard_core::{ExecutionMode, HarnessSurface};
use switchboard_runtime::{PortableRuntime, RuntimeAdapter};

pub const MAX_SESSION_INPUT_BYTES: usize = 1024 * 1024;
pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_INTERNAL: u8 = 1;
pub const EXIT_USAGE: u8 = 2;

pub const USAGE: &str = "Switchboard CLI

Usage:
  switchboard harness status
  switchboard workbench session serialize
  switchboard --help

Safety:
  harness status reports the fail-closed portable runtime contract.
  workbench session serialize reads one bounded JSON document from stdin,
  validates the content-free Workbench session, and writes deterministic JSON.
  Neither command starts a process, accesses a provider, or reads or writes files.";

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
            if command == "workbench" && subject == "session" && action == "serialize" =>
        {
            serialize_workbench_session(input, output, error)
        }
        [] => usage_error(error, "a command is required"),
        [command, ..] if command == "harness" => {
            usage_error(error, "expected `switchboard harness status`")
        }
        [command, ..] if command == "workbench" => {
            usage_error(error, "expected `switchboard workbench session serialize`")
        }
        _ => usage_error(error, "unsupported command"),
    }
}

fn write_harness_status<W: Write, E: Write>(output: &mut W, error: &mut E) -> u8 {
    let status = PortableRuntime.harness_status(HarnessSurface::Cli);
    if status.surface != HarnessSurface::Cli
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
    let bytes = match read_bounded(input) {
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
        Err(parse_error) => {
            return invalid_input(
                error,
                &format!("invalid Workbench session JSON: {parse_error}"),
            )
        }
    };
    if let Err(validation_error) = validate_session_for_serialization(&session) {
        return invalid_input(error, &validation_error);
    }

    let encoded = match serde_json::to_vec(&session) {
        Ok(encoded) => encoded,
        Err(_) => return internal_error(error, "failed to encode Workbench session"),
    };
    write_json(output, error, &encoded)
}

fn validate_session_for_serialization(session: &WorkbenchSession) -> Result<(), String> {
    session.validate().map_err(|error| error.to_string())?;
    for (value, label) in [
        (&session.created_at, "createdAt"),
        (&session.updated_at, "updatedAt"),
    ] {
        if value.len() > 64 || DateTime::parse_from_rfc3339(value).is_err() {
            return Err(format!(
                "Workbench session {label} must be a bounded RFC3339 timestamp"
            ));
        }
    }
    Ok(())
}

enum ReadFailure {
    TooLarge,
    Input,
}

fn read_bounded<R: Read>(input: &mut R) -> Result<Vec<u8>, ReadFailure> {
    let mut bytes = Vec::new();
    input
        .take((MAX_SESSION_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ReadFailure::Input)?;
    if bytes.len() > MAX_SESSION_INPUT_BYTES {
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
