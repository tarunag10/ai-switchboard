#![forbid(unsafe_code)]

use std::io::{self, Read, Write};

use codex_probe_helper::{
    decode_preparation_request, encode_preparation_response,
    prepare_shape_consistent_non_executing_response, FRAME_PREFIX_BYTES, MAX_FRAME_BYTES,
};

#[derive(Debug)]
struct HelperFailure;

fn main() -> Result<(), HelperFailure> {
    run().map_err(|_| HelperFailure)
}

fn run() -> Result<(), ()> {
    let mut input = io::stdin().lock();
    let frame = read_one_closed_frame(&mut input)?;
    let request = decode_preparation_request(&frame).map_err(|_| ())?;
    let response = prepare_shape_consistent_non_executing_response(&request).map_err(|_| ())?;
    let encoded = encode_preparation_response(&response).map_err(|_| ())?;

    let mut output = io::stdout().lock();
    output.write_all(&encoded).map_err(|_| ())?;
    output.flush().map_err(|_| ())
}

fn read_one_closed_frame<R: Read>(input: &mut R) -> Result<Vec<u8>, ()> {
    let mut prefix = [0_u8; FRAME_PREFIX_BYTES];
    input.read_exact(&mut prefix).map_err(|_| ())?;

    let payload_bytes = u32::from_be_bytes(prefix) as usize;
    let maximum_payload_bytes = MAX_FRAME_BYTES - FRAME_PREFIX_BYTES;
    if payload_bytes == 0 || payload_bytes > maximum_payload_bytes {
        return Err(());
    }

    let mut frame = Vec::with_capacity(FRAME_PREFIX_BYTES + payload_bytes);
    frame.extend_from_slice(&prefix);
    frame.resize(FRAME_PREFIX_BYTES + payload_bytes, 0);
    input
        .read_exact(&mut frame[FRAME_PREFIX_BYTES..])
        .map_err(|_| ())?;

    let mut trailing = [0_u8; 1];
    match input.read(&mut trailing) {
        Ok(0) => Ok(frame),
        Ok(_) | Err(_) => Err(()),
    }
}
