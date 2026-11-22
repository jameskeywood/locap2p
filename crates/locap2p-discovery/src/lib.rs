//! Bootstrap seed discovery.

mod behaviour;
mod protocol;

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use libp2p::{
    Multiaddr, PeerId,
    request_response::{self, Message},
};
use locap2p_config::ApplicationConfig;
use tracing::{debug, warn};

pub use behaviour::{Behaviour, BehaviourEvent};
pub use protocol::{SEED_PROTOCOL, SeedPeer, SeedRequest, SeedResponse};

const SEED_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct SeedLookup {
    pub bootstrap_peer: PeerId,
    pub request: SeedRequest,
}

pub struct SeedClient {
    application: ApplicationConfig,
    bootstrap: Vec<PeerId>,
    pending: HashSet<PeerId>,
    next_refresh: Instant,
}

impl SeedClient {
    pub fn new(
        application: &ApplicationConfig,
        bootstrap: impl IntoIterator<Item = PeerId>,
    ) -> Self {
        let mut seen = HashSet::new();
        let bootstrap: Vec<PeerId> = bootstrap
            .into_iter()
            .filter(|peer_id| seen.insert(*peer_id))
            .collect();

        Self {
            application: application.clone(),
            bootstrap,
            pending: HashSet::new(),
            next_refresh: Instant::now(),
        }
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        (!self.bootstrap.is_empty()).then_some(self.next_refresh)
    }

    pub fn poll(&mut self, now: Instant, local_addresses: &[Multiaddr]) -> Vec<SeedLookup> {
        if self.bootstrap.is_empty() || now < self.next_refresh {
            return Vec::new();
        }

        self.next_refresh = now + SEED_REFRESH_INTERVAL;
        let request = SeedRequest {
            application: self.application.id.clone(),
            session: self.application.session.clone(),
            addresses: local_addresses.iter().map(ToString::to_string).collect(),
        };

        let mut lookups = Vec::new();
        for bootstrap_peer in &self.bootstrap {
            if self.pending.contains(bootstrap_peer) {
                continue;
            }

            self.pending.insert(*bootstrap_peer);
            lookups.push(SeedLookup {
                bootstrap_peer: *bootstrap_peer,
                request: request.clone(),
            });
        }

        lookups
    }

    pub fn on_behaviour_event(&mut self, event: BehaviourEvent) -> Vec<SeedPeer> {
        match event {
            request_response::Event::Message { peer, message, .. } => match message {
                Message::Response { response, .. } => self.on_response(peer, response),
                Message::Request { .. } => {
                    debug!(%peer, "ignoring unexpected seed request");
                    Vec::new()
                }
            },
            request_response::Event::OutboundFailure { peer, error, .. } => {
                self.on_failure(peer);
                warn!(%peer, %error, "seed request failed");
                Vec::new()
            }
            request_response::Event::InboundFailure { peer, error, .. } => {
                warn!(%peer, %error, "seed response failed");
                Vec::new()
            }
            request_response::Event::ResponseSent { peer, .. } => {
                debug!(%peer, "seed response sent");
                Vec::new()
            }
        }
    }

    fn on_response(&mut self, from: PeerId, response: SeedResponse) -> Vec<SeedPeer> {
        if !self.bootstrap.contains(&from) {
            debug!(%from, "ignoring seed response from unexpected peer");
            return Vec::new();
        }

        self.pending.remove(&from);
        response.seeds
    }

    fn on_failure(&mut self, from: PeerId) {
        self.pending.remove(&from);
    }
}

#[cfg(test)]
mod tests {
    use libp2p::identity::Keypair;
    use locap2p_config::{ApplicationId, SessionId};

    use super::*;

    fn peer_id() -> PeerId {
        Keypair::generate_ed25519().public().to_peer_id()
    }

    fn application() -> ApplicationConfig {
        ApplicationConfig {
            id: ApplicationId::new("locap2p-test/1"),
            session: SessionId::new("test"),
        }
    }

    fn address(value: &str) -> Multiaddr {
        value.parse().unwrap()
    }

    #[test]
    fn requests_seeds_once_per_interval() {
        let bootstrap = peer_id();
        let mut client = SeedClient::new(&application(), vec![bootstrap]);
        let addresses = vec![address("/ip4/127.0.0.1/tcp/4000")];

        let now = Instant::now();
        assert_eq!(client.poll(now, &addresses).len(), 1);
        assert!(client.poll(now, &addresses).is_empty());

        client.on_failure(bootstrap);
        assert_eq!(
            client.poll(now + SEED_REFRESH_INTERVAL, &addresses).len(),
            1
        );
    }

    #[test]
    fn deduplicates_bootstrap_peers() {
        let bootstrap = peer_id();
        let mut client = SeedClient::new(&application(), vec![bootstrap, bootstrap]);

        assert_eq!(client.poll(Instant::now(), &[]).len(), 1);
    }

    #[test]
    fn failure_allows_the_next_request() {
        let bootstrap = peer_id();
        let mut client = SeedClient::new(&application(), vec![bootstrap]);

        let now = Instant::now();
        client.poll(now, &[]);
        client.on_failure(bootstrap);

        assert_eq!(client.poll(now + SEED_REFRESH_INTERVAL, &[]).len(), 1);
    }

    #[test]
    fn ignores_responses_from_unexpected_peers() {
        let bootstrap = peer_id();
        let mut client = SeedClient::new(&application(), vec![bootstrap]);

        let seeds = client.on_response(
            peer_id(),
            SeedResponse {
                seeds: vec![SeedPeer {
                    peer_id: peer_id().to_string(),
                    addresses: vec!["/ip4/127.0.0.1/tcp/4000".to_owned()],
                }],
            },
        );

        assert!(seeds.is_empty());
    }
}
