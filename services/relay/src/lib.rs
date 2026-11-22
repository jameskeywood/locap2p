//! Circuit relay v2 service for locap2p peers.

mod config;

use std::{str::FromStr, time::Duration};

use anyhow::Context;
use futures::StreamExt;
use libp2p::identity::Keypair;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, PeerId, Swarm, identify, noise, ping, relay, tcp, yamux};
use tracing::{debug, info, warn};

pub use config::{RelayConfig, RelayLimits};

#[derive(NetworkBehaviour)]
struct Behaviour {
    relay: relay::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
}

/// Runs the relay service until the process is terminated.
pub async fn run(config: RelayConfig) -> anyhow::Result<()> {
    let (identity, peer_id, endpoints) = prepare(&config)?;
    let mut swarm = build_swarm(identity, &config)?;
    swarm
        .behaviour_mut()
        .relay
        .set_status(Some(relay::Status::Enable));

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

    while let Some(event) = swarm.next().await {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Relay(event)) => match event {
                relay::Event::ReservationReqAccepted {
                    src_peer_id,
                    renewed,
                } => {
                    info!(%src_peer_id, renewed, "relay reservation accepted");
                }
                relay::Event::ReservationClosed { src_peer_id } => {
                    info!(%src_peer_id, "relay reservation closed");
                }
                relay::Event::ReservationTimedOut { src_peer_id } => {
                    info!(%src_peer_id, "relay reservation timed out");
                }
                relay::Event::CircuitReqAccepted {
                    src_peer_id,
                    dst_peer_id,
                } => {
                    debug!(%src_peer_id, %dst_peer_id, "relay circuit accepted");
                }
                relay::Event::CircuitClosed {
                    src_peer_id,
                    dst_peer_id,
                    error,
                } => {
                    debug!(%src_peer_id, %dst_peer_id, ?error, "relay circuit closed");
                }
                relay::Event::StatusChanged { status } => {
                    info!(?status, "relay status changed");
                }
                other => {
                    debug!(event = ?other, "relay event");
                }
            },
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            })) => {
                debug!(%peer_id, observed = %info.observed_addr, "relay identify");
            }
            SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {}
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(%address, "relay listening");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                debug!(%peer_id, "relay connection established");
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                debug!(%peer_id, "relay connection closed");
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, %error, "relay outgoing connection failed");
            }
            SwarmEvent::IncomingConnectionError { peer_id, error, .. } => {
                warn!(?peer_id, %error, "relay incoming connection failed");
            }
            _ => {}
        }
    }

    Ok(())
}

fn prepare(config: &RelayConfig) -> anyhow::Result<(Keypair, PeerId, Vec<String>)> {
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
    println!("Relay service endpoints");
    println!("  peer_id: {peer_id}");
    println!("  connectivity.relay:");
    println!(
        "    {}",
        serde_json::to_string(endpoints).unwrap_or_else(|_| "[]".to_owned())
    );
}

fn build_swarm(
    identity: libp2p::identity::Keypair,
    config: &RelayConfig,
) -> anyhow::Result<Swarm<Behaviour>> {
    let local_peer_id = identity.public().to_peer_id();
    let relay_config = relay::Config {
        max_reservations: config.limits.max_reservations,
        max_reservations_per_peer: config.limits.max_reservations_per_peer,
        reservation_duration: Duration::from_secs(60 * 60),
        max_circuits: config.limits.max_circuits,
        max_circuits_per_peer: config.limits.max_circuits_per_peer,
        max_circuit_duration: config.limits.max_circuit_duration(),
        max_circuit_bytes: config.limits.max_circuit_bytes,
        ..Default::default()
    };

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
            relay: relay::Behaviour::new(local_peer_id, relay_config),
            identify: identify::Behaviour::new(
                identify::Config::new("/locap2p/identify/1".to_owned(), key.public())
                    .with_hide_listen_addrs(true),
            ),
            ping: ping::Behaviour::new(ping::Config::new()),
        })?
        .build();

    Ok(swarm)
}
