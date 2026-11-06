use thiserror::Error;

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
    #[error("network task is no longer running")]
    Closed,
}
