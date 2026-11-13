use thiserror::Error;

/// Errors returned by the network layer.
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("failed to build the network: {0}")]
    Build(String),
    #[error("failed to listen on an address: {0}")]
    Listen(String),
    #[error("invalid peer id: {0}")]
    InvalidPeer(String),
    #[error("failed to send request: {0}")]
    Request(String),
    #[error("failed to send response: {0}")]
    Respond(String),
}
