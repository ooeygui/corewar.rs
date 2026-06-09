use thiserror::Error;

use crate::PROTOCOL_VERSION;

/// Errors that can occur while encoding or decoding protocol messages.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("failed to encode protocol message: {0}")]
    Encode(#[source] rmp_serde::encode::Error),
    #[error("failed to decode protocol message: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
    #[error("failed to encode protocol message as JSON: {0}")]
    JsonEncode(#[source] serde_json::Error),
    #[error("failed to decode protocol message from JSON: {0}")]
    JsonDecode(#[source] serde_json::Error),
    #[error("protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
}

impl ProtocolError {
    /// Construct a version mismatch against the current protocol version.
    pub fn version_mismatch(actual: u32) -> Self {
        Self::VersionMismatch {
            expected: PROTOCOL_VERSION,
            actual,
        }
    }
}
