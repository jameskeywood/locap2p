//! Core domain types and CRDT state for locap2p.

mod event;
mod event_set;
mod id;
pub mod membership;

pub use event::{Event, EventError};
pub use event_set::EventSet;
pub use id::{EventId, NodeId};
