use std::time::{SystemTime, UNIX_EPOCH};

use locap2p_core::{Event, EventSet, NodeId};
use locap2p_gossip::{Gossip, GossipError, GossipEvent};

/// Application node composed from the core state and a gossip strategy.
pub struct Node<G: Gossip> {
    id: NodeId,
    state: EventSet,
    gossip: G,
}

impl<G: Gossip> Node<G> {
    pub fn new(id: NodeId, gossip: G) -> Self {
        Self {
            id,
            state: EventSet::default(),
            gossip,
        }
    }

    pub fn create_event(&self, text: impl Into<String>) -> Event {
        Event::new(self.id.clone(), text, now_nanos())
    }

    pub fn observe(&mut self, event: Event) -> bool {
        self.state.insert(event)
    }

    pub async fn publish(&mut self, event: &Event) -> Result<(), GossipError> {
        self.gossip.publish(event.clone()).await
    }

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
