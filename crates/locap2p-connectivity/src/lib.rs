mod address;
mod behaviour;
mod client;

use libp2p::{Multiaddr, PeerId};
use locap2p_config::ConnectivityConfig;
use tracing::{debug, info};

pub use behaviour::{Behaviour, BehaviourEvent};
pub use client::RelayEndpoint;

use address::AddressBook;
use client::RelayClient;

pub struct Connectivity {
    addresses: AddressBook,
    relay: RelayClient,
    prefer_direct: bool,
}

impl Connectivity {
    pub fn new(
        local_peer_id: PeerId,
        config: &ConnectivityConfig,
        relays: Vec<RelayEndpoint>,
    ) -> Self {
        Self {
            addresses: AddressBook::new(local_peer_id),
            relay: RelayClient::new(relays, config.reserve_relay),
            prefer_direct: config.prefer_direct,
        }
    }

    pub fn reservation_addrs(&self) -> Vec<Multiaddr> {
        self.relay.reservation_addrs()
    }

    pub fn service_peers(&self) -> impl Iterator<Item = PeerId> + '_ {
        self.relay.service_peers()
    }

    pub fn on_listen_addr(&mut self, address: Multiaddr) {
        self.addresses.add_listen(address);
    }

    pub fn on_expired_listen_addr(&mut self, address: &Multiaddr) {
        self.addresses.remove_listen(address);
    }

    pub fn on_external_addr(&mut self, address: Multiaddr) {
        self.addresses.add_external(address);
    }

    pub fn on_expired_external_addr(&mut self, address: &Multiaddr) {
        self.addresses.remove_external(address);
    }

    pub fn on_observed_addr(&mut self, address: Multiaddr) {
        self.addresses.add_observed(address);
    }

    pub fn announceable_addresses(&self) -> Vec<Multiaddr> {
        self.addresses.announceable_addresses()
    }

    pub fn rank_peer_addresses(
        &self,
        addresses: impl IntoIterator<Item = Multiaddr>,
    ) -> Vec<Multiaddr> {
        self.addresses
            .rank_peer_addresses(addresses, self.prefer_direct)
    }

    pub fn on_behaviour_event(&mut self, event: BehaviourEvent) {
        match event {
            BehaviourEvent::RelayClient(event) => self.relay.on_event(event),
            BehaviourEvent::Dcutr(event) => match event.result {
                Ok(_) => info!(peer = %event.remote_peer_id, "direct connection established"),
                Err(error) => {
                    debug!(peer = %event.remote_peer_id, %error, "direct connection upgrade failed");
                }
            },
        }
    }
}
