use std::{
    collections::BTreeSet,
    str::FromStr,
    time::{Duration, Instant},
};

use futures::StreamExt;
use libp2p::{Multiaddr, PeerId, Swarm, identify, swarm::SwarmEvent};
use locap2p_config::FrameworkConfig;
use locap2p_connectivity::{Connectivity, RelayEndpoint};
use locap2p_core::NodeId;
use locap2p_discovery::{SeedClient, SeedPeer};
use locap2p_identity::load_or_generate;
use tokio::time::{Instant as TokioInstant, sleep_until};
use tracing::{debug, info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    endpoint::parse_endpoints,
    error::NetworkError,
    event::{NetworkEvent, RequestId},
    event_handler::SwarmEventHandler,
    swarm,
};

/// Owns the libp2p swarm and exposes it as a small pollable API.
pub struct Network {
    swarm: Swarm<Behaviour>,
    node_id: NodeId,
    handler: SwarmEventHandler,
    connectivity: Connectivity,
    seed_client: SeedClient,
}

impl Network {
    /// Builds the network from the application configuration and starts
    /// listening on the configured addresses.
    pub fn start(config: &FrameworkConfig) -> Result<Self, NetworkError> {
        let listen_addrs = config
            .network
            .listen_addrs
            .iter()
            .map(|address| {
                Multiaddr::from_str(address).map_err(|error| {
                    NetworkError::Config(format!("invalid listen address: {error}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let bootstrap = parse_endpoints("bootstrap", &config.discovery.bootstrap)?;
        let relay = parse_endpoints("relay", &config.connectivity.relay)?;
        let sync_protocol = crate::behaviour::session_protocol(
            &config.application.id,
            &config.application.session,
        )?;

        let identity = load_or_generate(&config.identity.path)?;
        let mut swarm = swarm::build(
            identity,
            &listen_addrs,
            sync_protocol,
            Duration::from_millis(config.network.request_timeout_ms),
            config.connectivity.enable_dcutr,
        )?;

        for endpoint in &bootstrap {
            swarm.add_peer_address(endpoint.peer_id, endpoint.address.clone());
        }

        let local_peer_id = *swarm.local_peer_id();
        let node_id = NodeId::from(local_peer_id.to_string());
        let relay_endpoints = relay
            .iter()
            .map(|endpoint| RelayEndpoint {
                peer_id: endpoint.peer_id,
                address: endpoint.address.clone(),
            })
            .collect();
        let connectivity = Connectivity::new(local_peer_id, &config.connectivity, relay_endpoints);

        for address in connectivity.reservation_addrs() {
            swarm
                .listen_on(address)
                .map_err(|error| NetworkError::Listen(error.to_string()))?;
        }

        let seed_client = SeedClient::new(
            &config.application,
            bootstrap.iter().map(|endpoint| endpoint.peer_id),
        );

        Ok(Self {
            swarm,
            node_id,
            handler: SwarmEventHandler::default(),
            connectivity,
            seed_client,
        })
    }

    /// Returns this node's peer identity.
    pub fn local_node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    /// Returns the addresses announced to other peers.
    pub fn local_addresses(&self) -> Vec<String> {
        self.connectivity
            .announceable_addresses()
            .into_iter()
            .map(|address| address.to_string())
            .collect()
    }

    /// Registers addresses learned through gossip membership.
    pub fn register_peer_addresses(
        &mut self,
        peer: &NodeId,
        addresses: &[String],
    ) -> Result<(), NetworkError> {
        let peer_id = PeerId::from_str(peer.as_str())
            .map_err(|error| NetworkError::InvalidPeer(error.to_string()))?;
        let addresses: Vec<Multiaddr> = addresses
            .iter()
            .filter_map(|address| Multiaddr::from_str(address).ok())
            .collect();

        for address in self.connectivity.rank_peer_addresses(addresses) {
            self.swarm.add_peer_address(peer_id, address);
        }

        Ok(())
    }

    /// Sends a request to a peer, dialing it if necessary.
    pub fn request(&mut self, peer: &NodeId, data: Vec<u8>) -> Result<(), NetworkError> {
        let peer = PeerId::from_str(peer.as_str())
            .map_err(|error| NetworkError::InvalidPeer(error.to_string()))?;

        self.swarm.behaviour_mut().sync.send_request(&peer, data);
        Ok(())
    }

    /// Sends a response to an inbound request.
    pub fn respond(&mut self, request_id: RequestId, data: Vec<u8>) -> Result<(), NetworkError> {
        let channel = self
            .handler
            .take_request(request_id)
            .ok_or_else(|| NetworkError::Respond(format!("unknown request id {request_id}")))?;

        self.swarm
            .behaviour_mut()
            .sync
            .send_response(channel, data)
            .map_err(|_| NetworkError::Respond("response channel closed".to_owned()))
    }

    /// Polls the swarm until the next network event is available.
    ///
    /// The network only makes progress while this future is being polled.
    pub async fn next_event(&mut self) -> Option<NetworkEvent> {
        loop {
            if let Some(event) = self.handler.pop_event() {
                return Some(event);
            }

            let addresses = self.connectivity.announceable_addresses();
            let lookups = self.seed_client.poll(Instant::now(), &addresses);
            if !lookups.is_empty() {
                for lookup in lookups {
                    self.swarm
                        .behaviour_mut()
                        .seeds
                        .request_seeds(lookup.bootstrap_peer, lookup.request);
                }
                continue;
            }

            match self.seed_client.next_deadline() {
                Some(deadline) => {
                    tokio::select! {
                        _ = sleep_until(TokioInstant::from_std(deadline)) => {}
                        event = self.swarm.select_next_some() => self.handle_swarm_event(event),
                    }
                }
                None => {
                    let event = self.swarm.select_next_some().await;
                    self.handle_swarm_event(event);
                }
            }
        }
    }

    fn handle_swarm_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Seeds(event)) => {
                let seeds = self.seed_client.on_behaviour_event(event);
                self.apply_seed_peers(seeds);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Connectivity(event)) => {
                self.connectivity.on_behaviour_event(event);
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(%address, "listening");
                self.connectivity.on_listen_addr(address);
            }
            SwarmEvent::ExpiredListenAddr { address, .. } => {
                self.connectivity.on_expired_listen_addr(&address);
            }
            SwarmEvent::ExternalAddrConfirmed { address } => {
                self.connectivity.on_external_addr(address);
            }
            SwarmEvent::ExternalAddrExpired { address } => {
                self.connectivity.on_expired_external_addr(&address);
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
                self.connectivity.on_observed_addr(info.observed_addr);
            }
            event => self.handler.handle(event),
        }
    }

    fn apply_seed_peers(&mut self, seeds: Vec<SeedPeer>) {
        let mut peers = BTreeSet::new();

        for seed in seeds {
            let Ok(peer_id) = PeerId::from_str(&seed.peer_id) else {
                warn!(peer_id = %seed.peer_id, "bootstrap returned an invalid seed peer id");
                continue;
            };

            if peer_id == *self.swarm.local_peer_id() {
                continue;
            }

            let addresses: Vec<Multiaddr> = seed
                .addresses
                .iter()
                .filter_map(|address| Multiaddr::from_str(address).ok())
                .collect();
            let addresses = self.connectivity.rank_peer_addresses(addresses);

            if addresses.is_empty() {
                continue;
            }

            for address in addresses {
                self.swarm.add_peer_address(peer_id, address);
            }
            peers.insert(node_id(peer_id));
        }

        if !peers.is_empty() {
            self.handler
                .push_event(NetworkEvent::SeedPeers(peers.into_iter().collect()));
        }
    }
}

fn node_id(peer_id: PeerId) -> NodeId {
    NodeId::from(peer_id.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use locap2p_config::{
        ApplicationConfig, ApplicationId, ConnectivityConfig, DiscoveryConfig, GossipConfig,
        IdentityConfig, NetworkConfig, SessionId,
    };

    use super::*;

    fn config(path: PathBuf) -> FrameworkConfig {
        FrameworkConfig {
            schema_version: locap2p_config::CONFIG_SCHEMA_VERSION,
            application: ApplicationConfig {
                id: ApplicationId::new("locap2p-test/1").unwrap(),
                session: SessionId::new("test").unwrap(),
            },
            identity: IdentityConfig { path },
            network: NetworkConfig {
                listen_addrs: Vec::new(),
                request_timeout_ms: 1_000,
            },
            discovery: DiscoveryConfig {
                bootstrap: Vec::new(),
            },
            connectivity: ConnectivityConfig::default(),
            gossip: GossipConfig::default(),
        }
    }

    #[tokio::test]
    async fn network_starts() {
        let directory = tempfile::tempdir().unwrap();
        let config = config(directory.path().join("node.key"));

        let network = Network::start(&config).unwrap();
        assert!(!network.local_node_id().as_str().is_empty());
    }
}
