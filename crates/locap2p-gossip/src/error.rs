use locap2p_network::NetworkError;
use thiserror::Error;

/// Errors returned by gossip operations.
#[derive(Debug, Error)]
pub enum GossipError {
    #[error("invalid gossip configuration: {0}")]
    InvalidConfig(&'static str),
    #[error("failed to encode gossip message: {0}")]
    Encode(#[from] serde_json::Error),
    #[error(transparent)]
    Network(#[from] NetworkError),
}

/// Errors returned when encoding or validating gossip messages.
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("malformed gossip message: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("event id does not match event content")]
    InvalidEventId,
}
