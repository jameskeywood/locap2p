use locap2p_core::NodeId;

/// Event emitted by the network layer.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A message received on a topic.
    Message {
        source: NodeId,
        topic: String,
        data: Vec<u8>,
    },
    /// A peer was discovered by mDNS.
    PeerDiscovered(NodeId),
    /// A discovered peer expired.
    PeerExpired(NodeId),
}
