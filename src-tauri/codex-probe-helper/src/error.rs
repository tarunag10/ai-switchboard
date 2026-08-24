use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    FrameTooShort,
    FrameLengthZero,
    FrameLengthTooLarge,
    FrameLengthMismatch,
    InvalidUtf8,
    JsonRejected,
    UnsupportedProtocolVersion,
    InvalidIdentifier,
    InvalidDigest,
    InvalidPreparedAt,
    FixedPolicyViolation,
    HostTranscriptMismatch,
    FrameDigestMismatch,
    NonCanonicalJson,
    EncodedFrameTooLarge,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::FrameTooShort => "protocol frame is shorter than its length prefix",
            Self::FrameLengthZero => "protocol frame payload is empty",
            Self::FrameLengthTooLarge => "protocol frame exceeds the fixed size limit",
            Self::FrameLengthMismatch => "protocol frame length does not match its prefix",
            Self::InvalidUtf8 => "protocol frame is not valid UTF-8",
            Self::JsonRejected => "protocol JSON does not match the closed schema",
            Self::UnsupportedProtocolVersion => "protocol version is unsupported",
            Self::InvalidIdentifier => "protocol identifier is invalid",
            Self::InvalidDigest => "protocol digest is invalid",
            Self::InvalidPreparedAt => "protocol preparation timestamp is invalid",
            Self::FixedPolicyViolation => "protocol message violates the no-process policy",
            Self::HostTranscriptMismatch => {
                "opaque host preparation transcript is not internally consistent"
            }
            Self::FrameDigestMismatch => "protocol frame digest does not match its content",
            Self::NonCanonicalJson => "protocol JSON is not in canonical wire form",
            Self::EncodedFrameTooLarge => "encoded protocol frame exceeds the fixed size limit",
        })
    }
}

impl std::error::Error for ProtocolError {}
