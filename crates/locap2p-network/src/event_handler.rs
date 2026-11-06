use std::collections::{HashSet, VecDeque};

use libp2p::{
    Multiaddr, PeerId, Swarm, gossipsub, mdns,
    swarm::{SwarmEvent, dial_opts::DialOpts},
};
use locap2p_core::NodeId;
use tracing::{debug, info};

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
                self.handle_expired_peers(swarm, peers);
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
            _ => {}
        }
    }

    fn handle_discovered_peers(
        &mut self,
        swarm: &mut Swarm<Behaviour>,
        peers: Vec<(PeerId, Multiaddr)>,
    ) {
        for (peer_id, address) in peers {
            if !self.discovered_peers.insert(peer_id) {
                continue;
            }

            info!(%peer_id, %address, "discovered peer");
            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

            let dial = DialOpts::peer_id(peer_id).addresses(vec![address]).build();
            if let Err(error) = swarm.dial(dial) {
                debug!(%peer_id, %error, "failed to dial discovered peer");
            }

            self.pending_events
                .push_back(NetworkEvent::PeerDiscovered(node_id(peer_id)));
        }
    }

    fn handle_expired_peers(
        &mut self,
        swarm: &mut Swarm<Behaviour>,
        peers: Vec<(PeerId, Multiaddr)>,
    ) {
        for (peer_id, _address) in peers {
            if !self.discovered_peers.remove(&peer_id) {
                continue;
            }

            swarm
                .behaviour_mut()
                .gossipsub
                .remove_explicit_peer(&peer_id);
            self.pending_events
                .push_back(NetworkEvent::PeerExpired(node_id(peer_id)));
        }
    }
}

fn node_id(peer_id: PeerId) -> NodeId {
    NodeId::from(peer_id.to_string())
}
