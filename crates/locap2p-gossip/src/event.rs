use locap2p_core::{Event, NodeId};

/// Event surfaced by the gossip layer.
#[derive(Debug, Clone)]
pub enum GossipEvent {
    Event(Event),
    PeerJoined(NodeId),
    PeerLeft(NodeId),
}
