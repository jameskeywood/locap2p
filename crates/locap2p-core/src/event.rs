use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::id::{EventId, NodeId};

/// A single immutable application event observed by a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    id: EventId,
    author: NodeId,
    seq: u64,
    timestamp_nanos: u64,
    payload: Value,
}

impl Event {
    /// Creates an event and computes its content-addressed identifier.
    pub fn new(author: NodeId, seq: u64, payload: Value, timestamp_nanos: u64) -> Self {
        let id = compute_event_id(&author, seq, &payload, timestamp_nanos);

        Self {
            id,
            author,
            seq,
            timestamp_nanos,
            payload,
        }
    }

    pub fn id(&self) -> &EventId {
        &self.id
    }

    pub fn author(&self) -> &NodeId {
        &self.author
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn timestamp_nanos(&self) -> u64 {
        self.timestamp_nanos
    }

    pub fn payload(&self) -> &Value {
        &self.payload
    }

    /// Recomputes the event's content-addressed identifier and verifies it.
    pub fn verify_id(&self) -> Result<(), EventError> {
        let expected =
            compute_event_id(&self.author, self.seq, &self.payload, self.timestamp_nanos);

        if self.id == expected {
            Ok(())
        } else {
            Err(EventError::InvalidId)
        }
    }
}

/// Errors returned when validating an event.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EventError {
    /// The event ID does not match the event content.
    #[error("event id does not match the event content")]
    InvalidId,
}

fn compute_event_id(author: &NodeId, seq: u64, payload: &Value, timestamp_nanos: u64) -> EventId {
    let payload = payload.to_string();
    let mut hasher = blake3::Hasher::new();
    hasher.update(author.as_str().as_bytes());
    hasher.update(&seq.to_le_bytes());
    hasher.update(&timestamp_nanos.to_le_bytes());
    hasher.update(&(payload.len() as u64).to_le_bytes());
    hasher.update(payload.as_bytes());

    EventId::from_hash(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn node_id() -> NodeId {
        NodeId::from("node-a")
    }

    fn event(seq: u64, payload: Value, timestamp_nanos: u64) -> Event {
        Event::new(node_id(), seq, payload, timestamp_nanos)
    }

    #[test]
    fn event_id_is_deterministic() {
        let first = event(1, json!({ "x": 1, "y": 2 }), 10);
        let second = event(1, json!({ "x": 1, "y": 2 }), 10);

        assert_eq!(first.id(), second.id());
        first.verify_id().unwrap();
    }

    #[test]
    fn event_id_changes_with_content() {
        let base = event(1, json!({ "x": 1, "y": 2 }), 10);
        let other_author = Event::new(NodeId::from("node-b"), 1, json!({ "x": 1, "y": 2 }), 10);

        assert_ne!(base.id(), event(1, json!({ "x": 1, "y": 2 }), 11).id());
        assert_ne!(base.id(), event(2, json!({ "x": 1, "y": 2 }), 10).id());
        assert_ne!(base.id(), event(1, json!({ "x": 2, "y": 2 }), 10).id());
        assert_ne!(base.id(), other_author.id());
    }

    #[test]
    fn tampered_event_fails_verification() {
        let mut event = event(1, json!({ "x": 1, "y": 2 }), 10);
        event.payload = json!({ "x": 9, "y": 9 });

        assert_eq!(event.verify_id(), Err(EventError::InvalidId));
    }
}
