use thiserror::Error;

/// Errors returned by the network layer.
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("failed to build the network: {0}")]
    Build(String),
    #[error("failed to listen on an address: {0}")]
    Listen(String),
    #[error("failed to subscribe to topic: {0}")]
    Subscribe(String),
    #[error("failed to publish message: {0}")]
    Publish(String),
}
