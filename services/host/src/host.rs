use std::time::{SystemTime, UNIX_EPOCH};

use locap2p_core::{Event, NodeId};
use locap2p_gossip::{Gossip, GossipError, GossipEvent};
use serde_json::Value;

/// Framework node that creates locally authored events and bridges them to gossip.
pub struct Host<G: Gossip> {
    id: NodeId,
    next_seq: u64,
    gossip: G,
}

impl<G: Gossip> Host<G> {
    /// Creates a host with the given gossip strategy.
    pub fn new(id: NodeId, gossip: G) -> Self {
        Self {
            id,
            next_seq: 0,
            gossip,
        }
    }

    /// Creates an event for the given application payload.
    pub fn create_event(&mut self, payload: Value) -> Event {
        self.next_seq = self.next_seq.saturating_add(1);
        Event::new(self.id.clone(), self.next_seq, payload, now_nanos())
    }

    /// Integrates and publishes an event through the gossip layer.
    pub async fn publish(&mut self, event: Event) -> Result<(), GossipError> {
        self.gossip.publish(event).await
    }

    /// Waits for the next gossip event.
    pub async fn next_event(&mut self) -> Option<GossipEvent> {
        self.gossip.next_event().await
    }
}

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default()
}
