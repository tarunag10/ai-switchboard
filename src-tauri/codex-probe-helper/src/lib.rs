//! Bounded, preparation-only protocol for a future Codex probe helper.
//!
//! Version 1 has no execution message, binary, transport, or launch authority.

#![forbid(unsafe_code)]

mod digest;
mod error;
mod protocol;

pub use error::ProtocolError;
pub use protocol::{
    decode_preparation_request, decode_preparation_response, encode_preparation_request,
    encode_preparation_response, preparation_request_from_host,
    prepare_shape_consistent_non_executing_response, CollectionProvenance, ContainmentProfile,
    HelperAction, HelperBoundary, HostPreparationProjection, LaunchAuthority,
    PreparationMessageKind, PreparationRequestFrame, PreparationResponseFrame,
    PreparationResultState, ProviderTraffic, FRAME_PREFIX_BYTES, MAX_FRAME_BYTES,
    MAX_IDENTIFIER_BYTES, MAX_PAYLOAD_BYTES, PROTOCOL_VERSION,
};
