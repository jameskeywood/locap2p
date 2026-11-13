use libp2p::{StreamProtocol, identity::Keypair, mdns, request_response, swarm::NetworkBehaviour};

/// Versioned application protocol used for anti-entropy exchanges.
pub(crate) const SYNC_PROTOCOL: StreamProtocol = StreamProtocol::new("/locap2p/sync/1");

#[derive(NetworkBehaviour)]
pub(crate) struct Behaviour {
    pub(crate) sync: request_response::cbor::Behaviour<Vec<u8>, Vec<u8>>,
    pub(crate) mdns: mdns::tokio::Behaviour,
}

impl Behaviour {
    pub(crate) fn new(key: &Keypair) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let sync = request_response::cbor::Behaviour::<Vec<u8>, Vec<u8>>::new(
            [(SYNC_PROTOCOL, request_response::ProtocolSupport::Full)],
            request_response::Config::default(),
        );

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
            .map_err(std::io::Error::other)?;

        Ok(Self { sync, mdns })
    }
}
