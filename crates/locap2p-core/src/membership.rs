use std::{
    collections::{BTreeSet, HashMap},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MemberState {
    Active,
    Left,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerRecord {
    pub(crate) peer_id: NodeId,
    pub(crate) addresses: Vec<String>,
    pub(crate) version: u64,
    pub(crate) expires_at_ms: u64,
    pub(crate) state: MemberState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipChange {
    Joined {
        peer_id: NodeId,
        addresses: Vec<String>,
    },
    Left {
        peer_id: NodeId,
    },
    AddressesUpdated {
        peer_id: NodeId,
        addresses: Vec<String>,
    },
}

pub struct MembershipView {
    local_peer_id: NodeId,
    local_record: PeerRecord,
    records: HashMap<NodeId, PeerRecord>,
    active: BTreeSet<NodeId>,
    ttl_ms: u64,
    grace_ms: u64,
}

impl MembershipView {
    pub fn new(
        local_peer_id: NodeId,
        addresses: Vec<String>,
        now_ms: u64,
        ttl_ms: u64,
        grace_ms: u64,
    ) -> Self {
        let local_record = PeerRecord {
            peer_id: local_peer_id.clone(),
            addresses,
            version: now_ms.max(1),
            expires_at_ms: now_ms.saturating_add(ttl_ms),
            state: MemberState::Active,
        };

        Self {
            local_peer_id,
            local_record,
            records: HashMap::new(),
            active: BTreeSet::new(),
            ttl_ms,
            grace_ms,
        }
    }

    pub fn refresh_local(&mut self, addresses: Vec<String>, now_ms: u64) {
        self.local_record.addresses = addresses;
        self.local_record.version = now_ms.max(self.local_record.version.saturating_add(1));
        self.local_record.expires_at_ms = now_ms.saturating_add(self.ttl_ms);
    }

    pub fn records(&self) -> Vec<PeerRecord> {
        let mut records = Vec::with_capacity(self.records.len() + 1);
        records.push(self.local_record.clone());
        records.extend(self.records.values().cloned());
        records
    }

    pub fn active_peers(&self) -> BTreeSet<NodeId> {
        self.active.clone()
    }

    pub fn is_active(&self, peer_id: &NodeId) -> bool {
        self.active.contains(peer_id)
    }

    pub fn merge(&mut self, records: Vec<PeerRecord>, now_ms: u64) -> Vec<MembershipChange> {
        let mut changes = Vec::new();

        for record in records {
            if record.peer_id == self.local_peer_id {
                continue;
            }

            if record.state == MemberState::Active && is_expired(&record, now_ms, self.grace_ms) {
                continue;
            }

            if self
                .records
                .get(&record.peer_id)
                .is_some_and(|existing| existing.version >= record.version)
            {
                continue;
            }

            match record.state {
                MemberState::Active => {
                    let was_active = self.active.contains(&record.peer_id);
                    let addresses_changed = self
                        .records
                        .get(&record.peer_id)
                        .is_some_and(|existing| existing.addresses != record.addresses);

                    self.records.insert(record.peer_id.clone(), record.clone());

                    if !was_active {
                        self.active.insert(record.peer_id.clone());
                        changes.push(MembershipChange::Joined {
                            peer_id: record.peer_id,
                            addresses: record.addresses,
                        });
                    } else if addresses_changed {
                        changes.push(MembershipChange::AddressesUpdated {
                            peer_id: record.peer_id,
                            addresses: record.addresses,
                        });
                    }
                }
                MemberState::Left => {
                    let was_active = self.active.remove(&record.peer_id);
                    self.records.insert(record.peer_id.clone(), record.clone());

                    if was_active {
                        changes.push(MembershipChange::Left {
                            peer_id: record.peer_id,
                        });
                    }
                }
            }
        }

        changes
    }

    pub fn expire(&mut self, now_ms: u64) -> Vec<MembershipChange> {
        let expired: Vec<NodeId> = self
            .active
            .iter()
            .filter(|peer_id| {
                self.records
                    .get(*peer_id)
                    .is_none_or(|record| is_expired(record, now_ms, self.grace_ms))
            })
            .cloned()
            .collect();

        let mut changes = Vec::new();

        for peer_id in expired {
            self.active.remove(&peer_id);
            self.records.remove(&peer_id);
            changes.push(MembershipChange::Left { peer_id });
        }

        changes
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

fn is_expired(record: &PeerRecord, now_ms: u64, grace_ms: u64) -> bool {
    record.expires_at_ms.saturating_add(grace_ms) <= now_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer_id(value: &str) -> NodeId {
        NodeId::from(value)
    }

    fn active(id: &str, addresses: &[&str], version: u64, expires_at_ms: u64) -> PeerRecord {
        PeerRecord {
            peer_id: peer_id(id),
            addresses: addresses.iter().map(|value| (*value).to_owned()).collect(),
            version,
            expires_at_ms,
            state: MemberState::Active,
        }
    }

    fn left(id: &str, version: u64) -> PeerRecord {
        PeerRecord {
            peer_id: peer_id(id),
            addresses: Vec::new(),
            version,
            expires_at_ms: u64::MAX,
            state: MemberState::Left,
        }
    }

    fn membership() -> MembershipView {
        MembershipView::new(
            peer_id("local"),
            vec!["/ip4/127.0.0.1/tcp/1".to_owned()],
            1,
            100,
            0,
        )
    }

    #[test]
    fn joins_and_updates_addresses() {
        let mut membership = membership();
        let remote = active("remote", &["/ip4/127.0.0.1/tcp/2"], 2, 200);

        assert_eq!(
            membership.merge(vec![remote], 0),
            vec![MembershipChange::Joined {
                peer_id: peer_id("remote"),
                addresses: vec!["/ip4/127.0.0.1/tcp/2".to_owned()],
            }]
        );

        let updated = active("remote", &["/ip4/127.0.0.1/tcp/3"], 3, 200);

        assert_eq!(
            membership.merge(vec![updated], 0),
            vec![MembershipChange::AddressesUpdated {
                peer_id: peer_id("remote"),
                addresses: vec!["/ip4/127.0.0.1/tcp/3".to_owned()],
            }]
        );
    }

    #[test]
    fn ignores_stale_records() {
        let mut membership = membership();

        membership.merge(vec![active("remote", &["/ip4/127.0.0.1/tcp/2"], 5, 200)], 0);

        assert!(
            membership
                .merge(vec![active("remote", &["/ip4/127.0.0.1/tcp/9"], 4, 200)], 0)
                .is_empty()
        );
    }

    #[test]
    fn left_record_removes_the_peer() {
        let mut membership = membership();
        membership.merge(vec![active("remote", &["/ip4/127.0.0.1/tcp/2"], 2, 200)], 0);

        assert_eq!(
            membership.merge(vec![left("remote", 3)], 0),
            vec![MembershipChange::Left {
                peer_id: peer_id("remote"),
            }]
        );
        assert!(!membership.is_active(&peer_id("remote")));

        assert!(
            membership
                .merge(vec![active("remote", &["/ip4/127.0.0.1/tcp/2"], 2, 200)], 0)
                .is_empty()
        );
    }

    #[test]
    fn expires_and_rejoins() {
        let mut membership = membership();
        membership.merge(vec![active("remote", &["/ip4/127.0.0.1/tcp/2"], 2, 10)], 0);

        assert_eq!(
            membership.expire(11),
            vec![MembershipChange::Left {
                peer_id: peer_id("remote"),
            }]
        );
        assert!(
            membership
                .merge(vec![active("remote", &["/ip4/127.0.0.1/tcp/2"], 2, 10)], 20)
                .is_empty()
        );

        assert_eq!(
            membership.merge(vec![active("remote", &["/ip4/127.0.0.1/tcp/3"], 3, 30)], 20),
            vec![MembershipChange::Joined {
                peer_id: peer_id("remote"),
                addresses: vec!["/ip4/127.0.0.1/tcp/3".to_owned()],
            }]
        );
    }

    #[test]
    fn local_record_is_refreshed_and_included() {
        let mut membership = membership();
        membership.refresh_local(vec!["/ip4/127.0.0.1/tcp/9".to_owned()], 5);

        let local = membership
            .records()
            .into_iter()
            .find(|record| record.peer_id == peer_id("local"))
            .unwrap();

        assert_eq!(local.addresses, vec!["/ip4/127.0.0.1/tcp/9".to_owned()]);
        assert_eq!(local.expires_at_ms, 105);
        assert_eq!(local.version, 5);
    }
}
