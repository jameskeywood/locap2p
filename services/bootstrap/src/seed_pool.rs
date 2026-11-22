use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use locap2p_config::{ApplicationId, SessionId};
use locap2p_discovery::SeedPeer;

type SessionKey = (ApplicationId, SessionId);

struct SeedEntry {
    peer_id: String,
    addresses: Vec<String>,
    expires_at: Instant,
}

pub(crate) struct SeedPool {
    ttl: Duration,
    max_seeds_per_session: usize,
    max_seeds_returned: usize,
    sessions: HashMap<SessionKey, Vec<SeedEntry>>,
}

impl SeedPool {
    pub(crate) fn new(
        ttl: Duration,
        max_seeds_per_session: usize,
        max_seeds_returned: usize,
    ) -> Self {
        Self {
            ttl,
            max_seeds_per_session,
            max_seeds_returned,
            sessions: HashMap::new(),
        }
    }

    pub(crate) fn seed(
        &mut self,
        application: ApplicationId,
        session: SessionId,
        peer_id: String,
        addresses: Vec<String>,
        now: Instant,
    ) -> Vec<SeedPeer> {
        let key = (application, session);
        let entries = self.sessions.entry(key.clone()).or_default();
        entries.retain(|entry| entry.expires_at > now);

        if !addresses.is_empty() {
            if let Some(entry) = entries.iter_mut().find(|entry| entry.peer_id == peer_id) {
                entry.addresses = addresses;
                entry.expires_at = now + self.ttl;
            } else if entries.len() < self.max_seeds_per_session {
                entries.push(SeedEntry {
                    peer_id: peer_id.clone(),
                    addresses,
                    expires_at: now + self.ttl,
                });
            }
        }

        let seeds = entries
            .iter()
            .filter(|entry| entry.peer_id != peer_id)
            .take(self.max_seeds_returned)
            .map(|entry| SeedPeer {
                peer_id: entry.peer_id.clone(),
                addresses: entry.addresses.clone(),
            })
            .collect();

        if entries.is_empty() {
            self.sessions.remove(&key);
        }

        seeds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn application() -> ApplicationId {
        ApplicationId::new("locap2p-test/1")
    }

    fn session() -> SessionId {
        SessionId::new("test")
    }

    fn pool() -> SeedPool {
        SeedPool::new(Duration::from_secs(60), 4, 2)
    }

    #[test]
    fn returns_other_seeds() {
        let mut pool = pool();
        let now = Instant::now();

        pool.seed(
            application(),
            session(),
            "peer-a".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/1".to_owned()],
            now,
        );
        let seeds = pool.seed(
            application(),
            session(),
            "peer-b".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/2".to_owned()],
            now,
        );

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].peer_id, "peer-a");
    }

    #[test]
    fn expired_entries_are_removed() {
        let mut pool = pool();
        let now = Instant::now();

        pool.seed(
            application(),
            session(),
            "peer-a".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/1".to_owned()],
            now,
        );
        let seeds = pool.seed(
            application(),
            session(),
            "peer-b".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/2".to_owned()],
            now + Duration::from_secs(61),
        );

        assert!(seeds.is_empty());
    }

    #[test]
    fn entries_without_addresses_are_not_stored() {
        let mut pool = pool();
        let now = Instant::now();

        pool.seed(
            application(),
            session(),
            "peer-a".to_owned(),
            Vec::new(),
            now,
        );
        let seeds = pool.seed(
            application(),
            session(),
            "peer-b".to_owned(),
            Vec::new(),
            now,
        );

        assert!(seeds.is_empty());
    }

    #[test]
    fn returned_seeds_are_limited() {
        let mut pool = pool();
        let now = Instant::now();

        for index in 0..3 {
            pool.seed(
                application(),
                session(),
                format!("peer-{index}"),
                vec![format!("/ip4/127.0.0.1/tcp/{index}")],
                now,
            );
        }

        let seeds = pool.seed(
            application(),
            session(),
            "peer-requester".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/10".to_owned()],
            now,
        );

        assert_eq!(seeds.len(), 2);
    }

    #[test]
    fn refuses_new_entries_when_the_pool_is_full() {
        let mut pool = SeedPool::new(Duration::from_secs(60), 1, 2);
        let now = Instant::now();

        pool.seed(
            application(),
            session(),
            "peer-a".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/1".to_owned()],
            now,
        );
        let seeds = pool.seed(
            application(),
            session(),
            "peer-b".to_owned(),
            vec!["/ip4/127.0.0.1/tcp/2".to_owned()],
            now,
        );

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].peer_id, "peer-a");
    }
}
