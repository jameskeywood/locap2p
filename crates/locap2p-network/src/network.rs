use futures::StreamExt;
use libp2p::{Swarm, gossipsub};
use locap2p_core::NodeId;

use crate::{
    behaviour::Behaviour, config::NetworkConfig, error::NetworkError, event::NetworkEvent,
    event_handler::SwarmEventHandler, swarm,
};

/// Owns the libp2p swarm and exposes it as a small pollable API.
pub struct Network {
    swarm: Swarm<Behaviour>,
    node_id: NodeId,
    handler: SwarmEventHandler,
}

impl Network {
    /// Builds the network and starts listening on the configured addresses.
    pub fn start(config: NetworkConfig) -> Result<Self, NetworkError> {
        let swarm = swarm::build(config)?;
        let node_id = NodeId::from(swarm.local_peer_id().to_string());

        Ok(Self {
            swarm,
            node_id,
            handler: SwarmEventHandler::default(),
        })
    }

    /// Returns this node's peer identity.
    pub fn local_node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    /// Subscribes to a gossipsub topic.
    pub fn subscribe(&mut self, topic: &str) -> Result<(), NetworkError> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&gossipsub::IdentTopic::new(topic))
            .map(|_| ())
            .map_err(|error| NetworkError::Subscribe(error.to_string()))
    }

    /// Publishes raw bytes to a gossipsub topic.
    pub fn publish(&mut self, topic: &str, data: Vec<u8>) -> Result<(), NetworkError> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(gossipsub::IdentTopic::new(topic), data)
            .map(|_| ())
            .map_err(|error| NetworkError::Publish(error.to_string()))
    }

    /// Polls the swarm until the next network event is available.
    ///
    /// The network only makes progress while this future is being polled.
    pub async fn next_event(&mut self) -> Option<NetworkEvent> {
        loop {
            if let Some(event) = self.handler.pop_event() {
                return Some(event);
            }

            let event = self.swarm.select_next_some().await;
            self.handler.handle(&mut self.swarm, event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn network_starts() {
        let config = NetworkConfig {
            listen_addrs: Vec::new(),
        };

        let network = Network::start(config).unwrap();
        assert!(!network.local_node_id().as_str().is_empty());
    }
}
