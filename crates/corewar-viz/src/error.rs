use thiserror::Error;

/// Errors that can occur while communicating with the visualization server.
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("websocket connection failed: {0}")]
    Connect(String),
    #[error("websocket send failed: {0}")]
    Send(String),
    #[error("outbound websocket channel closed")]
    ChannelClosed,
    #[error("failed to serialize client message: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to decode server JSON message: {0}")]
    DecodeJson(#[source] serde_json::Error),
    #[error("failed to decode server MessagePack message: {0}")]
    DecodeMessagePack(#[source] rmp_serde::decode::Error),
}
