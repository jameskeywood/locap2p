use std::collections::{HashMap, HashSet, VecDeque};

use libp2p::{
    Multiaddr, PeerId, Swarm, mdns,
    request_response::{self, Message, ResponseChannel},
    swarm::SwarmEvent,
};
use locap2p_core::NodeId;
use tracing::{debug, info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    event::{NetworkEvent, RequestId},
};

#[derive(Default)]
pub(crate) struct SwarmEventHandler {
    discovered_peers: HashSet<PeerId>,
    pending_requests: HashMap<RequestId, ResponseChannel<Vec<u8>>>,
    next_request_id: u64,
    pending_events: VecDeque<NetworkEvent>,
}

impl SwarmEventHandler {
    pub(crate) fn pop_event(&mut self) -> Option<NetworkEvent> {
        self.pending_events.pop_front()
    }

    pub(crate) fn take_request(
        &mut self,
        request_id: RequestId,
    ) -> Option<ResponseChannel<Vec<u8>>> {
        self.pending_requests.remove(&request_id)
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
            SwarmEvent::Behaviour(BehaviourEvent::Sync(request_response::Event::Message {
                peer,
                message,
                ..
            })) => {
                self.handle_sync_message(peer, message);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Sync(
                request_response::Event::OutboundFailure { peer, error, .. },
            )) => {
                warn!(%peer, %error, "sync request failed");
                self.pending_events.push_back(NetworkEvent::RequestFailed {
                    peer: node_id(peer),
                    error: error.to_string(),
                });
            }
            SwarmEvent::Behaviour(BehaviourEvent::Sync(
                request_response::Event::InboundFailure { peer, error, .. },
            )) => {
                warn!(%peer, %error, "sync response failed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Sync(
                request_response::Event::ResponseSent { peer, .. },
            )) => {
                debug!(%peer, "sync response sent");
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

    fn handle_sync_message(&mut self, peer: PeerId, message: Message<Vec<u8>, Vec<u8>>) {
        match message {
            Message::Request {
                request, channel, ..
            } => {
                let request_id = RequestId::new(self.next_request_id);
                self.next_request_id = self.next_request_id.wrapping_add(1);
                self.pending_requests.insert(request_id, channel);
                self.pending_events.push_back(NetworkEvent::Request {
                    peer: node_id(peer),
                    request_id,
                    data: request,
                });
            }
            Message::Response { response, .. } => {
                self.pending_events.push_back(NetworkEvent::Response {
                    peer: node_id(peer),
                    data: response,
                });
            }
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
            for address in &addresses {
                swarm.add_peer_address(peer_id, address.clone());
            }

            if self.discovered_peers.insert(peer_id) {
                info!(%peer_id, addresses = addresses.len(), "discovered peer");
                self.pending_events
                    .push_back(NetworkEvent::PeerDiscovered(node_id(peer_id)));
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
