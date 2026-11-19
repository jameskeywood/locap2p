use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use locap2p_config::{ApplicationId, SessionId};
use locap2p_discovery::PeerInfo;

const MAX_REGISTRATION_TTL: Duration = Duration::from_secs(300);
const MAX_ADDRESSES_PER_PEER: usize = 16;

type SessionKey = (ApplicationId, SessionId);

#[derive(Debug, Clone)]
struct Registration {
    addresses: Vec<String>,
    expires_at: Instant,
}

/// In-memory session registry owned by the bootstrap service.
#[derive(Debug, Default)]
pub struct Registry {
    max_peers_per_session: usize,
    sessions: HashMap<SessionKey, HashMap<String, Registration>>,
}

impl Registry {
    pub fn new(max_peers_per_session: usize) -> Self {
        Self {
            max_peers_per_session: max_peers_per_session.max(1),
            sessions: HashMap::new(),
        }
    }

    /// Registers or refreshes a peer and returns the other session members.
    pub fn announce(
        &mut self,
        application: ApplicationId,
        session: SessionId,
        peer_id: String,
        addresses: Vec<String>,
        ttl: Duration,
    ) -> Vec<PeerInfo> {
        self.announce_at(
            application,
            session,
            peer_id,
            addresses,
            ttl,
            Instant::now(),
        )
    }

    fn announce_at(
        &mut self,
        application: ApplicationId,
        session: SessionId,
        peer_id: String,
        mut addresses: Vec<String>,
        ttl: Duration,
        now: Instant,
    ) -> Vec<PeerInfo> {
        addresses.sort();
        addresses.dedup();
        addresses.truncate(MAX_ADDRESSES_PER_PEER);

        let key = (application, session);
        let members = self.sessions.entry(key.clone()).or_default();
        members.retain(|_, registration| registration.expires_at > now);

        let ttl = ttl.min(MAX_REGISTRATION_TTL);
        let expires_at = now + ttl;

        if members.contains_key(&peer_id) || members.len() < self.max_peers_per_session {
            members.insert(
                peer_id.clone(),
                Registration {
                    addresses,
                    expires_at,
                },
            );
        }

        let peers = members
            .iter()
            .filter(|(candidate, _)| **candidate != peer_id)
            .map(|(candidate, registration)| PeerInfo {
                peer_id: candidate.clone(),
                addresses: registration.addresses.clone(),
            })
            .collect();

        if members.is_empty() {
            self.sessions.remove(&key);
        }

        peers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn application() -> ApplicationId {
        ApplicationId::new("locap2p-test/1").unwrap()
    }

    fn session(value: &str) -> SessionId {
        SessionId::new(value).unwrap()
    }

    #[test]
    fn separates_sessions() {
        let mut registry = Registry::new(16);
        let now = Instant::now();
        let ttl = Duration::from_secs(30);

        registry.announce_at(
            application(),
            session("one"),
            "peer-a".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/1".to_owned()],
            ttl,
            now,
        );
        let peers = registry.announce_at(
            application(),
            session("two"),
            "peer-b".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/2".to_owned()],
            ttl,
            now,
        );

        assert!(peers.is_empty());
    }

    #[test]
    fn returns_other_members_and_excludes_requester() {
        let mut registry = Registry::new(16);
        let now = Instant::now();
        let ttl = Duration::from_secs(30);

        registry.announce_at(
            application(),
            session("one"),
            "peer-a".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/1".to_owned()],
            ttl,
            now,
        );
        let peers = registry.announce_at(
            application(),
            session("one"),
            "peer-b".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/2".to_owned()],
            ttl,
            now,
        );

        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer-a");
    }

    #[test]
    fn expired_members_are_removed() {
        let mut registry = Registry::new(16);
        let now = Instant::now();
        let ttl = Duration::from_secs(1);

        registry.announce_at(
            application(),
            session("one"),
            "peer-a".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/1".to_owned()],
            ttl,
            now,
        );

        let peers = registry.announce_at(
            application(),
            session("one"),
            "peer-b".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/2".to_owned()],
            ttl,
            now + Duration::from_secs(2),
        );

        assert!(peers.is_empty());
    }
}
