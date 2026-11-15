use locap2p_core::{Event, EventId};
use serde::{Deserialize, Serialize};

/// Messages exchanged by the anti-entropy protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum SyncMessage {
    /// Requests every event the receiver has that the sender does not.
    Pull { have: Vec<EventId> },
    /// Returns events missing from the requester's digest.
    PullResponse { events: Vec<Event> },
    /// Eagerly delivers new events to a peer.
    Push { events: Vec<Event> },
    /// Acknowledges an eager push.
    PushAck,
}
