use locap2p_core::NodeId;

/// Event emitted by the network actor.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Message {
        source: NodeId,
        topic: String,
        data: Vec<u8>,
    },
    PeerDiscovered(NodeId),
    PeerExpired(NodeId),
}
