use locap2p_core::Event;
use locap2p_network::{Network, NetworkEvent};
use tracing::warn;

use crate::{Gossip, GossipError, GossipEvent, Topic, codec};

/// Gossipsub-backed implementation of the gossip abstraction.
pub struct GossipsubGossip {
    network: Network,
    topic: Topic,
}

impl GossipsubGossip {
    /// Subscribes to `topic` and returns a gossipsub-backed gossip instance.
    pub fn start(mut network: Network, topic: Topic) -> Result<Self, GossipError> {
        network.subscribe(topic.as_str())?;
        Ok(Self { network, topic })
    }
}

impl Gossip for GossipsubGossip {
    async fn publish(&mut self, event: Event) -> Result<(), GossipError> {
        let data = codec::encode(&event)?;
        self.network.publish(self.topic.as_str(), data)?;
        Ok(())
    }

    async fn next_event(&mut self) -> Option<GossipEvent> {
        loop {
            match self.network.next_event().await? {
                NetworkEvent::Message { source, data, .. } => match codec::decode(&source, &data) {
                    Ok(event) => return Some(GossipEvent::Event(event)),
                    Err(error) => warn!(%error, "discarding invalid gossip message"),
                },
                NetworkEvent::PeerDiscovered(peer) => {
                    return Some(GossipEvent::PeerJoined(peer));
                }
                NetworkEvent::PeerExpired(peer) => {
                    return Some(GossipEvent::PeerLeft(peer));
                }
            }
        }
    }
}
