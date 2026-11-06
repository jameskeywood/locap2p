use libp2p::{Swarm, noise, tcp, yamux};

use crate::{behaviour::Behaviour, config::NetworkConfig, error::NetworkError};

pub(crate) fn build(config: NetworkConfig) -> Result<Swarm<Behaviour>, NetworkError> {
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|error| NetworkError::Build(error.to_string()))?
        .with_quic()
        .with_behaviour(|key| Behaviour::new(key))
        .map_err(|error| NetworkError::Build(error.to_string()))?
        .build();

    for address in &config.listen_addrs {
        swarm
            .listen_on(address.clone())
            .map_err(|error| NetworkError::Listen(error.to_string()))?;
    }

    Ok(swarm)
}
