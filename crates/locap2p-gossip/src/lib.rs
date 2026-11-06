use std::future::Future;

use locap2p_core::Event;

mod codec;
mod error;
mod event;
mod gossipsub;
mod topic;

pub use error::{CodecError, GossipError};
pub use event::GossipEvent;
pub use gossipsub::GossipsubGossip;
pub use topic::{DEFAULT_TOPIC, Topic};

/// Abstraction over an event dissemination strategy.
pub trait Gossip: Send {
    fn publish(&mut self, event: Event) -> impl Future<Output = Result<(), GossipError>> + Send;

    fn next_event(&mut self) -> impl Future<Output = Option<GossipEvent>> + Send;
}
