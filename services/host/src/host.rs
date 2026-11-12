use std::time::{SystemTime, UNIX_EPOCH};

use locap2p_core::{Event, EventStore, NodeId};
use locap2p_gossip::{Gossip, GossipError, GossipEvent};
use serde_json::Value;

/// Framework node state driven by application payloads from the IPC layer.
pub struct Host<G: Gossip> {
    id: NodeId,
    store: EventStore,
    next_seq: u64,
    gossip: G,
}

impl<G: Gossip> Host<G> {
    /// Creates a host with an empty event store and the given gossip strategy.
    pub fn new(id: NodeId, store: EventStore, gossip: G) -> Self {
        Self {
            id,
            store,
            next_seq: 0,
            gossip,
        }
    }

    /// Creates and stores an event for the given application payload.
    pub fn create_event(&mut self, payload: Value) -> Event {
        self.next_seq = self.next_seq.saturating_add(1);
        let event = Event::new(self.id.clone(), self.next_seq, payload, now_nanos());

        self.store.insert(event.clone());
        event
    }

    /// Publishes an event to the gossip network.
    pub async fn publish(&mut self, event: Event) -> Result<(), GossipError> {
        self.gossip.publish(event).await
    }

    /// Inserts an event into local state and returns `true` if it was new.
    pub fn observe(&mut self, event: Event) -> bool {
        self.store.insert(event)
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
