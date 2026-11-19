use libp2p::{
    StreamProtocol, dcutr, identify,
    identity::Keypair,
    relay, request_response,
    swarm::{NetworkBehaviour, behaviour::toggle::Toggle},
};
use locap2p_config::{ApplicationId, SessionId};
use locap2p_discovery::{BOOTSTRAP_PROTOCOL, BootstrapRequest, BootstrapResponse};

use crate::error::NetworkError;

/// Versioned application protocol used for anti-entropy exchanges.
pub(crate) fn session_protocol(
    application: &ApplicationId,
    session: &SessionId,
) -> Result<StreamProtocol, NetworkError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(application.as_str().as_bytes());
    hasher.update(&[0]);
    hasher.update(session.as_str().as_bytes());
    let hash = hasher.finalize().to_hex();

    StreamProtocol::try_from_owned(format!("/locap2p/{hash}/sync/1"))
        .map_err(|error| NetworkError::Config(format!("invalid synchronization protocol: {error}")))
}

#[derive(NetworkBehaviour)]
pub(crate) struct Behaviour {
    pub(crate) sync: request_response::cbor::Behaviour<Vec<u8>, Vec<u8>>,
    pub(crate) bootstrap: request_response::cbor::Behaviour<BootstrapRequest, BootstrapResponse>,
    pub(crate) relay_client: relay::client::Behaviour,
    pub(crate) identify: identify::Behaviour,
    pub(crate) dcutr: Toggle<dcutr::Behaviour>,
}

impl Behaviour {
    pub(crate) fn new(
        key: &Keypair,
        sync_protocol: StreamProtocol,
        request_timeout: std::time::Duration,
        enable_dcutr: bool,
        relay_client: relay::client::Behaviour,
    ) -> Self {
        let request_config =
            request_response::Config::default().with_request_timeout(request_timeout);

        let sync = request_response::cbor::Behaviour::<Vec<u8>, Vec<u8>>::new(
            [(sync_protocol, request_response::ProtocolSupport::Full)],
            request_config.clone(),
        );

        let bootstrap =
            request_response::cbor::Behaviour::<BootstrapRequest, BootstrapResponse>::new(
                [(
                    StreamProtocol::new(BOOTSTRAP_PROTOCOL),
                    request_response::ProtocolSupport::Outbound,
                )],
                request_config,
            );

        let identify = identify::Behaviour::new(identify::Config::new(
            "/locap2p/identify/1".to_owned(),
            key.public(),
        ));

        let dcutr = enable_dcutr
            .then(|| dcutr::Behaviour::new(key.public().to_peer_id()))
            .into();

        Self {
            sync,
            bootstrap,
            relay_client,
            identify,
            dcutr,
        }
    }
}
