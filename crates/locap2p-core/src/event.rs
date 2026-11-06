use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::id::{EventId, NodeId};

/// A single immutable application event observed by a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    id: EventId,
    author: NodeId,
    payload: String,
    timestamp_nanos: u64,
}

impl Event {
    pub fn new(author: NodeId, payload: impl Into<String>, timestamp_nanos: u64) -> Self {
        let payload = payload.into();
        let id = compute_event_id(&author, &payload, timestamp_nanos);

        Self {
            id,
            author,
            payload,
            timestamp_nanos,
        }
    }

    pub fn id(&self) -> &EventId {
        &self.id
    }

    pub fn author(&self) -> &NodeId {
        &self.author
    }

    pub fn payload(&self) -> &str {
        &self.payload
    }

    pub fn timestamp_nanos(&self) -> u64 {
        self.timestamp_nanos
    }

    pub fn verify_id(&self) -> Result<(), EventError> {
        let expected = compute_event_id(&self.author, &self.payload, self.timestamp_nanos);

        if self.id == expected {
            Ok(())
        } else {
            Err(EventError::InvalidId)
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EventError {
    #[error("event id does not match the event content")]
    InvalidId,
}

fn compute_event_id(author: &NodeId, payload: &str, timestamp_nanos: u64) -> EventId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(author.as_str().as_bytes());
    hasher.update(&timestamp_nanos.to_le_bytes());
    hasher.update(&(payload.len() as u64).to_le_bytes());
    hasher.update(payload.as_bytes());

    EventId::from_hash(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id() -> NodeId {
        NodeId::from("node-a")
    }

    #[test]
    fn event_id_is_deterministic() {
        let first = Event::new(node_id(), "hello", 10);
        let second = Event::new(node_id(), "hello", 10);

        assert_eq!(first.id(), second.id());
        first.verify_id().unwrap();
    }

    #[test]
    fn event_id_changes_with_content() {
        let base = Event::new(node_id(), "hello", 10);

        assert_ne!(base.id(), Event::new(node_id(), "hello", 11).id());
        assert_ne!(
            base.id(),
            Event::new(NodeId::from("node-b"), "hello", 10).id()
        );
        assert_ne!(base.id(), Event::new(node_id(), "goodbye", 10).id());
    }

    #[test]
    fn tampered_event_fails_verification() {
        let mut event = Event::new(node_id(), "hello", 10);
        event.payload = "tampered".to_owned();

        assert_eq!(event.verify_id(), Err(EventError::InvalidId));
    }
}
