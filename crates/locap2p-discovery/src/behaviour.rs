use std::time::Duration;

use libp2p::{
    PeerId, StreamProtocol,
    request_response::{self, ProtocolSupport, ResponseChannel},
    swarm::NetworkBehaviour,
};

use crate::protocol::{SEED_PROTOCOL, SeedRequest, SeedResponse};

pub type BehaviourEvent = request_response::Event<SeedRequest, SeedResponse>;

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "BehaviourEvent")]
pub struct Behaviour {
    seeds: request_response::cbor::Behaviour<SeedRequest, SeedResponse>,
}

impl Behaviour {
    pub fn client(request_timeout: Duration) -> Self {
        Self::new(
            ProtocolSupport::Outbound,
            request_response::Config::default().with_request_timeout(request_timeout),
        )
    }

    pub fn server() -> Self {
        Self::new(
            ProtocolSupport::Inbound,
            request_response::Config::default(),
        )
    }

    pub fn request_seeds(&mut self, bootstrap_peer: PeerId, request: SeedRequest) {
        self.seeds.send_request(&bootstrap_peer, request);
    }

    pub fn respond(
        &mut self,
        channel: ResponseChannel<SeedResponse>,
        response: SeedResponse,
    ) -> Result<(), SeedResponse> {
        self.seeds.send_response(channel, response)
    }

    fn new(support: ProtocolSupport, config: request_response::Config) -> Self {
        let seeds = request_response::cbor::Behaviour::new(
            [(StreamProtocol::new(SEED_PROTOCOL), support)],
            config,
        );

        Self { seeds }
    }
}
