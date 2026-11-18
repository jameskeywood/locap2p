use std::time::Duration;

use libp2p::{Multiaddr, StreamProtocol, Swarm, identity::Keypair, noise, tcp, yamux};

use crate::{behaviour::Behaviour, error::NetworkError};

pub(crate) fn build(
    identity: Keypair,
    listen_addrs: &[Multiaddr],
    sync_protocol: StreamProtocol,
    request_timeout: Duration,
    enable_dcutr: bool,
) -> Result<Swarm<Behaviour>, NetworkError> {
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|error| NetworkError::Build(error.to_string()))?
        .with_quic()
        .with_dns()
        .map_err(|error| NetworkError::Build(error.to_string()))?
        .with_relay_client(noise::Config::new, yamux::Config::default)
        .map_err(|error| NetworkError::Build(error.to_string()))?
        .with_behaviour(|key, relay_client| {
            Behaviour::new(
                key,
                sync_protocol,
                request_timeout,
                enable_dcutr,
                relay_client,
            )
        })
        .map_err(|error| NetworkError::Build(error.to_string()))?
        .build();

    for address in listen_addrs {
        swarm
            .listen_on(address.clone())
            .map_err(|error| NetworkError::Listen(error.to_string()))?;
    }

    Ok(swarm)
}
