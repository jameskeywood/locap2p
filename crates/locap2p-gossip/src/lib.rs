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
    /// Integrates an event into local state and disseminates it to peers.
    ///
    /// Implementations emit the event as [`GossipEvent::Event`] when it is
    /// newly integrated. Events that are already known are ignored.
    fn publish(&mut self, event: Event) -> impl Future<Output = Result<(), GossipError>> + Send;

    /// Waits for the next gossip event.
    fn next_event(&mut self) -> impl Future<Output = Option<GossipEvent>> + Send;
}
