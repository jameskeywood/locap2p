use std::{str::FromStr, time::Duration};

use futures::StreamExt;
use libp2p::{Multiaddr, PeerId, Swarm, multiaddr::Protocol, swarm::SwarmEvent};
use locap2p_config::FrameworkConfig;
use locap2p_core::NodeId;
use locap2p_identity::load_or_generate;
use tokio::time::{Instant as TokioInstant, sleep_until};

use crate::{
    behaviour::Behaviour,
    endpoint::parse_endpoints,
    error::NetworkError,
    event::{NetworkEvent, RequestId},
    event_handler::{SessionSettings, SwarmEventHandler},
    swarm,
};

/// Owns the libp2p swarm and exposes it as a small pollable API.
pub struct Network {
    swarm: Swarm<Behaviour>,
    node_id: NodeId,
    handler: SwarmEventHandler,
    announce_interval: Option<Duration>,
    next_announce: Option<TokioInstant>,
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

        if config.connectivity.reserve_relay && !relay.is_empty() {
            for endpoint in &relay {
                let address = endpoint
                    .address
                    .clone()
                    .with(Protocol::P2p(endpoint.peer_id))
                    .with(Protocol::P2pCircuit);
                swarm
                    .listen_on(address)
                    .map_err(|error| NetworkError::Listen(error.to_string()))?;
            }
        }

        let local_peer_id = *swarm.local_peer_id();
        let node_id = NodeId::from(local_peer_id.to_string());
        let handler = SwarmEventHandler::new(
            local_peer_id,
            bootstrap,
            &relay,
            SessionSettings {
                application: config.application.id.clone(),
                session: config.application.session.clone(),
                membership_ttl: config.discovery.membership_ttl(),
                expiry_grace: config.discovery.expiry_grace(),
                prefer_direct: config.connectivity.prefer_direct,
            },
        );

        let announce_interval = handler.has_bootstrap().then(|| {
            let interval = config.discovery.membership_ttl() / 3;
            interval.max(Duration::from_secs(1))
        });
        let next_announce = announce_interval.map(|_| TokioInstant::now());

        Ok(Self {
            swarm,
            node_id,
            handler,
            announce_interval,
            next_announce,
        })
    }

    /// Returns this node's peer identity.
    pub fn local_node_id(&self) -> NodeId {
        self.node_id.clone()
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

            match (self.next_announce, self.announce_interval) {
                (Some(next_announce), Some(announce_interval)) => {
                    tokio::select! {
                        _ = sleep_until(next_announce) => {
                            self.handler.announce(&mut self.swarm);
                            self.next_announce = Some(TokioInstant::now() + announce_interval);
                        }
                        event = self.swarm.select_next_some() => {
                            self.handle_swarm_event(event);
                        }
                    }
                }
                _ => {
                    let event = self.swarm.select_next_some().await;
                    self.handle_swarm_event(event);
                }
            }
        }
    }

    fn handle_swarm_event(&mut self, event: SwarmEvent<crate::behaviour::BehaviourEvent>) {
        self.handler.handle(&mut self.swarm, event);
    }
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
                membership_ttl_ms: 10_000,
                expiry_grace_ms: 0,
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
