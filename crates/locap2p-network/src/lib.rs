mod actor;
mod behaviour;
mod config;
mod error;
mod event;
mod handle;

pub use config::{DiscoveryMode, NetworkConfig};
pub use error::NetworkError;
pub use event::NetworkEvent;
pub use handle::{NetworkEvents, NetworkHandle};
pub use libp2p::Multiaddr;

/// Entry point for starting the libp2p network actor.
pub struct Network;

impl Network {
    pub async fn start(
        config: NetworkConfig,
    ) -> Result<(NetworkHandle, NetworkEvents), NetworkError> {
        actor::start(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn network_starts_and_stops_without_discovery() {
        let config = NetworkConfig {
            listen_addrs: Vec::new(),
            discovery: DiscoveryMode::None,
        };

        let (handle, _events) = Network::start(config).await.unwrap();
        assert!(!handle.local_node_id().as_str().is_empty());

        handle.shutdown().await;
    }
}
