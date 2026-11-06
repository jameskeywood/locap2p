//! libp2p networking and local peer discovery.

mod behaviour;
mod config;
mod error;
mod event;
mod event_handler;
mod network;
mod swarm;

pub use config::NetworkConfig;
pub use error::NetworkError;
pub use event::NetworkEvent;
pub use libp2p::Multiaddr;
pub use network::Network;
