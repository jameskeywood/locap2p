use locap2p_network::NetworkError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GossipError {
    #[error("topic must not be empty")]
    InvalidTopic,
    #[error("failed to encode event: {0}")]
    Encode(#[from] serde_json::Error),
    #[error(transparent)]
    Network(#[from] NetworkError),
}

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("malformed event: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("event id does not match event content")]
    InvalidEventId,
    #[error("event author does not match network message source")]
    AuthorMismatch,
}
