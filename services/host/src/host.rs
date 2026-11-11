use std::time::{SystemTime, UNIX_EPOCH};

use locap2p_core::{Event, EventSet, NodeId};
use locap2p_gossip::{Gossip, GossipError, GossipEvent};
use serde_json::Value;

/// Framework node state driven by application payloads from the IPC layer.
pub struct Host<G: Gossip> {
    id: NodeId,
    events: EventSet,
    next_seq: u64,
    gossip: G,
}

impl<G: Gossip> Host<G> {
    /// Creates a host with an empty event set and the given gossip strategy.
    pub fn new(id: NodeId, gossip: G) -> Self {
        Self {
            id,
            events: EventSet::default(),
            next_seq: 0,
            gossip,
        }
    }

    /// Creates, stores and publishes an event for the given application payload.
    pub async fn publish(&mut self, payload: Value) -> Result<Event, GossipError> {
        self.next_seq = self.next_seq.saturating_add(1);
        let event = Event::new(self.id.clone(), self.next_seq, payload, now_nanos());

        self.events.insert(event.clone());
        self.gossip.publish(event.clone()).await?;

        Ok(event)
    }

    /// Inserts an event into local state and returns `true` if it was new.
    pub fn observe(&mut self, event: Event) -> bool {
        self.events.insert(event)
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
