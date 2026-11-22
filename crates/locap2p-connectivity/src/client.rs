use libp2p::{Multiaddr, PeerId, multiaddr::Protocol, relay};
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayEndpoint {
    pub peer_id: PeerId,
    pub address: Multiaddr,
}

pub(crate) struct RelayClient {
    endpoints: Vec<RelayEndpoint>,
    reserve_relay: bool,
}

impl RelayClient {
    pub(crate) fn new(endpoints: Vec<RelayEndpoint>, reserve_relay: bool) -> Self {
        Self {
            endpoints,
            reserve_relay,
        }
    }

    pub(crate) fn reservation_addrs(&self) -> Vec<Multiaddr> {
        if !self.reserve_relay {
            return Vec::new();
        }

        self.endpoints
            .iter()
            .map(|endpoint| {
                endpoint
                    .address
                    .clone()
                    .with(Protocol::P2p(endpoint.peer_id))
                    .with(Protocol::P2pCircuit)
            })
            .collect()
    }

    pub(crate) fn service_peers(&self) -> impl Iterator<Item = PeerId> + '_ {
        self.endpoints.iter().map(|endpoint| endpoint.peer_id)
    }

    pub(crate) fn on_event(&self, event: relay::client::Event) {
        match event {
            relay::client::Event::ReservationReqAccepted {
                relay_peer_id,
                renewal,
                ..
            } => {
                info!(%relay_peer_id, renewal, "relay reservation accepted");
            }
            relay::client::Event::OutboundCircuitEstablished { relay_peer_id, .. } => {
                debug!(%relay_peer_id, "outbound relay circuit established");
            }
            relay::client::Event::InboundCircuitEstablished { src_peer_id, .. } => {
                debug!(%src_peer_id, "inbound relay circuit established");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer_id() -> PeerId {
        libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id()
    }

    #[test]
    fn builds_reservation_addresses() {
        let relay_peer_id = peer_id();
        let client = RelayClient::new(
            vec![RelayEndpoint {
                peer_id: relay_peer_id,
                address: "/ip4/127.0.0.1/tcp/4001".parse().unwrap(),
            }],
            true,
        );

        assert_eq!(
            client.reservation_addrs(),
            vec![
                format!("/ip4/127.0.0.1/tcp/4001/p2p/{relay_peer_id}/p2p-circuit")
                    .parse()
                    .unwrap()
            ]
        );
    }

    #[test]
    fn skips_reservations_when_disabled() {
        let client = RelayClient::new(
            vec![RelayEndpoint {
                peer_id: peer_id(),
                address: "/ip4/127.0.0.1/tcp/4001".parse().unwrap(),
            }],
            false,
        );

        assert!(client.reservation_addrs().is_empty());
    }

    #[test]
    fn exposes_service_peers() {
        let peer_id = peer_id();
        let client = RelayClient::new(
            vec![RelayEndpoint {
                peer_id,
                address: "/ip4/127.0.0.1/tcp/4001".parse().unwrap(),
            }],
            true,
        );

        assert_eq!(client.service_peers().collect::<Vec<_>>(), vec![peer_id]);
    }
}
