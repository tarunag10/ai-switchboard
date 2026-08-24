use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

const CHILD_DEADLINE: Duration = Duration::from_secs(5);
const REAP_DEADLINE: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_CHILD_OUTPUT_BYTES: usize = 1024 * 1024;

pub(crate) fn run_command(
    command: &mut Command,
    input: Option<&[u8]>,
    hold_stdin_open: Duration,
) -> Output {
    let started = Instant::now();
    let deadline = started + CHILD_DEADLINE;
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn supervised test child");
    let stdout = child.stdout.take().expect("piped child stdout");
    let stderr = child.stderr.take().expect("piped child stderr");
    let stdout_reader = thread::spawn(move || read_to_end(stdout));
    let stderr_reader = thread::spawn(move || read_to_end(stderr));
    let mut writer_ready = None;
    let mut writer_close = None;
    let writer = input.map(|input| {
        let mut stdin = child.stdin.take().expect("piped child stdin");
        let input = input.to_vec();
        let (ready_sender, ready_receiver) = mpsc::channel();
        let (close_sender, close_receiver) = mpsc::channel();
        writer_ready = Some(ready_receiver);
        writer_close = Some(close_sender);
        thread::spawn(move || {
            let write_result = stdin
                .write_all(&input)
                .and_then(|()| stdin.flush())
                .map_err(|_| ());
            let _ = ready_sender.send(write_result);
            write_result?;
            close_receiver.recv().map_err(|_| ())?;
            drop(stdin);
            Ok::<(), ()>(())
        })
    });

    let mut preclose_failure = None;
    if let Some(ready) = writer_ready.as_ref() {
        if let Err(reason) = wait_for_writer_ready(&mut child, ready, deadline) {
            preclose_failure = Some(reason);
        }
        if preclose_failure.is_none() && !hold_stdin_open.is_zero() {
            let hold_deadline = Instant::now() + hold_stdin_open;
            if let Err(reason) = require_running_until(&mut child, hold_deadline, deadline) {
                preclose_failure = Some(reason);
            }
        }
    } else if !hold_stdin_open.is_zero() {
        preclose_failure = Some("cannot hold null stdin open");
    }
    if let Some(close) = writer_close.take() {
        if close.send(()).is_err() && preclose_failure.is_none() {
            preclose_failure = Some("stdin writer ended before the close signal");
        }
    }
    if let Some(reason) = preclose_failure {
        terminate_and_reap(&mut child);
        if let Some(writer) = writer {
            let _ = writer.join();
        }
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
        panic!("supervised test child failed: {reason}");
    }

    let status = match poll_until_exit(&mut child, deadline) {
        Ok(status) => status,
        Err(reason) => {
            terminate_and_reap(&mut child);
            if let Some(writer) = writer {
                let _ = writer.join();
            }
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            panic!("supervised test child failed: {reason}");
        }
    };
    if let Some(writer) = writer {
        writer
            .join()
            .expect("join child stdin writer")
            .expect("write child stdin");
    }
    let stdout = stdout_reader
        .join()
        .expect("join child stdout reader")
        .expect("read child stdout");
    let stderr = stderr_reader
        .join()
        .expect("join child stderr reader")
        .expect("read child stderr");
    Output {
        status,
        stdout,
        stderr,
    }
}

fn wait_for_writer_ready(
    child: &mut Child,
    ready: &Receiver<Result<(), ()>>,
    deadline: Instant,
) -> Result<(), &'static str> {
    loop {
        match ready.recv_timeout(POLL_INTERVAL) {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(())) => return Err("stdin write failed"),
            Err(RecvTimeoutError::Disconnected) => return Err("stdin writer disconnected"),
            Err(RecvTimeoutError::Timeout) => match child.try_wait() {
                Ok(Some(_)) => return Err("child exited before stdin was written"),
                Ok(None) if Instant::now() < deadline => {}
                Ok(None) => return Err("deadline exceeded while writing stdin"),
                Err(_) => return Err("poll failed while writing stdin"),
            },
        }
    }
}

fn require_running_until(
    child: &mut Child,
    hold_deadline: Instant,
    process_deadline: Instant,
) -> Result<(), &'static str> {
    while Instant::now() < hold_deadline {
        match child.try_wait() {
            Ok(Some(_)) => return Err("child exited before the stdin close signal"),
            Ok(None) if Instant::now() < process_deadline => thread::sleep(POLL_INTERVAL),
            Ok(None) => return Err("deadline exceeded while holding stdin open"),
            Err(_) => return Err("poll failed while holding stdin open"),
        }
    }
    Ok(())
}

fn poll_until_exit(child: &mut Child, deadline: Instant) -> Result<ExitStatus, &'static str> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            Ok(None) => return Err("deadline exceeded"),
            Err(_) => return Err("poll failed"),
        }
    }
}

fn terminate_and_reap(child: &mut Child) {
    let _ = child.kill();
    let deadline = Instant::now() + REAP_DEADLINE;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            Ok(None) => panic!("killed test child was not reaped before the cleanup deadline"),
            Err(error) => panic!("could not reap test child after termination: {error}"),
        }
    }
}

fn read_to_end<R: Read>(reader: R) -> Result<Vec<u8>, ()> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_CHILD_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > MAX_CHILD_OUTPUT_BYTES {
        return Err(());
    }
    Ok(bytes)
}
