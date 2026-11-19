use std::{
    collections::{HashMap, HashSet, VecDeque},
    str::FromStr,
    time::{Duration, Instant},
};

use libp2p::{
    Multiaddr, PeerId, Swarm, identify,
    multiaddr::Protocol,
    relay,
    request_response::{self, Message, ResponseChannel},
    swarm::SwarmEvent,
};
use locap2p_config::{ApplicationId, SessionId};
use locap2p_core::NodeId;
use locap2p_discovery::{BootstrapRequest, BootstrapResponse, PeerInfo};
use tracing::{debug, info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    endpoint::ParsedEndpoint,
    event::{NetworkEvent, RequestId},
};

pub(crate) struct SessionSettings {
    pub(crate) application: ApplicationId,
    pub(crate) session: SessionId,
    pub(crate) membership_ttl: Duration,
    pub(crate) expiry_grace: Duration,
    pub(crate) prefer_direct: bool,
}

pub(crate) struct SwarmEventHandler {
    local_peer_id: PeerId,
    application: ApplicationId,
    session: SessionId,
    bootstrap: Vec<ParsedEndpoint>,
    excluded_peers: HashSet<PeerId>,
    membership_ttl: Duration,
    expiry_grace: Duration,
    prefer_direct: bool,
    discovered_peers: HashSet<PeerId>,
    last_seen: HashMap<PeerId, Instant>,
    local_addrs: Vec<Multiaddr>,
    pending_requests: HashMap<RequestId, ResponseChannel<Vec<u8>>>,
    next_request_id: u64,
    pending_announces: HashSet<PeerId>,
    pending_events: VecDeque<NetworkEvent>,
}

impl SwarmEventHandler {
    pub(crate) fn new(
        local_peer_id: PeerId,
        bootstrap: Vec<ParsedEndpoint>,
        relay: &[ParsedEndpoint],
        settings: SessionSettings,
    ) -> Self {
        let mut excluded_peers: HashSet<PeerId> =
            bootstrap.iter().map(|endpoint| endpoint.peer_id).collect();
        excluded_peers.extend(relay.iter().map(|endpoint| endpoint.peer_id));

        Self {
            local_peer_id,
            application: settings.application,
            session: settings.session,
            bootstrap,
            excluded_peers,
            membership_ttl: settings.membership_ttl,
            expiry_grace: settings.expiry_grace,
            prefer_direct: settings.prefer_direct,
            discovered_peers: HashSet::new(),
            last_seen: HashMap::new(),
            local_addrs: Vec::new(),
            pending_requests: HashMap::new(),
            next_request_id: 0,
            pending_announces: HashSet::new(),
            pending_events: VecDeque::new(),
        }
    }

    pub(crate) fn pop_event(&mut self) -> Option<NetworkEvent> {
        self.pending_events.pop_front()
    }

    pub(crate) fn has_bootstrap(&self) -> bool {
        !self.bootstrap.is_empty()
    }

    pub(crate) fn take_request(
        &mut self,
        request_id: RequestId,
    ) -> Option<ResponseChannel<Vec<u8>>> {
        self.pending_requests.remove(&request_id)
    }

    pub(crate) fn announce(&mut self, swarm: &mut Swarm<Behaviour>) {
        self.expire_peers();

        if self.bootstrap.is_empty() || self.pending_announces.len() >= self.bootstrap.len() {
            return;
        }

        let addresses = self.announce_addresses();
        if addresses.is_empty() {
            debug!("skipping bootstrap announce: no local addresses yet");
            return;
        }

        let request = BootstrapRequest {
            application: self.application.clone(),
            session: self.session.clone(),
            addresses: addresses.iter().map(ToString::to_string).collect(),
            ttl_ms: self.membership_ttl.as_millis() as u64,
        };

        for endpoint in &self.bootstrap {
            if self.pending_announces.contains(&endpoint.peer_id) {
                continue;
            }

            swarm
                .behaviour_mut()
                .bootstrap
                .send_request(&endpoint.peer_id, request.clone());
            self.pending_announces.insert(endpoint.peer_id);
        }
    }

    pub(crate) fn handle(
        &mut self,
        swarm: &mut Swarm<Behaviour>,
        event: SwarmEvent<BehaviourEvent>,
    ) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Bootstrap(
                request_response::Event::Message { peer, message, .. },
            )) => {
                self.handle_bootstrap_message(swarm, peer, message);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Bootstrap(
                request_response::Event::OutboundFailure { peer, error, .. },
            )) => {
                self.pending_announces.remove(&peer);
                warn!(%peer, %error, "bootstrap request failed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Bootstrap(
                request_response::Event::InboundFailure { peer, error, .. },
            )) => {
                warn!(%peer, %error, "bootstrap response failed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Bootstrap(
                request_response::Event::ResponseSent { peer, .. },
            )) => {
                debug!(%peer, "bootstrap response sent");
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
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            })) => {
                debug!(
                    %peer_id,
                    observed = %info.observed_addr,
                    protocols = info.protocols.len(),
                    "identify information received"
                );
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Error {
                peer_id,
                error,
                ..
            })) => {
                debug!(%peer_id, %error, "identify failed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::RelayClient(event)) => {
                self.handle_relay_event(event);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Dcutr(event)) => match event.result {
                Ok(_) => info!(peer = %event.remote_peer_id, "direct connection established"),
                Err(error) => {
                    debug!(peer = %event.remote_peer_id, %error, "direct connection upgrade failed")
                }
            },
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(%address, "listening");
                self.record_local_address(address);
            }
            SwarmEvent::ExpiredListenAddr { address, .. } => {
                self.local_addrs.retain(|candidate| candidate != &address);
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                debug!(%peer_id, ?endpoint, "connection established");
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

    fn handle_bootstrap_message(
        &mut self,
        swarm: &mut Swarm<Behaviour>,
        peer: PeerId,
        message: Message<BootstrapRequest, BootstrapResponse>,
    ) {
        if !self.is_bootstrap_peer(peer) {
            debug!(%peer, "ignoring bootstrap message from unexpected peer");
            return;
        }

        match message {
            Message::Response { response, .. } => {
                self.pending_announces.remove(&peer);
                self.apply_bootstrap_response(swarm, response);
            }
            Message::Request { .. } => {
                debug!(%peer, "ignoring unexpected inbound bootstrap request");
            }
        }
    }

    fn apply_bootstrap_response(
        &mut self,
        swarm: &mut Swarm<Behaviour>,
        response: BootstrapResponse,
    ) {
        let now = Instant::now();

        for peer in response.peers {
            if !self.register_peer(swarm, peer, now) {
                continue;
            }
        }
    }

    fn register_peer(
        &mut self,
        swarm: &mut Swarm<Behaviour>,
        peer: PeerInfo,
        now: Instant,
    ) -> bool {
        let Ok(peer_id) = PeerId::from_str(&peer.peer_id) else {
            warn!(peer_id = %peer.peer_id, "bootstrap returned an invalid peer id");
            return false;
        };

        if peer_id == self.local_peer_id || self.excluded_peers.contains(&peer_id) {
            return false;
        }

        let mut addresses: Vec<Multiaddr> = peer
            .addresses
            .iter()
            .filter_map(|address| Multiaddr::from_str(address).ok())
            .filter(|address| !has_unspecified_address(address))
            .collect();

        if addresses.is_empty() {
            debug!(%peer_id, "bootstrap returned a peer without usable addresses");
            return false;
        }

        addresses.sort_by_key(is_relayed);

        for address in addresses {
            swarm.add_peer_address(peer_id, address);
        }

        let newly_discovered = self.discovered_peers.insert(peer_id);
        self.last_seen.insert(peer_id, now);

        if newly_discovered {
            info!(%peer_id, "discovered peer");
            self.pending_events
                .push_back(NetworkEvent::PeerDiscovered(node_id(peer_id)));
        }

        true
    }

    fn expire_peers(&mut self) {
        let now = Instant::now();
        let expiry = self.membership_ttl + self.expiry_grace;

        let expired: Vec<PeerId> = self
            .last_seen
            .iter()
            .filter(|(_, last_seen)| now.duration_since(**last_seen) > expiry)
            .map(|(peer_id, _)| *peer_id)
            .collect();

        for peer_id in expired {
            self.last_seen.remove(&peer_id);
            if self.discovered_peers.remove(&peer_id) {
                info!(%peer_id, "peer expired");
                self.pending_events
                    .push_back(NetworkEvent::PeerExpired(node_id(peer_id)));
            }
        }
    }

    fn announce_addresses(&self) -> Vec<Multiaddr> {
        let direct: Vec<Multiaddr> = self
            .local_addrs
            .iter()
            .filter(|address| !is_relayed(address) && !has_unspecified_address(address))
            .cloned()
            .collect();
        let relayed: Vec<Multiaddr> = self
            .local_addrs
            .iter()
            .filter(|address| is_relayed(address))
            .cloned()
            .collect();

        let mut addresses = if self.prefer_direct || relayed.is_empty() {
            direct
        } else {
            Vec::new()
        };
        addresses.extend(relayed);
        addresses.sort();
        addresses.dedup();
        addresses
    }

    fn record_local_address(&mut self, address: Multiaddr) {
        if !self
            .local_addrs
            .iter()
            .any(|candidate| candidate == &address)
        {
            self.local_addrs.push(address);
        }
    }

    fn handle_relay_event(&self, event: relay::client::Event) {
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

    fn is_bootstrap_peer(&self, peer: PeerId) -> bool {
        self.bootstrap
            .iter()
            .any(|endpoint| endpoint.peer_id == peer)
    }
}

fn node_id(peer_id: PeerId) -> NodeId {
    NodeId::from(peer_id.to_string())
}

fn is_relayed(address: &Multiaddr) -> bool {
    address
        .iter()
        .any(|protocol| matches!(protocol, Protocol::P2pCircuit))
}

fn has_unspecified_address(address: &Multiaddr) -> bool {
    address.iter().any(|protocol| match protocol {
        Protocol::Ip4(address) => address.is_unspecified(),
        Protocol::Ip6(address) => address.is_unspecified(),
        _ => false,
    })
}
