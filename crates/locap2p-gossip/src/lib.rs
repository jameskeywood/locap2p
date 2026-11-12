//! Anti-entropy gossip implementation.

use std::future::Future;

use locap2p_core::Event;

mod anti_entropy;
mod codec;
mod error;
mod event;
mod message;

pub use anti_entropy::{AntiEntropyConfig, AntiEntropyGossip};
pub use error::{CodecError, GossipError};
pub use event::GossipEvent;

/// Abstraction over an event dissemination strategy.
pub trait Gossip: Send {
    /// Publishes an event to the gossip network.
    fn publish(&mut self, event: Event) -> impl Future<Output = Result<(), GossipError>> + Send;

    /// Waits for the next gossip event.
    fn next_event(&mut self) -> impl Future<Output = Option<GossipEvent>> + Send;
}
