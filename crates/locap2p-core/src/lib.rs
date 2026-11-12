//! Core domain types and CRDT state for locap2p.

mod event;
mod event_set;
mod id;
mod store;

pub use event::{Event, EventError};
pub use event_set::EventSet;
pub use id::{EventId, NodeId};
pub use store::EventStore;
