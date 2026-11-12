use locap2p_core::{Event, NodeId};

/// Event surfaced by the gossip layer.
#[derive(Debug, Clone)]
pub enum GossipEvent {
    /// A newly merged application event.
    Event(Event),
    /// A peer joined the gossip network.
    PeerJoined(NodeId),
    /// A peer left the gossip network.
    PeerLeft(NodeId),
}
