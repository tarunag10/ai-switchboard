use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut error = stderr.lock();
    ExitCode::from(switchboard_cli::run_cli(
        &args,
        &mut input,
        &mut output,
        &mut error,
    ))
}
