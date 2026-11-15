use std::fmt;

use locap2p_core::NodeId;

/// Opaque identifier for an inbound request awaiting a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(u64);

impl RequestId {
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Event emitted by the network layer.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A peer was discovered by mDNS.
    PeerDiscovered(NodeId),
    /// A discovered peer expired.
    PeerExpired(NodeId),
    /// A peer sent a request that must be answered.
    Request {
        peer: NodeId,
        request_id: RequestId,
        data: Vec<u8>,
    },
    /// A peer answered one of our requests.
    Response { peer: NodeId, data: Vec<u8> },
    /// An outbound request failed.
    RequestFailed { peer: NodeId, error: String },
}
