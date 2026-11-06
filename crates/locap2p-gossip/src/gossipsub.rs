use locap2p_core::Event;
use locap2p_network::{NetworkEvent, NetworkEvents, NetworkHandle};
use tracing::warn;

use crate::{Gossip, GossipError, GossipEvent, Topic, codec};

/// Gossipsub-backed implementation of the gossip abstraction.
pub struct GossipsubGossip {
    handle: NetworkHandle,
    events: NetworkEvents,
    topic: Topic,
}

impl GossipsubGossip {
    pub async fn start(
        handle: NetworkHandle,
        events: NetworkEvents,
        topic: Topic,
    ) -> Result<Self, GossipError> {
        handle.subscribe(topic.as_str()).await?;
        Ok(Self {
            handle,
            events,
            topic,
        })
    }
}

impl Gossip for GossipsubGossip {
    async fn publish(&mut self, event: Event) -> Result<(), GossipError> {
        let data = codec::encode(&event)?;
        self.handle.publish(self.topic.as_str(), data).await?;
        Ok(())
    }

    async fn next_event(&mut self) -> Option<GossipEvent> {
        loop {
            match self.events.next().await? {
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
