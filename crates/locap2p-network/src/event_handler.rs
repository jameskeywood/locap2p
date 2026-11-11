use std::collections::{HashMap, HashSet, VecDeque};

use libp2p::{
    Multiaddr, PeerId, Swarm, gossipsub, mdns,
    swarm::{SwarmEvent, dial_opts::DialOpts},
};
use locap2p_core::NodeId;
use tracing::{debug, info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    event::NetworkEvent,
};

#[derive(Default)]
pub(crate) struct SwarmEventHandler {
    discovered_peers: HashSet<PeerId>,
    pending_events: VecDeque<NetworkEvent>,
}

impl SwarmEventHandler {
    pub(crate) fn pop_event(&mut self) -> Option<NetworkEvent> {
        self.pending_events.pop_front()
    }

    pub(crate) fn handle(
        &mut self,
        swarm: &mut Swarm<Behaviour>,
        event: SwarmEvent<BehaviourEvent>,
    ) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                self.handle_discovered_peers(swarm, peers);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                self.handle_expired_peers(peers);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message,
                ..
            })) => {
                let source = message.source.unwrap_or(propagation_source);
                self.pending_events.push_back(NetworkEvent::Message {
                    source: node_id(source),
                    topic: message.topic.to_string(),
                    data: message.data,
                });
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(%address, "listening");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                debug!(%peer_id, "connection established");
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                debug!(%peer_id, "connection closed");
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, ?error, "outgoing connection failed");
            }
            SwarmEvent::IncomingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, ?error, "incoming connection failed");
            }
            _ => {}
        }
    }

    fn handle_discovered_peers(
        &mut self,
        swarm: &mut Swarm<Behaviour>,
        peers: Vec<(PeerId, Multiaddr)>,
    ) {
        let mut addresses_by_peer: HashMap<PeerId, Vec<Multiaddr>> = HashMap::new();
        for (peer_id, address) in peers {
            addresses_by_peer.entry(peer_id).or_default().push(address);
        }

        for (peer_id, addresses) in addresses_by_peer {
            if self.discovered_peers.insert(peer_id) {
                info!(%peer_id, ?addresses, "discovered peer");
                self.pending_events
                    .push_back(NetworkEvent::PeerDiscovered(node_id(peer_id)));
            }

            if swarm.is_connected(&peer_id) {
                continue;
            }

            let dial_opts = DialOpts::peer_id(peer_id)
                .addresses(addresses)
                .allocate_new_port()
                .build();

            if let Err(error) = swarm.dial(dial_opts) {
                debug!(%peer_id, %error, "could not start dial");
            }
        }
    }

    fn handle_expired_peers(&mut self, peers: Vec<(PeerId, Multiaddr)>) {
        for (peer_id, _address) in peers {
            if self.discovered_peers.remove(&peer_id) {
                self.pending_events
                    .push_back(NetworkEvent::PeerExpired(node_id(peer_id)));
            }
        }
    }
}

fn node_id(peer_id: PeerId) -> NodeId {
    NodeId::from(peer_id.to_string())
}
