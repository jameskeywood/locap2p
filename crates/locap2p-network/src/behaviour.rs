use std::time::Duration;

use libp2p::{gossipsub, identity::Keypair, mdns, swarm::NetworkBehaviour};

#[derive(NetworkBehaviour)]
pub(crate) struct Behaviour {
    pub(crate) gossipsub: gossipsub::Behaviour,
    pub(crate) mdns: mdns::tokio::Behaviour,
}

impl Behaviour {
    pub(crate) fn new(key: &Keypair) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let message_id_fn = |message: &gossipsub::Message| {
            gossipsub::MessageId::from(blake3::hash(&message.data).to_hex().to_string())
        };

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .build()
            .map_err(std::io::Error::other)?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(key.clone()),
            gossipsub_config,
        )
        .map_err(std::io::Error::other)?;

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
            .map_err(std::io::Error::other)?;

        Ok(Self { gossipsub, mdns })
    }
}
