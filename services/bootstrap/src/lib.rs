//! Bootstrap seed gateway for locap2p sessions.

mod config;
mod seed_pool;

use std::{str::FromStr, time::Instant};

use anyhow::Context;
use futures::StreamExt;
use libp2p::identity::Keypair;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, PeerId, Swarm, identify, noise, request_response, tcp, yamux};
use locap2p_discovery::{SeedRequest, SeedResponse};
use tracing::{debug, info, warn};

pub use config::BootstrapConfig;
use seed_pool::SeedPool;

#[derive(NetworkBehaviour)]
struct Behaviour {
    seeds: locap2p_discovery::Behaviour,
    identify: identify::Behaviour,
}

/// Runs the bootstrap seed service until the process is terminated.
pub async fn run(config: BootstrapConfig) -> anyhow::Result<()> {
    let (identity, peer_id, endpoints) = prepare(&config)?;
    let mut swarm = build_swarm(identity)?;

    for address in &config.advertise_addrs {
        let address = Multiaddr::from_str(address)
            .with_context(|| format!("invalid advertised address: {address}"))?;
        swarm.add_external_address(address);
    }

    for address in &config.listen_addrs {
        let address = Multiaddr::from_str(address)
            .with_context(|| format!("invalid listen address: {address}"))?;
        swarm
            .listen_on(address)
            .map_err(|error| anyhow::anyhow!("failed to listen: {error}"))?;
    }

    print_info_block(peer_id, &endpoints);

    let mut seed_pool = SeedPool::new(
        config.seed_ttl(),
        config.max_seeds_per_session,
        config.max_seeds_returned,
    );

    while let Some(event) = swarm.next().await {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Seeds(request_response::Event::Message {
                peer,
                message,
                ..
            })) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    let response = handle_request(&mut seed_pool, peer, request);
                    if let Err(rejected) = swarm.behaviour_mut().seeds.respond(channel, response) {
                        warn!(%peer, ?rejected, "failed to send seed response");
                    }
                }
                request_response::Message::Response { .. } => {
                    debug!(%peer, "ignoring unexpected seed response");
                }
            },
            SwarmEvent::Behaviour(BehaviourEvent::Seeds(
                request_response::Event::OutboundFailure { peer, error, .. },
            )) => {
                warn!(%peer, %error, "bootstrap outbound request failed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Seeds(
                request_response::Event::InboundFailure { peer, error, .. },
            )) => {
                warn!(%peer, %error, "bootstrap inbound request failed");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Seeds(
                request_response::Event::ResponseSent { peer, .. },
            )) => {
                debug!(%peer, "seed response sent");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            })) => {
                debug!(%peer_id, observed = %info.observed_addr, "bootstrap identify");
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(%address, "bootstrap listening");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                debug!(%peer_id, "bootstrap connection established");
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                debug!(%peer_id, "bootstrap connection closed");
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, %error, "bootstrap outgoing connection failed");
            }
            SwarmEvent::IncomingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, %error, "bootstrap incoming connection failed");
            }
            _ => {}
        }
    }

    Ok(())
}

fn prepare(config: &BootstrapConfig) -> anyhow::Result<(Keypair, PeerId, Vec<String>)> {
    let identity = locap2p_identity::load_or_generate(&config.identity_path)?;
    let peer_id = identity.public().to_peer_id();
    let source = if config.advertise_addrs.is_empty() {
        &config.listen_addrs
    } else {
        &config.advertise_addrs
    };

    let mut endpoints = Vec::new();
    for value in source {
        if let Some(endpoint) = format_endpoint(value, peer_id)
            .with_context(|| format!("invalid advertised endpoint: {value}"))?
            && !endpoints.contains(&endpoint)
        {
            endpoints.push(endpoint);
        }
    }

    if endpoints.is_empty() {
        anyhow::bail!("no usable TCP endpoints; configure a TCP listen address or advertise_addrs");
    }

    Ok((identity, peer_id, endpoints))
}

fn format_endpoint(value: &str, peer_id: PeerId) -> anyhow::Result<Option<String>> {
    let mut address = Multiaddr::from_str(value)?;
    let mut tcp_port = None;

    for protocol in address.iter() {
        match protocol {
            Protocol::Tcp(port) => tcp_port = Some(port),
            Protocol::Ip4(ip) if ip.is_unspecified() => return Ok(None),
            Protocol::Ip6(ip) if ip.is_unspecified() => return Ok(None),
            _ => {}
        }
    }

    let Some(port) = tcp_port else {
        return Ok(None);
    };
    if port == 0 {
        anyhow::bail!("service endpoints must use a fixed non-zero TCP port");
    }

    if matches!(address.iter().last(), Some(Protocol::P2p(_))) {
        address.pop();
    }
    address.push(Protocol::P2p(peer_id));
    Ok(Some(address.to_string()))
}

fn print_info_block(peer_id: PeerId, endpoints: &[String]) {
    println!("Bootstrap service endpoints");
    println!("  peer_id: {peer_id}");
    println!("  discovery.bootstrap:");
    println!(
        "    {}",
        serde_json::to_string(endpoints).unwrap_or_else(|_| "[]".to_owned())
    );
}

fn build_swarm(identity: libp2p::identity::Keypair) -> anyhow::Result<Swarm<Behaviour>> {
    let swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_dns()?
        .with_behaviour(|key| Behaviour {
            seeds: locap2p_discovery::Behaviour::server(),
            identify: identify::Behaviour::new(
                identify::Config::new("/locap2p/identify/1".to_owned(), key.public())
                    .with_hide_listen_addrs(true),
            ),
        })?
        .build();

    Ok(swarm)
}

fn handle_request(seed_pool: &mut SeedPool, peer: PeerId, request: SeedRequest) -> SeedResponse {
    let application = request.application.clone();
    let session = request.session.clone();
    let seeds = seed_pool.seed(
        request.application,
        request.session,
        peer.to_string(),
        request.addresses,
        Instant::now(),
    );

    debug!(
        %peer,
        application = %application,
        session = %session,
        seeds = seeds.len(),
        "seed request"
    );

    SeedResponse { seeds }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer_id() -> PeerId {
        PeerId::from_str("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN").unwrap()
    }

    #[test]
    fn formats_endpoint_with_peer_id() {
        let endpoint = format_endpoint("/ip4/127.0.0.1/tcp/4002", peer_id())
            .unwrap()
            .unwrap();

        assert_eq!(
            endpoint,
            "/ip4/127.0.0.1/tcp/4002/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN"
        );
    }

    #[test]
    fn rejects_zero_port() {
        assert!(format_endpoint("/ip4/127.0.0.1/tcp/0", peer_id()).is_err());
    }

    #[test]
    fn skips_unspecified_addresses() {
        assert!(
            format_endpoint("/ip4/0.0.0.0/tcp/4002", peer_id())
                .unwrap()
                .is_none()
        );
    }
}
